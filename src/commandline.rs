//! The command palette - a fuzzy-searchable list of config-mutating commands,
//! mirroring Monkeytype's command line. Opening it pauses the test; running a
//! command updates + persists config and (for test-affecting settings) restarts.

use crate::config::{
    CaretStyle, ConfidenceMode, Config, Difficulty, HighlightMode, IndicateTypos, IndicatorStyle,
    Mode, PaceCaret, PracticeMode, QuickRestart, QuoteLengthBand, SmoothCaret, StopOnError,
    TypingSpeedUnit,
};

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
    pub fn apply(&self, c: &mut Config) -> Outcome {
        match self {
            Action::SetMode(m) => {
                c.mode = *m;
                Outcome::Restart
            }
            Action::SetTime(t) => {
                c.time = *t;
                c.mode = Mode::Time;
                Outcome::Restart
            }
            Action::SetWords(w) => {
                c.words = *w;
                c.mode = Mode::Words;
                Outcome::Restart
            }
            Action::SetPractice(mode, words) => {
                c.mode = Mode::Practice;
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
                c.mode = Mode::Quote;
                Outcome::Restart
            }
            Action::SetQuoteLength(b) => {
                c.quote_length = vec![*b];
                c.mode = Mode::Quote;
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
        let active = if c.mode == Mode::Time && c.time == t {
            " •"
        } else {
            ""
        };
        push(format!("time > {t}{active}"), Action::SetTime(t));
    }
    // words
    for w in [10u32, 25, 50, 100] {
        let active = if c.mode == Mode::Words && c.words == w {
            " •"
        } else {
            ""
        };
        push(format!("words > {w}{active}"), Action::SetWords(w));
    }
    for mode in PracticeMode::ALL {
        for words in [10u32, 25, 50, 100] {
            let active = if c.mode == Mode::Practice
                && c.practice_mode == *mode
                && c.practice_word_count == words
            {
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
    push("quote length > all".to_string(), Action::SetQuoteLengthAll);
    for b in QuoteLengthBand::ALL {
        push(format!("quote length > {b}"), Action::SetQuoteLength(*b));
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
    push("minimum wpm > off".to_string(), Action::SetMinWpm(None));
    for value in [20u32, 40, 60, 80, 100, 120] {
        push(
            format!("minimum wpm > {value}"),
            Action::SetMinWpm(Some(value)),
        );
    }
    push(
        "minimum accuracy > off".to_string(),
        Action::SetMinAcc(None),
    );
    for value in [80u32, 90, 95, 98, 100] {
        push(
            format!("minimum accuracy > {value}%"),
            Action::SetMinAcc(Some(value)),
        );
    }
    push("minimum burst > off".to_string(), Action::SetMinBurst(None));
    for value in [20u32, 40, 60, 80, 100, 120] {
        push(
            format!("minimum burst > {value} wpm"),
            Action::SetMinBurst(Some(value)),
        );
    }
    // live readouts
    for s in IndicatorStyle::ALL {
        push(format!("live speed > {s}"), Action::SetLiveSpeed(*s));
    }
    for s in IndicatorStyle::ALL {
        push(format!("live acc > {s}"), Action::SetLiveAcc(*s));
    }
    for s in IndicatorStyle::ALL {
        push(format!("live burst > {s}"), Action::SetLiveBurst(*s));
    }
    for s in IndicatorStyle::ALL {
        push(
            format!("timer/progress style > {s}"),
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
}

impl CommandLine {
    pub fn new(config: &Config) -> CommandLine {
        CommandLine {
            query: String::new(),
            selected: 0,
            commands: all_commands(config),
        }
    }

    pub fn filtered(&self) -> Vec<usize> {
        self.commands
            .iter()
            .enumerate()
            .filter(|(_, cmd)| fuzzy_match(&cmd.label, &self.query))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn move_selection(&mut self, delta: i32) {
        let len = self.filtered().len();
        if len == 0 {
            self.selected = 0;
            return;
        }
        let cur = self.selected.min(len - 1) as i32;
        let next = (cur + delta).rem_euclid(len as i32);
        self.selected = next as usize;
    }

    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
        self.selected = 0;
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
        self.selected = 0;
    }

    /// The command currently selected, if any.
    pub fn current_action(&self) -> Option<Action> {
        let filtered = self.filtered();
        let idx = *filtered.get(self.selected.min(filtered.len().saturating_sub(1)))?;
        Some(self.commands[idx].action.clone())
    }
}

/// Render the palette as a centered overlay on top of the current screen.
pub fn render(app: &crate::app::App, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    use ratatui::layout::Rect;
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    let Some(cl) = app.command_line.as_ref() else {
        return;
    };
    let t = &app.theme;

    let width = 56u16.min(area.width.saturating_sub(2));
    let height = 18u16.min(area.height.saturating_sub(2));
    if width < 10 || height < 5 {
        return;
    }
    let rect = crate::ui::center_rect(area, width, height);
    frame.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.main))
        .style(Style::default().bg(t.sub_alt));
    let inner = block.inner(rect);
    frame.render_widget(block, rect);

    // query line
    let query_rect = Rect::new(inner.x, inner.y, inner.width, 1);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("> ", Style::default().fg(t.main)),
            Span::styled(cl.query.clone(), Style::default().fg(t.text)),
            Span::styled("▏", Style::default().fg(t.caret)),
        ])),
        query_rect,
    );

    // list
    let filtered = cl.filtered();
    let list_height = inner.height.saturating_sub(1) as usize;
    if list_height == 0 {
        return;
    }
    let sel = cl.selected.min(filtered.len().saturating_sub(1));
    // scroll window so the selection is visible
    let start = if sel >= list_height {
        sel - list_height + 1
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    for (row, &cmd_idx) in filtered.iter().enumerate().skip(start).take(list_height) {
        let label = &cl.commands[cmd_idx].label;
        let is_sel = row == sel;
        let style = if is_sel {
            Style::default()
                .fg(t.bg)
                .bg(t.main)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.text)
        };
        let prefix = if is_sel { "› " } else { "  " };
        lines.push(Line::from(Span::styled(format!("{prefix}{label}"), style)));
    }
    let list_rect = Rect::new(inner.x, inner.y + 1, inner.width, list_height as u16);
    frame.render_widget(Paragraph::new(lines), list_rect);
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
    fn set_time_switches_mode() {
        let mut c = Config {
            mode: Mode::Words,
            ..Config::default()
        };
        Action::SetTime(60).apply(&mut c);
        assert_eq!(c.mode, Mode::Time);
        assert_eq!(c.time, 60);
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
    fn caret_change_does_not_restart() {
        let mut c = Config::default();
        let out = Action::SetCaret(CaretStyle::Block).apply(&mut c);
        assert!(matches!(out, Outcome::StayAndRedraw));
        assert_eq!(c.caret_style, CaretStyle::Block);
    }
}
