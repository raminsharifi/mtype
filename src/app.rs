//! Top-level application state and the main event loop. Owns config + theme,
//! the current `Engine`, and routes keyboard input to it.

use crate::commandline::{CommandLine, Outcome};
use crate::config::{Config, Mode, QuickRestart, SessionOverrides};
use crate::engine::{Engine, State, TestResult};
use crate::theme::Theme;
use crate::tui::Tui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Test,
    Results,
    Stats,
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultsView {
    Summary,
    InputHistory,
    Replay,
}

pub struct App {
    pub config: Config,
    /// Pristine on-disk config from startup. Saves merge against it so
    /// session-only CLI overrides never leak into config.toml.
    disk_config: Config,
    /// Which config fields were set by CLI flags for this run only.
    session_overrides: SessionOverrides,
    pub theme: Theme,
    pub screen: Screen,
    pub engine: Engine,
    pub result: Option<TestResult>,
    pub pb_info: crate::persistence::PbInfo,
    pub command_line: Option<CommandLine>,
    /// Cached stats, computed when the stats screen is opened.
    pub profile: Option<crate::persistence::Profile>,
    /// Whether the current test has already been counted as "started".
    started_counted: bool,
    pub epoch: Instant,
    /// Completed palette-pause time (ms) excluded from the test clock: opening
    /// the command line pauses a running test, so `now_ms` subtracts this.
    paused_ms: u128,
    /// When the current palette pause began (palette opened mid-test).
    pause_started: Option<Instant>,
    pub should_quit: bool,
    pub pace_wpm: Option<f64>,
    pub focused: bool,
    pub caps_lock: bool,
    pub results_view: ResultsView,
    pub replay_epoch: Option<Instant>,
    pub editor_text: String,
}

impl App {
    #[allow(dead_code)] // convenience constructor exercised by the unit tests
    pub fn new(config: Config) -> App {
        let disk_config = config.clone();
        App::new_session(config, disk_config, SessionOverrides::default())
    }

    /// Build the app for a CLI session: `config` is the effective config
    /// (disk + flags), `disk_config` the pristine on-disk one, and `overrides`
    /// marks the flag-set fields that must stay out of config.toml.
    pub fn new_session(
        mut config: Config,
        disk_config: Config,
        session_overrides: SessionOverrides,
    ) -> App {
        refresh_practice_text(&mut config);
        let theme = Theme::by_name(&config.theme);
        let engine = Engine::new(config.clone(), StdRng::from_entropy());
        let pace_wpm = crate::persistence::pace_wpm(&config);
        App {
            config,
            disk_config,
            session_overrides,
            theme,
            screen: Screen::Test,
            engine,
            result: None,
            pb_info: crate::persistence::PbInfo::default(),
            command_line: None,
            profile: None,
            started_counted: false,
            epoch: Instant::now(),
            paused_ms: 0,
            pause_started: None,
            should_quit: false,
            pace_wpm,
            focused: true,
            caps_lock: false,
            results_view: ResultsView::Summary,
            replay_epoch: None,
            editor_text: String::new(),
        }
    }

    /// The test clock: wall time since `epoch`, minus time spent with the
    /// command palette open during a running test (opening it pauses the test,
    /// so browsing settings is never charged to WPM or the countdown).
    pub fn now_ms(&self) -> u128 {
        let mut now = self
            .epoch
            .elapsed()
            .as_millis()
            .saturating_sub(self.paused_ms);
        if let Some(started) = self.pause_started {
            now = now.saturating_sub(started.elapsed().as_millis());
        }
        now
    }

    /// Fold the in-progress palette pause into `paused_ms`. Called whenever
    /// input returns to the (still running) test.
    fn end_palette_pause(&mut self) {
        if let Some(started) = self.pause_started.take() {
            self.paused_ms += started.elapsed().as_millis();
        }
    }

    /// The config that a save would write right now: live values, except CLI
    /// session-only fields, which keep their on-disk value.
    fn persistable_config(&self) -> Config {
        self.session_overrides
            .merge_for_save(&self.config, &self.disk_config)
    }

    /// Persist the config, honoring the "flags are this run only" contract.
    fn save_config(&self) {
        if cfg!(test) {
            return; // unit tests must never touch the real config.toml
        }
        let _ = self.persistable_config().save();
    }

    /// Start a fresh test with the current config (quick-restart keys and the
    /// results/stats screens). Per upstream `repeatQuotes: "typing"`, only a
    /// restart that interrupts a quote test mid-typing repeats the quote; an
    /// unstarted or finished test draws fresh.
    pub fn restart(&mut self) {
        let repeat_pin = if self.config.mode == Mode::Quote
            && self.config.repeat_quotes
            && self.engine.state() == State::Running
        {
            self.engine.quote.as_ref().map(|quote| quote.id)
        } else {
            None
        };
        self.start_test(repeat_pin);
    }

    /// Rebuild the engine and reset per-test state. `repeat_pin` pins the
    /// quote for this one rebuild only; `config.quote_id` itself holds just an
    /// explicit `--quote-id` request, which stays pinned across restarts until
    /// a palette change to mode / quote length / language clears it (upstream
    /// keeps a specifically selected quote until quote settings change).
    fn start_test(&mut self, repeat_pin: Option<u32>) {
        let explicit_pin = self.config.quote_id;
        // both pins point at the same quote when a --quote-id test is
        // restarted mid-typing, so the repeat pin may take precedence
        self.config.quote_id = repeat_pin.or(explicit_pin);
        refresh_practice_text(&mut self.config);
        self.pace_wpm = crate::persistence::pace_wpm(&self.config);
        self.engine = Engine::new(self.config.clone(), StdRng::from_entropy());
        self.config.quote_id = explicit_pin;
        self.epoch = Instant::now();
        self.paused_ms = 0;
        self.pause_started = None;
        self.result = None;
        self.results_view = ResultsView::Summary;
        self.replay_epoch = None;
        self.started_counted = false;
        self.screen = Screen::Test;
    }

    /// Compute lifetime stats and switch to the stats screen.
    pub fn open_stats(&mut self) {
        // wall-clock now, in ms since the unix epoch, for streak/activity math
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        self.profile = Some(crate::persistence::compute_profile(now));
        self.screen = Screen::Stats;
    }

    /// Count the current test as started the first time typing begins.
    fn maybe_count_start(&mut self) {
        if !self.started_counted && self.engine.state() == State::Running {
            self.started_counted = true;
            if self.config.result_saving {
                crate::persistence::increment_started_tests();
            }
        }
    }

    pub fn run(&mut self, terminal: &mut Tui) -> Result<()> {
        let tick = Duration::from_millis(16);
        while !self.should_quit {
            terminal.draw(|frame| crate::ui::render(self, frame))?;

            if event::poll(tick)? {
                self.on_event(event::read()?);
            }

            // drive timed tests / fail conditions (paused while the palette is open)
            if self.screen == Screen::Test && self.command_line.is_none() {
                let now = self.now_ms();
                self.engine.tick(now);
                self.sync_finish();
            }
        }
        Ok(())
    }

    /// Route one terminal event. Key handling reacts to `Press` only, so the
    /// kitty keyboard protocol enabled in `tui::init` (and Windows' native
    /// release events) never double-types or otherwise changes behavior.
    pub fn on_event(&mut self, event: Event) {
        match event {
            Event::Key(key) => {
                let key = apply_caps_lock_translation(key);
                // crossterm reports the CapsLock key itself with a forced
                // CAPS_LOCK state bit, which says nothing about whether the
                // lock is now on; read the state from other keys only.
                if key.code != KeyCode::CapsLock {
                    self.caps_lock = key.state.contains(KeyEventState::CAPS_LOCK);
                }
                if key.kind == KeyEventKind::Press {
                    self.on_key(key);
                }
            }
            Event::FocusGained => self.focused = true,
            Event::FocusLost => self.focused = false,
            Event::Paste(text) if self.screen == Screen::Editor => {
                self.editor_text
                    .push_str(&text.replace(['\r', '\n', '\t'], " "));
            }
            _ => {}
        }
    }

    fn sync_finish(&mut self) {
        if matches!(self.engine.state(), State::Finished | State::Failed)
            && self.screen == Screen::Test
        {
            let result = self.engine.result();
            self.pb_info = crate::persistence::record(
                &result,
                self.config.difficulty.as_str(),
                self.config.result_saving,
            );
            self.result = Some(result);
            self.results_view = ResultsView::Summary;
            self.replay_epoch = None;
            self.screen = Screen::Results;
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let alt = key.modifiers.contains(KeyModifiers::ALT);

        // global quit
        if ctrl && matches!(key.code, KeyCode::Char('c')) {
            self.should_quit = true;
            return;
        }

        // command palette intercepts all input while open
        if self.command_line.is_some() {
            self.on_key_commandline(key, ctrl);
            return;
        }

        match self.screen {
            Screen::Test => self.on_key_test(key, ctrl, alt),
            Screen::Results => self.on_key_results(key),
            Screen::Stats => self.on_key_stats(key),
            Screen::Editor => self.on_key_editor(key, ctrl),
        }
    }

    fn open_command_line(&mut self) {
        // opening the palette over a running test pauses its clock
        if self.screen == Screen::Test
            && self.engine.state() == State::Running
            && self.pause_started.is_none()
        {
            self.pause_started = Some(Instant::now());
        }
        self.command_line = Some(CommandLine::new(&self.config));
    }

    fn on_key_commandline(&mut self, key: KeyEvent, ctrl: bool) {
        let Some(cl) = self.command_line.as_mut() else {
            return;
        };
        match key.code {
            KeyCode::Esc => {
                self.command_line = None;
                self.end_palette_pause();
            }
            KeyCode::Up => cl.move_selection(-1),
            KeyCode::Down => cl.move_selection(1),
            KeyCode::Tab => cl.next_tab(1),
            KeyCode::BackTab => cl.next_tab(-1),
            KeyCode::Left => {
                if !cl.close_group() {
                    cl.next_tab(-1);
                }
            }
            KeyCode::Right => cl.next_tab(1),
            KeyCode::Backspace => cl.pop_char(),
            KeyCode::Enter => {
                if let Some(action) = cl.activate() {
                    let closes_workspace = action.closes_config_workspace();
                    if closes_workspace {
                        self.command_line = None;
                    }
                    self.execute(action);
                    if !closes_workspace {
                        if let Some(command_line) = self.command_line.as_mut() {
                            command_line.refresh(&self.config);
                        }
                    }
                }
            }
            KeyCode::Char('j') if ctrl => cl.move_selection(1),
            KeyCode::Char('k') if ctrl => cl.move_selection(-1),
            KeyCode::Char(c @ '1'..='6') if !ctrl && cl.at_root() => {
                cl.select_tab(c.to_digit(10).unwrap_or(1) as usize - 1);
            }
            KeyCode::Char(c) if !ctrl => cl.push_char(c),
            _ => {}
        }
    }

    fn execute(&mut self, action: crate::commandline::Action) {
        // an explicit palette change persists again, even if a CLI flag set
        // the field for this session (`--time 15` then "time > 60" saves 60)
        action.clear_session_overrides(&mut self.session_overrides);
        match action.apply(&mut self.config) {
            Outcome::Restart => {
                self.save_config();
                self.theme = Theme::by_name(&self.config.theme);
                // a settings change always regenerates the test (upstream
                // draws fresh after config changes); the repeat-quotes
                // mid-typing pin applies to the quick-restart keys only
                self.start_test(None);
            }
            Outcome::StayAndRedraw => {
                self.save_config();
                self.theme = Theme::by_name(&self.config.theme);
                // non-restarting toggles (freedom mode, strict space, ...)
                // apply to the test in progress immediately, as upstream
                self.engine.sync_config(self.config.clone());
            }
            Outcome::OpenStats => self.open_stats(),
            Outcome::OpenCustomEditor => {
                self.editor_text = self.config.custom_text.clone();
                self.screen = Screen::Editor;
            }
            Outcome::Quit => self.should_quit = true,
        }
    }

    /// `no_quit` funbox blocks restarting/leaving a test that's underway.
    fn restart_blocked(&self) -> bool {
        self.engine.state() == State::Running
            && crate::funbox::has_no_quit(&crate::funbox::parse(&self.config.funbox))
    }

    /// Endless tests (zen, time 0, words 0) only finish via the bail-out key,
    /// which ends them with a normal, saveable result (upstream "bail out").
    fn is_endless(&self) -> bool {
        match self.config.mode {
            Mode::Zen => true,
            Mode::Time => self.config.time == 0,
            Mode::Words => self.config.words == 0,
            _ => false,
        }
    }

    fn on_key_test(&mut self, key: KeyEvent, ctrl: bool, alt: bool) {
        let now = self.now_ms();
        match key.code {
            KeyCode::Tab => {
                if self.restart_blocked() {
                    // can't leave mid-test
                } else if self.config.quick_restart == QuickRestart::Esc {
                    // upstream swaps the roles when quickRestart is esc:
                    // tab opens the command line and esc restarts
                    self.open_command_line();
                } else {
                    self.restart();
                }
            }
            KeyCode::Esc => {
                if self.restart_blocked() {
                    // can't bail mid-test
                } else if self.config.quick_restart == QuickRestart::Esc {
                    self.restart();
                } else {
                    self.open_command_line();
                }
            }
            KeyCode::Backspace => {
                self.engine.backspace(ctrl || alt, now);
            }
            KeyCode::Enter => {
                let shift = key.modifiers.contains(KeyModifiers::SHIFT);
                if shift && self.is_endless() {
                    // shift+enter always bails an endless test, even when
                    // quick restart is bound to enter
                    self.engine.bail(now);
                } else if self.config.quick_restart == QuickRestart::Enter {
                    if !self.restart_blocked() {
                        self.restart();
                    }
                } else if self.is_endless() {
                    self.engine.bail(now);
                }
            }
            KeyCode::Char('w') if ctrl => {
                self.engine.backspace(true, now);
            }
            KeyCode::Char(c) if !ctrl && !alt => {
                self.engine.type_char(c, now);
            }
            _ => {}
        }
        self.maybe_count_start();
        self.sync_finish();
    }

    fn on_key_results(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab | KeyCode::Enter => self.restart(),
            KeyCode::Char('s') => self.open_stats(),
            KeyCode::Char('i') => {
                self.results_view = if self.results_view == ResultsView::InputHistory {
                    ResultsView::Summary
                } else {
                    ResultsView::InputHistory
                };
            }
            KeyCode::Char('w') => {
                self.results_view = ResultsView::Replay;
                self.replay_epoch = Some(Instant::now());
            }
            KeyCode::Char('m') => self.start_practice(crate::config::PracticeMode::Missed),
            KeyCode::Char('l') => self.start_practice(crate::config::PracticeMode::Slow),
            KeyCode::Esc if self.results_view != ResultsView::Summary => {
                self.results_view = ResultsView::Summary;
                self.replay_epoch = None;
            }
            KeyCode::Esc => self.open_command_line(),
            KeyCode::Char('q') => self.should_quit = true,
            _ => {}
        }
    }

    fn start_practice(&mut self, mode: crate::config::PracticeMode) {
        // like the `practice` subcommand this is a session-only mode switch:
        // a later palette save must not persist mode=practice as the default
        self.session_overrides.mode = true;
        self.session_overrides.practice_mode = true;
        self.config.mode = Mode::Practice;
        self.config.practice_mode = mode;
        self.restart();
    }

    pub fn replay_elapsed_ms(&self) -> u64 {
        self.replay_epoch
            .map(|epoch| epoch.elapsed().as_millis().min(u64::MAX as u128) as u64)
            .unwrap_or(0)
    }

    fn on_key_stats(&mut self, key: KeyEvent) {
        match key.code {
            // go back to a fresh test
            KeyCode::Tab | KeyCode::Enter | KeyCode::Esc => {
                self.profile = None;
                self.restart();
            }
            KeyCode::Char('q') => self.should_quit = true,
            _ => {}
        }
    }

    fn on_key_editor(&mut self, key: KeyEvent, ctrl: bool) {
        match key.code {
            KeyCode::Char('s') if ctrl => {
                self.config.custom_text = self.editor_text.trim().to_string();
                // Editing the text changes only the custom target. Mode
                // selection is explicit in the Test workspace.
                self.session_overrides.custom_text = false;
                self.save_config();
                self.restart();
            }
            KeyCode::Esc => {
                self.editor_text.clear();
                self.screen = Screen::Test;
                // back to the (possibly still running) test: stop pausing
                self.end_palette_pause();
            }
            KeyCode::Backspace => {
                self.editor_text.pop();
            }
            KeyCode::Enter | KeyCode::Tab => self.editor_text.push(' '),
            KeyCode::Char(character) if !ctrl => self.editor_text.push(character),
            _ => {}
        }
    }
}

/// Kitty-protocol key events carry the un-shifted key plus a CAPS_LOCK state
/// bit instead of pre-translated text (legacy input applies the caps-lock
/// translation in the terminal driver), so mirror the driver here: with caps
/// lock engaged, letters toggle case. Legacy events never carry the state
/// bit, so this is a no-op outside kitty-capable terminals.
fn apply_caps_lock_translation(key: KeyEvent) -> KeyEvent {
    if !key.state.contains(KeyEventState::CAPS_LOCK)
        || key.modifiers.contains(KeyModifiers::CONTROL)
    {
        return key;
    }
    let KeyCode::Char(c) = key.code else {
        return key;
    };
    let toggled = if c.is_lowercase() {
        single_char(c.to_uppercase())
    } else if c.is_uppercase() {
        single_char(c.to_lowercase())
    } else {
        None
    };
    match toggled {
        Some(t) => KeyEvent {
            code: KeyCode::Char(t),
            ..key
        },
        None => key,
    }
}

/// The toggled case of a letter, only when it maps to exactly one char
/// (skips expansions like the German sharp s).
fn single_char(mut chars: impl Iterator<Item = char>) -> Option<char> {
    let first = chars.next();
    chars.next().is_none().then_some(first).flatten()
}

fn refresh_practice_text(config: &mut Config) {
    if config.mode != Mode::Practice {
        return;
    }
    config.practice_text = crate::analytics::practice_words(
        &config.language,
        config.practice_mode,
        config.practice_word_count as usize,
    )
    .unwrap_or_default()
    .join(" ");
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        let buf = terminal.backend().buffer();
        let mut s = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn renders_test_screen_without_panicking() {
        let cfg = Config {
            mode: Mode::Words,
            words: 10,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        // type the first target word's first two chars
        let first = app.engine.target_words[0].clone();
        for c in first.chars().take(2) {
            app.on_key(key(KeyCode::Char(c)));
        }
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| crate::ui::render(&app, f)).unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("0/10"), "expected word counter in: {text}");
        assert!(text.contains("restart"));
    }

    /// Regression: the caret reaching the end of a word (where the space is)
    /// must NOT shift the rest of the line - the words stay put.
    fn first_line_of_words(app: &App) -> String {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| crate::ui::render(app, f)).unwrap();
        let buf = terminal.backend().buffer();
        // the words row is the one containing the start of the second word; just
        // capture every non-empty content row and pick the widest (the words).
        let mut best = String::new();
        for y in 0..buf.area.height {
            let mut row = String::new();
            for x in 0..buf.area.width {
                row.push_str(buf[(x, y)].symbol());
            }
            let trimmed = row.trim_end();
            if trimmed.split_whitespace().count() >= 3 && trimmed.len() > best.len() {
                best = trimmed.to_string();
            }
        }
        best
    }

    #[test]
    fn caret_at_word_end_does_not_shift_line() {
        let cfg = Config {
            mode: Mode::Words,
            words: 10,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        let w0 = app.engine.target_words[0].clone();

        // type all but the last char of the first word
        for c in w0.chars().take(w0.chars().count().saturating_sub(1)) {
            app.on_key(key(KeyCode::Char(c)));
        }
        let before = first_line_of_words(&app);

        // type the final char -> caret now sits at the end-of-word space
        let last = w0.chars().last().unwrap();
        app.on_key(key(KeyCode::Char(last)));
        let after = first_line_of_words(&app);

        // the visible glyph layout of the line is unchanged (no inserted cell)
        assert_eq!(
            before, after,
            "caret reaching the space shifted the line:\nbefore: {before:?}\nafter:  {after:?}"
        );
    }

    #[test]
    fn completing_words_test_shows_results() {
        let cfg = Config {
            mode: Mode::Words,
            words: 3,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        let targets = app.engine.target_words.clone();
        for (i, w) in targets.iter().enumerate() {
            for c in w.chars() {
                app.on_key(key(KeyCode::Char(c)));
            }
            if i + 1 < targets.len() {
                app.on_key(key(KeyCode::Char(' ')));
            }
        }
        assert_eq!(app.screen, Screen::Results);
        assert!(app.result.is_some());

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| crate::ui::render(&app, f)).unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("wpm"), "results should show wpm: {text}");
        assert!(text.contains("acc"));
    }

    #[test]
    fn tiny_terminal_does_not_panic() {
        let app = App::new(Config::default());
        for (w, h) in [(1, 1), (5, 2), (20, 3), (10, 10)] {
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            terminal.draw(|f| crate::ui::render(&app, f)).unwrap();
        }
    }

    #[test]
    fn restart_resets_engine() {
        let mut app = App::new(Config::default());
        app.on_key(key(KeyCode::Char('a')));
        app.restart();
        assert_eq!(app.engine.state(), State::BeforeStart);
        assert_eq!(app.screen, Screen::Test);
    }

    #[test]
    fn esc_opens_command_line_and_renders() {
        let mut app = App::new(Config::default());
        app.on_key(key(KeyCode::Esc));
        assert!(app.command_line.is_some());
        for c in "punc".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| crate::ui::render(&app, f)).unwrap();
        let text = buffer_text(&terminal);
        assert!(
            text.contains("punctuation"),
            "palette should list punctuation: {text}"
        );
    }

    #[test]
    fn command_line_toggles_config_and_stays_open() {
        let mut app = App::new(Config::default());
        assert!(!app.config.punctuation);
        app.on_key(key(KeyCode::Esc)); // open
        for c in "punctuation".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        app.on_key(key(KeyCode::Enter)); // execute toggle
        let command_line = app.command_line.as_ref().expect("config should stay open");
        assert_eq!(command_line.query, "punctuation");
        assert!(command_line
            .commands
            .iter()
            .any(|command| command.label == "punctuation > on (toggle)"));
        assert!(app.config.punctuation);
    }

    #[test]
    fn config_workspace_renders_current_snapshot_and_tabs() {
        let mut app = App::new(Config::default());
        app.on_key(key(KeyCode::Esc));
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| crate::ui::render(&app, f)).unwrap();
        let text = buffer_text(&terminal);
        for needle in [
            "Current",
            "Behavior",
            "Appearance",
            "current configuration",
            "serika_dark",
            "result saving",
        ] {
            assert!(
                text.contains(needle),
                "config workspace missing {needle:?}:\n{text}"
            );
        }
    }

    #[test]
    fn config_workspace_drills_into_a_setting() {
        let mut app = App::new(Config::default());
        app.on_key(key(KeyCode::Esc));
        app.on_key(key(KeyCode::Char('2'))); // Test tab
        app.on_key(key(KeyCode::Enter)); // mode setting
        assert_eq!(
            app.command_line.as_ref().and_then(|line| line.group),
            Some("mode")
        );

        let mut terminal = Terminal::new(TestBackend::new(90, 26)).unwrap();
        terminal.draw(|f| crate::ui::render(&app, f)).unwrap();
        let text = buffer_text(&terminal);
        assert!(
            text.contains("Test / mode"),
            "missing drill-down header:\n{text}"
        );
        assert!(
            text.contains("current: time"),
            "missing active value:\n{text}"
        );
    }

    #[test]
    fn config_workspace_handles_small_terminals() {
        for (width, height) in [(1, 1), (27, 8), (28, 9), (48, 14), (80, 24)] {
            let mut app = App::new(Config::default());
            app.on_key(key(KeyCode::Esc));
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|f| crate::ui::render(&app, f)).unwrap();
        }
    }

    #[test]
    fn stats_screen_renders_sections() {
        let mut app = App::new(Config::default());
        let mut p = crate::persistence::Profile {
            completed: 5,
            started: 7,
            time_typing_sec: 600.0,
            estimated_words: 900,
            highest_wpm: 120.0,
            avg_wpm: 90.0,
            avg_wpm_last10: 95.0,
            highest_acc: 99.0,
            avg_acc: 96.0,
            today: 100,
            current_streak: 3,
            max_streak: 8,
            wpm_history: vec![70.0, 85.0, 90.0, 100.0, 120.0],
            ..Default::default()
        };
        p.activity = vec![crate::persistence::DayActivity { day: 100, count: 4 }];
        app.profile = Some(p);
        app.screen = Screen::Stats;

        let mut terminal = Terminal::new(TestBackend::new(100, 40)).unwrap();
        terminal.draw(|f| crate::ui::render(&app, f)).unwrap();
        let text = buffer_text(&terminal);
        for needle in [
            "your stats",
            "time typing",
            "highest wpm", // left column
            "highest acc", // right column
            "average consistency",
            "activity",
            "back to test",
        ] {
            assert!(
                text.contains(needle),
                "stats screen missing {needle:?}:\n{text}"
            );
        }
    }

    #[test]
    fn results_s_opens_stats_and_back_returns_to_test() {
        let mut app = App::new(Config::default());
        app.screen = Screen::Results;
        app.on_key(key(KeyCode::Char('s')));
        assert_eq!(app.screen, Screen::Stats);
        app.on_key(key(KeyCode::Esc));
        assert_eq!(app.screen, Screen::Test);
    }

    #[test]
    fn stats_screen_handles_small_terminals() {
        let mut app = App::new(Config::default());
        app.profile = Some(crate::persistence::Profile {
            completed: 2,
            started: 2,
            wpm_history: vec![70.0, 80.0],
            ..Default::default()
        });
        app.screen = Screen::Stats;

        for (width, height) in [(1, 1), (29, 9), (30, 10), (40, 20), (80, 24)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| crate::ui::render(&app, frame))
                .unwrap();
        }
    }

    #[test]
    fn esc_in_palette_closes_without_change() {
        let mut app = App::new(Config::default());
        app.on_key(key(KeyCode::Esc)); // open
        app.on_key(key(KeyCode::Esc)); // close
        assert!(app.command_line.is_none());
        assert!(!app.config.punctuation);
    }

    /// CLI flags are session-only: an unrelated palette action must not
    /// persist them, but an explicit palette change to the field must.
    #[test]
    fn cli_overrides_stay_session_only_until_changed_in_palette() {
        let disk = Config::default(); // mode=time, time=30
        let mut effective = disk.clone();
        effective.time = 15; // `mtype --time 15`
        let overrides = SessionOverrides {
            mode: true,
            time: true,
            ..SessionOverrides::default()
        };
        let mut app = App::new_session(effective, disk, overrides);

        // unrelated palette change: theme
        app.execute(crate::commandline::Action::SetTheme("dracula".to_string()));
        let saved = app.persistable_config();
        assert_eq!(saved.time, 30, "--time must not leak into config.toml");
        assert_eq!(saved.theme, "dracula", "theme change must persist");

        // explicit palette change: "time > 60" persists again
        app.execute(crate::commandline::Action::SetTime(60));
        let saved = app.persistable_config();
        assert_eq!(saved.time, 60);
        assert_eq!(saved.mode, Mode::Time);
    }

    /// The results-screen 'm'/'l' shortcuts switch to practice for this
    /// session only; a later palette save keeps the configured mode.
    #[test]
    fn practice_shortcut_does_not_persist_practice_mode() {
        let mut app = App::new(Config::default()); // mode=time on disk
        app.start_practice(crate::config::PracticeMode::Missed);
        assert_eq!(app.config.mode, Mode::Practice);
        assert_eq!(app.persistable_config().mode, Mode::Time);

        // Changing practice options alone still does not persist the shortcut's
        // session-only mode switch.
        app.execute(crate::commandline::Action::SetPractice(
            crate::config::PracticeMode::Missed,
            25,
        ));
        assert_eq!(app.persistable_config().mode, Mode::Time);

        // Only explicitly selecting the mode persists it.
        app.execute(crate::commandline::Action::SetMode(Mode::Practice));
        assert_eq!(app.persistable_config().mode, Mode::Practice);
    }

    #[test]
    fn editing_custom_text_does_not_switch_modes() {
        let mut app = App::new(Config::default());
        assert_eq!(app.config.mode, Mode::Time);
        app.screen = Screen::Editor;
        app.editor_text = "alpha beta".to_string();
        app.on_key_editor(key(KeyCode::Char('s')), true);
        assert_eq!(app.config.custom_text, "alpha beta");
        assert_eq!(app.config.mode, Mode::Time);
    }

    /// Non-restarting palette toggles must reach the running engine
    /// immediately (upstream applies settings to the test in progress).
    #[test]
    fn palette_toggle_syncs_running_engine_config() {
        let mut app = App::new(Config::default());
        app.on_key(key(KeyCode::Char('a')));
        assert_eq!(app.engine.state(), State::Running);
        assert!(!app.engine.config.freedom_mode);
        app.execute(crate::commandline::Action::ToggleField(
            crate::commandline::BoolField::FreedomMode,
        ));
        assert!(app.config.freedom_mode);
        assert!(
            app.engine.config.freedom_mode,
            "running engine must see the toggle without a restart"
        );
    }

    /// Time spent in the command palette is not charged to the test clock.
    #[test]
    fn palette_pause_is_not_charged_to_the_test_clock() {
        let mut app = App::new(Config::default());
        app.on_key(key(KeyCode::Char('a'))); // start the clock
        assert_eq!(app.engine.state(), State::Running);
        app.on_key(key(KeyCode::Esc)); // open the palette mid-test
        std::thread::sleep(Duration::from_millis(50));
        app.on_key(key(KeyCode::Esc)); // close it again
        let raw = app.epoch.elapsed().as_millis();
        let adjusted = app.now_ms();
        assert!(
            raw.saturating_sub(adjusted) >= 40,
            "palette time must be excluded: raw {raw}ms vs adjusted {adjusted}ms"
        );
    }

    /// time 0 is an infinite test: it never clock-finishes, and enter bails
    /// it out with a normal result.
    #[test]
    fn time_zero_is_endless_and_enter_bails() {
        let cfg = Config {
            mode: Mode::Time,
            time: 0,
            result_saving: false,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        app.on_key(key(KeyCode::Char('a')));
        app.engine.tick(600_000); // ten minutes later: still running
        assert_eq!(app.engine.state(), State::Running);
        app.on_key(key(KeyCode::Enter)); // bail out
        assert_eq!(app.screen, Screen::Results);
        assert!(app.result.as_ref().is_some_and(|r| !r.failed));
    }

    /// words 0 is an endless test that can be finished via the bail key.
    #[test]
    fn words_zero_endless_test_can_bail_to_results() {
        let cfg = Config {
            mode: Mode::Words,
            words: 0,
            result_saving: false,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        let first = app.engine.target_words[0].clone();
        for c in first.chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        app.on_key(key(KeyCode::Char(' ')));
        assert_eq!(app.screen, Screen::Test, "endless test must keep running");
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.screen, Screen::Results);
    }

    #[test]
    fn quick_restart_enter_restarts_the_test() {
        let cfg = Config {
            quick_restart: QuickRestart::Enter,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        app.on_key(key(KeyCode::Char('a')));
        assert_eq!(app.engine.state(), State::Running);
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.engine.state(), State::BeforeStart);
        assert_eq!(app.screen, Screen::Test);
    }

    /// With quick restart on esc the roles swap (as upstream): esc restarts
    /// and tab opens the command line, so settings stay reachable.
    #[test]
    fn quick_restart_esc_swaps_esc_and_tab() {
        let cfg = Config {
            quick_restart: QuickRestart::Esc,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        app.on_key(key(KeyCode::Char('a')));
        app.on_key(key(KeyCode::Esc)); // restart, not palette
        assert!(app.command_line.is_none());
        assert_eq!(app.engine.state(), State::BeforeStart);
        app.on_key(key(KeyCode::Tab)); // tab now opens the palette
        assert!(app.command_line.is_some());
    }

    /// A quote guaranteed to be outside the given length bands, so a random
    /// pick from the band pool can never coincide with it: equality after a
    /// restart proves a repeat, inequality proves a fresh draw.
    fn quote_outside_bands(bands: &[crate::config::QuoteLengthBand]) -> crate::content::Quote {
        let collection = crate::content::quotes("english");
        let in_band: std::collections::HashSet<u32> = bands
            .iter()
            .flat_map(|band| collection.in_band(band.index()))
            .map(|quote| quote.id)
            .collect();
        collection
            .quotes
            .iter()
            .find(|quote| !in_band.contains(&quote.id))
            .cloned()
            .expect("the english collection spans several length bands")
    }

    fn quote_config() -> Config {
        Config {
            mode: Mode::Quote,
            quote_length: vec![crate::config::QuoteLengthBand::Short],
            result_saving: false,
            ..Config::default()
        }
    }

    /// Upstream `repeatQuotes: "typing"`: a restart that interrupts a quote
    /// test mid-typing serves the same quote again.
    #[test]
    fn repeat_quotes_repeats_on_mid_typing_restart() {
        let cfg = Config {
            repeat_quotes: true,
            ..quote_config()
        };
        let mut app = App::new(cfg);
        let first = app.engine.target_words[0].clone();
        app.on_key(key(KeyCode::Char(first.chars().next().unwrap())));
        assert_eq!(app.engine.state(), State::Running);

        let planted = quote_outside_bands(&app.config.quote_length);
        app.engine.quote = Some(planted.clone());
        app.on_key(key(KeyCode::Tab));
        assert_eq!(
            app.engine.quote.as_ref().map(|quote| quote.id),
            Some(planted.id),
            "a mid-typing restart must repeat the quote"
        );
        // the repeat pin is per-rebuild, not a lasting quote_id
        assert_eq!(app.config.quote_id, None);
    }

    /// Upstream only repeats while typing: after a completed test (and for an
    /// unstarted one) the next quote is drawn fresh even with the toggle on.
    #[test]
    fn repeat_quotes_draws_fresh_after_finishing() {
        let cfg = Config {
            repeat_quotes: true,
            ..quote_config()
        };
        let mut app = App::new(cfg);
        let targets = app.engine.target_words.clone();
        for (i, word) in targets.iter().enumerate() {
            for c in word.chars() {
                app.on_key(key(KeyCode::Char(c)));
            }
            if i + 1 < targets.len() {
                app.on_key(key(KeyCode::Char(' ')));
            }
        }
        assert_eq!(app.screen, Screen::Results);

        let planted = quote_outside_bands(&app.config.quote_length);
        app.engine.quote = Some(planted.clone());
        app.on_key(key(KeyCode::Tab)); // next test from the results screen
        assert_eq!(app.screen, Screen::Test);
        assert_ne!(
            app.engine.quote.as_ref().map(|quote| quote.id),
            Some(planted.id),
            "a finished quote must not repeat"
        );
        assert_eq!(app.config.quote_id, None);
    }

    /// Changing the quote length band in the palette must take effect even
    /// with repeat quotes on and a test underway (upstream always draws a
    /// fresh quote after a config change).
    #[test]
    fn quote_length_change_takes_effect_with_repeat_quotes_on() {
        let cfg = Config {
            repeat_quotes: true,
            quote_length: vec![crate::config::QuoteLengthBand::Thicc],
            ..quote_config()
        };
        let mut app = App::new(cfg);
        let first = app.engine.target_words[0].clone();
        app.on_key(key(KeyCode::Char(first.chars().next().unwrap())));
        assert_eq!(app.engine.state(), State::Running);

        app.execute(crate::commandline::Action::SetQuoteLength(
            crate::config::QuoteLengthBand::Short,
        ));
        let collection = crate::content::quotes("english");
        let picked = app.engine.quote.as_ref().expect("quote mode").id;
        assert!(
            collection
                .in_band(crate::config::QuoteLengthBand::Short.index())
                .iter()
                .any(|quote| quote.id == picked),
            "the new band must be honored, got quote {picked}"
        );
    }

    /// `mtype --quote-id N` keeps serving quote N across plain restarts (that
    /// is the quote the user asked to type) until a palette change to the
    /// quote selection releases the pin.
    #[test]
    fn explicit_quote_id_survives_restart() {
        let mut cfg = quote_config();
        let target = quote_outside_bands(&cfg.quote_length);
        cfg.quote_id = Some(target.id);
        let mut app = App::new(cfg);
        assert_eq!(
            app.engine.quote.as_ref().map(|quote| quote.id),
            Some(target.id)
        );

        // fumbled start, then tab: the requested quote must come back
        let first = app.engine.target_words[0].clone();
        app.on_key(key(KeyCode::Char(first.chars().next().unwrap())));
        app.on_key(key(KeyCode::Tab));
        assert_eq!(
            app.engine.quote.as_ref().map(|quote| quote.id),
            Some(target.id),
            "an explicit --quote-id must survive a restart"
        );

        // an unstarted tab keeps it too
        app.on_key(key(KeyCode::Tab));
        assert_eq!(
            app.engine.quote.as_ref().map(|quote| quote.id),
            Some(target.id)
        );
        assert_eq!(app.config.quote_id, Some(target.id));

        // choosing a quote length in the palette releases the pin
        app.execute(crate::commandline::Action::SetQuoteLength(
            crate::config::QuoteLengthBand::Short,
        ));
        assert_eq!(app.config.quote_id, None);
        assert_ne!(
            app.engine.quote.as_ref().map(|quote| quote.id),
            Some(target.id)
        );
    }

    /// Deterministic words on screen, so warning-overlay assertions can never
    /// collide with randomly drawn words.
    fn fixed_text_config() -> Config {
        Config {
            mode: Mode::Custom,
            custom_text: "alpha beta gamma".to_string(),
            result_saving: false,
            ..Config::default()
        }
    }

    fn caps_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new_with_kind_and_state(
            code,
            KeyModifiers::NONE,
            KeyEventKind::Press,
            KeyEventState::CAPS_LOCK,
        ))
    }

    fn rendered(app: &App) -> String {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| crate::ui::render(app, f)).unwrap();
        buffer_text(&terminal)
    }

    /// FocusLost/FocusGained (delivered once tui::init enables focus-change
    /// reporting) drive the out-of-focus warning overlay.
    #[test]
    fn focus_loss_shows_warning_and_focus_gain_clears_it() {
        let mut app = App::new(fixed_text_config()); // warning on by default
        app.on_event(Event::FocusLost);
        assert!(!app.focused);
        assert!(rendered(&app).contains("click to focus"));

        app.on_event(Event::FocusGained);
        assert!(app.focused);
        assert!(!rendered(&app).contains("click to focus"));
    }

    #[test]
    fn focus_warning_respects_disabled_setting() {
        let cfg = Config {
            show_out_of_focus_warning: false,
            ..fixed_text_config()
        };
        let mut app = App::new(cfg);
        app.on_event(Event::FocusLost);
        assert!(!app.focused);
        assert!(!rendered(&app).contains("click to focus"));
    }

    /// A kitty-protocol CAPS_LOCK state bit shows the warning and the
    /// un-shifted key is translated to the letter caps lock produces.
    #[test]
    fn caps_lock_state_shows_warning_and_uppercases_letters() {
        let mut app = App::new(fixed_text_config()); // caps_lock_warning on
        app.on_event(caps_event(KeyCode::Char('a')));
        assert!(app.caps_lock);
        assert_eq!(app.engine.typed[0], "A", "caps lock must type uppercase");
        assert!(rendered(&app).contains("caps lock"));

        // shifted alternate under caps lock toggles back down
        app.on_event(caps_event(KeyCode::Char('B')));
        assert_eq!(app.engine.typed[0], "Ab");

        // a later event without the bit clears the warning
        app.on_event(Event::Key(key(KeyCode::Char('c'))));
        assert!(!app.caps_lock);
        assert!(!rendered(&app).contains("caps lock"));
    }

    #[test]
    fn caps_lock_warning_respects_disabled_setting() {
        let cfg = Config {
            caps_lock_warning: false,
            ..fixed_text_config()
        };
        let mut app = App::new(cfg);
        app.on_event(caps_event(KeyCode::Char('a')));
        assert!(app.caps_lock);
        assert!(!rendered(&app).contains("caps lock"));
    }

    /// The CapsLock key event itself must not drive the flag: crossterm tags
    /// it with a CAPS_LOCK state bit even when the press turns the lock off.
    #[test]
    fn caps_lock_key_event_does_not_set_flag() {
        let mut app = App::new(fixed_text_config());
        app.on_event(caps_event(KeyCode::CapsLock));
        assert!(!app.caps_lock);
        assert_eq!(app.engine.state(), State::BeforeStart);
    }

    /// Kitty-capable terminals could deliver non-press kinds; only Press
    /// events may reach key handling, so typing behavior never changes.
    #[test]
    fn non_press_key_events_are_ignored() {
        let mut app = App::new(fixed_text_config());
        app.on_event(Event::Key(KeyEvent::new_with_kind_and_state(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
            KeyEventState::NONE,
        )));
        assert_eq!(app.engine.state(), State::BeforeStart);
        assert_eq!(app.engine.typed[0], "");
    }
}
