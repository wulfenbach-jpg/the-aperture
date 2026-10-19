use serde::{Deserialize, Serialize};

use crate::card::{CardId, ManaColor};
use crate::ids::{ObjectId, PlayerId};
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
    Destroyed {
        object: ObjectId,
        card: CardId,
    },
    StateBasedAction {
        description: String,
    },
    GameOver {
        winner: PlayerId,
        reason: String,
    },
    Truncated {
        reason: String,
    },
}
