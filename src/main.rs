use std::{
    env,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use mpris_tui::{
    cli::Config,
    model::{PlayerSnapshot, ProviderState},
    mpris,
    theme::Theme,
    ui,
};
use signal_hook::consts::{SIGINT, SIGTERM};
use tokio::{sync::watch, time::sleep};

#[tokio::main]
async fn main() -> Result<()> {
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
    let provider = if config.demo {
        let sender = sender.clone();
        Some(tokio::spawn(async move {
            run_demo(sender).await;
        }))
    } else {
        let selector = config.player.clone();
        Some(tokio::spawn(async move {
            mpris::monitor(selector, sender).await;
        }))
    };

    let mut terminal = ratatui::init();
    let result = run_ui(&mut terminal, receiver, &config, theme, Arc::clone(&stop)).await;
    ratatui::restore();

    if let Some(provider) = provider {
        provider.abort();
    }
    result
}

async fn run_ui(
    terminal: &mut ratatui::DefaultTerminal,
    mut receiver: watch::Receiver<ProviderState>,
    config: &Config,
    theme: Theme,
    stop: Arc<AtomicBool>,
) -> Result<()> {
    let frame_interval = Duration::from_secs_f64(1.0 / f64::from(config.frames_per_second.max(1)));

    if config.once && matches!(*receiver.borrow(), ProviderState::Connecting) {
        let _ = tokio::time::timeout(Duration::from_secs(3), receiver.changed()).await;
    }

    loop {
        let state = receiver.borrow().clone();
        terminal
            .draw(|frame| ui::render(frame, &state, config.layout, theme))
            .context("could not draw the terminal")?;

        if config.once || stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        sleep(frame_interval).await;
    }
}

async fn run_demo(sender: watch::Sender<ProviderState>) {
    let mut snapshot = PlayerSnapshot::demo();
    let started = Instant::now();
    loop {
        snapshot.position = Duration::from_secs(127)
            .saturating_add(started.elapsed())
            .min(snapshot.duration);
        snapshot.synced_at = Instant::now();
        sender.send_replace(ProviderState::Ready(snapshot.clone()));
        sleep(Duration::from_secs(1)).await;
    }
}
