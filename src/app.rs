//! Top-level application state and the main event loop. Owns config + theme,
//! the current `Engine`, and routes keyboard input to it.

use crate::commandline::{CommandLine, Outcome};
use crate::config::{Config, Mode, QuickRestart};
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
    pub should_quit: bool,
    pub pace_wpm: Option<f64>,
    pub focused: bool,
    pub caps_lock: bool,
    pub results_view: ResultsView,
    pub replay_epoch: Option<Instant>,
    pub editor_text: String,
}

impl App {
    pub fn new(mut config: Config) -> App {
        refresh_practice_text(&mut config);
        let theme = Theme::by_name(&config.theme);
        let engine = Engine::new(config.clone(), StdRng::from_entropy());
        let pace_wpm = crate::persistence::pace_wpm(&config);
        App {
            config,
            theme,
            screen: Screen::Test,
            engine,
            result: None,
            pb_info: crate::persistence::PbInfo::default(),
            command_line: None,
            profile: None,
            started_counted: false,
            epoch: Instant::now(),
            should_quit: false,
            pace_wpm,
            focused: true,
            caps_lock: false,
            results_view: ResultsView::Summary,
            replay_epoch: None,
            editor_text: String::new(),
        }
    }

    pub fn now_ms(&self) -> u128 {
        self.epoch.elapsed().as_millis()
    }

    /// Start a fresh test with the current config.
    pub fn restart(&mut self) {
        self.config.quote_id = if self.config.mode == Mode::Quote && self.config.repeat_quotes {
            self.engine.quote.as_ref().map(|quote| quote.id)
        } else {
            None
        };
        refresh_practice_text(&mut self.config);
        self.pace_wpm = crate::persistence::pace_wpm(&self.config);
        self.engine = Engine::new(self.config.clone(), StdRng::from_entropy());
        self.epoch = Instant::now();
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
                match event::read()? {
                    Event::Key(key) => {
                        self.caps_lock = key.state.contains(KeyEventState::CAPS_LOCK);
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

            // drive timed tests / fail conditions (paused while the palette is open)
            if self.screen == Screen::Test && self.command_line.is_none() {
                let now = self.now_ms();
                self.engine.tick(now);
                self.sync_finish();
            }
        }
        Ok(())
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
        self.command_line = Some(CommandLine::new(&self.config));
    }

    fn on_key_commandline(&mut self, key: KeyEvent, ctrl: bool) {
        let Some(cl) = self.command_line.as_mut() else {
            return;
        };
        match key.code {
            KeyCode::Esc => self.command_line = None,
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
        match action.apply(&mut self.config) {
            Outcome::Restart => {
                let _ = self.config.save();
                self.theme = Theme::by_name(&self.config.theme);
                self.restart();
            }
            Outcome::StayAndRedraw => {
                let _ = self.config.save();
                self.theme = Theme::by_name(&self.config.theme);
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

    fn on_key_test(&mut self, key: KeyEvent, ctrl: bool, alt: bool) {
        let now = self.now_ms();
        match key.code {
            KeyCode::Tab => {
                if !self.restart_blocked() {
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
                if self.config.mode == Mode::Zen {
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
                self.config.mode = Mode::Custom;
                let _ = self.config.save();
                self.restart();
            }
            KeyCode::Esc => {
                self.editor_text.clear();
                self.screen = Screen::Test;
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
}
