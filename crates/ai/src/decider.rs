//! Optional local Decider-2B adapter.
//!
//! The adapter speaks the documented typed readout API (`POST /v1/systemone`
//! with `state`, `questions`, `model`), never a chat completion. It is off by
//! default, sends only the masked projection, and any timeout, HTTP error,
//! truncation or unparsable answer falls back to the baseline heuristic with a
//! visible reason. A returned confidence is preference concentration for the
//! chosen option, never a win probability.

use aperture_core::Action;
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
    #[error("request failed: {0}")]
    Transport(String),
    #[error("the service answered HTTP {status}: {body}")]
    Http { status: u16, body: String },
    #[error("could not parse the response: {0}")]
    Parse(String),
    #[error("the response carried no usable choice")]
    NoChoice,
    #[error("unknown option key '{0}'")]
    UnknownKey(String),
    #[error(
        "the service truncated the state ({detail}); the answer is refused because the model did not see the whole projection"
    )]
    Truncated { detail: String },
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
    /// Probability mass on the reported answer (provider field).
    pub answer_confidence: f32,
    /// Entropy concentration (provider field).
    pub confidence: f32,
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
        "type": if card.card.def().is_creature() { "creature" } else if card.card.def().is_land() { "land" } else { "spell" },
        "power": card.power,
        "toughness": card.toughness,
        "tapped": card.tapped,
        "summoning_sick": card.summoning_sick,
        "mana_cost": card.cost,
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
    life.insert(
        format!("P{}", aperture_core::opponent(view.viewer)),
        json!(view.opponent_life),
    );

    let state = json!({
        "game": "Magic: The Gathering practice slice (1v1, 20 life)",
        "you_are": format!("P{}", view.viewer),
        "turn": view.turn_number,
        "step": view.step.name(),
        "active_player": format!("P{}", view.active),
        "you_have_priority": view.priority == Some(view.viewer),
        "life": Value::Object(life),
        "your_hand": view.own_hand.iter().map(card_json).collect::<Vec<_>>(),
        "your_hand_count": view.own_hand.len(),
        "your_battlefield": view.own_battlefield.iter().map(card_json).collect::<Vec<_>>(),
        "your_mana_pool": view.own_mana,
        "your_library_count": view.own_library_count,
        "your_graveyard": view.own_graveyard.iter().map(|c| c.name).collect::<Vec<_>>(),
        "opponent_battlefield": view.opponent_battlefield.iter().map(card_json).collect::<Vec<_>>(),
        "opponent_hand_count": view.opponent_hand_count,
        "opponent_library_count": view.opponent_library_count,
        "opponent_graveyard": view.opponent_graveyard.iter().map(|c| c.name).collect::<Vec<_>>(),
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
    /// A truncated state means the model did not see the whole projection, so
    /// the answer is refused with [`DeciderError::Truncated`] even when a valid
    /// choice is present; the caller then falls back to the heuristic.
    pub fn choose(&self, request: &DecisionRequest) -> Result<ChoiceAnswer, DeciderError> {
        if request.options.is_empty() {
            return Err(DeciderError::NoOptions);
        }
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
        let text = response
            .text()
            .map_err(|e| DeciderError::Transport(e.to_string()))?;
        if !status.is_success() {
            return Err(DeciderError::Http {
                status: status.as_u16(),
                body: text.chars().take(200).collect(),
            });
        }
        let value: Value =
            serde_json::from_str(&text).map_err(|e| DeciderError::Parse(e.to_string()))?;
        if let Some(usage) = value.get("usage") {
            let truncated_flag = usage
                .get("truncated")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let dropped = usage
                .get("state_tokens_dropped")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let truncated_questions: Vec<String> = usage
                .get("truncated_questions")
                .and_then(|v| v.as_array())
                .map(|list| {
                    list.iter()
                        .filter_map(|q| q.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            if truncated_flag || dropped > 0 || !truncated_questions.is_empty() {
                return Err(DeciderError::Truncated {
                    detail: format!(
                        "state_tokens_dropped={dropped}, truncated={truncated_flag}, \
truncated_questions=[{}]",
                        truncated_questions.join(", ")
                    ),
                });
            }
        }
        let answer = value
            .get("answers")
            .and_then(|a| a.get("decision"))
            .ok_or_else(|| DeciderError::Parse("response has no answers.decision".to_string()))?;
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
            answer_confidence: answer
                .get("answer_confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
            confidence: answer
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
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

    /// Resolve the answered key back to a legal action.
    pub fn resolve<'a>(&self, view: &'a PlayerView, key: &str) -> Option<&'a Action> {
        view.legal
            .iter()
            .enumerate()
            .find(|(index, _)| option_key(*index) == key)
            .map(|(_, legal)| &legal.action)
    }
}

impl Policy for DeciderPolicy {
    fn name(&self) -> &str {
        "decider-2b"
    }

    fn decide(&mut self, view: &PlayerView) -> Decision {
        let fallback = self.heuristic.decide(view);
        if view.legal.len() < MIN_OPTIONS {
            // A forced move is not a decision to ask about; saying so keeps the
            // audit trail honest instead of reporting a spurious fallback.
            return Decision {
                action: fallback.action,
                source: DecisionSource::Forced,
                note: Some(format!(
                    "only {} legal option(s): the choice is forced and no model call was made",
                    view.legal.len()
                )),
                preference: None,
            };
        }
        let request = match decision_request(view) {
            Ok(request) => request,
            Err(err) => {
                return Decision {
                    action: fallback.action,
                    source: DecisionSource::DeciderFallback,
                    note: Some(format!("no decision request sent: {err}")),
                    preference: None,
                };
            }
        };
        match self.client.choose(&request) {
            Ok(answer) => match self.resolve(view, &answer.key) {
                Some(action) => Decision {
                    action: action.clone(),
                    source: DecisionSource::Decider,
                    note: Some(format!(
                        "model {} chose {} (preference {:.3}; not a win probability)",
                        answer.model.as_deref().unwrap_or("decider-2b"),
                        answer.label,
                        answer.answer_confidence
                    )),
                    preference: Some(answer.answer_confidence),
                },
                None => Decision {
                    action: fallback.action,
                    source: DecisionSource::DeciderFallback,
                    note: Some(format!(
                        "model answered an unknown option key '{}'; used the heuristic",
                        answer.key
                    )),
                    preference: Some(answer.answer_confidence),
                },
            },
            Err(err) => Decision {
                action: fallback.action,
                source: DecisionSource::DeciderFallback,
                note: Some(format!("decider-2b unavailable, used the heuristic: {err}")),
                preference: None,
            },
        }
    }
}
