//! Offline Commander format-legality analysis (analysis only).
//!
//! Parses the two versioned offline snapshots, binds the recorded digests to
//! the exact embedded bytes, and applies the documented rule subset. It never
//! constructs a playable deck, never calls the practice planner, and never
//! starts a game. Unknown, incomplete, ambiguous or unsupported metadata is
//! `not_certified`, never guessed `legal`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const DECK_SCHEMA: &str = "aperture.commander-deck.v1";
pub const CARD_METADATA_SCHEMA: &str = "aperture.card-metadata.v1";
pub const REPORT_SCHEMA: &str = "aperture.commander-report.v1";

#[derive(Debug, Error)]
pub enum CommanderError {
    #[error("cannot read {path}: {detail}")]
    Io { path: String, detail: String },
    #[error("invalid JSON: {0}")]
    Json(String),
    #[error("invalid Commander input: {0}")]
    Input(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegalityStatus {
    Legal,
    Illegal,
    NotCertified,
}

impl LegalityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            LegalityStatus::Legal => "legal",
            LegalityStatus::Illegal => "illegal",
            LegalityStatus::NotCertified => "not_certified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportStatus {
    Supported,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub code: String,
    pub status: LegalityStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckSummary {
    pub commanders: u64,
    pub library: u64,
    pub total: u64,
    pub unique_canonical_names: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderEntryReport {
    pub name: String,
    pub oracle_id: String,
    pub color_identity: Vec<String>,
    pub layout: Option<String>,
    pub eligibility: LegalityStatus,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatLegality {
    pub status: LegalityStatus,
    pub as_of: String,
    pub age_days: u64,
    pub stale: bool,
    pub snapshot_sha256: String,
    pub basis: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedSupportCard {
    pub name: String,
    pub oracle_id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineSupport {
    pub status: SupportStatus,
    pub unsupported_cards: Vec<UnsupportedSupportCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameplayReport {
    pub available: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardFaceEvidence {
    pub name: String,
    pub type_line: String,
    pub colors: Vec<String>,
    pub color_indicator: Vec<String>,
    pub mana_cost: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardEvidence {
    pub name: String,
    pub oracle_id: Option<String>,
    pub layout: Option<String>,
    pub top_level_color_identity: Vec<String>,
    pub faces: Vec<CardFaceEvidence>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportDeckProvenance {
    pub kind: String,
    pub provider: String,
    pub source: String,
    pub captured_at: String,
    pub raw_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportCardProvenance {
    pub kind: String,
    pub provider: String,
    pub api_url: Option<String>,
    pub download_url: Option<String>,
    pub bulk_type: Option<String>,
    pub captured_at: String,
    pub cards_sha256: String,
    pub raw_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportProvenance {
    pub deck: ReportDeckProvenance,
    pub cards: ReportCardProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommanderReport {
    pub schema: String,
    pub status: LegalityStatus,
    pub deck: DeckSummary,
    pub commanders: Vec<CommanderEntryReport>,
    pub format_legality: FormatLegality,
    pub engine_support: EngineSupport,
    pub gameplay: GameplayReport,
    pub findings: Vec<Finding>,
    pub card_evidence: Vec<CardEvidence>,
    pub provenance: ReportProvenance,
}

impl CommanderReport {
    pub fn finding(&self, code: &str) -> Option<&Finding> {
        self.findings.iter().find(|finding| finding.code == code)
    }

    pub fn has_finding(&self, code: &str, status: LegalityStatus) -> bool {
        self.findings
            .iter()
            .any(|finding| finding.code == code && finding.status == status)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeckSnapshotFile {
    schema: String,
    provenance: DeckProvenance,
    raw_deck: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeckProvenance {
    kind: String,
    provider: String,
    source: String,
    captured_at: String,
    raw_sha256: String,
    #[serde(default)]
    #[allow(dead_code)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDeck {
    commanders: Vec<DeckEntry>,
    library: Vec<DeckEntry>,
    #[serde(default)]
    companions: Vec<DeckEntry>,
    #[serde(default)]
    sideboard: Vec<DeckEntry>,
    #[serde(default)]
    excluded: Vec<DeckEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeckEntry {
    name: String,
    oracle_id: String,
    quantity: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CardsSnapshotFile {
    schema: String,
    provenance: CardProvenance,
    raw_cards_jsonl: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CardProvenance {
    kind: String,
    provider: String,
    #[serde(default)]
    api_url: Option<String>,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    bulk_type: Option<String>,
    #[serde(default)]
    api_updated_at: Option<String>,
    #[serde(default)]
    source: Option<String>,
    captured_at: String,
    cards_sha256: String,
    raw_sha256: String,
    #[serde(default)]
    #[allow(dead_code)]
    selection: Option<String>,
    #[serde(default)]
    card_count: Option<usize>,
    #[serde(default)]
    #[allow(dead_code)]
    note: Option<String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_hex64(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn https_host(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let authority = rest.split('/').next()?;
    if authority.is_empty() || authority.contains('@') || authority.starts_with('[') {
        return None;
    }
    if let Some((host, port)) = authority.split_once(':') {
        if port != "443" {
            return None;
        }
        return Some(host);
    }
    Some(authority)
}

fn parse_deck_snapshot(json: &str) -> Result<(DeckProvenance, RawDeck), CommanderError> {
    let file: DeckSnapshotFile =
        serde_json::from_str(json).map_err(|e| CommanderError::Json(e.to_string()))?;
    if file.schema != DECK_SCHEMA {
        return Err(CommanderError::Input(format!(
            "deck snapshot schema must be '{DECK_SCHEMA}', got '{}'",
            file.schema
        )));
    }
    let p = &file.provenance;
    for (field, value) in [
        ("kind", p.kind.as_str()),
        ("provider", p.provider.as_str()),
        ("source", p.source.as_str()),
        ("captured_at", p.captured_at.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(CommanderError::Input(format!(
                "deck provenance field '{field}' must not be empty"
            )));
        }
    }
    if !matches!(p.kind.as_str(), "real" | "constructed" | "synthetic") {
        return Err(CommanderError::Input(format!(
            "deck provenance kind must be real, constructed or synthetic; got '{}'",
            p.kind
        )));
    }
    if !valid_hex64(&p.raw_sha256) {
        return Err(CommanderError::Input(
            "deck provenance raw_sha256 must be exactly 64 hex characters".to_string(),
        ));
    }
    let actual = sha256_hex(file.raw_deck.as_bytes());
    if !actual.eq_ignore_ascii_case(&p.raw_sha256) {
        return Err(CommanderError::Input(format!(
            "deck raw_sha256 does not match the exact raw_deck bytes (computed {actual})"
        )));
    }
    parse_rfc3339(&p.captured_at).ok_or_else(|| {
        CommanderError::Input(format!(
            "deck captured_at '{}' is not an RFC 3339 timestamp",
            p.captured_at
        ))
    })?;
    let zones: RawDeck = serde_json::from_str(&file.raw_deck).map_err(|e| {
        CommanderError::Input(format!("raw_deck is not the expected zone object: {e}"))
    })?;
    for (zone, entries) in [
        ("commanders", &zones.commanders),
        ("library", &zones.library),
        ("companions", &zones.companions),
        ("sideboard", &zones.sideboard),
        ("excluded", &zones.excluded),
    ] {
        for (index, entry) in entries.iter().enumerate() {
            if entry.name.trim().is_empty() {
                return Err(CommanderError::Input(format!(
                    "{zone}[{index}] has an empty name"
                )));
            }
            if entry.oracle_id.trim().is_empty() {
                return Err(CommanderError::Input(format!(
                    "{zone}[{index}] ('{}') has an empty oracle_id",
                    entry.name
                )));
            }
            if !(1..=255).contains(&entry.quantity) {
                return Err(CommanderError::Input(format!(
                    "{zone}[{index}] ('{}') quantity {} is outside 1..=255",
                    entry.name, entry.quantity
                )));
            }
        }
    }
    Ok((p.clone(), zones))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommanderLegality {
    Legal,
    NotLegal,
    Banned,
    Restricted,
}

#[derive(Debug, Clone)]
struct CardMeta {
    oracle_id: Option<String>,
    name: Option<String>,
    layout: Option<String>,
    type_line: Option<String>,
    color_identity: Option<Vec<String>>,
    legality: Option<CommanderLegality>,
    keywords: Vec<String>,
    oracle_text: Option<String>,
    produced_mana: Option<Vec<String>>,
    faces: Vec<CardFaceEvidence>,
    issues: Vec<String>,
}

struct CardIndex {
    cards: Vec<CardMeta>,
    by_oracle: std::collections::BTreeMap<String, Vec<usize>>,
    by_name: std::collections::BTreeMap<String, Vec<usize>>,
}

#[derive(Debug)]
enum Resolution {
    Found(usize),
    Missing,
    Ambiguous(String),
}

const COLORS: [&str; 5] = ["W", "U", "B", "R", "G"];

fn canonical_name(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn nonempty_string(value: &Value, key: &str, issues: &mut Vec<String>) -> Option<String> {
    match value.get(key) {
        Some(Value::String(text)) if !text.trim().is_empty() => Some(text.clone()),
        Some(Value::String(_)) => {
            issues.push(format!("'{key}' is empty"));
            None
        }
        Some(_) => {
            issues.push(format!("'{key}' is not a string"));
            None
        }
        None => {
            issues.push(format!("'{key}' is missing"));
            None
        }
    }
}

fn validate_card(value: &Value) -> CardMeta {
    let mut issues = Vec::new();
    if nonempty_string(value, "object", &mut issues).as_deref() != Some("card") {
        issues.push("object is not 'card'".to_string());
    }
    let _id = nonempty_string(value, "id", &mut issues);
    let oracle_id = nonempty_string(value, "oracle_id", &mut issues);
    let name = nonempty_string(value, "name", &mut issues);
    let layout = nonempty_string(value, "layout", &mut issues);
    let type_line = nonempty_string(value, "type_line", &mut issues);
    if let Some(type_line) = &type_line
        && !type_line_is_known(type_line)
    {
        issues.push(format!(
            "type_line '{type_line}' contains an unknown supertype/type token"
        ));
    }
    let color_identity = match value.get("color_identity") {
        Some(Value::Array(items)) => {
            let mut colors = Vec::new();
            let mut seen = std::collections::BTreeSet::new();
            let mut bad = false;
            for item in items {
                match item.as_str() {
                    Some(color) if COLORS.contains(&color) => {
                        if !seen.insert(color.to_string()) {
                            issues.push(format!("color_identity repeats '{color}'"));
                            bad = true;
                        } else {
                            colors.push(color.to_string());
                        }
                    }
                    Some(color) => {
                        issues.push(format!("color_identity contains unknown color '{color}'"));
                        bad = true;
                    }
                    None => {
                        issues.push("color_identity contains a non-string entry".to_string());
                        bad = true;
                    }
                }
            }
            if bad { None } else { Some(colors) }
        }
        Some(_) => {
            issues.push("'color_identity' is not an array".to_string());
            None
        }
        None => {
            issues.push("'color_identity' is missing".to_string());
            None
        }
    };
    let legality = match value.get("legalities") {
        Some(Value::Object(legalities)) => match legalities.get("commander") {
            Some(Value::String(status)) => match status.as_str() {
                "legal" => Some(CommanderLegality::Legal),
                "not_legal" => Some(CommanderLegality::NotLegal),
                "banned" => Some(CommanderLegality::Banned),
                "restricted" => Some(CommanderLegality::Restricted),
                other => {
                    issues.push(format!("legalities.commander has unknown value '{other}'"));
                    None
                }
            },
            Some(_) => {
                issues.push("legalities.commander is not a string".to_string());
                None
            }
            None => {
                issues.push("legalities.commander is missing".to_string());
                None
            }
        },
        Some(_) => {
            issues.push("'legalities' is not an object".to_string());
            None
        }
        None => {
            issues.push("'legalities' is missing".to_string());
            None
        }
    };
    let keywords = match value.get("keywords") {
        Some(Value::Array(items)) => {
            let mut keywords = Vec::new();
            for item in items {
                match item.as_str() {
                    Some(keyword) => keywords.push(keyword.to_lowercase()),
                    None => issues.push("'keywords' contains a non-string entry".to_string()),
                }
            }
            keywords
        }
        Some(_) => {
            issues.push("'keywords' is not an array".to_string());
            Vec::new()
        }
        None => {
            issues.push("'keywords' is missing".to_string());
            Vec::new()
        }
    };
    let produced_mana = match value.get("produced_mana") {
        Some(Value::Array(items)) => {
            let mut colors = Vec::new();
            let mut bad = false;
            for item in items {
                match item.as_str() {
                    Some(color) if COLORS.contains(&color) || color == "C" => {
                        colors.push(color.to_string())
                    }
                    Some(color) => {
                        issues.push(format!("produced_mana contains unknown value '{color}'"));
                        bad = true;
                    }
                    None => {
                        issues.push("produced_mana contains a non-string entry".to_string());
                        bad = true;
                    }
                }
            }
            if bad { None } else { Some(colors) }
        }
        Some(_) => {
            issues.push("'produced_mana' is not an array".to_string());
            None
        }
        None => Some(Vec::new()),
    };
    let oracle_text = match value.get("oracle_text") {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Null) => {
            // Scryfall can use null, but this slice cannot distinguish "no
            // rules text" from "metadata dropped the rules text". Fail closed.
            issues.push("'oracle_text' is null; semantics cannot be certified".to_string());
            None
        }
        Some(_) => {
            issues.push("'oracle_text' is neither a string nor null".to_string());
            None
        }
        None => {
            issues.push("'oracle_text' is missing".to_string());
            None
        }
    };
    let mut faces = Vec::new();
    match value.get("card_faces") {
        Some(Value::Array(items)) => {
            if layout.as_deref() == Some("normal") && !items.is_empty() {
                issues.push("normal layout unexpectedly carries card_faces".to_string());
            }
            if layout.as_deref() != Some("normal") && items.is_empty() {
                issues.push("non-normal layout has an empty card_faces array".to_string());
            }
            for (index, face) in items.iter().enumerate() {
                let mut face_issues = Vec::new();
                let face_name = nonempty_string(face, "name", &mut face_issues)
                    .unwrap_or_else(|| format!("(face {index})"));
                let face_type = nonempty_string(face, "type_line", &mut face_issues)
                    .unwrap_or_else(|| "(unparsable type line)".to_string());
                for issue in face_issues {
                    issues.push(format!("face {index}: {issue}"));
                }
                let colors = face
                    .get("colors")
                    .and_then(Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                let color_indicator = face
                    .get("color_indicator")
                    .and_then(Value::as_array)
                    .map(|values| {
                        values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                faces.push(CardFaceEvidence {
                    name: face_name,
                    type_line: face_type,
                    colors,
                    color_indicator,
                    mana_cost: face
                        .get("mana_cost")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                });
            }
        }
        Some(_) => issues.push("'card_faces' is not an array".to_string()),
        None => {
            if layout.as_deref() != Some("normal") {
                issues.push("non-normal layout is missing card_faces".to_string());
            }
        }
    }
    CardMeta {
        oracle_id,
        name,
        layout,
        type_line,
        color_identity,
        legality,
        keywords,
        oracle_text,
        produced_mana,
        faces,
        issues,
    }
}

fn parse_card_snapshot(json: &str) -> Result<(CardProvenance, CardIndex), CommanderError> {
    let file: CardsSnapshotFile =
        serde_json::from_str(json).map_err(|e| CommanderError::Json(e.to_string()))?;
    if file.schema != CARD_METADATA_SCHEMA {
        return Err(CommanderError::Input(format!(
            "card metadata schema must be '{CARD_METADATA_SCHEMA}', got '{}'",
            file.schema
        )));
    }
    let p = &file.provenance;
    if !matches!(p.kind.as_str(), "real" | "synthetic") {
        return Err(CommanderError::Input(format!(
            "card metadata kind must be real or synthetic; got '{}'",
            p.kind
        )));
    }
    parse_rfc3339(&p.captured_at).ok_or_else(|| {
        CommanderError::Input(format!(
            "card captured_at '{}' is not an RFC 3339 timestamp",
            p.captured_at
        ))
    })?;
    if !valid_hex64(&p.cards_sha256) {
        return Err(CommanderError::Input(
            "card provenance cards_sha256 must be exactly 64 hex characters".to_string(),
        ));
    }
    if !valid_hex64(&p.raw_sha256) {
        return Err(CommanderError::Input(
            "card provenance raw_sha256 must be exactly 64 hex characters".to_string(),
        ));
    }
    let actual = sha256_hex(file.raw_cards_jsonl.as_bytes());
    if !actual.eq_ignore_ascii_case(&p.cards_sha256) {
        return Err(CommanderError::Input(format!(
            "cards_sha256 does not match the exact raw_cards_jsonl bytes (computed {actual})"
        )));
    }
    // raw_sha256 is the same embedded-byte binding, not a detached claim about
    // a full bulk download whose bytes are absent. Both must match the actual
    // captured JSONL; a zero or unrelated digest is rejected.
    if !actual.eq_ignore_ascii_case(&p.raw_sha256) {
        return Err(CommanderError::Input(format!(
            "raw_sha256 does not match the exact raw_cards_jsonl bytes (computed {actual}); a detached full-artifact claim is not accepted"
        )));
    }
    if p.kind == "real" {
        if p.provider != "scryfall" {
            return Err(CommanderError::Input(
                "real card metadata provider must be 'scryfall'".to_string(),
            ));
        }
        let api = p.api_url.as_deref().ok_or_else(|| {
            CommanderError::Input("real card metadata requires api_url".to_string())
        })?;
        let download = p.download_url.as_deref().ok_or_else(|| {
            CommanderError::Input("real card metadata requires download_url".to_string())
        })?;
        let api_host = https_host(api).ok_or_else(|| {
            CommanderError::Input("api_url must be an https URL without credentials".to_string())
        })?;
        if !matches!(
            api_host,
            "api.scryfall.com" | "scryfall.com" | "www.scryfall.com"
        ) {
            return Err(CommanderError::Input(format!(
                "api_url host '{api_host}' is not a Scryfall metadata host"
            )));
        }
        let download_host = https_host(download).ok_or_else(|| {
            CommanderError::Input(
                "download_url must be an https URL without credentials".to_string(),
            )
        })?;
        if download_host != "data.scryfall.io" {
            return Err(CommanderError::Input(format!(
                "download_url host '{download_host}' is not data.scryfall.io"
            )));
        }
        if p.bulk_type.as_deref().unwrap_or("").trim().is_empty() {
            return Err(CommanderError::Input(
                "real card metadata requires a non-empty bulk_type".to_string(),
            ));
        }
        let api_updated_at = p.api_updated_at.as_deref().unwrap_or("");
        if api_updated_at.trim().is_empty() {
            return Err(CommanderError::Input(
                "real card metadata requires api_updated_at".to_string(),
            ));
        }
        if parse_rfc3339(api_updated_at).is_none() {
            return Err(CommanderError::Input(format!(
                "api_updated_at '{api_updated_at}' is not an RFC 3339 timestamp"
            )));
        }
    } else {
        if p.provider != "synthetic" {
            return Err(CommanderError::Input(
                "synthetic card metadata provider must be 'synthetic'".to_string(),
            ));
        }
        if p.source.as_deref().unwrap_or("").trim().is_empty() {
            return Err(CommanderError::Input(
                "synthetic card metadata requires a non-empty source label".to_string(),
            ));
        }
    }
    let mut raw_cards = Vec::new();
    for (line_index, line) in file.raw_cards_jsonl.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line).map_err(|e| {
            CommanderError::Json(format!(
                "raw_cards_jsonl line {} is not JSON: {e}",
                line_index + 1
            ))
        })?;
        raw_cards.push(value);
    }
    if raw_cards.is_empty() {
        return Err(CommanderError::Input(
            "card metadata raw_cards_jsonl contains no card objects".to_string(),
        ));
    }
    if let Some(expected) = p.card_count
        && expected != raw_cards.len()
    {
        return Err(CommanderError::Input(format!(
            "card_count says {expected} but raw_cards_jsonl has {} objects",
            raw_cards.len()
        )));
    }
    let cards: Vec<CardMeta> = raw_cards.iter().map(validate_card).collect();
    let mut by_oracle: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
    let mut by_name: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, card) in cards.iter().enumerate() {
        if let Some(oracle_id) = &card.oracle_id {
            by_oracle.entry(oracle_id.clone()).or_default().push(index);
        }
        if let Some(name) = &card.name {
            by_name.entry(canonical_name(name)).or_default().push(index);
        }
    }
    Ok((
        p.clone(),
        CardIndex {
            cards,
            by_oracle,
            by_name,
        },
    ))
}

impl CardIndex {
    fn resolve(&self, entry: &DeckEntry) -> Resolution {
        let candidates = match self.by_oracle.get(&entry.oracle_id) {
            Some(candidates) => candidates,
            None => {
                if self.by_name.contains_key(&canonical_name(&entry.name)) {
                    return Resolution::Ambiguous(format!(
                        "deck entry '{}' names a card that has a different oracle_id",
                        entry.name
                    ));
                }
                return Resolution::Missing;
            }
        };
        if candidates.len() > 1 {
            for left in 0..candidates.len() {
                for right in (left + 1)..candidates.len() {
                    if !equivalent_printing(
                        &self.cards[candidates[left]],
                        &self.cards[candidates[right]],
                    ) {
                        return Resolution::Ambiguous(format!(
                            "card objects sharing oracle_id '{}' disagree on relevant metadata (name/type/layout/color/legality/text/faces)",
                            entry.oracle_id
                        ));
                    }
                }
            }
        }
        let mut names = std::collections::BTreeSet::new();
        for index in candidates {
            if let Some(name) = &self.cards[*index].name {
                names.insert(canonical_name(name));
            }
        }
        if names.len() != 1 {
            return Resolution::Ambiguous(format!(
                "card objects sharing oracle_id '{}' disagree on name",
                entry.oracle_id
            ));
        }
        if names.first() != Some(&canonical_name(&entry.name)) {
            return Resolution::Ambiguous(format!(
                "deck name '{}' and oracle_id '{}' resolve to different cards",
                entry.name, entry.oracle_id
            ));
        }
        if let Some(same_name) = self.by_name.get(&canonical_name(&entry.name)) {
            for index in same_name {
                let other = &self.cards[*index];
                if other.oracle_id.is_some()
                    && other.oracle_id.as_deref() != Some(entry.oracle_id.as_str())
                {
                    return Resolution::Ambiguous(format!(
                        "the name '{}' is shared by more than one oracle identity",
                        entry.name
                    ));
                }
            }
        }
        for index in candidates {
            if self.cards[*index].name.as_deref() == Some(entry.name.as_str()) {
                return Resolution::Found(*index);
            }
        }
        Resolution::Found(candidates[0])
    }
}

const KNOWN_SUPERTYPES: [&str; 6] = ["Basic", "Legendary", "Snow", "World", "Ongoing", "Host"];
const KNOWN_CARD_TYPES: [&str; 16] = [
    "Artifact",
    "Battle",
    "Conspiracy",
    "Creature",
    "Dungeon",
    "Enchantment",
    "Instant",
    "Kindred",
    "Land",
    "Phenomena",
    "Plane",
    "Planeswalker",
    "Scheme",
    "Sorcery",
    "Tribal",
    "Vanguard",
];

fn type_line_is_known(type_line: &str) -> bool {
    let before = type_line.split('—').next().unwrap_or("").trim();
    if before.is_empty() || before.contains("//") {
        return false;
    }
    let mut has_card_type = false;
    for token in before.split_whitespace() {
        if KNOWN_SUPERTYPES
            .iter()
            .any(|known| token.eq_ignore_ascii_case(known))
        {
            continue;
        }
        if KNOWN_CARD_TYPES
            .iter()
            .any(|known| token.eq_ignore_ascii_case(known))
        {
            has_card_type = true;
            continue;
        }
        return false;
    }
    has_card_type
}

fn equivalent_printing(left: &CardMeta, right: &CardMeta) -> bool {
    left.name == right.name
        && left.layout == right.layout
        && left.type_line == right.type_line
        && left.color_identity == right.color_identity
        && left.legality == right.legality
        && left.keywords == right.keywords
        && left.oracle_text == right.oracle_text
        && left.produced_mana == right.produced_mana
        && left.faces == right.faces
        && left.issues == right.issues
}

fn type_line_parts(type_line: &str) -> Option<Vec<String>> {
    let before = type_line.split('—').next()?.trim();
    if before.is_empty() {
        return None;
    }
    Some(before.split_whitespace().map(str::to_string).collect())
}

fn has_token(tokens: &[String], token: &str) -> bool {
    tokens.iter().any(|value| value.eq_ignore_ascii_case(token))
}

fn is_basic_land(type_line: &str) -> bool {
    type_line_parts(type_line)
        .map(|tokens| has_token(&tokens, "Basic") && has_token(&tokens, "Land"))
        .unwrap_or(false)
}

fn basic_land_types(type_line: &str) -> Vec<(&'static str, &'static str)> {
    let after = type_line
        .split_once('—')
        .map(|(_, after)| after)
        .unwrap_or("");
    [
        ("Plains", "W"),
        ("Island", "U"),
        ("Swamp", "B"),
        ("Mountain", "R"),
        ("Forest", "G"),
    ]
    .into_iter()
    .filter(|(name, _)| {
        after
            .split_whitespace()
            .any(|token| token.eq_ignore_ascii_case(name))
    })
    .collect()
}

fn is_background(type_line: &str) -> bool {
    type_line
        .split_once('—')
        .map(|(_, after)| {
            after
                .split_whitespace()
                .any(|token| token.eq_ignore_ascii_case("Background"))
        })
        .unwrap_or(false)
}

/// CR 903.4b: choosing a color before the game affects color identity during
/// deck construction. Class-based predicate, independent of other unsupported
/// variant markers (Background/Partner) and never card-name based.
fn pregame_color_choice(card: &CardMeta) -> bool {
    let text = card.oracle_text.clone().unwrap_or_default().to_lowercase();
    text.contains("before the game begins") && text.contains("choose a color")
}

fn variant_marker(card: &CardMeta) -> Option<String> {
    for keyword in &card.keywords {
        if keyword == "partner"
            || keyword.starts_with("partner with")
            || keyword == "friends forever"
            || keyword == "doctor's companion"
            || keyword == "choose a background"
        {
            return Some(format!("unsupported commander variant keyword '{keyword}'"));
        }
    }
    let text = card.oracle_text.clone().unwrap_or_default().to_lowercase();
    fn has_word(text: &str, word: &str) -> bool {
        text.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|part| part == word)
    }
    if has_word(&text, "partner") {
        return Some("unsupported commander variant 'partner' in rules text".to_string());
    }
    if text.contains("friends forever") {
        return Some("unsupported commander variant 'friends forever'".to_string());
    }
    if text.contains("doctor's companion") {
        return Some("unsupported commander variant \"doctor's companion\"".to_string());
    }
    if text.contains("choose a background") {
        return Some("unsupported commander variant 'choose a background'".to_string());
    }
    if let Some(type_line) = &card.type_line
        && is_background(type_line)
    {
        return Some("unsupported commander variant 'Background'".to_string());
    }
    if text.contains("before the game begins") && text.contains("choose a color") {
        return Some("pregame chosen-color commander (CR 903.4b) is not certifiable".to_string());
    }
    None
}

/// Static abilities can make a card a creature in zones other than the
/// battlefield (e.g. Grist, the Hunger Tide). That is a legal Commander class,
/// but this slice does not model zone-dependent characteristics, so it must be
/// refused as `not_certified` rather than guessed illegal. Class-based, not a
/// card-name special case.
fn zone_dependent_creature(card: &CardMeta) -> bool {
    let Some(text) = &card.oracle_text else {
        return false;
    };
    let text = text.to_lowercase();
    let off_battlefield = text.contains("isn't on the battlefield")
        || text.contains("is not on the battlefield")
        || text.contains("off the battlefield");
    let becomes_creature = text.contains("creature")
        && (text.contains("in addition to its other types")
            || text.contains("it's a")
            || text.contains("it is a"));
    off_battlefield && becomes_creature
}

fn commander_eligibility(card: &CardMeta) -> (LegalityStatus, String) {
    if !card.issues.is_empty() {
        return (
            LegalityStatus::NotCertified,
            format!("card metadata is incomplete: {}", card.issues.join("; ")),
        );
    }
    if card.layout.as_deref() != Some("normal") {
        return (
            LegalityStatus::NotCertified,
            "non-normal commander layouts are not certified in this slice".to_string(),
        );
    }
    if !card.faces.is_empty() {
        return (
            LegalityStatus::NotCertified,
            "commander carries face metadata; no front-only certification".to_string(),
        );
    }
    let Some(tokens) = card.type_line.as_deref().and_then(type_line_parts) else {
        return (
            LegalityStatus::NotCertified,
            "commander type line is empty or unparsable".to_string(),
        );
    };
    if let Some(marker) = variant_marker(card) {
        return (LegalityStatus::NotCertified, marker);
    }
    let subtypes: Vec<String> = card
        .type_line
        .as_deref()
        .and_then(|type_line| type_line.split_once('—'))
        .map(|(_, suffix)| suffix.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default();
    let legendary = has_token(&tokens, "Legendary");
    let creature = has_token(&tokens, "Creature");
    // Vehicle/Spacecraft are subtypes after the em dash, not card types.
    let vehicle = has_token(&tokens, "Vehicle") || has_token(&subtypes, "Vehicle");
    let spacecraft = has_token(&tokens, "Spacecraft") || has_token(&subtypes, "Spacecraft");
    if zone_dependent_creature(card) && !creature {
        return (
            LegalityStatus::NotCertified,
            "zone-dependent/type-altering off-battlefield commander eligibility (CR 903.3) is not certified in this slice"
                .to_string(),
        );
    }
    if (vehicle || spacecraft) && !creature {
        let category = if vehicle { "Vehicle" } else { "Spacecraft" };
        return (
            LegalityStatus::NotCertified,
            format!(
                "{category} commander (CR 903.3 category; power/toughness eligibility not modelled) is outside the front-face legendary-creature baseline"
            ),
        );
    }
    if !creature
        && let Some(text) = &card.oracle_text
        && text.to_lowercase().contains("can be your commander")
    {
        return (
            LegalityStatus::NotCertified,
            "card-specific 'can be your commander' ability (CR 903.3a) is unsupported".to_string(),
        );
    }
    if !legendary
        && let Some(text) = &card.oracle_text
        && text.to_lowercase().contains("can be your commander")
    {
        return (
            LegalityStatus::NotCertified,
            "card-specific 'can be your commander' ability (CR 903.3a) is unsupported".to_string(),
        );
    }
    if legendary && creature {
        return (
            LegalityStatus::Legal,
            "front-face legendary creature".to_string(),
        );
    }
    (
        LegalityStatus::Illegal,
        "commander is not a legendary creature".to_string(),
    )
}

fn status_worst(left: LegalityStatus, right: LegalityStatus) -> LegalityStatus {
    match (left, right) {
        (LegalityStatus::Illegal, _) | (_, LegalityStatus::Illegal) => LegalityStatus::Illegal,
        (LegalityStatus::NotCertified, _) | (_, LegalityStatus::NotCertified) => {
            LegalityStatus::NotCertified
        }
        _ => LegalityStatus::Legal,
    }
}

/// Analyze the two offline snapshots. Stage 1: count, commander baseline,
/// format legality and engine-support separation.
pub fn analyze_snapshots(
    deck_json: &str,
    cards_json: &str,
    now_unix: u64,
) -> Result<CommanderReport, CommanderError> {
    let (deck_provenance, deck) = parse_deck_snapshot(deck_json)?;
    let (card_provenance, index) = parse_card_snapshot(cards_json)?;
    let mut findings = Vec::new();
    let mut commanders = Vec::new();
    let refs: Vec<(&DeckEntry, bool)> = deck
        .commanders
        .iter()
        .map(|entry| (entry, true))
        .chain(deck.library.iter().map(|entry| (entry, false)))
        .collect();
    let resolutions: Vec<Resolution> = refs.iter().map(|(entry, _)| index.resolve(entry)).collect();

    // Metadata resolution failures are not_certified, never guessed.
    let mut metadata_seen = std::collections::BTreeSet::new();
    let mut card_evidence = Vec::new();
    let mut evidence_seen = std::collections::BTreeSet::new();
    for (index_in_refs, (entry, _)) in refs.iter().enumerate() {
        match &resolutions[index_in_refs] {
            Resolution::Found(card_index) => {
                let card = &index.cards[*card_index];
                if !card.issues.is_empty() && metadata_seen.insert(format!("card:{card_index}")) {
                    findings.push(Finding {
                        code: "CMD-METADATA".to_string(),
                        status: LegalityStatus::NotCertified,
                        message: format!(
                            "card '{}' metadata is incomplete: {}",
                            card.name.clone().unwrap_or_else(|| entry.name.clone()),
                            card.issues.join("; ")
                        ),
                    });
                }
                let non_normal = card.layout.as_deref() != Some("normal") || !card.faces.is_empty();
                if non_normal && evidence_seen.insert(*card_index) {
                    card_evidence.push(CardEvidence {
                        name: card.name.clone().unwrap_or_else(|| entry.name.clone()),
                        oracle_id: card.oracle_id.clone(),
                        layout: card.layout.clone(),
                        top_level_color_identity: card.color_identity.clone().unwrap_or_default(),
                        faces: card.faces.clone(),
                        reason:
                            "non-normal layouts are metadata/refusal evidence only in this slice"
                                .to_string(),
                    });
                }
            }
            Resolution::Missing => {
                if metadata_seen.insert(format!("missing:{}", entry.oracle_id)) {
                    findings.push(Finding {
                        code: "CMD-METADATA".to_string(),
                        status: LegalityStatus::NotCertified,
                        message: format!(
                            "no card metadata resolves '{}' ({})",
                            entry.name, entry.oracle_id
                        ),
                    });
                }
            }
            Resolution::Ambiguous(reason) => {
                if metadata_seen.insert(format!("ambiguous:{}", entry.oracle_id)) {
                    findings.push(Finding {
                        code: "CMD-AMBIGUOUS-IDENTITY".to_string(),
                        status: LegalityStatus::NotCertified,
                        message: reason.clone(),
                    });
                }
            }
        }
    }

    // Non-normal layouts are never certified, commander or library.
    for (index_in_refs, (_, _)) in refs.iter().enumerate() {
        if let Resolution::Found(card_index) = &resolutions[index_in_refs] {
            let card = &index.cards[*card_index];
            if card.layout.as_deref() != Some("normal") || !card.faces.is_empty() {
                findings.push(Finding {
                    code: "CMD-LAYOUT-UNSUPPORTED".to_string(),
                    status: LegalityStatus::NotCertified,
                    message: format!(
                        "'{}' has layout '{}'; all non-normal layouts are not_certified in this slice",
                        card.name.clone().unwrap_or_default(),
                        card.layout.clone().unwrap_or_else(|| "unknown".to_string())
                    ),
                });
            }
        }
    }

    // Commander zone count.
    let commander_count = deck.commanders.len();
    match commander_count {
        0 => findings.push(Finding {
            code: "CMD-COMMANDER-COUNT".to_string(),
            status: LegalityStatus::Illegal,
            message: "the deck has no commander".to_string(),
        }),
        1 => findings.push(Finding {
            code: "CMD-COMMANDER-COUNT".to_string(),
            status: LegalityStatus::Legal,
            message: "exactly one commander".to_string(),
        }),
        _ => findings.push(Finding {
            code: "CMD-COMMANDER-COUNT".to_string(),
            status: LegalityStatus::NotCertified,
            message: format!(
                "{commander_count} commanders: every 2+ commander variant is conservatively not_certified"
            ),
        }),
    }
    let single = commander_count == 1;
    for (index_in_refs, (entry, is_commander)) in refs.iter().enumerate() {
        if !is_commander {
            continue;
        }
        if entry.quantity != 1 {
            findings.push(Finding {
                code: "CMD-COMMANDER-QUANTITY".to_string(),
                status: LegalityStatus::Illegal,
                message: format!(
                    "commander '{}' has quantity {}; a commander entry must be exactly 1",
                    entry.name, entry.quantity
                ),
            });
        }
        match &resolutions[index_in_refs] {
            Resolution::Found(card_index) => {
                let card = &index.cards[*card_index];
                let (status, reason) = if single {
                    commander_eligibility(card)
                } else {
                    (
                        LegalityStatus::NotCertified,
                        "multi-commander eligibility is not certified".to_string(),
                    )
                };
                if single {
                    match status {
                        LegalityStatus::Legal => findings.push(Finding {
                            code: "CMD-ELIGIBILITY".to_string(),
                            status,
                            message: format!(
                                "commander '{}' is a front-face legendary creature",
                                card.name.clone().unwrap_or_default()
                            ),
                        }),
                        LegalityStatus::Illegal => findings.push(Finding {
                            code: "CMD-ELIGIBILITY".to_string(),
                            status,
                            message: format!(
                                "commander '{}': {reason}",
                                card.name.clone().unwrap_or_default()
                            ),
                        }),
                        LegalityStatus::NotCertified => findings.push(Finding {
                            code: "CMD-ELIGIBILITY-VARIANT".to_string(),
                            status,
                            message: format!(
                                "commander '{}': {reason}",
                                card.name.clone().unwrap_or_default()
                            ),
                        }),
                    }
                }
                commanders.push(CommanderEntryReport {
                    name: card.name.clone().unwrap_or_else(|| entry.name.clone()),
                    oracle_id: card
                        .oracle_id
                        .clone()
                        .unwrap_or_else(|| entry.oracle_id.clone()),
                    color_identity: card.color_identity.clone().unwrap_or_default(),
                    layout: card.layout.clone(),
                    eligibility: status,
                    reason,
                });
            }
            _ => commanders.push(CommanderEntryReport {
                name: entry.name.clone(),
                oracle_id: entry.oracle_id.clone(),
                color_identity: Vec::new(),
                layout: None,
                eligibility: LegalityStatus::NotCertified,
                reason: "card metadata does not resolve a single identity".to_string(),
            }),
        }
    }

    let commander_total: u64 = deck.commanders.iter().map(|e| u64::from(e.quantity)).sum();
    let library_total: u64 = deck.library.iter().map(|e| u64::from(e.quantity)).sum();
    let total = commander_total + library_total;
    findings.push(if total == 100 {
        Finding {
            code: "CMD-COUNT-100".to_string(),
            status: LegalityStatus::Legal,
            message: format!(
                "100 cards including {commander_total} commander(s) and {library_total} library cards"
            ),
        }
    } else {
        Finding {
            code: "CMD-COUNT-100".to_string(),
            status: LegalityStatus::Illegal,
            message: format!(
                "the deck has {total} cards including {commander_total} commander(s); Commander requires exactly 100"
            ),
        }
    });

    // Other zones never count toward the 100.
    if !deck.companions.is_empty() {
        findings.push(Finding {
            code: "CMD-COMPANION".to_string(),
            status: LegalityStatus::NotCertified,
            message: format!(
                "{} companion entry/entries: companion / 101st-card variants are unsupported",
                deck.companions.len()
            ),
        });
    }
    if !deck.sideboard.is_empty() {
        findings.push(Finding {
            code: "CMD-SIDEBOARD".to_string(),
            status: LegalityStatus::NotCertified,
            message: format!(
                "{} sideboard entry/entries: Commander games do not use sideboards (CR 903.5e) and this slice does not model them",
                deck.sideboard.len()
            ),
        });
    }
    if !deck.excluded.is_empty() {
        findings.push(Finding {
            code: "CMD-EXCLUDED".to_string(),
            status: LegalityStatus::Legal,
            message: format!(
                "{} excluded/maybeboard entry/entries are not part of the deck and were not counted",
                deck.excluded.len()
            ),
        });
    }

    // Color identity (CR 903.5c) and basic land types (CR 903.5d).
    let chosen_color_unresolved = if single {
        match &resolutions[0] {
            Resolution::Found(card_index) => pregame_color_choice(&index.cards[*card_index]),
            _ => false,
        }
    } else {
        false
    };
    let commander_identity: Option<Vec<String>> = if single && !chosen_color_unresolved {
        match &resolutions[0] {
            Resolution::Found(card_index) => {
                let card = &index.cards[*card_index];
                if card.issues.is_empty() && card.layout.as_deref() == Some("normal") {
                    card.color_identity.clone()
                } else {
                    None
                }
            }
            _ => None,
        }
    } else {
        None
    };
    if single && chosen_color_unresolved {
        findings.push(Finding {
            code: "CMD-IDENTITY".to_string(),
            status: LegalityStatus::NotCertified,
            message: "pregame chosen color (CR 903.4b) is unresolved; color identity and dependent 903.5c/903.5d checks cannot be certified"
                .to_string(),
        });
    } else if single && commander_identity.is_none() {
        findings.push(Finding {
            code: "CMD-IDENTITY".to_string(),
            status: LegalityStatus::NotCertified,
            message: "the commander's color identity is not certifiable from complete metadata"
                .to_string(),
        });
    }
    if !single {
        findings.push(Finding {
            code: "CMD-IDENTITY".to_string(),
            status: LegalityStatus::NotCertified,
            message: "library color identity requires exactly one certifiable commander"
                .to_string(),
        });
    }
    for (index_in_refs, (_, is_commander)) in refs.iter().enumerate() {
        if *is_commander {
            continue;
        }
        let Resolution::Found(card_index) = &resolutions[index_in_refs] else {
            continue;
        };
        let card = &index.cards[*card_index];
        if !card.issues.is_empty()
            || card.layout.as_deref() != Some("normal")
            || !card.faces.is_empty()
        {
            continue;
        }
        let Some(identity) = &card.color_identity else {
            continue;
        };
        if let Some(commander_identity) = &commander_identity {
            let offending: Vec<&String> = identity
                .iter()
                .filter(|color| !commander_identity.contains(color))
                .collect();
            if !offending.is_empty() {
                findings.push(Finding {
                    code: "CMD-IDENTITY".to_string(),
                    status: LegalityStatus::Illegal,
                    message: format!(
                        "'{}' has color identity [{}] outside the commander's [{}] (offending: {}) (CR 903.5c)",
                        card.name.clone().unwrap_or_default(),
                        identity.join(","),
                        commander_identity.join(","),
                        offending.iter().map(|c| c.as_str()).collect::<Vec<_>>().join(",")
                    ),
                });
            }
        }
        if let Some(type_line) = &card.type_line {
            let types = basic_land_types(type_line);
            if !types.is_empty()
                && let Some(commander_identity) = &commander_identity
            {
                let mut produced: std::collections::BTreeSet<String> = types
                    .iter()
                    .map(|(_, color)| (*color).to_string())
                    .collect();
                if let Some(colors) = &card.produced_mana {
                    for color in colors {
                        if COLORS.contains(&color.as_str()) {
                            produced.insert(color.clone());
                        }
                    }
                }
                let offending: Vec<&str> = produced
                    .iter()
                    .map(String::as_str)
                    .filter(|color| !commander_identity.iter().any(|own| own == color))
                    .collect();
                if !offending.is_empty() {
                    findings.push(Finding {
                        code: "CMD-BASIC-LAND-TYPE".to_string(),
                        status: LegalityStatus::Illegal,
                        message: format!(
                            "'{}' has basic land type(s) that could produce [{}] outside the commander's identity (CR 903.5d)",
                            card.name.clone().unwrap_or_default(),
                            offending.join(",")
                        ),
                    });
                }
            }
        }
    }

    // Singleton (CR 903.5b): canonical English name, Basic-supertype exemption,
    // and explicit not_certified for unmodeled card-specific allowances.
    let mut groups: std::collections::BTreeMap<String, Vec<(usize, u32)>> =
        std::collections::BTreeMap::new();
    for (index_in_refs, (entry, _)) in refs.iter().enumerate() {
        if let Resolution::Found(card_index) = &resolutions[index_in_refs] {
            let card = &index.cards[*card_index];
            let name = card.name.clone().unwrap_or_else(|| entry.name.clone());
            groups
                .entry(canonical_name(&name))
                .or_default()
                .push((*card_index, entry.quantity));
        }
    }
    for (name, entries) in &groups {
        let quantity: u32 = entries.iter().map(|(_, qty)| *qty).sum();
        if quantity <= 1 {
            continue;
        }
        let oracle_ids: std::collections::BTreeSet<&str> = entries
            .iter()
            .filter_map(|(card_index, _)| index.cards[*card_index].oracle_id.as_deref())
            .collect();
        if oracle_ids.len() > 1 {
            findings.push(Finding {
                code: "CMD-AMBIGUOUS-IDENTITY".to_string(),
                status: LegalityStatus::NotCertified,
                message: format!("the canonical name '{name}' maps to multiple oracle identities"),
            });
            continue;
        }
        let all_basic = entries.iter().all(|(card_index, _)| {
            index.cards[*card_index]
                .type_line
                .as_deref()
                .map(is_basic_land)
                .unwrap_or(false)
        });
        if all_basic {
            continue;
        }
        // A non-basic duplicate is only proven illegal after ruling out
        // card-specific allowances. Missing rules text is not evidence that
        // no allowance exists (an empty, present string is valid metadata).
        if entries.iter().any(|(card_index, _)| {
            let card = &index.cards[*card_index];
            card.type_line
                .as_deref()
                .is_none_or(|type_line| !type_line_is_known(type_line))
                || card.oracle_text.is_none()
        }) {
            findings.push(Finding {
                code: "CMD-SINGLETON".to_string(),
                status: LegalityStatus::NotCertified,
                message: format!(
                    "singleton cannot be certified for '{name}': type or Oracle rules-text metadata is incomplete or unusable"
                ),
            });
            continue;
        }
        let allowance = entries.iter().any(|(card_index, _)| {
            index.cards[*card_index]
                .oracle_text
                .as_deref()
                .map(|text| {
                    let text = text.to_lowercase();
                    text.contains("a deck can have")
                        && (text.contains("cards named") || text.contains("card named"))
                })
                .unwrap_or(false)
        });
        if allowance {
            findings.push(Finding {
                code: "CMD-DECKBUILDING-EXCEPTION".to_string(),
                status: LegalityStatus::NotCertified,
                message: format!(
                    "'{name}' has a card-specific deck-construction allowance (for example 'a deck can have any number/up to N cards named ...'); this slice does not model it, so the duplicate is not certified"
                ),
            });
            continue;
        }
        let interchangeable = entries.iter().any(|(card_index, _)| {
            index.cards[*card_index]
                .oracle_text
                .as_deref()
                .map(|text| text.to_lowercase().contains("interchangeable"))
                .unwrap_or(false)
        });
        if interchangeable {
            findings.push(Finding {
                code: "CMD-DECKBUILDING-UNKNOWN".to_string(),
                status: LegalityStatus::NotCertified,
                message: format!(
                    "'{name}' mentions an interchangeable-name relationship; the singleton result is not certified"
                ),
            });
            continue;
        }
        findings.push(Finding {
            code: "CMD-SINGLETON".to_string(),
            status: LegalityStatus::Illegal,
            message: format!(
                "'{name}' appears {quantity} times; every non-basic card must have a different English name (CR 903.5b)"
            ),
        });
    }
    if !findings
        .iter()
        .any(|finding| finding.code == "CMD-SINGLETON")
    {
        findings.push(Finding {
            code: "CMD-SINGLETON".to_string(),
            status: LegalityStatus::Legal,
            message: "no duplicate non-basic English names".to_string(),
        });
    }

    // Format legality (bans) is independent of engine support.
    let mut format_status = LegalityStatus::Legal;
    let mut format_checked = 0usize;
    let mut format_nonlegal = false;
    for (index_in_refs, (_, _)) in refs.iter().enumerate() {
        let Resolution::Found(card_index) = &resolutions[index_in_refs] else {
            format_status = status_worst(format_status, LegalityStatus::NotCertified);
            format_nonlegal = true;
            continue;
        };
        let card = &index.cards[*card_index];
        format_checked += 1;
        match card.legality {
            Some(CommanderLegality::Legal) => {}
            Some(CommanderLegality::Banned) => {
                format_nonlegal = true;
                format_status = status_worst(format_status, LegalityStatus::Illegal);
                findings.push(Finding {
                    code: "CMD-FORMAT-BANNED".to_string(),
                    status: LegalityStatus::Illegal,
                    message: format!(
                        "'{}' is banned in Commander as of the recorded snapshot",
                        card.name.clone().unwrap_or_default()
                    ),
                });
            }
            Some(CommanderLegality::NotLegal) => {
                format_nonlegal = true;
                format_status = status_worst(format_status, LegalityStatus::Illegal);
                findings.push(Finding {
                    code: "CMD-FORMAT-NOT-LEGAL".to_string(),
                    status: LegalityStatus::Illegal,
                    message: format!(
                        "'{}' is not legal in Commander as of the recorded snapshot",
                        card.name.clone().unwrap_or_default()
                    ),
                });
            }
            Some(CommanderLegality::Restricted) => {
                format_nonlegal = true;
                format_status = status_worst(format_status, LegalityStatus::NotCertified);
                findings.push(Finding {
                    code: "CMD-FORMAT-RESTRICTED".to_string(),
                    status: LegalityStatus::NotCertified,
                    message: format!(
                        "'{}' has legalities.commander = restricted",
                        card.name.clone().unwrap_or_default()
                    ),
                });
            }
            None => {
                format_nonlegal = true;
                format_status = status_worst(format_status, LegalityStatus::NotCertified);
                findings.push(Finding {
                    code: "CMD-FORMAT-MISSING".to_string(),
                    status: LegalityStatus::NotCertified,
                    message: format!(
                        "'{}' has no usable legalities.commander status",
                        card.name.clone().unwrap_or_default()
                    ),
                });
            }
        }
    }
    if !format_nonlegal && format_checked > 0 && format_checked == refs.len() {
        findings.push(Finding {
            code: "CMD-FORMAT-LEGAL".to_string(),
            status: LegalityStatus::Legal,
            message: "every referenced card is legal in Commander as of the recorded snapshot"
                .to_string(),
        });
    }

    // Engine support is deliberately separate from format legality.
    let mut unsupported_cards = Vec::new();
    for (index_in_refs, (entry, _)) in refs.iter().enumerate() {
        match &resolutions[index_in_refs] {
            Resolution::Found(card_index) => {
                let card = &index.cards[*card_index];
                let name = card.name.clone().unwrap_or_else(|| entry.name.clone());
                let reason = if !card.issues.is_empty() {
                    Some("card metadata is incomplete".to_string())
                } else if card.layout.as_deref() != Some("normal") {
                    Some("non-normal layouts are not modelled by the engine".to_string())
                } else if aperture_core::catalog::by_name(&name).is_none() {
                    Some(format!(
                        "not in the implemented {}-card practice pool",
                        aperture_core::catalog::pool_size()
                    ))
                } else {
                    None
                };
                if let Some(reason) = reason {
                    unsupported_cards.push(UnsupportedSupportCard {
                        name,
                        oracle_id: card.oracle_id.clone(),
                        reason,
                    });
                }
            }
            _ => unsupported_cards.push(UnsupportedSupportCard {
                name: entry.name.clone(),
                oracle_id: Some(entry.oracle_id.clone()),
                reason: "card metadata does not resolve a single identity".to_string(),
            }),
        }
    }
    unsupported_cards.sort_by(|left, right| {
        (left.name.as_str(), left.reason.as_str())
            .cmp(&(right.name.as_str(), right.reason.as_str()))
    });
    unsupported_cards
        .dedup_by(|left, right| left.name == right.name && left.reason == right.reason);
    let engine_status = if unsupported_cards.is_empty() && !refs.is_empty() {
        SupportStatus::Supported
    } else {
        SupportStatus::Unsupported
    };

    let mut unique_names = std::collections::BTreeSet::new();
    for (index_in_refs, (entry, _)) in refs.iter().enumerate() {
        match &resolutions[index_in_refs] {
            Resolution::Found(card_index) => {
                if let Some(name) = &index.cards[*card_index].name {
                    unique_names.insert(canonical_name(name));
                }
            }
            _ => {
                unique_names.insert(canonical_name(&entry.name));
            }
        }
    }

    let status = if findings
        .iter()
        .any(|finding| finding.status == LegalityStatus::Illegal)
    {
        LegalityStatus::Illegal
    } else if findings
        .iter()
        .any(|finding| finding.status == LegalityStatus::NotCertified)
    {
        LegalityStatus::NotCertified
    } else {
        LegalityStatus::Legal
    };

    let captured = parse_rfc3339(&card_provenance.captured_at).unwrap_or(0) as u64;
    let age_days = if now_unix > captured {
        (now_unix - captured) / 86_400
    } else {
        0
    };
    Ok(CommanderReport {
        schema: REPORT_SCHEMA.to_string(),
        status,
        deck: DeckSummary {
            commanders: commander_total,
            library: library_total,
            total,
            unique_canonical_names: unique_names.len(),
        },
        commanders,
        format_legality: FormatLegality {
            status: format_status,
            as_of: card_provenance.captured_at.clone(),
            age_days,
            stale: age_days > 30,
            snapshot_sha256: card_provenance.cards_sha256.clone(),
            basis: "as of the captured offline snapshot; not a live ban-list lookup".to_string(),
        },
        engine_support: EngineSupport {
            status: engine_status,
            unsupported_cards,
        },
        gameplay: GameplayReport {
            available: false,
            reasons: vec![
                "command zone and commander return rules are not implemented".to_string(),
                "commander tax is not implemented".to_string(),
                "40 starting life is not implemented".to_string(),
                "21 commander combat damage is not implemented".to_string(),
                "Commander multiplayer seat semantics are not implemented".to_string(),
            ],
        },
        findings,
        card_evidence,
        provenance: ReportProvenance {
            deck: ReportDeckProvenance {
                kind: deck_provenance.kind,
                provider: deck_provenance.provider,
                source: deck_provenance.source,
                captured_at: deck_provenance.captured_at,
                raw_sha256: deck_provenance.raw_sha256,
            },
            cards: ReportCardProvenance {
                kind: card_provenance.kind,
                provider: card_provenance.provider,
                api_url: card_provenance.api_url,
                download_url: card_provenance.download_url,
                bulk_type: card_provenance.bulk_type,
                captured_at: card_provenance.captured_at,
                cards_sha256: card_provenance.cards_sha256,
                raw_sha256: card_provenance.raw_sha256,
            },
        },
    })
}

/// Read and analyze two offline snapshot files. No network is used.
pub fn analyze_files(
    deck_path: &Path,
    cards_path: &Path,
    now_unix: u64,
) -> Result<CommanderReport, CommanderError> {
    let deck = std::fs::read_to_string(deck_path).map_err(|e| CommanderError::Io {
        path: deck_path.display().to_string(),
        detail: e.to_string(),
    })?;
    let cards = std::fs::read_to_string(cards_path).map_err(|e| CommanderError::Io {
        path: cards_path.display().to_string(),
        detail: e.to_string(),
    })?;
    analyze_snapshots(&deck, &cards, now_unix)
}

fn parse_rfc3339(input: &str) -> Option<i64> {
    let text = input.trim();
    let bytes = text.as_bytes();
    if bytes.len() < 20 {
        return None;
    }
    let number =
        |range: std::ops::Range<usize>| -> Option<i64> { text.get(range)?.parse::<i64>().ok() };
    if bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || !matches!(bytes.get(10), Some(b'T') | Some(b't'))
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return None;
    }
    let year = number(0..4)? as i32;
    let month = number(5..7)? as u32;
    let day = number(8..10)? as u32;
    let hour = number(11..13)?;
    let minute = number(14..16)?;
    let second = number(17..19)?;
    if !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || !(0..24).contains(&hour)
        || !(0..60).contains(&minute)
        || !(0..=60).contains(&second)
    {
        return None;
    }
    let mut index = 19;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let start = index;
        while bytes.get(index).map(|byte| byte.is_ascii_digit()) == Some(true) {
            index += 1;
        }
        if index == start {
            return None;
        }
    }
    let offset_seconds = match bytes.get(index) {
        Some(b'Z') | Some(b'z') => {
            index += 1;
            0
        }
        Some(sign @ (b'+' | b'-')) => {
            let sign = if *sign == b'+' { 1 } else { -1 };
            let offset_hour = number(index + 1..index + 3)?;
            if bytes.get(index + 3) != Some(&b':') {
                return None;
            }
            let offset_minute = number(index + 4..index + 6)?;
            if !(0..24).contains(&offset_hour) || !(0..60).contains(&offset_minute) {
                return None;
            }
            index += 6;
            sign * (offset_hour * 3600 + offset_minute * 60)
        }
        _ => return None,
    };
    if index != bytes.len() {
        return None;
    }
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3600 + minute * 60 + second - offset_seconds)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = (year - era * 400) as i64;
    let month_prime = (i64::from(month) + 9) % 12;
    let day_of_year = (153 * month_prime + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    i64::from(era) * 146_097 + day_of_era - 719_468
}
