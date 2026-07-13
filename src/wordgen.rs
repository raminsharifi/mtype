//! Word generation - a faithful English-only port of
//! `frontend/src/ts/test/words-generator.ts` (+ `wordset.ts`, `utils/generate.ts`,
//! `utils/misc.ts::zipfyRandomArrayIndex`).
//!
//! The generator is stateful (it tracks the previous two emitted words for the
//! no-repeat rule, like Monkeytype) and generic over an `rng::Rng` so tests can
//! drive it deterministically. Funbox transforms hook in at Phase 6.

use crate::config::{Config, Mode};
use crate::content::{self, Quote};
use crate::funbox::{self, Funbox};
use rand::Rng;

const GAMMA: f64 = 0.5772156649015329; // Euler-Mascheroni constant

/// Where words come from for the current test.
enum Source {
    /// Random draws from a pool with the no-repeat + transform pipeline
    /// (time / words modes).
    Pool(Vec<String>),
    /// Verbatim ordered words (quote / custom modes).
    Ordered(Vec<String>),
}

pub struct WordGenerator {
    config: Config,
    funboxes: Vec<Funbox>,
    source: Source,
    /// `wordsBound` passed to `punctuateWord` (forces a sentence end on the last
    /// word, capitalizes the first).
    bound: usize,
    word_index: usize,
    ordered_index: usize,
    prev1: String,
    prev2: String,
    /// For quote mode: the selected quote (source/id for the results screen).
    pub quote: Option<Quote>,
}

/// Random integer in `[min, max]` inclusive (mirrors `randomIntFromRange`).
fn rand_int<R: Rng>(rng: &mut R, min: i64, max: i64) -> i64 {
    if max < min {
        return min;
    }
    rng.gen_range(min..=max)
}

/// `utils/generate.ts::getNumbers` - a 1..=len digit string, first digit 1-9.
fn get_numbers<R: Rng>(rng: &mut R, len: i64) -> String {
    let rand_len = rand_int(rng, 1, len);
    let mut s = String::new();
    for i in 0..rand_len {
        let d = if i == 0 {
            rand_int(rng, 1, 9)
        } else {
            rand_int(rng, 0, 9)
        };
        s.push_str(&d.to_string());
    }
    s
}

/// `utils/misc.ts::zipfyRandomArrayIndex`.
fn zipf_index<R: Rng>(rng: &mut R, dict_len: usize) -> usize {
    let h_n = ((dict_len as f64) + 0.5).ln() + GAMMA;
    let r: f64 = rng.gen();
    let inverse_cdf = (r * h_n - GAMMA).exp() - 0.5;
    (inverse_cdf.floor() as usize).min(dict_len.saturating_sub(1))
}

fn should_capitalize(last: char) -> bool {
    matches!(last, '?' | '!' | '.' | '؟')
}

fn capitalize_first(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn strip_for_compare(word: &str, also_apostrophe: bool) -> String {
    word.chars()
        .filter(|c| {
            let is_punct = matches!(c, '.' | '?' | '!' | '"' | ':' | '-' | ',')
                || (also_apostrophe && *c == '\'');
            !is_punct
        })
        .collect::<String>()
        .to_lowercase()
}

fn has_symbol(word: &str) -> bool {
    word.chars().any(|c| {
        matches!(
            c,
            '-' | '='
                | '_'
                | '+'
                | '['
                | ']'
                | '{'
                | '}'
                | ';'
                | '\''
                | '\\'
                | ':'
                | '"'
                | '|'
                | ','
                | '.'
                | '/'
                | '<'
                | '>'
                | '?'
        )
    })
}

fn has_digit(word: &str) -> bool {
    word.chars().any(|c| c.is_ascii_digit())
}

/// Practice words are replayed verbatim from past results, which store targets
/// with whatever decorations they carried at the time ("Word.", "(like)",
/// "'order'"). When punctuation is off, strip those decorations back to the
/// base word; word-internal characters (apostrophes, hyphens) are kept since
/// they belong to the word itself.
fn strip_practice_decorations(word: &str) -> String {
    let stripped = word.trim_matches(|c: char| !c.is_alphanumeric());
    if stripped.chars().any(|c| c.is_uppercase()) {
        stripped.to_lowercase()
    } else {
        stripped.to_string()
    }
}

fn british_spelling(word: &str) -> String {
    match word {
        "color" => "colour",
        "colors" => "colours",
        "favorite" => "favourite",
        "favorites" => "favourites",
        "honor" => "honour",
        "honors" => "honours",
        "labor" => "labour",
        "neighbor" => "neighbour",
        "neighbors" => "neighbours",
        "center" => "centre",
        "centers" => "centres",
        "theater" => "theatre",
        "theaters" => "theatres",
        "meter" => "metre",
        "meters" => "metres",
        "liter" => "litre",
        "liters" => "litres",
        "organize" => "organise",
        "organized" => "organised",
        "organizing" => "organising",
        "recognize" => "recognise",
        "recognized" => "recognised",
        "realize" => "realise",
        "realized" => "realised",
        "analyze" => "analyse",
        "analyzed" => "analysed",
        "behavior" => "behaviour",
        "catalog" => "catalogue",
        "dialog" => "dialogue",
        "traveling" => "travelling",
        "traveled" => "travelled",
        "canceled" => "cancelled",
        "canceling" => "cancelling",
        "defense" => "defence",
        "offense" => "offence",
        "license" => "licence",
        "practice" => "practise",
        "gray" => "grey",
        _ => return word.to_string(),
    }
    .to_string()
}

/// Ordered words (quote / custom / practice) carry decorations and capitals
/// ("Color," / "color."). Upstream's `applyBritishEnglishToWord` runs on
/// quote/custom words too, matching case-insensitively inside the word - so
/// split off the non-letter edges, map the core, keep the original first
/// letter's capitalization, and reattach the decorations.
fn british_spelling_decorated(word: &str) -> String {
    let Some(start) = word.find(|c: char| c.is_alphabetic()) else {
        return word.to_string();
    };
    let end = word
        .char_indices()
        .rfind(|(_, c)| c.is_alphabetic())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(word.len());
    let (prefix, rest) = word.split_at(start);
    let (core, suffix) = rest.split_at(end - start);

    let lower = core.to_lowercase();
    let mapped = british_spelling(&lower);
    if mapped == lower {
        return word.to_string();
    }
    let mapped = if core.chars().next().is_some_and(char::is_uppercase) {
        capitalize_first(&mapped)
    } else {
        mapped
    };
    format!("{prefix}{mapped}{suffix}")
}

/// English-only port of `punctuateWord`. Each `else if` branch draws one fresh
/// random and is taken on the first match, exactly as the JS `&&`-short-circuit
/// chain does - so the resulting punctuation distribution matches Monkeytype.
fn punctuate_word<R: Rng>(
    rng: &mut R,
    prev_word: &str,
    word: &str,
    index: usize,
    maxindex: usize,
) -> String {
    let mut w = word.to_string();
    let last = prev_word.chars().last().unwrap_or(' ');
    let idx = index as i64;
    let maxi = maxindex as i64;

    let r = |rng: &mut R| -> f64 { rng.gen() };

    if index == 0 || should_capitalize(last) {
        w = capitalize_first(&w);
    } else if (r(rng) < 0.1 && last != '.' && last != ',' && idx != maxi - 2) || idx == maxi - 1 {
        let rand: f64 = rng.gen();
        if rand <= 0.8 {
            w.push('.');
        } else if rand < 0.9 {
            w.push('?');
        } else {
            w.push('!');
        }
    } else if r(rng) < 0.01 && last != ',' && last != '.' {
        w = format!("\"{w}\"");
    } else if r(rng) < 0.011 && last != ',' && last != '.' {
        w = format!("'{w}'");
    } else if r(rng) < 0.012 && last != ',' && last != '.' {
        w = format!("({w})");
    } else if r(rng) < 0.013 && !matches!(last, ',' | '.' | ';' | '؛' | ':' | '；' | '：') {
        w.push(':');
    } else if r(rng) < 0.014 && last != ',' && last != '.' && prev_word != "-" {
        w = "-".to_string();
    } else if r(rng) < 0.015 && !matches!(last, ',' | '.' | ';' | '؛' | '；' | '：') {
        w.push(';');
    } else if r(rng) < 0.2 && last != ',' {
        w.push(',');
    }
    // The english-punctuation contraction branch (random < 0.5) is omitted -     // its lookup data is not vendored. Everything else matches.
    w
}

impl WordGenerator {
    /// Build a generator for the given config, selecting a quote / custom text
    /// up front where relevant.
    pub fn new<R: Rng>(config: &Config, rng: &mut R) -> WordGenerator {
        let cfg = config.clone();
        let (source, bound, quote) = match cfg.mode {
            Mode::Words => {
                let lang = content::language(&cfg.language);
                let bound = if cfg.words == 0 {
                    100
                } else {
                    cfg.words as usize
                };
                (Source::Pool(lang.words.clone()), bound, None)
            }
            Mode::Time | Mode::Zen => {
                let lang = content::language(&cfg.language);
                (Source::Pool(lang.words.clone()), 100, None)
            }
            Mode::Quote => {
                let (words, q) = pick_quote(&cfg, rng);
                let bound = words.len().max(1);
                (Source::Ordered(words), bound, q)
            }
            Mode::Custom => {
                // upstream's default custom text; an empty text would
                // otherwise produce a zero-word test
                let text = if cfg.custom_text.trim().is_empty() {
                    "The quick brown fox jumps over the lazy dog"
                } else {
                    cfg.custom_text.as_str()
                };
                let words: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();
                let bound = words.len().max(1);
                (Source::Ordered(words), bound, None)
            }
            Mode::Practice => {
                let mut words: Vec<String> = cfg
                    .practice_text
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                let count = cfg.practice_word_count.max(1) as usize;
                if !cfg.punctuation {
                    words = words
                        .iter()
                        .map(|w| strip_practice_decorations(w))
                        .filter(|w| !w.is_empty())
                        .collect();
                }
                if !cfg.numbers {
                    // Stored digit words are whole-word `getNumbers` injections
                    // recorded by past numbers-on tests; drop them when numbers
                    // is off (mirrors the pool digit gate in next_pool_word).
                    words.retain(|w| !w.chars().all(|c| c.is_ascii_digit()));
                }
                // Filtering can remove every stored practice target (for
                // example, an all-number practice set with numbers disabled).
                // Fall back after filtering so the finite test is never empty.
                if words.is_empty() {
                    let lang = content::language(&cfg.language);
                    let mut fallback: Vec<String> = lang.words.clone();
                    if !cfg.punctuation {
                        fallback = fallback
                            .iter()
                            .map(|w| strip_practice_decorations(w))
                            .filter(|w| !w.is_empty())
                            .collect();
                    }
                    if !cfg.numbers {
                        fallback.retain(|w| !w.chars().all(|c| c.is_ascii_digit()));
                    }
                    if fallback.is_empty() {
                        fallback.push("the".to_string());
                    }
                    for _ in 0..count {
                        words.push(fallback[rng.gen_range(0..fallback.len())].clone());
                    }
                }
                let bound = words.len().max(1);
                (Source::Ordered(words), bound, None)
            }
        };

        let funboxes = funbox::parse(&cfg.funbox);
        WordGenerator {
            config: cfg,
            funboxes,
            source,
            bound,
            word_index: 0,
            ordered_index: 0,
            prev1: String::new(),
            prev2: String::new(),
            quote,
        }
    }

    /// Whether this generator yields a finite, fixed word list (quote/custom/
    /// words-with-count) vs an endless stream (time / words=0 / zen).
    pub fn is_finite(&self) -> bool {
        match (&self.source, self.config.mode) {
            (Source::Ordered(_), _) => true,
            (Source::Pool(_), Mode::Words) => self.config.words != 0,
            _ => false,
        }
    }

    fn random_pool_word<R: Rng>(&self, rng: &mut R, pool: &[String]) -> String {
        if pool.is_empty() {
            return String::new();
        }
        let i = if funbox::has_zipf(&self.funboxes) {
            zipf_index(rng, pool.len())
        } else {
            rng.gen_range(0..pool.len())
        };
        pool[i].clone()
    }

    /// Emit the next word, or `None` if an ordered source is exhausted.
    pub fn next<R: Rng>(&mut self, rng: &mut R) -> Option<String> {
        let word = match &self.source {
            Source::Ordered(words) => {
                if self.ordered_index >= words.len() {
                    return None;
                }
                let mut w = words[self.ordered_index].clone();
                self.ordered_index += 1;
                // Upstream's getNextWord runs these on every branch, so quote/
                // custom/practice words get them too: the british english
                // replacement pass, then alterText funboxes (rot13, backwards,
                // …). getWord funboxes stay pool-only - upstream replaces
                // generated words with them rather than altering quote text.
                if self.config.british_english {
                    w = british_spelling_decorated(&w);
                }
                w = funbox::alter_all(&self.funboxes, w, self.word_index, self.bound, rng);
                w
            }
            Source::Pool(pool) => {
                let pool = pool.clone();
                self.next_pool_word(rng, &pool)
            }
        };

        self.prev2 = std::mem::take(&mut self.prev1);
        self.prev1 = word.clone();
        self.word_index += 1;
        Some(word)
    }

    fn next_pool_word<R: Rng>(&mut self, rng: &mut R, pool: &[String]) -> String {
        // getWord funboxes (gibberish, ascii, numbers, …) replace the word
        // entirely and skip the normal punctuation/number pipeline.
        if let Some(w) = funbox::get_word(&self.funboxes, rng) {
            return funbox::alter_all(&self.funboxes, w, self.word_index, self.bound, rng);
        }

        let punctuation = self.config.punctuation;
        let numbers = self.config.numbers;
        let prev1raw = strip_for_compare(&self.prev1, false);
        let prev2raw = strip_for_compare(&self.prev2, true);

        let mut word = self.random_pool_word(rng, pool);
        let mut first = word.split(' ').next().unwrap_or("").to_lowercase();

        let mut count = 0;
        while count < 100
            && ((!prev1raw.is_empty() && prev1raw == first)
                || (!prev2raw.is_empty() && prev2raw == first)
                || (!punctuation && word == "I")
                || (!punctuation && has_symbol(&word))
                || (!numbers && has_digit(&word)))
        {
            count += 1;
            word = self.random_pool_word(rng, pool);
            first = word.split(' ').next().unwrap_or("").to_lowercase();
        }

        // lowercase stray capitals when punctuation is off
        if !punctuation && word.chars().any(|c| c.is_ascii_uppercase()) {
            word = word.to_lowercase();
        }
        // lazy mode is a no-op for english (noLazyMode: true)

        let mut out = if self.config.british_english {
            british_spelling(&word)
        } else {
            word
        };
        if punctuation {
            out = punctuate_word(rng, &self.prev1, &out, self.word_index, self.bound);
        }
        if numbers && rng.gen::<f64>() < 0.1 {
            out = get_numbers(rng, 4);
        }
        // alterText funboxes (capitals, rot13, backwards, …)
        out = funbox::alter_all(&self.funboxes, out, self.word_index, self.bound, rng);
        out
    }
}

/// Pick a quote for the configured length bands and return its word list (after
/// the same whitespace/ellipsis cleanup Monkeytype applies) plus the quote meta.
fn quote_language(lang: &str) -> String {
    // strip a trailing `_<n>k` size suffix (english_1k -> english)
    if let Some(idx) = lang.rfind('_') {
        let suffix = &lang[idx + 1..];
        let digits = &suffix[..suffix.len().saturating_sub(1)];
        if suffix.ends_with('k') && !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
        {
            return lang[..idx].to_string();
        }
    }
    lang.to_string()
}

fn pick_quote<R: Rng>(config: &Config, rng: &mut R) -> (Vec<String>, Option<Quote>) {
    let collection = content::quotes(&quote_language(&config.language));
    if let Some(id) = config.quote_id {
        if let Some(chosen) = collection
            .quotes
            .iter()
            .find(|quote| quote.id == id)
            .cloned()
        {
            let cleaned = clean_quote_text(&chosen.text);
            let words = cleaned.split(' ').map(str::to_string).collect();
            return (words, Some(chosen));
        }
    }
    // union of selected bands
    let mut pool: Vec<&Quote> = Vec::new();
    for band in &config.quote_length {
        pool.extend(collection.in_band(band.index()));
    }
    if pool.is_empty() {
        pool = collection.quotes.iter().collect();
    }
    let chosen = pool[rng.gen_range(0..pool.len())].clone();

    let cleaned = clean_quote_text(&chosen.text);
    let words: Vec<String> = cleaned.split(' ').map(|s| s.to_string()).collect();
    (words, Some(chosen))
}

/// Mirror the quote text normalization in `getQuoteWordList`.
fn clean_quote_text(text: &str) -> String {
    let mut t = text.replace('…', "...");
    // collapse runs of spaces
    while t.contains("  ") {
        t = t.replace("  ", " ");
    }
    // normalize newlines to "\n " then we just treat as spaces for the CLI
    t = t.replace(['\r', '\n'], " ");
    while t.contains("  ") {
        t = t.replace("  ", " ");
    }
    t.trim().to_string()
}

/// Generate a fixed target word list for finite modes (or an initial batch for
/// streaming modes - the engine tops up from the generator as needed).
pub fn generate_test_words<R: Rng>(config: &Config, rng: &mut R) -> (Vec<String>, WordGenerator) {
    let mut gen = WordGenerator::new(config, rng);
    let mut words = Vec::new();

    match config.mode {
        Mode::Zen => { /* no target words */ }
        Mode::Words => {
            let n = if config.words == 0 {
                100
            } else {
                config.words as usize
            };
            for _ in 0..n {
                if let Some(w) = gen.next(rng) {
                    words.push(w);
                }
            }
        }
        Mode::Time => {
            for _ in 0..100 {
                if let Some(w) = gen.next(rng) {
                    words.push(w);
                }
            }
        }
        Mode::Quote | Mode::Custom | Mode::Practice => {
            while let Some(w) = gen.next(rng) {
                words.push(w);
            }
        }
    }
    (words, gen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Mode, QuoteLengthBand};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn cfg() -> Config {
        Config::default()
    }

    #[test]
    fn words_mode_generates_exact_count() {
        let mut c = cfg();
        c.mode = Mode::Words;
        c.words = 25;
        let mut rng = StdRng::seed_from_u64(1);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert_eq!(words.len(), 25);
        assert!(words.iter().all(|w| !w.is_empty()));
    }

    #[test]
    fn no_consecutive_duplicates() {
        let mut c = cfg();
        c.mode = Mode::Words;
        c.words = 100;
        let mut rng = StdRng::seed_from_u64(7);
        let (words, _) = generate_test_words(&c, &mut rng);
        for pair in words.windows(2) {
            assert_ne!(pair[0], pair[1], "consecutive duplicate: {pair:?}");
        }
    }

    #[test]
    fn no_capital_i_without_punctuation() {
        let mut c = cfg();
        c.mode = Mode::Words;
        c.words = 200;
        c.punctuation = false;
        let mut rng = StdRng::seed_from_u64(3);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert!(words.iter().all(|w| w != "I"));
        // no stray capitals when punctuation off
        assert!(words
            .iter()
            .all(|w| !w.chars().any(|ch| ch.is_ascii_uppercase())));
    }

    #[test]
    fn practice_strips_stored_punctuation_when_off() {
        let mut c = cfg();
        c.mode = Mode::Practice;
        c.punctuation = false;
        c.practice_text = "interest. order, the? (like) 'order' \"world\" Word don't -".into();
        let mut rng = StdRng::seed_from_u64(11);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert_eq!(
            words,
            ["interest", "order", "the", "like", "order", "world", "word", "don't"]
        );
    }

    #[test]
    fn practice_keeps_stored_punctuation_when_on() {
        let mut c = cfg();
        c.mode = Mode::Practice;
        c.punctuation = true;
        c.practice_text = "interest. order, Word".into();
        let mut rng = StdRng::seed_from_u64(11);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert_eq!(words, ["interest.", "order,", "Word"]);
    }

    #[test]
    fn practice_fallback_has_no_capitals_without_punctuation() {
        let mut c = cfg();
        c.mode = Mode::Practice;
        c.punctuation = false;
        c.practice_text = String::new();
        c.practice_word_count = 50;
        let mut rng = StdRng::seed_from_u64(13);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert!(!words.is_empty());
        assert!(words
            .iter()
            .all(|w| !w.chars().any(|ch| ch.is_ascii_uppercase())));
    }

    #[test]
    fn practice_drops_stored_digit_words_when_numbers_off() {
        let mut c = cfg();
        c.mode = Mode::Practice;
        c.punctuation = false;
        c.numbers = false;
        c.practice_text = "8425 interest 19 order 3".into();
        let mut rng = StdRng::seed_from_u64(11);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert_eq!(words, ["interest", "order"]);
    }

    #[test]
    fn practice_falls_back_when_filtering_removes_every_word() {
        let mut c = cfg();
        c.mode = Mode::Practice;
        c.punctuation = false;
        c.numbers = false;
        c.practice_word_count = 7;
        c.practice_text = "8425 19 3".into();
        let mut rng = StdRng::seed_from_u64(11);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert_eq!(words.len(), 7);
        assert!(words.iter().all(|word| !word.is_empty()));
        assert!(words
            .iter()
            .all(|word| !word.chars().all(|c| c.is_ascii_digit())));
    }

    #[test]
    fn practice_keeps_stored_digit_words_when_numbers_on() {
        let mut c = cfg();
        c.mode = Mode::Practice;
        c.punctuation = false;
        c.numbers = true;
        c.practice_text = "8425 interest 19 order".into();
        let mut rng = StdRng::seed_from_u64(11);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert_eq!(words, ["8425", "interest", "19", "order"]);
    }

    #[test]
    fn british_english_applies_to_custom_words_with_decorations() {
        let mut c = cfg();
        c.mode = Mode::Custom;
        c.british_english = true;
        c.custom_text = "color Center realize, \"gray\" (Behavior) plain don't".into();
        let mut rng = StdRng::seed_from_u64(11);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert_eq!(
            words,
            [
                "colour",
                "Centre",
                "realise,",
                "\"grey\"",
                "(Behaviour)",
                "plain",
                "don't"
            ]
        );
    }

    #[test]
    fn british_english_applies_to_quote_words() {
        let mut c = cfg();
        c.mode = Mode::Quote;
        c.british_english = true;
        // "But on this particular day I did not like the color of the sky."
        c.quote_id = Some(611);
        let mut rng = StdRng::seed_from_u64(2);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert!(words.iter().any(|w| w == "colour"), "words: {words:?}");
        assert!(!words.iter().any(|w| w == "color"));
    }

    #[test]
    fn funbox_alter_text_applies_to_ordered_modes() {
        // custom
        let mut c = cfg();
        c.mode = Mode::Custom;
        c.custom_text = "abc hello".into();
        c.funbox = vec!["backwards".to_string()];
        let mut rng = StdRng::seed_from_u64(3);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert_eq!(words, ["cba", "olleh"]);

        // practice
        let mut c = cfg();
        c.mode = Mode::Practice;
        c.punctuation = false;
        c.practice_text = "hello world".into();
        c.funbox = vec!["rot13".to_string()];
        let mut rng = StdRng::seed_from_u64(3);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert_eq!(words, ["uryyb", "jbeyq"]);

        // quote
        let mut c = cfg();
        c.mode = Mode::Quote;
        c.quote_length = vec![QuoteLengthBand::Short];
        c.funbox = vec!["ALL_CAPS".to_string()];
        let mut rng = StdRng::seed_from_u64(2);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert!(!words.is_empty());
        assert!(
            words.iter().all(|w| *w == w.to_uppercase()),
            "words: {words:?}"
        );
    }

    #[test]
    fn punctuation_adds_marks_and_capitals() {
        let mut c = cfg();
        c.mode = Mode::Words;
        c.words = 200;
        c.punctuation = true;
        let mut rng = StdRng::seed_from_u64(5);
        let (words, _) = generate_test_words(&c, &mut rng);
        // first word is always capitalized
        assert!(words[0].chars().next().unwrap().is_uppercase());
        // some punctuation appears across 200 words
        let joined = words.join(" ");
        assert!(joined.contains('.') || joined.contains(',') || joined.contains('?'));
    }

    #[test]
    fn numbers_injects_digit_words() {
        let mut c = cfg();
        c.mode = Mode::Words;
        c.words = 300;
        c.numbers = true;
        let mut rng = StdRng::seed_from_u64(9);
        let (words, _) = generate_test_words(&c, &mut rng);
        assert!(words
            .iter()
            .any(|w| w.chars().all(|ch| ch.is_ascii_digit()) && !w.is_empty()));
    }

    #[test]
    fn quote_mode_yields_a_quote() {
        let mut c = cfg();
        c.mode = Mode::Quote;
        c.quote_length = vec![QuoteLengthBand::Short];
        let mut rng = StdRng::seed_from_u64(2);
        let (words, gen) = generate_test_words(&c, &mut rng);
        assert!(!words.is_empty());
        assert!(gen.quote.is_some());
    }

    #[test]
    fn zen_has_no_target() {
        let mut c = cfg();
        c.mode = Mode::Zen;
        let mut rng = StdRng::seed_from_u64(2);
        let (words, gen) = generate_test_words(&c, &mut rng);
        assert!(words.is_empty());
        assert!(!gen.is_finite());
    }

    #[test]
    fn time_mode_is_infinite_batch() {
        let mut c = cfg();
        c.mode = Mode::Time;
        let mut rng = StdRng::seed_from_u64(4);
        let (words, gen) = generate_test_words(&c, &mut rng);
        assert_eq!(words.len(), 100);
        assert!(!gen.is_finite());
    }
}
