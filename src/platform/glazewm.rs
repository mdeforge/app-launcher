//! GlazeWM config lookup for theme colors

use iced::Color;
use yaml_rust2::{Yaml, YamlLoader};

/// Window border colors from GlazeWM's `window_effects` config
#[derive(Debug, Default, PartialEq)]
pub struct BorderColors {
    /// `window_effects.focused_window.border.color`
    pub focused: Option<Color>,
    /// `window_effects.other_windows.border.color`
    pub other: Option<Color>,
}

/// Read border colors from `~/.glzr/glazewm/config.yaml`.
/// A missing file, missing key, or disabled border yields `None` for that color.
pub fn border_colors() -> BorderColors {
    dirs::home_dir()
        .map(|home| home.join(r".glzr\glazewm\config.yaml"))
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|source| parse_config(&source))
        .unwrap_or_default()
}

/// Extract border colors from GlazeWM config YAML
fn parse_config(source: &str) -> BorderColors {
    let Some(doc) = YamlLoader::load_from_str(source).ok().and_then(|docs| docs.into_iter().next())
    else {
        return BorderColors::default();
    };

    let effects = &doc["window_effects"];
    BorderColors {
        focused: border_color(&effects["focused_window"]),
        other: border_color(&effects["other_windows"]),
    }
}

/// Border color of one `window_effects` entry, unless that border is disabled
fn border_color(window: &Yaml) -> Option<Color> {
    let border = &window["border"];
    if border["enabled"].as_bool() == Some(false) {
        return None;
    }
    parse_hex(border["color"].as_str()?)
}

/// Parse `#rrggbb` or `#rrggbbaa`
fn parse_hex(s: &str) -> Option<Color> {
    let hex = s.strip_prefix('#')?;
    if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();

    match hex.len() {
        6 => Some(Color::from_rgb8(byte(0)?, byte(2)?, byte(4)?)),
        8 => Some(Color::from_rgba8(byte(0)?, byte(2)?, byte(4)?, byte(6)? as f32 / 255.0)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_border_colors() {
        let colors = parse_config(
            "window_effects:
  focused_window:
    border:
      enabled: true
      color: '#fabd2f'
  other_windows:
    border:
      enabled: true
      color: '#504945'
",
        );
        assert_eq!(colors.focused, Some(Color::from_rgb8(0xfa, 0xbd, 0x2f)));
        assert_eq!(colors.other, Some(Color::from_rgb8(0x50, 0x49, 0x45)));
    }

    #[test]
    fn disabled_or_missing_border_is_none() {
        let colors = parse_config(
            "window_effects:
  focused_window:
    border:
      enabled: false
      color: '#fabd2f'
",
        );
        assert_eq!(colors, BorderColors::default());
    }

    #[test]
    fn invalid_yaml_is_default() {
        assert_eq!(parse_config("window_effects: ["), BorderColors::default());
    }

    #[test]
    fn parses_hex_with_and_without_alpha() {
        assert_eq!(parse_hex("#ffffff"), Some(Color::WHITE));
        assert_eq!(parse_hex("#00000080"), Some(Color::from_rgba8(0, 0, 0, 128.0 / 255.0)));
        assert_eq!(parse_hex("ffffff"), None);
        assert_eq!(parse_hex("#fff"), None);
        assert_eq!(parse_hex("#+fffff"), None);
        assert_eq!(parse_hex("#ffé"), None);
    }
}
