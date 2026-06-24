//! mtype: a terminal typing test. A Rust port of Monkeytype, offline and
//! account-free.
//!
//! Copyright (C) 2026 Ramin Sharifi and mtype contributors.
//! Portions derived from Monkeytype (https://github.com/monkeytypegame/monkeytype),
//! copyright the Monkeytype contributors. Licensed under GPL-3.0; see LICENSE
//! and NOTICE.md.

mod app;
mod commandline;
mod config;
mod content;
mod engine;
mod funbox;
mod numbers;
mod persistence;
mod results;
mod theme;
mod tui;
mod ui;
mod web;
mod wordgen;

use anyhow::Result;
use app::App;
use clap::{Parser, Subcommand};
use config::{Config, Difficulty, Mode};
use rand::SeedableRng;

/// A terminal typing test (offline Monkeytype port).
///
/// Flags set the config for this run only (they are not persisted). Change
/// settings persistently in-app via the command line (press Esc).
#[derive(Parser, Debug)]
#[command(name = "mtype", version, about)]
struct Cli {
    /// Test mode: time, words, quote, zen, custom
    #[arg(long)]
    mode: Option<String>,
    /// Seconds for time mode (implies --mode time)
    #[arg(long)]
    time: Option<u32>,
    /// Word count for words mode (implies --mode words)
    #[arg(long)]
    words: Option<u32>,
    /// Enable punctuation
    #[arg(long)]
    punctuation: bool,
    /// Enable numbers
    #[arg(long)]
    numbers: bool,
    /// Difficulty: normal, expert, master
    #[arg(long)]
    difficulty: Option<String>,
    /// Run a custom-text test from this string (implies --mode custom)
    #[arg(long)]
    custom: Option<String>,
    /// Language (default: english; sync others first with `mtype sync`)
    #[arg(long)]
    language: Option<String>,
    /// Print the generated words for the current settings and exit (no TUI)
    #[arg(long, hide = true)]
    dump_words: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Download extra content from the Monkeytype repo for offline use.
    ///
    /// Examples:
    ///   mtype sync language spanish
    ///   mtype sync quotes french
    Sync {
        /// What to download: "language" or "quotes"
        kind: String,
        /// The language name (e.g. spanish, english_5k, code_rust)
        name: String,
    },
}

fn apply_cli(cfg: &mut Config, cli: &Cli) {
    if let Some(m) = cli.mode.as_deref().and_then(Mode::from_str_opt) {
        cfg.mode = m;
    }
    if let Some(t) = cli.time {
        cfg.time = t;
        cfg.mode = Mode::Time;
    }
    if let Some(w) = cli.words {
        cfg.words = w;
        cfg.mode = Mode::Words;
    }
    if cli.punctuation {
        cfg.punctuation = true;
    }
    if cli.numbers {
        cfg.numbers = true;
    }
    if let Some(d) = cli.difficulty.as_deref().and_then(Difficulty::from_str_opt) {
        cfg.difficulty = d;
    }
    if let Some(text) = cli.custom.as_ref() {
        cfg.custom_text = text.clone();
        cfg.mode = Mode::Custom;
    }
    if let Some(lang) = cli.language.as_ref() {
        cfg.language = lang.clone();
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // `sync` subcommand runs without the TUI
    if let Some(Commands::Sync { kind, name }) = &cli.command {
        return web::run_sync(kind, name);
    }

    let mut config = Config::load();
    apply_cli(&mut config, &cli);

    // warn (don't fail) if an explicitly requested language isn't available
    // offline - it will fall back to the base English list.
    if let Some(name) = cli.language.as_deref() {
        if !content::language_available(name) {
            eprintln!(
                "note: '{name}' is not bundled or synced - falling back to english. \
                 download it with: mtype sync language {name}"
            );
        }
    }

    // debug/preview: print generated words without launching the TUI
    if cli.dump_words {
        let mut rng = rand::rngs::StdRng::from_entropy();
        let (words, _) = wordgen::generate_test_words(&config, &mut rng);
        println!("{}", words.join(" "));
        return Ok(());
    }

    let mut terminal = tui::init()?;
    let mut app = App::new(config);
    let result = app.run(&mut terminal);
    tui::restore()?;
    result
}
