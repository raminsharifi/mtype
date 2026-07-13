//! Local results history + personal bests - the offline, account-free stand-in
//! for Monkeytype's saved results. Stored as JSON in the platform data dir.
//!
//! PB rule mirrors `backend/src/utils/pb.ts`: a result is a personal best only
//! if its WPM is *strictly greater* than the previous best for the same test
//! category. Only `time` and `words` modes are PB-eligible (quote/zen/custom
//! are not, as in Monkeytype).

use crate::config::{Config, Mode, PaceCaret};
use crate::engine::{InputEvent, TestResult, WordOutcome};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredResult {
    pub wpm: f64,
    pub raw_wpm: f64,
    pub acc: f64,
    pub consistency: f64,
    pub mode: String,
    pub mode2: String,
    pub punctuation: bool,
    pub numbers: bool,
    pub language: String,
    pub difficulty: String,
    pub duration_sec: f64,
    pub timestamp_ms: u128,
    // character breakdown (added later; old records default to 0)
    #[serde(default)]
    pub char_correct: usize,
    #[serde(default)]
    pub char_incorrect: usize,
    #[serde(default)]
    pub char_extra: usize,
    #[serde(default)]
    pub char_missed: usize,
    #[serde(default)]
    pub word_outcomes: Vec<WordOutcome>,
    #[serde(default)]
    pub input_events: Vec<InputEvent>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PbInfo {
    pub is_pb: bool,
    pub previous_best: Option<f64>,
}

fn data_path() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "monkeytype", "mtype")?;
    Some(dirs.data_dir().join("results.json"))
}

pub fn load_history() -> Vec<StoredResult> {
    let Some(path) = data_path() else {
        return Vec::new();
    };
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn save_history(history: &[StoredResult]) {
    let Some(path) = data_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string(history) {
        let _ = std::fs::write(path, s);
    }
}

pub fn pb_eligible(mode: Mode) -> bool {
    matches!(mode, Mode::Time | Mode::Words)
}

/// Whether a result is worth saving (mirrors Monkeytype's basic validity gate:
/// not failed, long enough, and something was typed).
pub fn is_valid(result: &TestResult) -> bool {
    !result.failed && result.duration_sec >= 1.0 && result.char_total > 0
}

fn category_matches(s: &StoredResult, r: &TestResult, difficulty: &str) -> bool {
    s.mode == r.mode.as_str()
        && s.mode2 == r.mode2
        && s.punctuation == r.punctuation
        && s.numbers == r.numbers
        && s.language == r.language
        && s.difficulty == difficulty
}

/// Best WPM previously recorded for this result's category, if any.
pub fn previous_best(history: &[StoredResult], r: &TestResult, difficulty: &str) -> Option<f64> {
    history
        .iter()
        .filter(|s| category_matches(s, r, difficulty))
        .map(|s| s.wpm)
        .fold(None, |acc, w| Some(acc.map_or(w, |a: f64| a.max(w))))
}

/// Resolve a pace-caret speed once per test so rendering never touches disk.
pub fn pace_wpm(config: &Config) -> Option<f64> {
    pace_wpm_from(&load_history(), config)
}

/// Pure pace resolver (history in, speed out) so the filter is testable.
/// The match uses the same category key as `category_matches` / upstream's
/// `getLocalPB`: mode, mode2, punctuation, numbers, language, and difficulty -
/// a time-15 PB must never pace a time-60 test. mode2 is only predictable
/// before the test starts for time/words/practice (a quote's id is picked at
/// generation time), so the other modes match on mode alone.
fn pace_wpm_from(history: &[StoredResult], config: &Config) -> Option<f64> {
    if config.pace_caret == PaceCaret::Off {
        return None;
    }
    let mode2 = match config.mode {
        Mode::Time => Some(config.time.to_string()),
        Mode::Words => Some(config.words.to_string()),
        Mode::Practice => Some(config.practice_mode.as_str().to_string()),
        Mode::Quote | Mode::Zen | Mode::Custom => None,
    };
    let matching = |result: &&StoredResult| {
        result.mode == config.mode.as_str()
            && mode2.as_deref().is_none_or(|mode2| result.mode2 == mode2)
            && result.language == config.language
            && result.punctuation == config.punctuation
            && result.numbers == config.numbers
            && result.difficulty == config.difficulty.as_str()
    };
    match config.pace_caret {
        PaceCaret::Off => None,
        PaceCaret::Custom => Some(config.pace_caret_custom_speed as f64),
        PaceCaret::Last => history.iter().rev().find(matching).map(|result| result.wpm),
        PaceCaret::Pb => history
            .iter()
            .filter(matching)
            .map(|result| result.wpm)
            .reduce(f64::max),
        PaceCaret::Average => {
            let values = history
                .iter()
                .rev()
                .filter(matching)
                .take(10)
                .map(|result| result.wpm)
                .collect::<Vec<_>>();
            (!values.is_empty()).then(|| mean(&values))
        }
    }
}

/// Record a completed result (if valid + saving enabled) and report whether it
/// was a personal best. Returns PB info for the results screen.
pub fn record(result: &TestResult, difficulty: &str, saving_enabled: bool) -> PbInfo {
    if !saving_enabled || !is_valid(result) {
        return PbInfo::default();
    }

    let mut history = load_history();
    let prev = previous_best(&history, result, difficulty);
    let is_pb = pb_eligible(result.mode) && prev.is_none_or(|b| result.wpm > b);

    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);

    history.push(StoredResult {
        wpm: result.wpm,
        raw_wpm: result.raw_wpm,
        acc: result.acc,
        consistency: result.consistency,
        mode: result.mode.as_str().to_string(),
        mode2: result.mode2.clone(),
        punctuation: result.punctuation,
        numbers: result.numbers,
        language: result.language.clone(),
        difficulty: difficulty.to_string(),
        duration_sec: result.duration_sec,
        timestamp_ms,
        char_correct: result.char_correct,
        char_incorrect: result.char_incorrect,
        char_extra: result.char_extra,
        char_missed: result.char_missed,
        word_outcomes: result.word_outcomes.clone(),
        input_events: result.input_events.clone(),
    });

    // Keep the full history: the profile screen derives lifetime totals and
    // personal bests from these records, so discarding older tests would make
    // both silently become inaccurate over time.
    save_history(&history);
    // The normalized database is best-effort so a database issue never loses
    // the human-readable result or interrupts the typing flow.
    let _ = crate::analytics::record_test(result, timestamp_ms, difficulty);

    PbInfo {
        is_pb,
        previous_best: prev,
    }
}

// ---------------------------------------------------------------------------
// Profile / progress tracking (the local equivalent of Monkeytype's account
// page: lifetime stats, a WPM-over-time graph, an activity heatmap, streaks).
// ---------------------------------------------------------------------------

const MS_PER_DAY: u128 = 86_400_000;

fn meta_path() -> Option<PathBuf> {
    data_path().map(|p| p.with_file_name("meta.json"))
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Meta {
    #[serde(default)]
    started_tests: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct DataBundle {
    version: u32,
    results: Vec<StoredResult>,
    started_tests: u64,
}

/// Export all portable local data, including word outcomes and input replay.
pub fn export_data(path: &std::path::Path) -> Result<()> {
    let bundle = DataBundle {
        version: 1,
        results: load_history(),
        started_tests: load_meta().started_tests,
    };
    let json = serde_json::to_string_pretty(&bundle)?;
    std::fs::write(path, json)
        .with_context(|| format!("could not write export to {}", path.display()))
}

/// Merge a portable export into local history and rebuild normalized analytics.
pub fn import_data(path: &std::path::Path) -> Result<usize> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("could not read import from {}", path.display()))?;
    let bundle: DataBundle = serde_json::from_str(&source).context("invalid mtype data export")?;
    anyhow::ensure!(bundle.version == 1, "unsupported data export version");
    let mut history = load_history();
    let before = history.len();
    history.extend(bundle.results);
    history.sort_by_key(|result| result.timestamp_ms);
    history.dedup_by_key(|result| result.timestamp_ms);
    save_history(&history);

    let mut meta = load_meta();
    meta.started_tests = meta
        .started_tests
        .max(bundle.started_tests)
        .max(history.len() as u64);
    save_meta(&meta);

    for stored in &history {
        if let Some(result) = stored.as_test_result() {
            let _ = crate::analytics::record_test(&result, stored.timestamp_ms, &stored.difficulty);
        }
    }
    Ok(history.len().saturating_sub(before))
}

/// Remove results, counters, and the normalized analytics database.
pub fn reset_all_data() -> Result<()> {
    for path in [
        data_path(),
        meta_path(),
        crate::analytics::database_path().ok(),
    ]
    .into_iter()
    .flatten()
    {
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("could not remove {}", path.display()))?;
        }
        for suffix in ["-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{suffix}", path.display()));
            let _ = std::fs::remove_file(sidecar);
        }
    }
    Ok(())
}

impl StoredResult {
    fn as_test_result(&self) -> Option<TestResult> {
        Some(TestResult {
            wpm: self.wpm,
            raw_wpm: self.raw_wpm,
            acc: self.acc,
            consistency: self.consistency,
            char_correct: self.char_correct,
            char_incorrect: self.char_incorrect,
            char_extra: self.char_extra,
            char_missed: self.char_missed,
            char_total: self.char_correct + self.char_incorrect + self.char_extra,
            duration_sec: self.duration_sec,
            mode: Mode::from_str_opt(&self.mode)?,
            mode2: self.mode2.clone(),
            punctuation: self.punctuation,
            numbers: self.numbers,
            language: self.language.clone(),
            wpm_history: vec![],
            raw_history: vec![],
            failed: false,
            fail_reason: None,
            quote_source: None,
            word_outcomes: self.word_outcomes.clone(),
            input_events: self.input_events.clone(),
        })
    }
}

fn load_meta() -> Meta {
    let Some(path) = meta_path() else {
        return Meta::default();
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_meta(meta: &Meta) {
    if let Some(path) = meta_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string(meta) {
            let _ = std::fs::write(path, json);
        }
    }
}

/// Count a test as started (called once when typing begins). Mirrors
/// Monkeytype's `startedTests`.
#[cfg(not(test))]
pub fn increment_started_tests() {
    let mut meta = load_meta();
    meta.started_tests = meta.started_tests.saturating_add(1);
    save_meta(&meta);
}

// App unit tests type into an `App` directly. Do not let those keystrokes
// mutate the developer's real profile in the platform data directory.
#[cfg(test)]
pub fn increment_started_tests() {}

/// One day's worth of activity for the heatmap.
#[derive(Debug, Clone, Copy)]
pub struct DayActivity {
    /// Whole days since the Unix epoch (UTC).
    pub day: i64,
    pub count: u32,
}

/// All lifetime stats, derived from the local results history.
#[derive(Debug, Default, Clone)]
pub struct Profile {
    pub started: u64,
    pub completed: u64,
    pub time_typing_sec: f64,
    pub estimated_words: u64,

    pub highest_wpm: f64,
    pub avg_wpm: f64,
    pub avg_wpm_last10: f64,
    pub highest_raw: f64,
    pub avg_raw: f64,
    pub highest_acc: f64,
    pub avg_acc: f64,
    pub avg_acc_last10: f64,
    pub highest_consistency: f64,
    pub avg_consistency: f64,

    /// WPM of every completed test in chronological order (the progress graph).
    pub wpm_history: Vec<f64>,
    /// Per-day activity in chronological order (the heatmap).
    pub activity: Vec<DayActivity>,
    /// Whole days since the epoch for "today" (the heatmap reference column).
    pub today: i64,
    pub current_streak: u32,
    pub max_streak: u32,

    /// Most recent results first.
    pub recent: Vec<StoredResult>,
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

fn max_of(xs: &[f64]) -> f64 {
    xs.iter().copied().fold(0.0_f64, f64::max)
}

/// Build the profile from the local results history on disk.
pub fn compute_profile(now_ms: u128) -> Profile {
    let history = load_history();
    let started = load_meta().started_tests.max(history.len() as u64);
    build_profile(history, started, now_ms)
}

/// Pure profile builder (history + counters in, stats out) so it is testable.
fn build_profile(mut history: Vec<StoredResult>, started: u64, now_ms: u128) -> Profile {
    history.sort_by_key(|r| r.timestamp_ms);

    let today = (now_ms / MS_PER_DAY) as i64;
    if history.is_empty() {
        return Profile {
            started,
            today,
            ..Default::default()
        };
    }

    let wpms: Vec<f64> = history.iter().map(|r| r.wpm).collect();
    let raws: Vec<f64> = history.iter().map(|r| r.raw_wpm).collect();
    let accs: Vec<f64> = history.iter().map(|r| r.acc).collect();
    let cons: Vec<f64> = history
        .iter()
        .map(|r| r.consistency)
        .filter(|c| *c > 0.0)
        .collect();

    let last10 = |xs: &[f64]| -> f64 {
        let n = xs.len();
        let start = n.saturating_sub(10);
        mean(&xs[start..])
    };

    let time_typing_sec: f64 = history.iter().map(|r| r.duration_sec).sum();
    let estimated_words: u64 = history
        .iter()
        .map(|r| (r.wpm * r.duration_sec / 60.0).round() as u64)
        .sum();

    // activity per day
    let mut by_day: std::collections::BTreeMap<i64, u32> = std::collections::BTreeMap::new();
    for r in &history {
        let day = (r.timestamp_ms / MS_PER_DAY) as i64;
        *by_day.entry(day).or_insert(0) += 1;
    }
    let activity: Vec<DayActivity> = by_day
        .iter()
        .map(|(&day, &count)| DayActivity { day, count })
        .collect();
    let (current_streak, max_streak) = streaks(&by_day.keys().copied().collect::<Vec<_>>(), now_ms);

    let mut recent = history.clone();
    recent.reverse();
    recent.truncate(20);

    Profile {
        started,
        completed: history.len() as u64,
        time_typing_sec,
        estimated_words,
        highest_wpm: max_of(&wpms),
        avg_wpm: mean(&wpms),
        avg_wpm_last10: last10(&wpms),
        highest_raw: max_of(&raws),
        avg_raw: mean(&raws),
        highest_acc: max_of(&accs),
        avg_acc: mean(&accs),
        avg_acc_last10: last10(&accs),
        highest_consistency: max_of(&cons),
        avg_consistency: mean(&cons),
        wpm_history: wpms,
        activity,
        today,
        current_streak,
        max_streak,
        recent,
    }
}

/// Current and maximum streak (consecutive days with at least one test). The
/// current streak counts only if the latest active day is today or yesterday.
fn streaks(days_sorted: &[i64], now_ms: u128) -> (u32, u32) {
    if days_sorted.is_empty() {
        return (0, 0);
    }
    // longest run of consecutive days anywhere
    let mut max_streak = 1u32;
    let mut run = 1u32;
    for w in days_sorted.windows(2) {
        if w[1] == w[0] + 1 {
            run += 1;
        } else {
            run = 1;
        }
        max_streak = max_streak.max(run);
    }

    // current streak: walk back from the most recent active day, but only if it
    // is today or yesterday (otherwise the streak is broken).
    let today = (now_ms / MS_PER_DAY) as i64;
    let last = *days_sorted.last().unwrap();
    let current_streak = if last == today || last == today - 1 {
        let mut s = 1u32;
        let mut prev = last;
        for &d in days_sorted.iter().rev().skip(1) {
            if d == prev - 1 {
                s += 1;
                prev = d;
            } else {
                break;
            }
        }
        s
    } else {
        0
    };

    (current_streak, max_streak)
}

/// Convert whole days since the Unix epoch into (year, month, day), using
/// Howard Hinnant's civil-from-days algorithm. Used for heatmap month labels.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Mode;

    fn mk_stored(wpm: f64, mode: &str, mode2: &str, timestamp_ms: u128) -> StoredResult {
        StoredResult {
            wpm,
            raw_wpm: wpm + 5.0,
            acc: 96.0,
            consistency: 80.0,
            mode: mode.to_string(),
            mode2: mode2.to_string(),
            punctuation: false,
            numbers: false,
            language: "english".to_string(),
            difficulty: "normal".to_string(),
            duration_sec: 30.0,
            timestamp_ms,
            char_correct: 150,
            char_incorrect: 2,
            char_extra: 0,
            char_missed: 0,
            word_outcomes: vec![],
            input_events: vec![],
        }
    }

    fn mk_result(wpm: f64, mode: Mode, mode2: &str) -> TestResult {
        TestResult {
            wpm,
            raw_wpm: wpm,
            acc: 100.0,
            consistency: 90.0,
            char_correct: 50,
            char_incorrect: 0,
            char_extra: 0,
            char_missed: 0,
            char_total: 50,
            duration_sec: 30.0,
            mode,
            mode2: mode2.to_string(),
            punctuation: false,
            numbers: false,
            language: "english".to_string(),
            wpm_history: vec![],
            raw_history: vec![],
            failed: false,
            fail_reason: None,
            quote_source: None,
            word_outcomes: vec![],
            input_events: vec![],
        }
    }

    #[test]
    fn pb_is_strictly_greater() {
        let history = vec![mk_stored(80.0, "time", "30", 0)];
        let equal = mk_result(80.0, Mode::Time, "30");
        let better = mk_result(81.0, Mode::Time, "30");
        let previous = previous_best(&history, &equal, "normal").unwrap();
        // equal WPM is NOT a pb
        assert_eq!(equal.wpm, previous);
        assert!(better.wpm > previous);
    }

    #[test]
    fn different_category_is_independent() {
        let history = vec![mk_stored(100.0, "time", "60", 0)];
        let other = mk_result(50.0, Mode::Time, "30");
        assert_eq!(previous_best(&history, &other, "normal"), None);
    }

    #[test]
    fn pace_sources_filter_by_mode2() {
        use crate::config::{Config, Difficulty, PaceCaret};
        // a fast time-15 burst must never pace a time-60 test
        let history = vec![
            mk_stored(130.0, "time", "15", 0),
            mk_stored(90.0, "time", "60", 1),
            mk_stored(80.0, "time", "60", 2),
        ];
        let mut config = Config {
            mode: Mode::Time,
            time: 60,
            pace_caret: PaceCaret::Pb,
            ..Config::default()
        };
        assert_eq!(pace_wpm_from(&history, &config), Some(90.0));
        config.time = 15;
        assert_eq!(pace_wpm_from(&history, &config), Some(130.0));

        config.time = 60;
        config.pace_caret = PaceCaret::Average;
        assert_eq!(pace_wpm_from(&history, &config), Some(85.0));
        config.pace_caret = PaceCaret::Last;
        assert_eq!(pace_wpm_from(&history, &config), Some(80.0));

        // difficulty is part of the category too: no expert history -> no pace
        config.pace_caret = PaceCaret::Pb;
        config.difficulty = Difficulty::Expert;
        assert_eq!(pace_wpm_from(&history, &config), None);
    }

    #[test]
    fn profile_aggregates_correctly() {
        let day = MS_PER_DAY;
        let history = vec![
            mk_stored(60.0, "time", "30", 10 * day),
            mk_stored(80.0, "time", "30", 11 * day),
            mk_stored(100.0, "time", "30", 12 * day),
        ];
        let now = 12 * day + 1000; // "today" is day 12
        let p = build_profile(history, 5, now);
        assert_eq!(p.completed, 3);
        assert_eq!(p.started, 5);
        assert_eq!(p.highest_wpm, 100.0);
        assert!((p.avg_wpm - 80.0).abs() < 1e-9);
        assert_eq!(p.wpm_history, vec![60.0, 80.0, 100.0]);
        assert!((p.time_typing_sec - 90.0).abs() < 1e-9);
        // 3 consecutive days ending today
        assert_eq!(p.current_streak, 3);
        assert_eq!(p.max_streak, 3);
        assert_eq!(p.activity.len(), 3);
    }

    #[test]
    fn streak_breaks_with_a_gap_and_resets_if_stale() {
        let today = 100i64;
        let now = (today as u128) * MS_PER_DAY + 5;
        // days 96,97 (run of 2), gap, 99,100 (run of 2 ending today)
        let (cur, max) = streaks(&[96, 97, 99, 100], now);
        assert_eq!(cur, 2);
        assert_eq!(max, 2);
        // latest activity was 3 days ago -> current streak is 0 (stale)
        let (cur2, _) = streaks(&[90, 91, 92], now);
        assert_eq!(cur2, 0);
        // active yesterday still counts as a live streak
        let (cur3, _) = streaks(&[98, 99], now);
        assert_eq!(cur3, 2);
    }

    #[test]
    fn civil_from_days_matches_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1)); // unix epoch
        assert_eq!(civil_from_days(18_993), (2022, 1, 1));
    }

    #[test]
    fn empty_history_profile_is_zeroed() {
        let p = build_profile(vec![], 0, 1000);
        assert_eq!(p.completed, 0);
        assert_eq!(p.highest_wpm, 0.0);
        assert!(p.wpm_history.is_empty());
        assert_eq!(p.current_streak, 0);
    }

    #[test]
    fn quote_mode_not_pb_eligible() {
        assert!(!pb_eligible(Mode::Quote));
        assert!(pb_eligible(Mode::Time));
        assert!(pb_eligible(Mode::Words));
    }

    #[test]
    fn invalid_results_rejected() {
        let mut r = mk_result(50.0, Mode::Time, "30");
        r.failed = true;
        assert!(!is_valid(&r));
        let mut short = mk_result(50.0, Mode::Time, "30");
        short.duration_sec = 0.5;
        assert!(!is_valid(&short));
    }
}
