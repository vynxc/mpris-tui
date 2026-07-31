# Changelog

All notable changes to MPRIS TUI will be documented here.

## [Unreleased]

- Added a centered vertical layout as the default.
- Added bounded local album-art decoding and transparent half-block rendering.
- Added click-only previous, play/pause, next, and seek controls with no
  keyboard shortcuts or in-app exit control.
- Added elapsed and total duration directly alongside the seek rail.
- Added process-aware player naming so Pear Desktop is shown by name instead of
  its inherited reverse-DNS identity.
- Removed the simulated audio waveform.

## [0.1.0] - 2026-07-29

- Initial public release.
- Automatic and explicit MPRIS player selection.
- Signal-driven metadata, status, rate, volume, and seek updates.
- Locally extrapolated progress with periodic position resynchronization.
- Responsive hero, wide, compact, and minimal layouts.
- Foreground-only truecolor rendering for transparent terminals.
- Deterministic demo mode and one-frame rendering.
- Isolated mock D-Bus integration test, GitHub Actions CI, and release workflow.

[Unreleased]: https://github.com/vynxc/mpris-tui/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/vynxc/mpris-tui/releases/tag/v0.1.0
