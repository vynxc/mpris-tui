<div align="center">

# MPRIS TUI

**A transparent, click-only now-playing controller for Linux.**

[![CI](https://github.com/vynxc/mpris-tui/actions/workflows/ci.yml/badge.svg)](https://github.com/vynxc/mpris-tui/actions/workflows/ci.yml)
[![Rust 1.88+](https://img.shields.io/badge/Rust-1.88%2B-b7410e?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![MPRIS](https://img.shields.io/badge/MPRIS-2.2-8b7cf6)](https://specifications.freedesktop.org/mpris/latest/)
[![MIT](https://img.shields.io/badge/license-MIT-7d89c7)](LICENSE)

![MPRIS TUI floating directly over a wallpaper](docs/hero.gif)

</div>

MPRIS TUI turns the active Linux media player into a polished Ratatui canvas
with artwork, progress, and mouse controls. It has no prompt, keyboard
shortcuts, window chrome, opaque card, or player-specific API. Run it in a
terminal, or place it directly on a KDE Plasma desktop with
[Desktop TUI](https://github.com/vynxc/desktop-tui).

## Highlights

- Discovers every `org.mpris.MediaPlayer2.*` service on the session bus.
- Prefers the playing service automatically, or matches a requested player.
- Reacts to metadata, playback, rate, volume, and seek signals.
- Extrapolates progress locally and resynchronizes position every two seconds.
- Shows elapsed and total duration on a clickable seek rail.
- Sends previous, play/pause, next, and absolute seek commands over MPRIS.
- Renders local MPRIS artwork as truecolor foreground-only block pixels.
- Defaults to a centered vertical player and adapts to smaller widget sizes.
- Leaves every cell outside the artwork transparent.
- Uses a configurable 1–30 FPS redraw ceiling; the default is a quiet 4 FPS.
- Uses left-click only; there are no application hotkeys or in-app exit control.

![Four responsive MPRIS TUI layouts](docs/layouts.webp)

## Install

MPRIS TUI requires Linux, a session D-Bus, and Rust 1.88 or newer.

```bash
cargo install --locked --git https://github.com/vynxc/mpris-tui
```

Then run it while any MPRIS-compatible player is open:

```bash
mpris-tui
```

The application restores the terminal when its host closes it or sends
`SIGINT`/`SIGTERM`. It defines no keyboard shortcuts.

## Layouts and options

```text
mpris-tui [OPTIONS]

  --layout <NAME>    vertical, hero, wide, compact, or minimal
  --player <MATCH>   auto, a bus name, or identity fragment
  --fps <1-30>       maximum redraw rate (default: 4)
  --accent <#RRGGBB> override the accent color
  --no-mouse         disable clickable controls
  --demo             use deterministic sample playback
  --once             render one frame and exit
```

Examples:

```bash
mpris-tui --layout vertical
mpris-tui --layout compact --player spotify
mpris-tui --layout minimal --fps 2 --accent '#e06c9f'
mpris-tui --demo
```

Layouts automatically collapse when the terminal is too small, so a vertical
canvas remains readable when moved to a narrow widget or portrait display.

## Put it on the KDE desktop

Install [Desktop TUI 0.2.0 or newer](https://github.com/vynxc/desktop-tui),
add a widget instance, and choose **Command output**.

| Desktop TUI setting | Value |
| --- | --- |
| Program | `mpris-tui` |
| Arguments | `--layout` on one line, `vertical` on the next |
| After exit | Keep running |
| Timeout | Disabled |
| Clear between runs | Enabled |
| Terminal mouse interaction | Enabled |

The command canvas is a real PTY, so MPRIS TUI's Ratatui output and alpha
behavior are preserved. Desktop TUI forwards left-clicks when terminal mouse
interaction is enabled; middle- and right-click remain available to Plasma.

Use separate widget instances—and different `--layout`, `--player`, `--fps`,
or `--accent` arguments—on as many monitors as you like.

## Player selection

`--player auto` is the default. It chooses a currently playing MPRIS service,
then falls back to the first compatible service.

Any other value is matched case-insensitively against both the D-Bus name and
the player's reported identity:

```bash
mpris-tui --player spotify
mpris-tui --player youtube
mpris-tui --player org.mpris.MediaPlayer2.vlc
```

If the player exits, MPRIS TUI returns to its quiet empty state and discovers
the next match when it appears.

## Transparency

MPRIS TUI leaves all unused cells transparent. Artwork averages two source
rows into each foreground-only `█` cell. This avoids terminal background
painting entirely, so the cover survives transparent hosts that composite only
glyphs. Transparency is ultimately decided by the terminal:

- a normal terminal needs a transparent profile;
- Desktop TUI ships a transparent terminal surface;
- multiplexers and wrappers must not inject a background color.

The default palette uses truecolor foregrounds. Because color carries the
artwork itself, MPRIS TUI intentionally overrides a process-level `NO_COLOR`
setting. `--accent` changes only the accent role; it does not recolor artwork.

## Efficient by design

MPRIS metadata and state are signal-driven. Position is the exception: the
MPRIS specification does not emit continuous position changes, so MPRIS TUI
advances progress from the last position/rate sample and performs a two-second
D-Bus resync. The UI defaults to four draws per second. Local `file://`
artwork is bounded, decoded once when its URL changes, and resized once per
terminal geometry. Remote artwork is never downloaded.

## Development

```bash
make check
make act
```

`make check` runs formatting, Clippy with warnings denied, 20 unit tests, a
mock-player integration test inside an isolated D-Bus session, and an optimized
release build.

The same Ubuntu 24.04 workflow can run locally with
[act](https://github.com/nektos/act):

```bash
DOCKER_HOST=unix:///run/user/1000/podman/podman.sock \
  act pull_request -W .github/workflows/ci.yml -j test \
  -P ubuntu-24.04=catthehacker/ubuntu:act-24.04
```

Project layout:

```text
src/artwork.rs   bounded local artwork cache and foreground-only renderer
src/cli.rs       dependency-light argument parsing
src/model.rs     playback state and position extrapolation
src/mpris.rs     discovery, typed D-Bus proxies, controls, and signal loop
src/theme.rs     transparent semantic palette
src/ui.rs        responsive layouts and click hit regions
tests/           isolated mock MPRIS service and control verification
tools/           reproducible README media
```

See [design.md](design.md) for lifecycle, privacy, and rendering decisions.

## Compatibility

MPRIS is implemented by many Linux players and browsers, including VLC,
Spotify clients, Chromium-based browsers, Firefox, mpv integrations, and
YouTube Music clients. Exact metadata and capabilities depend on the player.

The implementation follows the freedesktop.org
[MPRIS Player interface](https://specifications.freedesktop.org/mpris/latest/Player_Interface.html).

## License

MPRIS TUI is available under the [MIT License](LICENSE).
