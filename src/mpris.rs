use std::{
    collections::HashMap,
    fs,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use tokio::{
    sync::{mpsc, watch},
    time::{interval, sleep, MissedTickBehavior},
};
use zbus::{
    names::BusName,
    proxy,
    zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Str},
    Connection,
};

use crate::model::{Playback, PlayerSnapshot, ProviderState};

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const POSITION_RESYNC: Duration = Duration::from_secs(2);
const PLAYER_RESELECT: Duration = Duration::from_secs(3);
const DISCOVERY_RETRY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerCommand {
    Previous,
    TogglePlayback,
    Next,
    Seek(f64),
}

#[proxy(
    interface = "org.mpris.MediaPlayer2",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait MediaPlayer {
    #[zbus(property)]
    fn identity(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn desktop_entry(&self) -> zbus::Result<String>;
}

#[proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait Player {
    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, OwnedValue>>;

    #[zbus(property)]
    fn position(&self) -> zbus::Result<i64>;

    #[zbus(property)]
    fn rate(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn volume(&self) -> zbus::Result<f64>;

    #[zbus(property)]
    fn can_go_previous(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_go_next(&self) -> zbus::Result<bool>;

    #[zbus(property)]
    fn can_seek(&self) -> zbus::Result<bool>;

    fn previous(&self) -> zbus::Result<()>;

    fn play_pause(&self) -> zbus::Result<()>;

    fn next(&self) -> zbus::Result<()>;

    fn set_position(&self, track_id: ObjectPath<'_>, position: i64) -> zbus::Result<()>;

    #[zbus(signal)]
    fn seeked(&self, position: i64) -> zbus::Result<()>;
}

pub async fn monitor(
    selector: String,
    sender: watch::Sender<ProviderState>,
    mut commands: mpsc::Receiver<PlayerCommand>,
) {
    loop {
        let connection = match Connection::session().await {
            Ok(connection) => connection,
            Err(error) => {
                sender.send_replace(ProviderState::Unavailable(format!(
                    "Session D-Bus unavailable: {error}"
                )));
                sleep(DISCOVERY_RETRY).await;
                continue;
            }
        };

        let service = match select_service(&connection, &selector).await {
            Ok(Some(service)) => service,
            Ok(None) => {
                sender.send_replace(ProviderState::Unavailable(
                    "No compatible media player is running".into(),
                ));
                sleep(DISCOVERY_RETRY).await;
                continue;
            }
            Err(error) => {
                sender.send_replace(ProviderState::Unavailable(error.to_string()));
                sleep(DISCOVERY_RETRY).await;
                continue;
            }
        };

        if let Err(error) =
            watch_service(&connection, &service, &selector, &sender, &mut commands).await
        {
            sender.send_replace(ProviderState::Unavailable(format!(
                "Lost {service}: {error}"
            )));
            sleep(DISCOVERY_RETRY).await;
        }
    }
}

async fn watch_service(
    connection: &Connection,
    service: &str,
    selector: &str,
    sender: &watch::Sender<ProviderState>,
    commands: &mut mpsc::Receiver<PlayerCommand>,
) -> Result<()> {
    let player = player_proxy(connection, service).await?;
    let identity = read_identity(connection, service)
        .await
        .unwrap_or_else(|_| service.trim_start_matches(MPRIS_PREFIX).into());
    let mut metadata_changes = player.receive_metadata_changed().await;
    let mut playback_changes = player.receive_playback_status_changed().await;
    let mut rate_changes = player.receive_rate_changed().await;
    let mut volume_changes = player.receive_volume_changed().await;
    let mut previous_changes = player.receive_can_go_previous_changed().await;
    let mut next_changes = player.receive_can_go_next_changed().await;
    let mut seek_changes = player.receive_can_seek_changed().await;
    let mut seeked = player.receive_seeked().await?;
    let mut resync = interval(POSITION_RESYNC);
    resync.set_missed_tick_behavior(MissedTickBehavior::Delay);
    resync.tick().await;
    let mut reselect = interval(PLAYER_RESELECT);
    reselect.set_missed_tick_behavior(MissedTickBehavior::Delay);
    reselect.tick().await;

    loop {
        let snapshot = read_player_with_proxy(service, &identity, &player).await?;
        sender.send_replace(ProviderState::Ready(Box::new(snapshot.clone())));

        tokio::select! {
            change = metadata_changes.next() => require_signal(change, service)?,
            change = playback_changes.next() => require_signal(change, service)?,
            change = rate_changes.next() => require_signal(change, service)?,
            change = volume_changes.next() => require_signal(change, service)?,
            change = previous_changes.next() => require_signal(change, service)?,
            change = next_changes.next() => require_signal(change, service)?,
            change = seek_changes.next() => require_signal(change, service)?,
            change = seeked.next() => require_signal(change, service)?,
            command = commands.recv(), if !commands.is_closed() => {
                if let Some(command) = command {
                    apply_command(&player, &snapshot, command).await?;
                }
            },
            _ = resync.tick() => {},
            _ = reselect.tick(), if selector == "auto" => {
                if select_service(connection, selector).await?.as_deref() != Some(service) {
                    return Ok(());
                }
            }
        }
    }
}

fn require_signal<T>(signal: Option<T>, service: &str) -> Result<()> {
    signal
        .map(drop)
        .ok_or_else(|| anyhow!("signal stream closed for {service}"))
}

pub async fn select_service(connection: &Connection, selector: &str) -> Result<Option<String>> {
    let dbus = zbus::fdo::DBusProxy::new(connection).await?;
    let mut services = dbus
        .list_names()
        .await?
        .into_iter()
        .map(|name| name.to_string())
        .filter(|name| name.starts_with(MPRIS_PREFIX))
        .filter(|name| !name.ends_with(".playerctld"))
        .collect::<Vec<_>>();
    services.sort();

    if selector != "auto" {
        let selector = selector.to_lowercase();
        for service in services {
            let identity = read_identity(connection, &service)
                .await
                .unwrap_or_default();
            if service.to_lowercase().contains(&selector)
                || identity.to_lowercase().contains(&selector)
            {
                return Ok(Some(service));
            }
        }
        return Ok(None);
    }

    let mut fallback = None;
    for service in services {
        let player = match player_proxy(connection, &service).await {
            Ok(player) => player,
            Err(_) => continue,
        };
        if fallback.is_none() {
            fallback = Some(service.clone());
        }
        if player
            .playback_status()
            .await
            .is_ok_and(|status| status == "Playing")
        {
            return Ok(Some(service));
        }
    }
    Ok(fallback)
}

pub async fn read_player(connection: &Connection, service: &str) -> Result<PlayerSnapshot> {
    let player = player_proxy(connection, service).await?;
    let identity = read_identity(connection, service)
        .await
        .unwrap_or_else(|_| service.trim_start_matches(MPRIS_PREFIX).into());
    read_player_with_proxy(service, &identity, &player).await
}

pub async fn send_command(
    connection: &Connection,
    service: &str,
    snapshot: &PlayerSnapshot,
    command: PlayerCommand,
) -> Result<()> {
    let player = player_proxy(connection, service).await?;
    apply_command(&player, snapshot, command).await
}

async fn player_proxy<'a>(connection: &'a Connection, service: &'a str) -> Result<PlayerProxy<'a>> {
    PlayerProxy::builder(connection)
        .destination(service)?
        .path(MPRIS_PATH)?
        .build()
        .await
        .with_context(|| format!("could not connect to {service}"))
}

async fn read_player_with_proxy(
    service: &str,
    identity: &str,
    player: &PlayerProxy<'_>,
) -> Result<PlayerSnapshot> {
    let metadata = player.metadata().await.context("could not read metadata")?;
    let playback = player
        .playback_status()
        .await
        .map(|value| Playback::from_mpris(&value))
        .unwrap_or(Playback::Stopped);

    Ok(PlayerSnapshot {
        service: service.into(),
        identity: identity.into(),
        track_id: metadata_object_path(&metadata, "mpris:trackid"),
        art_url: metadata_string(&metadata, "mpris:artUrl"),
        title: metadata_string(&metadata, "xesam:title").unwrap_or_else(|| "Untitled".into()),
        artists: metadata_strings(&metadata, "xesam:artist").unwrap_or_default(),
        album: metadata_string(&metadata, "xesam:album").unwrap_or_default(),
        playback,
        duration: microseconds(metadata_i64(&metadata, "mpris:length").unwrap_or(0)),
        position: microseconds(player.position().await.unwrap_or(0)),
        rate: player.rate().await.unwrap_or(1.0).max(0.0),
        volume: player.volume().await.unwrap_or(1.0).clamp(0.0, 1.0),
        can_go_previous: player.can_go_previous().await.unwrap_or(false),
        can_go_next: player.can_go_next().await.unwrap_or(false),
        can_seek: player.can_seek().await.unwrap_or(false),
        synced_at: Instant::now(),
    })
}

async fn read_identity(connection: &Connection, service: &str) -> Result<String> {
    let proxy = MediaPlayerProxy::builder(connection)
        .destination(service)?
        .path(MPRIS_PATH)?
        .build()
        .await?;
    let identity = proxy.identity().await?;
    let desktop_entry = proxy.desktop_entry().await.ok();
    let process = read_process_command_line(connection, service).await;
    Ok(display_identity(
        &identity,
        desktop_entry.as_deref(),
        process.as_deref(),
    ))
}

async fn read_process_command_line(connection: &Connection, service: &str) -> Option<Vec<u8>> {
    let dbus = zbus::fdo::DBusProxy::new(connection).await.ok()?;
    let bus_name = BusName::try_from(service).ok()?;
    let process_id = dbus.get_connection_unix_process_id(bus_name).await.ok()?;
    fs::read(format!("/proc/{process_id}/cmdline")).ok()
}

fn display_identity(
    identity: &str,
    desktop_entry: Option<&str>,
    command_line: Option<&[u8]>,
) -> String {
    if command_line.is_some_and(|command| {
        String::from_utf8_lossy(command)
            .to_ascii_lowercase()
            .contains("pear-desktop")
    }) {
        return "Pear Desktop".into();
    }

    if !identity.contains('.') {
        return identity.into();
    }

    let candidate = desktop_entry
        .filter(|entry| !entry.trim().is_empty())
        .or_else(|| identity.rsplit('.').next())
        .unwrap_or(identity);
    candidate
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(title_case_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_case_word(word: &str) -> String {
    if word.eq_ignore_ascii_case("youtube") {
        return "YouTube".into();
    }
    let mut characters = word.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };
    first.to_uppercase().chain(characters).collect()
}

async fn apply_command(
    player: &PlayerProxy<'_>,
    snapshot: &PlayerSnapshot,
    command: PlayerCommand,
) -> Result<()> {
    match command {
        PlayerCommand::Previous if snapshot.can_go_previous => player.previous().await?,
        PlayerCommand::TogglePlayback => player.play_pause().await?,
        PlayerCommand::Next if snapshot.can_go_next => player.next().await?,
        PlayerCommand::Seek(ratio) if snapshot.can_seek && !snapshot.duration.is_zero() => {
            let Some(track_id) = snapshot.track_id.as_deref() else {
                return Ok(());
            };
            let track_id = ObjectPath::try_from(track_id)?;
            let position = (snapshot.duration.as_micros() as f64 * ratio.clamp(0.0, 1.0))
                .min(i64::MAX as f64) as i64;
            player.set_position(track_id, position).await?;
        }
        _ => {}
    }
    Ok(())
}

fn metadata_string(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(|value| String::try_from(value.clone()).ok())
        .or_else(|| {
            metadata
                .get(key)
                .and_then(|value| Str::try_from(value.clone()).ok())
                .map(String::from)
        })
}

fn metadata_strings(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<Vec<String>> {
    metadata
        .get(key)
        .and_then(|value| Vec::<String>::try_from(value.clone()).ok())
}

fn metadata_i64(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<i64> {
    metadata
        .get(key)
        .and_then(|value| i64::try_from(value.clone()).ok())
}

fn metadata_object_path(metadata: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(|value| OwnedObjectPath::try_from(value.clone()).ok())
        .map(|path| path.to_string())
}

fn microseconds(value: i64) -> Duration {
    Duration::from_micros(value.max(0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::Value;

    fn owned(value: Value<'_>) -> OwnedValue {
        OwnedValue::try_from(value).unwrap()
    }

    #[test]
    fn reads_standard_metadata_types() {
        let metadata = HashMap::from([
            ("xesam:title".into(), owned(Value::from("Glass Signal"))),
            (
                "xesam:artist".into(),
                owned(Value::from(vec!["Aster", "Vale"])),
            ),
            ("mpris:length".into(), owned(Value::from(42_000_000_i64))),
        ]);

        assert_eq!(
            metadata_string(&metadata, "xesam:title").as_deref(),
            Some("Glass Signal")
        );
        assert_eq!(
            metadata_strings(&metadata, "xesam:artist").unwrap(),
            ["Aster", "Vale"]
        );
        assert_eq!(
            microseconds(metadata_i64(&metadata, "mpris:length").unwrap()),
            Duration::from_secs(42)
        );
    }

    #[test]
    fn negative_positions_become_zero() {
        assert_eq!(microseconds(-1), Duration::ZERO);
    }

    #[test]
    fn identifies_pear_desktop_from_its_process() {
        assert_eq!(
            display_identity(
                "com.github.th-ch.youtube-music",
                None,
                Some(b"/usr/lib/electron/electron\0/usr/lib/pear-desktop/app.asar")
            ),
            "Pear Desktop"
        );
    }

    #[test]
    fn humanizes_reverse_dns_identity() {
        assert_eq!(
            display_identity("com.github.th-ch.youtube-music", None, None),
            "YouTube Music"
        );
    }
}
