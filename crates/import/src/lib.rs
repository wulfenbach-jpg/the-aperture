//! Bounded Archidekt import for THE APERTURE.
//!
//! Security contract: only `https://archidekt.com/decks/<digits>` (and the
//! equivalent `/api/decks/<digits>` form), optionally followed by a single
//! non-authoritative human-readable slug, is accepted; the request URL is
//! rebuilt from the numeric deck id against an allowlisted host, never taken
//! from the pasted string; redirects are followed at most once and only to an
//! allowlisted host; the request has a timeout and the body is read through a
//! hard size cap.
//!
//! Support contract: Archidekt metadata is *recognized*, not *certified*.
//! `analyze` separates the practice-profile legality report from engine
//! support, and `plan_deck` refuses to launch anything containing a card the
//! engine does not implement.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub mod commander;
pub mod start;

use aperture_core::catalog;
use aperture_core::deck::DeckList;
use aperture_core::digest;

pub const ALLOWED_HOSTS: [&str; 2] = ["archidekt.com", "www.archidekt.com"];
pub const MAX_BODY_BYTES: usize = 8 * 1024 * 1024;
pub const REQUEST_TIMEOUT_SECS: u64 = 15;
pub const MAX_REDIRECTS: usize = 1;
pub const USER_AGENT: &str =
    "the-aperture/0.1 (local practice slice; Archidekt deck import; 1v1 practice profile)";
pub const PRACTICE_DECK_SIZE: usize = 40;
pub const MAX_COPIES: u8 = 4;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("'{input}' is not a usable Archidekt deck URL: {reason}")]
    BadUrl { input: String, reason: String },

    #[error("only https:// URLs are accepted (got '{scheme}')")]
    BadScheme { scheme: String },

    #[error("host '{host}' is not allowlisted (approved hosts: archidekt.com, www.archidekt.com)")]
    HostNotAllowed { host: String },

    #[error("URL credentials are not accepted")]
    UrlCredentials,

    #[error("unexpected port '{port}' in the URL")]
    BadPort { port: String },

    #[error("the URL must be a deck link of the form https://archidekt.com/decks/<digits>")]
    BadPath,

    #[error("'{value}' is not a numeric Archidekt deck id")]
    BadDeckId { value: String },

    #[error("the provider returned HTTP {status}{detail}")]
    Http { status: u16, detail: String },

    #[error("deck {deck_id} was not found: it may have been deleted or made private")]
    NotFound { deck_id: u64 },

    #[error("deck {deck_id} is private or requires authorization, so it cannot be imported")]
    Private { deck_id: u64 },

    #[error("the provider is rate limiting requests (HTTP 429); try again later")]
    RateLimited,

    #[error("the response body exceeded the {MAX_BODY_BYTES}-byte cap")]
    TooLarge,

    #[error("network failure: {0}")]
    Network(String),

    #[error("the provider response could not be parsed: {0}")]
    Parse(String),

    #[error("the deck cannot be launched: {0}")]
    NotLaunchable(String),

    #[error("cache error: {0}")]
    Cache(String),
}

// ---------------------------------------------------------------------- URL

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckRef {
    pub deck_id: u64,
    /// Canonical link form; the request itself is built by `api_url`.
    pub url: String,
}

pub fn host_allowed(host: &str) -> bool {
    ALLOWED_HOSTS
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(host))
}

pub fn parse_deck_url(input: &str) -> Result<DeckRef, ImportError> {
    let trimmed = input.trim();
    let bad = |reason: &str| ImportError::BadUrl {
        input: input.to_string(),
        reason: reason.to_string(),
    };
    if trimmed.is_empty() {
        return Err(bad("empty input"));
    }
    if trimmed.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(bad("whitespace or control characters are not allowed"));
    }
    let rest = match trimmed.split_once("://") {
        Some(("https", rest)) => rest,
        Some((scheme, _)) => {
            return Err(ImportError::BadScheme {
                scheme: scheme.to_string(),
            });
        }
        None => {
            return Err(ImportError::BadScheme {
                scheme: "(none)".to_string(),
            });
        }
    };
    let (authority, path) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, ""),
    };
    if authority.is_empty() {
        return Err(bad("no host"));
    }
    if authority.contains('@') {
        return Err(ImportError::UrlCredentials);
    }
    if authority.starts_with('[') {
        return Err(ImportError::HostNotAllowed {
            host: authority.to_string(),
        });
    }
    let (host, port) = match authority.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if let Some(port) = port
        && !port.is_empty()
        && port != "443"
    {
        return Err(ImportError::BadPort {
            port: port.to_string(),
        });
    }
    if !host_allowed(host) {
        return Err(ImportError::HostNotAllowed {
            host: host.to_string(),
        });
    }
    let path_only = path.split(['?', '#']).next().unwrap_or("");
    let segments: Vec<&str> = path_only
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    // `/decks/<id>` or `/api/decks/<id>`, optionally followed by a single
    // non-authoritative human-readable slug segment (`/decks/12345/temur-beats`).
    // The slug is ignored: the request URL is rebuilt from the numeric id only.
    let (id_text, slug) = match segments.as_slice() {
        ["decks", id] | ["api", "decks", id] => (*id, None),
        ["decks", id, slug] | ["api", "decks", id, slug] => (*id, Some(*slug)),
        _ => return Err(ImportError::BadPath),
    };
    if let Some(slug) = slug
        && (slug == "." || slug == ".." || slug.contains('%'))
    {
        return Err(ImportError::BadPath);
    }
    if id_text.is_empty() || !id_text.chars().all(|c| c.is_ascii_digit()) {
        return Err(ImportError::BadDeckId {
            value: id_text.to_string(),
        });
    }
    let deck_id: u64 = id_text.parse().map_err(|_| ImportError::BadDeckId {
        value: id_text.to_string(),
    })?;
    if deck_id == 0 {
        return Err(ImportError::BadDeckId {
            value: id_text.to_string(),
        });
    }
    Ok(DeckRef {
        deck_id,
        url: format!("https://{host}/decks/{deck_id}"),
    })
}

/// Canonical provider endpoint. Built from the numeric id only.
pub fn api_url(deck: &DeckRef) -> String {
    format!("https://archidekt.com/api/decks/{}/", deck.deck_id)
}

// -------------------------------------------------------------------- fetch

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fetch {
    pub deck_id: u64,
    pub url: String,
    pub fetched_at_unix: u64,
    pub sha256: String,
    pub raw_json: String,
    pub cached_path: Option<PathBuf>,
    pub from_cache: bool,
    pub note: Option<String>,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn http_client() -> Result<reqwest::blocking::Client, ImportError> {
    reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(5))
        .https_only(true)
        // Egress stays under this allowlist: no environment proxy may reroute it.
        .no_proxy()
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let host = attempt.url().host_str().unwrap_or("");
            if attempt.previous().len() >= MAX_REDIRECTS {
                attempt.error("redirect limit reached")
            } else if !host_allowed(host) {
                attempt.error("redirect target is not allowlisted")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|e| ImportError::Network(e.to_string()))
}

/// Perform one bounded HTTPS request for the deck JSON.
pub fn fetch_deck(deck: &DeckRef) -> Result<Fetch, ImportError> {
    let client = http_client()?;
    let url = api_url(deck);
    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .map_err(|e| {
            if e.is_redirect() {
                ImportError::Network(format!("refused redirect: {e}"))
            } else if e.is_timeout() {
                ImportError::Network(format!("request timed out after {REQUEST_TIMEOUT_SECS}s"))
            } else {
                ImportError::Network(e.to_string())
            }
        })?;
    let status = response.status();
    match status.as_u16() {
        200 => {}
        404 => {
            return Err(ImportError::NotFound {
                deck_id: deck.deck_id,
            });
        }
        401 | 403 => {
            return Err(ImportError::Private {
                deck_id: deck.deck_id,
            });
        }
        429 => return Err(ImportError::RateLimited),
        other => {
            return Err(ImportError::Http {
                status: other,
                detail: format!(" while fetching {}", deck.deck_id),
            });
        }
    }
    if let Some(length) = response.content_length()
        && length > MAX_BODY_BYTES as u64
    {
        return Err(ImportError::TooLarge);
    }
    let mut body = Vec::new();
    response
        .take(MAX_BODY_BYTES as u64 + 1)
        .read_to_end(&mut body)
        .map_err(|e| ImportError::Network(e.to_string()))?;
    if body.len() > MAX_BODY_BYTES {
        return Err(ImportError::TooLarge);
    }
    let raw_json = String::from_utf8(body)
        .map_err(|_| ImportError::Parse("response body is not UTF-8".to_string()))?;
    // A response that is not a JSON object is never treated as a deck.
    if !raw_json.trim_start().starts_with('{') {
        return Err(ImportError::Parse(
            "response is not a JSON object (provider error page?)".to_string(),
        ));
    }
    Ok(Fetch {
        deck_id: deck.deck_id,
        url,
        fetched_at_unix: now_unix(),
        sha256: digest::digest_bytes(raw_json.as_bytes()),
        raw_json,
        cached_path: None,
        from_cache: false,
        note: None,
    })
}

fn raw_cache_path(cache_dir: &Path, deck_id: u64) -> PathBuf {
    cache_dir.join(format!("archidekt-{deck_id}.json"))
}

fn normalized_cache_path(cache_dir: &Path, deck_id: u64) -> PathBuf {
    cache_dir.join(format!("archidekt-{deck_id}.normalized.json"))
}

fn report_cache_path(cache_dir: &Path, deck_id: u64) -> PathBuf {
    cache_dir.join(format!("archidekt-{deck_id}.report.json"))
}

/// Fetch with caching: always writes the raw response, and falls back to the
/// cached copy (with a visible note) when the provider is unreachable.
pub fn fetch_deck_cached(deck: &DeckRef, cache_dir: &Path) -> Result<Fetch, ImportError> {
    std::fs::create_dir_all(cache_dir).map_err(|e| ImportError::Cache(e.to_string()))?;
    let path = raw_cache_path(cache_dir, deck.deck_id);
    match fetch_deck(deck) {
        Ok(mut fetch) => {
            std::fs::write(&path, fetch.raw_json.as_bytes())
                .map_err(|e| ImportError::Cache(e.to_string()))?;
            fetch.cached_path = Some(path);
            Ok(fetch)
        }
        Err(err) => match std::fs::read_to_string(&path) {
            Ok(raw_json) => Ok(Fetch {
                deck_id: deck.deck_id,
                url: api_url(deck),
                fetched_at_unix: now_unix(),
                sha256: digest::digest_bytes(raw_json.as_bytes()),
                raw_json,
                cached_path: Some(path.clone()),
                from_cache: true,
                note: Some(format!(
                    "provider request failed ({err}); used the cached copy at {}",
                    path.display()
                )),
            }),
            Err(_) => Err(err),
        },
    }
}

// -------------------------------------------------------------- normalization

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormEntry {
    /// Provider card name; empty only when there is genuinely no card data.
    pub name: String,
    pub faces: Vec<String>,
    pub quantity: u8,
    pub categories: Vec<String>,
    pub companion: bool,
    pub deleted: bool,
    pub layout: String,
    /// Provider-declared color identity as single letters (`WUBRG`). Empty with
    /// `color_identity_known` set means the provider certified the card as
    /// colorless; empty with it unset means the provider sent no data at all.
    #[serde(default)]
    pub color_identity: Vec<String>,
    /// True when the provider sent a color-identity field for this entry.
    #[serde(default)]
    pub color_identity_known: bool,
    /// Provider-declared supertypes (`Legendary`, `Basic`, ...). Used only by
    /// the Commander construction checks in [`crate::start`].
    #[serde(default)]
    pub supertypes: Vec<String>,
    /// Provider-declared card types (`Creature`, `Planeswalker`, ...).
    #[serde(default)]
    pub types: Vec<String>,
}

/// Archidekt sends color-identity names (`"Red"`); normalize to letters so the
/// Commander color-identity check is a plain set comparison. Anything already
/// in letter form passes through, and an unrecognized token is kept verbatim so
/// it can never be mistaken for a color the deck actually has.
fn identity_letters(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(|item| {
                    let upper = item.trim().to_ascii_uppercase();
                    match upper.as_str() {
                        "WHITE" => "W".to_string(),
                        "BLUE" => "U".to_string(),
                        "BLACK" => "B".to_string(),
                        "RED" => "R".to_string(),
                        "GREEN" => "G".to_string(),
                        otherwise => otherwise.to_string(),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn plain_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|i| i.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedDeck {
    pub provider: String,
    pub source: String,
    pub deck_id: Option<u64>,
    pub name: String,
    pub format_code: Option<i64>,
    pub private: bool,
    pub unlisted: bool,
    pub theorycrafted: bool,
    pub updated_at: String,
    pub entries: Vec<NormEntry>,
    /// Categories the provider marks as not part of the deck (e.g. Maybeboard).
    pub excluded_categories: Vec<String>,
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|i| i.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub fn normalize(json: &str) -> Result<NormalizedDeck, ImportError> {
    let value: Value =
        serde_json::from_str(json).map_err(|e| ImportError::Parse(format!("invalid JSON: {e}")))?;
    let obj = value
        .as_object()
        .ok_or_else(|| ImportError::Parse("top level is not an object".to_string()))?;
    let deck_id = obj
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ImportError::Parse("response has no numeric deck id".to_string()))?;
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("(unnamed deck)")
        .to_string();
    let excluded_categories: Vec<String> = obj
        .get("categories")
        .and_then(|v| v.as_array())
        .map(|list| {
            list.iter()
                .filter(|c| c.get("includedInDeck").and_then(|v| v.as_bool()) == Some(false))
                .filter_map(|c| {
                    c.get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default();

    let mut entries = Vec::new();
    for (index, entry) in obj
        .get("cards")
        .and_then(|v| v.as_array())
        .map(|v| v.as_slice())
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        // Quantities are never silently clamped or skipped: a malformed count
        // is a hard parse error that names the entry and the offending value.
        let quantity_value = entry
            .get("quantity")
            .ok_or_else(|| ImportError::Parse(format!("card entry {index}: missing quantity")))?;
        let quantity = quantity_value.as_u64().ok_or_else(|| {
            ImportError::Parse(format!(
                "card entry {index}: quantity {quantity_value} is not an unsigned integer"
            ))
        })?;
        if quantity == 0 {
            return Err(ImportError::Parse(format!(
                "card entry {index}: zero quantity is not a valid card entry"
            )));
        }
        if quantity > u8::MAX as u64 {
            return Err(ImportError::Parse(format!(
                "card entry {index}: quantity {quantity} exceeds the supported 1..255 range"
            )));
        }
        let oracle = entry
            .get("card")
            .and_then(|c| c.get("oracleCard"))
            .cloned()
            .unwrap_or(Value::Null);
        // Face names come from objects inside the faces array.
        let face_names: Vec<String> = oracle
            .get("faces")
            .and_then(|v| v.as_array())
            .map(|faces| {
                faces
                    .iter()
                    .filter_map(|face| face.get("name").and_then(|n| n.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();
        let oracle_name = oracle
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        entries.push(NormEntry {
            name: if oracle_name.is_empty() {
                face_names.first().cloned().unwrap_or_default()
            } else {
                oracle_name
            },
            faces: face_names,
            quantity: quantity as u8,
            categories: string_list(entry.get("categories")),
            companion: entry
                .get("companion")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            deleted: entry
                .get("deletedAt")
                .map(|v| !v.is_null())
                .unwrap_or(false),
            layout: oracle
                .get("layout")
                .and_then(|v| v.as_str())
                .unwrap_or("normal")
                .to_string(),
            color_identity: identity_letters(oracle.get("colorIdentity")),
            color_identity_known: oracle.get("colorIdentity").is_some(),
            supertypes: plain_list(oracle.get("superTypes")),
            types: plain_list(oracle.get("types")),
        });
    }

    Ok(NormalizedDeck {
        provider: "archidekt".to_string(),
        source: format!("https://archidekt.com/decks/{deck_id}"),
        deck_id: Some(deck_id),
        name,
        format_code: obj.get("deckFormat").and_then(|v| v.as_i64()),
        private: obj
            .get("private")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        unlisted: obj
            .get("unlisted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        theorycrafted: obj
            .get("theorycrafted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        updated_at: obj
            .get("updatedAt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        entries,
        excluded_categories,
    })
}

// ----------------------------------------------------------------- analysis

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedCard {
    pub name: String,
    pub quantity: u8,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedCard {
    pub name: String,
    pub quantity: u8,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryStatus {
    Supported,
    Unsupported,
    Excluded,
    Sideboard,
    Commander,
    Companion,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryReport {
    pub name: String,
    pub quantity: u8,
    pub categories: Vec<String>,
    pub status: EntryStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckReport {
    pub source: String,
    pub provider: String,
    pub deck_id: Option<u64>,
    pub deck_name: String,
    pub format_code: Option<i64>,
    /// Main-deck cards, including ones we cannot identify.
    pub total_cards: u32,
    /// Main-deck cards whose name the provider supplied.
    pub recognized_cards: u32,
    pub supported_cards: u32,
    pub unsupported_cards: u32,
    pub entries: Vec<EntryReport>,
    pub unsupported: Vec<UnsupportedCard>,
    pub excluded: Vec<PlannedCard>,
    pub excluded_categories: Vec<String>,
    pub sideboard: Vec<PlannedCard>,
    pub commander: Vec<String>,
    pub companion: Vec<String>,
    pub legality: Vec<String>,
    pub launchable: bool,
    pub refusal: Option<String>,
    pub raw_sha256: Option<String>,
    pub cached_path: Option<PathBuf>,
    pub notes: Vec<String>,
}

pub(crate) fn classify(entry: &NormEntry, excluded: &[String]) -> (EntryStatus, Option<String>) {
    let lower: Vec<String> = entry.categories.iter().map(|c| c.to_lowercase()).collect();
    if entry.deleted {
        return (
            EntryStatus::Deleted,
            Some("entry is marked deleted".to_string()),
        );
    }
    if lower.iter().any(|c| c.starts_with("commander")) {
        return (EntryStatus::Commander, None);
    }
    if lower.iter().any(|c| c == "companion") || entry.companion {
        return (EntryStatus::Companion, None);
    }
    if lower.iter().any(|c| c == "sideboard") {
        return (EntryStatus::Sideboard, None);
    }
    if let Some(name) = entry
        .categories
        .iter()
        .find(|c| excluded.iter().any(|e| e.eq_ignore_ascii_case(c)))
    {
        return (
            EntryStatus::Excluded,
            Some(format!("category '{name}' is excluded from the deck")),
        );
    }
    if entry.name.is_empty() {
        return (
            EntryStatus::Unsupported,
            Some("provider sent no card data: cannot certify support".to_string()),
        );
    }
    if entry.faces.len() > 1 || !entry.layout.eq_ignore_ascii_case("normal") {
        return (
            EntryStatus::Unsupported,
            Some(format!(
                "multi-face card (layout '{}, faces: {}): not modeled",
                entry.layout,
                entry.faces.join(" / ")
            )),
        );
    }
    match catalog::by_name(&entry.name) {
        Some(card) if card.is_supported_commander() => (
            EntryStatus::Unsupported,
            Some(format!(
                "'{}' is commander-only in the command-zone practice profile; it cannot be certified as an ordinary practice library card (the legend rule is not modeled)",
                entry.name
            )),
        ),
        Some(_) => (EntryStatus::Supported, None),
        None => (
            EntryStatus::Unsupported,
            Some(format!(
                "'{}' is not in the supported pool ({} cards)",
                entry.name,
                catalog::pool_size()
            )),
        ),
    }
}

struct Bucket {
    name: String,
    quantity: u8,
    categories: Vec<String>,
    detail: Option<String>,
}

impl Bucket {
    fn detail_reason(&self) -> String {
        self.detail
            .clone()
            .unwrap_or_else(|| "unsupported".to_string())
    }
}

fn push_merged(
    buckets: &mut Vec<Bucket>,
    name: &str,
    quantity: u8,
    categories: &[String],
    detail: Option<String>,
) {
    if let Some(found) = buckets.iter_mut().find(|b| b.name == name) {
        found.quantity = found.quantity.saturating_add(quantity);
        for category in categories {
            if !found.categories.contains(category) {
                found.categories.push(category.clone());
            }
        }
        if found.detail.is_none() {
            found.detail = detail;
        }
    } else {
        buckets.push(Bucket {
            name: name.to_string(),
            quantity,
            categories: categories.to_vec(),
            detail,
        });
    }
}

pub fn analyze(deck: &NormalizedDeck) -> DeckReport {
    let mut entries = Vec::new();
    let mut supported: Vec<Bucket> = Vec::new();
    let mut unsupported: Vec<Bucket> = Vec::new();
    let mut excluded: Vec<Bucket> = Vec::new();
    let mut sideboard: Vec<Bucket> = Vec::new();
    let mut commander: Vec<Bucket> = Vec::new();
    let mut companion: Vec<Bucket> = Vec::new();
    let mut total = 0u32;
    let mut recognized = 0u32;
    let mut supported_cards = 0u32;
    let mut unnamed = 0u32;

    for entry in &deck.entries {
        let (status, detail) = classify(entry, &deck.excluded_categories);
        let label = if entry.name.is_empty() {
            unnamed += entry.quantity as u32;
            format!("(no card data #{})", entries.len() + 1)
        } else {
            entry.name.clone()
        };
        entries.push(EntryReport {
            name: label.clone(),
            quantity: entry.quantity,
            categories: entry.categories.clone(),
            status,
            detail: detail.clone(),
        });
        match status {
            EntryStatus::Supported => {
                total += entry.quantity as u32;
                recognized += entry.quantity as u32;
                supported_cards += entry.quantity as u32;
                push_merged(
                    &mut supported,
                    &entry.name,
                    entry.quantity,
                    &entry.categories,
                    None,
                );
            }
            EntryStatus::Unsupported => {
                total += entry.quantity as u32;
                if !entry.name.is_empty() {
                    recognized += entry.quantity as u32;
                }
                push_merged(
                    &mut unsupported,
                    &label,
                    entry.quantity,
                    &entry.categories,
                    detail,
                );
            }
            EntryStatus::Excluded | EntryStatus::Deleted => push_merged(
                &mut excluded,
                &label,
                entry.quantity,
                &entry.categories,
                detail,
            ),
            EntryStatus::Sideboard => push_merged(
                &mut sideboard,
                &label,
                entry.quantity,
                &entry.categories,
                None,
            ),
            EntryStatus::Commander => push_merged(
                &mut commander,
                &label,
                entry.quantity,
                &entry.categories,
                None,
            ),
            EntryStatus::Companion => push_merged(
                &mut companion,
                &label,
                entry.quantity,
                &entry.categories,
                None,
            ),
        }
    }

    let unsupported_cards = total - supported_cards;
    let mut legality = Vec::new();
    legality.push(format!(
        "practice profile: exactly {PRACTICE_DECK_SIZE} main-deck cards (found {total})"
    ));
    legality.push(format!(
        "at most {MAX_COPIES} copies of each nonbasic card (basics unlimited)"
    ));
    legality.push(format!(
        "every card must be in the supported pool ({} cards)",
        catalog::pool_size()
    ));
    legality.push("1v1, 20 life, London mulligan: Commander is not implemented".to_string());

    let mut refusal = None;
    if deck.private {
        refusal = Some(format!(
            "deck {} is private: Archidekt does not serve private deck data without authorization",
            deck.deck_id.map(|id| id.to_string()).unwrap_or_default()
        ));
    } else if unsupported_cards > 0 {
        let names: Vec<String> = unsupported
            .iter()
            .take(3)
            .map(|u| format!("{} x{} ({})", u.name, u.quantity, u.detail_reason()))
            .collect();
        refusal = Some(format!(
            "unsupported card(s) in the main deck: {}{} — the supported pool has {} cards; unsupported cards are refused, never substituted",
            names.join("; "),
            if unsupported.len() > 3 {
                format!(" (+{} more)", unsupported.len() - 3)
            } else {
                String::new()
            },
            catalog::pool_size()
        ));
    } else if !commander.is_empty() {
        refusal = Some(format!(
            "Commander is not available in this slice (1v1 practice profile): {} listed as commander",
            commander
                .iter()
                .map(|c| c.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    } else if total as usize != PRACTICE_DECK_SIZE {
        refusal = Some(format!(
            "the practice profile requires exactly {PRACTICE_DECK_SIZE} main-deck cards; this deck has {total}"
        ));
    } else {
        let lands: u32 = supported
            .iter()
            .filter(|b| {
                catalog::by_name(&b.name)
                    .map(|id| id.def().is_land())
                    .unwrap_or(false)
            })
            .map(|b| b.quantity as u32)
            .sum();
        if lands == 0 {
            refusal = Some(
                "the deck has no lands: the practice profile needs a working mana base".to_string(),
            );
        } else {
            if let Some(over) = supported.iter().find(|b| {
                catalog::by_name(&b.name)
                    .map(|id| !id.def().is_land())
                    .unwrap_or(false)
                    && b.quantity > MAX_COPIES
            }) {
                refusal = Some(format!(
                    "more than {MAX_COPIES} copies of {} (x{}) breaks the practice profile",
                    over.name, over.quantity
                ));
            }
        }
    }

    let mut notes = Vec::new();
    if unnamed > 0 {
        notes.push(format!(
            "{unnamed} card(s) arrived without provider card data and are listed as unsupported rather than guessed"
        ));
    }
    if !deck.excluded_categories.is_empty() {
        notes.push(format!(
            "excluded categories honored: {}",
            deck.excluded_categories.join(", ")
        ));
    }
    if !companion.is_empty() {
        notes.push(
            "companion entries are reported separately and never added to the deck".to_string(),
        );
    }
    if deck.format_code.is_some() {
        notes.push(format!(
            "provider deck format code {:?} is recorded as metadata; only the 1v1 practice profile is validated",
            deck.format_code
        ));
    }

    DeckReport {
        source: deck.source.clone(),
        provider: deck.provider.clone(),
        deck_id: deck.deck_id,
        deck_name: deck.name.clone(),
        format_code: deck.format_code,
        total_cards: total,
        recognized_cards: recognized,
        supported_cards,
        unsupported_cards,
        entries,
        unsupported: unsupported
            .into_iter()
            .map(|b| UnsupportedCard {
                name: b.name,
                quantity: b.quantity,
                reason: b.detail.unwrap_or_default(),
            })
            .collect(),
        excluded: excluded
            .into_iter()
            .map(|b| PlannedCard {
                name: b.name,
                quantity: b.quantity,
                categories: b.categories,
            })
            .collect(),
        excluded_categories: deck.excluded_categories.clone(),
        sideboard: sideboard
            .into_iter()
            .map(|b| PlannedCard {
                name: b.name,
                quantity: b.quantity,
                categories: b.categories,
            })
            .collect(),
        commander: commander.into_iter().map(|b| b.name).collect(),
        companion: companion.into_iter().map(|b| b.name).collect(),
        legality,
        launchable: refusal.is_none(),
        refusal,
        raw_sha256: None,
        cached_path: None,
        notes,
    }
}

/// Build the playable deck list, or refuse with the recorded explanation.
pub fn plan_deck(report: &DeckReport) -> Result<DeckList, ImportError> {
    if let Some(refusal) = &report.refusal {
        return Err(ImportError::NotLaunchable(refusal.clone()));
    }
    let mut cards: Vec<(aperture_core::CardId, u8)> = Vec::new();
    for entry in &report.entries {
        if entry.status != EntryStatus::Supported {
            continue;
        }
        let Some(id) = catalog::by_name(&entry.name) else {
            return Err(ImportError::NotLaunchable(format!(
                "'{}' was reported as supported but is not in the pool",
                entry.name
            )));
        };
        if id.is_supported_commander() {
            return Err(ImportError::NotLaunchable(format!(
                "'{}' is commander-only; it cannot be launched as an ordinary practice deck",
                entry.name
            )));
        }
        match cards.iter_mut().find(|(c, _)| *c == id) {
            Some((_, quantity)) => *quantity = quantity.saturating_add(entry.quantity),
            None => cards.push((id, entry.quantity)),
        }
    }
    if cards.is_empty() {
        return Err(ImportError::NotLaunchable(
            "the deck has no supported cards".to_string(),
        ));
    }
    Ok(DeckList::new(report.deck_name.clone(), cards))
}

// --------------------------------------------------------------- text import

/// Text fallback analysis: `N Card Name` lines grouped by `Deck`, `Sideboard`,
/// `Commander`, `Maybeboard` sections. Real counts only; nothing is inferred.
pub fn text_report(name: &str, text: &str) -> Result<DeckReport, ImportError> {
    let deck = normalize_text(name, text)?;
    let report = analyze(&deck);
    Ok(report)
}

/// Normalize a pasted list into the same shape a provider response produces, so
/// the start-flow certification runs one code path for every source. The text
/// form carries no provider card data, so color identity and type facts stay
/// empty and the Commander construction checks report `not_certified` instead of
/// guessing.
pub(crate) fn normalize_text(name: &str, text: &str) -> Result<NormalizedDeck, ImportError> {
    let mut entries: Vec<NormEntry> = Vec::new();
    let mut section = "Deck".to_string();
    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
            continue;
        }
        let normalized_header = line.trim_end_matches(':').trim().to_lowercase();
        let is_header = !line
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false);
        if is_header {
            match normalized_header.as_str() {
                "deck" | "mainboard" | "main" | "creatures" | "spells" | "lands" => {
                    section = "Deck".to_string();
                }
                "sideboard" => section = "Sideboard".to_string(),
                "commander" | "commanders" | "command zone" => section = "Commander".to_string(),
                "maybeboard" | "maybe" => section = "Maybeboard".to_string(),
                "companion" => section = "Companion".to_string(),
                _ => {
                    // Unknown grouping line: ignore it, it carries no counts.
                    continue;
                }
            }
            continue;
        }
        let mut parts = line.split_whitespace();
        let quantity: u8 = parts
            .next()
            .ok_or_else(|| ImportError::Parse(format!("line {}: empty", lineno + 1)))?
            .parse()
            .map_err(|_| {
                ImportError::Parse(format!(
                    "line {}: expected a quantity in '{line}'",
                    lineno + 1
                ))
            })?;
        let card_name = parts.collect::<Vec<_>>().join(" ");
        if card_name.is_empty() {
            return Err(ImportError::Parse(format!(
                "line {}: missing card name",
                lineno + 1
            )));
        }
        entries.push(NormEntry {
            name: card_name,
            faces: vec![],
            quantity,
            categories: vec![section.clone()],
            companion: section == "Companion",
            deleted: false,
            layout: "normal".to_string(),
            color_identity: Vec::new(),
            color_identity_known: false,
            supertypes: Vec::new(),
            types: Vec::new(),
        });
    }
    if entries.is_empty() {
        return Err(ImportError::Parse(
            "the list contains no card lines".to_string(),
        ));
    }
    // The text format names the card directly, so faces are populated from the
    // name; an unknown name is reported as unsupported by `analyze`.
    for entry in entries.iter_mut() {
        entry.faces = vec![entry.name.clone()];
    }
    let deck = NormalizedDeck {
        provider: "text".to_string(),
        source: format!("text list '{name}'"),
        deck_id: None,
        name: name.to_string(),
        format_code: None,
        private: false,
        unlisted: false,
        theorycrafted: false,
        updated_at: String::new(),
        entries,
        excluded_categories: vec!["Maybeboard".to_string()],
    };
    Ok(deck)
}

/// Text import for play: analysis plus strict refusal.
pub fn import_text(name: &str, text: &str) -> Result<(DeckList, DeckReport), ImportError> {
    let report = text_report(name, text)?;
    let list = plan_deck(&report)?;
    Ok((list, report))
}

// ------------------------------------------------------------------- one-shot

/// Fetch, cache, normalize and analyze a pasted deck URL.
pub fn import_url(url: &str, cache_dir: &Path) -> Result<DeckReport, ImportError> {
    let deck_ref = parse_deck_url(url)?;
    let mut fetch = fetch_deck_cached(&deck_ref, cache_dir)?;
    let mut normalized = normalize(&fetch.raw_json)?;
    if let Some(id) = normalized.deck_id
        && id != deck_ref.deck_id
    {
        return Err(ImportError::Parse(format!(
            "provider returned deck id {id} for a request for {}",
            deck_ref.deck_id
        )));
    }
    normalized.source = deck_ref.url.clone();
    let mut report = analyze(&normalized);
    report.raw_sha256 = Some(fetch.sha256.clone());
    report.cached_path = fetch.cached_path.clone();
    if let Some(note) = fetch.note.take() {
        report.notes.push(note);
    }
    // Cache the normalized form and the report beside the raw response.
    let normalized_path = normalized_cache_path(cache_dir, deck_ref.deck_id);
    std::fs::write(
        &normalized_path,
        serde_json::to_vec_pretty(&normalized).unwrap_or_default(),
    )
    .map_err(|e| ImportError::Cache(e.to_string()))?;
    report.notes.push(format!(
        "normalized deck cached at {}",
        normalized_path.display()
    ));
    let report_path = report_cache_path(cache_dir, deck_ref.deck_id);
    std::fs::write(
        &report_path,
        serde_json::to_vec_pretty(&report).unwrap_or_default(),
    )
    .map_err(|e| ImportError::Cache(e.to_string()))?;
    report
        .notes
        .push(format!("report cached at {}", report_path.display()));
    Ok(report)
}

/// Fetch a URL and launch it, refusing anything not fully supported.
pub fn import_url_for_play(
    url: &str,
    cache_dir: &Path,
) -> Result<(DeckList, DeckReport), ImportError> {
    let report = import_url(url, cache_dir)?;
    let list = plan_deck(&report)?;
    Ok((list, report))
}
