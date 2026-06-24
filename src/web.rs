//! Optional online content sync (the only networked feature). Downloads
//! additional language word lists and quote collections from the Monkeytype
//! GitHub repo into the local data dir, where the content loader picks them up
//! ahead of the embedded English defaults.
//!
//! The app is fully functional offline without ever running this.

use crate::content::{self, Language, QuoteCollection};
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

const RAW_BASE: &str =
    "https://raw.githubusercontent.com/monkeytypegame/monkeytype/master/frontend/static";

/// A few popular language names to point users at (Monkeytype ships ~445).
pub const POPULAR_LANGUAGES: &[&str] = &[
    "english_1k",
    "english_5k",
    "english_10k",
    "spanish",
    "french",
    "german",
    "italian",
    "portuguese",
    "dutch",
    "russian",
    "code_python",
    "code_javascript",
    "code_rust",
    "code_c",
];

fn get(url: &str) -> Result<String> {
    let resp = ureq::get(url)
        .timeout(std::time::Duration::from_secs(20))
        .call()
        .with_context(|| format!("requesting {url}"))?;
    resp.into_string().context("reading response body")
}

fn save(kind: &str, name: &str, body: &str) -> Result<PathBuf> {
    let dir = content::data_dir()
        .context("could not resolve data dir")?
        .join(kind);
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join(format!("{name}.json"));
    std::fs::write(&path, body).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// Download a language word list (e.g. `spanish`, `english_5k`, `code_rust`).
pub fn sync_language(name: &str) -> Result<PathBuf> {
    let url = format!("{RAW_BASE}/languages/{name}.json");
    let body = get(&url)?;
    // validate before storing
    serde_json::from_str::<Language>(&body)
        .with_context(|| format!("'{name}' did not parse as a language word list"))?;
    save("languages", name, &body)
}

/// Download a quote collection for a language (e.g. `french`).
pub fn sync_quotes(language: &str) -> Result<PathBuf> {
    let url = format!("{RAW_BASE}/quotes/{language}.json");
    let body = get(&url)?;
    serde_json::from_str::<QuoteCollection>(&body)
        .with_context(|| format!("'{language}' did not parse as a quote collection"))?;
    save("quotes", language, &body)
}

/// Entry point for the `sync` subcommand.
pub fn run_sync(kind: &str, name: &str) -> Result<()> {
    let path = match kind {
        "language" | "languages" | "lang" => sync_language(name)?,
        "quotes" | "quote" => sync_quotes(name)?,
        other => {
            bail!(
                "unknown sync kind '{other}' (use 'language' or 'quotes')\n\
                 popular languages: {}",
                POPULAR_LANGUAGES.join(", ")
            );
        }
    };
    println!("✓ downloaded {kind} '{name}' → {}", path.display());
    println!("  it will now be available offline (set language/mode in-app or via flags)");
    Ok(())
}
