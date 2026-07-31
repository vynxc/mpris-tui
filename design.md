# MPRIS TUI design

## Product

MPRIS TUI is a glanceable, click-only now-playing controller. Its primary use
is a transparent desktop canvas, while remaining a normal terminal program
with no KDE or Desktop TUI dependency.

The visual signature is real cover art rendered with terminal half-blocks,
paired with a thin seek rail that keeps elapsed and total duration visible.
Information hierarchy is artwork, title, artist, progress, then three icon-only
transport controls. No card, shadow, fake audio visualization, hotkey legend,
or full-canvas background competes with the wallpaper.

## Data flow

```text
session D-Bus
    |
    +-- org.freedesktop.DBus.ListNames
    |
    `-- org.mpris.MediaPlayer2.*
            |
            +-- metadata/status/rate/volume property signals
            +-- Seeked signal
            +-- Position resync every two seconds
            `-- previous / play-pause / next / set-position calls
                    |
                    v
              PlayerSnapshot
                    |
                    v
          responsive Ratatui renderer
                    ^
                    |
             left-click hit regions
```

Automatic player selection prefers a service reporting `Playing`, then falls
back to the first compatible service. Explicit selection matches either the
service name or player identity.

MPRIS does not continuously signal `Position`. A snapshot records position,
rate, playback state, and sync time. The UI extrapolates between samples and
clamps progress to known duration. A periodic read corrects drift, while the
`Seeked` signal triggers an immediate refresh.

## Rendering

The maximum redraw rate defaults to 4 FPS and is configurable from 1–30 FPS.
The provider loop sends immutable snapshots through a Tokio watch channel;
rendering never blocks on D-Bus.

Text and rails set foreground colors only. Artwork uses `▀`/`▄` glyphs with
foreground and background colors to represent two source rows per terminal
cell. Every cell outside the artwork remains untouched, allowing a transparent
terminal host to composite the wallpaper.

Requested layouts degrade by available geometry:

- very small areas use `minimal`;
- narrow or short areas use `compact`;
- larger areas preserve `vertical`, `hero`, or `wide`.

`vertical` is the default and centers a bounded player column. `hero` remains
as a compatibility alias for the same vertical composition.

## Interaction

Crossterm mouse capture is enabled by default. Only left-button presses are
acted upon, and only inside the previous, play/pause, next, and seek regions.
The app defines no keyboard shortcuts and no exit control. `--no-mouse`
disables capture for display-only hosts.

UI actions travel over a bounded Tokio channel to the active player watcher.
The watcher owns the typed MPRIS proxy, checks advertised capabilities, and
refreshes state immediately after a command.

## Lifecycle

The process listens for `SIGINT` and `SIGTERM`, exits its render loop, restores
the terminal, and aborts the provider task. Player disappearance is not fatal:
the provider reports an unavailable state and resumes discovery.

`--once` waits briefly for its first provider state, draws one frame, restores
the terminal, and exits. `--demo` uses deterministic public-safe sample
metadata and requires no D-Bus player.

## Privacy and security

- Only local `file://` artwork is read; remote URLs are never fetched.
- Decode dimensions and allocation are bounded, and artwork is cached by URL
  and terminal geometry.
- No telemetry, network request, or persistent player history exists.
- Playback changes happen only after a left-click in a rendered hit region.
- Player metadata is held only in memory.
- README media uses fictional fixture metadata.

## Testing

Unit tests cover argument bounds, theme parsing, metadata conversion, identity
normalization, artwork URL decoding and half-block output, playback
extrapolation, progress clamping, responsive fallback, click hit regions, and
every public layout.

The integration test registers two mock interfaces at
`/org/mpris/MediaPlayer2` inside an isolated `dbus-run-session`, discovers the
service through the standard bus API, validates the complete snapshot, then
verifies previous, play/pause, next, and set-position method calls.
