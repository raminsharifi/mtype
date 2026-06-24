//! Typing content: the embedded English word list and quote collection.
//! Mirrors the shape of `frontend/static/languages/english.json` and
//! `frontend/static/quotes/english.json`. Everything is compiled into the
//! binary via `include_str!` so the app is fully offline.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

macro_rules! asset {
    ($file:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $file))
    };
}

const ENGLISH_JSON: &str = asset!("english.json");
const ENGLISH_QUOTES_JSON: &str = asset!("english-quotes.json");

/// Every English word list bundled in the binary, available fully offline.
/// Mirrors the `english*` languages shipped by Monkeytype.
const EMBEDDED_LANGUAGES: &[(&str, &str)] = &[
    ("english", ENGLISH_JSON),
    ("english_1k", asset!("english_1k.json")),
    ("english_5k", asset!("english_5k.json")),
    ("english_10k", asset!("english_10k.json")),
    ("english_25k", asset!("english_25k.json")),
    ("english_450k", asset!("english_450k.json")),
    (
        "english_commonly_misspelled",
        asset!("english_commonly_misspelled.json"),
    ),
    ("english_contractions", asset!("english_contractions.json")),
    ("english_doubleletter", asset!("english_doubleletter.json")),
    ("english_legal", asset!("english_legal.json")),
    ("english_medical", asset!("english_medical.json")),
    ("english_old", asset!("english_old.json")),
    (
        "english_shakespearean",
        asset!("english_shakespearean.json"),
    ),
];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // metadata fields are deserialized but not all consumed yet
pub struct Language {
    pub name: String,
    #[serde(default)]
    pub no_lazy_mode: bool,
    #[serde(default)]
    pub ordered_by_frequency: bool,
    #[serde(default)]
    pub right_to_left: bool,
    pub words: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Quote {
    pub text: String,
    #[serde(default)]
    pub source: String,
    pub length: usize,
    pub id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // `language` is part of the on-disk shape
pub struct QuoteCollection {
    pub language: String,
    /// Length bands: `[[min,max], ...]` for short/medium/long/thicc.
    pub groups: Vec<[i64; 2]>,
    pub quotes: Vec<Quote>,
}

impl QuoteCollection {
    /// Quotes whose length falls within band index `band` (0=short..3=thicc).
    pub fn in_band(&self, band: usize) -> Vec<&Quote> {
        let Some(range) = self.groups.get(band) else {
            return self.quotes.iter().collect();
        };
        let (lo, hi) = (range[0], range[1]);
        self.quotes
            .iter()
            .filter(|q| (q.length as i64) >= lo && (q.length as i64) <= hi)
            .collect()
    }
}

static ENGLISH: OnceLock<Language> = OnceLock::new();
static ENGLISH_QUOTES: OnceLock<QuoteCollection> = OnceLock::new();

/// The embedded English word list (the 200-word frequency-ordered default).
pub fn english() -> &'static Language {
    ENGLISH
        .get_or_init(|| serde_json::from_str(ENGLISH_JSON).expect("embedded english.json is valid"))
}

/// The embedded English quote collection.
pub fn english_quotes() -> &'static QuoteCollection {
    ENGLISH_QUOTES.get_or_init(|| {
        serde_json::from_str(ENGLISH_QUOTES_JSON).expect("embedded english-quotes.json is valid")
    })
}

/// Directory where `mtype sync` stores downloaded content.
pub fn data_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("com", "monkeytype", "mtype").map(|d| d.data_dir().to_path_buf())
}

fn local_file(kind: &str, name: &str) -> Option<std::path::PathBuf> {
    let path = data_dir()?.join(kind).join(format!("{name}.json"));
    path.exists().then_some(path)
}

/// Parse cache for embedded languages - big lists (e.g. english_450k, ~8 MB)
/// are parsed at most once and reused across test restarts.
static EMBEDDED_CACHE: OnceLock<Mutex<HashMap<&'static str, &'static Language>>> = OnceLock::new();

fn embedded_language(name: &str) -> Option<&'static Language> {
    let &(sname, json) = EMBEDDED_LANGUAGES.iter().find(|(n, _)| *n == name)?;
    let cache = EMBEDDED_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().expect("language cache mutex");
    if let Some(lang) = guard.get(sname) {
        return Some(lang);
    }
    let parsed: Language = serde_json::from_str(json).expect("embedded language json is valid");
    let leaked: &'static Language = Box::leak(Box::new(parsed));
    guard.insert(sname, leaked);
    Some(leaked)
}

/// Names of every bundled (offline) language.
pub fn embedded_language_names() -> Vec<&'static str> {
    EMBEDDED_LANGUAGES.iter().map(|(n, _)| *n).collect()
}

/// Whether a language is available offline (bundled or already synced to disk).
pub fn language_available(name: &str) -> bool {
    EMBEDDED_LANGUAGES.iter().any(|(n, _)| *n == name) || local_file("languages", name).is_some()
}

/// Resolve a language by name. A synced file in the data dir takes priority
/// (lets users override or add languages), then the bundled English variants,
/// and finally a fallback to the base English list.
pub fn language(name: &str) -> Language {
    if let Some(path) = local_file("languages", name) {
        if let Ok(s) = std::fs::read_to_string(&path) {
            if let Ok(lang) = serde_json::from_str::<Language>(&s) {
                return lang;
            }
        }
    }
    if let Some(lang) = embedded_language(name) {
        return lang.clone();
    }
    english().clone()
}

/// Resolve a quote collection by language. Prefers a synced file
/// (`quotes/<language>.json`); otherwise falls back to embedded English quotes.
pub fn quotes(language: &str) -> QuoteCollection {
    if let Some(path) = local_file("quotes", language) {
        if let Ok(s) = std::fs::read_to_string(&path) {
            if let Ok(q) = serde_json::from_str::<QuoteCollection>(&s) {
                return q;
            }
        }
    }
    english_quotes().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_loads() {
        let lang = english();
        assert_eq!(lang.name, "english");
        assert!(lang.ordered_by_frequency);
        assert!(lang.words.len() >= 100);
        assert!(lang.words.iter().any(|w| w == "the"));
    }

    #[test]
    fn quotes_load_and_band_filters() {
        let q = english_quotes();
        assert_eq!(q.language, "english");
        assert_eq!(q.groups.len(), 4);
        assert!(!q.quotes.is_empty());
        // medium band (index 1) should be non-empty and within range
        let medium = q.in_band(1);
        assert!(!medium.is_empty());
        let [lo, hi] = q.groups[1];
        assert!(medium
            .iter()
            .all(|qt| (qt.length as i64) >= lo && (qt.length as i64) <= hi));
    }

    #[test]
    fn all_embedded_english_variants_load() {
        // every bundled language parses and has words; the name matches
        for name in embedded_language_names() {
            let lang = language(name);
            assert!(!lang.words.is_empty(), "{name} produced an empty word list");
            assert_eq!(lang.name, name, "{name} has mismatched internal name");
            assert!(language_available(name));
        }
        // the headline sizes match Monkeytype's
        assert_eq!(language("english").words.len(), 200);
        assert_eq!(language("english_1k").words.len(), 1000);
        assert_eq!(language("english_5k").words.len(), 5000);
        assert!(language("english_25k").words.len() > 20000);
        assert!(language("english_450k").words.len() > 400000);
    }

    #[test]
    fn unknown_language_is_not_available() {
        assert!(!language_available("klingon_zzz"));
        // …but resolving still falls back to base english rather than panicking
        assert!(!language("klingon_zzz").words.is_empty());
    }
}
