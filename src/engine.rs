//! The typing-test engine: a faithful port of Monkeytype's scoring, ported from
//! the event-log model in `frontend/src/ts/test/events/stats.ts` and the input
//! semantics in `frontend/src/ts/input/handlers/*`.
//!
//! All input methods take an explicit `now_ms` timestamp so the engine is pure
//! and deterministic for unit tests; the TUI passes real elapsed milliseconds.
//!
//! Key fidelity points:
//! - char classification == `utils/strings.ts::countChars`
//! - every non-last word's target gets a trailing space (`getTargetWord`), so
//!   spaces count toward WPM
//! - `wpm = calculateWpm(correctWord, dur)`, `raw = calculateWpm(allCorrect +
//!   incorrect + extra, dur)`, accuracy is keystroke-level, consistency is
//!   `kogasa(stdDev/mean)` over per-word burst speeds.

use crate::config::{ConfidenceMode, Config, Difficulty, Mode, StopOnError};
use crate::content::Quote;
use crate::numbers::{calculate_wpm, consistency, round_to_2};
use crate::wordgen::WordGenerator;
use rand::rngs::StdRng;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CharCounts {
    pub all_correct: usize,
    pub correct_word: usize,
    pub incorrect: usize,
    pub extra: usize,
    pub missed: usize,
}

impl std::ops::AddAssign for CharCounts {
    fn add_assign(&mut self, o: CharCounts) {
        self.all_correct += o.all_correct;
        self.correct_word += o.correct_word;
        self.incorrect += o.incorrect;
        self.extra += o.extra;
        self.missed += o.missed;
    }
}

/// Direct port of `utils/strings.ts::countChars`.
pub fn count_chars(input: &str, target: &str, credit_partial: bool) -> CharCounts {
    let inp: Vec<char> = input.chars().collect();
    let tgt: Vec<char> = target.chars().collect();
    let word_correct = input == target;
    let word_partial = target.starts_with(input);
    let input_has_space = input.contains(' ');
    let mut c = CharCounts::default();

    for i in 0..inp.len().max(tgt.len()) {
        let ic = inp.get(i).copied();
        let tc = tgt.get(i).copied();
        match (ic, tc) {
            (Some(a), Some(b)) if a == b => {
                if b == ' ' && !word_correct {
                    c.extra += 1;
                } else {
                    c.all_correct += 1;
                }
                if word_correct || (credit_partial && word_partial) {
                    c.correct_word += 1;
                }
            }
            (None, _) => {
                if !credit_partial {
                    c.missed += 1;
                }
            }
            (Some(a), tcopt) => {
                let tc_undef = tcopt.is_none();
                let space_case = tcopt == Some(' ') && a != ' ' && !input_has_space;
                if tc_undef || space_case {
                    c.extra += 1;
                } else {
                    c.incorrect += 1;
                }
            }
        }
    }
    c
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    BeforeStart,
    Running,
    Finished,
    Failed,
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub wpm: f64,
    pub raw_wpm: f64,
    pub acc: f64,
    pub consistency: f64,
    pub char_correct: usize, // correctWord
    pub char_incorrect: usize,
    pub char_extra: usize,
    pub char_missed: usize,
    pub char_total: usize,
    pub duration_sec: f64,
    pub mode: Mode,
    pub mode2: String, // "30" for time, "50" for words, quote source, etc.
    pub punctuation: bool,
    pub numbers: bool,
    pub language: String,
    pub wpm_history: Vec<f64>,
    pub raw_history: Vec<f64>,
    pub failed: bool,
    pub fail_reason: Option<String>,
    pub quote_source: Option<String>,
}

struct Keystroke {
    ms: u128,
    correct: bool,
    /// whether this keystroke was a letter (counts toward raw chars) vs a
    /// committing space.
    letter: bool,
}

struct CommittedWord {
    correct_word_chars: usize, // contribution to net WPM at commit time
    commit_ms: u128,
}

pub struct Engine {
    pub config: Config,
    generator: WordGenerator,
    rng: StdRng,

    pub target_words: Vec<String>,
    pub typed: Vec<String>, // parallel to target_words; typed[active] is in progress
    pub active: usize,

    state: State,
    fail_reason: Option<String>,
    start_ms: Option<u128>,
    finish_ms: Option<u128>,
    word_start_ms: u128,

    keystrokes: Vec<Keystroke>,
    word_bursts: Vec<f64>,
    committed: Vec<CommittedWord>,

    pub quote: Option<Quote>,
}

impl Engine {
    pub fn new(config: Config, mut rng: StdRng) -> Engine {
        let (target_words, generator) = crate::wordgen::generate_test_words(&config, &mut rng);
        let quote = generator.quote.clone();
        let mut typed = vec![String::new(); target_words.len()];
        if typed.is_empty() {
            typed.push(String::new()); // zen / empty: always have an active slot
        }
        Engine {
            config,
            generator,
            rng,
            target_words,
            typed,
            active: 0,
            state: State::BeforeStart,
            fail_reason: None,
            start_ms: None,
            finish_ms: None,
            word_start_ms: 0,
            keystrokes: Vec::new(),
            word_bursts: Vec::new(),
            committed: Vec::new(),
            quote,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn is_zen(&self) -> bool {
        self.config.mode == Mode::Zen
    }

    fn is_timed(&self) -> bool {
        self.config.mode == Mode::Time || self.config.mode == Mode::Zen
    }

    fn target(&self, i: usize) -> &str {
        self.target_words.get(i).map(|s| s.as_str()).unwrap_or("")
    }

    /// The target word for the active position; in zen the target is whatever
    /// was typed (always correct).
    fn active_target(&self) -> String {
        if self.is_zen() {
            self.typed.get(self.active).cloned().unwrap_or_default()
        } else {
            self.target(self.active).to_string()
        }
    }

    fn ensure_slot(&mut self, i: usize) {
        while self.typed.len() <= i {
            self.typed.push(String::new());
        }
    }

    /// Top up the streaming word pool (time / zen / infinite words) so there are
    /// always a few words ahead of the cursor.
    fn top_up(&mut self) {
        if self.generator.is_finite() && !self.is_zen() {
            return;
        }
        if self.is_zen() {
            return; // zen has no pre-generated targets
        }
        while self.target_words.len() < self.active + 3 {
            match self.generator.next(&mut self.rng) {
                Some(w) => {
                    self.target_words.push(w);
                    self.typed.push(String::new());
                }
                None => break,
            }
        }
    }

    pub fn type_char(&mut self, c: char, now_ms: u128) {
        if matches!(self.state, State::Finished | State::Failed) {
            return;
        }
        if c == ' ' {
            self.commit_word(now_ms);
            return;
        }
        // ignore other control chars
        if c.is_control() {
            return;
        }

        if self.state == State::BeforeStart {
            self.state = State::Running;
            self.start_ms = Some(now_ms);
            self.word_start_ms = now_ms;
        }
        self.ensure_slot(self.active);

        let target = self.active_target();
        let pos = self.typed[self.active].chars().count();
        let target_char = target.chars().nth(pos);
        let correct = if self.is_zen() {
            true
        } else {
            target_char == Some(c)
        };

        // hide extra letters: block typing past the word
        if self.config.hide_extra_letters && target_char.is_none() && !self.is_zen() {
            return;
        }
        // cap runaway extra letters
        if pos > target.chars().count() + 20 {
            return;
        }
        // stop on error (letter): block an incorrect char from being inserted,
        // but still record it as an incorrect keystroke.
        if self.config.stop_on_error == StopOnError::Letter && !correct {
            self.keystrokes.push(Keystroke {
                ms: now_ms,
                correct: false,
                letter: true,
            });
            self.maybe_master_fail();
            return;
        }

        self.typed[self.active].push(c);
        self.keystrokes.push(Keystroke {
            ms: now_ms,
            correct,
            letter: true,
        });

        if !correct {
            self.maybe_master_fail();
            if self.state == State::Failed {
                return;
            }
        }

        // finite finish: completing the last word exactly ends the test
        self.maybe_finish_on_completion(now_ms);
    }

    fn maybe_master_fail(&mut self) {
        if self.config.difficulty == Difficulty::Master {
            self.state = State::Failed;
            self.fail_reason = Some("master: incorrect key".to_string());
        }
    }

    fn maybe_finish_on_completion(&mut self, now_ms: u128) {
        if self.is_timed() {
            return;
        }
        let last = self.target_words.len().saturating_sub(1);
        let is_last_word = self.active == last && !self.target_words.is_empty();
        if matches!(self.config.mode, Mode::Words | Mode::Quote | Mode::Custom)
            && is_last_word
            && self.typed[self.active] == self.target(last)
        {
            self.finish(now_ms);
        }
    }

    fn commit_word(&mut self, now_ms: u128) {
        // a leading space before any typing does nothing
        if self.state == State::BeforeStart {
            return;
        }
        let current = self.typed.get(self.active).cloned().unwrap_or_default();
        if current.is_empty() && !self.config.strict_space {
            // committing an empty word does nothing (no double-space skip)
            return;
        }

        let target = self.active_target();
        let word_correct = current == target;

        // strict space / stop-on-error(word): if the word is wrong, a space does
        // not commit - it is rejected (and counts as an incorrect keystroke).
        if self.config.stop_on_error == StopOnError::Word && !word_correct {
            self.keystrokes.push(Keystroke {
                ms: now_ms,
                correct: false,
                letter: false,
            });
            return;
        }

        // expert: submitting an imperfect word fails the test
        if self.config.difficulty == Difficulty::Expert && !word_correct {
            self.keystrokes.push(Keystroke {
                ms: now_ms,
                correct: false,
                letter: false,
            });
            self.state = State::Failed;
            self.fail_reason = Some("expert: imperfect word submitted".to_string());
            return;
        }

        // record the committing space as a keystroke
        self.keystrokes.push(Keystroke {
            ms: now_ms,
            correct: word_correct,
            letter: false,
        });

        // per-word stats (committed words always carry a trailing space target)
        let input_ws = format!("{current} ");
        let target_ws = if self.is_zen() {
            input_ws.clone()
        } else {
            format!("{target} ")
        };
        let cc = count_chars(&input_ws, &target_ws, false);
        self.committed.push(CommittedWord {
            correct_word_chars: cc.correct_word,
            commit_ms: now_ms,
        });

        // burst for this word: (chars+trigger)/5 / minutes
        let raw_len = current.chars().count() + 1; // +1 for the committing space
        let secs = (now_ms.saturating_sub(self.word_start_ms)) as f64 / 1000.0;
        if secs > 0.0 {
            self.word_bursts.push(calculate_wpm(raw_len as f64, secs));
        }

        // advance
        if self.is_zen() {
            // freeze this word's target as what was typed
            self.target_words.push(current);
        }
        self.active += 1;
        self.ensure_slot(self.active);
        self.word_start_ms = now_ms;
        self.top_up();

        // finite finish: committed the last word
        let finite_done = matches!(self.config.mode, Mode::Words | Mode::Quote | Mode::Custom)
            && self.active >= self.target_words.len();
        if finite_done {
            self.finish(now_ms);
        }
    }

    pub fn backspace(&mut self, ctrl: bool, _now_ms: u128) {
        if matches!(self.state, State::Finished | State::Failed) {
            return;
        }
        if self.config.confidence_mode == ConfidenceMode::Max {
            return; // no backspacing at all
        }
        self.ensure_slot(self.active);
        if !self.typed[self.active].is_empty() {
            if ctrl {
                self.typed[self.active].clear();
            } else {
                self.typed[self.active].pop();
            }
            return;
        }
        // at the start of a word: maybe step back to the previous word
        if self.active == 0 {
            return;
        }
        if self.config.confidence_mode == ConfidenceMode::On {
            return; // can't go back to previous words
        }
        let prev = self.active - 1;
        let prev_correct =
            self.typed.get(prev).map(|s| s.as_str()).unwrap_or("") == self.target(prev);
        if self.config.freedom_mode || !prev_correct {
            // pop the committed-word record for the word we're re-entering
            self.committed.pop();
            self.word_bursts.pop();
            self.active = prev;
        }
    }

    /// Periodic tick: ends a timed test and applies live fail conditions.
    pub fn tick(&mut self, now_ms: u128) {
        if self.state != State::Running {
            return;
        }
        // time mode: end on the clock
        if self.config.mode == Mode::Time {
            if let Some(start) = self.start_ms {
                if now_ms.saturating_sub(start) >= (self.config.time as u128) * 1000 {
                    self.finish(now_ms);
                    return;
                }
            }
        }
        self.check_fail_conditions(now_ms);
    }

    fn check_fail_conditions(&mut self, now_ms: u128) {
        let elapsed = self.elapsed_secs(now_ms);
        if elapsed < 1.0 {
            return;
        }
        let cc = self.char_counts(true);
        if let Some(min) = self.config.min_wpm {
            let wpm = calculate_wpm(cc.correct_word as f64, elapsed);
            if wpm < min as f64 {
                self.fail("min wpm not met");
                return;
            }
        }
        if let Some(min) = self.config.min_acc {
            let acc = self.accuracy_pct();
            if acc < min as f64 {
                self.fail("min accuracy not met");
                return;
            }
        }
        if let Some(min) = self.config.min_burst {
            if let Some(&last) = self.word_bursts.last() {
                if last < min as f64 {
                    self.fail("min burst not met");
                }
            }
        }
    }

    fn fail(&mut self, reason: &str) {
        self.state = State::Failed;
        self.fail_reason = Some(reason.to_string());
    }

    /// Bail out / finish a zen test (shift+enter).
    pub fn bail(&mut self, now_ms: u128) {
        if self.state == State::Running {
            self.finish(now_ms);
        }
    }

    fn finish(&mut self, now_ms: u128) {
        if self.state != State::Running {
            // allow finishing from BeforeStart only if something happened
            if self.start_ms.is_none() {
                return;
            }
        }
        self.finish_ms = Some(now_ms);
        self.state = State::Finished;
    }

    fn elapsed_secs(&self, now_ms: u128) -> f64 {
        match self.start_ms {
            Some(start) => (now_ms.saturating_sub(start)) as f64 / 1000.0,
            None => 0.0,
        }
    }

    /// Index of the word counted as "last" (no trailing space, gets partial
    /// credit in timed tests).
    fn last_word_index(&self) -> usize {
        if self.target_words.is_empty() {
            return 0;
        }
        if self.active < self.target_words.len() {
            self.active
        } else {
            self.target_words.len() - 1
        }
    }

    /// Aggregate CharCounts across all reached words (port of `getChars`).
    pub fn char_counts(&self, credit_partial_override: bool) -> CharCounts {
        let mut acc = CharCounts::default();
        if self.target_words.is_empty() && !self.is_zen() {
            return acc;
        }
        let credit_partial = self.is_timed() || credit_partial_override;
        let last = self.last_word_index();

        let upper = if self.is_zen() {
            self.typed.len().min(self.active + 1)
        } else {
            (last + 1).min(self.target_words.len())
        };

        for i in 0..upper {
            let input = self.typed.get(i).cloned().unwrap_or_default();
            let is_last = i == last;
            let (input, target) = if is_last {
                let t = if self.is_zen() {
                    input.clone()
                } else {
                    self.target(i).to_string()
                };
                (input, t)
            } else {
                let t = if self.is_zen() {
                    format!("{input} ")
                } else {
                    format!("{} ", self.target(i))
                };
                (format!("{input} "), t)
            };
            acc += count_chars(&input, &target, is_last && credit_partial);
            if is_last {
                break;
            }
        }
        acc
    }

    fn accuracy_pct(&self) -> f64 {
        let mut correct = 0usize;
        let mut total = 0usize;
        for k in &self.keystrokes {
            total += 1;
            if k.correct {
                correct += 1;
            }
        }
        if total == 0 {
            0.0
        } else {
            (correct as f64 / total as f64) * 100.0
        }
    }

    // ---- live readouts for the UI ----

    pub fn live_wpm(&self, now_ms: u128) -> f64 {
        let elapsed = self.elapsed_secs(now_ms);
        if elapsed <= 0.0 {
            return 0.0;
        }
        let cc = self.char_counts(true);
        calculate_wpm(cc.correct_word as f64, elapsed)
    }

    pub fn live_acc(&self) -> f64 {
        self.accuracy_pct()
    }

    pub fn live_burst(&self) -> f64 {
        self.word_bursts.last().copied().unwrap_or(0.0)
    }

    /// Remaining seconds (time mode) for the timer display.
    pub fn time_left(&self, now_ms: u128) -> Option<u32> {
        if self.config.mode != Mode::Time {
            return None;
        }
        let elapsed = match self.start_ms {
            Some(s) => now_ms.saturating_sub(s),
            None => 0,
        };
        let total = (self.config.time as u128) * 1000;
        Some(((total.saturating_sub(elapsed)) as f64 / 1000.0).ceil() as u32)
    }

    /// Words typed so far (for the words-mode counter).
    pub fn words_progress(&self) -> (usize, usize) {
        (
            self.active.min(self.target_words.len()),
            self.target_words.len(),
        )
    }

    fn mode2(&self) -> String {
        match self.config.mode {
            Mode::Time => self.config.time.to_string(),
            Mode::Words => self.config.words.to_string(),
            Mode::Quote => self
                .quote
                .as_ref()
                .map(|q| q.id.to_string())
                .unwrap_or_default(),
            Mode::Zen => "zen".to_string(),
            Mode::Custom => "custom".to_string(),
        }
    }

    /// Build the final result. Call once `state()` is `Finished`/`Failed`.
    pub fn result(&self) -> TestResult {
        let now = self.finish_ms.unwrap_or(0);
        let duration = self.elapsed_secs(now).max(0.0);
        let cc = self.char_counts(true);

        let wpm = round_to_2(calculate_wpm(cc.correct_word as f64, duration));
        let raw = round_to_2(calculate_wpm(
            (cc.all_correct + cc.incorrect + cc.extra) as f64,
            duration,
        ));
        let acc = round_to_2(self.accuracy_pct());
        let cons = consistency(&self.word_bursts);

        let (wpm_history, raw_history) = self.histories(duration);

        TestResult {
            wpm,
            raw_wpm: raw,
            acc,
            consistency: cons,
            char_correct: cc.correct_word,
            char_incorrect: cc.incorrect,
            char_extra: cc.extra,
            char_missed: cc.missed,
            char_total: cc.all_correct + cc.incorrect + cc.extra,
            duration_sec: round_to_2(duration),
            mode: self.config.mode,
            mode2: self.mode2(),
            punctuation: self.config.punctuation,
            numbers: self.config.numbers,
            language: self.config.language.clone(),
            wpm_history,
            raw_history,
            failed: self.state == State::Failed,
            fail_reason: self.fail_reason.clone(),
            quote_source: self.quote.as_ref().map(|q| q.source.clone()),
        }
    }

    /// Per-second net & raw WPM curves for the results chart. Approximates
    /// Monkeytype's per-second boundaries: cumulative correct/raw chars / time.
    fn histories(&self, duration: f64) -> (Vec<f64>, Vec<f64>) {
        let seconds = duration.ceil().max(1.0) as usize;
        let start = self.start_ms.unwrap_or(0);
        let mut wpm_hist = Vec::with_capacity(seconds);
        let mut raw_hist = Vec::with_capacity(seconds);
        for s in 1..=seconds {
            let boundary = start + (s as u128) * 1000;
            // net: cumulative correctWord chars from words committed by boundary
            let correct: usize = self
                .committed
                .iter()
                .filter(|w| w.commit_ms <= boundary)
                .map(|w| w.correct_word_chars)
                .sum();
            // raw: cumulative letters typed by boundary
            let raw: usize = self
                .keystrokes
                .iter()
                .filter(|k| k.letter && k.ms <= boundary)
                .count();
            wpm_hist.push(round_to_2(calculate_wpm(correct as f64, s as f64)));
            raw_hist.push(round_to_2(calculate_wpm(raw as f64, s as f64)));
        }
        (wpm_hist, raw_hist)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Mode;
    use rand::SeedableRng;

    fn engine_with(words: &[&str], cfg: Config) -> Engine {
        let rng = StdRng::seed_from_u64(0);
        let mut e = Engine::new(cfg, rng);
        e.target_words = words.iter().map(|s| s.to_string()).collect();
        e.typed = vec![String::new(); e.target_words.len().max(1)];
        e.active = 0;
        e.committed.clear();
        e.word_bursts.clear();
        e.keystrokes.clear();
        e.state = State::BeforeStart;
        e.start_ms = None;
        e.finish_ms = None;
        e
    }

    /// Type a string of letters/spaces with 100ms between keystrokes starting at
    /// `t0`. Returns the timestamp after the last keystroke.
    fn type_str(e: &mut Engine, s: &str, t0: u128) -> u128 {
        let mut t = t0;
        for c in s.chars() {
            e.type_char(c, t);
            t += 100;
        }
        t.saturating_sub(100)
    }

    fn words_cfg(words: u32) -> Config {
        let mut c = Config::default();
        c.mode = Mode::Words;
        c.words = words;
        c
    }

    #[test]
    fn perfect_two_word_test() {
        let mut e = engine_with(&["the", "cat"], words_cfg(2));
        // t,h,e at 0,100,200; space@300 commits "the"; c,a,t at 400,500,600
        e.type_char('t', 0);
        e.type_char('h', 100);
        e.type_char('e', 200);
        e.type_char(' ', 300);
        e.type_char('c', 400);
        e.type_char('a', 500);
        e.type_char('t', 600); // completes last word -> finish
        assert_eq!(e.state(), State::Finished);

        let r = e.result();
        // correctWord = 4 (the+space) + 3 (cat) = 7; duration 0.6s
        // wpm = 7/5/(0.6/60) = 140
        assert_eq!(r.char_correct, 7);
        assert_eq!(r.wpm, 140.0);
        assert_eq!(r.raw_wpm, 140.0);
        assert_eq!(r.acc, 100.0);
        assert_eq!(r.char_incorrect, 0);
        assert_eq!(r.char_extra, 0);
    }

    #[test]
    fn incorrect_word_counts_zero_net_but_raw() {
        let mut e = engine_with(&["the", "cat"], words_cfg(2));
        // mistype "teh" then space, then "cat"
        e.type_char('t', 0);
        e.type_char('e', 100);
        e.type_char('h', 200);
        e.type_char(' ', 300);
        e.type_char('c', 400);
        e.type_char('a', 500);
        e.type_char('t', 600);
        let r = e.result();
        // "teh " vs "the ": t correct(1), e/h incorrect(2), space->extra(1) => correctWord 0
        // "cat" correct => correctWord 3. net correctWord = 3
        assert_eq!(r.char_correct, 3);
        assert_eq!(r.char_incorrect, 2);
        assert_eq!(r.char_extra, 1);
        // raw chars = allCorrect(1+3=4) + incorrect(2) + extra(1) = 7
        assert_eq!(r.char_total, 7);
        // accuracy: 6 letters typed + 1 space-commit (incorrect) = 7 keystrokes;
        // correct letters: t,c,a,t = 4; space commit incorrect. => 4/7
        assert!((r.acc - round_to_2(4.0 / 7.0 * 100.0)).abs() < 0.01);
    }

    #[test]
    fn backspace_within_word() {
        let mut e = engine_with(&["hello"], words_cfg(1));
        type_str(&mut e, "helx", 0);
        e.backspace(false, 500);
        type_str(&mut e, "lo", 600);
        assert_eq!(e.state(), State::Finished);
        let r = e.result();
        assert_eq!(r.char_correct, 5);
        assert_eq!(r.char_incorrect, 0);
        // accuracy: typed h,e,l,x(wrong),l,o = 6 letters, 5 correct => 5/6
        assert!((r.acc - round_to_2(5.0 / 6.0 * 100.0)).abs() < 0.01);
    }

    #[test]
    fn cannot_backspace_to_correct_previous_word() {
        let mut e = engine_with(&["the", "cat"], words_cfg(2));
        e.type_char('t', 0);
        e.type_char('h', 100);
        e.type_char('e', 200);
        e.type_char(' ', 300); // commit "the" (correct)
        assert_eq!(e.active, 1);
        e.backspace(false, 400); // at start of word 2, prev correct -> blocked
        assert_eq!(e.active, 1);
    }

    #[test]
    fn can_backspace_to_incorrect_previous_word() {
        let mut e = engine_with(&["the", "cat"], words_cfg(2));
        e.type_char('t', 0);
        e.type_char('e', 100);
        e.type_char('h', 200);
        e.type_char(' ', 300); // commit "teh" (incorrect)
        assert_eq!(e.active, 1);
        e.backspace(false, 400); // prev incorrect -> allowed
        assert_eq!(e.active, 0);
    }

    #[test]
    fn confidence_max_blocks_backspace() {
        let mut c = words_cfg(1);
        c.confidence_mode = ConfidenceMode::Max;
        let mut e = engine_with(&["hello"], c);
        type_str(&mut e, "hel", 0);
        e.backspace(false, 400);
        assert_eq!(e.typed[0], "hel");
    }

    #[test]
    fn master_difficulty_fails_on_error() {
        let mut c = words_cfg(5);
        c.difficulty = Difficulty::Master;
        let mut e = engine_with(&["hello", "world", "foo", "bar", "baz"], c);
        e.type_char('h', 0);
        e.type_char('x', 100); // wrong
        assert_eq!(e.state(), State::Failed);
    }

    #[test]
    fn expert_difficulty_fails_on_imperfect_commit() {
        let mut c = words_cfg(5);
        c.difficulty = Difficulty::Expert;
        let mut e = engine_with(&["hello", "world", "foo", "bar", "baz"], c);
        type_str(&mut e, "helo", 0); // missing an l
        e.type_char(' ', 500); // commit imperfect word
        assert_eq!(e.state(), State::Failed);
    }

    #[test]
    fn time_mode_finishes_on_tick() {
        let mut c = Config::default();
        c.mode = Mode::Time;
        c.time = 1;
        let mut e = engine_with(&["the", "cat", "dog", "run"], c);
        e.type_char('t', 0);
        e.type_char('h', 200);
        e.type_char('e', 400);
        e.type_char(' ', 600);
        e.tick(1100); // past 1s
        assert_eq!(e.state(), State::Finished);
        let r = e.result();
        assert!(r.wpm > 0.0);
    }

    #[test]
    fn zen_counts_everything_correct() {
        let mut c = Config::default();
        c.mode = Mode::Zen;
        let mut e = engine_with(&[], c);
        e.type_char('a', 0);
        e.type_char('s', 100);
        e.type_char('d', 200);
        e.type_char('f', 300);
        e.type_char(' ', 400);
        e.type_char('j', 500);
        e.type_char('k', 600);
        e.bail(700);
        let r = e.result();
        assert_eq!(r.char_incorrect, 0);
        assert!(r.char_correct >= 4);
    }

    #[test]
    fn min_wpm_fails_slow_typing() {
        let mut c = words_cfg(50);
        c.min_wpm = Some(100);
        let mut e = engine_with(&["the", "cat", "dog"], c);
        e.type_char('t', 0);
        e.type_char('h', 1000);
        e.type_char('e', 2000);
        e.type_char(' ', 3000);
        e.tick(4000); // very slow -> below 100 wpm
        assert_eq!(e.state(), State::Failed);
    }
}
