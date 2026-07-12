//! Offline analytics database.
//!
//! `results.json` remains the human-readable compatibility store. This SQLite
//! database preserves normalized test, word, and input-event rows for adaptive
//! practice, replay, exports, and future local ML without requiring an account
//! or sending data anywhere.

use crate::config::PracticeMode;
use crate::engine::{InputEventKind, TestResult};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;

const SCHEMA_VERSION: i64 = 1;

pub fn database_path() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "monkeytype", "mtype")
        .context("could not resolve a platform data directory")?;
    Ok(dirs.data_dir().join("analytics.sqlite3"))
}

fn open() -> Result<Connection> {
    let path = database_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("could not create {}", parent.display()))?;
    }
    let conn =
        Connection::open(&path).with_context(|| format!("could not open {}", path.display()))?;
    initialize(&conn)?;
    Ok(conn)
}

fn initialize(conn: &Connection) -> Result<()> {
    conn.execute_batch(&format!(
        r#"
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;

        CREATE TABLE IF NOT EXISTS schema_meta (
            version INTEGER NOT NULL
        );
        INSERT INTO schema_meta(version)
        SELECT {SCHEMA_VERSION}
        WHERE NOT EXISTS (SELECT 1 FROM schema_meta);

        CREATE TABLE IF NOT EXISTS test_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            external_id TEXT NOT NULL UNIQUE,
            occurred_at_ms INTEGER NOT NULL,
            mode TEXT NOT NULL,
            mode2 TEXT NOT NULL,
            language TEXT NOT NULL,
            difficulty TEXT NOT NULL,
            punctuation INTEGER NOT NULL,
            numbers INTEGER NOT NULL,
            duration_sec REAL NOT NULL,
            wpm REAL NOT NULL,
            raw_wpm REAL NOT NULL,
            accuracy REAL NOT NULL,
            consistency REAL NOT NULL,
            failed INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS word_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            test_id INTEGER NOT NULL REFERENCES test_sessions(id) ON DELETE CASCADE,
            word_index INTEGER NOT NULL,
            target_word TEXT NOT NULL,
            typed_word TEXT NOT NULL,
            preceding_word TEXT,
            duration_ms INTEGER NOT NULL,
            burst_wpm REAL NOT NULL,
            correct INTEGER NOT NULL,
            had_error INTEGER NOT NULL,
            incorrect_keystrokes INTEGER NOT NULL,
            char_correct INTEGER NOT NULL,
            char_incorrect INTEGER NOT NULL,
            char_extra INTEGER NOT NULL,
            char_missed INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS input_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            test_id INTEGER NOT NULL REFERENCES test_sessions(id) ON DELETE CASCADE,
            sequence INTEGER NOT NULL,
            elapsed_ms INTEGER NOT NULL,
            word_index INTEGER NOT NULL,
            kind TEXT NOT NULL,
            value TEXT,
            correct INTEGER
        );

        CREATE INDEX IF NOT EXISTS idx_tests_occurred
            ON test_sessions(occurred_at_ms DESC);
        CREATE INDEX IF NOT EXISTS idx_words_target
            ON word_events(target_word);
        CREATE INDEX IF NOT EXISTS idx_words_errors
            ON word_events(had_error, correct);
        CREATE INDEX IF NOT EXISTS idx_inputs_test_sequence
            ON input_events(test_id, sequence);

        CREATE VIEW IF NOT EXISTS word_practice_stats AS
        SELECT
            t.language AS language,
            w.target_word AS target_word,
            COUNT(*) AS attempts,
            SUM(w.had_error) AS error_attempts,
            SUM(CASE WHEN w.correct = 0 THEN 1 ELSE 0 END) AS missed_attempts,
            AVG(CASE WHEN w.burst_wpm > 0 THEN w.burst_wpm END) AS avg_burst_wpm,
            MAX(t.occurred_at_ms) AS last_seen_ms
        FROM word_events w
        JOIN test_sessions t ON t.id = w.test_id
        WHERE w.target_word <> ''
        GROUP BY t.language, w.target_word;
        "#
    ))
    .context("could not initialize analytics database")?;
    Ok(())
}

fn int(value: usize) -> i64 {
    value.min(i64::MAX as usize) as i64
}

fn timestamp(value: u128) -> i64 {
    value.min(i64::MAX as u128) as i64
}

fn event_kind(kind: &InputEventKind) -> &'static str {
    match kind {
        InputEventKind::Character => "character",
        InputEventKind::Commit => "commit",
        InputEventKind::Backspace => "backspace",
        InputEventKind::WordBackspace => "word_backspace",
    }
}

/// Persist a complete test and all word/input events in one transaction.
/// Duplicate `external_id`s are ignored, making retries idempotent.
pub fn record_test(result: &TestResult, occurred_at_ms: u128, difficulty: &str) -> Result<()> {
    let mut conn = open()?;
    record_test_in(&mut conn, result, occurred_at_ms, difficulty)
}

fn record_test_in(
    conn: &mut Connection,
    result: &TestResult,
    occurred_at_ms: u128,
    difficulty: &str,
) -> Result<()> {
    let tx = conn
        .transaction()
        .context("could not start analytics transaction")?;
    let external_id = occurred_at_ms.to_string();
    let inserted = tx.execute(
        r#"INSERT OR IGNORE INTO test_sessions (
            external_id, occurred_at_ms, mode, mode2, language, difficulty,
            punctuation, numbers, duration_sec, wpm, raw_wpm, accuracy,
            consistency, failed
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        params![
            external_id,
            timestamp(occurred_at_ms),
            result.mode.as_str(),
            result.mode2,
            result.language,
            difficulty,
            result.punctuation,
            result.numbers,
            result.duration_sec,
            result.wpm,
            result.raw_wpm,
            result.acc,
            result.consistency,
            result.failed,
        ],
    )?;
    if inserted == 0 {
        tx.commit()?;
        return Ok(());
    }
    let test_id = tx.last_insert_rowid();

    {
        let mut insert_word = tx.prepare_cached(
            r#"INSERT INTO word_events (
                test_id, word_index, target_word, typed_word, preceding_word,
                duration_ms, burst_wpm, correct, had_error,
                incorrect_keystrokes, char_correct, char_incorrect,
                char_extra, char_missed
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )?;
        for word in &result.word_outcomes {
            insert_word.execute(params![
                test_id,
                int(word.word_index),
                word.target,
                word.typed,
                word.preceding_word,
                word.duration_ms.min(i64::MAX as u64) as i64,
                word.burst_wpm,
                word.correct,
                word.had_error,
                int(word.incorrect_keystrokes),
                int(word.char_correct),
                int(word.char_incorrect),
                int(word.char_extra),
                int(word.char_missed),
            ])?;
        }
    }

    {
        let mut insert_input = tx.prepare_cached(
            r#"INSERT INTO input_events (
                test_id, sequence, elapsed_ms, word_index, kind, value, correct
            ) VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )?;
        for (sequence, event) in result.input_events.iter().enumerate() {
            insert_input.execute(params![
                test_id,
                int(sequence),
                event.elapsed_ms.min(i64::MAX as u64) as i64,
                int(event.word_index),
                event_kind(&event.kind),
                event.value,
                event.correct,
            ])?;
        }
    }

    tx.commit()
        .context("could not commit analytics transaction")?;
    Ok(())
}

/// Select an ordered adaptive-practice list. Missed mode favors words with
/// errors, slow mode favors the lowest burst speed, and mixed balances both.
pub fn practice_words(language: &str, mode: PracticeMode, count: usize) -> Result<Vec<String>> {
    let conn = open()?;
    practice_words_in(&conn, language, mode, count)
}

fn practice_words_in(
    conn: &Connection,
    language: &str,
    mode: PracticeMode,
    count: usize,
) -> Result<Vec<String>> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let (predicate, ordering) = match mode {
        PracticeMode::Missed => (
            "error_attempts > 0",
            "(missed_attempts * 3 + error_attempts) DESC, last_seen_ms DESC",
        ),
        PracticeMode::Slow => (
            "avg_burst_wpm IS NOT NULL",
            "avg_burst_wpm ASC, attempts DESC, last_seen_ms DESC",
        ),
        PracticeMode::Mixed => (
            "error_attempts > 0 OR avg_burst_wpm IS NOT NULL",
            "((missed_attempts * 4 + error_attempts) * 100.0 / attempts + 600.0 / (COALESCE(avg_burst_wpm, 0) + 10.0)) DESC, last_seen_ms DESC",
        ),
    };
    let sql = format!(
        "SELECT target_word FROM word_practice_stats \
         WHERE language = ?1 AND ({predicate}) ORDER BY {ordering} LIMIT ?2"
    );
    let limit = count.min(500) as i64;
    let mut statement = conn.prepare(&sql)?;
    let unique: Vec<String> = statement
        .query_map(params![language, limit], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if unique.is_empty() {
        return Ok(Vec::new());
    }

    let mut words = Vec::with_capacity(count);
    while words.len() < count {
        for word in &unique {
            if words.len() == count {
                break;
            }
            // Avoid identical adjacent words when only a few weak words exist.
            if words.last() == Some(word) && unique.len() > 1 {
                continue;
            }
            words.push(word.clone());
        }
    }
    Ok(words)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Mode;
    use crate::engine::{InputEvent, WordOutcome};

    fn result() -> TestResult {
        TestResult {
            wpm: 60.0,
            raw_wpm: 65.0,
            acc: 95.0,
            consistency: 80.0,
            char_correct: 10,
            char_incorrect: 1,
            char_extra: 0,
            char_missed: 0,
            char_total: 11,
            duration_sec: 10.0,
            mode: Mode::Words,
            mode2: "2".to_string(),
            punctuation: false,
            numbers: false,
            language: "english".to_string(),
            wpm_history: vec![60.0],
            raw_history: vec![65.0],
            failed: false,
            fail_reason: None,
            quote_source: None,
            word_outcomes: vec![WordOutcome {
                word_index: 0,
                target: "world".to_string(),
                typed: "wrold".to_string(),
                preceding_word: Some("hello".to_string()),
                duration_ms: 900,
                burst_wpm: 72.0,
                correct: false,
                had_error: true,
                incorrect_keystrokes: 2,
                char_correct: 3,
                char_incorrect: 2,
                char_extra: 0,
                char_missed: 0,
            }],
            input_events: vec![InputEvent {
                elapsed_ms: 0,
                word_index: 0,
                kind: InputEventKind::Character,
                value: Some("w".to_string()),
                correct: Some(true),
            }],
        }
    }

    #[test]
    fn stores_normalized_events_and_selects_mistakes() {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        record_test_in(&mut conn, &result(), 1234, "normal").unwrap();
        let words = practice_words_in(&conn, "english", PracticeMode::Missed, 3).unwrap();
        assert_eq!(words, vec!["world", "world", "world"]);
        let events: i64 = conn
            .query_row("SELECT COUNT(*) FROM input_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(events, 1);
    }

    #[test]
    fn duplicate_test_ids_are_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        initialize(&conn).unwrap();
        record_test_in(&mut conn, &result(), 1234, "normal").unwrap();
        record_test_in(&mut conn, &result(), 1234, "normal").unwrap();
        let words: i64 = conn
            .query_row("SELECT COUNT(*) FROM word_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(words, 1);
    }
}
