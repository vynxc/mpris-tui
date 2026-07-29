use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use tokio::{
    sync::watch,
    time::{interval, sleep, MissedTickBehavior},
};
use zbus::{
    proxy,
    zvariant::{OwnedValue, Str},
    Connection,
};

use crate::model::{Playback, PlayerSnapshot, ProviderState};

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const POSITION_RESYNC: Duration = Duration::from_secs(2);
const PLAYER_RESELECT: Duration = Duration::from_secs(3);
const DISCOVERY_RETRY: Duration = Duration::from_secs(1);

#[proxy(
    interface = "org.mpris.MediaPlayer2",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait MediaPlayer {
    #[zbus(property)]
    fn identity(&self) -> zbus::Result<String>;
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

    #[zbus(signal)]
    fn seeked(&self, position: i64) -> zbus::Result<()>;
}

pub async fn monitor(selector: String, sender: watch::Sender<ProviderState>) {
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

        if let Err(error) = watch_service(&connection, &service, &selector, &sender).await {
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
    let mut seeked = player.receive_seeked().await?;
    let mut resync = interval(POSITION_RESYNC);
    resync.set_missed_tick_behavior(MissedTickBehavior::Delay);
    resync.tick().await;
    let mut reselect = interval(PLAYER_RESELECT);
    reselect.set_missed_tick_behavior(MissedTickBehavior::Delay);
    reselect.tick().await;

    loop {
        sender.send_replace(ProviderState::Ready(
            read_player_with_proxy(service, &identity, &player).await?,
        ));

        tokio::select! {
            change = metadata_changes.next() => require_signal(change, service)?,
            change = playback_changes.next() => require_signal(change, service)?,
            change = rate_changes.next() => require_signal(change, service)?,
            change = volume_changes.next() => require_signal(change, service)?,
            change = previous_changes.next() => require_signal(change, service)?,
            change = next_changes.next() => require_signal(change, service)?,
            change = seeked.next() => require_signal(change, service)?,
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
        synced_at: Instant::now(),
    })
}

async fn read_identity(connection: &Connection, service: &str) -> Result<String> {
    let proxy = MediaPlayerProxy::builder(connection)
        .destination(service)?
        .path(MPRIS_PATH)?
        .build()
        .await?;
    Ok(proxy.identity().await?)
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
}
