use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use mpris_tui::{
    model::Playback,
    mpris::{read_player, select_service, send_command, PlayerCommand},
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
        45_000_000
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
}
