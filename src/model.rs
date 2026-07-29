use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Playback {
    Playing,
    Paused,
    Stopped,
}

impl Playback {
    pub fn from_mpris(value: &str) -> Self {
        match value {
            "Playing" => Self::Playing,
            "Paused" => Self::Paused,
            _ => Self::Stopped,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Playing => "PLAYING",
            Self::Paused => "PAUSED",
            Self::Stopped => "STOPPED",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Playing => "▶",
            Self::Paused => "Ⅱ",
            Self::Stopped => "■",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlayerSnapshot {
    pub service: String,
    pub identity: String,
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub playback: Playback,
    pub duration: Duration,
    pub position: Duration,
    pub rate: f64,
    pub volume: f64,
    pub can_go_previous: bool,
    pub can_go_next: bool,
    pub synced_at: Instant,
}

impl PlayerSnapshot {
    pub fn demo() -> Self {
        Self {
            service: "org.mpris.MediaPlayer2.demo".into(),
            identity: "MPRIS TUI".into(),
            title: "Afterglow Circuit".into(),
            artists: vec!["Nocturne Assembly".into()],
            album: "Signals in the Static".into(),
            playback: Playback::Playing,
            duration: Duration::from_secs(4 * 60 + 18),
            position: Duration::from_secs(2 * 60 + 7),
            rate: 1.0,
            volume: 0.72,
            can_go_previous: true,
            can_go_next: true,
            synced_at: Instant::now(),
        }
    }

    pub fn artist_line(&self) -> String {
        if self.artists.is_empty() {
            "Unknown artist".into()
        } else {
            self.artists.join(", ")
        }
    }

    pub fn position_now(&self, now: Instant) -> Duration {
        let position = if self.playback == Playback::Playing && self.rate > 0.0 {
            self.position.saturating_add(Duration::from_secs_f64(
                now.saturating_duration_since(self.synced_at).as_secs_f64() * self.rate,
            ))
        } else {
            self.position
        };
        if self.duration.is_zero() {
            position
        } else {
            position.min(self.duration)
        }
    }

    pub fn progress(&self, now: Instant) -> f64 {
        if self.duration.is_zero() {
            0.0
        } else {
            (self.position_now(now).as_secs_f64() / self.duration.as_secs_f64()).clamp(0.0, 1.0)
        }
    }
}

#[derive(Debug, Clone)]
pub enum ProviderState {
    Connecting,
    Unavailable(String),
    Ready(PlayerSnapshot),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrapolates_playing_position() {
        let mut snapshot = PlayerSnapshot::demo();
        snapshot.position = Duration::from_secs(10);
        snapshot.duration = Duration::from_secs(100);
        snapshot.synced_at = Instant::now();

        assert_eq!(
            snapshot.position_now(snapshot.synced_at + Duration::from_secs(5)),
            Duration::from_secs(15)
        );
    }

    #[test]
    fn paused_position_does_not_move() {
        let mut snapshot = PlayerSnapshot::demo();
        snapshot.playback = Playback::Paused;
        let now = snapshot.synced_at + Duration::from_secs(20);
        assert_eq!(snapshot.position_now(now), snapshot.position);
    }

    #[test]
    fn progress_is_bounded() {
        let mut snapshot = PlayerSnapshot::demo();
        snapshot.position = Duration::from_secs(100);
        snapshot.duration = Duration::from_secs(10);
        assert_eq!(snapshot.progress(snapshot.synced_at), 1.0);
        assert_eq!(
            snapshot.position_now(snapshot.synced_at),
            Duration::from_secs(10)
        );
    }
}
