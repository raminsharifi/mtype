//! Local browser dashboard and its data contract.
//!
//! The server binds to loopback only and embeds every frontend asset in the
//! binary. No typing data leaves the machine.

use crate::config::TypingSpeedUnit;
use crate::persistence::StoredResult;
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const HTML: &str = include_str!("../assets/dashboard.html");
const CSS: &str = include_str!("../assets/dashboard.css");
const JS: &str = include_str!("../assets/dashboard.js");
const MS_PER_DAY: u128 = 86_400_000;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    generated_at_ms: u64,
    /// Display unit every speed value in this payload is already converted to.
    speed_unit: String,
    start_graphs_at_zero: bool,
    overview: Overview,
    timeline: Vec<TimelinePoint>,
    activity: Vec<ActivityDay>,
    modes: Vec<ModeSummary>,
    wrong_words: Vec<WrongWord>,
    slow_words: Vec<SlowWord>,
    confusions: Vec<Confusion>,
    insights: Vec<Insight>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct Overview {
    tests_completed: u64,
    tests_started: u64,
    time_typing_sec: f64,
    estimated_words: u64,
    highest_wpm: f64,
    average_wpm: f64,
    recent_wpm: f64,
    previous_wpm: f64,
    trend_percent: Option<f64>,
    average_accuracy: f64,
    recent_accuracy: f64,
    average_consistency: f64,
    speed_leak: f64,
    current_streak: u32,
    longest_streak: u32,
    mistake_events: u64,
    weak_word_count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelinePoint {
    timestamp_ms: u64,
    wpm: f64,
    raw_wpm: f64,
    accuracy: f64,
    consistency: f64,
    mode: String,
    mode2: String,
    language: String,
}

#[derive(Debug, Serialize)]
struct ActivityDay {
    day: i64,
    count: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModeSummary {
    mode: String,
    tests: u64,
    average_wpm: f64,
    best_wpm: f64,
    average_accuracy: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WrongWord {
    word: String,
    attempts: u64,
    error_attempts: u64,
    missed_attempts: u64,
    error_rate: f64,
    average_burst_wpm: f64,
    last_seen_ms: u64,
    variants: Vec<TypedVariant>,
}

#[derive(Debug, Serialize)]
struct TypedVariant {
    typed: String,
    count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SlowWord {
    word: String,
    attempts: u64,
    average_burst_wpm: f64,
}

#[derive(Debug, Serialize)]
struct Confusion {
    expected: String,
    typed: String,
    count: u64,
}

#[derive(Debug, Serialize)]
struct Insight {
    kind: &'static str,
    title: String,
    body: String,
    action: String,
}

#[derive(Default)]
struct ModeAccumulator {
    tests: u64,
    wpm: f64,
    best: f64,
    accuracy: f64,
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn rounded(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn timestamp(value: u128) -> u64 {
    value.min(u64::MAX as u128) as u64
}

pub fn build_data() -> Result<DashboardData> {
    // read the config per request so palette changes show up on refresh
    let config = crate::config::Config::load();
    let unit = config.typing_speed_unit;
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let mut history = crate::persistence::load_history();
    history.sort_by_key(|result| result.timestamp_ms);
    let profile = crate::persistence::compute_profile(now_ms);

    let wpms = history.iter().map(|result| result.wpm).collect::<Vec<_>>();
    let accuracies = history.iter().map(|result| result.acc).collect::<Vec<_>>();
    let consistencies = history
        .iter()
        .map(|result| result.consistency)
        .filter(|value| *value > 0.0)
        .collect::<Vec<_>>();
    let recent_wpm = mean(&wpms[wpms.len().saturating_sub(10)..]);
    let recent_accuracy = mean(&accuracies[accuracies.len().saturating_sub(10)..]);
    let previous_end = wpms.len().saturating_sub(10);
    let previous_start = previous_end.saturating_sub(10);
    let previous_wpm = mean(&wpms[previous_start..previous_end]);
    let trend_percent =
        (previous_wpm > 0.0).then(|| rounded((recent_wpm - previous_wpm) / previous_wpm * 100.0));
    let speed_leak = mean(
        &history
            .iter()
            .map(|result| (result.raw_wpm - result.wpm).max(0.0))
            .collect::<Vec<_>>(),
    );

    let timeline = history.iter().map(timeline_point).collect::<Vec<_>>();
    let activity = activity(&history);
    let modes = mode_summaries(&history);
    let connection = crate::analytics::open_connection()?;
    let wrong_words = wrong_words(&connection)?;
    let slow_words = slow_words(&connection)?;
    let confusions = confusions(&connection)?;
    let mistake_events = wrong_words.iter().map(|word| word.error_attempts).sum();

    let mut overview = Overview {
        tests_completed: profile.completed,
        tests_started: profile.started,
        time_typing_sec: profile.time_typing_sec,
        estimated_words: profile.estimated_words,
        highest_wpm: profile.highest_wpm,
        average_wpm: profile.avg_wpm,
        recent_wpm,
        previous_wpm,
        trend_percent,
        average_accuracy: profile.avg_acc,
        recent_accuracy,
        average_consistency: mean(&consistencies),
        speed_leak,
        current_streak: profile.current_streak,
        longest_streak: profile.max_streak,
        mistake_events,
        weak_word_count: wrong_words.len() as u64,
    };
    normalize_overview(&mut overview);
    // insight thresholds run on raw WPM values; only their display text (and
    // the payload conversion below) uses the configured unit
    let insights = insights(&overview, &wrong_words, &slow_words, &history, unit);

    let mut data = DashboardData {
        generated_at_ms: timestamp(now_ms),
        speed_unit: unit.as_str().to_string(),
        start_graphs_at_zero: config.start_graphs_at_zero,
        overview,
        timeline,
        activity,
        modes,
        wrong_words,
        slow_words,
        confusions,
        insights,
    };
    convert_speeds(&mut data, unit);
    Ok(data)
}

/// Convert every WPM-based value in the payload to the configured display
/// unit, so the browser dashboard shows the same numbers as the TUI.
fn convert_speeds(data: &mut DashboardData, unit: TypingSpeedUnit) {
    if unit == TypingSpeedUnit::Wpm {
        return;
    }
    let convert = |value: f64| rounded(unit.convert_from_wpm(value));
    let overview = &mut data.overview;
    overview.highest_wpm = convert(overview.highest_wpm);
    overview.average_wpm = convert(overview.average_wpm);
    overview.recent_wpm = convert(overview.recent_wpm);
    overview.previous_wpm = convert(overview.previous_wpm);
    overview.speed_leak = convert(overview.speed_leak);
    for point in &mut data.timeline {
        point.wpm = convert(point.wpm);
        point.raw_wpm = convert(point.raw_wpm);
    }
    for mode in &mut data.modes {
        mode.average_wpm = convert(mode.average_wpm);
        mode.best_wpm = convert(mode.best_wpm);
    }
    for word in &mut data.wrong_words {
        word.average_burst_wpm = convert(word.average_burst_wpm);
    }
    for word in &mut data.slow_words {
        word.average_burst_wpm = convert(word.average_burst_wpm);
    }
}

/// A WPM value rendered in the configured unit, e.g. "250.0 cpm".
fn format_speed(wpm: f64, unit: TypingSpeedUnit) -> String {
    format!("{:.1} {}", unit.convert_from_wpm(wpm), unit.as_str())
}

fn normalize_overview(overview: &mut Overview) {
    overview.highest_wpm = rounded(overview.highest_wpm);
    overview.average_wpm = rounded(overview.average_wpm);
    overview.recent_wpm = rounded(overview.recent_wpm);
    overview.previous_wpm = rounded(overview.previous_wpm);
    overview.average_accuracy = rounded(overview.average_accuracy);
    overview.recent_accuracy = rounded(overview.recent_accuracy);
    overview.average_consistency = rounded(overview.average_consistency);
    overview.speed_leak = rounded(overview.speed_leak);
}

fn timeline_point(result: &StoredResult) -> TimelinePoint {
    TimelinePoint {
        timestamp_ms: timestamp(result.timestamp_ms),
        wpm: result.wpm,
        raw_wpm: result.raw_wpm,
        accuracy: result.acc,
        consistency: result.consistency,
        mode: result.mode.clone(),
        mode2: result.mode2.clone(),
        language: result.language.clone(),
    }
}

fn activity(history: &[StoredResult]) -> Vec<ActivityDay> {
    let mut days = BTreeMap::<i64, u32>::new();
    for result in history {
        *days
            .entry((result.timestamp_ms / MS_PER_DAY) as i64)
            .or_default() += 1;
    }
    days.into_iter()
        .map(|(day, count)| ActivityDay { day, count })
        .collect()
}

fn mode_summaries(history: &[StoredResult]) -> Vec<ModeSummary> {
    let mut modes = BTreeMap::<String, ModeAccumulator>::new();
    for result in history {
        let key = if matches!(result.mode.as_str(), "time" | "words") {
            format!("{} {}", result.mode, result.mode2)
        } else {
            result.mode.clone()
        };
        let entry = modes.entry(key).or_default();
        entry.tests += 1;
        entry.wpm += result.wpm;
        entry.best = entry.best.max(result.wpm);
        entry.accuracy += result.acc;
    }
    let mut summaries = modes
        .into_iter()
        .map(|(mode, value)| ModeSummary {
            mode,
            tests: value.tests,
            average_wpm: rounded(value.wpm / value.tests as f64),
            best_wpm: rounded(value.best),
            average_accuracy: rounded(value.accuracy / value.tests as f64),
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|a, b| b.tests.cmp(&a.tests));
    summaries
}

fn wrong_words(connection: &Connection) -> Result<Vec<WrongWord>> {
    let mut statement = connection.prepare(
        r#"SELECT
            w.target_word,
            COUNT(*),
            SUM(w.had_error),
            SUM(CASE WHEN w.correct = 0 THEN 1 ELSE 0 END),
            COALESCE(AVG(CASE WHEN w.burst_wpm > 0 THEN w.burst_wpm END), 0),
            MAX(t.occurred_at_ms)
        FROM word_events w
        JOIN test_sessions t ON t.id = w.test_id
        WHERE w.target_word <> ''
        GROUP BY w.target_word
        HAVING SUM(w.had_error) > 0
        ORDER BY (SUM(CASE WHEN w.correct = 0 THEN 3 ELSE w.had_error END) * 1.0 / COUNT(*)) DESC,
                 SUM(w.had_error) DESC,
                 MAX(t.occurred_at_ms) DESC
        LIMIT 100"#,
    )?;
    let rows = statement.query_map([], |row| {
        let attempts = row.get::<_, i64>(1)?.max(0) as u64;
        let errors = row.get::<_, i64>(2)?.max(0) as u64;
        Ok(WrongWord {
            word: row.get(0)?,
            attempts,
            error_attempts: errors,
            missed_attempts: row.get::<_, i64>(3)?.max(0) as u64,
            error_rate: if attempts == 0 {
                0.0
            } else {
                rounded(errors as f64 / attempts as f64 * 100.0)
            },
            average_burst_wpm: rounded(row.get(4)?),
            last_seen_ms: row.get::<_, i64>(5)?.max(0) as u64,
            variants: Vec::new(),
        })
    })?;
    let mut words = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    drop(statement);
    for word in &mut words {
        let mut variants = connection.prepare(
            r#"SELECT typed_word, COUNT(*) AS uses
               FROM word_events
               WHERE target_word = ?1 AND (correct = 0 OR had_error = 1)
               GROUP BY typed_word
               ORDER BY uses DESC
               LIMIT 3"#,
        )?;
        word.variants = variants
            .query_map(params![word.word], |row| {
                Ok(TypedVariant {
                    typed: row.get(0)?,
                    count: row.get::<_, i64>(1)?.max(0) as u64,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
    }
    Ok(words)
}

fn slow_words(connection: &Connection) -> Result<Vec<SlowWord>> {
    let mut statement = connection.prepare(
        r#"SELECT target_word, COUNT(*), AVG(burst_wpm)
           FROM word_events
           WHERE correct = 1 AND burst_wpm > 0 AND target_word <> ''
           GROUP BY target_word
           ORDER BY AVG(burst_wpm) ASC, COUNT(*) DESC
           LIMIT 40"#,
    )?;
    let words = statement
        .query_map([], |row| {
            Ok(SlowWord {
                word: row.get(0)?,
                attempts: row.get::<_, i64>(1)?.max(0) as u64,
                average_burst_wpm: rounded(row.get(2)?),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(words)
}

fn confusions(connection: &Connection) -> Result<Vec<Confusion>> {
    let mut statement = connection
        .prepare("SELECT target_word, typed_word FROM word_events WHERE had_error = 1")?;
    let pairs = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut counts = HashMap::<(String, String), u64>::new();
    for (target, typed) in pairs {
        let expected = target.chars().collect::<Vec<_>>();
        let actual = typed.chars().collect::<Vec<_>>();
        for index in 0..expected.len().max(actual.len()) {
            let left = expected.get(index).copied();
            let right = actual.get(index).copied();
            if left != right {
                let key = (
                    left.map(|value| value.to_string())
                        .unwrap_or_else(|| "extra".to_string()),
                    right
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "missed".to_string()),
                );
                *counts.entry(key).or_default() += 1;
            }
        }
    }
    let mut values = counts
        .into_iter()
        .map(|((expected, typed), count)| Confusion {
            expected,
            typed,
            count,
        })
        .collect::<Vec<_>>();
    values.sort_by(|a, b| b.count.cmp(&a.count));
    values.truncate(24);
    Ok(values)
}

fn regression_slope(history: &[StoredResult]) -> f64 {
    let sample = &history[history.len().saturating_sub(20)..];
    if sample.len() < 4 {
        return 0.0;
    }
    let x_mean = (sample.len() - 1) as f64 / 2.0;
    let y_mean = sample.iter().map(|result| result.wpm).sum::<f64>() / sample.len() as f64;
    let numerator = sample
        .iter()
        .enumerate()
        .map(|(index, result)| (index as f64 - x_mean) * (result.wpm - y_mean))
        .sum::<f64>();
    let denominator = (0..sample.len())
        .map(|index| (index as f64 - x_mean).powi(2))
        .sum::<f64>();
    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

fn insights(
    overview: &Overview,
    wrong_words: &[WrongWord],
    slow_words: &[SlowWord],
    history: &[StoredResult],
    unit: TypingSpeedUnit,
) -> Vec<Insight> {
    if history.is_empty() {
        return vec![Insight {
            kind: "start",
            title: "Your baseline starts with one test".to_string(),
            body: "Complete a saved test and this dashboard will begin separating speed, accuracy, consistency, and word-level friction.".to_string(),
            action: "Run mtype and complete a test.".to_string(),
        }];
    }

    let mut values = Vec::new();
    if let Some(trend) = overview.trend_percent {
        if trend >= 2.0 {
            values.push(Insight {
                kind: "gain",
                title: format!("Recent speed is up {:.1}%", trend),
                body: format!("Your last 10 tests average {} versus {} before that. The gain is large enough to treat as a real direction, not one lucky test.", format_speed(overview.recent_wpm, unit), format_speed(overview.previous_wpm, unit)),
                action: "Keep the same test mix for another 10 tests to confirm the gain.".to_string(),
            });
        } else if trend <= -2.0 {
            values.push(Insight {
                kind: "watch",
                title: format!("Recent speed is down {:.1}%", trend.abs()),
                body: "A short decline often reflects fatigue, a harder language set, or accuracy work. Compare the mode breakdown before changing technique.".to_string(),
                action: "Use two short accuracy-first sessions before chasing speed.".to_string(),
            });
        }
    }

    if overview.recent_accuracy < 96.0 && history.len() >= 3 {
        values.push(Insight {
            kind: "accuracy",
            title: "Accuracy is limiting usable speed".to_string(),
            body: format!("Recent accuracy is {:.1}%. Raw speed is {} above net speed on average, so effort is being lost to corrections and invalid words.", overview.recent_accuracy, format_speed(overview.speed_leak, unit)),
            action: "Slow down slightly until accuracy stays above 97% for five tests.".to_string(),
        });
    } else if overview.speed_leak >= 6.0 {
        values.push(Insight {
            kind: "efficiency",
            title: "Raw speed is not converting cleanly".to_string(),
            body: format!("You lose {} between raw and scored speed. This usually points to bursts followed by corrections rather than an absolute speed limit.", format_speed(overview.speed_leak, unit)),
            action: "Practice an even cadence and finish each word before accelerating.".to_string(),
        });
    }

    if let Some(word) = wrong_words.first() {
        let variant = word
            .variants
            .first()
            .map(|value| format!(" Most often entered as '{}'.", value.typed))
            .unwrap_or_default();
        values.push(Insight {
            kind: "word",
            title: format!("'{}' is your highest-friction word", word.word),
            body: format!(
                "It produced errors in {:.1}% of {} attempts.{}",
                word.error_rate, word.attempts, variant
            ),
            action: "Run mtype practice missed --words 25.".to_string(),
        });
    } else if let Some(word) = slow_words.first() {
        values.push(Insight {
            kind: "word",
            title: format!("'{}' is slowing your rhythm", word.word),
            body: format!(
                "Its average burst is {} across {} clean attempts.",
                format_speed(word.average_burst_wpm, unit),
                word.attempts
            ),
            action: "Run mtype practice slow --words 25.".to_string(),
        });
    }

    let slope = regression_slope(history);
    if history.len() >= 10 && slope.abs() < 0.12 {
        values.push(Insight {
            kind: "plateau",
            title: "Your recent pace is stable".to_string(),
            body: "The last 20-test trend is nearly flat. Stable technique is useful, but another speed gain now needs a more specific stimulus.".to_string(),
            action: "Alternate missed-word practice with one short speed session.".to_string(),
        });
    }

    if values.is_empty() {
        values.push(Insight {
            kind: "steady",
            title: "Your foundation is balanced".to_string(),
            body: "Speed and accuracy are moving without a dominant weakness. More saved tests will make the next constraint easier to identify.".to_string(),
            action: "Keep your test settings consistent for the next five sessions.".to_string(),
        });
    }
    values.truncate(4);
    values
}

pub fn serve(port: u16, open_browser: bool) -> Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("could not bind dashboard to 127.0.0.1:{port}"))?;
    let address = listener.local_addr()?;
    let url = format!("http://{address}");
    println!("mtype dashboard: {url}");
    println!("press ctrl+c to stop");
    if open_browser {
        open_url(&url);
    }
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_connection(stream) {
                    eprintln!("dashboard request failed: {error}");
                }
            }
            Err(error) => eprintln!("dashboard connection failed: {error}"),
        }
    }
    Ok(())
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let result = Command::new("xdg-open").arg(url).spawn();
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    let result: std::io::Result<std::process::Child> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "automatic browser launch is unsupported",
    ));
    if result.is_err() {
        eprintln!("open {url} in your browser");
    }
}

fn handle_connection(mut stream: TcpStream) -> Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    let mut parts = request.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/").split('?').next().unwrap_or("/");
    if method != "GET" {
        return respond(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            b"GET only",
        );
    }
    let (status, content_type, body) = route(path);
    respond(&mut stream, status, content_type, &body)
}

fn route(path: &str) -> (&'static str, &'static str, Vec<u8>) {
    match path {
        "/" | "/index.html" => (
            "200 OK",
            "text/html; charset=utf-8",
            HTML.as_bytes().to_vec(),
        ),
        "/dashboard.css" => ("200 OK", "text/css; charset=utf-8", CSS.as_bytes().to_vec()),
        "/dashboard.js" => (
            "200 OK",
            "text/javascript; charset=utf-8",
            JS.as_bytes().to_vec(),
        ),
        "/api/health" => ("200 OK", "application/json", br#"{"ok":true}"#.to_vec()),
        "/api/dashboard" => {
            match build_data().and_then(|data| serde_json::to_vec(&data).map_err(Into::into)) {
                Ok(body) => ("200 OK", "application/json", body),
                Err(error) => (
                    "500 Internal Server Error",
                    "application/json",
                    serde_json::json!({ "error": error.to_string() })
                        .to_string()
                        .into_bytes(),
                ),
            }
        }
        _ => (
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"not found".to_vec(),
        ),
    }
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_routes_have_expected_types() {
        assert_eq!(route("/").0, "200 OK");
        assert_eq!(route("/dashboard.css").1, "text/css; charset=utf-8");
        assert_eq!(route("/missing").0, "404 Not Found");
    }

    #[test]
    fn api_contract_uses_browser_friendly_names() {
        let json = serde_json::to_value(Overview::default()).unwrap();
        assert!(json.get("testsCompleted").is_some());
        assert!(json.get("averageAccuracy").is_some());
        assert!(json.get("speedLeak").is_some());
    }

    fn sample_data() -> DashboardData {
        DashboardData {
            generated_at_ms: 0,
            speed_unit: "cpm".to_string(),
            start_graphs_at_zero: true,
            overview: Overview {
                highest_wpm: 100.0,
                average_wpm: 80.0,
                recent_wpm: 90.0,
                previous_wpm: 70.0,
                speed_leak: 4.0,
                average_accuracy: 97.0,
                ..Overview::default()
            },
            timeline: vec![TimelinePoint {
                timestamp_ms: 1,
                wpm: 60.0,
                raw_wpm: 66.0,
                accuracy: 97.0,
                consistency: 70.0,
                mode: "time".to_string(),
                mode2: "30".to_string(),
                language: "english".to_string(),
            }],
            activity: Vec::new(),
            modes: vec![ModeSummary {
                mode: "time 30".to_string(),
                tests: 1,
                average_wpm: 60.0,
                best_wpm: 60.0,
                average_accuracy: 97.0,
            }],
            wrong_words: vec![WrongWord {
                word: "the".to_string(),
                attempts: 4,
                error_attempts: 2,
                missed_attempts: 1,
                error_rate: 50.0,
                average_burst_wpm: 40.0,
                last_seen_ms: 1,
                variants: Vec::new(),
            }],
            slow_words: vec![SlowWord {
                word: "queue".to_string(),
                attempts: 2,
                average_burst_wpm: 50.0,
            }],
            confusions: Vec::new(),
            insights: Vec::new(),
        }
    }

    #[test]
    fn speeds_convert_to_the_configured_unit() {
        let mut data = sample_data();
        convert_speeds(&mut data, TypingSpeedUnit::Cpm);
        assert_eq!(data.overview.recent_wpm, 450.0);
        assert_eq!(data.overview.highest_wpm, 500.0);
        assert_eq!(data.overview.speed_leak, 20.0);
        assert_eq!(data.timeline[0].wpm, 300.0);
        assert_eq!(data.timeline[0].raw_wpm, 330.0);
        assert_eq!(data.timeline[0].accuracy, 97.0, "accuracy is not a speed");
        assert_eq!(data.modes[0].best_wpm, 300.0);
        assert_eq!(data.wrong_words[0].average_burst_wpm, 200.0);
        assert_eq!(data.slow_words[0].average_burst_wpm, 250.0);
    }

    #[test]
    fn wpm_unit_leaves_values_untouched() {
        let mut data = sample_data();
        convert_speeds(&mut data, TypingSpeedUnit::Wpm);
        assert_eq!(data.overview.recent_wpm, 90.0);
        assert_eq!(data.timeline[0].wpm, 60.0);
    }

    #[test]
    fn payload_carries_unit_and_graph_baseline_settings() {
        let json = serde_json::to_value(sample_data()).unwrap();
        assert_eq!(json["speedUnit"], "cpm");
        assert_eq!(json["startGraphsAtZero"], true);
    }

    #[test]
    fn insight_speeds_render_in_the_configured_unit() {
        assert_eq!(format_speed(50.0, TypingSpeedUnit::Cpm), "250.0 cpm");
        assert_eq!(format_speed(60.0, TypingSpeedUnit::Wps), "1.0 wps");
        assert_eq!(format_speed(72.5, TypingSpeedUnit::Wpm), "72.5 wpm");
    }
}
