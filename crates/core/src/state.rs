use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::card::CardId;
use crate::ids::{ObjectId, PlayerId, Zone};
use crate::mana::ManaPool;
use crate::rng::Pcg32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Step {
    Untap,
    Upkeep,
    Draw,
    PrecombatMain,
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndCombat,
    PostcombatMain,
    EndStep,
    Cleanup,
}

pub const STEP_ORDER: [Step; 12] = [
    Step::Untap,
    Step::Upkeep,
    Step::Draw,
    Step::PrecombatMain,
    Step::BeginCombat,
    Step::DeclareAttackers,
    Step::DeclareBlockers,
    Step::CombatDamage,
    Step::EndCombat,
    Step::PostcombatMain,
    Step::EndStep,
    Step::Cleanup,
];

impl Step {
    pub fn name(self) -> &'static str {
        match self {
            Step::Untap => "untap step",
            Step::Upkeep => "upkeep step",
            Step::Draw => "draw step",
            Step::PrecombatMain => "precombat main phase",
            Step::BeginCombat => "beginning of combat step",
            Step::DeclareAttackers => "declare attackers step",
            Step::DeclareBlockers => "declare blockers step",
            Step::CombatDamage => "combat damage step",
            Step::EndCombat => "end of combat step",
            Step::PostcombatMain => "postcombat main phase",
            Step::EndStep => "end step",
            Step::Cleanup => "cleanup step",
        }
    }

    pub fn index(self) -> usize {
        STEP_ORDER.iter().position(|s| *s == self).unwrap_or(0)
    }

    /// Next step, plus whether the turn wraps.
    pub fn next(self) -> (Step, bool) {
        let i = self.index();
        if i + 1 >= STEP_ORDER.len() {
            (Step::Untap, true)
        } else {
            (STEP_ORDER[i + 1], false)
        }
    }

    pub fn is_main(self) -> bool {
        matches!(self, Step::PrecombatMain | Step::PostcombatMain)
    }

    /// Steps where a player receives priority (CR 502.4: no untap-step
    /// priority; CR 514.3: normally no cleanup-step priority).
    pub fn grants_priority(self) -> bool {
        !matches!(self, Step::Untap | Step::Cleanup)
    }

    pub fn is_combat(self) -> bool {
        matches!(
            self,
            Step::BeginCombat
                | Step::DeclareAttackers
                | Step::DeclareBlockers
                | Step::CombatDamage
                | Step::EndCombat
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderMode {
    Scry,
    Surveil,
}

/// A decision the engine is waiting on. While a `Pending` exists, no player has
/// priority (CR 608.2d: resolution choices are not priority).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pending {
    Mulligan {
        player: PlayerId,
    },
    BottomCards {
        player: PlayerId,
        count: u8,
    },
    DeclareAttackers {
        player: PlayerId,
    },
    DeclareBlockers {
        player: PlayerId,
    },
    /// Owner is looking at the top of their own library (`viewed`) and decides
    /// one card at a time. Decided cards are collected in `to_top`.
    OrderCard {
        player: PlayerId,
        viewed: Vec<ObjectId>,
        index: usize,
        to_top: Vec<ObjectId>,
        mode: OrderMode,
    },
    Discard {
        player: PlayerId,
        count: u8,
    },
}

impl Pending {
    pub fn player(&self) -> PlayerId {
        match self {
            Pending::Mulligan { player }
            | Pending::BottomCards { player, .. }
            | Pending::DeclareAttackers { player }
            | Pending::DeclareBlockers { player }
            | Pending::OrderCard { player, .. }
            | Pending::Discard { player, .. } => *player,
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Pending::Mulligan { player } => {
                format!("P{player}: keep or mulligan the opening hand (CR 103.5)")
            }
            Pending::BottomCards { player, count } => format!(
                "P{player}: put {count} card(s) on the bottom of the library (London mulligan, CR 103.5)"
            ),
            Pending::DeclareAttackers { player } => {
                format!("P{player}: declare attackers (CR 508.1a)")
            }
            Pending::DeclareBlockers { player } => {
                format!("P{player}: declare blockers (CR 509.1a)")
            }
            Pending::OrderCard {
                player,
                index,
                viewed,
                mode,
                ..
            } => {
                let verb = match mode {
                    OrderMode::Scry => "on top or on the bottom",
                    OrderMode::Surveil => "on top or into the graveyard",
                };
                format!(
                    "P{player}: {} card {} of {} — {verb} (CR {})",
                    match mode {
                        OrderMode::Scry => "scry",
                        OrderMode::Surveil => "surveil",
                    },
                    index + 1,
                    viewed.len(),
                    match mode {
                        OrderMode::Scry => "701.22a",
                        OrderMode::Surveil => "701.25a",
                    }
                )
            }
            Pending::Discard { player, count } => {
                format!("P{player}: discard {count} card(s) down to maximum hand size (CR 514.1)")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Object {
    pub id: ObjectId,
    pub card: CardId,
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub zone: Zone,
    pub tapped: bool,
    /// Damage marked this turn (CR 120.3); cleared in cleanup (CR 514.2).
    pub damage: u32,
    /// Summoning sickness (CR 302.6).
    pub summoning_sick: bool,
    /// "Until end of turn" pump from `Effect::Pump` (CR 613.4c).
    pub pump_power: i32,
    pub pump_toughness: i32,
    /// +1/+1 counters are stored for display and sandbox editing only. They do
    /// not affect combat math yet; see HANDOFF.md "not implemented".
    pub plus_one_counters: u32,
}

impl Object {
    pub fn is_creature(&self) -> bool {
        self.card.def().is_creature()
    }

    pub fn power(&self) -> i32 {
        self.card.def().power.unwrap_or(0) + self.pump_power
    }

    pub fn toughness(&self) -> i32 {
        self.card.def().toughness.unwrap_or(0) + self.pump_toughness
    }

    pub fn name(&self) -> &'static str {
        self.card.name()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StackItem {
    pub object: ObjectId,
    pub controller: PlayerId,
    pub targets: Vec<crate::action::Target>,
    /// Index of the next effect to apply during resolution (CR 608.2c).
    pub cursor: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub deck_name: String,
    pub library: Vec<ObjectId>,
    pub hand: Vec<ObjectId>,
    pub graveyard: Vec<ObjectId>,
    pub exile: Vec<ObjectId>,
    pub life: i32,
    pub mana: ManaPool,
    pub mulligans: u8,
    /// Set when a draw was attempted with an empty library (CR 104.3c/704.5b).
    pub drew_from_empty: bool,
    pub lost: bool,
}

impl Player {
    pub fn hand_size(&self) -> usize {
        self.hand.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Turn {
    pub number: u32,
    pub active: PlayerId,
    pub step: Step,
    /// `None` means no player has priority right now (mid-resolution, mid
    /// pending choice, or between steps).
    pub priority: Option<PlayerId>,
    /// Consecutive passes in the current priority round (CR 117.4).
    pub passes: u8,
    /// Lands played by the active player this turn (CR 305.2).
    pub lands_played: u8,
    pub attackers: Vec<ObjectId>,
    /// Attackers that became blocked when blockers were declared (CR 509.1h):
    /// they stay blocked even if every blocker later leaves the battlefield.
    pub blocked: Vec<ObjectId>,
    /// blocker -> attacker pairs.
    pub blockers: Vec<(ObjectId, ObjectId)>,
    pub combat_damage_done: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Win { winner: PlayerId, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub seed: u64,
    /// Who plays first; that player skips their first draw step (CR 103.8a).
    pub first_player: PlayerId,
    pub rng: Pcg32,
    pub objects: BTreeMap<ObjectId, Object>,
    pub players: [Player; 2],
    pub stack: Vec<StackItem>,
    pub turn: Turn,
    pub pending: Option<Pending>,
    /// Object id of the stack item currently resolving (CR 608.2), if any.
    pub resolving: Option<ObjectId>,
    pub next_object: u32,
    /// Incremented on every accepted state mutation.
    pub revision: u64,
    /// Serial of the current decision point; submissions must match it.
    pub decision: u64,
    pub outcome: Option<Outcome>,
    pub rules_revision: String,
}

impl GameState {
    pub fn player(&self, p: PlayerId) -> &Player {
        &self.players[p as usize]
    }

    pub fn object(&self, id: ObjectId) -> &Object {
        self.objects
            .get(&id)
            .expect("object id present in state objects")
    }

    pub fn try_object(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(&id)
    }

    pub fn battlefield(&self, p: PlayerId) -> Vec<&Object> {
        self.objects
            .values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller == p)
            .collect()
    }

    pub fn battlefield_creatures(&self, p: PlayerId) -> Vec<&Object> {
        self.battlefield(p)
            .into_iter()
            .filter(|o| o.is_creature())
            .collect()
    }

    pub fn find_permanent(&self, p: PlayerId, card: CardId) -> Option<&Object> {
        self.objects
            .values()
            .find(|o| o.zone == Zone::Battlefield && o.controller == p && o.card == card)
    }

    pub fn hand_cards(&self, p: PlayerId) -> Vec<&Object> {
        self.player(p)
            .hand
            .iter()
            .filter_map(|id| self.try_object(*id))
            .collect()
    }

    pub fn is_over(&self) -> bool {
        self.outcome.is_some()
    }

    /// The player who must act now, if any.
    pub fn decision_owner(&self) -> Option<PlayerId> {
        if self.outcome.is_some() {
            return None;
        }
        if let Some(pending) = &self.pending {
            Some(pending.player())
        } else {
            self.turn.priority
        }
    }
}
