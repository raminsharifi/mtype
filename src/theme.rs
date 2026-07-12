//! Color themes. A theme is the same 10-color palette Monkeytype uses
//! (`frontend/src/ts/constants/themes.ts`). v1 ships the single iconic
//! `serika_dark` default; the struct is built so additional palettes (loaded
//! from disk in the web phase) drop straight in.

use ratatui::style::Color;
use serde::Deserialize;

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
    fn palette(name: &str, colors: [&str; 10]) -> Theme {
        Theme {
            name: name.to_string(),
            bg: hex(colors[0]),
            main: hex(colors[1]),
            caret: hex(colors[2]),
            sub: hex(colors[3]),
            sub_alt: hex(colors[4]),
            text: hex(colors[5]),
            error: hex(colors[6]),
            error_extra: hex(colors[7]),
            colorful_error: hex(colors[8]),
            colorful_error_extra: hex(colors[9]),
        }
    }

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
            "dracula" => Self::palette(
                "dracula",
                [
                    "#282a36", "#bd93f9", "#bd93f9", "#6272a4", "#21222c", "#f8f8f2", "#ff5555",
                    "#b83b5e", "#ff5555", "#b83b5e",
                ],
            ),
            "nord" => Self::palette(
                "nord",
                [
                    "#2e3440", "#88c0d0", "#88c0d0", "#4c566a", "#3b4252", "#d8dee9", "#bf616a",
                    "#8f3f4a", "#bf616a", "#8f3f4a",
                ],
            ),
            "catppuccin" => Self::palette(
                "catppuccin",
                [
                    "#1e1e2e", "#cba6f7", "#f5e0dc", "#6c7086", "#181825", "#cdd6f4", "#f38ba8",
                    "#a65d73", "#f38ba8", "#a65d73",
                ],
            ),
            "gruvbox_dark" => Self::palette(
                "gruvbox_dark",
                [
                    "#282828", "#fabd2f", "#fabd2f", "#665c54", "#1d2021", "#ebdbb2", "#fb4934",
                    "#9d0006", "#fb4934", "#9d0006",
                ],
            ),
            "tokyo_night" => Self::palette(
                "tokyo_night",
                [
                    "#1a1b26", "#7aa2f7", "#bb9af7", "#565f89", "#16161e", "#c0caf5", "#f7768e",
                    "#9d3b53", "#f7768e", "#9d3b53",
                ],
            ),
            "rose_pine" => Self::palette(
                "rose_pine",
                [
                    "#191724", "#c4a7e7", "#ebbcba", "#6e6a86", "#1f1d2e", "#e0def4", "#eb6f92",
                    "#9b405d", "#eb6f92", "#9b405d",
                ],
            ),
            "solarized_dark" => Self::palette(
                "solarized_dark",
                [
                    "#002b36", "#b58900", "#b58900", "#586e75", "#073642", "#eee8d5", "#dc322f",
                    "#8b1f1d", "#dc322f", "#8b1f1d",
                ],
            ),
            _ => Self::load_custom(name).unwrap_or_else(Theme::serika_dark),
        }
    }

    fn load_custom(name: &str) -> Option<Theme> {
        let path = crate::content::data_dir()?
            .join("themes")
            .join(format!("{name}.toml"));
        let value: ThemeFile = toml::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
        Some(Self::palette(
            name,
            [
                &value.bg,
                &value.main,
                value.caret.as_deref().unwrap_or(&value.main),
                &value.sub,
                &value.sub_alt,
                &value.text,
                &value.error,
                value.error_extra.as_deref().unwrap_or(&value.error),
                value.colorful_error.as_deref().unwrap_or(&value.error),
                value
                    .colorful_error_extra
                    .as_deref()
                    .or(value.error_extra.as_deref())
                    .unwrap_or(&value.error),
            ],
        ))
    }

    pub fn available_names() -> Vec<String> {
        let mut names = [
            "serika_dark",
            "dracula",
            "nord",
            "catppuccin",
            "gruvbox_dark",
            "tokyo_night",
            "rose_pine",
            "solarized_dark",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
        if let Some(dir) = crate::content::data_dir().map(|path| path.join("themes")) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for path in entries.flatten().map(|entry| entry.path()) {
                    if path.extension().and_then(|ext| ext.to_str()) == Some("toml") {
                        if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                            if !names.iter().any(|name| name == stem) {
                                names.push(stem.to_string());
                            }
                        }
                    }
                }
            }
        }
        names.sort();
        names
    }
}

#[derive(Debug, Deserialize)]
struct ThemeFile {
    bg: String,
    main: String,
    sub: String,
    sub_alt: String,
    text: String,
    error: String,
    caret: Option<String>,
    error_extra: Option<String>,
    colorful_error: Option<String>,
    colorful_error_extra: Option<String>,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::serika_dark()
    }
}
