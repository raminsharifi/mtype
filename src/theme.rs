//! Color themes. A theme is the same 10-color palette Monkeytype uses
//! (`frontend/src/ts/constants/themes.ts`). v1 ships the single iconic
//! `serika_dark` default; the struct is built so additional palettes (loaded
//! from disk in the web phase) drop straight in.

use ratatui::style::Color;

#[derive(Debug, Clone)]
#[allow(dead_code)] // `name` is kept for diagnostics / future theme switching
pub struct Theme {
    pub name: String,
    pub bg: Color,
    pub main: Color,
    pub caret: Color,
    pub sub: Color,
    pub sub_alt: Color,
    pub text: Color,
    pub error: Color,
    pub error_extra: Color,
    pub colorful_error: Color,
    pub colorful_error_extra: Color,
}

/// Parse a `#rrggbb` (or `#rgb`) hex string into a ratatui truecolor.
pub fn hex(s: &str) -> Color {
    let s = s.trim_start_matches('#');
    let expand = |c: u8| -> u8 {
        let v = (c as char).to_digit(16).unwrap_or(0) as u8;
        v * 16 + v
    };
    match s.len() {
        3 => {
            let b = s.as_bytes();
            Color::Rgb(expand(b[0]), expand(b[1]), expand(b[2]))
        }
        6 | 8 => {
            let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
            Color::Rgb(r, g, b)
        }
        _ => Color::Reset,
    }
}

impl Theme {
    /// The Monkeytype default theme: `serika_dark`
    /// (`frontend/src/ts/constants/themes.ts`).
    pub fn serika_dark() -> Theme {
        Theme {
            name: "serika_dark".to_string(),
            bg: hex("#323437"),
            main: hex("#e2b714"),
            caret: hex("#e2b714"),
            sub: hex("#646669"),
            sub_alt: hex("#2c2e31"),
            text: hex("#d1d0c5"),
            error: hex("#ca4754"),
            error_extra: hex("#7e2a33"),
            colorful_error: hex("#ca4754"),
            colorful_error_extra: hex("#7e2a33"),
        }
    }

    /// Resolve a theme by name; unknown names fall back to the default.
    pub fn by_name(name: &str) -> Theme {
        match name {
            "serika_dark" => Theme::serika_dark(),
            _ => Theme::serika_dark(),
        }
    }

    pub fn available_names() -> &'static [&'static str] {
        &["serika_dark"]
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::serika_dark()
    }
}
