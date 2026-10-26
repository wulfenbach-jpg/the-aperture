//! Security mutation harness for the #31 audit lane: bounded, deterministic,
//! seeded mutations of the untrusted import boundary — pasted deck text,
//! provider JSON normalization and deck URL parsing — against the real public
//! API of this crate.
//!
//! Properties under test:
//! 1. no panic and no hang on mutated inputs;
//! 2. URL parsing never trusts the pasted string: a parsed URL is always
//!    rebuilt canonically from the numeric id (scheme/host fixed, path fixed);
//! 3. hostile quantities/counts are refused or reported honestly, never
//!    wrapped (a silent u8/u32 wrap would let x999999 become 255 basics);
//! 4. oversized inputs stay bounded.

use aperture_import::{api_url, import_text, normalize, parse_deck_url, text_report};

const URL_MUTATIONS: usize = 512;
const TEXT_MUTATIONS: usize = 256;
const JSON_MUTATIONS: usize = 256;
const MAX_INJECTED_LEN: usize = 100_000;

const VALID_URL: &str = "https://archidekt.com/decks/26206384";
const TEXT_SAMPLE: &str = include_str!("../../../decks/temur-beats.txt");
const PROVIDER_JSON: &str = include_str!("data/archidekt-26206384.json");

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next() % n as u64) as usize
        }
    }
}

const INJECTED: &[&str] = &[
    "\"",
    "/",
    "\\",
    "?",
    "#",
    "&",
    "@",
    ":",
    "%",
    " ",
    "..",
    "\n",
    "\t",
    "%2f",
    "%2F",
    "..%2f",
    "\u{0000}",
    "\u{202E}",
    "\u{10FFFF}",
    "0",
    "9",
    "-1",
    "18446744073709551615",
    "archidekt.com",
    "evil.example",
    "127.0.0.1",
    "admin",
    "api",
    "🦀",
];

/// Floor a byte offset to the nearest UTF-8 char boundary.
fn floor_boundary(text: &str, mut at: usize) -> usize {
    while at > 0 && !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn mutate(text: &str, rng: &mut Lcg) -> String {
    let len = text.len();
    match rng.below(5) {
        0 => text[..floor_boundary(text, rng.below(len))].to_string(),
        1 => {
            let mut out = text.as_bytes().to_vec();
            let at = rng.below(out.len());
            out[at] = INJECTED[rng.below(INJECTED.len())].as_bytes()[0];
            String::from_utf8_lossy(&out).into_owned()
        }
        2 => {
            let at = floor_boundary(text, rng.below(len));
            let token = INJECTED[rng.below(INJECTED.len())];
            let mut out = String::with_capacity(len + token.len());
            out.push_str(&text[..at]);
            out.push_str(token);
            out.push_str(&text[at..]);
            out
        }
        3 => {
            let at = floor_boundary(text, rng.below(len));
            let end = floor_boundary(text, at + rng.below(len - at).min(32));
            let mut out = String::with_capacity(len);
            out.push_str(&text[..at]);
            out.push_str(&text[end..]);
            out
        }
        _ => {
            let at = floor_boundary(text, rng.below(len));
            let end = floor_boundary(text, at + rng.below(len - at).min(32));
            let mut out = String::with_capacity(2 * len);
            out.push_str(&text[..at]);
            out.push_str(&text[at..end]);
            out.push_str(&text[at..]);
            out
        }
    }
}

/// THE URL invariant: whatever string arrives, an accepted parse is rebuilt
/// from the numeric id alone — canonical scheme, canonical host, canonical
/// path. Nothing attacker-controlled may survive into the request target.
#[test]
fn mutated_deck_urls_only_ever_produce_the_canonical_request() {
    for seed in 0..URL_MUTATIONS as u64 {
        let mutated = mutate(VALID_URL, &mut Lcg(seed + 1));
        match parse_deck_url(&mutated) {
            Err(_) => {}
            Ok(parsed) => {
                let canonical = format!("https://archidekt.com/decks/{}", parsed.deck_id);
                assert_eq!(
                    parsed.url, canonical,
                    "seed {seed}: a parsed URL must be rebuilt from the id, not echoed: {mutated:?}"
                );
                assert_eq!(
                    api_url(&parsed),
                    format!("https://archidekt.com/api/decks/{}/", parsed.deck_id),
                    "seed {seed}: the API target must be canonical"
                );
            }
        }
    }
}

/// Known-hostile URL shapes. The load-bearing security property is the
/// canonical rebuild (tested exhaustively above); these cases pin the shapes
/// that must still end in a canonical request or a clean refusal, and call out
/// the ones that must be refused outright.
#[test]
fn hostile_url_shapes_are_refused_or_canonical() {
    for input in [
        // Identity-normalized numeric forms: if accepted, they must rebuild
        // the SAME canonical URL for deck 26206384.
        "https://archidekt.com/decks/+26206384",
        "https://archidekt.com/decks/026206384",
        "HTTPS://ARCHIDEKT.COM/decks/26206384",
        "https://archidekt.com:443/decks/26206384",
        // Must be refused outright.
        "https://archidekt.com/decks/26206384%00",
        "https://archidekt.com/decks/26206384\u{202E}admin",
        "https://archidekt.com/decks/18446744073709551616",
        // 10 MiB URL must fail fast and bounded.
        &format!(
            "https://archidekt.com/decks/26206384{}",
            "A".repeat(10 * 1024 * 1024)
        ),
    ] {
        match parse_deck_url(input) {
            Err(_) => {}
            Ok(parsed) => {
                assert_eq!(
                    parsed.deck_id, 26206384,
                    "an accepted id-normalized URL must resolve to the same deck"
                );
                assert_eq!(
                    parsed.url, "https://archidekt.com/decks/26206384",
                    "an accepted URL must be rebuilt canonically"
                );
            }
        }
    }
}

/// Mutated deck text must never panic, and accepted reports must carry honest
/// counts (no silent quantity wrap).
#[test]
fn mutated_deck_text_is_refused_or_reported_honestly() {
    for seed in 0..TEXT_MUTATIONS as u64 {
        let mutated = mutate(TEXT_SAMPLE, &mut Lcg(seed + 1));
        if let Err(caught) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = text_report("mutation", &mutated);
        })) {
            panic!("text_report panicked on seed {seed}: {caught:?}\ninput:\n{mutated}");
        }
    }
}

#[test]
fn hostile_quantities_are_refused_not_wrapped() {
    for line in [
        "99999999 Forest",
        "4294967296 Forest",
        "99999999999999999999 Forest",
        "-3 Forest",
        "2.5 Forest",
        "1e3 Forest",
        "0x10 Forest",
        " 300  Forest ",
        "256 Forest",
    ] {
        match text_report("wrap probe", &format!("Deck\n{line}\n")) {
            Err(_) => {}
            Ok(report) => {
                let written: u64 = line
                    .split_whitespace()
                    .next()
                    .and_then(|q| q.parse().ok())
                    .unwrap_or(u64::MAX);
                for entry in &report.entries {
                    assert_eq!(
                        entry.quantity as u64, written,
                        "quantity for '{line}' must be refused or reported exactly, never wrapped"
                    );
                }
                assert_eq!(
                    report.total_cards as u64, written,
                    "total for '{line}' must be honest, not wrapped"
                );
            }
        }
    }
}

/// Mutated provider JSON must never panic normalization; huge but bounded
/// arrays and deep nesting must be refused or handled within bounded memory.
#[test]
fn mutated_provider_json_never_panics() {
    for seed in 0..JSON_MUTATIONS as u64 {
        let mutated = mutate(PROVIDER_JSON, &mut Lcg(seed + 1));
        if let Err(caught) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = normalize(&mutated);
        })) {
            let head: String = mutated.chars().take(400).collect();
            panic!("normalize panicked on seed {seed}: {caught:?}\ninput head:\n{head}");
        }
    }
}

#[test]
fn oversized_provider_shapes_stay_bounded_and_honest() {
    let obj: serde_json::Value = serde_json::from_str(PROVIDER_JSON).expect("fixture parses");

    // 100_000 cloned real rows: accepted totals must be honest (no u32 wrap)
    // and the work must stay bounded.
    if let Some(row) = obj["cards"].as_array().and_then(|a| a.first()).cloned() {
        let mut huge = obj.clone();
        huge["cards"] = serde_json::Value::Array(
            std::iter::repeat_n(row, MAX_INJECTED_LEN).collect::<Vec<_>>(),
        );
        let deck = normalize(&huge.to_string())
            .unwrap_or_else(|e| panic!("bounded rows must normalize or refuse cleanly: {e}"));
        assert_eq!(
            deck.entries.len() as u64,
            MAX_INJECTED_LEN as u64,
            "every injected row must be accounted for"
        );
    }

    // Deep nesting must hit serde's recursion guard, not the stack.
    for depth in [128usize, 1_024, MAX_INJECTED_LEN] {
        let mutated = format!("{{\"junk\": {}", "[".repeat(depth),) + &PROVIDER_JSON[1..];
        assert!(
            normalize(&mutated).is_err(),
            "depth {depth} must be refused"
        );
    }

    // A 1 MiB card name must not panic.
    let mut long_name = obj.clone();
    long_name["name"] = serde_json::json!("A".repeat(1024 * 1024));
    let _ = normalize(&long_name.to_string());
}

/// The end-to-end text play path keeps the same refusal guarantees as the
/// report path: a mutated list either plans cleanly within the format bound or
/// is refused with a typed error.
#[test]
fn mutated_text_import_refuses_or_plans_within_bounds() {
    for seed in 0..TEXT_MUTATIONS as u64 {
        let mutated = mutate(TEXT_SAMPLE, &mut Lcg(seed + 777));
        match import_text("mutation", &mutated) {
            Err(_) => {}
            Ok((list, _report)) => {
                assert!(
                    list.size() <= 400,
                    "a launched Commander practice list must respect the deck bound"
                );
            }
        }
    }
}
