use std::{
    env, io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, MouseButton, MouseEventKind,
    },
    execute,
    style::force_color_output,
};
use futures_util::StreamExt;
use mpris_tui::{
    artwork::ArtworkCache,
    cli::Config,
    model::{PlayerSnapshot, ProviderState},
    mpris::{self, PlayerCommand},
    theme::Theme,
    ui::{self, HitRegions, UiAction},
};
use signal_hook::consts::{SIGINT, SIGTERM};
use tokio::{
    sync::{mpsc, watch},
    time::{interval, sleep, MissedTickBehavior},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Color carries the artwork itself, so NO_COLOR cannot be honored without
    // replacing the image with an unreadable solid block.
    force_color_output(true);

    let config = match Config::parse(env::args().skip(1)) {
        Ok(config) => config,
        Err(message) if message.starts_with("mpris-tui —") || message.starts_with("mpris-tui ") =>
        {
            println!("{message}");
            return Ok(());
        }
        Err(message) => return Err(anyhow!(message)),
    };
    let theme = Theme::from_accent(config.accent.as_deref()).map_err(|error| anyhow!(error))?;
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGINT, Arc::clone(&stop))?;
    signal_hook::flag::register(SIGTERM, Arc::clone(&stop))?;

    let (sender, receiver) = watch::channel(ProviderState::Connecting);
    let (command_sender, command_receiver) = mpsc::channel(16);
    let provider = if config.demo {
        let sender = sender.clone();
        Some(tokio::spawn(async move {
            run_demo(sender, command_receiver).await;
        }))
    } else {
        let selector = config.player.clone();
        Some(tokio::spawn(async move {
            mpris::monitor(selector, sender, command_receiver).await;
        }))
    };

    let mut terminal = ratatui::init();
    let mouse_enabled = config.mouse && !config.once;
    if mouse_enabled {
        execute!(io::stdout(), EnableMouseCapture)?;
    }
    let result = run_ui(
        &mut terminal,
        receiver,
        command_sender,
        &config,
        theme,
        Arc::clone(&stop),
    )
    .await;
    if mouse_enabled {
        let _ = execute!(io::stdout(), DisableMouseCapture);
    }
    ratatui::restore();

    if let Some(provider) = provider {
        provider.abort();
    }
    result
}

async fn run_ui(
    terminal: &mut ratatui::DefaultTerminal,
    mut receiver: watch::Receiver<ProviderState>,
    command_sender: mpsc::Sender<PlayerCommand>,
    config: &Config,
    theme: Theme,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    let frame_interval = Duration::from_secs_f64(1.0 / f64::from(config.frames_per_second.max(1)));
    let mut ticker = interval(frame_interval);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    ticker.tick().await;
    let mut events = EventStream::new();
    let mut artwork = ArtworkCache::default();

    if config.once && matches!(*receiver.borrow(), ProviderState::Connecting) {
        let _ = tokio::time::timeout(Duration::from_secs(3), receiver.changed()).await;
    }

    loop {
        let state = receiver.borrow().clone();
        artwork.update(match &state {
            ProviderState::Ready(snapshot) => snapshot.art_url.as_deref(),
            _ => None,
        });
        let mut hits = HitRegions::default();
        terminal
            .draw(|frame| {
                let (width, height) = ui::artwork_size(config.layout, frame.area());
                let image = artwork.render_for(width, height);
                hits = ui::render(frame, &state, config.layout, theme, image);
            })
            .context("could not draw the terminal")?;

        if config.once || stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        tokio::select! {
            _ = ticker.tick() => {}
            changed = receiver.changed() => {
                if changed.is_err() {
                    return Ok(());
                }
            }
            event = events.next() => {
                let Some(event) = event.transpose().context("could not read terminal input")? else {
                    return Ok(());
                };
                handle_event(event, hits, &command_sender, config.mouse).await?;
            }
        }
    }
}

async fn handle_event(
    event: Event,
    hits: HitRegions,
    commands: &mpsc::Sender<PlayerCommand>,
    mouse_enabled: bool,
) -> Result<()> {
    let action = match event {
        Event::Mouse(mouse)
            if mouse_enabled && matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) =>
        {
            hits.action_at(mouse.column, mouse.row)
        }
        _ => None,
    };

    if let Some(action) = action {
        let command = match action {
            UiAction::Previous => PlayerCommand::Previous,
            UiAction::TogglePlayback => PlayerCommand::TogglePlayback,
            UiAction::Next => PlayerCommand::Next,
            UiAction::Seek(ratio) => PlayerCommand::Seek(ratio),
        };
        commands
            .send(command)
            .await
            .context("media player command channel closed")?;
    }
    Ok(())
}

async fn run_demo(
    sender: watch::Sender<ProviderState>,
    mut commands: mpsc::Receiver<PlayerCommand>,
) {
    let mut snapshot = PlayerSnapshot::demo();
    loop {
        let now = Instant::now();
        snapshot.position = snapshot.position_now(now);
        snapshot.synced_at = now;
        sender.send_replace(ProviderState::Ready(Box::new(snapshot.clone())));
        tokio::select! {
            _ = sleep(Duration::from_secs(1)) => {}
            command = commands.recv() => match command {
                Some(PlayerCommand::TogglePlayback) => {
                    snapshot.playback = if snapshot.playback == mpris_tui::model::Playback::Playing {
                        mpris_tui::model::Playback::Paused
                    } else {
                        mpris_tui::model::Playback::Playing
                    };
                }
                Some(PlayerCommand::Seek(ratio)) => {
                    snapshot.position = snapshot.duration.mul_f64(ratio.clamp(0.0, 1.0));
                }
                Some(PlayerCommand::Previous) => snapshot.position = Duration::ZERO,
                Some(PlayerCommand::Next) => snapshot.position = Duration::ZERO,
                None => return,
            }
        }
    }
}
