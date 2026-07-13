//! Rendering. Dispatches to the test screen or the results screen.

use crate::app::{App, Screen};
use crate::config::{
    CaretStyle, HighlightMode, IndicateTypos, IndicatorStyle, Mode, SmoothCaret, TypingSpeedUnit,
};
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
    render_build_revision(app, frame, area);
}

const BUILD_GIT_HASH: &str = env!("MTYPE_BUILD_GIT_HASH");

fn build_revision_label() -> String {
    format!("git:{BUILD_GIT_HASH}")
}

/// Keep the exact build revision visible regardless of the active screen or
/// overlay. A dirty suffix means the binary includes uncommitted source edits.
fn render_build_revision(app: &App, frame: &mut Frame, area: Rect) {
    let label = build_revision_label();
    let width = UnicodeWidthStr::width(label.as_str()).min(u16::MAX as usize) as u16;
    if area.height == 0 || width == 0 || area.width < width {
        return;
    }
    let rect = Rect::new(
        area.right().saturating_sub(width),
        area.bottom().saturating_sub(1),
        width,
        1,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            label,
            Style::default().fg(app.theme.sub).bg(app.theme.bg),
        ))
        .alignment(Alignment::Right),
        rect,
    );
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
    // The upper bound can drop below the preferred 20-column minimum on very
    // narrow terminals, so the minimum must yield to it (clamp panics when
    // min > max).
    let max = area.width.saturating_sub(2).max(1);
    let min = 20u16.min(max);
    let desired = if app.config.max_line_width > 0 {
        app.config.max_line_width as u16
    } else {
        (area.width as u32 * 4 / 5) as u16
    };
    desired.clamp(min, max)
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
                } else {
                    // time 0 = infinite test: count elapsed seconds up instead
                    let elapsed = app.engine.live_elapsed_secs(now).floor() as u64;
                    top_spans.push(Span::styled(
                        format!("{elapsed}"),
                        Style::default().fg(t.main),
                    ));
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
    let unit = app.config.typing_speed_unit;
    if app.config.live_speed_style != IndicatorStyle::Off {
        let wpm = app.engine.live_wpm(now);
        let value = fmt_speed_value(unit.convert_from_wpm(wpm), unit);
        let text = match app.config.live_speed_style {
            IndicatorStyle::Bar => format!("  {}", speed_bar(wpm)),
            IndicatorStyle::Mini => format!("  {value}"),
            _ => format!("  {value} {}", crate::stats::unit_label(unit)),
        };
        top_spans.push(Span::styled(text, Style::default().fg(t.sub)));
    }
    if app.config.live_acc_style != IndicatorStyle::Off {
        let acc = app.engine.live_acc();
        let value = acc.round() as i64;
        let text = match app.config.live_acc_style {
            IndicatorStyle::Bar => format!("  {}", progress_bar(acc.round() as usize, 100)),
            IndicatorStyle::Mini => format!("  {value}%"),
            _ => format!("  {value}% acc"),
        };
        top_spans.push(Span::styled(text, Style::default().fg(t.sub)));
    }
    if app.config.live_burst_style != IndicatorStyle::Off {
        let burst = app.engine.live_burst();
        let value = fmt_speed_value(unit.convert_from_wpm(burst), unit);
        let text = match app.config.live_burst_style {
            IndicatorStyle::Bar => format!("  {}", speed_bar(burst)),
            IndicatorStyle::Mini => format!("  {value}"),
            _ => format!("  {value} burst"),
        };
        top_spans.push(Span::styled(text, Style::default().fg(t.sub)));
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
        // Show everything when it fits; once the text outgrows the terminal,
        // scroll just enough to keep the active line in view.
        let cap = avail.max(1);
        if lines.len() <= cap {
            0
        } else {
            (active_line + 1).saturating_sub(cap).min(lines.len() - cap)
        }
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

/// Live speed/burst rendered as a bar: a fixed gauge that fills up at
/// 200 raw wpm (unit-independent, since it is a ratio).
fn speed_bar(wpm: f64) -> String {
    const FULL_SCALE_WPM: f64 = 200.0;
    progress_bar(
        wpm.clamp(0.0, FULL_SCALE_WPM).round() as usize,
        FULL_SCALE_WPM as usize,
    )
}

/// Format a speed already converted to the configured unit. The per-second
/// units are too coarse as integers, so they keep one decimal.
fn fmt_speed_value(value: f64, unit: TypingSpeedUnit) -> String {
    match unit {
        TypingSpeedUnit::Wps | TypingSpeedUnit::Cps => format!("{value:.1}"),
        _ => format!("{}", value.round() as i64),
    }
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
            // upstream highlights the active word AND the one after it
            HighlightMode::NextWord => is_active || i == active.saturating_add(1),
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

/// Style for the cell the user caret sits on: the configured caret shape plus
/// the smooth-caret blink. `caret_style = off` means no caret effect at all,
/// so the blink is gated on it too.
fn user_caret_style(theme: &Theme, config: &crate::config::Config, base: Style) -> Style {
    if config.caret_style == CaretStyle::Off {
        return base;
    }
    let style = caret_style(theme, config.caret_style, base);
    match config.smooth_caret {
        SmoothCaret::Off => style,
        SmoothCaret::Slow | SmoothCaret::Medium => style.add_modifier(Modifier::SLOW_BLINK),
        SmoothCaret::Fast => style.add_modifier(Modifier::RAPID_BLINK),
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

    // Letter palette, mirroring upstream test.scss: colorful mode brightens
    // correct letters to the main color (and errors to the colorful pair);
    // flip test colors swaps the typed/untyped pair.
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
    let correct_color = match (config.flip_test_colors, config.colorful_mode) {
        (true, _) => theme.sub,
        (false, true) => theme.main,
        (false, false) => theme.text,
    };
    let untyped_color = match (config.flip_test_colors, config.colorful_mode) {
        (true, true) => theme.main,
        (true, false) => theme.text,
        (false, _) => theme.sub,
    };
    // highlight mode off leaves typed text un-highlighted: correct letters
    // keep the untyped color (upstream .highlight-off letter.correct rule)
    let correct_display = if highlighted {
        theme.main
    } else if config.highlight_mode == HighlightMode::Off {
        untyped_color
    } else {
        correct_color
    };
    let untyped_display = if highlighted {
        theme.main
    } else {
        untyped_color
    };

    for pos in 0..n {
        let tc = tgt.get(pos).copied();
        let ic = inp.get(pos).copied();

        // base style + glyph
        let mut below_glyph = ' ';
        let mut below_style = Style::default();
        let (glyph, mut style) = match (ic, tc) {
            (Some(a), Some(b)) => {
                if config.blind_mode || crate::engine::chars_equal(a, b, config.lazy_mode) {
                    (a, Style::default().fg(correct_display))
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
                        // off keeps the target letter (error-colored); only
                        // replace swaps in the typed character
                        IndicateTypos::Off => (b, Style::default().fg(err)),
                        IndicateTypos::Replace => (a, Style::default().fg(err)),
                    }
                }
            }
            (Some(a), None) => {
                // extra letter typed past the word; blind mode hides extras
                // entirely, like upstream (.blind letter.extra display:none)
                if config.hide_extra_letters || config.blind_mode {
                    continue;
                }
                (a, Style::default().fg(err_extra))
            }
            (None, Some(b)) => {
                if committed && !config.blind_mode {
                    // missed letter in a finished word
                    (b, Style::default().fg(err))
                } else {
                    (b, Style::default().fg(untyped_display))
                }
            }
            (None, None) => continue,
        };

        if is_active && pos == cursor {
            style = user_caret_style(theme, config, style);
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
        user_caret_style(theme, config, Style::default().fg(theme.sub))
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
    // In time mode the generated buffer only covers ~100 words up front and is
    // topped up around the typist, so a fast pace can outrun it. The pace
    // caret exists precisely for paces faster than the typist, so park it on
    // the last generated word's trailing space instead of vanishing (rendering
    // has only `&App`, so growing the buffer from here is not possible).
    words
        .last()
        .map(|word| (words.len() - 1, word.chars().count()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::config::Config;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn state(committed: bool, is_active: bool, highlighted: bool) -> WordRenderState {
        WordRenderState {
            committed,
            is_active,
            highlighted,
            pace_position: None,
        }
    }

    fn draw(app: &App, width: u16, height: u16) -> Terminal<TestBackend> {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|f| render(app, f)).unwrap();
        terminal
    }

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

    #[test]
    fn build_revision_is_pinned_to_the_bottom_right() {
        let app = App::new(Config::default());
        let terminal = draw(&app, 80, 24);
        let text = buffer_text(&terminal);
        let bottom = text.lines().last().unwrap();
        assert!(
            bottom.ends_with(&build_revision_label()),
            "revision must end the bottom row:\n{bottom}"
        );
    }

    /// Regression: 20/21-column terminals used to panic in content_width
    /// (Ord::clamp called with min > max).
    #[test]
    fn narrow_terminal_widths_never_panic() {
        for max_line_width in [0, 60] {
            let cfg = Config {
                max_line_width,
                ..Config::default()
            };
            let app = App::new(cfg);
            for width in 18..=25 {
                draw(&app, width, 10);
            }
        }
    }

    #[test]
    fn content_width_is_bounded_on_all_narrow_widths() {
        for max_line_width in [0, 60] {
            let cfg = Config {
                max_line_width,
                ..Config::default()
            };
            let app = App::new(cfg);
            for width in 0..=30u16 {
                let got = content_width(Rect::new(0, 0, width, 10), &app);
                assert!(got >= 1, "width {width}: got {got}");
                assert!(
                    got <= width.saturating_sub(2).max(1),
                    "width {width}: got {got}"
                );
            }
        }
    }

    #[test]
    fn indicate_typos_off_shows_target_letter_replace_shows_typed() {
        let theme = Theme::serika_dark();
        let mut config = Config::default();
        assert_eq!(config.indicate_typos, IndicateTypos::Off);
        let (spans, _, _, _) =
            word_spans("hello", "hx", state(false, true, false), &theme, &config);
        assert_eq!(spans[1].content, "e", "off keeps the target letter");
        assert_eq!(spans[1].style.fg, Some(theme.error));

        config.indicate_typos = IndicateTypos::Replace;
        let (spans, _, _, _) =
            word_spans("hello", "hx", state(false, true, false), &theme, &config);
        assert_eq!(spans[1].content, "x", "replace shows the typed letter");
        assert_eq!(spans[1].style.fg, Some(theme.error));
    }

    #[test]
    fn blind_mode_never_renders_error_colors() {
        let theme = Theme::serika_dark();
        let mut config = Config {
            blind_mode: true,
            ..Config::default()
        };

        // a mistyped letter renders as if correct
        let (spans, _, _, _) =
            word_spans("hello", "hx", state(false, true, false), &theme, &config);
        assert_eq!(spans[1].style.fg, Some(theme.text));

        // extra letters are hidden entirely (upstream `.blind letter.extra`)
        let (spans, _, width, _) =
            word_spans("hi", "hix", state(false, true, false), &theme, &config);
        assert_eq!(width, 3, "two letters + trailing space, extra hidden");
        assert!(spans
            .iter()
            .all(|span| span.style.fg != Some(theme.error_extra)));

        // committed word with missed letters keeps the untyped color
        let (spans, _, _, _) =
            word_spans("hello", "hel", state(true, false, false), &theme, &config);
        assert_eq!(spans[3].style.fg, Some(theme.sub));
        assert_eq!(spans[4].style.fg, Some(theme.sub));

        // sanity: without blind mode both cases are error colored
        config.blind_mode = false;
        let (spans, _, _, _) =
            word_spans("hello", "hel", state(true, false, false), &theme, &config);
        assert_eq!(spans[3].style.fg, Some(theme.error));
        let (spans, _, _, _) = word_spans("hi", "hix", state(false, true, false), &theme, &config);
        assert_eq!(spans[2].style.fg, Some(theme.error_extra));
    }

    #[test]
    fn caret_style_off_renders_no_caret_and_no_blink() {
        let theme = Theme::serika_dark();
        let config = Config {
            caret_style: CaretStyle::Off,
            ..Config::default()
        };
        assert_eq!(config.smooth_caret, SmoothCaret::Medium);

        // mid-word cursor cell styled like any untyped letter
        let (spans, _, _, _) =
            word_spans("hello", "he", state(false, true, false), &theme, &config);
        let cell = spans[2].style;
        assert_eq!(cell.fg, Some(theme.sub));
        assert_eq!(cell.bg, None);
        assert!(!cell.add_modifier.contains(Modifier::SLOW_BLINK));
        assert!(!cell.add_modifier.contains(Modifier::UNDERLINED));

        // caret resting on the trailing space: same
        let (spans, _, _, _) = word_spans("hi", "hi", state(false, true, false), &theme, &config);
        let space = spans.last().unwrap().style;
        assert_eq!(space.bg, None);
        assert!(!space.add_modifier.contains(Modifier::SLOW_BLINK));
        assert!(!space.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn smooth_caret_blinks_on_trailing_space_too() {
        let theme = Theme::serika_dark();
        let mut config = Config {
            caret_style: CaretStyle::Block,
            smooth_caret: SmoothCaret::Medium,
            ..Config::default()
        };
        let (spans, _, _, _) = word_spans("hi", "hi", state(false, true, false), &theme, &config);
        let space = spans.last().unwrap().style;
        assert_eq!(space.bg, Some(theme.caret));
        assert!(space.add_modifier.contains(Modifier::SLOW_BLINK));

        config.smooth_caret = SmoothCaret::Fast;
        let (spans, _, _, _) = word_spans("hi", "hi", state(false, true, false), &theme, &config);
        assert!(spans
            .last()
            .unwrap()
            .style
            .add_modifier
            .contains(Modifier::RAPID_BLINK));
    }

    #[test]
    fn highlight_off_leaves_typed_letters_unhighlighted() {
        let theme = Theme::serika_dark();
        let mut config = Config {
            highlight_mode: HighlightMode::Off,
            ..Config::default()
        };
        let (spans, _, _, _) =
            word_spans("hello", "he", state(false, true, false), &theme, &config);
        // correct letters keep the untyped color instead of brightening
        assert_eq!(spans[0].style.fg, Some(theme.sub));
        // errors still show: upstream highlight-off only dims correct letters
        let (spans, _, _, _) =
            word_spans("hello", "hx", state(false, true, false), &theme, &config);
        assert_eq!(spans[1].style.fg, Some(theme.error));

        config.highlight_mode = HighlightMode::Letter;
        let (spans, _, _, _) =
            word_spans("hello", "he", state(false, true, false), &theme, &config);
        assert_eq!(spans[0].style.fg, Some(theme.text));
    }

    #[test]
    fn colorful_mode_brightens_correct_letters_to_main() {
        let theme = Theme::serika_dark();
        let mut config = Config {
            colorful_mode: true,
            ..Config::default()
        };
        let (spans, _, _, _) =
            word_spans("hello", "he", state(false, false, false), &theme, &config);
        assert_eq!(spans[0].style.fg, Some(theme.main));
        assert_eq!(spans[3].style.fg, Some(theme.sub));

        // flipped + colorful swaps the pair: untyped main, correct sub
        config.flip_test_colors = true;
        let (spans, _, _, _) =
            word_spans("hello", "he", state(false, false, false), &theme, &config);
        assert_eq!(spans[0].style.fg, Some(theme.sub));
        assert_eq!(spans[3].style.fg, Some(theme.main));
    }

    #[test]
    fn highlight_next_word_includes_active_word() {
        let cfg = Config {
            mode: Mode::Words,
            words: 4,
            highlight_mode: HighlightMode::NextWord,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        app.engine.target_words = vec![
            "alpha".to_string(),
            "beta".to_string(),
            "gamma".to_string(),
            "delta".to_string(),
        ];
        app.engine.typed = vec![String::new()];
        app.engine.active = 0;
        let terminal = draw(&app, 80, 24);
        let buf = terminal.backend().buffer();
        let mut bold_main = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                let cell = &buf[(x, y)];
                let style = cell.style();
                if style.fg == Some(app.theme.main) && style.add_modifier.contains(Modifier::BOLD) {
                    bold_main.push_str(cell.symbol());
                }
            }
        }
        assert_eq!(
            bold_main, "alphabeta",
            "next_word must highlight the active word AND the next"
        );
    }

    #[test]
    fn live_indicators_honor_speed_unit() {
        let cfg = Config {
            mode: Mode::Words,
            words: 10,
            live_speed_style: IndicatorStyle::Text,
            live_burst_style: IndicatorStyle::Text,
            typing_speed_unit: TypingSpeedUnit::Cpm,
            ..Config::default()
        };
        let app = App::new(cfg);
        let text = buffer_text(&draw(&app, 80, 24));
        assert!(text.contains(" cpm"), "live speed must use the unit label");
        assert!(!text.contains(" wpm"), "no hardcoded wpm label:\n{text}");
        assert!(text.contains(" burst"), "burst text style keeps its label");
    }

    #[test]
    fn live_bar_styles_render_bars() {
        let cfg = Config {
            mode: Mode::Words,
            words: 10,
            live_speed_style: IndicatorStyle::Bar,
            live_acc_style: IndicatorStyle::Bar,
            ..Config::default()
        };
        let app = App::new(cfg);
        let text = buffer_text(&draw(&app, 80, 24));
        // two 12-cell gauges (speed 0, acc 0 with no keystrokes yet)
        assert_eq!(
            text.matches("────────────").count(),
            2,
            "speed and acc bars must render as bars:\n{text}"
        );
        assert!(!text.contains("wpm"), "bar style has no text label");
        assert!(!text.contains("% acc"), "bar style has no text label");
    }

    /// Regression: the pace caret used to vanish (None) once the pace passed
    /// the end of the generated word buffer in time mode.
    #[test]
    fn pace_position_clamps_to_last_generated_word() {
        let words = vec!["the".to_string(), "cat".to_string()];
        // within the buffer: normal advancing position
        assert_eq!(pace_position(&words, 60.0, 0.2), Some((0, 1)));
        // way past the buffer: parked on the last word's trailing space
        assert_eq!(pace_position(&words, 300.0, 60.0), Some((1, 3)));
        // empty buffer (zen): still no pace caret
        assert_eq!(pace_position(&[], 300.0, 60.0), None);
    }

    /// Regression: with show_all_lines on, the window used to pin to line 0
    /// and the caret vanished once the text outgrew the terminal.
    #[test]
    fn show_all_lines_keeps_active_line_visible() {
        let cfg = Config {
            mode: Mode::Words,
            words: 100,
            show_all_lines: true,
            ..Config::default()
        };
        let mut app = App::new(cfg);
        app.engine.target_words = vec!["word".to_string(); 100];
        let mut typed = vec!["word".to_string(); 60];
        typed.push(String::new());
        app.engine.typed = typed;
        app.engine.active = 60;

        let terminal = draw(&app, 40, 8);
        let buf = terminal.backend().buffer();
        let mut caret_cells = 0;
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                if buf[(x, y)]
                    .style()
                    .add_modifier
                    .contains(Modifier::UNDERLINED)
                {
                    caret_cells += 1;
                }
            }
        }
        assert_eq!(
            caret_cells, 1,
            "the caret (active line) must stay in the visible window"
        );
    }
}
