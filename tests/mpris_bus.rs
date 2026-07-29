use std::{collections::HashMap, time::Duration};

use mpris_tui::{
    model::Playback,
    mpris::{read_player, select_service},
};
use zbus::{
    connection::Builder,
    interface,
    zvariant::{OwnedValue, Value},
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

struct MockPlayer;

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
        false
    }
}

#[tokio::test]
async fn discovers_and_reads_a_mock_mpris_player() {
    let connection = Builder::session()
        .unwrap()
        .name(SERVICE)
        .unwrap()
        .serve_at(PATH, MockRoot)
        .unwrap()
        .serve_at(PATH, MockPlayer)
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
    assert_eq!(snapshot.playback, Playback::Playing);
    assert_eq!(snapshot.duration, Duration::from_secs(180));
    assert_eq!(snapshot.position, Duration::from_secs(45));
    assert_eq!(snapshot.volume, 0.64);
    assert!(snapshot.can_go_previous);
    assert!(!snapshot.can_go_next);
}
