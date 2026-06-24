//! Local results history + personal bests - the offline, account-free stand-in
//! for Monkeytype's saved results. Stored as JSON in the platform data dir.
//!
//! PB rule mirrors `backend/src/utils/pb.ts`: a result is a personal best only
//! if its WPM is *strictly greater* than the previous best for the same test
//! category. Only `time` and `words` modes are PB-eligible (quote/zen/custom
//! are not, as in Monkeytype).

use crate::config::Mode;
use crate::engine::TestResult;
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
    });

    // keep the most recent 1000 results
    if history.len() > 1000 {
        let excess = history.len() - 1000;
        history.drain(0..excess);
    }
    save_history(&history);

    PbInfo {
        is_pb,
        previous_best: prev,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Mode;

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
        }
    }

    #[test]
    fn pb_is_strictly_greater() {
        let history = vec![StoredResult {
            wpm: 80.0,
            raw_wpm: 80.0,
            acc: 100.0,
            consistency: 90.0,
            mode: "time".to_string(),
            mode2: "30".to_string(),
            punctuation: false,
            numbers: false,
            language: "english".to_string(),
            difficulty: "normal".to_string(),
            duration_sec: 30.0,
            timestamp_ms: 0,
        }];
        let equal = mk_result(80.0, Mode::Time, "30");
        let better = mk_result(81.0, Mode::Time, "30");
        assert_eq!(previous_best(&history, &equal, "normal"), Some(80.0));
        // equal WPM is NOT a pb
        assert!(!(80.0 > previous_best(&history, &equal, "normal").unwrap()));
        assert!(81.0 > previous_best(&history, &better, "normal").unwrap());
    }

    #[test]
    fn different_category_is_independent() {
        let history = vec![StoredResult {
            wpm: 100.0,
            raw_wpm: 100.0,
            acc: 100.0,
            consistency: 90.0,
            mode: "time".to_string(),
            mode2: "60".to_string(),
            punctuation: false,
            numbers: false,
            language: "english".to_string(),
            difficulty: "normal".to_string(),
            duration_sec: 60.0,
            timestamp_ms: 0,
        }];
        let other = mk_result(50.0, Mode::Time, "30");
        assert_eq!(previous_best(&history, &other, "normal"), None);
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
