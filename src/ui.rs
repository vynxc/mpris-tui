use std::{str::FromStr, time::Instant};

use ratatui::{
    layout::{Alignment, Constraint, Layout as RatatuiLayout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{LineGauge, Paragraph, Wrap},
    Frame,
};

use crate::{
    artwork::TerminalArtwork,
    model::{Playback, PlayerSnapshot, ProviderState},
    theme::Theme,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Vertical,
    Hero,
    Wide,
    Compact,
    Minimal,
}

impl FromStr for Layout {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "vertical" => Ok(Self::Vertical),
            "hero" => Ok(Self::Hero),
            "wide" => Ok(Self::Wide),
            "compact" => Ok(Self::Compact),
            "minimal" => Ok(Self::Minimal),
            value => Err(format!("unknown layout `{value}`")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiAction {
    Previous,
    TogglePlayback,
    Next,
    Seek(f64),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HitRegions {
    previous: Option<Rect>,
    toggle: Option<Rect>,
    next: Option<Rect>,
    seek: Option<Rect>,
}

impl HitRegions {
    pub fn action_at(self, column: u16, row: u16) -> Option<UiAction> {
        if self
            .previous
            .is_some_and(|area| contains(area, column, row))
        {
            return Some(UiAction::Previous);
        }
        if self.toggle.is_some_and(|area| contains(area, column, row)) {
            return Some(UiAction::TogglePlayback);
        }
        if self.next.is_some_and(|area| contains(area, column, row)) {
            return Some(UiAction::Next);
        }
        self.seek
            .filter(|area| contains(*area, column, row))
            .map(|area| {
                let width = area.width.saturating_sub(1).max(1);
                UiAction::Seek(f64::from(column.saturating_sub(area.x)) / f64::from(width))
            })
    }
}

pub fn artwork_size(layout: Layout, area: Rect) -> (u16, u16) {
    match responsive_layout(layout, area) {
        Layout::Vertical | Layout::Hero => (
            area.width.saturating_sub(6).min(38),
            area.height.saturating_sub(13).min(15),
        ),
        Layout::Wide => (area.width.min(28), area.height.saturating_sub(6).min(16)),
        Layout::Compact | Layout::Minimal => (0, 0),
    }
}

pub fn render(
    frame: &mut Frame<'_>,
    state: &ProviderState,
    layout: Layout,
    theme: Theme,
    artwork: Option<&TerminalArtwork>,
) -> HitRegions {
    match state {
        ProviderState::Ready(snapshot) => match responsive_layout(layout, frame.area()) {
            Layout::Vertical | Layout::Hero => {
                render_vertical(frame, frame.area(), snapshot, theme, artwork)
            }
            Layout::Wide => render_wide(frame, frame.area(), snapshot, theme, artwork),
            Layout::Compact => render_compact(frame, frame.area(), snapshot, theme),
            Layout::Minimal => render_minimal(frame, frame.area(), snapshot, theme),
        },
        ProviderState::Connecting => {
            render_empty(frame, "CONNECTING", "Looking for media players", theme);
            HitRegions::default()
        }
        ProviderState::Unavailable(message) => {
            render_empty(frame, "NO PLAYER", message, theme);
            HitRegions::default()
        }
    }
}

fn responsive_layout(requested: Layout, area: Rect) -> Layout {
    if area.width < 42 || area.height < 7 {
        Layout::Minimal
    } else if area.width < 56 || area.height < 17 {
        Layout::Compact
    } else {
        requested
    }
}

fn render_vertical(
    frame: &mut Frame<'_>,
    area: Rect,
    snapshot: &PlayerSnapshot,
    theme: Theme,
    artwork: Option<&TerminalArtwork>,
) -> HitRegions {
    let area = centered_width(inset(area, 2, 1), 72);
    let [status, art, title, artist, album, progress, controls] = RatatuiLayout::vertical([
        Constraint::Length(1),
        Constraint::Min(6),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(3),
    ])
    .spacing(1)
    .areas(area);

    render_status(frame, status, snapshot, theme, true);
    render_artwork(frame, art, artwork, snapshot, theme);
    render_metadata(frame, title, artist, album, snapshot, theme, true);
    let seek = render_progress(frame, progress, snapshot, theme);
    let mut hits = render_controls(frame, controls, snapshot, theme);
    hits.seek = seek;
    hits
}

fn render_wide(
    frame: &mut Frame<'_>,
    area: Rect,
    snapshot: &PlayerSnapshot,
    theme: Theme,
    artwork: Option<&TerminalArtwork>,
) -> HitRegions {
    let area = inset(area, 3, 2);
    let [header, body, progress, controls] = RatatuiLayout::vertical([
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(1),
        Constraint::Length(3),
    ])
    .spacing(1)
    .areas(area);
    render_status(frame, header, snapshot, theme, false);

    let [art, text] =
        RatatuiLayout::horizontal([Constraint::Length(30), Constraint::Min(20)]).areas(body);
    render_artwork(frame, art, artwork, snapshot, theme);
    let text = inset(text, 2, 1);
    let [title, artist, album] = RatatuiLayout::vertical([
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .areas(text);
    render_metadata(frame, title, artist, album, snapshot, theme, false);

    let seek = render_progress(frame, progress, snapshot, theme);
    let mut hits = render_controls(frame, controls, snapshot, theme);
    hits.seek = seek;
    hits
}

fn render_compact(
    frame: &mut Frame<'_>,
    area: Rect,
    snapshot: &PlayerSnapshot,
    theme: Theme,
) -> HitRegions {
    let area = inset(area, 2, 1);
    let [status, title, artist, progress, controls] = RatatuiLayout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .spacing(1)
    .areas(area);
    render_status(frame, status, snapshot, theme, true);
    frame.render_widget(
        Paragraph::new(snapshot.title.as_str())
            .alignment(Alignment::Center)
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
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.accent)),
        artist,
    );
    let seek = render_progress(frame, progress, snapshot, theme);
    let mut hits = render_compact_controls(frame, controls, snapshot, theme);
    hits.seek = seek;
    hits
}

fn render_minimal(
    frame: &mut Frame<'_>,
    area: Rect,
    snapshot: &PlayerSnapshot,
    theme: Theme,
) -> HitRegions {
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
    HitRegions {
        toggle: Some(area),
        ..HitRegions::default()
    }
}

fn render_status(
    frame: &mut Frame<'_>,
    area: Rect,
    snapshot: &PlayerSnapshot,
    theme: Theme,
    centered: bool,
) {
    let line = Line::from(vec![
        Span::styled(
            snapshot.identity.to_uppercase(),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  •  ", Style::default().fg(theme.faint)),
        Span::styled(
            snapshot.playback.label(),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    let paragraph = Paragraph::new(line);
    frame.render_widget(
        if centered {
            paragraph.alignment(Alignment::Center)
        } else {
            paragraph
        },
        area,
    );
}

fn render_metadata(
    frame: &mut Frame<'_>,
    title: Rect,
    artist: Rect,
    album: Rect,
    snapshot: &PlayerSnapshot,
    theme: Theme,
    centered: bool,
) {
    let alignment = if centered {
        Alignment::Center
    } else {
        Alignment::Left
    };
    frame.render_widget(
        Paragraph::new(snapshot.title.as_str())
            .alignment(alignment)
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
            .alignment(alignment)
            .style(Style::default().fg(theme.accent)),
        artist,
    );
    frame.render_widget(
        Paragraph::new(snapshot.album.as_str())
            .alignment(alignment)
            .style(Style::default().fg(theme.muted)),
        album,
    );
}

fn render_artwork(
    frame: &mut Frame<'_>,
    area: Rect,
    artwork: Option<&TerminalArtwork>,
    snapshot: &PlayerSnapshot,
    theme: Theme,
) {
    if let Some(artwork) = artwork {
        frame.render_widget(artwork, area);
        return;
    }

    let monogram = snapshot
        .album
        .chars()
        .find(|character| character.is_alphanumeric())
        .or_else(|| {
            snapshot
                .title
                .chars()
                .find(|character| character.is_alphanumeric())
        })
        .unwrap_or('♪')
        .to_uppercase()
        .collect::<String>();
    let [top, mark, caption, bottom] = RatatuiLayout::vertical([
        Constraint::Min(1),
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(1),
    ])
    .areas(area);
    frame.render_widget(
        Paragraph::new(format!("╭── {monogram} ──╮\n│   ◉   │\n╰───────╯"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.faint)),
        mark,
    );
    frame.render_widget(
        Paragraph::new("artwork unavailable")
            .alignment(Alignment::Center)
            .style(Style::default().fg(theme.faint)),
        caption,
    );
    let _ = (top, bottom);
}

fn render_progress(
    frame: &mut Frame<'_>,
    area: Rect,
    snapshot: &PlayerSnapshot,
    theme: Theme,
) -> Option<Rect> {
    let position = snapshot.position_now(Instant::now());
    let [elapsed, rail, total] = RatatuiLayout::horizontal([
        Constraint::Length(7),
        Constraint::Min(8),
        Constraint::Length(7),
    ])
    .areas(area);
    frame.render_widget(
        Paragraph::new(format_duration(position)).style(Style::default().fg(theme.text)),
        elapsed,
    );
    frame.render_widget(
        LineGauge::default()
            .ratio(snapshot.progress(Instant::now()))
            .filled_style(Style::default().fg(theme.accent))
            .unfilled_style(Style::default().fg(theme.faint))
            .filled_symbol("━")
            .unfilled_symbol("─")
            .label(""),
        rail,
    );
    frame.render_widget(
        Paragraph::new(format_duration(snapshot.duration))
            .alignment(Alignment::Right)
            .style(Style::default().fg(theme.muted)),
        total,
    );
    (snapshot.can_seek && !snapshot.duration.is_zero()).then_some(rail)
}

fn render_controls(
    frame: &mut Frame<'_>,
    area: Rect,
    snapshot: &PlayerSnapshot,
    theme: Theme,
) -> HitRegions {
    let row = Rect {
        y: area.y + area.height.saturating_sub(1) / 2,
        height: 1,
        ..area
    };
    let total_width = 25_u16.min(row.width);
    let x = row.x + row.width.saturating_sub(total_width) / 2;
    let previous = Rect::new(x, row.y, 7.min(total_width), 1);
    let toggle_width = 11.min(total_width.saturating_sub(previous.width));
    let toggle = Rect::new(
        previous.right() + 2,
        row.y,
        toggle_width.saturating_sub(2),
        1,
    );
    let next = Rect::new(
        toggle.right() + 2,
        row.y,
        total_width
            .saturating_sub(previous.width)
            .saturating_sub(toggle_width),
        1,
    );

    render_button(frame, previous, "◀◀", snapshot.can_go_previous, theme);
    render_button(
        frame,
        toggle,
        if snapshot.playback == Playback::Playing {
            "||"
        } else {
            "▶"
        },
        true,
        theme,
    );
    render_button(frame, next, "▶▶", snapshot.can_go_next, theme);

    HitRegions {
        previous: snapshot.can_go_previous.then_some(previous),
        toggle: Some(toggle),
        next: snapshot.can_go_next.then_some(next),
        seek: None,
    }
}

fn render_compact_controls(
    frame: &mut Frame<'_>,
    area: Rect,
    snapshot: &PlayerSnapshot,
    theme: Theme,
) -> HitRegions {
    let width = 17.min(area.width);
    let area = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y,
        width,
        1,
    );
    let [previous, toggle, next] = RatatuiLayout::horizontal([
        Constraint::Length(5),
        Constraint::Length(7),
        Constraint::Length(5),
    ])
    .areas(area);
    render_button(frame, previous, "‹", snapshot.can_go_previous, theme);
    render_button(
        frame,
        toggle,
        if snapshot.playback == Playback::Playing {
            "||"
        } else {
            "▶"
        },
        true,
        theme,
    );
    render_button(frame, next, "›", snapshot.can_go_next, theme);
    HitRegions {
        previous: snapshot.can_go_previous.then_some(previous),
        toggle: Some(toggle),
        next: snapshot.can_go_next.then_some(next),
        seek: None,
    }
}

fn render_button(frame: &mut Frame<'_>, area: Rect, label: &str, enabled: bool, theme: Theme) {
    frame.render_widget(
        Paragraph::new(format!("[ {label} ]"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(if enabled { theme.text } else { theme.faint })),
        area,
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

fn centered_width(area: Rect, max_width: u16) -> Rect {
    let width = area.width.min(max_width);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        width,
        ..area
    }
}

fn inset(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(horizontal).min(area.right()),
        y: area.y.saturating_add(vertical).min(area.bottom()),
        width: area.width.saturating_sub(horizontal.saturating_mul(2)),
        height: area.height.saturating_sub(vertical.saturating_mul(2)),
    }
}

fn contains(area: Rect, column: u16, row: u16) -> bool {
    column >= area.x && column < area.right() && row >= area.y && row < area.bottom()
}

fn format_duration(duration: std::time::Duration) -> String {
    let seconds = duration.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use ratatui::{backend::TestBackend, Terminal};

    use super::*;

    fn rendered(width: u16, height: u16, layout: Layout) -> (String, HitRegions) {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let snapshot = PlayerSnapshot::demo();
        let theme = Theme::from_accent(None).unwrap();
        let mut hits = HitRegions::default();
        terminal
            .draw(|frame| {
                hits = render(
                    frame,
                    &ProviderState::Ready(Box::new(snapshot.clone())),
                    layout,
                    theme,
                    None,
                )
            })
            .unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        (output, hits)
    }

    #[test]
    fn vertical_renders_metadata_duration_and_controls() {
        let (output, _) = rendered(80, 30, Layout::Vertical);
        assert!(output.contains("Afterglow Circuit"));
        assert!(output.contains("Nocturne Assembly"));
        assert!(output.contains("4:18"));
        assert!(output.contains("||"));
        assert!(!output.contains("space play/pause"));
    }

    #[test]
    fn narrow_layout_falls_back_without_losing_title() {
        let (output, _) = rendered(40, 4, Layout::Vertical);
        assert!(output.contains("Afterglow Circuit"));
    }

    #[test]
    fn every_public_layout_renders() {
        for layout in [
            Layout::Vertical,
            Layout::Hero,
            Layout::Wide,
            Layout::Compact,
            Layout::Minimal,
        ] {
            let (output, _) = rendered(100, 30, layout);
            assert!(output.contains("Afterglow Circuit"));
            assert!(!output.contains('▁'));
        }
    }

    #[test]
    fn clickable_regions_map_to_actions() {
        let (_, hits) = rendered(80, 30, Layout::Vertical);
        let toggle = hits.toggle.unwrap();
        assert_eq!(
            hits.action_at(toggle.x, toggle.y),
            Some(UiAction::TogglePlayback)
        );
        let seek = hits.seek.unwrap();
        let Some(UiAction::Seek(ratio)) = hits.action_at(seek.x + seek.width / 2, seek.y) else {
            panic!("seek rail did not return a seek action");
        };
        assert!((ratio - 0.5).abs() < 0.02);
    }
}
