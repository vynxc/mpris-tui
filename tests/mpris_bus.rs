use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use mpris_tui::{
    model::{Playback, ProviderState},
    mpris::{monitor, read_player, select_service, send_command, PlayerCommand},
};
use tokio::{
    sync::{mpsc, watch},
    time::timeout,
};
use zbus::{
    connection::Builder,
    interface,
    zvariant::{ObjectPath, OwnedValue, Value},
};

const SERVICE: &str = "org.mpris.MediaPlayer2.TestHarness";
const PATH: &str = "/org/mpris/MediaPlayer2";

struct MockRoot;

#[interface(name = "org.mpris.MediaPlayer2")]
impl MockRoot {
    #[zbus(property)]
    fn identity(&self) -> &str {
        "Test Harness"
    }
}

struct MockPlayer {
    actions: Arc<Mutex<Vec<String>>>,
    position: Arc<AtomicI64>,
}

#[interface(name = "org.mpris.MediaPlayer2.Player")]
impl MockPlayer {
    #[zbus(property)]
    fn playback_status(&self) -> &str {
        "Playing"
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedValue> {
        HashMap::from([
            (
                "xesam:title".into(),
                OwnedValue::try_from(Value::from("Mock Signal")).unwrap(),
            ),
            (
                "xesam:artist".into(),
                OwnedValue::try_from(Value::from(vec!["Fixture Artist"])).unwrap(),
            ),
            (
                "xesam:album".into(),
                OwnedValue::try_from(Value::from("Fixture Album")).unwrap(),
            ),
            (
                "mpris:length".into(),
                OwnedValue::try_from(Value::from(180_000_000_i64)).unwrap(),
            ),
            (
                "mpris:artUrl".into(),
                OwnedValue::try_from(Value::from("file:///tmp/mock-cover.png")).unwrap(),
            ),
            (
                "mpris:trackid".into(),
                OwnedValue::try_from(Value::from(
                    ObjectPath::try_from("/org/mpris/MediaPlayer2/TrackList/mock").unwrap(),
                ))
                .unwrap(),
            ),
        ])
    }

    #[zbus(property)]
    fn position(&self) -> i64 {
        self.position.load(Ordering::Relaxed)
    }

    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn volume(&self) -> f64 {
        0.64
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        true
    }

    fn previous(&self) {
        self.actions.lock().unwrap().push("previous".into());
    }

    fn play_pause(&self) {
        self.actions.lock().unwrap().push("toggle".into());
    }

    fn next(&self) {
        self.actions.lock().unwrap().push("next".into());
    }

    fn set_position(&self, track_id: ObjectPath<'_>, position: i64) {
        self.actions
            .lock()
            .unwrap()
            .push(format!("seek:{track_id}:{position}"));
    }
}

#[tokio::test]
async fn discovers_and_reads_a_mock_mpris_player() {
    let actions = Arc::new(Mutex::new(Vec::new()));
    let position = Arc::new(AtomicI64::new(45_000_000));
    let connection = Builder::session()
        .unwrap()
        .name(SERVICE)
        .unwrap()
        .serve_at(PATH, MockRoot)
        .unwrap()
        .serve_at(
            PATH,
            MockPlayer {
                actions: Arc::clone(&actions),
                position: Arc::clone(&position),
            },
        )
        .unwrap()
        .build()
        .await
        .unwrap();

    assert_eq!(
        select_service(&connection, "test harness").await.unwrap(),
        Some(SERVICE.into())
    );
    assert_eq!(
        select_service(&connection, "auto").await.unwrap(),
        Some(SERVICE.into())
    );

    let snapshot = read_player(&connection, SERVICE).await.unwrap();
    assert_eq!(snapshot.identity, "Test Harness");
    assert_eq!(snapshot.title, "Mock Signal");
    assert_eq!(snapshot.artists, ["Fixture Artist"]);
    assert_eq!(snapshot.album, "Fixture Album");
    assert_eq!(
        snapshot.art_url.as_deref(),
        Some("file:///tmp/mock-cover.png")
    );
    assert_eq!(
        snapshot.track_id.as_deref(),
        Some("/org/mpris/MediaPlayer2/TrackList/mock")
    );
    assert_eq!(snapshot.playback, Playback::Playing);
    assert_eq!(snapshot.duration, Duration::from_secs(180));
    assert_eq!(snapshot.position, Duration::from_secs(45));
    assert_eq!(snapshot.volume, 0.64);
    assert!(snapshot.can_go_previous);
    assert!(snapshot.can_go_next);
    assert!(snapshot.can_seek);

    send_command(&connection, SERVICE, &snapshot, PlayerCommand::Previous)
        .await
        .unwrap();
    send_command(
        &connection,
        SERVICE,
        &snapshot,
        PlayerCommand::TogglePlayback,
    )
    .await
    .unwrap();
    send_command(&connection, SERVICE, &snapshot, PlayerCommand::Next)
        .await
        .unwrap();
    send_command(&connection, SERVICE, &snapshot, PlayerCommand::Seek(0.5))
        .await
        .unwrap();

    assert_eq!(
        *actions.lock().unwrap(),
        [
            "previous",
            "toggle",
            "next",
            "seek:/org/mpris/MediaPlayer2/TrackList/mock:90000000"
        ]
    );

    let (state_sender, mut state_receiver) = watch::channel(ProviderState::Connecting);
    let (_command_sender, command_receiver) = mpsc::channel(4);
    let monitor_task = tokio::spawn(monitor(
        "test harness".into(),
        state_sender,
        command_receiver,
    ));

    timeout(Duration::from_secs(2), async {
        loop {
            state_receiver.changed().await.unwrap();
            if let ProviderState::Ready(snapshot) = &*state_receiver.borrow() {
                assert_eq!(snapshot.position, Duration::from_secs(45));
                break;
            }
        }
    })
    .await
    .expect("monitor did not publish its initial position");

    position.store(90_000_000, Ordering::Relaxed);
    timeout(Duration::from_secs(4), async {
        loop {
            state_receiver.changed().await.unwrap();
            if let ProviderState::Ready(snapshot) = &*state_receiver.borrow() {
                if snapshot.position == Duration::from_secs(90) {
                    break;
                }
            }
        }
    })
    .await
    .expect("monitor reused a cached MPRIS position");

    monitor_task.abort();
}
