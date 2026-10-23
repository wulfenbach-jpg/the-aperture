use serde::{Deserialize, Serialize};

use crate::ids::{ObjectId, PlayerId};

/// A chosen target. The slice supports one target per spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Target {
    Player(PlayerId),
    /// A permanent on the battlefield.
    Permanent(ObjectId),
    /// A spell on the stack, identified by the stack object's id.
    Spell(ObjectId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockDecl {
    pub blocker: ObjectId,
    pub attacker: ObjectId,
}

/// Everything a player can submit. Legal-action generation produces these; the
/// engine validates every one before mutating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Pass priority (CR 117.3d).
    Pass,
    PlayLand {
        card: ObjectId,
    },
    CastSpell {
        card: ObjectId,
        targets: Vec<Target>,
    },
    ActivateManaAbility {
        land: ObjectId,
    },
    /// Structured attacker declaration. In the bounded M1 multiplayer model every
    /// attacker in one declaration attacks the same defending seat; `defender`
    /// is `None` exactly when no attackers are declared.
    DeclareAttackers {
        attackers: Vec<ObjectId>,
        defender: Option<PlayerId>,
    },
    DeclareBlockers {
        blocks: Vec<BlockDecl>,
    },
    Mulligan {
        keep: bool,
    },
    /// London mulligan bottoming (CR 103.5).
    BottomCards {
        cards: Vec<ObjectId>,
    },
    /// Scry/surveil: put the currently viewed card on top (`to_top = true`) or
    /// on the bottom / into the graveyard.
    OrderCard {
        to_top: bool,
    },
    Discard {
        cards: Vec<ObjectId>,
    },
    /// Answer Crack the Earth's APNAP sacrifice queue. `None` is legal only for
    /// a seat with no controlled permanents (the choice is forced but explicit).
    ChooseSacrifice {
        permanent: Option<ObjectId>,
    },
    /// CR 903.9a state-based action: return the commander from graveyard/exile
    /// to the command zone, or leave it where it is.
    AnswerCommanderReturn {
        to_command_zone: bool,
    },
    /// CR 903.9b replacement choice: put the commander into the command zone
    /// instead of its hand/library destination.
    AnswerCommanderReplacement {
        to_command_zone: bool,
    },
    /// CR 903.9b replacement on a draw.
    AnswerCommanderDraw {
        to_command_zone: bool,
    },
}

impl Action {
    pub fn kind(&self) -> &'static str {
        match self {
            Action::Pass => "pass",
            Action::PlayLand { .. } => "play land",
            Action::CastSpell { .. } => "cast spell",
            Action::ActivateManaAbility { .. } => "mana ability",
            Action::DeclareAttackers { .. } => "declare attackers",
            Action::DeclareBlockers { .. } => "declare blockers",
            Action::Mulligan { .. } => "mulligan choice",
            Action::BottomCards { .. } => "bottom cards",
            Action::OrderCard { .. } => "order card",
            Action::Discard { .. } => "discard",
            Action::ChooseSacrifice { .. } => "choose sacrifice",
            Action::AnswerCommanderReturn { .. } => "commander return choice",
            Action::AnswerCommanderReplacement { .. } => "commander replacement choice",
            Action::AnswerCommanderDraw { .. } => "commander draw replacement choice",
        }
    }
}

/// A player action with identity: `revision` is the state revision the actor
/// observed, `decision` is the decision-point serial. Stale submissions are
/// rejected without touching the state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Submission {
    pub player: PlayerId,
    pub revision: u64,
    pub decision: u64,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegalAction {
    pub action: Action,
    /// Human-readable label. Also the string used as the opaque option label
    /// for the local decision model.
    pub label: String,
    /// Rule citations backing this action's availability.
    pub rules: Vec<&'static str>,
}
