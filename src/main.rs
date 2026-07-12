//! mtype: a terminal typing test. A Rust port of Monkeytype, offline and
//! account-free.
//!
//! Copyright (C) 2026 Ramin Sharifi and mtype contributors.
//! Portions derived from Monkeytype (https://github.com/monkeytypegame/monkeytype),
//! copyright the Monkeytype contributors. Licensed under GPL-3.0; see LICENSE
//! and NOTICE.md.

mod analytics;
mod app;
mod commandline;
mod config;
mod content;
mod dashboard;
mod engine;
mod funbox;
mod numbers;
mod persistence;
mod presets;
mod results;
mod stats;
mod theme;
mod tui;
mod ui;
mod web;
mod wordgen;

use anyhow::Result;
use app::App;
use clap::{Parser, Subcommand};
use config::{Config, Difficulty, Mode, PracticeMode};
use rand::SeedableRng;
use std::path::PathBuf;

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
    /// Run a specific quote ID (implies --mode quote)
    #[arg(long)]
    quote_id: Option<u32>,
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
    /// Open local stats in the terminal or serve the browser dashboard.
    Stats {
        #[command(subcommand)]
        command: Option<StatsCommands>,
    },
    /// Practice words you previously missed, typed slowly, or both.
    Practice {
        /// Practice source: missed, slow, or mixed.
        #[arg(default_value = "mixed")]
        kind: String,
        /// Number of words in the practice test.
        #[arg(long, default_value_t = 25)]
        words: u32,
    },
    /// Export, import, or reset local typing data.
    Data {
        #[command(subcommand)]
        command: DataCommands,
    },
}

#[derive(Subcommand, Debug)]
enum StatsCommands {
    /// Start the local browser dashboard.
    Serve {
        /// Localhost port for the dashboard.
        #[arg(long, default_value_t = 4242)]
        port: u16,
        /// Print the URL without opening the default browser.
        #[arg(long)]
        no_open: bool,
    },
}

#[derive(Subcommand, Debug)]
enum DataCommands {
    /// Export results, mistakes, and replay data as portable JSON.
    Export { path: PathBuf },
    /// Merge a portable JSON export into local data.
    Import { path: PathBuf },
    /// Permanently remove all local results and analytics.
    Reset {
        /// Required acknowledgement for the destructive reset.
        #[arg(long)]
        yes: bool,
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
    if let Some(id) = cli.quote_id {
        cfg.quote_id = Some(id);
        cfg.mode = Mode::Quote;
    }
    if let Some(lang) = cli.language.as_ref() {
        cfg.language = lang.clone();
    }
    if let Some(Commands::Practice { kind, words }) = &cli.command {
        cfg.mode = Mode::Practice;
        cfg.practice_mode = PracticeMode::from_str_opt(kind).unwrap_or_default();
        cfg.practice_word_count = (*words).max(1);
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // `sync` subcommand runs without the TUI
    if let Some(Commands::Sync { kind, name }) = &cli.command {
        return web::run_sync(kind, name);
    }
    if let Some(Commands::Stats {
        command: Some(StatsCommands::Serve { port, no_open }),
    }) = &cli.command
    {
        return dashboard::serve(*port, !no_open);
    }
    if let Some(Commands::Data { command }) = &cli.command {
        match command {
            DataCommands::Export { path } => {
                persistence::export_data(path)?;
                println!("exported local typing data to {}", path.display());
            }
            DataCommands::Import { path } => {
                let imported = persistence::import_data(path)?;
                println!("imported {imported} new results from {}", path.display());
            }
            DataCommands::Reset { yes: true } => {
                persistence::reset_all_data()?;
                println!("all local typing data has been removed");
            }
            DataCommands::Reset { yes: false } => {
                anyhow::bail!("refusing to reset data without --yes");
            }
        }
        return Ok(());
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
    // `mtype stats` opens straight to the progress screen
    if matches!(cli.command, Some(Commands::Stats { command: None })) {
        app.open_stats();
    }
    let result = app.run(&mut terminal);
    tui::restore()?;
    result
}
