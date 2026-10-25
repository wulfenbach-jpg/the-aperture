//! THE APERTURE — a deliberately narrow Magic: The Gathering rules lab.
//!
//! Scope contract: a 1v1 constructed *practice slice* with a small, explicitly
//! implemented card pool, 20 life, 40-card decks, London mulligan. This is not
//! Commander and not full Magic. Every state transition in this crate is
//! validated before mutation; a rejected action leaves the state untouched.
//!
//! Pinned rules revision: Comprehensive Rules effective 2026-09-25
//! (`RULES_REVISION`). Card behaviors outside the supported pool are not
//! emulated: they are rejected at import time.

pub mod ability;
pub mod action;
pub mod card;
pub mod catalog;
pub mod coach;
pub mod deck;
pub mod digest;
pub mod engine;
pub mod error;
pub mod event;
pub mod ids;
pub mod mana;
pub mod replay;
pub mod rng;
pub mod settings;
pub mod state;
pub mod view;

pub use ability::{
    Ability, AbilityCost, AbilityMode, ActivatedAbility, TriggerEvent, TriggeredAbility, abilities,
};
pub use action::{Action, BlockDecl, LegalAction, Submission, Target};
pub use card::{CardDef, CardId, Effect, Kind, ManaColor, SpellSpec, TargetSpec};
pub use deck::DeckList;
pub use engine::Game;
pub use error::EngineError;
pub use event::Event;
pub use ids::{CommanderId, MAX_PLAYERS, MIN_PLAYERS, ObjectId, P0, P1, PlayerId, Zone};
pub use mana::{ManaCost, ManaPool};
pub use replay::{CommanderSetup, Replay, ReplayError, ReplayStep, SessionSave};
pub use settings::AiSettings;
pub use state::{
    Commander, CommanderDamageEntry, DrawResume, GameState, OrderMode, Outcome, Pending, Profile,
    StackEntry, Step, TriggerRef,
};
pub use view::{OpponentView, PlayerView};

/// Comprehensive Rules revision this engine was written against.
pub const RULES_REVISION: &str = "2026-09-25";
/// Product label shown in every UI surface; keeps the scope honest.
pub const PRODUCT_LABEL: &str = "THE APERTURE — practice slice, supported card pool, 2-4 live seats, named 20-life and bounded 40-life profiles, NOT Commander";
