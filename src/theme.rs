use ratatui::style::Color;

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub accent: Color,
    pub bright: Color,
    pub text: Color,
    pub muted: Color,
    pub faint: Color,
}

impl Theme {
    pub fn from_accent(value: Option<&str>) -> Result<Self, String> {
        let accent = match value {
            Some(value) => parse_hex_color(value)?,
            None => Color::Rgb(196, 143, 255),
        };
        Ok(Self {
            accent,
            bright: Color::Rgb(238, 232, 245),
            text: Color::Rgb(196, 189, 207),
            muted: Color::Rgb(137, 130, 150),
            faint: Color::Rgb(78, 73, 89),
        })
    }
}

fn parse_hex_color(value: &str) -> Result<Color, String> {
    let value = value.strip_prefix('#').unwrap_or(value);
    if value.len() != 6 {
        return Err(format!("accent `{value}` must use RRGGBB hexadecimal"));
    }
    let component = |range| {
        u8::from_str_radix(&value[range], 16)
            .map_err(|_| format!("accent `{value}` must use RRGGBB hexadecimal"))
    };
    Ok(Color::Rgb(
        component(0..2)?,
        component(2..4)?,
        component(4..6)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hash_and_plain_hex() {
        assert_eq!(
            Theme::from_accent(Some("#102030")).unwrap().accent,
            Color::Rgb(16, 32, 48)
        );
        assert!(Theme::from_accent(Some("abcdef")).is_ok());
    }

    #[test]
    fn rejects_invalid_colors() {
        assert!(Theme::from_accent(Some("#fff")).is_err());
        assert!(Theme::from_accent(Some("#gg0000")).is_err());
    }
}
