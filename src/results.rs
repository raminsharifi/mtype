//! Results screen: headline WPM/acc, a WPM-over-time chart, secondary stats,
//! and the local personal-best banner.

use crate::app::{App, ResultsView};
use crate::config::TypingSpeedUnit;
use crate::engine::{InputEventKind, TestResult};
use crate::stats::{fmt_accuracy, fmt_num, unit_label};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph, Wrap};
use ratatui::Frame;

pub fn render_results(app: &App, frame: &mut Frame, area: Rect) {
    let t = &app.theme;
    let Some(r) = &app.result else { return };
    match app.results_view {
        ResultsView::InputHistory => {
            render_input_history(app, frame, area, r);
            return;
        }
        ResultsView::Replay => {
            render_replay(app, frame, area, r);
            return;
        }
        ResultsView::Summary => {}
    }

    if area.width < 24 || area.height < 8 {
        // too small for the full layout - just the headline numbers
        let unit = app.config.typing_speed_unit;
        let dec = app.config.always_show_decimal_places;
        frame.render_widget(
            Paragraph::new(format!(
                "{} {}  {}% acc",
                fmt_speed(r.wpm, unit, dec),
                unit_label(unit),
                fmt_accuracy(r.acc, dec)
            ))
            .alignment(Alignment::Center),
            area,
        );
        return;
    }

    // proportional to the terminal: ~80% width, ~85% height (so the chart grows)
    let max_width = area.width.saturating_sub(2);
    let width = ((area.width as u32 * 4 / 5) as u16).clamp(max_width.min(40), max_width);
    let max_height = area.height.saturating_sub(1);
    let height = ((area.height as u32 * 17 / 20) as u16).clamp(max_height.min(13), max_height);
    let region = crate::ui::center_rect(area, width, height);

    let chunks = Layout::vertical([
        Constraint::Length(if r.failed { 2 } else { 1 }), // status / spacer
        Constraint::Length(3),                            // headline
        Constraint::Min(6),                               // chart
        Constraint::Length(7),                            // stats
        Constraint::Length(1),                            // hint
    ])
    .split(region);

    render_status(app, frame, chunks[0], r);
    render_headline(app, frame, chunks[1], r);
    render_chart(app, frame, chunks[2], r);
    render_stats(app, frame, chunks[3], r);

    frame.render_widget(
        Paragraph::new(Span::styled(
            "tab next  s stats  i input  w replay  m missed  l slow  esc menu  q quit",
            Style::default().fg(t.sub),
        ))
        .alignment(Alignment::Center),
        chunks[4],
    );
}

fn render_input_history(app: &App, frame: &mut Frame, area: Rect, r: &TestResult) {
    let unit = app.config.typing_speed_unit;
    let dec = app.config.always_show_decimal_places;
    let width = area.width.saturating_sub(4).min(100);
    let height = area.height.saturating_sub(2);
    let region = crate::ui::center_rect(area, width, height);
    let mut lines = vec![Line::from(vec![
        Span::styled(
            "input history",
            Style::default()
                .fg(app.theme.main)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "    corrected errors are retained",
            Style::default().fg(app.theme.sub),
        ),
    ])];
    let available = region.height.saturating_sub(2) as usize;
    for word in r.word_outcomes.iter().take(available) {
        let marker = if word.had_error { "×" } else { "✓" };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{marker} {:>3}  ", word.word_index + 1),
                Style::default().fg(if word.had_error {
                    app.theme.error
                } else {
                    app.theme.sub
                }),
            ),
            Span::styled(
                format!("{:<18}", word.target),
                Style::default().fg(app.theme.text),
            ),
            Span::styled(
                format!(" typed {:<18}", word.typed),
                Style::default().fg(if word.correct {
                    app.theme.sub
                } else {
                    app.theme.error
                }),
            ),
            Span::styled(
                format!(
                    "  {}ms  {} {} burst  {} error keys",
                    word.duration_ms,
                    fmt_speed(word.burst_wpm, unit, dec),
                    unit_label(unit),
                    word.incorrect_keystrokes
                ),
                Style::default().fg(app.theme.sub),
            ),
        ]));
    }
    lines.push(Line::from(Span::styled(
        "esc - summary    m - practice missed    l - practice slow",
        Style::default().fg(app.theme.sub),
    )));
    frame.render_widget(Paragraph::new(lines), region);
}

/// Rebuild the per-word typed strings from the input log up to `elapsed` ms.
/// Only applied events mutate the text: inputs the engine blocked (stop on
/// error: letter, confidence max backspaces, ...) were logged for analytics
/// but never appeared in the test, so the replay must skip them too.
fn replay_typed_words(r: &TestResult, elapsed: u64) -> Vec<String> {
    let word_count = r
        .word_outcomes
        .iter()
        .map(|word| word.word_index + 1)
        .max()
        .unwrap_or(1);
    let mut typed = vec![String::new(); word_count];
    for event in r
        .input_events
        .iter()
        .take_while(|event| event.elapsed_ms <= elapsed)
        .filter(|event| event.applied)
    {
        if event.word_index >= typed.len() {
            typed.resize(event.word_index + 1, String::new());
        }
        match event.kind {
            InputEventKind::Character => {
                if let Some(value) = &event.value {
                    typed[event.word_index].push_str(value);
                }
            }
            InputEventKind::Backspace => {
                typed[event.word_index].pop();
            }
            InputEventKind::WordBackspace => typed[event.word_index].clear(),
            InputEventKind::Commit => {}
        }
    }
    typed
}

fn render_replay(app: &App, frame: &mut Frame, area: Rect, r: &TestResult) {
    let elapsed = app.replay_elapsed_ms();
    let typed = replay_typed_words(r, elapsed);
    let targets = r
        .word_outcomes
        .iter()
        .map(|word| word.target.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let replayed = typed.join(" ");
    let last_event = r
        .input_events
        .last()
        .map(|event| event.elapsed_ms)
        .unwrap_or(0);
    let status = if elapsed >= last_event {
        "complete"
    } else {
        "playing"
    };
    let region = crate::ui::center_rect(
        area,
        area.width.saturating_sub(6).min(100),
        area.height.saturating_sub(4).min(14),
    );
    let lines = vec![
        Line::from(vec![
            Span::styled(
                "replay",
                Style::default()
                    .fg(app.theme.main)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(
                    "    {status}  {:.1}s / {:.1}s",
                    elapsed as f64 / 1000.0,
                    last_event as f64 / 1000.0
                ),
                Style::default().fg(app.theme.sub),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("target", Style::default().fg(app.theme.sub))),
        Line::from(Span::styled(targets, Style::default().fg(app.theme.text))),
        Line::from(""),
        Line::from(Span::styled("typed", Style::default().fg(app.theme.sub))),
        Line::from(Span::styled(replayed, Style::default().fg(app.theme.main))),
        Line::from(""),
        Line::from(Span::styled(
            "w - restart replay    esc - summary",
            Style::default().fg(app.theme.sub),
        )),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), region);
}

fn render_status(app: &App, frame: &mut Frame, area: Rect, r: &TestResult) {
    let t = &app.theme;
    let mut spans: Vec<Span> = Vec::new();
    if r.failed {
        spans.push(Span::styled(
            format!(
                "test failed{}",
                r.fail_reason
                    .as_ref()
                    .map(|s| format!(" - {s}"))
                    .unwrap_or_default()
            ),
            Style::default().fg(t.error).add_modifier(Modifier::BOLD),
        ));
    } else if app.pb_info.is_pb {
        // previous best is stored in raw wpm; show it in the same unit as the
        // converted headline right below it
        let unit = app.config.typing_speed_unit;
        let dec = app.config.always_show_decimal_places;
        let prev = app
            .pb_info
            .previous_best
            .map(|b| format!(" (prev {})", fmt_speed(b, unit, dec)))
            .unwrap_or_default();
        spans.push(Span::styled(
            format!("🏆 new personal best!{prev}"),
            Style::default().fg(t.main).add_modifier(Modifier::BOLD),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).alignment(Alignment::Center),
        area,
    );
}

fn render_headline(app: &App, frame: &mut Frame, area: Rect, r: &TestResult) {
    let t = &app.theme;
    let unit = app.config.typing_speed_unit;
    let dec = app.config.always_show_decimal_places;
    let lines = vec![Line::from(vec![
        Span::styled(
            format!("{} ", fmt_speed(r.wpm, unit, dec)),
            Style::default().fg(t.main).add_modifier(Modifier::BOLD),
        ),
        Span::styled(unit_label(unit), Style::default().fg(t.sub)),
        Span::styled(
            format!("        {}% ", fmt_accuracy(r.acc, dec)),
            Style::default().fg(t.main).add_modifier(Modifier::BOLD),
        ),
        Span::styled("acc", Style::default().fg(t.sub)),
    ])];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

fn render_chart(app: &App, frame: &mut Frame, area: Rect, r: &TestResult) {
    let t = &app.theme;
    if r.wpm_history.len() < 2 || area.height < 4 {
        return;
    }
    // histories are stored in raw wpm; plot them in the configured unit so the
    // chart agrees with the converted headline above it (and the stats graph)
    let unit = app.config.typing_speed_unit;
    let wpm_pts: Vec<(f64, f64)> = r
        .wpm_history
        .iter()
        .enumerate()
        .map(|(i, &w)| ((i + 1) as f64, unit.convert_from_wpm(w)))
        .collect();
    let raw_pts: Vec<(f64, f64)> = r
        .raw_history
        .iter()
        .enumerate()
        .map(|(i, &w)| ((i + 1) as f64, unit.convert_from_wpm(w)))
        .collect();

    let n = r.wpm_history.len() as f64;
    let all_values = raw_pts
        .iter()
        .chain(wpm_pts.iter())
        .map(|(_, value)| *value)
        .collect::<Vec<_>>();
    let mut ymax = all_values.iter().copied().fold(0.0_f64, f64::max);
    let ymin = if app.config.start_graphs_at_zero {
        0.0
    } else {
        all_values
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min)
            .mul_add(0.9, 0.0)
            .max(0.0)
    };
    ymax = ymax.max(ymin + 10.0);
    let ymax = (ymax * 1.1).ceil().max(10.0);

    let datasets = vec![
        Dataset::default()
            .name("raw")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(t.sub))
            .data(&raw_pts),
        Dataset::default()
            .name(unit_label(unit))
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(t.main))
            .data(&wpm_pts),
    ];

    let chart = Chart::new(datasets)
        .x_axis(
            Axis::default()
                .style(Style::default().fg(t.sub))
                .bounds([1.0, n.max(2.0)])
                .labels([
                    Span::raw("0s"),
                    Span::raw(format!("{}s", r.wpm_history.len())),
                ]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(t.sub))
                .bounds([ymin, ymax])
                .labels([
                    Span::raw(format!("{}", ymin.round() as i64)),
                    Span::raw(format!("{}", ymax.round() as i64)),
                ]),
        );
    frame.render_widget(chart, area);
}

fn render_stats(app: &App, frame: &mut Frame, area: Rect, r: &TestResult) {
    let t = &app.theme;
    let unit = app.config.typing_speed_unit;
    let dec = app.config.always_show_decimal_places;
    let mut lines: Vec<Line> = Vec::new();
    for (label, value) in [
        ("raw".to_string(), fmt_speed(r.raw_wpm, unit, dec)),
        (
            "consistency".to_string(),
            format!("{}%", fmt_num(r.consistency, dec)),
        ),
        (
            "characters".to_string(),
            format!(
                "{}/{}/{}/{}",
                r.char_correct, r.char_incorrect, r.char_extra, r.char_missed
            ),
        ),
        (
            "time".to_string(),
            format!("{}s", fmt_num(r.duration_sec, dec)),
        ),
        ("test".to_string(), test_descriptor(r)),
    ] {
        lines.push(Line::from(vec![
            Span::styled(format!("{label:>12}  "), Style::default().fg(t.sub)),
            Span::styled(value, Style::default().fg(t.text)),
        ]));
    }
    if let Some(src) = &r.quote_source {
        lines.push(Line::from(vec![
            Span::styled("      source  ".to_string(), Style::default().fg(t.sub)),
            Span::styled(src.clone(), Style::default().fg(t.text)),
        ]));
    }
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Left), area);
}

fn test_descriptor(r: &TestResult) -> String {
    use crate::config::Mode;
    let base = match r.mode {
        Mode::Time => format!("time {}", r.mode2),
        Mode::Words => format!("words {}", r.mode2),
        Mode::Quote => "quote".to_string(),
        Mode::Zen => "zen".to_string(),
        Mode::Custom => "custom".to_string(),
        Mode::Practice => format!("practice {}", r.mode2),
    };
    let mut extra = Vec::new();
    if r.punctuation {
        extra.push("punctuation");
    }
    if r.numbers {
        extra.push("numbers");
    }
    if extra.is_empty() {
        base
    } else {
        format!("{base}  {}", extra.join(" "))
    }
}

/// A raw-wpm value in the configured unit, honoring the decimals toggle.
fn fmt_speed(wpm: f64, unit: TypingSpeedUnit, force_dec: bool) -> String {
    fmt_num(unit.convert_from_wpm(wpm), force_dec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::config::{Config, Mode};
    use crate::engine::{InputEvent, WordOutcome};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn mk_result() -> TestResult {
        TestResult {
            wpm: 60.0,
            raw_wpm: 70.0,
            acc: 96.43,
            consistency: 70.55,
            char_correct: 30,
            char_incorrect: 1,
            char_extra: 0,
            char_missed: 0,
            char_total: 31,
            duration_sec: 15.0,
            mode: Mode::Time,
            mode2: "15".to_string(),
            punctuation: false,
            numbers: false,
            language: "english".to_string(),
            wpm_history: vec![60.0, 60.0],
            raw_history: vec![70.0, 70.0],
            failed: false,
            fail_reason: None,
            quote_source: None,
            word_outcomes: vec![WordOutcome {
                word_index: 0,
                target: "hello".to_string(),
                typed: "hello".to_string(),
                preceding_word: None,
                duration_ms: 600,
                burst_wpm: 100.0,
                correct: true,
                had_error: false,
                incorrect_keystrokes: 0,
                char_correct: 5,
                char_incorrect: 0,
                char_extra: 0,
                char_missed: 0,
            }],
            input_events: vec![],
        }
    }

    fn event(kind: InputEventKind, value: Option<&str>, applied: bool, ms: u64) -> InputEvent {
        InputEvent {
            elapsed_ms: ms,
            word_index: 0,
            kind,
            value: value.map(str::to_string),
            correct: None,
            applied,
        }
    }

    fn draw(app: &App, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_results(app, frame, frame.area()))
            .unwrap();
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

    /// Regression: blocked inputs (stop on error: letter, confidence max) are
    /// in the event log for analytics but must not appear in the replay.
    #[test]
    fn replay_skips_blocked_events() {
        let mut r = mk_result();
        r.input_events = vec![
            event(InputEventKind::Character, Some("h"), true, 0),
            event(InputEventKind::Character, Some("x"), false, 100), // blocked letter
            event(InputEventKind::Backspace, None, false, 200),      // confidence max
            event(InputEventKind::Character, Some("i"), true, 300),
        ];
        assert_eq!(replay_typed_words(&r, u64::MAX), vec!["hi".to_string()]);
        // an applied backspace still deletes in the replay
        r.input_events[2].applied = true;
        assert_eq!(replay_typed_words(&r, u64::MAX), vec!["i".to_string()]);
    }

    #[test]
    fn results_screen_converts_speed_unit_everywhere() {
        let cfg = Config {
            typing_speed_unit: TypingSpeedUnit::Cpm,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        app.result = Some(mk_result());
        app.pb_info = crate::persistence::PbInfo {
            is_pb: true,
            previous_best: Some(50.0),
        };
        let text = draw(&app, 80, 24);
        assert!(text.contains("300 cpm"), "headline must convert:\n{text}");
        assert!(
            text.contains("(prev 250)"),
            "pb banner must convert the previous best:\n{text}"
        );
        assert!(text.contains("350"), "raw stat must convert:\n{text}");
        assert!(
            !text.contains("wpm"),
            "no hardcoded wpm label (chart legend included):\n{text}"
        );
    }

    #[test]
    fn results_decimals_toggle_governs_display() {
        let cfg = Config {
            always_show_decimal_places: false,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        let mut r = mk_result();
        r.wpm = 81.37;
        r.acc = 96.99;
        r.duration_sec = 15.55;
        app.result = Some(r);
        let text = draw(&app, 80, 24);
        assert!(text.contains("81 wpm"), "off rounds to integers:\n{text}");
        assert!(text.contains("96% "), "off rounds acc:\n{text}");
        assert!(text.contains("16s"), "off rounds duration:\n{text}");
        assert!(!text.contains("81.37"), "off shows no decimals:\n{text}");

        app.config.always_show_decimal_places = true;
        let text = draw(&app, 80, 24);
        assert!(text.contains("81.37 wpm"), "on forces decimals:\n{text}");
        assert!(text.contains("96.99%"), "on forces acc decimals:\n{text}");
        assert!(
            text.contains("15.55s"),
            "on forces duration decimals:\n{text}"
        );
        assert!(
            text.contains("70.55%"),
            "on forces consistency decimals:\n{text}"
        );
    }

    #[test]
    fn input_history_burst_uses_speed_unit() {
        let cfg = Config {
            typing_speed_unit: TypingSpeedUnit::Cpm,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        app.result = Some(mk_result());
        app.results_view = ResultsView::InputHistory;
        let text = draw(&app, 100, 24);
        assert!(
            text.contains("500 cpm burst"),
            "burst must convert and carry a unit label:\n{text}"
        );
    }

    #[test]
    fn tiny_results_fallback_converts_and_labels() {
        let cfg = Config {
            typing_speed_unit: TypingSpeedUnit::Cpm,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        app.result = Some(mk_result());
        let text = draw(&app, 20, 6); // too small for the full layout
        assert!(text.contains("300 cpm"), "fallback must convert:\n{text}");
        assert!(
            text.contains("96% acc"),
            "fallback honors decimals:\n{text}"
        );
    }
}
