//! Policies for THE APERTURE: a deterministic baseline heuristic, an optional
//! local Decider adapter, and a self-play driver.

mod decider;
mod heuristic;
mod search;
mod selfplay;

pub use decider::{
    ChoiceAnswer, DeciderClient, DeciderConfig, DeciderError, DeciderPolicy, DecisionRequest,
    OptionSpec, decision_request,
};
pub use heuristic::{HeuristicPolicy, available_mana, card_mana_value, land_color, pool_size};
pub use search::{
    CandidateScore, Knowledge, SearchAnalysis, SearchCancellation, SearchConfig, SearchOutcome,
    SearchUnavailable, determinization_seed, fallback, search,
};
pub use selfplay::{RunStatus, SelfplayLimits, SelfplayOutcome, play_to_end};

use aperture_core::Action;
use aperture_core::view::PlayerView;

/// Where a decision came from. Fallbacks are never silent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DecisionSource {
    Heuristic,
    Decider,
    DeciderFallback,
    /// Budgeted information-set search chose the action from offered ones
    /// only (no real opponent hands or library order were read).
    Search,
    /// Only one legal action exists, so the choice is forced and no model call
    /// is made (the typed readout requires 2-100 criteria).
    Forced,
}

impl DecisionSource {
    pub fn label(self) -> &'static str {
        match self {
            DecisionSource::Heuristic => "heuristic",
            DecisionSource::Decider => "decider-2b",
            DecisionSource::DeciderFallback => "heuristic (decider fallback)",
            DecisionSource::Search => "information-set search",
            DecisionSource::Forced => "forced (single legal option)",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub action: Action,
    pub source: DecisionSource,
    /// Visible reason when the model was consulted and did not answer.
    pub note: Option<String>,
    /// Model preference concentration for the chosen option. Never a win
    /// probability.
    pub preference: Option<f32>,
}

impl Decision {
    pub fn heuristic(action: Action) -> Self {
        Decision {
            action,
            source: DecisionSource::Heuristic,
            note: None,
            preference: None,
        }
    }
}

/// A policy sees exactly one thing: a masked projection of the game.
/// It cannot reach hidden zones because it never receives `Game`.
pub trait Policy {
    fn name(&self) -> &str;
    fn decide(&mut self, view: &PlayerView) -> Decision;
}
