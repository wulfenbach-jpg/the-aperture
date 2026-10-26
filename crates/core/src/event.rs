use serde::{Deserialize, Serialize};

use crate::card::{CardId, ManaColor};
use crate::ids::{CommanderId, ObjectId, PlayerId};
use crate::state::{OrderMode, Step};

/// Rules-visible event trace. Events are the coaching source of truth and the
/// UI log; rendering is viewer-aware so hidden information stays hidden.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    GameStart {
        seed: u64,
        first: PlayerId,
    },
    Mulligan {
        player: PlayerId,
        count: u8,
    },
    KeepHand {
        player: PlayerId,
        size: usize,
    },
    Bottom {
        player: PlayerId,
        count: usize,
    },
    TurnBegin {
        turn: u32,
        active: PlayerId,
    },
    StepBegin {
        step: Step,
    },
    Priority {
        player: PlayerId,
    },
    Pass {
        player: PlayerId,
    },
    LandPlayed {
        player: PlayerId,
        object: ObjectId,
        card: CardId,
    },
    ManaTapped {
        player: PlayerId,
        land: ObjectId,
        color: ManaColor,
    },
    ManaPoolEmptied {
        player: PlayerId,
        amount: u32,
    },
    Cast {
        player: PlayerId,
        object: ObjectId,
        card: CardId,
        targets: Vec<String>,
    },
    Resolve {
        controller: PlayerId,
        object: ObjectId,
        card: CardId,
    },
    PermanentEntered {
        object: ObjectId,
        card: CardId,
        controller: PlayerId,
    },
    NoLegalTargets {
        card: CardId,
        reason: String,
    },
    Countered {
        object: ObjectId,
        card: CardId,
        by: CardId,
    },
    Damage {
        source: Option<ObjectId>,
        target: String,
        amount: u32,
    },
    LifeChange {
        player: PlayerId,
        delta: i32,
        life: i32,
    },
    Draw {
        player: PlayerId,
        object: ObjectId,
        card: CardId,
    },
    DrewFromEmpty {
        player: PlayerId,
    },
    /// CR 701.13a: a card was put from the top of a library into its owner's
    /// graveyard. One event per card, so a replay can show exactly what was
    /// milled and in what order.
    Milled {
        player: PlayerId,
        object: ObjectId,
        card: CardId,
    },
    /// CR 106.4: mana was added to a player's mana pool without tapping a
    /// permanent (Pyretic Ritual, Seething Song).
    ManaAdded {
        player: PlayerId,
        color: ManaColor,
        amount: u32,
        pool_total: u32,
    },
    Scried {
        player: PlayerId,
        count: u8,
    },
    Surveilled {
        player: PlayerId,
        count: u8,
    },
    /// A private scry/surveil decision. The card is only public when a
    /// surveilled card goes to a graveyard.
    OrderedCard {
        player: PlayerId,
        card: CardId,
        to_top: bool,
        mode: OrderMode,
    },
    Discarded {
        player: PlayerId,
        card: CardId,
    },
    /// Public APNAP choice; the permanent is not sacrificed until every live
    /// seat has answered.
    SacrificeChosen {
        player: PlayerId,
        permanent: Option<ObjectId>,
    },
    AttackersDeclared {
        player: PlayerId,
        attackers: Vec<ObjectId>,
    },
    BlockersDeclared {
        player: PlayerId,
        blocks: Vec<(ObjectId, ObjectId)>,
    },
    CombatDamage {
        attacker: ObjectId,
        attacker_card: CardId,
        blocker: Option<(ObjectId, CardId)>,
        amount: u32,
    },
    /// Typed CR 903.10a credit: combat damage actually dealt to a player by one
    /// stable commander identity, plus the resulting cumulative total.
    CommanderDamage {
        commander: CommanderId,
        card: CardId,
        recipient: PlayerId,
        amount: u32,
        total: u32,
    },
    /// Full Fog-style combat damage prevention became active for this turn.
    PreventAllCombatDamage {
        turn: u32,
        card: CardId,
    },
    /// One combat-damage instance was prevented before any life, marked damage
    /// or commander-ledger credit (CR 615.6).
    CombatDamagePrevented {
        source: ObjectId,
        card: CardId,
        recipient: Option<PlayerId>,
        permanent: Option<ObjectId>,
        amount: u32,
    },
    Destroyed {
        object: ObjectId,
        card: CardId,
    },
    /// CR 602.2b: a player activated an ability. `text` is the printed ability
    /// text from the rules layer, never model output.
    AbilityActivated {
        player: PlayerId,
        source: ObjectId,
        card: CardId,
        text: String,
        targets: Vec<String>,
    },
    /// CR 603.3: a triggered ability was put on the stack (after any CR 603.3d
    /// target choice).
    TriggerStaged {
        controller: PlayerId,
        source: ObjectId,
        card: CardId,
        text: String,
    },
    /// CR 603.3d: a triggered ability that requires targets had none and was
    /// removed from the stack without ever being put on it.
    TriggerRemoved {
        controller: PlayerId,
        card: CardId,
        reason: String,
    },
    /// CR 603.5: the controller declined an optional triggered ability.
    TriggerDeclined {
        controller: PlayerId,
        card: CardId,
    },
    /// One ability finished resolving (an ability is not a card: nothing moves
    /// to a graveyard merely because the ability resolved).
    AbilityResolved {
        controller: PlayerId,
        source: ObjectId,
        card: CardId,
        text: String,
    },
    /// CR 608.2b: every target of the ability is now illegal, so it does not
    /// resolve.
    AbilityFizzled {
        controller: PlayerId,
        card: CardId,
        reason: String,
    },
    /// CR 615.1: a prevention effect created a one-shot shield.
    DamageShieldCreated {
        player: PlayerId,
        card: CardId,
        target: String,
        /// `None` for a shield that prevents all applicable damage this turn.
        amount: Option<u32>,
        combat_only: bool,
    },
    /// CR 615: one damage instance (or part of it) was prevented. `card` is the
    /// prevention effect's card, `source` the damage source when it still has an
    /// object representation.
    DamagePrevented {
        source: Option<ObjectId>,
        card: CardId,
        target: String,
        amount: u32,
        /// Remaining shield capacity; `None` for an unlimited shield.
        remaining: Option<u32>,
        combat: bool,
    },
    /// CR 614.1a with CR 614.5: one replacement effect applied once to one
    /// event. `chooser` is the player CR 616.1 gives the choice to.
    ReplacementApplied {
        card: CardId,
        controller: PlayerId,
        chooser: PlayerId,
        target: String,
        before: u32,
        after: u32,
    },
    /// CR 613: a continuous effect began applying in its layer. `layer` is the
    /// layer label (`"7a"`, `"7c"`) and `text` the rule it comes from.
    ContinuousEffectStarted {
        object: ObjectId,
        card: CardId,
        layer: String,
        text: String,
    },
    StateBasedAction {
        description: String,
    },
    /// A manual table override applied through the sandbox controls. This is
    /// never a legal-match action: the session that uses one is tainted. The
    /// event is rendered viewer-aware, so `private_cards` (the cards a scry or
    /// surveil looked at) are shown only to the seat that looked.
    SandboxOverride {
        player: PlayerId,
        kind: crate::sandbox::SandboxKind,
        count: u32,
        /// Public description; never contains a hidden card name.
        detail: String,
        #[serde(default)]
        private_cards: Vec<CardId>,
    },
    GameOver {
        winner: PlayerId,
        reason: String,
    },
    /// Engine projection of CR 104.4a, not a Comprehensive Rules term.
    GameDrawn {
        reason: String,
    },
    Truncated {
        reason: String,
    },
}
