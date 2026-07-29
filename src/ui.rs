use std::{str::FromStr, time::Instant};

use ratatui::{
    layout::{Alignment, Constraint, Layout as RatatuiLayout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{LineGauge, Paragraph, Wrap},
    Frame,
};

use crate::{
    model::{PlayerSnapshot, ProviderState},
    theme::Theme,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Hero,
    Wide,
    Compact,
    Minimal,
}

impl FromStr for Layout {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "hero" => Ok(Self::Hero),
            "wide" => Ok(Self::Wide),
            "compact" => Ok(Self::Compact),
            "minimal" => Ok(Self::Minimal),
            value => Err(format!("unknown layout `{value}`")),
        }
    }
}

pub fn render(frame: &mut Frame<'_>, state: &ProviderState, layout: Layout, theme: Theme) {
    match state {
        ProviderState::Ready(snapshot) => match responsive_layout(layout, frame.area()) {
            Layout::Hero => render_hero(frame, frame.area(), snapshot, theme),
            Layout::Wide => render_wide(frame, frame.area(), snapshot, theme),
            Layout::Compact => render_compact(frame, frame.area(), snapshot, theme),
            Layout::Minimal => render_minimal(frame, frame.area(), snapshot, theme),
        },
        ProviderState::Connecting => {
            render_empty(frame, "CONNECTING", "Looking for MPRIS players", theme)
        }
        ProviderState::Unavailable(message) => render_empty(frame, "NO PLAYER", message, theme),
    }
}

fn responsive_layout(requested: Layout, area: Rect) -> Layout {
    if area.width < 46 || area.height < 8 {
        Layout::Minimal
    } else if area.width < 76 || area.height < 16 {
        Layout::Compact
    } else {
        requested
    }
}

fn render_hero(frame: &mut Frame<'_>, area: Rect, snapshot: &PlayerSnapshot, theme: Theme) {
    let area = inset(area, 2, 1);
    let [visual, content] =
        RatatuiLayout::horizontal([Constraint::Percentage(34), Constraint::Percentage(66)])
            .areas(area);
    render_visualizer(frame, visual, snapshot, theme);

    let content = inset(content, 2, 1);
    let [status, spacer, title, artist, album, progress, time] = RatatuiLayout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(content);

    render_status(frame, status, snapshot, theme);
    frame.render_widget(
        Paragraph::new(snapshot.title.as_str())
            .style(
                Style::default()
                    .fg(theme.bright)
                    .add_modifier(Modifier::BOLD),
            )
            .wrap(Wrap { trim: true }),
        title,
    );
    frame.render_widget(
        Paragraph::new(snapshot.artist_line())
            .style(Style::default().fg(theme.accent))
            .wrap(Wrap { trim: true }),
        artist,
    );
    frame.render_widget(
        Paragraph::new(snapshot.album.as_str()).style(Style::default().fg(theme.muted)),
        album,
    );
    render_progress(frame, progress, snapshot, theme);
    render_time(frame, time, snapshot, theme);
    let _ = spacer;
}

fn render_wide(frame: &mut Frame<'_>, area: Rect, snapshot: &PlayerSnapshot, theme: Theme) {
    let area = inset(area, 3, 2);
    let [header, body, progress, time] = RatatuiLayout::vertical([
        Constraint::Length(1),
        Constraint::Min(4),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    render_status(frame, header, snapshot, theme);

    let [visual, text] =
        RatatuiLayout::horizontal([Constraint::Length(22), Constraint::Min(20)]).areas(body);
    render_visualizer(frame, visual, snapshot, theme);
    let text = inset(text, 2, 1);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                snapshot.title.as_str(),
                Style::default()
                    .fg(theme.bright)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::styled(snapshot.artist_line(), Style::default().fg(theme.accent)),
            Line::styled(snapshot.album.as_str(), Style::default().fg(theme.muted)),
        ])
        .wrap(Wrap { trim: true }),
        text,
    );
    render_progress(frame, progress, snapshot, theme);
    render_time(frame, time, snapshot, theme);
}

fn render_compact(frame: &mut Frame<'_>, area: Rect, snapshot: &PlayerSnapshot, theme: Theme) {
    let area = inset(area, 2, 1);
    let [status, title, artist, spacer, progress, time] = RatatuiLayout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    render_status(frame, status, snapshot, theme);
    frame.render_widget(
        Paragraph::new(snapshot.title.as_str())
            .style(
                Style::default()
                    .fg(theme.bright)
                    .add_modifier(Modifier::BOLD),
            )
            .wrap(Wrap { trim: true }),
        title,
    );
    frame.render_widget(
        Paragraph::new(snapshot.artist_line()).style(Style::default().fg(theme.accent)),
        artist,
    );
    render_progress(frame, progress, snapshot, theme);
    render_time(frame, time, snapshot, theme);
    let _ = spacer;
}

fn render_minimal(frame: &mut Frame<'_>, area: Rect, snapshot: &PlayerSnapshot, theme: Theme) {
    let position = snapshot.position_now(Instant::now());
    let line = Line::from(vec![
        Span::styled(
            format!("{}  ", snapshot.playback.symbol()),
            Style::default().fg(theme.accent),
        ),
        Span::styled(
            snapshot.title.as_str(),
            Style::default()
                .fg(theme.bright)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  —  {}  ", snapshot.artist_line()),
            Style::default().fg(theme.text),
        ),
        Span::styled(
            format!(
                "{} / {}",
                format_duration(position),
                format_duration(snapshot.duration)
            ),
            Style::default().fg(theme.muted),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(line)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_status(frame: &mut Frame<'_>, area: Rect, snapshot: &PlayerSnapshot, theme: Theme) {
    let controls = format!(
        "{}  {}  {}",
        if snapshot.can_go_previous {
            "◂"
        } else {
            "·"
        },
        snapshot.playback.symbol(),
        if snapshot.can_go_next { "▸" } else { "·" },
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                snapshot.playback.label(),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  /  {}", snapshot.identity.to_uppercase()),
                Style::default().fg(theme.muted),
            ),
            Span::styled(format!("    {controls}"), Style::default().fg(theme.text)),
        ])),
        area,
    );
}

fn render_visualizer(frame: &mut Frame<'_>, area: Rect, snapshot: &PlayerSnapshot, theme: Theme) {
    let seed = snapshot.title.bytes().fold(0_u64, |value, byte| {
        value.wrapping_mul(33) + u64::from(byte)
    });
    let phase = snapshot.position_now(Instant::now()).as_millis() as u64 / 220;
    let width = area.width.saturating_sub(2).min(28) as usize;
    let bars = (0..width)
        .map(|index| {
            let level = (seed
                .wrapping_add((index as u64 + phase) * 17)
                .rotate_left((index % 13) as u32)
                % 8) as usize;
            ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"][level]
        })
        .collect::<String>();
    let [top, center, bottom] = RatatuiLayout::vertical([
        Constraint::Min(1),
        Constraint::Length(3),
        Constraint::Min(1),
    ])
    .areas(area);
    frame.render_widget(
        Paragraph::new(bars)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.accent)),
        center,
    );
    let _ = (top, bottom);
}

fn render_progress(frame: &mut Frame<'_>, area: Rect, snapshot: &PlayerSnapshot, theme: Theme) {
    frame.render_widget(
        LineGauge::default()
            .ratio(snapshot.progress(Instant::now()))
            .filled_style(Style::default().fg(theme.accent))
            .unfilled_style(Style::default().fg(theme.faint))
            .filled_symbol("━")
            .unfilled_symbol("─")
            .label(""),
        area,
    );
}

fn render_time(frame: &mut Frame<'_>, area: Rect, snapshot: &PlayerSnapshot, theme: Theme) {
    let position = snapshot.position_now(Instant::now());
    let [left, right] =
        RatatuiLayout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(area);
    frame.render_widget(
        Paragraph::new(format_duration(position)).style(Style::default().fg(theme.text)),
        left,
    );
    frame.render_widget(
        Paragraph::new(format_duration(snapshot.duration))
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.muted)),
        right,
    );
}

fn render_empty(frame: &mut Frame<'_>, heading: &str, message: &str, theme: Theme) {
    let area = inset(frame.area(), 2, 1);
    let [top, heading_area, message_area, bottom] = RatatuiLayout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Min(1),
    ])
    .areas(area);
    frame.render_widget(
        Paragraph::new(heading).alignment(Alignment::Center).style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        heading_area,
    );
    frame.render_widget(
        Paragraph::new(message)
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.muted))
            .wrap(Wrap { trim: true }),
        message_area,
    );
    let _ = (top, bottom);
}

fn inset(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(horizontal).min(area.right()),
        y: area.y.saturating_add(vertical).min(area.bottom()),
        width: area.width.saturating_sub(horizontal.saturating_mul(2)),
        height: area.height.saturating_sub(vertical.saturating_mul(2)),
    }
}

fn format_duration(duration: std::time::Duration) -> String {
    let seconds = duration.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, Terminal};

    use super::*;

    fn rendered(width: u16, height: u16, layout: Layout) -> String {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let snapshot = PlayerSnapshot::demo();
        let theme = Theme::from_accent(None).unwrap();
        terminal
            .draw(|frame| {
                render(
                    frame,
                    &ProviderState::Ready(snapshot.clone()),
                    layout,
                    theme,
                )
            })
            .unwrap();
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn hero_renders_now_playing_content() {
        let output = rendered(100, 28, Layout::Hero);
        assert!(output.contains("Afterglow Circuit"));
        assert!(output.contains("Nocturne Assembly"));
        assert!(output.contains("PLAYING"));
    }

    #[test]
    fn narrow_layout_falls_back_without_losing_title() {
        let output = rendered(40, 4, Layout::Hero);
        assert!(output.contains("Afterglow Circuit"));
    }

    #[test]
    fn every_public_layout_renders() {
        for layout in [Layout::Hero, Layout::Wide, Layout::Compact, Layout::Minimal] {
            let output = rendered(100, 28, layout);
            assert!(output.contains("Afterglow Circuit"));
        }
    }
}
