//! User configuration - a Rust port of the relevant subset of Monkeytype's
//! `ConfigSchema` (`packages/schemas/src/configs.ts`) with the same defaults as
//! `frontend/src/ts/constants/default-config.ts`. Persisted as TOML in the
//! platform config dir. No account sync - this is the whole settings story.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Enums (mirror the Monkeytype config enums, trimmed to terminal-relevant ones)
// ---------------------------------------------------------------------------

macro_rules! str_enum {
    ($(#[$m:meta])* $name:ident { $($variant:ident => $s:literal),+ $(,)? }, default $def:ident) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
        #[allow(dead_code)] // not every enum uses every generated helper
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];
            pub fn as_str(&self) -> &'static str { match self { $($name::$variant => $s),+ } }
            pub fn from_str_opt(s: &str) -> Option<$name> {
                match s { $($s => Some($name::$variant),)+ _ => None }
            }
        }
        impl Default for $name { fn default() -> Self { $name::$def } }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str(self.as_str()) }
        }
    };
}

str_enum!(Mode { Time => "time", Words => "words", Quote => "quote", Zen => "zen", Custom => "custom" }, default Time);
str_enum!(Difficulty { Normal => "normal", Expert => "expert", Master => "master" }, default Normal);
str_enum!(StopOnError { Off => "off", Letter => "letter", Word => "word" }, default Off);
str_enum!(ConfidenceMode { Off => "off", On => "on", Max => "max" }, default Off);
str_enum!(IndicateTypos { Off => "off", Below => "below", Replace => "replace" }, default Off);
str_enum!(CaretStyle { Off => "off", Default => "default", Block => "block", Outline => "outline", Underline => "underline" }, default Default);
str_enum!(SmoothCaret { Off => "off", Slow => "slow", Medium => "medium", Fast => "fast" }, default Medium);
str_enum!(HighlightMode { Off => "off", Letter => "letter", Word => "word", NextWord => "next_word" }, default Letter);
str_enum!(IndicatorStyle { Off => "off", Text => "text", Bar => "bar", Mini => "mini" }, default Mini);
str_enum!(QuickRestart { Off => "off", Esc => "esc", Tab => "tab", Enter => "enter" }, default Off);
str_enum!(PaceCaret { Off => "off", Average => "average", Pb => "pb", Last => "last", Custom => "custom" }, default Off);
str_enum!(TypingSpeedUnit { Wpm => "wpm", Cpm => "cpm", Wps => "wps", Cps => "cps", Wph => "wph" }, default Wpm);
str_enum!(QuoteLengthBand { Short => "short", Medium => "medium", Long => "long", Thicc => "thicc" }, default Medium);

impl QuoteLengthBand {
    /// Numeric band index used by Monkeytype's quote `groups`.
    pub fn index(&self) -> usize {
        match self {
            QuoteLengthBand::Short => 0,
            QuoteLengthBand::Medium => 1,
            QuoteLengthBand::Long => 2,
            QuoteLengthBand::Thicc => 3,
        }
    }
}

impl TypingSpeedUnit {
    /// Multiplier applied to a raw WPM value (chars/5/min) to convert units.
    pub fn convert_from_wpm(&self, wpm: f64) -> f64 {
        match self {
            TypingSpeedUnit::Wpm => wpm,
            TypingSpeedUnit::Cpm => wpm * 5.0,
            TypingSpeedUnit::Wps => wpm / 60.0,
            TypingSpeedUnit::Cps => (wpm * 5.0) / 60.0,
            TypingSpeedUnit::Wph => wpm * 60.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    // mode + targets
    pub mode: Mode,
    pub time: u32,
    pub words: u32,
    pub quote_length: Vec<QuoteLengthBand>,
    pub custom_text: String,

    // modifiers
    pub punctuation: bool,
    pub numbers: bool,
    pub language: String,

    // behaviour
    pub difficulty: Difficulty,
    pub freedom_mode: bool,
    pub confidence_mode: ConfidenceMode,
    pub stop_on_error: StopOnError,
    pub strict_space: bool,
    pub quick_end: bool,
    pub quick_restart: QuickRestart,
    pub blind_mode: bool,
    pub lazy_mode: bool,
    pub british_english: bool,
    pub indicate_typos: IndicateTypos,
    pub hide_extra_letters: bool,
    pub funbox: Vec<String>,

    // fail conditions ("off" => None)
    pub min_wpm: Option<u32>,
    pub min_acc: Option<u32>,
    pub min_burst: Option<u32>,

    // caret
    pub caret_style: CaretStyle,
    pub smooth_caret: SmoothCaret,
    pub pace_caret: PaceCaret,
    pub pace_caret_custom_speed: u32,
    pub pace_caret_style: CaretStyle,

    // live readouts
    pub timer_style: IndicatorStyle,
    pub live_speed_style: IndicatorStyle,
    pub live_acc_style: IndicatorStyle,
    pub live_burst_style: IndicatorStyle,

    // display
    pub theme: String,
    pub highlight_mode: HighlightMode,
    pub flip_test_colors: bool,
    pub colorful_mode: bool,
    pub show_all_lines: bool,
    pub max_line_width: u32,
    pub typing_speed_unit: TypingSpeedUnit,
    pub start_graphs_at_zero: bool,
    pub always_show_decimal_places: bool,
    pub show_out_of_focus_warning: bool,
    pub caps_lock_warning: bool,
    pub repeat_quotes: bool,

    // local-only persistence toggle (offline stand-in for account result saving)
    pub result_saving: bool,
}

impl Default for Config {
    fn default() -> Self {
        // Values mirror frontend/src/ts/constants/default-config.ts
        Config {
            mode: Mode::Time,
            time: 30,
            words: 50,
            quote_length: vec![QuoteLengthBand::Medium],
            custom_text: String::new(),
            punctuation: false,
            numbers: false,
            language: "english".to_string(),
            difficulty: Difficulty::Normal,
            freedom_mode: false,
            confidence_mode: ConfidenceMode::Off,
            stop_on_error: StopOnError::Off,
            strict_space: false,
            quick_end: false,
            quick_restart: QuickRestart::Off,
            blind_mode: false,
            lazy_mode: false,
            british_english: false,
            indicate_typos: IndicateTypos::Off,
            hide_extra_letters: false,
            funbox: vec![],
            min_wpm: None,
            min_acc: None,
            min_burst: None,
            caret_style: CaretStyle::Default,
            smooth_caret: SmoothCaret::Medium,
            pace_caret: PaceCaret::Off,
            pace_caret_custom_speed: 100,
            pace_caret_style: CaretStyle::Default,
            timer_style: IndicatorStyle::Mini,
            live_speed_style: IndicatorStyle::Off,
            live_acc_style: IndicatorStyle::Off,
            live_burst_style: IndicatorStyle::Off,
            theme: "serika_dark".to_string(),
            highlight_mode: HighlightMode::Letter,
            flip_test_colors: false,
            colorful_mode: false,
            show_all_lines: false,
            max_line_width: 0,
            typing_speed_unit: TypingSpeedUnit::Wpm,
            start_graphs_at_zero: true,
            always_show_decimal_places: false,
            show_out_of_focus_warning: true,
            caps_lock_warning: true,
            repeat_quotes: false,
            result_saving: true,
        }
    }
}

impl Config {
    pub fn config_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("com", "monkeytype", "mtype")
            .context("could not resolve a platform config directory")?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    /// Load config from disk; falls back to defaults if the file is missing or
    /// unreadable. Unknown/missing keys are tolerated via `#[serde(default)]`.
    pub fn load() -> Config {
        let Ok(path) = Self::config_path() else {
            return Config::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(s) => toml::from_str(&s).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating config dir {}", parent.display()))?;
        }
        let s = toml::to_string_pretty(self).context("serializing config")?;
        std::fs::write(&path, s).with_context(|| format!("writing config {}", path.display()))?;
        Ok(())
    }
}
