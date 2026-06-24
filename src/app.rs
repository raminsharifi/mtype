//! Top-level application state and the main event loop. Owns config + theme,
//! the current `Engine`, and routes keyboard input to it.

use crate::commandline::{CommandLine, Outcome};
use crate::config::{Config, Mode, QuickRestart};
use crate::engine::{Engine, State, TestResult};
use crate::theme::Theme;
use crate::tui::Tui;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Test,
    Results,
}

pub struct App {
    pub config: Config,
    pub theme: Theme,
    pub screen: Screen,
    pub engine: Engine,
    pub result: Option<TestResult>,
    pub pb_info: crate::persistence::PbInfo,
    pub command_line: Option<CommandLine>,
    pub epoch: Instant,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: Config) -> App {
        let theme = Theme::by_name(&config.theme);
        let engine = Engine::new(config.clone(), StdRng::from_entropy());
        App {
            config,
            theme,
            screen: Screen::Test,
            engine,
            result: None,
            pb_info: crate::persistence::PbInfo::default(),
            command_line: None,
            epoch: Instant::now(),
            should_quit: false,
        }
    }

    pub fn now_ms(&self) -> u128 {
        self.epoch.elapsed().as_millis()
    }

    /// Start a fresh test with the current config.
    pub fn restart(&mut self) {
        self.engine = Engine::new(self.config.clone(), StdRng::from_entropy());
        self.epoch = Instant::now();
        self.result = None;
        self.screen = Screen::Test;
    }

    pub fn run(&mut self, terminal: &mut Tui) -> Result<()> {
        let tick = Duration::from_millis(16);
        while !self.should_quit {
            terminal.draw(|frame| crate::ui::render(self, frame))?;

            if event::poll(tick)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.on_key(key);
                    }
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
            KeyCode::Backspace => cl.pop_char(),
            KeyCode::Enter => {
                if let Some(action) = cl.current_action() {
                    self.command_line = None;
                    self.execute(action);
                }
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
        self.sync_finish();
    }

    fn on_key_results(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab | KeyCode::Enter => self.restart(),
            KeyCode::Esc => self.open_command_line(),
            KeyCode::Char('q') => self.should_quit = true,
            _ => {}
        }
    }
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
        let mut cfg = Config::default();
        cfg.mode = Mode::Words;
        cfg.words = 10;
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
        let mut cfg = Config::default();
        cfg.mode = Mode::Words;
        cfg.words = 10;
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
        let mut cfg = Config::default();
        cfg.mode = Mode::Words;
        cfg.words = 3;
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
    fn command_line_toggles_config_and_closes() {
        let mut app = App::new(Config::default());
        assert!(!app.config.punctuation);
        app.on_key(key(KeyCode::Esc)); // open
        for c in "punctuation".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        app.on_key(key(KeyCode::Enter)); // execute toggle
        assert!(app.command_line.is_none());
        assert!(app.config.punctuation);
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
