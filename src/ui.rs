//! Rendering. Dispatches to the test screen or the results screen.

use crate::app::{App, Screen};
use crate::config::{CaretStyle, HighlightMode, IndicateTypos, IndicatorStyle, Mode, SmoothCaret};
use crate::engine::Engine;
use crate::theme::Theme;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;
use unicode_width::UnicodeWidthStr;

pub fn render(app: &App, frame: &mut Frame) {
    let area = frame.area();
    // paint the themed background
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.bg)),
        area,
    );
    match app.screen {
        Screen::Test => render_test(app, frame, area),
        Screen::Results => crate::results::render_results(app, frame, area),
        Screen::Stats => crate::stats::render_stats(app, frame, area),
        Screen::Editor => render_editor(app, frame, area),
    }
    // command palette overlays everything when open
    if app.command_line.is_some() {
        crate::commandline::render(app, frame, area);
    }
    if app.screen == Screen::Test
        && app.command_line.is_none()
        && ((app.config.show_out_of_focus_warning && !app.focused)
            || (app.config.caps_lock_warning && app.caps_lock))
    {
        let message = if !app.focused {
            "click to focus"
        } else {
            "caps lock"
        };
        let rect = center_rect(area, (message.len() as u16 + 6).min(area.width), 3);
        frame.render_widget(
            Paragraph::new(Span::styled(
                message,
                Style::default()
                    .fg(app.theme.bg)
                    .bg(app.theme.main)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Center),
            rect,
        );
    }
}

fn render_editor(app: &App, frame: &mut Frame, area: Rect) {
    let region = center_rect(
        area,
        area.width.saturating_sub(6).min(100),
        area.height.saturating_sub(4).min(24),
    );
    let lines = vec![
        Line::from(Span::styled(
            "custom text editor",
            Style::default()
                .fg(app.theme.main)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "type or paste text below    ctrl+s save & start    esc cancel",
            Style::default().fg(app.theme.sub),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(app.editor_text.clone(), Style::default().fg(app.theme.text)),
            Span::styled("▏", Style::default().fg(app.theme.caret)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false }),
        region,
    );
}

/// A centered rectangle of the given width/height inside `area`.
pub fn center_rect(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

fn content_width(area: Rect, app: &App) -> u16 {
    // An explicit max_line_width wins; otherwise scale with the terminal
    // (~80% of width) so the test fills the screen instead of a fixed column.
    if app.config.max_line_width > 0 {
        return (app.config.max_line_width as u16).clamp(20, area.width.saturating_sub(2));
    }
    let proportional = (area.width as u32 * 4 / 5) as u16;
    proportional.clamp(20, area.width.saturating_sub(2))
}

/// Render a "terminal too small" notice and report whether the area is usable.
fn too_small(frame: &mut Frame, area: Rect, theme: &Theme) -> bool {
    if area.width < 20 || area.height < 4 {
        let line = Rect::new(area.x, area.y, area.width, 1).intersection(area);
        frame.render_widget(
            Paragraph::new("terminal too small").alignment(Alignment::Center),
            line,
        );
        let _ = theme;
        return true;
    }
    false
}

fn render_test(app: &App, frame: &mut Frame, area: Rect) {
    let t = &app.theme;
    if too_small(frame, area, t) {
        return;
    }
    let now = app.now_ms();
    let width = content_width(area, app);

    // ---- top indicator line (timer / word count + optional live wpm/acc) ----
    let mut top_spans: Vec<Span> = Vec::new();
    if app.config.timer_style != IndicatorStyle::Off {
        match app.config.mode {
            Mode::Time => {
                if let Some(left) = app.engine.time_left(now) {
                    let value = if app.config.timer_style == IndicatorStyle::Bar {
                        progress_bar(
                            app.config.time.saturating_sub(left) as usize,
                            app.config.time.max(1) as usize,
                        )
                    } else {
                        format!("{left}")
                    };
                    top_spans.push(Span::styled(value, Style::default().fg(t.main)));
                }
            }
            Mode::Words | Mode::Quote | Mode::Custom | Mode::Practice => {
                let (done, total) = app.engine.words_progress();
                let value = if app.config.timer_style == IndicatorStyle::Bar {
                    progress_bar(done, total.max(1))
                } else {
                    format!("{done}/{total}")
                };
                top_spans.push(Span::styled(value, Style::default().fg(t.main)));
            }
            Mode::Zen => {
                top_spans.push(Span::styled("zen", Style::default().fg(t.main)));
            }
        }
    }
    if app.config.live_speed_style != IndicatorStyle::Off {
        let value = app.engine.live_wpm(now).round() as i64;
        top_spans.push(Span::styled(
            if app.config.live_speed_style == IndicatorStyle::Mini {
                format!("  {value}")
            } else {
                format!("  {value} wpm")
            },
            Style::default().fg(t.sub),
        ));
    }
    if app.config.live_acc_style != IndicatorStyle::Off {
        let value = app.engine.live_acc().round() as i64;
        top_spans.push(Span::styled(
            if app.config.live_acc_style == IndicatorStyle::Mini {
                format!("  {value}%")
            } else {
                format!("  {value}% acc")
            },
            Style::default().fg(t.sub),
        ));
    }
    if app.config.live_burst_style != IndicatorStyle::Off {
        top_spans.push(Span::styled(
            format!("  {} burst", app.engine.live_burst().round() as i64),
            Style::default().fg(t.sub),
        ));
    }

    // ---- words ----
    let pace_position = app.pace_wpm.and_then(|wpm| {
        pace_position(
            &app.engine.target_words,
            wpm,
            app.engine.live_elapsed_secs(now),
        )
    });
    let (lines, active_line) =
        build_word_lines(&app.engine, t, &app.config, width as usize, pace_position);
    // Show a block of lines proportional to the terminal height (about half),
    // keeping one line of context above the active line. Scales from 3 lines on
    // a short terminal up to filling the screen on a tall one.
    let avail = (area.height as usize).saturating_sub(4);
    let start = if app.config.show_all_lines {
        0
    } else {
        active_line.saturating_sub(1)
    };
    let remaining = lines.len().saturating_sub(start).max(1);
    let visible = if app.config.show_all_lines {
        remaining.min(avail.max(1))
    } else {
        (area.height as usize / 2)
            .clamp(3, avail.max(3))
            .min(remaining)
    };
    let window: Vec<Line> = lines.into_iter().skip(start).take(visible).collect();

    // vertically center: indicator line, blank, then the word lines
    let block_height = (window.len() as u16) + 3;
    let inner = center_rect(area, width, block_height);

    // indicator
    let ind_rect = Rect::new(inner.x, inner.y, inner.width, 1).intersection(area);
    frame.render_widget(
        Paragraph::new(Line::from(top_spans)).alignment(Alignment::Left),
        ind_rect,
    );

    // words (left-aligned within the centered block)
    let words_rect =
        Rect::new(inner.x, inner.y + 2, inner.width, window.len() as u16).intersection(area);
    frame.render_widget(
        Paragraph::new(window).alignment(Alignment::Left),
        words_rect,
    );

    // ---- footer hint ----
    let hint = Line::from(Span::styled(
        "tab restart    esc menu    ctrl+c quit",
        Style::default().fg(t.sub),
    ));
    let footer = Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
    frame.render_widget(Paragraph::new(hint).alignment(Alignment::Center), footer);
}

fn progress_bar(done: usize, total: usize) -> String {
    let width = 12usize;
    let filled = done.min(total).saturating_mul(width) / total.max(1);
    format!("{}{}", "━".repeat(filled), "─".repeat(width - filled))
}

/// Build the wrapped, per-character-colored word lines, returning them plus the
/// index of the line that contains the active word.
fn build_word_lines(
    engine: &Engine,
    theme: &Theme,
    config: &crate::config::Config,
    max_width: usize,
    pace_position: Option<(usize, usize)>,
) -> (Vec<Line<'static>>, usize) {
    let active = engine.active;
    let mut lines: Vec<Line> = Vec::new();
    let mut current: Vec<Span> = Vec::new();
    let mut current_below: Vec<Span> = Vec::new();
    let mut current_has_below = false;
    let mut current_w = 0usize;
    let mut active_line = 0usize;

    let words = display_words(engine);

    for (i, target) in words.iter().enumerate() {
        let typed = engine.typed.get(i).map(|s| s.as_str()).unwrap_or("");
        let committed = i < active;
        let is_active = i == active;
        let highlighted = match config.highlight_mode {
            HighlightMode::Word => is_active,
            HighlightMode::NextWord => i == active.saturating_add(1),
            _ => false,
        };
        let (spans, below, wordw, has_below) = word_spans(
            target,
            typed,
            WordRenderState {
                committed,
                is_active,
                highlighted,
                pace_position: pace_position
                    .filter(|(word, _)| *word == i)
                    .map(|(_, character)| character),
            },
            theme,
            config,
        );

        // wrap (each word already includes its trailing inter-word space)
        if current_w + wordw > max_width && !current.is_empty() {
            lines.push(Line::from(std::mem::take(&mut current)));
            if current_has_below {
                lines.push(Line::from(std::mem::take(&mut current_below)));
            } else {
                current_below.clear();
            }
            current_has_below = false;
            current_w = 0;
        }
        if is_active {
            active_line = lines.len();
        }
        current.extend(spans);
        current_below.extend(below);
        current_has_below |= has_below;
        current_w += wordw;
    }
    if !current.is_empty() {
        lines.push(Line::from(current));
        if current_has_below {
            lines.push(Line::from(current_below));
        }
    }
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "start typing…",
            Style::default().fg(theme.sub),
        )));
    }
    (lines, active_line)
}

/// The target words to display. For zen we show what has been typed so far plus
/// the active in-progress word.
fn display_words(engine: &Engine) -> Vec<String> {
    if engine.is_zen() {
        let mut v: Vec<String> = engine.target_words.clone();
        // active in-progress word (typed but not committed)
        if let Some(cur) = engine.typed.get(engine.active) {
            if v.len() <= engine.active {
                v.push(cur.clone());
            }
        }
        if v.is_empty() {
            v.push(String::new());
        }
        v
    } else {
        engine.target_words.clone()
    }
}

fn caret_style(theme: &Theme, caret: CaretStyle, base: Style) -> Style {
    match caret {
        CaretStyle::Off => base,
        CaretStyle::Block => base.bg(theme.caret).fg(theme.bg),
        CaretStyle::Underline => base.fg(theme.caret).add_modifier(Modifier::UNDERLINED),
        // default / outline / line approximated as an underlined caret-colored cell
        _ => base.fg(theme.caret).add_modifier(Modifier::UNDERLINED),
    }
}

/// Build styled spans for one word and return (spans, display_width).
struct WordRenderState {
    committed: bool,
    is_active: bool,
    highlighted: bool,
    pace_position: Option<usize>,
}

fn word_spans(
    target: &str,
    typed: &str,
    state: WordRenderState,
    theme: &Theme,
    config: &crate::config::Config,
) -> (Vec<Span<'static>>, Vec<Span<'static>>, usize, bool) {
    let WordRenderState {
        committed,
        is_active,
        highlighted,
        pace_position,
    } = state;
    let tgt: Vec<char> = target.chars().collect();
    let inp: Vec<char> = typed.chars().collect();
    let cursor = inp.len();
    let n = tgt.len().max(inp.len());
    let mut spans: Vec<Span> = Vec::new();
    let mut below: Vec<Span> = Vec::new();
    let mut has_below = false;
    let mut width = 0usize;

    let push = |spans: &mut Vec<Span>, ch: char, style: Style| {
        spans.push(Span::styled(ch.to_string(), style));
    };

    for pos in 0..n {
        let tc = tgt.get(pos).copied();
        let ic = inp.get(pos).copied();

        // colorful mode uses the brighter error palette
        let err = if config.colorful_mode {
            theme.colorful_error
        } else {
            theme.error
        };
        let err_extra = if config.colorful_mode {
            theme.colorful_error_extra
        } else {
            theme.error_extra
        };

        // base style + glyph
        let mut below_glyph = ' ';
        let mut below_style = Style::default();
        let (glyph, mut style) = match (ic, tc) {
            (Some(a), Some(b)) => {
                if config.blind_mode || crate::engine::chars_equal(a, b, config.lazy_mode) {
                    let color = if highlighted {
                        theme.main
                    } else if config.flip_test_colors {
                        theme.sub
                    } else {
                        theme.text
                    };
                    (a, Style::default().fg(color))
                } else {
                    match config.indicate_typos {
                        IndicateTypos::Below => {
                            below_glyph = a;
                            below_style = Style::default().fg(err);
                            has_below = true;
                            (b, Style::default().fg(err))
                        }
                        IndicateTypos::Both => {
                            below_glyph = b;
                            below_style = Style::default().fg(theme.sub);
                            has_below = true;
                            (a, Style::default().fg(err))
                        }
                        IndicateTypos::Off | IndicateTypos::Replace => {
                            (a, Style::default().fg(err))
                        }
                    }
                }
            }
            (Some(a), None) => {
                // extra letter typed past the word
                if config.hide_extra_letters {
                    continue;
                }
                (a, Style::default().fg(err_extra))
            }
            (None, Some(b)) => {
                if committed {
                    // missed letter in a finished word
                    (b, Style::default().fg(err))
                } else {
                    let color = if highlighted {
                        theme.main
                    } else if config.flip_test_colors {
                        theme.text
                    } else {
                        theme.sub
                    };
                    (b, Style::default().fg(color))
                }
            }
            (None, None) => continue,
        };

        if is_active && pos == cursor {
            style = caret_style(theme, config.caret_style, style);
            style = match config.smooth_caret {
                SmoothCaret::Off => style,
                SmoothCaret::Slow | SmoothCaret::Medium => style.add_modifier(Modifier::SLOW_BLINK),
                SmoothCaret::Fast => style.add_modifier(Modifier::RAPID_BLINK),
            };
        } else if pace_position == Some(pos) {
            style = caret_style(theme, config.pace_caret_style, style);
        }
        if highlighted && !matches!(style.fg, Some(color) if color == err || color == err_extra) {
            style = style.add_modifier(Modifier::BOLD);
        }
        push(&mut spans, glyph, style);
        push(&mut below, below_glyph, below_style);
        width += 1;
    }

    // Trailing inter-word space is part of every word. When the caret sits at
    // the end of the active word, we simply restyle this existing space cell
    // instead of inserting a new one - so the line never shifts.
    let caret_on_space = is_active && cursor >= n;
    let space_style = if caret_on_space {
        caret_style(theme, config.caret_style, Style::default().fg(theme.sub))
    } else if pace_position.is_some_and(|position| position >= n) {
        caret_style(
            theme,
            config.pace_caret_style,
            Style::default().fg(theme.sub),
        )
    } else {
        Style::default().fg(theme.sub)
    };
    spans.push(Span::styled(" ", space_style));
    below.push(Span::raw(" "));
    width += 1;

    let _ = target.width(); // keep unicode-width available for future CJK widths
    (spans, below, width, has_below)
}

fn pace_position(words: &[String], wpm: f64, elapsed_secs: f64) -> Option<(usize, usize)> {
    let mut characters = (wpm.max(0.0) * 5.0 * elapsed_secs.max(0.0) / 60.0) as usize;
    for (word_index, word) in words.iter().enumerate() {
        let width = word.chars().count() + 1;
        if characters < width {
            return Some((word_index, characters));
        }
        characters = characters.saturating_sub(width);
    }
    None
}
