use serde::{Deserialize, Serialize};

use crate::ids::{ObjectId, PlayerId};

/// A chosen target. The slice supports one target per spell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Target {
    Player(PlayerId),
    /// A permanent on the battlefield.
    Permanent(ObjectId),
    /// CR 110.1 + 400.1: a **card** in a zone that is not the battlefield. Today
    /// only a card in a graveyard is ever offered ("target creature card with no
    /// abilities **from your graveyard**", Ruxa, Patient Professor). It is a
    /// distinct variant rather than a reuse of `Permanent` so a
    /// `Target::Permanent` can never name an object that is not on the
    /// battlefield, and so a permanent-only effect can never consume a
    /// graveyard card by accident.
    Card(ObjectId),
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
    /// Cast a modal spell with exactly two distinct modes chosen at cast time.
    CastSpellWithModes {
        card: ObjectId,
        modes: [u8; 2],
        targets: Vec<Target>,
    },
    /// CR 107.3a + 601.2b: cast a spell whose printed cost contains `{X}` and
    /// announce the value of X. The value is paid as additional generic mana and
    /// travels with the spell (manifest value, entry counters, entry triggers),
    /// so a later resolution never has to guess it. A card that does not print
    /// `{X}` is refused, and a card that does print it cannot be cast through
    /// [`Action::CastSpell`] — X is announced explicitly or not at all.
    CastSpellWithX {
        card: ObjectId,
        x: u8,
        targets: Vec<Target>,
    },
    ActivateManaAbility {
        land: ObjectId,
    },
    /// Dynamic mana ability with an explicit chosen color.
    ActivateColorManaAbility {
        source: ObjectId,
        color: crate::card::ManaColor,
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
    /// CR 602.2: activate an ability of a permanent this player controls.
    /// `ability` indexes [`crate::ability::abilities`] for the source's card and
    /// `mode` indexes that ability's printed modes, so a caller can never
    /// supply a cost, target shape or effect of its own.
    ActivateAbility {
        source: ObjectId,
        ability: u8,
        mode: u8,
        targets: Vec<Target>,
    },
    /// CR 118.3 + 602.2b: activate an ability whose printed cost includes
    /// cards ("Discard a land card", "Exile two cards from your graveyard").
    /// The activator names those cards; the engine validates each one against
    /// the cost the card actually prints and the zone that cost reads, so a
    /// caller can never supply a cost the card does not have. Abilities with
    /// no card cost are still activated through [`Action::ActivateAbility`],
    /// which keeps every pre-existing replay and test literal valid.
    /// CR 702.171: pay a Mount's saddle cost by tapping named other creatures.
    /// Choose one basic land for the active library search, or finish.
    SearchLibraryPick {
        card: ObjectId,
    },
    SearchLibraryDone,
    SaddleMount {
        source: ObjectId,
        creatures: Vec<ObjectId>,
    },
    ActivateCostedAbility {
        source: ObjectId,
        ability: u8,
        mode: u8,
        targets: Vec<Target>,
        /// Cards named to pay the cost, in the order the printed cost reads
        /// them (the land to discard first, then the graveyard cards).
        cost_cards: Vec<ObjectId>,
    },
    /// CR 603.3d: choose the targets for a triggered ability being put on the
    /// stack.
    ChooseTriggerTarget {
        target: Target,
    },
    /// CR 115.6: put a triggered ability whose printed target is "up to one"
    /// on the stack with no target at all. Legal only while such a trigger is
    /// waiting for its target choice; a trigger with a mandatory target never
    /// offers it.
    DeclineTriggerTarget,
    /// CR 601.2c: finish choosing the *set* of targets for a triggered ability
    /// printed as "any number of target ..." (Rampaging Yao Guai). Legal only
    /// while that trigger is waiting for its set; the chosen set may be empty,
    /// which is the "destroy nothing" answer.
    FinishTriggerTargets,
    /// CR 603.5: accept or decline an optional ("you may") triggered ability as
    /// it resolves.
    AnswerTriggerChoice {
        use_ability: bool,
    },
    /// CR 510.1 + 608.2c: answer Ruxa, Patient Professor's "you may have that
    /// creature assign its combat damage as though it weren't blocked" for the
    /// creature the pending choice names. `true` assigns the creature's whole
    /// combat damage as though it had no blockers (so it goes to the defending
    /// player and none to the blockers); `false` assigns normally.
    AnswerCombatDamageAssignment {
        as_though_unblocked: bool,
    },
    /// CR 701.9a + 101.4: answer Kamahl's Summons' "each player may reveal any
    /// number of creature cards from their hand". The named cards must all be
    /// in the choosing seat's hand and be creature cards; they stay in hand.
    /// An empty list is the legal "reveal nothing" answer.
    RevealCreatureCards {
        cards: Vec<ObjectId>,
    },
}

impl Action {
    pub fn kind(&self) -> &'static str {
        match self {
            Action::Pass => "pass",
            Action::PlayLand { .. } => "play land",
            Action::CastSpell { .. } => "cast spell",
            Action::CastSpellWithModes { .. } => "cast modal spell",
            Action::CastSpellWithX { .. } => "cast spell",
            Action::ActivateManaAbility { .. } => "mana ability",
            Action::ActivateColorManaAbility { .. } => "mana ability",
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
            Action::ActivateAbility { .. } => "activate ability",
            Action::ActivateCostedAbility { .. } => "activate ability",
            Action::SaddleMount { .. } => "saddle mount",
            Action::SearchLibraryPick { .. } => "choose a basic land",
            Action::SearchLibraryDone => "finish searching",
            Action::ChooseTriggerTarget { .. } => "choose trigger target",
            Action::DeclineTriggerTarget => "decline trigger target",
            Action::FinishTriggerTargets => "finish trigger targets",
            Action::AnswerTriggerChoice { .. } => "trigger choice",
            Action::AnswerCombatDamageAssignment { .. } => "combat damage assignment choice",
            Action::RevealCreatureCards { .. } => "reveal creature cards",
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
