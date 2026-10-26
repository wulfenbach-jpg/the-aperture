//! Optional local Decider-2B adapter.
//!
//! The adapter speaks the documented typed readout API (`POST /v1/systemone`
//! with `state`, `questions`, `model`), never a chat completion. It is off by
//! default, sends only the masked projection, and any timeout, HTTP error,
//! malformed/oversized response, truncation or unparsable answer falls back to
//! the deterministic baseline heuristic with a visible reason code. A returned
//! confidence is preference concentration for the chosen option, never a win
//! probability or a search evaluation.
//!
//! A decision is applied only through [`DecisionEnvelope`], which binds the
//! masked request to the viewer, revision, decision serial and session
//! generation and keeps an immutable snapshot of the engine-offered actions.
//! The service does not echo those identifiers (the installed typed API has no
//! field for them), so the adapter resolves the opaque option key against that
//! snapshot and revalidates the resolved action against the live legal offer.

use std::collections::HashSet;
use std::io::Read;

use aperture_core::Action;
use aperture_core::PlayerId;
use aperture_core::view::PlayerView;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::heuristic::HeuristicPolicy;
use crate::{Decision, DecisionSource, Policy};

/// Maximum number of options asked in one question. The provider's typed
/// readout answers choices in one request; above this the adapter refuses
/// instead of silently dropping legal options.
pub const MAX_OPTIONS: usize = 90;

/// The typed readout requires 2 to 100 named criteria for a choice question
/// (verified against the live service: a one-option request is answered with
/// HTTP 422). A single legal action is therefore forced and never asked.
pub const MIN_OPTIONS: usize = 2;

/// Hard bound on one typed-response body. A local protocol response is tiny;
/// this keeps a misbehaving or hostile loopback service from growing memory.
pub const MAX_RESPONSE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeciderConfig {
    pub base_url: String,
    pub model: String,
    pub timeout_ms: u64,
    pub max_len: u32,
}

impl Default for DeciderConfig {
    fn default() -> Self {
        DeciderConfig {
            base_url: "http://127.0.0.1:8765".to_string(),
            model: "decider-2b".to_string(),
            timeout_ms: 4_000,
            // The masked projection for a real position runs past 512 tokens
            // (a live probe measured state_tokens_dropped=54..184 at 512), which
            // would make every request truncated and therefore refused. 1024 is
            // the service's packaged default for non-English checkpoints and
            // stays inside the bounded context the local service advertises.
            max_len: 1024,
        }
    }
}

#[derive(Debug, Error)]
pub enum DeciderError {
    #[error("the endpoint must be a loopback http:// URL without credentials (got '{0}')")]
    BadEndpoint(String),
    #[error("no legal action to choose from")]
    NoOptions,
    #[error("only {0} legal option(s): the choice is forced, not a model question")]
    TooFewOptions(usize),
    #[error("{0} legal options exceed the {MAX_OPTIONS}-option limit for one request")]
    TooManyOptions(usize),
    #[error("an offered option key is empty")]
    EmptyOptionKey,
    #[error("the offered option key '{0}' appears more than once")]
    DuplicateOptionKey(String),
    #[error("request failed: {0}")]
    Transport(String),
    #[error("the service answered HTTP {status}: {body}")]
    Http { status: u16, body: String },
    #[error("the response body exceeded the {limit}-byte limit")]
    ResponseTooLarge { limit: usize },
    #[error("could not parse the response: {0}")]
    Parse(String),
    #[error("the usage schema is malformed: {detail}")]
    MalformedUsage { detail: String },
    #[error("the response field '{field}' is not a valid confidence: {detail}")]
    InvalidConfidence { field: &'static str, detail: String },
    #[error("the response carried no usable choice")]
    NoChoice,
    #[error("unknown option key '{0}'")]
    UnknownKey(String),
    #[error(
        "the service truncated the state ({detail}); the answer is refused because the model did not see the whole projection"
    )]
    Truncated { detail: String },
    #[error("the envelope belongs to P{expected}, not the observed P{got}")]
    WrongSeat { expected: PlayerId, got: PlayerId },
    #[error(
        "the envelope was prepared in session generation {observed}; the current generation is {current}"
    )]
    StaleGeneration { observed: u64, current: u64 },
    #[error("the envelope observed revision {observed}; the current revision is {current}")]
    StaleRevision { observed: u64, current: u64 },
    #[error("the envelope observed decision {observed}; the current decision is {current}")]
    StaleDecision { observed: u64, current: u64 },
    #[error("the chosen action is not in the live legal offer: {detail}")]
    IllegalChoice { detail: String },
}

impl DeciderError {
    /// Stable, machine-readable fallback reason. Visible notes carry this code
    /// so callers do not have to parse prose to distinguish a truncated answer
    /// from a stale envelope or an illegal choice.
    pub fn reason_code(&self) -> &'static str {
        match self {
            DeciderError::BadEndpoint(_) => "bad_endpoint",
            DeciderError::NoOptions => "no_options",
            DeciderError::TooFewOptions(_) => "too_few_options",
            DeciderError::TooManyOptions(_) => "too_many_options",
            DeciderError::EmptyOptionKey => "empty_option_key",
            DeciderError::DuplicateOptionKey(_) => "duplicate_option_key",
            DeciderError::Transport(_) => "transport",
            DeciderError::Http { .. } => "http_status",
            DeciderError::ResponseTooLarge { .. } => "response_too_large",
            DeciderError::Parse(_) => "malformed_response",
            DeciderError::MalformedUsage { .. } => "malformed_usage",
            DeciderError::InvalidConfidence { .. } => "invalid_confidence",
            DeciderError::NoChoice => "no_choice",
            DeciderError::UnknownKey(_) => "unknown_key",
            DeciderError::Truncated { .. } => "truncated",
            DeciderError::WrongSeat { .. } => "wrong_seat",
            DeciderError::StaleGeneration { .. } => "stale_generation",
            DeciderError::StaleRevision { .. } => "stale_revision",
            DeciderError::StaleDecision { .. } => "stale_decision",
            DeciderError::IllegalChoice { .. } => "illegal_choice",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionSpec {
    /// Opaque short key sent as the criteria key ("a", "b", ...).
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionRequest {
    /// Masked state: own hand and public zones only.
    pub state: Value,
    pub instructions: String,
    pub options: Vec<OptionSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceAnswer {
    pub key: String,
    pub label: String,
    /// Probability mass on the reported answer (provider field), when present.
    /// This is preference concentration for the chosen option, never a win
    /// probability or a search evaluation.
    pub answer_confidence: Option<f32>,
    /// Entropy concentration (provider field), when present.
    pub confidence: Option<f32>,
    pub model: Option<String>,
}

/// Values observed when the caller asked for a decision. Applying an answer
/// requires presenting the same binding again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionBinding {
    pub viewer: PlayerId,
    pub revision: u64,
    pub decision: u64,
    pub generation: u64,
}

/// Typed local request plus the immutable engine-offered action snapshot it was
/// built from. The service cannot supply revision/decision/generation ids, so
/// the envelope is the authority for what was asked and offered.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionEnvelope {
    binding: DecisionBinding,
    request: DecisionRequest,
    offered: Vec<Action>,
}

impl DecisionEnvelope {
    /// Build one envelope. `binding.viewer` must be the seat this masked view
    /// belongs to; the offered snapshot is taken from that same view.
    pub fn prepare(
        view: &PlayerView,
        binding: DecisionBinding,
    ) -> Result<DecisionEnvelope, DeciderError> {
        if binding.viewer != view.viewer {
            return Err(DeciderError::WrongSeat {
                expected: view.viewer,
                got: binding.viewer,
            });
        }
        let request = decision_request(view)?;
        let offered: Vec<Action> = view
            .legal
            .iter()
            .map(|legal| legal.action.clone())
            .collect();
        debug_assert_eq!(request.options.len(), offered.len());
        Ok(DecisionEnvelope {
            binding,
            request,
            offered,
        })
    }

    pub fn binding(&self) -> DecisionBinding {
        self.binding
    }

    pub fn request(&self) -> &DecisionRequest {
        &self.request
    }

    pub fn offered(&self) -> &[Action] {
        &self.offered
    }

    /// Resolve the returned opaque key against the original offered snapshot.
    pub fn resolve(&self, answer: &ChoiceAnswer) -> Result<Proposal, DeciderError> {
        let index = self
            .request
            .options
            .iter()
            .position(|option| option.key == answer.key)
            .ok_or_else(|| DeciderError::UnknownKey(answer.key.clone()))?;
        let action = self
            .offered
            .get(index)
            .cloned()
            .ok_or_else(|| DeciderError::UnknownKey(answer.key.clone()))?;
        Ok(Proposal {
            key: answer.key.clone(),
            action,
            preference: answer.answer_confidence,
            model: answer.model.clone(),
        })
    }
}

/// An answer resolved against the envelope's offered snapshot. It is not
/// applied until the live legal offer is revalidated.
#[derive(Debug, Clone, PartialEq)]
pub struct Proposal {
    pub key: String,
    pub action: Action,
    /// Reported preference concentration only; never a win probability.
    pub preference: Option<f32>,
    pub model: Option<String>,
}

pub fn option_key(index: usize) -> String {
    let mut n = index;
    let mut out = String::new();
    loop {
        out.insert(0, (b'a' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    out
}

fn card_json(card: &aperture_core::view::CardView) -> Value {
    json!({
        "name": card.name,
        "type": if card.card.def().is_creature() {
            "creature"
        } else if card.card.def().is_land() {
            "land"
        } else {
            "spell"
        },
        "power": card.power,
        "toughness": card.toughness,
        "damage": card.damage,
        "tapped": card.tapped,
        "summoning_sick": card.summoning_sick,
        "plus_one_counters": card.plus_one_counters,
        "pump_power": card.pump_power,
        "pump_toughness": card.pump_toughness,
        "mana_cost": card.cost,
        // Printed text is the only keyword source the masked view carries;
        // do not infer keywords the engine has not modelled.
        "oracle": card.oracle,
    })
}

/// Build one masked decision request from the seat's projection.
pub fn decision_request(view: &PlayerView) -> Result<DecisionRequest, DeciderError> {
    if view.legal.len() < MIN_OPTIONS {
        return Err(if view.legal.is_empty() {
            DeciderError::NoOptions
        } else {
            DeciderError::TooFewOptions(view.legal.len())
        });
    }
    if view.legal.len() > MAX_OPTIONS {
        return Err(DeciderError::TooManyOptions(view.legal.len()));
    }
    let options: Vec<OptionSpec> = view
        .legal
        .iter()
        .enumerate()
        .map(|(index, legal)| OptionSpec {
            key: option_key(index),
            label: legal.label.clone(),
        })
        .collect();

    let mut life = serde_json::Map::new();
    life.insert(format!("P{}", view.viewer), json!(view.own_life));
    for opponent in &view.opponents {
        life.insert(format!("P{}", opponent.seat), json!(opponent.life));
    }

    let state = json!({
        // No starting-life assertion: the same projection serves every named
        // profile (20- and 40-life practice), so only live totals are reported.
        "format": "Magic: The Gathering practice slice",
        "seat_count": view.seat_count,
        "you_are": format!("P{}", view.viewer),
        "your_name": view.viewer_name,
        "turn": view.turn_number,
        "step": view.step.name(),
        "active_player": format!("P{}", view.active),
        "you_have_priority": view.priority == Some(view.viewer),
        "you_are_eliminated": view.own_eliminated,
        "life": Value::Object(life),
        "your_hand": view.own_hand.iter().map(card_json).collect::<Vec<_>>(),
        "your_hand_count": view.own_hand.len(),
        "your_battlefield": view.own_battlefield.iter().map(card_json).collect::<Vec<_>>(),
        "your_mana_pool": view.own_mana,
        "your_mana_pool_detail": view.own_mana_pool,
        "your_lands_in_hand": view.own_lands_in_hand,
        "your_mulligans": view.own_mulligans,
        "your_library_count": view.own_library_count,
        "your_graveyard": view.own_graveyard.iter().map(|c| c.name).collect::<Vec<_>>(),
        "your_exile": view.own_exile.iter().map(card_json).collect::<Vec<_>>(),
        "your_command_zone": view.own_command_zone.iter().map(card_json).collect::<Vec<_>>(),
        // Command-zone identities and damage are public information; the exact
        // starting-life profile is not exposed by PlayerView and is not guessed.
        "command_zone_present": !view.commanders.is_empty(),
        "commanders": view.commanders.iter().map(|c| json!({
            "owner": format!("P{}", c.owner),
            "name": c.name,
            "zone": c.zone,
            "casts_from_command_zone": c.casts_from_command_zone,
            "tax": c.tax,
        })).collect::<Vec<_>>(),
        "commander_damage": view.commander_damage.iter().map(|d| json!({
            "commander": d.commander,
            "recipient": format!("P{}", d.recipient),
            "amount": d.amount,
        })).collect::<Vec<_>>(),
        "opponents": view.opponents.iter().map(|o| json!({
            "seat": format!("P{}", o.seat),
            "name": o.name,
            "deck_name": o.deck_name,
            "eliminated": o.eliminated,
            "life": o.life,
            "hand_count": o.hand_count,
            "library_count": o.library_count,
            "battlefield": o.battlefield.iter().map(card_json).collect::<Vec<_>>(),
            "graveyard": o.graveyard.iter().map(|c| c.name).collect::<Vec<_>>(),
            "exile": o.exile.iter().map(card_json).collect::<Vec<_>>(),
            "command_zone": o.command_zone.iter().map(card_json).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
        "stack": view.stack.iter().map(|item| json!({
            "name": item.name,
            "controller": format!("P{}", item.controller),
            "targets": item.targets,
        })).collect::<Vec<_>>(),
        "waiting_for": view.pending,
        "pending_viewed": view.pending_viewed.iter().map(|c| c.name).collect::<Vec<_>>(),
        "legal_actions": options.iter().map(|o| o.label.clone()).collect::<Vec<_>>(),
    });

    Ok(DecisionRequest {
        state,
        instructions: "Choose one legal action for the described seat of a Magic: The Gathering \
practice game. Answer with the criteria key of exactly one offered option; do not invent options."
            .to_string(),
        options,
    })
}

#[derive(Debug)]
pub struct DeciderClient {
    client: reqwest::blocking::Client,
    config: DeciderConfig,
}

fn loopback_ok(base_url: &str) -> bool {
    let Some(rest) = base_url.strip_prefix("http://") else {
        return false;
    };
    if rest.contains('@') || rest.contains('/') {
        return false;
    }
    let (host, port) = match rest.split_once(':') {
        Some((host, port)) => (host, port),
        None => (rest, ""),
    };
    if !matches!(host, "127.0.0.1" | "localhost" | "[::1]" | "::1") {
        return false;
    }
    port.parse::<u16>().map(|p| p > 0).unwrap_or(false)
}

fn validate_options(request: &DecisionRequest) -> Result<(), DeciderError> {
    if request.options.len() < MIN_OPTIONS {
        return Err(DeciderError::TooFewOptions(request.options.len()));
    }
    if request.options.len() > MAX_OPTIONS {
        return Err(DeciderError::TooManyOptions(request.options.len()));
    }
    let mut keys = HashSet::with_capacity(request.options.len());
    for option in &request.options {
        if option.key.is_empty() {
            return Err(DeciderError::EmptyOptionKey);
        }
        if !keys.insert(option.key.as_str()) {
            return Err(DeciderError::DuplicateOptionKey(option.key.clone()));
        }
    }
    Ok(())
}

fn malformed_usage(field: &str, detail: &str) -> DeciderError {
    DeciderError::MalformedUsage {
        detail: format!("{field} {detail}"),
    }
}

/// Fail closed on the truncation schema: a missing field or a wrong type is a
/// malformed response, never a clean one.
fn validate_usage(value: &Value) -> Result<(), DeciderError> {
    let usage = value
        .get("usage")
        .ok_or_else(|| malformed_usage("usage", "is missing"))?
        .as_object()
        .ok_or_else(|| malformed_usage("usage", "is not an object"))?;
    let truncated = usage
        .get("truncated")
        .ok_or_else(|| malformed_usage("usage.truncated", "is missing"))?
        .as_bool()
        .ok_or_else(|| malformed_usage("usage.truncated", "is not a boolean"))?;
    let dropped = usage
        .get("state_tokens_dropped")
        .ok_or_else(|| malformed_usage("usage.state_tokens_dropped", "is missing"))?
        .as_u64()
        .ok_or_else(|| {
            malformed_usage(
                "usage.state_tokens_dropped",
                "is not a non-negative integer",
            )
        })?;
    let questions = usage
        .get("truncated_questions")
        .ok_or_else(|| malformed_usage("usage.truncated_questions", "is missing"))?
        .as_array()
        .ok_or_else(|| malformed_usage("usage.truncated_questions", "is not an array"))?;
    let mut truncated_questions = Vec::with_capacity(questions.len());
    for question in questions {
        truncated_questions.push(
            question
                .as_str()
                .ok_or_else(|| {
                    malformed_usage("usage.truncated_questions", "contains a non-string item")
                })?
                .to_string(),
        );
    }
    if truncated || dropped > 0 || !truncated_questions.is_empty() {
        return Err(DeciderError::Truncated {
            detail: format!(
                "state_tokens_dropped={dropped}, truncated={truncated}, \
truncated_questions=[{}]",
                truncated_questions.join(", ")
            ),
        });
    }
    Ok(())
}

fn confidence_field(answer: &Value, field: &'static str) -> Result<Option<f32>, DeciderError> {
    let Some(value) = answer.get(field) else {
        // The installed readout may omit a concentration field; an absent field
        // is not a reported zero.
        return Ok(None);
    };
    let number = value
        .as_f64()
        .ok_or_else(|| DeciderError::InvalidConfidence {
            field,
            detail: "not a number".to_string(),
        })?;
    if !number.is_finite() {
        return Err(DeciderError::InvalidConfidence {
            field,
            detail: "not finite".to_string(),
        });
    }
    if !(0.0..=1.0).contains(&number) {
        return Err(DeciderError::InvalidConfidence {
            field,
            detail: format!("{number} is outside [0, 1]"),
        });
    }
    Ok(Some(number as f32))
}

impl DeciderClient {
    pub fn new(config: DeciderConfig) -> Result<Self, DeciderError> {
        if !loopback_ok(&config.base_url) {
            return Err(DeciderError::BadEndpoint(config.base_url.clone()));
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout_ms))
            .connect_timeout(std::time::Duration::from_millis(
                config.timeout_ms.min(2_000),
            ))
            // A loopback service must not be able to redirect the masked payload
            // to another target, and an environment proxy must not silently
            // reroute the request.
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .map_err(|e| DeciderError::Transport(e.to_string()))?;
        Ok(DeciderClient { client, config })
    }

    pub fn config(&self) -> &DeciderConfig {
        &self.config
    }

    pub fn health(&self) -> Result<bool, DeciderError> {
        let url = format!("{}/health", self.config.base_url);
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|e| DeciderError::Transport(e.to_string()))?;
        Ok(response.status().is_success())
    }

    /// Ask the installed typed readout which offered option to take.
    ///
    /// A malformed usage schema, an invalid confidence, an oversized body or a
    /// truncated state is refused; the caller then falls back to the heuristic.
    /// Directly constructed requests are validated too, so they cannot bypass
    /// the provider's cardinality or unique-key requirements.
    pub fn choose(&self, request: &DecisionRequest) -> Result<ChoiceAnswer, DeciderError> {
        validate_options(request)?;
        let mut criteria = serde_json::Map::new();
        for option in &request.options {
            criteria.insert(option.key.clone(), Value::String(option.label.clone()));
        }
        let body = json!({
            "state": request.state,
            "questions": {
                "decision": {
                    "type": "choice",
                    "instructions": request.instructions,
                    "criteria": Value::Object(criteria),
                }
            },
            "model": self.config.model,
            "max_len": self.config.max_len,
        });
        let url = format!("{}/v1/systemone", self.config.base_url);
        let response = self
            .client
            .post(url)
            .json(&body)
            .send()
            .map_err(|e| DeciderError::Transport(e.to_string()))?;
        let status = response.status();
        let mut reader = response.take((MAX_RESPONSE_BYTES as u64) + 1);
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|e| DeciderError::Transport(e.to_string()))?;
        if bytes.len() > MAX_RESPONSE_BYTES {
            return Err(DeciderError::ResponseTooLarge {
                limit: MAX_RESPONSE_BYTES,
            });
        }
        let text = String::from_utf8(bytes)
            .map_err(|e| DeciderError::Parse(format!("response is not UTF-8: {e}")))?;
        if !status.is_success() {
            return Err(DeciderError::Http {
                status: status.as_u16(),
                body: text.chars().take(200).collect(),
            });
        }
        let value: Value =
            serde_json::from_str(&text).map_err(|e| DeciderError::Parse(e.to_string()))?;
        validate_usage(&value)?;
        let answer = value
            .get("answers")
            .and_then(|a| a.get("decision"))
            .ok_or_else(|| DeciderError::Parse("response has no answers.decision".to_string()))?;
        // The installed fixtures may omit `type`; when it is present it must be
        // the supported choice readout. A score readout is not a choice answer.
        if let Some(kind) = answer.get("type") {
            let kind = kind.as_str().ok_or_else(|| {
                DeciderError::Parse("answers.decision.type is not a string".to_string())
            })?;
            if kind != "choice" {
                return Err(DeciderError::Parse(format!(
                    "answers.decision.type '{kind}' is not a supported choice"
                )));
            }
        }
        let key = answer
            .get("choice")
            .and_then(|c| c.as_str())
            .ok_or(DeciderError::NoChoice)?
            .to_string();
        let label = request
            .options
            .iter()
            .find(|o| o.key == key)
            .map(|o| o.label.clone())
            .ok_or_else(|| DeciderError::UnknownKey(key.clone()))?;
        Ok(ChoiceAnswer {
            key,
            label,
            answer_confidence: confidence_field(answer, "answer_confidence")?,
            confidence: confidence_field(answer, "confidence")?,
            model: value
                .get("routing")
                .and_then(|r| r.get("model"))
                .or_else(|| value.get("model"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string()),
        })
    }
}

/// Policy that consults the local model and always keeps a legal baseline.
pub struct DeciderPolicy {
    client: DeciderClient,
    heuristic: HeuristicPolicy,
}

fn forced_decision(action: Action, count: usize) -> Decision {
    Decision {
        action,
        source: DecisionSource::Forced,
        note: Some(format!(
            "only {count} legal option(s): the choice is forced and no model call was made"
        )),
        preference: None,
    }
}

fn fallback_decision(fallback: Decision, err: &DeciderError) -> Decision {
    Decision {
        action: fallback.action,
        source: DecisionSource::DeciderFallback,
        note: Some(format!(
            "decider-2b unavailable, used the heuristic [{}]: {err}",
            err.reason_code()
        )),
        // Preference concentration belongs to the rejected model proposal, not
        // to the heuristic action eventually taken.
        preference: None,
    }
}

fn preference_note(preference: Option<f32>) -> String {
    match preference {
        Some(value) => format!("preference {value:.3}; not a win probability"),
        None => "preference not reported; not a win probability".to_string(),
    }
}

impl DeciderPolicy {
    pub fn new(config: DeciderConfig) -> Result<Self, DeciderError> {
        Ok(DeciderPolicy {
            client: DeciderClient::new(config)?,
            heuristic: HeuristicPolicy::new(),
        })
    }

    pub fn client(&self) -> &DeciderClient {
        &self.client
    }

    /// Resolve the answered key back to a legal action in this view.
    pub fn resolve<'a>(&self, view: &'a PlayerView, key: &str) -> Option<&'a Action> {
        view.legal
            .iter()
            .enumerate()
            .find(|(index, _)| option_key(*index) == key)
            .map(|(_, legal)| &legal.action)
    }

    fn check_envelope(
        &self,
        view: &PlayerView,
        envelope: &DecisionEnvelope,
        current: DecisionBinding,
    ) -> Result<(), DeciderError> {
        if envelope.binding.viewer != view.viewer {
            return Err(DeciderError::WrongSeat {
                expected: view.viewer,
                got: envelope.binding.viewer,
            });
        }
        if current.viewer != view.viewer {
            return Err(DeciderError::WrongSeat {
                expected: view.viewer,
                got: current.viewer,
            });
        }
        if envelope.binding.generation != current.generation {
            return Err(DeciderError::StaleGeneration {
                observed: envelope.binding.generation,
                current: current.generation,
            });
        }
        if envelope.binding.revision != current.revision {
            return Err(DeciderError::StaleRevision {
                observed: envelope.binding.revision,
                current: current.revision,
            });
        }
        if envelope.binding.decision != current.decision {
            return Err(DeciderError::StaleDecision {
                observed: envelope.binding.decision,
                current: current.decision,
            });
        }
        Ok(())
    }

    /// Generation-aware entry point: ask the model through a previously
    /// prepared envelope and apply the answer only when the binding still
    /// matches and the resolved action is in the live legal offer.
    pub fn decide_bound(
        &mut self,
        view: &PlayerView,
        envelope: &DecisionEnvelope,
        current: DecisionBinding,
    ) -> Decision {
        let fallback = self.heuristic.decide(view);
        // Binding staleness wins over the forced choice: a forced view must not
        // erase the reason an envelope can no longer be applied.
        if let Err(err) = self.check_envelope(view, envelope, current) {
            return fallback_decision(fallback, &err);
        }
        if view.legal.len() < MIN_OPTIONS {
            return forced_decision(fallback.action, view.legal.len());
        }
        match self.client.choose(&envelope.request) {
            Ok(answer) => match envelope.resolve(&answer) {
                Ok(proposal) => {
                    if !view
                        .legal
                        .iter()
                        .any(|legal| legal.action == proposal.action)
                    {
                        let err = DeciderError::IllegalChoice {
                            detail: format!(
                                "resolved key '{}' outside the current engine offer",
                                proposal.key
                            ),
                        };
                        return fallback_decision(fallback, &err);
                    }
                    Decision {
                        action: proposal.action,
                        source: DecisionSource::Decider,
                        note: Some(format!(
                            "model {} chose {} ({})",
                            proposal.model.as_deref().unwrap_or("decider-2b"),
                            answer.label,
                            preference_note(proposal.preference)
                        )),
                        preference: proposal.preference,
                    }
                }
                Err(err) => fallback_decision(fallback, &err),
            },
            Err(err) => fallback_decision(fallback, &err),
        }
    }
}

impl Policy for DeciderPolicy {
    fn name(&self) -> &str {
        "decider-2b"
    }

    /// Synchronous policy entry point. It cannot observe revision/decision
    /// serials (PlayerView does not carry them), so it uses generation 0 and
    /// zeroed binding values; callers that have those values use
    /// [`DeciderPolicy::decide_bound`] instead. The UI keeps its independent
    /// generation guard around either path.
    fn decide(&mut self, view: &PlayerView) -> Decision {
        let fallback = self.heuristic.decide(view);
        if view.legal.len() < MIN_OPTIONS {
            // A forced move is not a decision to ask about; saying so keeps the
            // audit trail honest instead of reporting a spurious fallback.
            return forced_decision(fallback.action, view.legal.len());
        }
        let binding = DecisionBinding {
            viewer: view.viewer,
            revision: 0,
            decision: 0,
            generation: 0,
        };
        match DecisionEnvelope::prepare(view, binding) {
            Ok(envelope) => self.decide_bound(view, &envelope, binding),
            Err(err) => Decision {
                action: fallback.action,
                source: DecisionSource::DeciderFallback,
                note: Some(format!(
                    "no decision request sent [{}]: {err}",
                    err.reason_code()
                )),
                preference: None,
            },
        }
    }
}
