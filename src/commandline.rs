//! The command palette - a fuzzy-searchable list of config-mutating commands,
//! mirroring Monkeytype's command line. Opening it pauses the test; running a
//! command updates + persists config and (for test-affecting settings) restarts.

use crate::config::{
    CaretStyle, ConfidenceMode, Config, Difficulty, HighlightMode, IndicateTypos, IndicatorStyle,
    Mode, PaceCaret, PracticeMode, QuickRestart, QuoteLengthBand, SessionOverrides, SmoothCaret,
    StopOnError, TypingSpeedUnit,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigTab {
    Current,
    Test,
    Behavior,
    Appearance,
    Feedback,
    System,
}

impl ConfigTab {
    pub const ALL: &'static [ConfigTab] = &[
        ConfigTab::Current,
        ConfigTab::Test,
        ConfigTab::Behavior,
        ConfigTab::Appearance,
        ConfigTab::Feedback,
        ConfigTab::System,
    ];

    fn label(self) -> &'static str {
        match self {
            ConfigTab::Current => "Current",
            ConfigTab::Test => "Test",
            ConfigTab::Behavior => "Behavior",
            ConfigTab::Appearance => "Appearance",
            ConfigTab::Feedback => "Feedback",
            ConfigTab::System => "System",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    SetMode(Mode),
    SetTime(u32),
    SetWords(u32),
    SetPractice(PracticeMode, u32),
    SetDifficulty(Difficulty),
    SetQuoteLengthAll,
    SetQuoteLength(QuoteLengthBand),
    ToggleField(BoolField),
    SetCaret(CaretStyle),
    SetSmoothCaret(SmoothCaret),
    SetStopOnError(StopOnError),
    SetConfidence(ConfidenceMode),
    SetQuickRestart(QuickRestart),
    SetIndicateTypos(IndicateTypos),
    SetHighlight(HighlightMode),
    SetLiveSpeed(IndicatorStyle),
    SetLiveAcc(IndicatorStyle),
    SetLiveBurst(IndicatorStyle),
    SetTimerStyle(IndicatorStyle),
    SetSpeedUnit(TypingSpeedUnit),
    SetPaceCaret(PaceCaret),
    SetPaceSpeed(u32),
    SetPaceStyle(CaretStyle),
    SetMinWpm(Option<u32>),
    SetMinAcc(Option<u32>),
    SetMinBurst(Option<u32>),
    SetMaxLineWidth(u32),
    SetTheme(String),
    SetLanguage(String),
    ToggleFunbox(String),
    ClearFunbox,
    ViewStats,
    EditCustomText,
    SavePreset(String),
    LoadPreset(String),
    Quit,
}

#[derive(Debug, Clone, Copy)]
pub enum BoolField {
    Punctuation,
    Numbers,
    FreedomMode,
    BlindMode,
    LazyMode,
    BritishEnglish,
    HideExtraLetters,
    StrictSpace,
    ResultSaving,
    QuickEnd,
    ColorfulMode,
    FlipTestColors,
    ShowAllLines,
    StartGraphsAtZero,
    AlwaysShowDecimalPlaces,
    ShowOutOfFocusWarning,
    CapsLockWarning,
    RepeatQuotes,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub label: String,
    pub action: Action,
}

/// What the app should do after a command runs.
pub enum Outcome {
    Restart,
    StayAndRedraw,
    OpenStats,
    OpenCustomEditor,
    Quit,
}

impl Action {
    pub fn closes_config_workspace(&self) -> bool {
        matches!(
            self,
            Action::ViewStats | Action::EditCustomText | Action::Quit
        )
    }

    fn tab(&self) -> ConfigTab {
        match self {
            Action::SetMode(_)
            | Action::SetTime(_)
            | Action::SetWords(_)
            | Action::SetPractice(_, _)
            | Action::SetDifficulty(_)
            | Action::SetQuoteLengthAll
            | Action::SetQuoteLength(_)
            | Action::SetLanguage(_)
            | Action::EditCustomText
            | Action::ToggleField(
                BoolField::Punctuation
                | BoolField::Numbers
                | BoolField::BritishEnglish
                | BoolField::RepeatQuotes,
            ) => ConfigTab::Test,
            Action::SetStopOnError(_)
            | Action::SetConfidence(_)
            | Action::SetQuickRestart(_)
            | Action::ToggleFunbox(_)
            | Action::ClearFunbox
            | Action::ToggleField(
                BoolField::FreedomMode
                | BoolField::BlindMode
                | BoolField::LazyMode
                | BoolField::HideExtraLetters
                | BoolField::StrictSpace
                | BoolField::QuickEnd,
            ) => ConfigTab::Behavior,
            Action::SetCaret(_)
            | Action::SetSmoothCaret(_)
            | Action::SetIndicateTypos(_)
            | Action::SetHighlight(_)
            | Action::SetMaxLineWidth(_)
            | Action::SetTheme(_)
            | Action::ToggleField(
                BoolField::ColorfulMode | BoolField::FlipTestColors | BoolField::ShowAllLines,
            ) => ConfigTab::Appearance,
            Action::SetLiveSpeed(_)
            | Action::SetLiveAcc(_)
            | Action::SetLiveBurst(_)
            | Action::SetTimerStyle(_)
            | Action::SetSpeedUnit(_)
            | Action::SetPaceCaret(_)
            | Action::SetPaceSpeed(_)
            | Action::SetPaceStyle(_)
            | Action::SetMinWpm(_)
            | Action::SetMinAcc(_)
            | Action::SetMinBurst(_)
            | Action::ToggleField(
                BoolField::ResultSaving
                | BoolField::StartGraphsAtZero
                | BoolField::AlwaysShowDecimalPlaces
                | BoolField::ShowOutOfFocusWarning
                | BoolField::CapsLockWarning,
            ) => ConfigTab::Feedback,
            Action::ViewStats | Action::SavePreset(_) | Action::LoadPreset(_) | Action::Quit => {
                ConfigTab::System
            }
        }
    }

    fn group(&self) -> &'static str {
        match self {
            Action::SetMode(_) => "mode",
            Action::SetTime(_) => "time",
            Action::SetWords(_) => "words",
            Action::SetPractice(_, _) => "practice",
            Action::SetDifficulty(_) => "difficulty",
            Action::SetQuoteLengthAll | Action::SetQuoteLength(_) => "quote length",
            Action::ToggleField(field) => match field {
                BoolField::Punctuation => "punctuation",
                BoolField::Numbers => "numbers",
                BoolField::FreedomMode => "freedom mode",
                BoolField::BlindMode => "blind mode",
                BoolField::LazyMode => "lazy mode",
                BoolField::BritishEnglish => "british english",
                BoolField::HideExtraLetters => "hide extra letters",
                BoolField::StrictSpace => "strict space",
                BoolField::ResultSaving => "result saving",
                BoolField::QuickEnd => "quick end",
                BoolField::ColorfulMode => "colorful mode",
                BoolField::FlipTestColors => "flip test colors",
                BoolField::ShowAllLines => "show all lines",
                BoolField::StartGraphsAtZero => "start graphs at zero",
                BoolField::AlwaysShowDecimalPlaces => "always show decimals",
                BoolField::ShowOutOfFocusWarning => "out of focus warning",
                BoolField::CapsLockWarning => "caps lock warning",
                BoolField::RepeatQuotes => "repeat quotes",
            },
            Action::SetCaret(_) => "caret style",
            Action::SetSmoothCaret(_) => "smooth caret",
            Action::SetStopOnError(_) => "stop on error",
            Action::SetConfidence(_) => "confidence mode",
            Action::SetQuickRestart(_) => "quick restart",
            Action::SetIndicateTypos(_) => "indicate typos",
            Action::SetHighlight(_) => "highlight mode",
            Action::SetLiveSpeed(_) => "live speed",
            Action::SetLiveAcc(_) => "live accuracy",
            Action::SetLiveBurst(_) => "live burst",
            Action::SetTimerStyle(_) => "timer / progress",
            Action::SetSpeedUnit(_) => "speed unit",
            Action::SetPaceCaret(_) => "pace caret",
            Action::SetPaceSpeed(_) => "pace speed",
            Action::SetPaceStyle(_) => "pace style",
            Action::SetMinWpm(_) => "minimum wpm",
            Action::SetMinAcc(_) => "minimum accuracy",
            Action::SetMinBurst(_) => "minimum burst",
            Action::SetMaxLineWidth(_) => "max line width",
            Action::SetTheme(_) => "theme",
            Action::SetLanguage(_) => "language",
            Action::ToggleFunbox(_) | Action::ClearFunbox => "funbox",
            Action::ViewStats => "stats / progress",
            Action::EditCustomText => "custom text",
            Action::SavePreset(_) | Action::LoadPreset(_) => "presets",
            Action::Quit => "quit mtype",
        }
    }

    /// Clear the session-only CLI override marks for the fields this action
    /// sets, so the explicit palette change persists to config.toml (CLI flags
    /// are "this run only"; see `SessionOverrides`).
    pub fn clear_session_overrides(&self, o: &mut SessionOverrides) {
        match self {
            Action::SetMode(_) => o.mode = false,
            Action::SetTime(_) => o.time = false,
            Action::SetWords(_) => o.words = false,
            Action::SetPractice(_, _) => {
                o.practice_mode = false;
                o.practice_word_count = false;
            }
            Action::SetDifficulty(_) => o.difficulty = false,
            Action::ToggleField(BoolField::Punctuation) => o.punctuation = false,
            Action::ToggleField(BoolField::Numbers) => o.numbers = false,
            Action::SetLanguage(_) => o.language = false,
            // a loaded preset replaces the whole config on purpose
            Action::LoadPreset(_) => *o = SessionOverrides::default(),
            _ => {}
        }
    }

    pub fn apply(&self, c: &mut Config) -> Outcome {
        match self {
            Action::SetMode(m) => {
                c.mode = *m;
                // changing mode releases an explicit `--quote-id` pin
                c.quote_id = None;
                Outcome::Restart
            }
            Action::SetTime(t) => {
                c.time = *t;
                Outcome::Restart
            }
            Action::SetWords(w) => {
                c.words = *w;
                Outcome::Restart
            }
            Action::SetPractice(mode, words) => {
                c.practice_mode = *mode;
                c.practice_word_count = *words;
                Outcome::Restart
            }
            Action::SetDifficulty(d) => {
                c.difficulty = *d;
                Outcome::Restart
            }
            Action::SetQuoteLengthAll => {
                c.quote_length = QuoteLengthBand::ALL.to_vec();
                // a new band releases the pinned quote so it can take effect
                c.quote_id = None;
                Outcome::Restart
            }
            Action::SetQuoteLength(b) => {
                c.quote_length = vec![*b];
                // a new band releases the pinned quote so it can take effect
                c.quote_id = None;
                Outcome::Restart
            }
            Action::ToggleField(f) => {
                let restart = toggle_field(c, *f);
                if restart {
                    Outcome::Restart
                } else {
                    Outcome::StayAndRedraw
                }
            }
            Action::SetCaret(s) => {
                c.caret_style = *s;
                Outcome::StayAndRedraw
            }
            Action::SetSmoothCaret(s) => {
                c.smooth_caret = *s;
                Outcome::StayAndRedraw
            }
            Action::SetStopOnError(s) => {
                c.stop_on_error = *s;
                Outcome::Restart
            }
            Action::SetConfidence(m) => {
                c.confidence_mode = *m;
                Outcome::Restart
            }
            Action::SetQuickRestart(q) => {
                c.quick_restart = *q;
                Outcome::StayAndRedraw
            }
            Action::SetIndicateTypos(i) => {
                c.indicate_typos = *i;
                Outcome::StayAndRedraw
            }
            Action::SetHighlight(h) => {
                c.highlight_mode = *h;
                Outcome::StayAndRedraw
            }
            Action::SetLiveSpeed(s) => {
                c.live_speed_style = *s;
                Outcome::StayAndRedraw
            }
            Action::SetLiveAcc(s) => {
                c.live_acc_style = *s;
                Outcome::StayAndRedraw
            }
            Action::SetLiveBurst(s) => {
                c.live_burst_style = *s;
                Outcome::StayAndRedraw
            }
            Action::SetTimerStyle(s) => {
                c.timer_style = *s;
                Outcome::StayAndRedraw
            }
            Action::SetSpeedUnit(u) => {
                c.typing_speed_unit = *u;
                Outcome::StayAndRedraw
            }
            Action::SetPaceCaret(p) => {
                c.pace_caret = *p;
                Outcome::Restart
            }
            Action::SetPaceSpeed(speed) => {
                c.pace_caret_custom_speed = *speed;
                c.pace_caret = PaceCaret::Custom;
                Outcome::Restart
            }
            Action::SetPaceStyle(style) => {
                c.pace_caret_style = *style;
                Outcome::StayAndRedraw
            }
            Action::SetMinWpm(value) => {
                c.min_wpm = *value;
                Outcome::Restart
            }
            Action::SetMinAcc(value) => {
                c.min_acc = *value;
                Outcome::Restart
            }
            Action::SetMinBurst(value) => {
                c.min_burst = *value;
                Outcome::Restart
            }
            Action::SetMaxLineWidth(value) => {
                c.max_line_width = *value;
                Outcome::StayAndRedraw
            }
            Action::SetTheme(name) => {
                c.theme = name.clone();
                Outcome::StayAndRedraw
            }
            Action::SetLanguage(name) => {
                c.language = name.clone();
                // quote ids are per-language: keeping the pin would look the
                // old id up in the new language's collection
                c.quote_id = None;
                Outcome::Restart
            }
            Action::ToggleFunbox(name) => {
                if let Some(pos) = c.funbox.iter().position(|f| f == name) {
                    c.funbox.remove(pos);
                } else {
                    c.funbox.push(name.clone());
                }
                Outcome::Restart
            }
            Action::ClearFunbox => {
                c.funbox.clear();
                Outcome::Restart
            }
            Action::ViewStats => Outcome::OpenStats,
            Action::EditCustomText => Outcome::OpenCustomEditor,
            Action::SavePreset(name) => {
                let _ = crate::presets::save(name, c);
                Outcome::StayAndRedraw
            }
            Action::LoadPreset(name) => {
                if let Some(preset) = crate::presets::load(name) {
                    *c = preset;
                    Outcome::Restart
                } else {
                    Outcome::StayAndRedraw
                }
            }
            Action::Quit => Outcome::Quit,
        }
    }
}

/// Returns whether toggling requires a test restart.
fn toggle_field(c: &mut Config, f: BoolField) -> bool {
    match f {
        BoolField::Punctuation => {
            c.punctuation = !c.punctuation;
            true
        }
        BoolField::Numbers => {
            c.numbers = !c.numbers;
            true
        }
        BoolField::FreedomMode => {
            c.freedom_mode = !c.freedom_mode;
            false
        }
        BoolField::BlindMode => {
            c.blind_mode = !c.blind_mode;
            false
        }
        BoolField::LazyMode => {
            c.lazy_mode = !c.lazy_mode;
            true
        }
        BoolField::BritishEnglish => {
            c.british_english = !c.british_english;
            true
        }
        BoolField::HideExtraLetters => {
            c.hide_extra_letters = !c.hide_extra_letters;
            false
        }
        BoolField::StrictSpace => {
            c.strict_space = !c.strict_space;
            false
        }
        BoolField::ResultSaving => {
            c.result_saving = !c.result_saving;
            false
        }
        BoolField::QuickEnd => {
            c.quick_end = !c.quick_end;
            true
        }
        BoolField::ColorfulMode => {
            c.colorful_mode = !c.colorful_mode;
            false
        }
        BoolField::FlipTestColors => {
            c.flip_test_colors = !c.flip_test_colors;
            false
        }
        BoolField::ShowAllLines => {
            c.show_all_lines = !c.show_all_lines;
            false
        }
        BoolField::StartGraphsAtZero => {
            c.start_graphs_at_zero = !c.start_graphs_at_zero;
            false
        }
        BoolField::AlwaysShowDecimalPlaces => {
            c.always_show_decimal_places = !c.always_show_decimal_places;
            false
        }
        BoolField::ShowOutOfFocusWarning => {
            c.show_out_of_focus_warning = !c.show_out_of_focus_warning;
            false
        }
        BoolField::CapsLockWarning => {
            c.caps_lock_warning = !c.caps_lock_warning;
            false
        }
        BoolField::RepeatQuotes => {
            c.repeat_quotes = !c.repeat_quotes;
            false
        }
    }
}

fn on_off(b: bool) -> &'static str {
    if b {
        "on"
    } else {
        "off"
    }
}

/// Build the full command list, annotating the currently-active value.
pub fn all_commands(c: &Config) -> Vec<Command> {
    let mut v: Vec<Command> = Vec::new();
    let mut push = |label: String, action: Action| v.push(Command { label, action });

    // mode
    for m in Mode::ALL {
        let active = if c.mode == *m { " •" } else { "" };
        push(format!("mode > {m}{active}"), Action::SetMode(*m));
    }
    // time
    for t in [15u32, 30, 60, 120] {
        let active = if c.time == t { " •" } else { "" };
        push(format!("time > {t}{active}"), Action::SetTime(t));
    }
    // words
    for w in [10u32, 25, 50, 100] {
        let active = if c.words == w { " •" } else { "" };
        push(format!("words > {w}{active}"), Action::SetWords(w));
    }
    for mode in PracticeMode::ALL {
        for words in [10u32, 25, 50, 100] {
            let active = if c.practice_mode == *mode && c.practice_word_count == words {
                " •"
            } else {
                ""
            };
            push(
                format!("practice > {mode} > {words} words{active}"),
                Action::SetPractice(*mode, words),
            );
        }
    }
    // quote length
    let all_quote_lengths = QuoteLengthBand::ALL
        .iter()
        .all(|band| c.quote_length.contains(band));
    push(
        format!(
            "quote length > all{}",
            if all_quote_lengths { " •" } else { "" }
        ),
        Action::SetQuoteLengthAll,
    );
    for b in QuoteLengthBand::ALL {
        let active = if c.quote_length.len() == 1 && c.quote_length.contains(b) {
            " •"
        } else {
            ""
        };
        push(
            format!("quote length > {b}{active}"),
            Action::SetQuoteLength(*b),
        );
    }
    // toggles
    push(
        format!("punctuation > {} (toggle)", on_off(c.punctuation)),
        Action::ToggleField(BoolField::Punctuation),
    );
    push(
        format!("numbers > {} (toggle)", on_off(c.numbers)),
        Action::ToggleField(BoolField::Numbers),
    );
    push(
        format!("freedom mode > {} (toggle)", on_off(c.freedom_mode)),
        Action::ToggleField(BoolField::FreedomMode),
    );
    push(
        format!("blind mode > {} (toggle)", on_off(c.blind_mode)),
        Action::ToggleField(BoolField::BlindMode),
    );
    push(
        format!("lazy mode > {} (toggle)", on_off(c.lazy_mode)),
        Action::ToggleField(BoolField::LazyMode),
    );
    push(
        format!("british english > {} (toggle)", on_off(c.british_english)),
        Action::ToggleField(BoolField::BritishEnglish),
    );
    push(
        format!(
            "hide extra letters > {} (toggle)",
            on_off(c.hide_extra_letters)
        ),
        Action::ToggleField(BoolField::HideExtraLetters),
    );
    push(
        format!("strict space > {} (toggle)", on_off(c.strict_space)),
        Action::ToggleField(BoolField::StrictSpace),
    );
    push(
        format!("result saving > {} (toggle)", on_off(c.result_saving)),
        Action::ToggleField(BoolField::ResultSaving),
    );
    for (label, value, field) in [
        ("quick end", c.quick_end, BoolField::QuickEnd),
        ("colorful mode", c.colorful_mode, BoolField::ColorfulMode),
        (
            "flip test colors",
            c.flip_test_colors,
            BoolField::FlipTestColors,
        ),
        ("show all lines", c.show_all_lines, BoolField::ShowAllLines),
        (
            "start graphs at zero",
            c.start_graphs_at_zero,
            BoolField::StartGraphsAtZero,
        ),
        (
            "always show decimals",
            c.always_show_decimal_places,
            BoolField::AlwaysShowDecimalPlaces,
        ),
        (
            "out of focus warning",
            c.show_out_of_focus_warning,
            BoolField::ShowOutOfFocusWarning,
        ),
        (
            "caps lock warning",
            c.caps_lock_warning,
            BoolField::CapsLockWarning,
        ),
        ("repeat quotes", c.repeat_quotes, BoolField::RepeatQuotes),
    ] {
        push(
            format!("{label} > {} (toggle)", on_off(value)),
            Action::ToggleField(field),
        );
    }
    // difficulty
    for d in Difficulty::ALL {
        let active = if c.difficulty == *d { " •" } else { "" };
        push(
            format!("difficulty > {d}{active}"),
            Action::SetDifficulty(*d),
        );
    }
    // caret
    for s in CaretStyle::ALL {
        let active = if c.caret_style == *s { " •" } else { "" };
        push(format!("caret style > {s}{active}"), Action::SetCaret(*s));
    }
    for s in SmoothCaret::ALL {
        let active = if c.smooth_caret == *s { " •" } else { "" };
        push(
            format!("smooth caret > {s}{active}"),
            Action::SetSmoothCaret(*s),
        );
    }
    for p in PaceCaret::ALL {
        let active = if c.pace_caret == *p { " •" } else { "" };
        push(
            format!("pace caret > {p}{active}"),
            Action::SetPaceCaret(*p),
        );
    }
    for speed in [30u32, 60, 90, 120, 150, 200] {
        let active = if c.pace_caret == PaceCaret::Custom && c.pace_caret_custom_speed == speed {
            " •"
        } else {
            ""
        };
        push(
            format!("pace caret speed > {speed} wpm{active}"),
            Action::SetPaceSpeed(speed),
        );
    }
    for style in CaretStyle::ALL {
        let active = if c.pace_caret_style == *style {
            " •"
        } else {
            ""
        };
        push(
            format!("pace caret style > {style}{active}"),
            Action::SetPaceStyle(*style),
        );
    }
    // behaviour
    for s in StopOnError::ALL {
        let active = if c.stop_on_error == *s { " •" } else { "" };
        push(
            format!("stop on error > {s}{active}"),
            Action::SetStopOnError(*s),
        );
    }
    for m in ConfidenceMode::ALL {
        let active = if c.confidence_mode == *m { " •" } else { "" };
        push(
            format!("confidence mode > {m}{active}"),
            Action::SetConfidence(*m),
        );
    }
    for q in QuickRestart::ALL {
        let active = if c.quick_restart == *q { " •" } else { "" };
        push(
            format!("quick restart > {q}{active}"),
            Action::SetQuickRestart(*q),
        );
    }
    for i in IndicateTypos::ALL {
        let active = if c.indicate_typos == *i { " •" } else { "" };
        push(
            format!("indicate typos > {i}{active}"),
            Action::SetIndicateTypos(*i),
        );
    }
    for h in HighlightMode::ALL {
        let active = if c.highlight_mode == *h { " •" } else { "" };
        push(
            format!("highlight mode > {h}{active}"),
            Action::SetHighlight(*h),
        );
    }
    push(
        format!(
            "minimum wpm > off{}",
            if c.min_wpm.is_none() { " •" } else { "" }
        ),
        Action::SetMinWpm(None),
    );
    for value in [20u32, 40, 60, 80, 100, 120] {
        let active = if c.min_wpm == Some(value) { " •" } else { "" };
        push(
            format!("minimum wpm > {value}{active}"),
            Action::SetMinWpm(Some(value)),
        );
    }
    push(
        format!(
            "minimum accuracy > off{}",
            if c.min_acc.is_none() { " •" } else { "" }
        ),
        Action::SetMinAcc(None),
    );
    for value in [80u32, 90, 95, 98, 100] {
        let active = if c.min_acc == Some(value) { " •" } else { "" };
        push(
            format!("minimum accuracy > {value}%{active}"),
            Action::SetMinAcc(Some(value)),
        );
    }
    push(
        format!(
            "minimum burst > off{}",
            if c.min_burst.is_none() { " •" } else { "" }
        ),
        Action::SetMinBurst(None),
    );
    for value in [20u32, 40, 60, 80, 100, 120] {
        let active = if c.min_burst == Some(value) {
            " •"
        } else {
            ""
        };
        push(
            format!("minimum burst > {value} wpm{active}"),
            Action::SetMinBurst(Some(value)),
        );
    }
    // live readouts
    for s in IndicatorStyle::ALL {
        let active = if c.live_speed_style == *s { " •" } else { "" };
        push(
            format!("live speed > {s}{active}"),
            Action::SetLiveSpeed(*s),
        );
    }
    for s in IndicatorStyle::ALL {
        let active = if c.live_acc_style == *s { " •" } else { "" };
        push(format!("live acc > {s}{active}"), Action::SetLiveAcc(*s));
    }
    for s in IndicatorStyle::ALL {
        let active = if c.live_burst_style == *s { " •" } else { "" };
        push(
            format!("live burst > {s}{active}"),
            Action::SetLiveBurst(*s),
        );
    }
    for s in IndicatorStyle::ALL {
        let active = if c.timer_style == *s { " •" } else { "" };
        push(
            format!("timer/progress style > {s}{active}"),
            Action::SetTimerStyle(*s),
        );
    }
    for u in TypingSpeedUnit::ALL {
        let active = if c.typing_speed_unit == *u {
            " •"
        } else {
            ""
        };
        push(
            format!("speed unit > {u}{active}"),
            Action::SetSpeedUnit(*u),
        );
    }
    for width in [0u32, 40, 60, 80, 100, 120] {
        let active = if c.max_line_width == width {
            " •"
        } else {
            ""
        };
        let label = if width == 0 {
            "full".to_string()
        } else {
            width.to_string()
        };
        push(
            format!("max line width > {label}{active}"),
            Action::SetMaxLineWidth(width),
        );
    }
    // bundled and locally synced languages
    for name in crate::content::available_language_names() {
        let active = if c.language == name { " •" } else { "" };
        push(
            format!("language > {name}{active}"),
            Action::SetLanguage(name),
        );
    }
    // theme
    for name in crate::theme::Theme::available_names() {
        let active = if c.theme == name { " •" } else { "" };
        push(format!("theme > {name}{active}"), Action::SetTheme(name));
    }
    // funbox
    push("funbox > clear all".to_string(), Action::ClearFunbox);
    for fb in crate::funbox::SUPPORTED {
        let name = fb.name();
        let active = if c.funbox.iter().any(|f| f == name) {
            " •"
        } else {
            ""
        };
        push(
            format!("funbox > {name}{active} (toggle)"),
            Action::ToggleFunbox(name.to_string()),
        );
    }
    // quit
    push("custom text > edit".to_string(), Action::EditCustomText);
    for slot in 1..=5 {
        let name = format!("slot {slot}");
        push(
            format!("preset > save > {name}"),
            Action::SavePreset(name.clone()),
        );
        if crate::presets::names().iter().any(|preset| preset == &name) {
            push(format!("preset > load > {name}"), Action::LoadPreset(name));
        }
    }
    push("view stats / progress".to_string(), Action::ViewStats);
    push("quit mtype".to_string(), Action::Quit);

    v
}

/// Case-insensitive subsequence match (fuzzy). Empty query matches everything.
pub fn fuzzy_match(label: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let label = label.to_lowercase();
    let mut chars = label.chars();
    for qc in query.to_lowercase().chars() {
        if qc == ' ' {
            continue;
        }
        loop {
            match chars.next() {
                Some(lc) if lc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// Live command-palette state.
pub struct CommandLine {
    pub query: String,
    pub selected: usize,
    pub commands: Vec<Command>,
    pub tab: ConfigTab,
    pub group: Option<&'static str>,
    mode: Mode,
}

/// Test controls shared by every mode stay at the top. Everything after them
/// is relevant only to the active mode, so changing a value can never select a
/// different test type as a side effect.
fn test_groups(mode: Mode) -> &'static [&'static str] {
    match mode {
        Mode::Time => &[
            "mode",
            "british english",
            "language",
            "time",
            "difficulty",
            "punctuation",
            "numbers",
        ],
        Mode::Words => &[
            "mode",
            "british english",
            "language",
            "words",
            "difficulty",
            "punctuation",
            "numbers",
        ],
        Mode::Quote => &[
            "mode",
            "british english",
            "language",
            "quote length",
            "difficulty",
            "repeat quotes",
        ],
        Mode::Zen => &["mode", "british english", "language"],
        Mode::Custom => &[
            "mode",
            "british english",
            "language",
            "custom text",
            "difficulty",
        ],
        Mode::Practice => &[
            "mode",
            "british english",
            "language",
            "practice",
            "difficulty",
            "punctuation",
            "numbers",
        ],
    }
}

impl CommandLine {
    pub fn new(config: &Config) -> CommandLine {
        CommandLine {
            query: String::new(),
            selected: 0,
            commands: all_commands(config),
            tab: ConfigTab::Current,
            group: None,
            mode: config.mode,
        }
    }

    pub fn filtered(&self) -> Vec<usize> {
        if self.query.is_empty() && self.group.is_none() {
            return Vec::new();
        }
        let visible_groups = self.groups();
        self.commands
            .iter()
            .enumerate()
            .filter(|(_, cmd)| self.tab == ConfigTab::Current || cmd.action.tab() == self.tab)
            .filter(|(_, cmd)| {
                self.tab != ConfigTab::Test || visible_groups.contains(&cmd.action.group())
            })
            .filter(|(_, cmd)| self.group.is_none_or(|group| cmd.action.group() == group))
            .filter(|(_, cmd)| fuzzy_match(&cmd.label, &self.query))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn groups(&self) -> Vec<&'static str> {
        let groups: &[&'static str] = match self.tab {
            ConfigTab::Current => &[],
            ConfigTab::Test => test_groups(self.mode),
            ConfigTab::Behavior => &[
                "stop on error",
                "confidence mode",
                "strict space",
                "quick restart",
                "quick end",
                "freedom mode",
                "blind mode",
                "lazy mode",
                "hide extra letters",
                "funbox",
            ],
            ConfigTab::Appearance => &[
                "theme",
                "caret style",
                "smooth caret",
                "highlight mode",
                "indicate typos",
                "max line width",
                "show all lines",
                "colorful mode",
                "flip test colors",
            ],
            ConfigTab::Feedback => &[
                "timer / progress",
                "live speed",
                "live accuracy",
                "live burst",
                "speed unit",
                "pace caret",
                "pace speed",
                "pace style",
                "minimum wpm",
                "minimum accuracy",
                "minimum burst",
                "result saving",
                "start graphs at zero",
                "always show decimals",
                "out of focus warning",
                "caps lock warning",
            ],
            ConfigTab::System => &["stats / progress", "presets", "quit mtype"],
        };
        groups.to_vec()
    }

    pub fn move_selection(&mut self, delta: i32) {
        if self.tab == ConfigTab::Current && self.query.is_empty() {
            self.selected = if delta < 0 {
                self.selected.saturating_sub(delta.unsigned_abs() as usize)
            } else {
                self.selected.saturating_add(delta as usize).min(64)
            };
            return;
        }
        let len = if self.query.is_empty() && self.group.is_none() {
            self.groups().len()
        } else {
            self.filtered().len()
        };
        if len == 0 {
            self.selected = 0;
            return;
        }
        let cur = self.selected.min(len - 1) as i32;
        let next = (cur + delta).rem_euclid(len as i32);
        self.selected = next as usize;
    }

    pub fn push_char(&mut self, ch: char) {
        self.group = None;
        self.query.push(ch);
        self.selected = 0;
    }

    pub fn pop_char(&mut self) {
        if self.query.is_empty() {
            self.group = None;
        } else {
            self.query.pop();
        }
        self.selected = 0;
    }

    pub fn next_tab(&mut self, delta: i32) {
        let current = ConfigTab::ALL
            .iter()
            .position(|tab| *tab == self.tab)
            .unwrap_or(0) as i32;
        let next = (current + delta).rem_euclid(ConfigTab::ALL.len() as i32) as usize;
        self.select_tab(next);
    }

    pub fn select_tab(&mut self, index: usize) {
        if let Some(tab) = ConfigTab::ALL.get(index) {
            self.tab = *tab;
            self.query.clear();
            self.group = None;
            self.selected = 0;
        }
    }

    pub fn at_root(&self) -> bool {
        self.query.is_empty() && self.group.is_none()
    }

    pub fn close_group(&mut self) -> bool {
        if self.group.is_some() {
            self.group = None;
            self.selected = 0;
            true
        } else {
            false
        }
    }

    /// Open the selected setting or return the selected concrete action.
    pub fn activate(&mut self) -> Option<Action> {
        if self.tab == ConfigTab::Current && self.query.is_empty() {
            return None;
        }
        if self.query.is_empty() && self.group.is_none() {
            let group = self.groups().get(self.selected).copied()?;
            let mut actions = self
                .commands
                .iter()
                .filter(|command| command.action.tab() == self.tab)
                .filter(|command| command.action.group() == group)
                .map(|command| command.action.clone());
            let first = actions.next()?;
            if actions.next().is_none() {
                return Some(first);
            }
            self.group = Some(group);
            self.selected = 0;
            return None;
        }
        self.current_action()
    }

    pub fn refresh(&mut self, config: &Config) {
        self.commands = all_commands(config);
        self.mode = config.mode;
        let len = if self.tab == ConfigTab::Current && self.query.is_empty() {
            65
        } else if self.query.is_empty() && self.group.is_none() {
            self.groups().len()
        } else {
            self.filtered().len()
        };
        self.selected = self.selected.min(len.saturating_sub(1));
    }

    /// The command currently selected, if any.
    pub fn current_action(&self) -> Option<Action> {
        let filtered = self.filtered();
        let idx = *filtered.get(self.selected.min(filtered.len().saturating_sub(1)))?;
        Some(self.commands[idx].action.clone())
    }
}

fn option_value(value: Option<u32>, suffix: &str) -> String {
    value
        .map(|number| format!("{number}{suffix}"))
        .unwrap_or_else(|| "off".to_string())
}

fn current_value(group: &str, config: &Config) -> String {
    match group {
        "mode" => config.mode.to_string(),
        "time" => format!("{} seconds", config.time),
        "words" => config.words.to_string(),
        "practice" => format!(
            "{} · {} words",
            config.practice_mode, config.practice_word_count
        ),
        "quote length" => config
            .quote_length
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", "),
        "punctuation" => on_off(config.punctuation).to_string(),
        "numbers" => on_off(config.numbers).to_string(),
        "difficulty" => config.difficulty.to_string(),
        "language" => config.language.clone(),
        "freedom mode" => on_off(config.freedom_mode).to_string(),
        "blind mode" => on_off(config.blind_mode).to_string(),
        "lazy mode" => on_off(config.lazy_mode).to_string(),
        "british english" => on_off(config.british_english).to_string(),
        "hide extra letters" => on_off(config.hide_extra_letters).to_string(),
        "strict space" => on_off(config.strict_space).to_string(),
        "quick end" => on_off(config.quick_end).to_string(),
        "stop on error" => config.stop_on_error.to_string(),
        "confidence mode" => config.confidence_mode.to_string(),
        "quick restart" => config.quick_restart.to_string(),
        "repeat quotes" => on_off(config.repeat_quotes).to_string(),
        "funbox" => {
            if config.funbox.is_empty() {
                "off".to_string()
            } else {
                config.funbox.join(", ")
            }
        }
        "caret style" => config.caret_style.to_string(),
        "smooth caret" => config.smooth_caret.to_string(),
        "indicate typos" => config.indicate_typos.to_string(),
        "highlight mode" => config.highlight_mode.to_string(),
        "max line width" => {
            if config.max_line_width == 0 {
                "full".to_string()
            } else {
                config.max_line_width.to_string()
            }
        }
        "theme" => config.theme.clone(),
        "colorful mode" => on_off(config.colorful_mode).to_string(),
        "flip test colors" => on_off(config.flip_test_colors).to_string(),
        "show all lines" => on_off(config.show_all_lines).to_string(),
        "result saving" => on_off(config.result_saving).to_string(),
        "pace caret" => config.pace_caret.to_string(),
        "pace speed" => format!("{} wpm", config.pace_caret_custom_speed),
        "pace style" => config.pace_caret_style.to_string(),
        "minimum wpm" => option_value(config.min_wpm, " wpm"),
        "minimum accuracy" => option_value(config.min_acc, "%"),
        "minimum burst" => option_value(config.min_burst, " wpm"),
        "live speed" => config.live_speed_style.to_string(),
        "live accuracy" => config.live_acc_style.to_string(),
        "live burst" => config.live_burst_style.to_string(),
        "timer / progress" => config.timer_style.to_string(),
        "speed unit" => config.typing_speed_unit.to_string(),
        "start graphs at zero" => on_off(config.start_graphs_at_zero).to_string(),
        "always show decimals" => on_off(config.always_show_decimal_places).to_string(),
        "out of focus warning" => on_off(config.show_out_of_focus_warning).to_string(),
        "caps lock warning" => on_off(config.caps_lock_warning).to_string(),
        "custom text" => {
            if config.custom_text.is_empty() {
                "empty".to_string()
            } else {
                format!("{} chars", config.custom_text.chars().count())
            }
        }
        "presets" => format!("{} saved", crate::presets::names().len()),
        "stats / progress" => "open".to_string(),
        "quit mtype" => "exit".to_string(),
        _ => String::new(),
    }
}

struct SummarySection {
    title: &'static str,
    rows: Vec<(&'static str, String)>,
}

fn current_sections(config: &Config) -> Vec<SummarySection> {
    let values = |groups: &[&'static str]| {
        groups
            .iter()
            .map(|group| (*group, current_value(group, config)))
            .collect()
    };
    vec![
        SummarySection {
            title: "Test",
            rows: values(test_groups(config.mode)),
        },
        SummarySection {
            title: "Behavior",
            rows: values(&[
                "freedom mode",
                "blind mode",
                "lazy mode",
                "hide extra letters",
                "strict space",
                "quick end",
                "stop on error",
                "confidence mode",
                "quick restart",
                "funbox",
            ]),
        },
        SummarySection {
            title: "Appearance",
            rows: values(&[
                "theme",
                "caret style",
                "smooth caret",
                "indicate typos",
                "highlight mode",
                "max line width",
                "colorful mode",
                "flip test colors",
                "show all lines",
            ]),
        },
        SummarySection {
            title: "Feedback",
            rows: values(&[
                "timer / progress",
                "live speed",
                "live accuracy",
                "live burst",
                "speed unit",
                "pace caret",
                "pace speed",
                "pace style",
                "minimum wpm",
                "minimum accuracy",
                "minimum burst",
                "start graphs at zero",
                "always show decimals",
                "out of focus warning",
                "caps lock warning",
                "result saving",
            ]),
        },
    ]
}

/// Render the config workspace as a large lazy.nvim-inspired overlay.
pub fn render(app: &crate::app::App, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    use ratatui::layout::{Constraint, Layout};
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs};

    let Some(cl) = app.command_line.as_ref() else {
        return;
    };
    let t = &app.theme;

    let width = 112u16.min(area.width.saturating_sub(2));
    let height = 34u16.min(area.height.saturating_sub(2));
    if width < 28 || height < 9 {
        return;
    }
    let rect = crate::ui::center_rect(area, width, height);
    frame.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::styled(
                " mtype ",
                Style::default()
                    .fg(t.bg)
                    .bg(t.main)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" config ", Style::default().fg(t.text)),
        ]))
        .border_style(Style::default().fg(t.sub))
        .style(Style::default().bg(t.sub_alt));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);

    let compact = rows[0].width < 70;
    let titles = ConfigTab::ALL.iter().enumerate().map(|(index, tab)| {
        let label = if compact {
            match tab {
                ConfigTab::Current => "Cur",
                ConfigTab::Test => "Test",
                ConfigTab::Behavior => "Beh",
                ConfigTab::Appearance => "Look",
                ConfigTab::Feedback => "Feed",
                ConfigTab::System => "Sys",
            }
        } else {
            tab.label()
        };
        Line::from(format!(" {} {label} ", index + 1))
    });
    let active = ConfigTab::ALL
        .iter()
        .position(|tab| *tab == cl.tab)
        .unwrap_or(0);
    frame.render_widget(
        Tabs::new(titles)
            .select(active)
            .divider(Span::styled("│", Style::default().fg(t.sub)))
            .style(Style::default().fg(t.sub))
            .highlight_style(Style::default().fg(t.main).add_modifier(Modifier::BOLD)),
        rows[0],
    );

    let filtered = cl.filtered();
    let context = if !cl.query.is_empty() {
        Line::from(vec![
            Span::styled(" filter › ", Style::default().fg(t.main)),
            Span::styled(cl.query.clone(), Style::default().fg(t.text)),
            Span::styled("▏", Style::default().fg(t.caret)),
            Span::styled(
                format!("   {} matches", filtered.len()),
                Style::default().fg(t.sub),
            ),
        ])
    } else if let Some(group) = cl.group {
        Line::from(vec![
            Span::styled(format!(" {} ", cl.tab.label()), Style::default().fg(t.sub)),
            Span::styled("/", Style::default().fg(t.sub)),
            Span::styled(
                format!(" {group}"),
                Style::default().fg(t.main).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("   current: {}", current_value(group, &app.config)),
                Style::default().fg(t.text),
            ),
            Span::styled("   backspace returns", Style::default().fg(t.sub)),
        ])
    } else if cl.tab == ConfigTab::Current {
        Line::from(vec![
            Span::styled(
                " current configuration",
                Style::default().fg(t.main).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "   type anywhere to search every setting",
                Style::default().fg(t.sub),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!(" {}", cl.tab.label()),
                Style::default().fg(t.main).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "   {} settings   enter opens   type to filter",
                    cl.groups().len()
                ),
                Style::default().fg(t.sub),
            ),
        ])
    };
    frame.render_widget(
        Paragraph::new(context).style(Style::default().bg(t.bg)),
        rows[1],
    );

    if cl.tab == ConfigTab::Current && cl.query.is_empty() {
        render_current(app, frame, rows[2]);
    } else if cl.query.is_empty() && cl.group.is_none() {
        render_groups(app, frame, rows[2]);
    } else {
        render_commands(app, frame, rows[2], &filtered);
    }

    let footer = if rows[3].width >= 76 {
        " tab/shift+tab switch   ↑↓ move   enter open/apply   backspace back   esc close "
    } else {
        " tab switch  ↑↓ move  enter select  esc close "
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(footer, Style::default().fg(t.sub)))),
        rows[3],
    );
}

fn render_current(app: &crate::app::App, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    use ratatui::layout::{Constraint, Layout};
    let sections = current_sections(&app.config);
    if area.width >= 96 {
        let columns = Layout::horizontal([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .spacing(1)
        .split(area);
        render_summary_column(app, frame, columns[0], &sections[..1]);
        render_summary_column(app, frame, columns[1], &sections[1..3]);
        render_summary_column(app, frame, columns[2], &sections[3..]);
    } else if area.width >= 68 {
        let columns = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .spacing(2)
            .split(area);
        render_summary_column(app, frame, columns[0], &sections[..2]);
        render_summary_column(app, frame, columns[1], &sections[2..]);
    } else {
        render_summary_column(app, frame, area, &sections);
    }
}

fn render_summary_column(
    app: &crate::app::App,
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    sections: &[SummarySection],
) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;
    let mut lines = Vec::new();
    for (section_index, section) in sections.iter().enumerate() {
        if section_index > 0 {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            format!(" {}", section.title),
            Style::default()
                .fg(app.theme.main)
                .add_modifier(Modifier::BOLD),
        )));
        for (label, value) in &section.rows {
            let label_width = (area.width as usize / 2).clamp(12, 22);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {label:<label_width$}"),
                    Style::default().fg(app.theme.sub),
                ),
                Span::styled(value.clone(), Style::default().fg(app.theme.text)),
            ]));
        }
    }
    let start = app
        .command_line
        .as_ref()
        .map(|command_line| command_line.selected)
        .unwrap_or(0)
        .min(lines.len().saturating_sub(1));
    let visible = lines
        .into_iter()
        .skip(start)
        .take(area.height as usize)
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(visible), area);
}

fn selected_window(selected: usize, length: usize, height: usize) -> (usize, usize) {
    if length == 0 || height == 0 {
        return (0, 0);
    }
    let selected = selected.min(length - 1);
    let start = if selected >= height {
        selected - height + 1
    } else {
        0
    };
    (selected, start)
}

fn row_text(prefix: &str, label: &str, value: &str, width: u16) -> String {
    let usable = width.saturating_sub(1) as usize;
    let left = format!("{prefix}{label}");
    let spaces = usable.saturating_sub(left.chars().count() + value.chars().count());
    format!("{left}{}{value}", " ".repeat(spaces.max(1)))
}

fn render_groups(app: &crate::app::App, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;
    let Some(command_line) = app.command_line.as_ref() else {
        return;
    };
    let groups = command_line.groups();
    let (selected, start) =
        selected_window(command_line.selected, groups.len(), area.height as usize);
    let lines = groups
        .iter()
        .enumerate()
        .skip(start)
        .take(area.height as usize)
        .map(|(index, group)| {
            let active = index == selected;
            let prefix = if active { " › " } else { "   " };
            let text = row_text(
                prefix,
                group,
                &current_value(group, &app.config),
                area.width,
            );
            let style = if active {
                Style::default()
                    .fg(app.theme.bg)
                    .bg(app.theme.main)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.text)
            };
            Line::from(Span::styled(text, style))
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), area);
}

fn render_commands(
    app: &crate::app::App,
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    filtered: &[usize],
) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;
    let Some(command_line) = app.command_line.as_ref() else {
        return;
    };
    if filtered.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" no matches", Style::default().fg(app.theme.error)),
                Span::styled(
                    "   backspace changes the filter",
                    Style::default().fg(app.theme.sub),
                ),
            ])),
            area,
        );
        return;
    }
    let (selected, start) =
        selected_window(command_line.selected, filtered.len(), area.height as usize);
    let lines = filtered
        .iter()
        .enumerate()
        .skip(start)
        .take(area.height as usize)
        .map(|(row, command_index)| {
            let command = &command_line.commands[*command_index];
            let mut label = command.label.as_str();
            if command_line.query.is_empty() && command_line.group.is_some() {
                label = label
                    .split_once(" > ")
                    .map(|(_, option)| option)
                    .unwrap_or(label);
            }
            let active_value = label.ends_with(" •");
            let label = label.strip_suffix(" •").unwrap_or(label);
            let prefix = if row == selected {
                " › "
            } else if active_value {
                " ✓ "
            } else {
                "   "
            };
            let style = if row == selected {
                Style::default()
                    .fg(app.theme.bg)
                    .bg(app.theme.main)
                    .add_modifier(Modifier::BOLD)
            } else if active_value {
                Style::default().fg(app.theme.main)
            } else {
                Style::default().fg(app.theme.text)
            };
            Line::from(Span::styled(format!("{prefix}{label}"), style))
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_matches_subsequence() {
        assert!(fuzzy_match("punctuation > on (toggle)", "punc"));
        assert!(fuzzy_match("mode > words", "mowo"));
        assert!(fuzzy_match("anything", ""));
        assert!(!fuzzy_match("mode > time", "zzz"));
    }

    #[test]
    fn toggle_punctuation_flips_and_restarts() {
        let mut c = Config::default();
        assert!(!c.punctuation);
        let out = Action::ToggleField(BoolField::Punctuation).apply(&mut c);
        assert!(c.punctuation);
        assert!(matches!(out, Outcome::Restart));
    }

    #[test]
    fn mode_specific_settings_do_not_switch_mode() {
        let mut c = Config {
            mode: Mode::Zen,
            ..Config::default()
        };
        Action::SetTime(60).apply(&mut c);
        assert_eq!(c.time, 60);
        assert_eq!(c.mode, Mode::Zen);

        Action::SetWords(100).apply(&mut c);
        assert_eq!(c.words, 100);
        assert_eq!(c.mode, Mode::Zen);

        Action::SetPractice(PracticeMode::Slow, 25).apply(&mut c);
        assert_eq!(c.practice_mode, PracticeMode::Slow);
        assert_eq!(c.practice_word_count, 25);
        assert_eq!(c.mode, Mode::Zen);

        Action::SetQuoteLength(QuoteLengthBand::Short).apply(&mut c);
        assert_eq!(c.quote_length, vec![QuoteLengthBand::Short]);
        assert_eq!(c.mode, Mode::Zen);
    }

    #[test]
    fn only_mode_selection_clears_the_session_mode_override() {
        let mut overrides = SessionOverrides {
            mode: true,
            time: true,
            ..SessionOverrides::default()
        };
        Action::SetTime(60).clear_session_overrides(&mut overrides);
        assert!(!overrides.time);
        assert!(overrides.mode);

        Action::SetMode(Mode::Time).clear_session_overrides(&mut overrides);
        assert!(!overrides.mode);
    }

    #[test]
    fn command_line_filters_and_selects() {
        let c = Config::default();
        let mut cl = CommandLine::new(&c);
        cl.query = "punctuation".to_string();
        let f = cl.filtered();
        assert!(!f.is_empty());
        assert!(cl.current_action().is_some());
    }

    #[test]
    fn tabs_reduce_commands_to_setting_groups() {
        let c = Config::default();
        let mut cl = CommandLine::new(&c);
        cl.select_tab(1);
        let groups = cl.groups();
        assert!(groups.contains(&"mode"));
        assert!(groups.contains(&"british english"));
        assert!(groups.contains(&"language"));
        assert!(groups.contains(&"time"));
        assert!(!groups.contains(&"words"));
        assert!(!groups.contains(&"practice"));
        assert!(!groups.contains(&"quote length"));
        assert!(!groups.contains(&"theme"));
        assert!(cl.filtered().is_empty());

        assert!(cl.activate().is_none());
        assert_eq!(cl.group, Some("mode"));
        assert!(cl
            .filtered()
            .iter()
            .all(|index| cl.commands[*index].action.group() == "mode"));
    }

    #[test]
    fn every_command_is_reachable_from_its_tab() {
        let c = Config::default();
        let mut cl = CommandLine::new(&c);
        for tab_index in 1..ConfigTab::ALL.len() {
            cl.select_tab(tab_index);
            if cl.tab == ConfigTab::Test {
                // Test commands are intentionally mode-dependent and covered
                // by `test_tab_only_shows_active_mode_settings` below.
                continue;
            }
            let groups = cl.groups();
            for command in cl
                .commands
                .iter()
                .filter(|command| command.action.tab() == cl.tab)
            {
                assert!(
                    groups.contains(&command.action.group()),
                    "{} is not reachable from {}",
                    command.label,
                    cl.tab.label()
                );
            }
        }
    }

    #[test]
    fn test_tab_only_shows_active_mode_settings() {
        let cases: &[(Mode, &[&str])] = &[
            (
                Mode::Time,
                &[
                    "mode",
                    "british english",
                    "language",
                    "time",
                    "difficulty",
                    "punctuation",
                    "numbers",
                ],
            ),
            (
                Mode::Words,
                &[
                    "mode",
                    "british english",
                    "language",
                    "words",
                    "difficulty",
                    "punctuation",
                    "numbers",
                ],
            ),
            (
                Mode::Quote,
                &[
                    "mode",
                    "british english",
                    "language",
                    "quote length",
                    "difficulty",
                    "repeat quotes",
                ],
            ),
            (Mode::Zen, &["mode", "british english", "language"]),
            (
                Mode::Custom,
                &[
                    "mode",
                    "british english",
                    "language",
                    "custom text",
                    "difficulty",
                ],
            ),
            (
                Mode::Practice,
                &[
                    "mode",
                    "british english",
                    "language",
                    "practice",
                    "difficulty",
                    "punctuation",
                    "numbers",
                ],
            ),
        ];

        for (mode, expected) in cases {
            let config = Config {
                mode: *mode,
                ..Config::default()
            };
            let mut cl = CommandLine::new(&config);
            cl.select_tab(1);
            assert_eq!(cl.groups(), *expected, "wrong groups for {mode}");

            for group in cl.groups() {
                assert!(
                    cl.commands.iter().any(|command| {
                        command.action.tab() == ConfigTab::Test && command.action.group() == group
                    }),
                    "{group} has no commands in {mode} mode"
                );
            }
        }
    }

    #[test]
    fn test_tab_search_hides_inactive_mode_settings() {
        let mut cl = CommandLine::new(&Config::default()); // time mode
        cl.select_tab(1);
        for ch in "words".chars() {
            cl.push_char(ch);
        }
        let filtered = cl.filtered();
        assert!(filtered
            .iter()
            .any(|index| matches!(cl.commands[*index].action, Action::SetMode(Mode::Words))));
        assert!(filtered
            .iter()
            .all(|index| !matches!(cl.commands[*index].action, Action::SetWords(_))));

        // The Current tab remains a global search, but changing the dormant
        // word count there still cannot change the active mode.
        cl.select_tab(0);
        for ch in "words".chars() {
            cl.push_char(ch);
        }
        assert!(cl
            .filtered()
            .iter()
            .any(|index| matches!(cl.commands[*index].action, Action::SetWords(_))));
    }

    #[test]
    fn current_tab_searches_every_category() {
        let c = Config::default();
        let mut cl = CommandLine::new(&c);
        for ch in "theme nord".chars() {
            cl.push_char(ch);
        }
        assert!(cl.filtered().iter().any(|index| matches!(
            cl.commands[*index].action,
            Action::SetTheme(ref name) if name == "nord"
        )));
    }

    #[test]
    fn tab_change_resets_drill_down_and_filter() {
        let c = Config::default();
        let mut cl = CommandLine::new(&c);
        cl.select_tab(1);
        cl.activate();
        cl.push_char('t');
        cl.next_tab(1);
        assert_eq!(cl.tab, ConfigTab::Behavior);
        assert!(cl.group.is_none());
        assert!(cl.query.is_empty());
        assert_eq!(cl.selected, 0);
    }

    #[test]
    fn single_action_setting_activates_without_drill_down() {
        let c = Config::default();
        let mut cl = CommandLine::new(&c);
        cl.select_tab(2);
        cl.selected = cl
            .groups()
            .iter()
            .position(|group| *group == "freedom mode")
            .unwrap();

        assert!(matches!(
            cl.activate(),
            Some(Action::ToggleField(BoolField::FreedomMode))
        ));
        assert!(cl.group.is_none());
    }

    #[test]
    fn refresh_preserves_context_and_updates_active_values() {
        let mut config = Config::default();
        let mut cl = CommandLine::new(&config);
        cl.select_tab(1);
        cl.activate();
        cl.selected = 1;

        config.mode = Mode::Words;
        cl.refresh(&config);

        assert_eq!(cl.tab, ConfigTab::Test);
        assert_eq!(cl.group, Some("mode"));
        assert_eq!(cl.selected, 1);
        assert!(cl
            .commands
            .iter()
            .any(|command| command.label == "mode > words •"));
    }

    #[test]
    fn navigation_actions_close_the_workspace() {
        assert!(Action::ViewStats.closes_config_workspace());
        assert!(Action::EditCustomText.closes_config_workspace());
        assert!(Action::Quit.closes_config_workspace());
        assert!(!Action::SetMode(Mode::Words).closes_config_workspace());
    }

    #[test]
    fn caret_change_does_not_restart() {
        let mut c = Config::default();
        let out = Action::SetCaret(CaretStyle::Block).apply(&mut c);
        assert!(matches!(out, Outcome::StayAndRedraw));
        assert_eq!(c.caret_style, CaretStyle::Block);
    }
}
