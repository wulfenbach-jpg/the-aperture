use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::card::CardId;
use crate::error::EngineError;
use crate::ids::{MAX_PLAYERS, MIN_PLAYERS, ObjectId, PlayerId, Zone};
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
    /// Seat bitmask of players who have passed consecutively in the current
    /// priority round (CR 117.4). Bit `i` is seat `i`; at most four seats.
    pub passed: u8,
    /// Lands played by the active player this turn (CR 305.2).
    pub lands_played: u8,
    pub attackers: Vec<ObjectId>,
    /// Attackers that became blocked when blockers were declared (CR 509.1h):
    /// they stay blocked even if every blocker later leaves the battlefield.
    pub blocked: Vec<ObjectId>,
    /// blocker -> attacker pairs.
    pub blockers: Vec<(ObjectId, ObjectId)>,
    pub combat_damage_done: bool,
    /// The defending seat chosen in this combat's attacker declaration. In the
    /// bounded M1 model every attacker in one declaration attacks this seat.
    pub defender: Option<PlayerId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Win {
        winner: PlayerId,
        reason: String,
    },
    /// Engine projection of CR 104.4a (all remaining players lose
    /// simultaneously). Not a Comprehensive Rules term. Unexercised by the
    /// current card pool.
    Draw {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    pub seed: u64,
    /// Who plays first. In a two-player game that player skips their first draw
    /// step (CR 103.8a); in 3/4-seat free-for-all they draw (CR 103.8c, 800.7).
    pub first_player: PlayerId,
    pub rng: Pcg32,
    pub objects: BTreeMap<ObjectId, Object>,
    /// Ordered seats. `players.len()` is the only seat-count source; seats are
    /// never renumbered when a player leaves the game.
    pub players: Vec<Player>,
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
        self.players
            .get(p as usize)
            .unwrap_or_else(|| panic!("seat P{p} is not in this game"))
    }

    pub fn try_player(&self, p: PlayerId) -> Option<&Player> {
        self.players.get(p as usize)
    }

    /// Number of ordered seats. `players.len()` is the single source of truth.
    pub fn seat_count(&self) -> usize {
        self.players.len()
    }

    pub fn is_live(&self, p: PlayerId) -> bool {
        self.try_player(p).is_some_and(|player| !player.lost)
    }

    pub fn live_count(&self) -> usize {
        self.players.iter().filter(|p| !p.lost).count()
    }

    pub fn live_seats(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.players
            .iter()
            .enumerate()
            .filter(|(_, p)| !p.lost)
            .map(|(index, _)| index as PlayerId)
    }

    /// Bitmask of live seats, for the pass-round comparison (max four seats).
    pub fn live_mask(&self) -> u8 {
        self.live_seats().fold(0u8, |mask, p| mask | (1u8 << p))
    }

    /// Next live seat in turn order after `after`, wrapping and skipping
    /// eliminated seats (CR 800.4k). Departed seats are never renumbered.
    pub fn next_live_seat_after(&self, after: PlayerId) -> Option<PlayerId> {
        let n = self.seat_count();
        if n == 0 {
            return None;
        }
        for offset in 1..=n {
            let index = (after as usize + offset) % n;
            if self.players[index].lost {
                continue;
            }
            return Some(index as PlayerId);
        }
        None
    }

    /// Reject invalid seat/state configurations with a typed error instead of a
    /// panic or a silent repair.
    pub fn validate(&self) -> Result<(), EngineError> {
        let n = self.seat_count();
        if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&n) {
            return Err(EngineError::InvalidSeats {
                reason: format!("{n} seats supplied; supported range is 2..=4"),
            });
        }
        for (index, player) in self.players.iter().enumerate() {
            if player.id != index as PlayerId {
                return Err(EngineError::InvalidSeats {
                    reason: format!(
                        "seat {index} carries id P{}; seats must equal their index",
                        player.id
                    ),
                });
            }
        }
        let in_range = |p: PlayerId| (p as usize) < n;
        if !in_range(self.first_player) {
            return Err(EngineError::InvalidState {
                reason: format!("first_player P{} is out of range", self.first_player),
            });
        }
        if !in_range(self.turn.active) {
            return Err(EngineError::InvalidState {
                reason: format!("active seat P{} is out of range", self.turn.active),
            });
        }
        if let Some(p) = self.turn.priority
            && (!in_range(p) || !self.is_live(p))
        {
            return Err(EngineError::InvalidState {
                reason: format!("priority points at eliminated/out-of-range seat P{p}"),
            });
        }
        if let Some(p) = self.turn.defender
            && !in_range(p)
        {
            return Err(EngineError::InvalidState {
                reason: format!("defender P{p} is out of range"),
            });
        }
        if self.turn.passed >> n != 0 {
            return Err(EngineError::InvalidState {
                reason: format!(
                    "pass bitmask {} contains bits outside the {n} seats",
                    self.turn.passed
                ),
            });
        }
        if self.turn.passed & !self.live_mask() != 0 {
            return Err(EngineError::InvalidState {
                reason: format!(
                    "pass bitmask {} contains an eliminated seat",
                    self.turn.passed
                ),
            });
        }
        if let Some(pending) = &self.pending {
            let owner = pending.player();
            if !in_range(owner) || !self.is_live(owner) {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "pending choice belongs to eliminated/out-of-range seat P{owner}"
                    ),
                });
            }
        }
        for object in self.objects.values() {
            if !in_range(object.owner) || !in_range(object.controller) {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "object {} has out-of-range owner P{} or controller P{}",
                        object.id, object.owner, object.controller
                    ),
                });
            }
        }
        for item in &self.stack {
            if !in_range(item.controller) {
                return Err(EngineError::InvalidState {
                    reason: format!(
                        "stack object {} has out-of-range controller P{}",
                        item.object, item.controller
                    ),
                });
            }
        }
        if let Some(Outcome::Win { winner, .. }) = &self.outcome
            && (!in_range(*winner) || !self.is_live(*winner))
        {
            return Err(EngineError::InvalidState {
                reason: format!("recorded winner P{winner} is not a live seat"),
            });
        }
        Ok(())
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

    /// The player who must act now, if any. An eliminated seat never owns a
    /// decision, even while its turn continues (CR 800.4j).
    pub fn decision_owner(&self) -> Option<PlayerId> {
        if self.outcome.is_some() {
            return None;
        }
        if let Some(pending) = &self.pending {
            let owner = pending.player();
            return self.is_live(owner).then_some(owner);
        }
        self.turn.priority.filter(|p| self.is_live(*p))
    }
}
