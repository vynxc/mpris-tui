use std::{fmt::Write as _, str::FromStr};

use crate::ui::Layout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub layout: Layout,
    pub player: String,
    pub frames_per_second: u16,
    pub accent: Option<String>,
    pub demo: bool,
    pub once: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            layout: Layout::Hero,
            player: "auto".into(),
            frames_per_second: 4,
            accent: None,
            demo: false,
            once: false,
        }
    }
}

impl Config {
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut config = Self::default();
        let mut arguments = arguments.into_iter();

        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--layout" => {
                    let value = next_value(&mut arguments, "--layout")?;
                    config.layout = Layout::from_str(&value)?;
                }
                "--player" => config.player = next_value(&mut arguments, "--player")?,
                "--fps" => {
                    let value = next_value(&mut arguments, "--fps")?;
                    config.frames_per_second = value
                        .parse::<u16>()
                        .map_err(|_| format!("invalid frame rate `{value}`"))?
                        .clamp(1, 30);
                }
                "--accent" => config.accent = Some(next_value(&mut arguments, "--accent")?),
                "--demo" => config.demo = true,
                "--once" => config.once = true,
                "--help" | "-h" => return Err(help()),
                "--version" | "-V" => {
                    return Err(format!("mpris-tui {}", env!("CARGO_PKG_VERSION")));
                }
                value => return Err(format!("unknown option `{value}`\n\n{}", help())),
            }
        }

        if config.player.trim().is_empty() {
            return Err("player selector cannot be empty".into());
        }
        Ok(config)
    }
}

fn next_value(
    arguments: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<String, String> {
    arguments
        .next()
        .ok_or_else(|| format!("{option} requires a value"))
}

pub fn help() -> String {
    let mut help = String::new();
    writeln!(help, "mpris-tui — a transparent now-playing display").unwrap();
    writeln!(help).unwrap();
    writeln!(help, "Usage: mpris-tui [OPTIONS]").unwrap();
    writeln!(help).unwrap();
    writeln!(help, "  --layout <NAME>    hero, wide, compact, or minimal").unwrap();
    writeln!(
        help,
        "  --player <MATCH>   auto, a bus name, or identity fragment"
    )
    .unwrap();
    writeln!(
        help,
        "  --fps <1-30>       maximum redraw rate (default: 4)"
    )
    .unwrap();
    writeln!(help, "  --accent <#RRGGBB> override the accent color").unwrap();
    writeln!(
        help,
        "  --demo             use deterministic sample playback"
    )
    .unwrap();
    writeln!(help, "  --once             render one frame and exit").unwrap();
    writeln!(help, "  -h, --help         show this help").unwrap();
    writeln!(help, "  -V, --version      show the version").unwrap();
    help
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<Config, String> {
        Config::parse(arguments.iter().map(ToString::to_string))
    }

    #[test]
    fn parses_all_options() {
        let config = parse(&[
            "--layout", "compact", "--player", "chromium", "--fps", "12", "--accent", "#aabbcc",
            "--demo", "--once",
        ])
        .unwrap();

        assert_eq!(config.layout, Layout::Compact);
        assert_eq!(config.player, "chromium");
        assert_eq!(config.frames_per_second, 12);
        assert_eq!(config.accent.as_deref(), Some("#aabbcc"));
        assert!(config.demo);
        assert!(config.once);
    }

    #[test]
    fn clamps_frame_rate() {
        assert_eq!(parse(&["--fps", "0"]).unwrap().frames_per_second, 1);
        assert_eq!(parse(&["--fps", "99"]).unwrap().frames_per_second, 30);
    }

    #[test]
    fn rejects_missing_and_unknown_values() {
        assert!(parse(&["--layout"]).is_err());
        assert!(parse(&["--layout", "giant"]).is_err());
        assert!(parse(&["--wat"]).is_err());
    }
}
