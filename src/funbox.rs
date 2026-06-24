//! Funbox modifiers - the terminal-feasible subset of Monkeytype's funboxes,
//! ported from `frontend/src/ts/test/funbox/funbox-functions.ts` and
//! `frontend/src/ts/utils/generate.ts`.
//!
//! Two kinds are supported:
//! - **getWord** funboxes generate a word from scratch (ignoring the language):
//!   `58008` (numbers), `gibberish`, `ascii`, `specials`, `binary`,
//!   `hexadecimal`, `IPv4`, `IPv6`.
//! - **alterText** funboxes transform a word: `capitals`, `rAnDoMcAsE`,
//!   `sPoNgEcAsE`, `ALL_CAPS`, `rot13`, `backwards`, `ddoouubblleedd`,
//!   `instant_messaging`, `underscore_spaces`, `morse`.
//!
//! Plus `zipf` (frequency) and `no_quit` (blocks restart). Purely visual/audio
//! funboxes (mirror, nausea, tts, …) and ones needing deeper engine work
//! (nospace, read_ahead, memory, plus_one, weakspot, pseudolang, polyglot) are
//! intentionally not implemented here.

use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Funbox {
    // getWord
    Numbers58008,
    Gibberish,
    Ascii,
    Specials,
    Binary,
    Hexadecimal,
    Ipv4,
    Ipv6,
    // alterText
    Capitals,
    RandomCase,
    SpongeCase,
    AllCaps,
    Rot13,
    Backwards,
    Doubled,
    InstantMessaging,
    UnderscoreSpaces,
    Morse,
    // behaviour
    Zipf,
    NoQuit,
}

impl Funbox {
    pub fn from_name(s: &str) -> Option<Funbox> {
        Some(match s {
            "58008" => Funbox::Numbers58008,
            "gibberish" => Funbox::Gibberish,
            "ascii" => Funbox::Ascii,
            "specials" => Funbox::Specials,
            "binary" => Funbox::Binary,
            "hexadecimal" => Funbox::Hexadecimal,
            "IPv4" => Funbox::Ipv4,
            "IPv6" => Funbox::Ipv6,
            "capitals" => Funbox::Capitals,
            "rAnDoMcAsE" => Funbox::RandomCase,
            "sPoNgEcAsE" => Funbox::SpongeCase,
            "ALL_CAPS" => Funbox::AllCaps,
            "rot13" => Funbox::Rot13,
            "backwards" => Funbox::Backwards,
            "ddoouubblleedd" => Funbox::Doubled,
            "instant_messaging" => Funbox::InstantMessaging,
            "underscore_spaces" => Funbox::UnderscoreSpaces,
            "morse" => Funbox::Morse,
            "zipf" => Funbox::Zipf,
            "no_quit" => Funbox::NoQuit,
            _ => return None,
        })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Funbox::Numbers58008 => "58008",
            Funbox::Gibberish => "gibberish",
            Funbox::Ascii => "ascii",
            Funbox::Specials => "specials",
            Funbox::Binary => "binary",
            Funbox::Hexadecimal => "hexadecimal",
            Funbox::Ipv4 => "IPv4",
            Funbox::Ipv6 => "IPv6",
            Funbox::Capitals => "capitals",
            Funbox::RandomCase => "rAnDoMcAsE",
            Funbox::SpongeCase => "sPoNgEcAsE",
            Funbox::AllCaps => "ALL_CAPS",
            Funbox::Rot13 => "rot13",
            Funbox::Backwards => "backwards",
            Funbox::Doubled => "ddoouubblleedd",
            Funbox::InstantMessaging => "instant_messaging",
            Funbox::UnderscoreSpaces => "underscore_spaces",
            Funbox::Morse => "morse",
            Funbox::Zipf => "zipf",
            Funbox::NoQuit => "no_quit",
        }
    }

    fn is_get_word(&self) -> bool {
        matches!(
            self,
            Funbox::Numbers58008
                | Funbox::Gibberish
                | Funbox::Ascii
                | Funbox::Specials
                | Funbox::Binary
                | Funbox::Hexadecimal
                | Funbox::Ipv4
                | Funbox::Ipv6
        )
    }
}

/// All funboxes exposed in the command palette.
pub const SUPPORTED: &[Funbox] = &[
    Funbox::Numbers58008,
    Funbox::Gibberish,
    Funbox::Ascii,
    Funbox::Specials,
    Funbox::Binary,
    Funbox::Hexadecimal,
    Funbox::Ipv4,
    Funbox::Ipv6,
    Funbox::Capitals,
    Funbox::RandomCase,
    Funbox::SpongeCase,
    Funbox::AllCaps,
    Funbox::Rot13,
    Funbox::Backwards,
    Funbox::Doubled,
    Funbox::InstantMessaging,
    Funbox::UnderscoreSpaces,
    Funbox::Morse,
    Funbox::Zipf,
    Funbox::NoQuit,
];

/// Parse the config funbox list into known funboxes (unknown names ignored).
pub fn parse(names: &[String]) -> Vec<Funbox> {
    names.iter().filter_map(|n| Funbox::from_name(n)).collect()
}

pub fn has_zipf(fbs: &[Funbox]) -> bool {
    fbs.contains(&Funbox::Zipf)
}

pub fn has_no_quit(fbs: &[Funbox]) -> bool {
    fbs.contains(&Funbox::NoQuit)
}

fn rand_int<R: Rng>(rng: &mut R, min: i64, max: i64) -> i64 {
    if max < min {
        return min;
    }
    rng.gen_range(min..=max)
}

// ---- getWord generators (ported from utils/generate.ts) ----

fn get_numbers<R: Rng>(rng: &mut R, len: i64) -> String {
    let n = rand_int(rng, 1, len);
    (0..n)
        .map(|i| {
            if i == 0 {
                rand_int(rng, 1, 9)
            } else {
                rand_int(rng, 0, 9)
            }
            .to_string()
        })
        .collect()
}

fn get_gibberish<R: Rng>(rng: &mut R) -> String {
    let n = rand_int(rng, 1, 7);
    (0..n)
        .map(|_| (b'a' + rand_int(rng, 0, 25) as u8) as char)
        .collect()
}

fn get_ascii<R: Rng>(rng: &mut R) -> String {
    let n = rand_int(rng, 1, 10);
    (0..n)
        .map(|_| (33 + rand_int(rng, 0, 93) as u8) as char)
        .collect()
}

const SPECIALS: &[char] = &[
    '`', '~', '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '-', '_', '=', '+', '{', '}', '[',
    ']', '\'', '"', '/', '\\', '|', '?', ';', ':', '>', '<', ',', '.',
];

fn get_specials<R: Rng>(rng: &mut R) -> String {
    let n = rand_int(rng, 1, 7);
    (0..n)
        .map(|_| SPECIALS[rng.gen_range(0..SPECIALS.len())])
        .collect()
}

fn get_binary<R: Rng>(rng: &mut R) -> String {
    format!("{:08b}", rng.gen_range(0..256u16))
}

fn get_hexadecimal<R: Rng>(rng: &mut R) -> String {
    let n = rand_int(rng, 1, 4);
    let body: String = (0..n)
        .map(|_| format!("{:02x}", rng.gen_range(0..256u16)))
        .collect();
    format!("0x{body}")
}

fn get_ipv4<R: Rng>(rng: &mut R) -> String {
    format!(
        "{}.{}.{}.{}",
        rng.gen_range(0..256u16),
        rng.gen_range(0..256u16),
        rng.gen_range(0..256u16),
        rng.gen_range(0..256u16)
    )
}

fn get_ipv6<R: Rng>(rng: &mut R) -> String {
    (0..8)
        .map(|_| format!("{:x}", rng.gen_range(0..65536u32)))
        .collect::<Vec<_>>()
        .join(":")
}

const MORSE: &[(char, &str)] = &[
    ('a', ".-"),
    ('b', "-..."),
    ('c', "-.-."),
    ('d', "-.."),
    ('e', "."),
    ('f', "..-."),
    ('g', "--."),
    ('h', "...."),
    ('i', ".."),
    ('j', ".---"),
    ('k', "-.-"),
    ('l', ".-.."),
    ('m', "--"),
    ('n', "-."),
    ('o', "---"),
    ('p', ".--."),
    ('q', "--.-"),
    ('r', ".-."),
    ('s', "..."),
    ('t', "-"),
    ('u', "..-"),
    ('v', "...-"),
    ('w', ".--"),
    ('x', "-..-"),
    ('y', "-.--"),
    ('z', "--.."),
    ('0', "-----"),
    ('1', ".----"),
    ('2', "..---"),
    ('3', "...--"),
    ('4', "....-"),
    ('5', "....."),
    ('6', "-...."),
    ('7', "--..."),
    ('8', "---.."),
    ('9', "----."),
];

fn to_morse(word: &str) -> String {
    let mut out = String::new();
    for ch in word.to_lowercase().chars() {
        if let Some((_, code)) = MORSE.iter().find(|(c, _)| *c == ch) {
            out.push_str(code);
            out.push('/');
        }
    }
    out
}

fn rot13(word: &str) -> String {
    word.chars()
        .map(|ch| {
            if ch.is_ascii_lowercase() {
                (((ch as u8 - b'a' + 13) % 26) + b'a') as char
            } else if ch.is_ascii_uppercase() {
                (((ch as u8 - b'A' + 13) % 26) + b'A') as char
            } else {
                ch
            }
        })
        .collect()
}

fn capitalize_first(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// First active getWord funbox produces the base word (or None).
pub fn get_word<R: Rng>(fbs: &[Funbox], rng: &mut R) -> Option<String> {
    let fb = fbs.iter().find(|f| f.is_get_word())?;
    Some(match fb {
        Funbox::Numbers58008 => get_numbers(rng, 7),
        Funbox::Gibberish => get_gibberish(rng),
        Funbox::Ascii => get_ascii(rng),
        Funbox::Specials => get_specials(rng),
        Funbox::Binary => get_binary(rng),
        Funbox::Hexadecimal => get_hexadecimal(rng),
        Funbox::Ipv4 => get_ipv4(rng),
        Funbox::Ipv6 => get_ipv6(rng),
        _ => unreachable!(),
    })
}

/// Apply every active alterText funbox, in declared order.
pub fn alter_all<R: Rng>(
    fbs: &[Funbox],
    word: String,
    word_index: usize,
    limit: usize,
    rng: &mut R,
) -> String {
    let mut w = word;
    for fb in fbs {
        w = match fb {
            Funbox::Capitals => capitalize_first(&w),
            Funbox::AllCaps => w.to_uppercase(),
            Funbox::RandomCase => w
                .chars()
                .map(|c| {
                    if rng.gen::<f64>() < 0.5 {
                        c.to_uppercase().to_string()
                    } else {
                        c.to_lowercase().to_string()
                    }
                })
                .collect(),
            Funbox::SpongeCase => w
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i % 2 == 0 {
                        c.to_lowercase().to_string()
                    } else {
                        c.to_uppercase().to_string()
                    }
                })
                .collect(),
            Funbox::Rot13 => rot13(&w),
            Funbox::Backwards => w.chars().rev().collect(),
            Funbox::Doubled => w.chars().flat_map(|c| [c, c]).collect(),
            Funbox::InstantMessaging => {
                let mut s = w.to_lowercase();
                s = s.replace(['(', ')', '.', '\'', '"'], "");
                s = s.replace(['!', '?'], "");
                s
            }
            Funbox::UnderscoreSpaces => {
                if word_index == limit.saturating_sub(1) {
                    w.clone()
                } else {
                    format!("{w}_")
                }
            }
            Funbox::Morse => to_morse(&w),
            _ => w.clone(),
        };
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(123)
    }

    #[test]
    fn parse_known_and_unknown() {
        let fbs = parse(&[
            "rot13".to_string(),
            "mirror".to_string(),
            "ALL_CAPS".to_string(),
        ]);
        assert_eq!(fbs, vec![Funbox::Rot13, Funbox::AllCaps]); // mirror dropped
    }

    #[test]
    fn rot13_is_reversible() {
        assert_eq!(rot13("hello"), "uryyb");
        assert_eq!(rot13(&rot13("monkeytype")), "monkeytype");
    }

    #[test]
    fn alter_transforms() {
        let mut r = rng();
        assert_eq!(
            alter_all(&[Funbox::AllCaps], "the".to_string(), 0, 10, &mut r),
            "THE"
        );
        assert_eq!(
            alter_all(&[Funbox::Backwards], "abc".to_string(), 0, 10, &mut r),
            "cba"
        );
        assert_eq!(
            alter_all(&[Funbox::Doubled], "ab".to_string(), 0, 10, &mut r),
            "aabb"
        );
        assert_eq!(
            alter_all(&[Funbox::Capitals], "word".to_string(), 0, 10, &mut r),
            "Word"
        );
        // underscore on non-last word, not on last
        assert_eq!(
            alter_all(&[Funbox::UnderscoreSpaces], "a".to_string(), 0, 5, &mut r),
            "a_"
        );
        assert_eq!(
            alter_all(&[Funbox::UnderscoreSpaces], "a".to_string(), 4, 5, &mut r),
            "a"
        );
    }

    #[test]
    fn spongecase_alternates() {
        let mut r = rng();
        assert_eq!(
            alter_all(&[Funbox::SpongeCase], "abcd".to_string(), 0, 10, &mut r),
            "aBcD"
        );
    }

    #[test]
    fn getword_funboxes_produce_output() {
        let mut r = rng();
        for fb in [
            Funbox::Numbers58008,
            Funbox::Gibberish,
            Funbox::Ascii,
            Funbox::Specials,
            Funbox::Binary,
            Funbox::Hexadecimal,
            Funbox::Ipv4,
            Funbox::Ipv6,
        ] {
            let w = get_word(&[fb], &mut r).unwrap();
            assert!(!w.is_empty(), "{} produced empty", fb.name());
        }
    }

    #[test]
    fn binary_is_8_bits() {
        let mut r = rng();
        let w = get_word(&[Funbox::Binary], &mut r).unwrap();
        assert_eq!(w.len(), 8);
        assert!(w.chars().all(|c| c == '0' || c == '1'));
    }

    #[test]
    fn ipv4_has_four_octets() {
        let mut r = rng();
        let w = get_word(&[Funbox::Ipv4], &mut r).unwrap();
        assert_eq!(w.split('.').count(), 4);
    }

    #[test]
    fn morse_converts() {
        assert_eq!(to_morse("sos"), ".../---/.../");
    }
}
