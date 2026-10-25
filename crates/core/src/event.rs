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
        amount: u8,
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
    StateBasedAction {
        description: String,
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
