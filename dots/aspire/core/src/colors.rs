//! Lectura de la paleta canónica desde `theme-engine/current/colors.rasi`
//! o, como compatibilidad, `theme-engine/colors.rasi`.
//!
//! La escribe `theme_engine` (aspire-bg al mando) y la leen waybar, rofi,
//! mako, aspire-menu, aspire-clip y aspire-bg. Un solo formato, un solo
//! lector: este módulo.

/// Color RGB leído de `colors.rasi`. Tipo propio para no depender del toolkit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Colores del tema. Cada campo es `None` si no aparece en `colors.rasi`.
#[derive(Debug, Clone, Default)]
pub struct ThemeColors {
    pub background: Option<RgbColor>,
    pub background_alt: Option<RgbColor>,
    pub foreground: Option<RgbColor>,
    pub selected: Option<RgbColor>,
    pub selectedtext: Option<RgbColor>,
    pub active: Option<RgbColor>,
    pub urgent: Option<RgbColor>,
    pub ansi: [Option<RgbColor>; 16],
}

pub fn parse_color(text: &str, name: &str) -> Option<RgbColor> {
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(name) {
            if let Some(v) = rest.trim_start().strip_prefix(':') {
                let hex = v.trim().trim_end_matches(';').trim();
                let hex = hex.trim_start_matches('#');
                if (hex.len() == 6 || hex.len() == 8) && hex.is_ascii() {
                    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                    return Some(RgbColor { r, g, b });
                }
            }
        }
    }
    None
}

pub fn load_theme_colors() -> ThemeColors {
    let mut colors = ThemeColors::default();
    let Some(home) = dirs::home_dir() else {
        return colors;
    };
    let theme_dir = home.join(".config/theme-engine");
    let current_colors = theme_dir.join("current/colors.rasi");
    let legacy_colors = theme_dir.join("colors.rasi");
    let path = if current_colors.is_file() {
        current_colors
    } else {
        legacy_colors
    };
    let Ok(text) = std::fs::read_to_string(path) else {
        return colors;
    };
    colors.background = parse_color(&text, "background");
    colors.background_alt = parse_color(&text, "background-alt");
    colors.foreground = parse_color(&text, "foreground");
    colors.selected = parse_color(&text, "selected");
    colors.selectedtext = parse_color(&text, "selectedtext");
    colors.active = parse_color(&text, "active");
    colors.urgent = parse_color(&text, "urgent");
    for (index, color) in colors.ansi.iter_mut().enumerate() {
        *color = parse_color(&text, &format!("color{index}"));
    }
    colors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rgb_and_rgba_values() {
        assert_eq!(
            parse_color("* { background: #112233; }", "background"),
            None
        );
        assert_eq!(
            parse_color("background: #112233FF;", "background"),
            Some(RgbColor {
                r: 0x11,
                g: 0x22,
                b: 0x33
            })
        );
        assert_eq!(
            parse_color("background: #aabbcc;", "background"),
            Some(RgbColor {
                r: 0xaa,
                g: 0xbb,
                b: 0xcc
            })
        );
    }

    #[test]
    fn rejects_invalid_hex_without_panicking() {
        assert_eq!(parse_color("background: #12zz56;", "background"), None);
        assert_eq!(parse_color("background: #123456789;", "background"), None);
        assert_eq!(parse_color("background: #éééééé;", "background"), None);
    }
}
