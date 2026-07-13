//! Terminal lifecycle: enter the alternate screen + raw mode, and restore it
//! reliably, including on panic, so a crash never leaves the user's terminal
//! in a broken state.

use anyhow::Result;
use crossterm::{
    event::{
        DisableFocusChange, EnableFocusChange, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::sync::atomic::{AtomicBool, Ordering};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Whether we pushed kitty keyboard-protocol flags, so restore() (including
/// the panic hook) pops exactly what was pushed and nothing else.
static KITTY_KEYBOARD: AtomicBool = AtomicBool::new(false);

pub fn init() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::cursor::Hide,
        EnableFocusChange
    )?;
    // The kitty keyboard protocol is the only channel through which Unix
    // terminals report caps-lock state (for the caps-lock warning). Enable it
    // where supported and degrade silently everywhere else. REPORT_EVENT_TYPES
    // is deliberately left off so no release/repeat events are delivered.
    if matches!(
        crossterm::terminal::supports_keyboard_enhancement(),
        Ok(true)
    ) {
        let flags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
            | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS;
        if execute!(stdout, PushKeyboardEnhancementFlags(flags)).is_ok() {
            KITTY_KEYBOARD.store(true, Ordering::SeqCst);
        }
    }
    install_panic_hook();
    let terminal = Terminal::new(CrosstermBackend::new(stdout))?;
    Ok(terminal)
}

pub fn restore() -> Result<()> {
    let mut stdout = io::stdout();
    // pop while still on the alternate screen: the kitty keyboard stack is
    // tracked per screen, so the pop must precede LeaveAlternateScreen
    if KITTY_KEYBOARD.swap(false, Ordering::SeqCst) {
        let _ = execute!(stdout, PopKeyboardEnhancementFlags);
    }
    execute!(
        stdout,
        DisableFocusChange,
        LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;
    disable_raw_mode()?;
    Ok(())
}

/// Restore the terminal before the default panic handler prints, so panic
/// messages are readable instead of being swallowed by the alt screen.
fn install_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore();
        hook(info);
    }));
}
