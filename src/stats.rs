//! The stats / progress screen: a terminal version of Monkeytype's account
//! page, built entirely from the local results history. Shows lifetime stats,
//! a WPM-over-time graph, an activity heatmap, and streaks.

use crate::app::App;
use crate::config::TypingSpeedUnit;
use crate::persistence::{civil_from_days, Profile};
use crate::theme::Theme;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, Paragraph};
use ratatui::Frame;

pub fn render_stats(app: &App, frame: &mut Frame, area: Rect) {
    let t = &app.theme;
    let Some(p) = &app.profile else { return };

    if area.width < 30 || area.height < 10 {
        frame.render_widget(
            Paragraph::new("terminal too small for stats").alignment(Alignment::Center),
            area,
        );
        return;
    }

    let max_width = area.width.saturating_sub(2);
    let width = ((area.width as u32 * 9 / 10) as u16).clamp(max_width.min(40), max_width);
    let height = area.height.saturating_sub(1).min(40);
    let region = crate::ui::center_rect(area, width, height);

    if p.completed == 0 {
        let lines = vec![
            Line::from(Span::styled(
                "your stats",
                Style::default().fg(t.main).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "no results yet. complete a test and your progress shows up here.",
                Style::default().fg(t.sub),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "tab / esc - back    q - quit",
                Style::default().fg(t.sub),
            )),
        ];
        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Left), region);
        return;
    }

    // footer is pinned to the bottom so it never gets clipped on short terminals
    let footer = Rect::new(region.x, region.bottom().saturating_sub(1), region.width, 1);
    frame.render_widget(
        Paragraph::new(Span::styled(
            "tab / esc - back to test    q - quit",
            Style::default().fg(t.sub),
        ))
        .alignment(Alignment::Center),
        footer,
    );
    let content = Rect::new(
        region.x,
        region.y,
        region.width,
        region.height.saturating_sub(1),
    );

    // vertical sections; later sections drop off gracefully on short terminals
    let chunks = Layout::vertical([
        Constraint::Length(1),  // title
        Constraint::Length(7),  // summary stats
        Constraint::Min(7),     // wpm graph
        Constraint::Length(10), // activity heatmap
        Constraint::Length(7),  // recent tests
    ])
    .split(content);

    render_title(app, frame, chunks[0], p);
    render_summary(app, frame, chunks[1], p);
    render_graph(app, frame, chunks[2], p);
    render_activity(app, frame, chunks[3], p);
    render_recent(app, frame, chunks[4], p);
}

fn render_title(app: &App, frame: &mut Frame, area: Rect, p: &Profile) {
    let t = &app.theme;
    let line = Line::from(vec![
        Span::styled(
            "your stats",
            Style::default().fg(t.main).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "    {} completed   {} started   {} streak ({} max)",
                p.completed, p.started, p.current_streak, p.max_streak
            ),
            Style::default().fg(t.sub),
        ),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn render_summary(app: &App, frame: &mut Frame, area: Rect, p: &Profile) {
    let t = &app.theme;
    let unit = app.config.typing_speed_unit;
    let conv = |w: f64| unit.convert_from_wpm(w);

    let cols =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);

    let u = unit_label(unit);
    let left = vec![
        stat_line(t, "time typing", &secs_to_hms(p.time_typing_sec)),
        stat_line(t, "estimated words", &p.estimated_words.to_string()),
        stat_line(t, &format!("highest {u}"), &fmt(conv(p.highest_wpm))),
        stat_line(t, &format!("average {u}"), &fmt(conv(p.avg_wpm))),
        stat_line(
            t,
            &format!("avg {u} (last 10)"),
            &fmt(conv(p.avg_wpm_last10)),
        ),
        stat_line(t, &format!("highest raw {u}"), &fmt(conv(p.highest_raw))),
        stat_line(t, &format!("average raw {u}"), &fmt(conv(p.avg_raw))),
    ];
    let right = vec![
        stat_line(t, "highest acc", &format!("{}%", fmt(p.highest_acc))),
        stat_line(t, "average acc", &format!("{}%", fmt(p.avg_acc))),
        stat_line(
            t,
            "avg acc (last 10)",
            &format!("{}%", fmt(p.avg_acc_last10)),
        ),
        stat_line(
            t,
            "highest consistency",
            &format!("{}%", fmt(p.highest_consistency)),
        ),
        stat_line(
            t,
            "average consistency",
            &format!("{}%", fmt(p.avg_consistency)),
        ),
    ];

    frame.render_widget(Paragraph::new(left), cols[0]);
    frame.render_widget(Paragraph::new(right), cols[1]);
}

fn stat_line(t: &Theme, label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<22}"), Style::default().fg(t.sub)),
        Span::styled(
            value.to_string(),
            Style::default().fg(t.text).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn render_graph(app: &App, frame: &mut Frame, area: Rect, p: &Profile) {
    let t = &app.theme;
    if p.wpm_history.len() < 2 || area.height < 4 {
        // not enough data for a line; just say so
        frame.render_widget(
            Paragraph::new(Span::styled(
                "wpm over time appears once you have a couple of tests",
                Style::default().fg(t.sub),
            )),
            area,
        );
        return;
    }
    let unit = app.config.typing_speed_unit;
    let points: Vec<(f64, f64)> = p
        .wpm_history
        .iter()
        .enumerate()
        .map(|(i, &w)| ((i + 1) as f64, unit.convert_from_wpm(w)))
        .collect();
    let n = points.len() as f64;
    let ymax = points
        .iter()
        .map(|(_, y)| *y)
        .fold(0.0_f64, f64::max)
        .max(10.0)
        * 1.1;
    let ymin = if app.config.start_graphs_at_zero {
        0.0
    } else {
        points
            .iter()
            .map(|(_, value)| *value)
            .fold(f64::INFINITY, f64::min)
            .mul_add(0.9, 0.0)
            .max(0.0)
    };

    let datasets = vec![Dataset::default()
        .name(format!("{} over time", unit_label(unit)))
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(t.main))
        .data(&points)];

    let chart = Chart::new(datasets)
        .x_axis(
            Axis::default()
                .style(Style::default().fg(t.sub))
                .bounds([1.0, n])
                .labels([Span::raw("oldest"), Span::raw("latest")]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(t.sub))
                .bounds([ymin, ymax.max(ymin + 10.0)])
                .labels([
                    Span::raw(format!("{}", ymin.round() as i64)),
                    Span::raw(format!("{}", ymax.round() as i64)),
                ]),
        );
    frame.render_widget(chart, area);
}

fn render_activity(app: &App, frame: &mut Frame, area: Rect, p: &Profile) {
    let t = &app.theme;
    if area.height < 9 {
        return;
    }
    // map day -> count
    let mut counts: std::collections::HashMap<i64, u32> = std::collections::HashMap::new();
    for a in &p.activity {
        counts.insert(a.day, a.count);
    }

    // how many weeks fit in the available width (one column per week)
    let weeks = ((area.width as usize).saturating_sub(2)).clamp(6, 26);
    let today = p.today;
    let today_weekday = (today.rem_euclid(7) + 4).rem_euclid(7); // 0 = Sunday
    let this_sunday = today - today_weekday;

    // caption
    let caption = Line::from(vec![Span::styled(
        format!("activity (last {weeks} weeks)"),
        Style::default().fg(t.sub),
    )]);
    frame.render_widget(
        Paragraph::new(caption),
        Rect::new(area.x, area.y, area.width, 1),
    );

    // 7 rows (Sun..Sat), `weeks` columns
    let mut rows: Vec<Line> = Vec::with_capacity(7);
    for r in 0..7i64 {
        let mut spans: Vec<Span> = Vec::new();
        for c in 0..weeks as i64 {
            let week_sunday = this_sunday - (weeks as i64 - 1 - c) * 7;
            let day = week_sunday + r;
            if day > today {
                spans.push(Span::raw(" "));
                continue;
            }
            let count = counts.get(&day).copied().unwrap_or(0);
            let color = heat_color(t, count);
            spans.push(Span::styled("\u{25A0}", Style::default().fg(color)));
        }
        rows.push(Line::from(spans));
    }

    // Month labels line up with the first week column in each new month.
    let mut month_label = vec![' '; weeks];
    let mut last_month = 0u32;
    for c in 0..weeks as i64 {
        let week_sunday = this_sunday - (weeks as i64 - 1 - c) * 7;
        let (_, m, _) = civil_from_days(week_sunday);
        if m != last_month {
            last_month = m;
            for (offset, ch) in month_abbr(m).chars().enumerate() {
                if let Some(cell) = month_label.get_mut(c as usize + offset) {
                    *cell = ch;
                }
            }
        }
    }

    // At the full section height, reserve a row for month names. If Ratatui
    // shrinks the section, retain the heatmap and legend and omit the labels.
    let has_month_row = area.height >= 10;
    let grid_y = area.y + if has_month_row { 2 } else { 1 };
    if has_month_row {
        let label_rect = Rect::new(area.x, area.y + 1, area.width, 1).intersection(area);
        frame.render_widget(
            Paragraph::new(Span::styled(
                month_label.into_iter().collect::<String>(),
                Style::default().fg(t.sub),
            )),
            label_rect,
        );
    }

    let grid_rect = Rect::new(area.x, grid_y, area.width, 7).intersection(area);
    frame.render_widget(Paragraph::new(rows), grid_rect);

    // intensity legend
    let legend = Line::from(vec![
        Span::styled("less ", Style::default().fg(t.sub)),
        Span::styled("\u{25A0}", Style::default().fg(heat_color(t, 0))),
        Span::styled("\u{25A0}", Style::default().fg(heat_color(t, 1))),
        Span::styled("\u{25A0}", Style::default().fg(heat_color(t, 3))),
        Span::styled("\u{25A0}", Style::default().fg(heat_color(t, 5))),
        Span::styled("\u{25A0}", Style::default().fg(heat_color(t, 8))),
        Span::styled(" more", Style::default().fg(t.sub)),
    ]);
    let legend_rect = Rect::new(area.x, grid_y + 7, area.width, 1).intersection(area);
    frame.render_widget(Paragraph::new(legend), legend_rect);
}

fn render_recent(app: &App, frame: &mut Frame, area: Rect, p: &Profile) {
    let t = &app.theme;
    if area.height < 3 || p.recent.is_empty() {
        return;
    }
    let unit = app.config.typing_speed_unit;
    let u = unit_label(unit);
    let conv = |w: f64| unit.convert_from_wpm(w);

    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        format!("recent tests        {u:>5}  raw   acc   con   test            date",),
        Style::default().fg(t.sub),
    ))];

    let rows = (area.height as usize).saturating_sub(1).min(p.recent.len());
    for r in p.recent.iter().take(rows) {
        let (_, m, d) = civil_from_days((r.timestamp_ms / 86_400_000) as i64);
        let test = test_descriptor(r);
        lines.push(Line::from(Span::styled(
            format!(
                "                {:>6}  {:>4}  {:>3}%  {:>3}%  {:<14}  {:>2} {}",
                fmt(conv(r.wpm)),
                fmt(conv(r.raw_wpm)),
                fmt(r.acc),
                fmt(r.consistency),
                truncate(&test, 14),
                d,
                month_abbr(m),
            ),
            Style::default().fg(t.text),
        )));
    }
    frame.render_widget(Paragraph::new(lines), area);
}

fn test_descriptor(r: &crate::persistence::StoredResult) -> String {
    let mut s = match r.mode.as_str() {
        "time" => format!("time {}", r.mode2),
        "words" => format!("words {}", r.mode2),
        other => other.to_string(),
    };
    if r.punctuation {
        s.push_str(" !");
    }
    if r.numbers {
        s.push_str(" #");
    }
    s
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max.saturating_sub(1)).collect::<String>() + "\u{2026}"
    }
}

fn month_abbr(m: u32) -> &'static str {
    [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ]
    .get((m.saturating_sub(1)) as usize)
    .copied()
    .unwrap_or("")
}

/// Blend the empty-cell color toward the accent color by activity intensity.
fn heat_color(t: &Theme, count: u32) -> Color {
    if count == 0 {
        return t.sub_alt;
    }
    let tf = (count.min(8) as f32) / 8.0;
    blend(t.sub_alt, t.main, 0.25 + 0.75 * tf)
}

fn rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (128, 128, 128),
    }
}

fn blend(a: Color, b: Color, tf: f32) -> Color {
    let (ar, ag, ab) = rgb(a);
    let (br, bg, bb) = rgb(b);
    let lerp = |x: u8, y: u8| ((x as f32) + ((y as f32) - (x as f32)) * tf).round() as u8;
    Color::Rgb(lerp(ar, br), lerp(ag, bg), lerp(ab, bb))
}

fn secs_to_hms(secs: f64) -> String {
    let s = secs.round() as u64;
    let (h, m, sec) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 {
        format!("{h}h {m}m {sec}s")
    } else if m > 0 {
        format!("{m}m {sec}s")
    } else {
        format!("{sec}s")
    }
}

fn fmt(v: f64) -> String {
    if v.fract().abs() < 1e-9 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.2}")
    }
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
