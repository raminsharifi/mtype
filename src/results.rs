//! Results screen: headline WPM/acc, a WPM-over-time chart, secondary stats,
//! and the local personal-best banner.

use crate::app::App;
use crate::config::TypingSpeedUnit;
use crate::engine::TestResult;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph};
use ratatui::Frame;

pub fn render_results(app: &App, frame: &mut Frame, area: Rect) {
    let t = &app.theme;
    let Some(r) = &app.result else { return };

    if area.width < 24 || area.height < 8 {
        // too small for the full layout - just the headline numbers
        frame.render_widget(
            Paragraph::new(format!(
                "{} wpm  {}% acc",
                fmt_num(r.wpm, false),
                fmt_num(r.acc, false)
            ))
            .alignment(Alignment::Center),
            area,
        );
        return;
    }

    // proportional to the terminal: ~80% width, ~85% height (so the chart grows)
    let width = ((area.width as u32 * 4 / 5) as u16).clamp(40, area.width.saturating_sub(2));
    let height = ((area.height as u32 * 17 / 20) as u16).clamp(13, area.height.saturating_sub(1));
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
            "tab / enter - next test    esc - quit",
            Style::default().fg(t.sub),
        ))
        .alignment(Alignment::Center),
        chunks[4],
    );
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
        let prev = app
            .pb_info
            .previous_best
            .map(|b| format!(" (prev {})", fmt_num(b, false)))
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
            format!("        {}% ", fmt_num(r.acc, dec)),
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
    let wpm_pts: Vec<(f64, f64)> = r
        .wpm_history
        .iter()
        .enumerate()
        .map(|(i, &w)| ((i + 1) as f64, w))
        .collect();
    let raw_pts: Vec<(f64, f64)> = r
        .raw_history
        .iter()
        .enumerate()
        .map(|(i, &w)| ((i + 1) as f64, w))
        .collect();

    let n = r.wpm_history.len() as f64;
    let mut ymax = r
        .raw_history
        .iter()
        .chain(r.wpm_history.iter())
        .cloned()
        .fold(0.0_f64, f64::max);
    if app.config.start_graphs_at_zero {
        ymax = ymax.max(10.0);
    }
    let ymax = (ymax * 1.1).ceil().max(10.0);

    let datasets = vec![
        Dataset::default()
            .name("raw")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(t.sub))
            .data(&raw_pts),
        Dataset::default()
            .name("wpm")
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
                .bounds([0.0, ymax])
                .labels([
                    Span::raw("0"),
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
            format!("{}s", fmt_num(r.duration_sec, false)),
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

fn fmt_num(v: f64, force_dec: bool) -> String {
    if force_dec || v.fract().abs() > f64::EPSILON {
        format!("{v:.2}")
    } else {
        format!("{}", v.round() as i64)
    }
}

fn fmt_speed(wpm: f64, unit: TypingSpeedUnit, force_dec: bool) -> String {
    fmt_num(unit.convert_from_wpm(wpm), force_dec)
}

fn unit_label(unit: TypingSpeedUnit) -> &'static str {
    match unit {
        TypingSpeedUnit::Wpm => "wpm",
        TypingSpeedUnit::Cpm => "cpm",
        TypingSpeedUnit::Wps => "wps",
        TypingSpeedUnit::Cps => "cps",
        TypingSpeedUnit::Wph => "wph",
    }
}
