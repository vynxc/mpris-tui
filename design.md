# MPRIS TUI design

## Product

MPRIS TUI is a glanceable now-playing surface, not a player controller. Its
primary use is a transparent desktop canvas, while remaining a normal terminal
program with no KDE or Desktop TUI dependency.

The visual signature is an animated signal strip paired with a thin progress
rail. Information hierarchy is title, artist, playback state, album, then
timing. No card, border, shadow, or background competes with the wallpaper.

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
            `-- Position resync every two seconds
                    |
                    v
              PlayerSnapshot
                    |
                    v
          responsive Ratatui renderer
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

All styles set foreground colors only. Ratatui's default cell background is
left untouched, which allows a transparent terminal host to composite the
wallpaper through every unused cell.

Requested layouts degrade by available geometry:

- very small areas use `minimal`;
- narrow or short areas use `compact`;
- larger areas preserve `hero` or `wide`.

## Lifecycle

The process listens for `SIGINT` and `SIGTERM`, exits its render loop, restores
the terminal, and aborts the provider task. Player disappearance is not fatal:
the provider reports an unavailable state and resumes discovery.

`--once` waits briefly for its first provider state, draws one frame, restores
the terminal, and exits. `--demo` uses deterministic public-safe sample
metadata and requires no D-Bus player.

## Privacy and security

- No album art or other URL is fetched.
- No telemetry, network request, or persistent player history exists.
- MPRIS methods that change playback are not exposed.
- Player metadata is held only in memory.
- README media uses fictional fixture metadata.

## Testing

Unit tests cover argument bounds, theme parsing, metadata conversion, playback
extrapolation, progress clamping, responsive fallback, and every public layout.

The integration test registers two mock interfaces at
`/org/mpris/MediaPlayer2` inside an isolated `dbus-run-session`, discovers the
service through the standard bus API, and validates the complete snapshot.

