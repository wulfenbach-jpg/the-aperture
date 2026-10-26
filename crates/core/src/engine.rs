//! The rules engine: priority/stack scheduler, transaction-validated actions,
//! state-based actions, bounded combat, turn-based actions.
//!
//! Scheduler rules implemented (CR = Comprehensive Rules 2026-09-25):
//! * consecutive passes resolve exactly one stack item (117.4); with an empty
//!   stack they end the step,
//! * the caster receives priority after casting (117.3c),
//! * the active player receives priority after a spell resolves,
//! * no priority during resolution or while a pending choice is outstanding
//!   (608.2d), and none in the untap step (502.4) or cleanup step (514.3),
//! * state-based actions run before any player receives priority (117.5),
//! * mana pools empty at every step/phase change (500.5).

use std::collections::BTreeMap;

use crate::ability::{self, Ability, TriggerEvent};
use crate::action::{Action, BlockDecl, LegalAction, Submission, Target};
use crate::card::{CardDef, CardId, Effect, ManaColor, SpellSpec, TargetSpec};
use crate::catalog;
use crate::deck::DeckList;
use crate::digest;
use crate::error::EngineError;
use crate::event::Event;
use crate::ids::{ObjectId, P0, PlayerId, Zone};
use crate::layer::{
    ContinuousEffect, ContinuousKind, DamageShield, DamageTarget, EffectDuration, Layer,
    ShieldScope, ShieldTarget,
};
use crate::mana::{self, ManaCost, ManaPool, Payment, Sources};
use crate::rng::Pcg32;
use crate::sandbox::{SandboxEdit, SandboxOp, SandboxOutcome};
use crate::state::{
    DrawResume, GameState, MulliganRound, Object, OrderMode, Outcome, Pending, Player, Profile,
    StackEntry, StackItem, Step, TriggerRef, Turn,
};

/// Safety bound on scheduler iterations inside one submission. Reaching it is a
/// truncation event, never a silently invented result.
const SETTLE_LIMIT: usize = 20_000;
const STARTING_HAND: usize = 7;
const MAX_HAND_SIZE: usize = 7;

/// Total mulligans a player may take: the first is free in multiplayer
/// (CR 103.5c), so a 3/4-seat player may take eight (bottoming at most seven).
fn mulligan_limit(seat_count: usize) -> u8 {
    if seat_count >= 3 { 8 } else { 7 }
}

/// Cards bottomed for the current London mulligan count. The first multiplayer
/// mulligan is free (CR 103.5c).
fn cards_to_bottom(seat_count: usize, mulligans: u8) -> u8 {
    if seat_count >= 3 {
        mulligans.saturating_sub(1)
    } else {
        mulligans
    }
}

#[derive(Debug, Clone)]
pub struct Game {
    pub state: GameState,
}

fn blank_object(id: ObjectId, card: CardId, owner: PlayerId) -> Object {
    Object {
        id,
        card,
        owner,
        controller: owner,
        zone: Zone::Library,
        commander: None,
        tapped: false,
        damage: 0,
        summoning_sick: false,
        pump_power: 0,
        pump_toughness: 0,
        plus_one_counters: 0,
        counters: crate::counters::Counters::default(),
    }
}

impl Game {
    /// Set up a 2–4 seat game: build/shuffle each seat's library, deal seven to
    /// every seat, roll the first player, then await mulligan decisions in seat
    /// order. An unsupported seat count is a typed error, never a panic.
    pub fn new(seed: u64, decks: &[DeckList]) -> Result<Game, EngineError> {
        Ok(Game::new_logged(seed, decks)?.0)
    }

    /// Build the explicit command-zone practice profile: an ordinary practice
    /// game plus exactly one separate commander card per seat, starting in that
    /// seat's command zone (CR 903.6). The commander is NOT part of the library
    /// snapshot; callers must supply one supported commander per seat.
    pub fn new_command_zone(
        seed: u64,
        decks: &[DeckList],
        commanders: &[(PlayerId, CardId)],
    ) -> Result<Game, EngineError> {
        Ok(Game::new_command_zone_logged(seed, decks, commanders)?.0)
    }

    pub fn new_command_zone_logged(
        seed: u64,
        decks: &[DeckList],
        commanders: &[(PlayerId, CardId)],
    ) -> Result<(Game, Vec<Event>), EngineError> {
        Game::new_profile_logged(seed, decks, Profile::CommandZone20, commanders)
    }

    /// Build the explicit BOUNDED Commander-combat practice profile: 40 starting
    /// life and one supported commander per seat in the command zone. This is
    /// not the Commander format (no 100-card construction, partners, legend rule
    /// or native table). The commander-damage ledger is added by its own slice.
    pub fn new_commander_combat(
        seed: u64,
        decks: &[DeckList],
        commanders: &[(PlayerId, CardId)],
    ) -> Result<Game, EngineError> {
        Ok(Game::new_commander_combat_logged(seed, decks, commanders)?.0)
    }

    pub fn new_commander_combat_logged(
        seed: u64,
        decks: &[DeckList],
        commanders: &[(PlayerId, CardId)],
    ) -> Result<(Game, Vec<Event>), EngineError> {
        Game::new_profile_logged(seed, decks, Profile::CommanderCombat40, commanders)
    }

    fn new_profile_logged(
        seed: u64,
        decks: &[DeckList],
        profile: Profile,
        commanders: &[(PlayerId, CardId)],
    ) -> Result<(Game, Vec<Event>), EngineError> {
        if profile.has_command_zone() && commanders.is_empty() {
            return Err(EngineError::InvalidCommanderSetup {
                reason: format!("{} needs exactly one commander per seat", profile.label()),
            });
        }
        let (base, events) = Game::new_base_logged(seed, decks, profile)?;
        let mut game = base;
        if !profile.has_command_zone() {
            if !commanders.is_empty() {
                return Err(EngineError::InvalidCommanderSetup {
                    reason: format!("{} does not carry a command zone", profile.label()),
                });
            }
            return Ok((game, events));
        }
        if commanders.len() != decks.len() {
            return Err(EngineError::InvalidCommanderSetup {
                reason: format!(
                    "{} commander spec(s) for {} seat(s); exactly one per seat is required",
                    commanders.len(),
                    decks.len()
                ),
            });
        }
        let mut seen: Vec<PlayerId> = Vec::new();
        for (seat, card) in commanders {
            if (*seat as usize) >= decks.len() {
                return Err(EngineError::InvalidCommanderSetup {
                    reason: format!("P{seat} is outside this {}-seat game", decks.len()),
                });
            }
            if seen.contains(seat) {
                return Err(EngineError::InvalidCommanderSetup {
                    reason: format!(
                        "P{seat} has more than one commander; multi-commander variants are unsupported"
                    ),
                });
            }
            seen.push(*seat);
            if !card.is_supported_commander() {
                return Err(EngineError::UnsupportedCommander { card: *card });
            }
        }
        for seat in 0..decks.len() as PlayerId {
            if !seen.contains(&seat) {
                return Err(EngineError::InvalidCommanderSetup {
                    reason: format!("P{seat} has no commander; one per seat is required"),
                });
            }
        }

        for (index, (seat, card)) in commanders.iter().enumerate() {
            let commander_id = crate::ids::CommanderId(index as u32 + 1);
            let object_id = ObjectId(game.state.next_object);
            game.state.next_object += 1;
            let mut object = blank_object(object_id, *card, *seat);
            object.zone = Zone::Command;
            object.commander = Some(commander_id);
            game.state.objects.insert(object_id, object);
            game.state.players[*seat as usize]
                .command_zone
                .push(object_id);
            game.state.commanders.push(crate::state::Commander {
                id: commander_id,
                owner: *seat,
                card: *card,
                object: object_id,
                casts_from_command_zone: 0,
                return_eligible: false,
            });
        }
        game.state.validate()?;
        Ok((game, events))
    }

    pub fn new_logged(seed: u64, decks: &[DeckList]) -> Result<(Game, Vec<Event>), EngineError> {
        Game::new_profile_logged(seed, decks, Profile::Practice20, &[])
    }

    fn new_base_logged(
        seed: u64,
        decks: &[DeckList],
        profile: Profile,
    ) -> Result<(Game, Vec<Event>), EngineError> {
        if !(crate::ids::MIN_PLAYERS..=crate::ids::MAX_PLAYERS).contains(&decks.len()) {
            return Err(EngineError::InvalidSeats {
                reason: format!(
                    "{} seats supplied; supported range is {}..={}",
                    decks.len(),
                    crate::ids::MIN_PLAYERS,
                    crate::ids::MAX_PLAYERS
                ),
            });
        }
        for deck in decks {
            for card in deck.expand() {
                if card.is_supported_commander() {
                    return Err(EngineError::UnsupportedCommander { card });
                }
            }
        }
        let players = decks
            .iter()
            .enumerate()
            .map(|(index, deck)| Player {
                id: index as PlayerId,
                name: format!("P{index}"),
                deck_name: deck.name.clone(),
                library: Vec::new(),
                hand: Vec::new(),
                graveyard: Vec::new(),
                exile: Vec::new(),
                command_zone: Vec::new(),
                life: profile.starting_life(),
                mana: ManaPool::default(),
                mulligans: 0,
                drew_from_empty: false,
                lost: false,
            })
            .collect();
        let mut state = GameState {
            seed,
            first_player: P0,
            rng: Pcg32::new(seed),
            objects: BTreeMap::new(),
            commanders: Vec::new(),
            profile,
            commander_damage: Vec::new(),
            retired_commanders: Vec::new(),
            prevent_all_combat_damage_turn: None,
            players,
            stack: Vec::new(),
            turn: Turn {
                number: 1,
                active: P0,
                step: Step::Untap,
                priority: None,
                passed: 0,
                lands_played: 0,
                attackers: Vec::new(),
                blocked: Vec::new(),
                blockers: Vec::new(),
                combat_damage_done: false,
                defender: None,
            },
            pending: None,
            mulligan: None,
            resolving: None,
            next_object: 1,
            trigger_queue: Vec::new(),
            next_trigger_seq: 0,
            revealed: Vec::new(),
            activation_uses: BTreeMap::new(),
            effects: Vec::new(),
            shields: Vec::new(),
            // Allocator cursors start at zero so a state with no live effect has
            // exactly the canonical JSON it had before the effect pipeline
            // existed; recorded save/replay digests therefore stay verifiable.
            next_effect: 0,
            next_timestamp: 0,
            revision: 0,
            decision: 0,
            outcome: None,
            rules_revision: crate::RULES_REVISION.to_string(),
        };

        for (index, deck) in decks.iter().enumerate() {
            let player = index as PlayerId;
            let mut ids = Vec::with_capacity(deck.size());
            for card in deck.expand() {
                let id = ObjectId(state.next_object);
                state.next_object += 1;
                state.objects.insert(id, blank_object(id, card, player));
                ids.push(id);
            }
            state.rng.shuffle(&mut ids);
            state.players[index].library = ids;
        }

        let mut game = Game { state };
        let mut events = Vec::new();
        for player in 0..decks.len() as PlayerId {
            for _ in 0..STARTING_HAND {
                game.draw(player, &mut events);
            }
        }
        let first = game.state.rng.below(decks.len() as u32) as PlayerId;
        game.state.turn.active = first;
        game.state.first_player = first;
        // Turn 1 exists before the first step change, so it is announced here.
        events.push(Event::GameStart { seed, first });
        events.push(Event::TurnBegin {
            turn: 1,
            active: first,
        });
        // CR 103.5: declarations start with the starting player, then turn
        // order. All seats are live at game start.
        let mut order = vec![first];
        while order.len() < decks.len() {
            let next = game
                .state
                .next_live_seat_after(*order.last().expect("order is not empty"))
                .expect("all seats are live at game start");
            order.push(next);
        }
        game.state.mulligan = Some(MulliganRound {
            round: 1,
            eligible: order.clone(),
            previous_mulliganers: Vec::new(),
            kept: Vec::new(),
            order,
            cursor: 0,
            declarations: BTreeMap::new(),
            bottom_queue: Vec::new(),
            bottomed: Vec::new(),
        });
        game.state.pending = Some(Pending::Mulligan { player: first });
        // Command-zone profiles add their registry after this base setup, so
        // validation is deferred to new_profile_logged for those profiles.
        if !profile.has_command_zone() {
            game.state.validate()?;
        }
        Ok((game, events))
    }

    /// Validate and wrap a state loaded from a save or built by a fixture.
    pub fn from_state(state: GameState) -> Result<Game, EngineError> {
        state.validate()?;
        Ok(Game { state })
    }

    pub fn digest(&self) -> String {
        digest::digest(&self.state)
    }

    /// Current decision-point serial; submissions must carry it.
    pub fn decision(&self) -> u64 {
        self.state.decision
    }

    pub fn revision(&self) -> u64 {
        self.state.revision
    }

    pub fn rules_revision(&self) -> &str {
        &self.state.rules_revision
    }

    pub fn decision_owner(&self) -> Option<PlayerId> {
        self.state.decision_owner()
    }

    // ---------------------------------------------------------------- access

    fn objects_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        self.state.objects.get_mut(&id)
    }

    // ------------------------------------------------------- effects (CR 613-616)

    /// Next rule-visible timestamp (CR 613.7). Monotonic within a game; never
    /// derived from wall-clock time, so a replay reproduces the same order.
    fn next_timestamp(&mut self) -> u64 {
        self.state.next_timestamp += 1;
        self.state.next_timestamp
    }

    fn next_effect_id(&mut self) -> u32 {
        let id = self.state.next_effect;
        self.state.next_effect += 1;
        id
    }

    /// Drop the effect/timestamp allocator cursors once nothing is live.
    ///
    /// The canonical digest is SHA-256 over the serialized state, so a cursor
    /// left behind by an effect that has already ended would permanently change
    /// the serialization of an otherwise identical position. Resetting while no
    /// effect exists keeps such a position byte-identical to the same position
    /// recorded before the effect pipeline existed, which is what lets earlier
    /// saves and replays still verify. A live effect always keeps its cursor.
    fn release_idle_effect_cursors(&mut self) {
        if self.state.effects.is_empty() && self.state.shields.is_empty() {
            self.state.next_effect = 0;
            self.state.next_timestamp = 0;
        }
    }

    /// How many cards a printed count-based static ability counts for the
    /// object controlled by `controller`.
    fn count_value(&self, controller: PlayerId, source: ability::CountSource) -> i32 {
        match source {
            ability::CountSource::CreatureCardsInOpponentsGraveyards => {
                self.state
                    .players
                    .iter()
                    .enumerate()
                    .filter(|(player, _)| *player as PlayerId != controller)
                    .flat_map(|(_, player)| player.graveyard.iter())
                    .filter_map(|id| self.state.try_object(*id))
                    .filter(|object| object.card.def().is_creature())
                    .count() as i32
            }
        }
    }

    /// CR 613: recompute every battlefield object's power/toughness in layer
    /// order. Only layer 7c exists in this starter set: printed count-based
    /// statics (Wight of Precinct Six's "+1/+1 for each creature card in your
    /// opponents' graveyards") and one-shot until-end-of-turn modifiers, the
    /// latter in timestamp order (CR 613.7). Silent by design: a layer
    /// recomputation is not itself a rules event, so the event trace only shows
    /// effects starting (`ContinuousEffectStarted`) and their consequences.
    ///
    /// 7c modifiers are additive, so the order between the static group and the
    /// timestamp-ordered group cannot change the resulting power/toughness
    /// (addition commutes); each group is still iterated in a fixed order —
    /// object id for statics, timestamp for one-shot effects — so every replay
    /// recomputes identically.
    ///
    /// Layer 7d (+1/+1 counters) is deliberately not applied here: counters are
    /// still display-only until the counters lane makes them
    /// characteristic-changing.
    fn recompute_layers(&mut self) {
        let battlefield: Vec<(ObjectId, CardId)> = self
            .state
            .objects
            .values()
            .filter(|object| object.zone == Zone::Battlefield)
            .map(|object| (object.id, object.card))
            .collect();
        for (id, _) in &battlefield {
            if let Some(object) = self.state.objects.get_mut(id) {
                object.pump_power = 0;
                object.pump_toughness = 0;
            }
        }
        // Printed layer 7c statics: +P/+T for each counted card, evaluated
        // against the current game state for the source's controller.
        for (id, card) in &battlefield {
            let controller = self
                .state
                .try_object(*id)
                .map(|object| object.controller)
                .unwrap_or(0);
            for static_ability in ability::static_abilities(*card) {
                let ability::StaticAbility::PowerToughnessPerCount {
                    power, toughness, ..
                } = static_ability
                else {
                    continue;
                };
                let Some(count) = static_ability.count_source() else {
                    continue;
                };
                let value = self.count_value(controller, count);
                if value == 0 {
                    continue;
                }
                if let Some(object) = self.state.objects.get_mut(id) {
                    object.pump_power += power * value;
                    object.pump_toughness += toughness * value;
                }
            }
        }
        // Layer 7c one-shot effects in timestamp order (CR 613.7).
        let mut modifying: Vec<(u64, ObjectId, i32, i32)> = self
            .state
            .effects
            .iter()
            .filter(|effect| effect.layer == Layer::Modifying)
            .map(|effect| {
                let ContinuousKind::ModifyPowerToughness { power, toughness } = effect.kind;
                (effect.timestamp, effect.target, power, toughness)
            })
            .collect();
        modifying.sort_unstable_by_key(|(timestamp, ..)| *timestamp);
        for (_, target, power, toughness) in modifying {
            if let Some(object) = self.state.objects.get_mut(&target)
                && object.zone == Zone::Battlefield
            {
                object.pump_power += power;
                object.pump_toughness += toughness;
            }
        }
        // An effect whose target left the battlefield (CR 400.7) stops applying;
        // so does one that only lasts while its source is on the battlefield.
        let effects = std::mem::take(&mut self.state.effects);
        self.state.effects = effects
            .into_iter()
            .filter(|effect| {
                let target_present = self
                    .state
                    .try_object(effect.target)
                    .is_some_and(|object| object.zone == Zone::Battlefield);
                let source_present = effect.duration != EffectDuration::WhileSourceOnBattlefield
                    || effect.source.is_some_and(|source| {
                        self.state
                            .try_object(source)
                            .is_some_and(|object| object.zone == Zone::Battlefield)
                    });
                target_present && source_present
            })
            .collect();
        // A shield on a permanent that left the battlefield can never apply again.
        let shields = std::mem::take(&mut self.state.shields);
        self.state.shields = shields
            .into_iter()
            .filter(|shield| match shield.target {
                ShieldTarget::Permanent(id) => self
                    .state
                    .try_object(id)
                    .is_some_and(|object| object.zone == Zone::Battlefield),
                ShieldTarget::Player(_) | ShieldTarget::Any => true,
            })
            .collect();
    }

    /// Start a layer 7c continuous effect on `target` from `item`'s source.
    fn add_modifier_effect(
        &mut self,
        item: &StackItem,
        target: ObjectId,
        power: i32,
        toughness: i32,
        events: &mut Vec<Event>,
    ) {
        if power == 0 && toughness == 0 {
            return;
        }
        let card = self.state.object(item.object).card;
        let timestamp = self.next_timestamp();
        let id = self.next_effect_id();
        self.state.effects.push(ContinuousEffect {
            id,
            timestamp,
            source: Some(item.object),
            card,
            controller: item.controller,
            layer: Layer::Modifying,
            target,
            kind: ContinuousKind::ModifyPowerToughness { power, toughness },
            duration: EffectDuration::UntilEndOfTurn,
        });
        events.push(Event::ContinuousEffectStarted {
            object: target,
            card,
            layer: Layer::Modifying.label().to_string(),
            text: format!(
                "{power:+}/{toughness:+} until end of turn ({})",
                Layer::Modifying.rules()
            ),
        });
    }

    /// Player CR 616.1 gives the choice of replacement/prevention order to: the
    /// affected player, or the controller of the affected permanent.
    fn damage_chooser(&self, target: DamageTarget) -> PlayerId {
        match target {
            DamageTarget::Player(player) => player,
            DamageTarget::Permanent(id) => self
                .state
                .try_object(id)
                .map(|object| object.controller)
                .unwrap_or(0),
        }
    }

    fn damage_target_label(&self, target: DamageTarget) -> String {
        match target {
            DamageTarget::Player(player) => format!("P{player}"),
            DamageTarget::Permanent(id) => self.describe_object(id),
        }
    }

    /// Replacement effects that apply to damage dealt by `source` (CR 614.1a).
    /// The printed wording is "a **source** you control would deal damage", so
    /// the source may be a permanent on the battlefield (a creature's combat or
    /// activated-ability damage) or a spell on the stack (Shock) — the test is
    /// that `source`'s controller controls it, not that it is a creature.
    fn damage_multipliers(
        &self,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Vec<(CardId, PlayerId, u32)> {
        let controller = match source_controller {
            Some(controller) => controller,
            None => {
                let Some(source_object) = source.and_then(|id| self.state.try_object(id)) else {
                    return Vec::new();
                };
                if !matches!(source_object.zone, Zone::Battlefield | Zone::Stack) {
                    return Vec::new();
                }
                source_object.controller
            }
        };
        let mut found: Vec<(CardId, PlayerId, u32)> = self
            .state
            .objects
            .values()
            .filter(|object| object.zone == Zone::Battlefield && object.controller == controller)
            .flat_map(|object| {
                ability::static_abilities(object.card)
                    .iter()
                    .filter_map(move |static_ability| match static_ability {
                        ability::StaticAbility::DamageFromControlledSources { factor, .. } => {
                            Some((object.card, object.controller, *factor))
                        }
                        _ => None,
                    })
            })
            .collect();
        // Two copies of the same multiplier commute, so object order is a stable
        // deterministic order rather than a timestamp one.
        found
            .sort_unstable_by_key(|(card, controller, factor)| (*controller, *card as u8, *factor));
        found
    }

    /// Deal one damage instance through the replacement/prevention pipeline
    /// (CR 614.1a, 615.1, 616.1) and return the amount actually dealt.
    ///
    /// CR 616.1 policy (engine-authoritative, because the engine has no interact
    /// channel for order choices): the chooser resolves prevention shields
    /// first — shields controlled by the chooser, then any other shield, each in
    /// timestamp order — and only then the damage multipliers. That is the order
    /// the affected player would choose (it removes damage before anything can
    /// double it), and every application is recorded with its chooser in the
    /// event trace, so no later agent can pick a different order.
    fn deal_damage(
        &mut self,
        source: Option<ObjectId>,
        target: DamageTarget,
        amount: u32,
        combat: bool,
        events: &mut Vec<Event>,
    ) -> u32 {
        self.deal_damage_from(source, None, target, amount, combat, events)
    }

    /// Damage with an explicit last-known source controller. Bounded use: an
    /// ability whose source has already left the battlefield still uses that
    /// source's controller for the existing replacement/multiplier pipeline.
    fn deal_damage_from(
        &mut self,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
        target: DamageTarget,
        amount: u32,
        combat: bool,
        events: &mut Vec<Event>,
    ) -> u32 {
        if amount == 0 {
            return 0;
        }
        let chooser = self.damage_chooser(target);
        let label = self.damage_target_label(target);
        let mut remaining = amount;

        let mut shields: Vec<(u8, u64, u32)> = self
            .state
            .shields
            .iter()
            .filter(|shield| shield.applies_to(target, combat))
            .map(|shield| {
                (
                    if shield.controller == chooser { 0 } else { 1 },
                    shield.timestamp,
                    shield.id,
                )
            })
            .collect();
        shields.sort_unstable();
        for (_, _, id) in shields {
            if remaining == 0 {
                break;
            }
            let Some(index) = self.state.shields.iter().position(|shield| shield.id == id) else {
                continue;
            };
            let prevented = self.state.shields[index].absorb(remaining);
            if prevented == 0 {
                continue;
            }
            remaining -= prevented;
            let card = self.state.shields[index].card;
            let now_remaining = self.state.shields[index].remaining;
            if self.state.shields[index].exhausted() {
                self.state.shields.remove(index);
                self.release_idle_effect_cursors();
            }
            events.push(Event::DamagePrevented {
                source,
                card,
                target: label.clone(),
                amount: prevented,
                remaining: now_remaining,
                combat,
            });
        }

        // CR 614.5: each replacement effect applies at most once per event.
        for (card, controller, factor) in self.damage_multipliers(source, source_controller) {
            if remaining == 0 || factor <= 1 {
                continue;
            }
            let before = remaining;
            remaining = remaining.saturating_mul(factor);
            events.push(Event::ReplacementApplied {
                card,
                controller,
                chooser,
                target: label.clone(),
                before,
                after: remaining,
            });
        }

        if remaining == 0 {
            return 0;
        }
        match target {
            DamageTarget::Player(player) => {
                self.state.players[player as usize].life -= remaining as i32;
            }
            DamageTarget::Permanent(id) => {
                if let Some(object) = self.objects_mut(id) {
                    object.damage += remaining;
                }
            }
        }
        remaining
    }

    fn tap_for(&mut self, lands: &[ObjectId], events: &mut Vec<Event>) {
        let controller = self.state.turn.priority.unwrap_or(self.state.turn.active);
        for land in lands {
            let color = self
                .state
                .try_object(*land)
                .and_then(|o| o.card.def().basic_mana);
            if let Some(color) = color {
                if let Some(obj) = self.objects_mut(*land) {
                    obj.tapped = true;
                }
                events.push(Event::ManaTapped {
                    player: controller,
                    land: *land,
                    color,
                });
            }
        }
    }

    /// Untapped basic lands controlled by `p`, in ascending object-id order.
    fn mana_lands(&self, p: PlayerId) -> Vec<(ObjectId, ManaColor)> {
        self.state
            .objects
            .values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                    && o.controller == p
                    && !o.tapped
                    && o.card.def().is_land()
            })
            .filter_map(|o| o.card.def().basic_mana.map(|c| (o.id, c)))
            .collect()
    }

    fn sources(&self, p: PlayerId) -> Sources {
        Sources {
            pool: self.state.player(p).mana,
            lands: self.mana_lands(p),
        }
    }

    fn plan_payment(&self, p: PlayerId, cost: &ManaCost) -> Result<Payment, EngineError> {
        let sources = self.sources(p);
        mana::plan(&sources, cost).ok_or_else(|| EngineError::CannotPay {
            cost: cost.render(),
            available: format!(
                "{}{}",
                sources.pool.render(),
                if sources.lands.is_empty() {
                    ""
                } else {
                    " + untapped lands"
                }
            ),
        })
    }

    /// Total mana cost of casting `card` from its current zone. A command-zone
    /// cast adds {2} per previous command-zone cast (CR 903.8); generic-mana
    /// overflow is a typed refusal, never a silent wrap.
    fn effective_cost(&self, card: ObjectId) -> Result<ManaCost, EngineError> {
        let object = self.state.object(card);
        let mut cost = object.card.mana_cost().unwrap_or_else(ManaCost::empty);
        if object.zone == Zone::Command
            && let Some(commander) = object.commander
        {
            let registry = self
                .state
                .commanders
                .iter()
                .find(|registry| registry.id == commander)
                .ok_or(EngineError::InvalidCommanderSetup {
                    reason: format!("object {} carries an unknown commander", card),
                })?;
            let tax = registry
                .casts_from_command_zone
                .checked_mul(2)
                .and_then(|tax| u8::try_from(tax).ok())
                .ok_or_else(|| {
                    EngineError::illegal(
                        "cast spell",
                        "commander tax exceeds the supported generic mana range",
                    )
                })?;
            cost.generic = cost.generic.checked_add(tax).ok_or_else(|| {
                EngineError::illegal(
                    "cast spell",
                    "commander tax makes the generic cost overflow",
                )
            })?;
        }
        Ok(cost)
    }

    fn apply_payment(&mut self, p: PlayerId, payment: Payment, events: &mut Vec<Event>) {
        self.tap_for(&payment.taps, events);
        self.state.players[p as usize].mana = payment.remaining;
    }

    fn empty_mana_pools(&mut self, events: &mut Vec<Event>) {
        for p in 0..self.state.seat_count() as PlayerId {
            if !self.state.is_live(p) {
                continue;
            }
            let amount = self.state.player(p).mana.total();
            if amount > 0 {
                self.state.players[p as usize].mana = ManaPool::default();
                events.push(Event::ManaPoolEmptied { player: p, amount });
            }
        }
    }

    /// Remove an object from its containers without changing its zone identity.
    /// Callers that move a commander then insert it back with `insert_object`,
    /// which allocates a fresh ObjectId (CR 400.7).
    fn detach_object(&mut self, id: ObjectId) -> Option<Object> {
        let object = self.state.objects.remove(&id)?;
        for player in &mut self.state.players {
            player.library.retain(|c| *c != id);
            player.hand.retain(|c| *c != id);
            player.graveyard.retain(|c| *c != id);
            player.exile.retain(|c| *c != id);
            player.command_zone.retain(|c| *c != id);
        }
        // A represented card changing zone leaves the stack in the same move.
        // A spell leaves the stack with its card. An ability does not: its
        // source may change zones (or leave the game) while the ability is
        // still on the stack (CR 603.3, 608.2).
        self.state
            .stack
            .retain(|item| item.object != id || item.entry != StackEntry::Spell);
        if self.state.resolving == Some(id) {
            self.state.resolving = None;
        }
        self.state.turn.attackers.retain(|c| *c != id);
        self.state.turn.blocked.retain(|c| *c != id);
        self.state
            .turn
            .blockers
            .retain(|(blocker, attacker)| *blocker != id && *attacker != id);
        Some(object)
    }

    /// Insert an object into `zone`, re-keying commander objects. The stable
    /// commander registry then points at the new object; stale target ids do not
    /// follow (CR 400.7, 903.3).
    fn insert_object(&mut self, mut object: Object, zone: Zone) -> ObjectId {
        let owner = object.owner;
        let commander = object.commander;
        let old_id = object.id;
        let new_id = if commander.is_some() {
            let id = ObjectId(self.state.next_object);
            self.state.next_object += 1;
            object.id = id;
            id
        } else {
            old_id
        };
        object.zone = zone;
        // CR 122.2: counters cease to exist with the object that carried them,
        // so a zone change can never deliver counters to the new object.
        object.counters.clear();
        if commander.is_some() {
            // A zone change creates a new object (CR 400.7): no previous
            // battlefield-only memory may follow the commander.
            object.plus_one_counters = 0;
            if matches!(
                zone,
                Zone::Library | Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Command
            ) {
                object.controller = object.owner;
                object.tapped = false;
                object.damage = 0;
                object.summoning_sick = false;
                object.pump_power = 0;
                object.pump_toughness = 0;
            }
        }
        if let Some(commander) = commander
            && let Some(registry) = self
                .state
                .commanders
                .iter_mut()
                .find(|registry| registry.id == commander)
        {
            registry.object = new_id;
            // CR 903.9a: arm the owner choice exactly for a move into
            // graveyard/exile; leaving those zones clears it.
            registry.return_eligible = matches!(zone, Zone::Graveyard | Zone::Exile);
        }
        match zone {
            Zone::Library => self.state.players[owner as usize].library.push(new_id),
            Zone::Hand => self.state.players[owner as usize].hand.push(new_id),
            Zone::Graveyard => self.state.players[owner as usize].graveyard.push(new_id),
            Zone::Exile => self.state.players[owner as usize].exile.push(new_id),
            Zone::Command => self.state.players[owner as usize].command_zone.push(new_id),
            Zone::Battlefield | Zone::Stack | Zone::Removed => {}
        }
        self.state.objects.insert(new_id, object);
        new_id
    }

    /// CR 903.9b: a commander moving to hand/library is replaced before the
    /// move. The owner chooses; the resolving spell is suspended at the exact
    /// source/cursor so the continuation is deterministic.
    fn begin_commander_replacement(
        &mut self,
        id: ObjectId,
        destination: Zone,
        item: &StackItem,
    ) -> bool {
        let Some(object) = self.state.try_object(id) else {
            return false;
        };
        let Some(commander) = object.commander else {
            return false;
        };
        let Some(registry) = self
            .state
            .commanders
            .iter()
            .find(|registry| registry.id == commander)
        else {
            return false;
        };
        self.state.pending = Some(Pending::CommanderReplacement {
            player: registry.owner,
            commander,
            object: id,
            from_zone: object.zone,
            destination,
            source: item.object,
            cursor: item.cursor + 1,
        });
        self.state.turn.priority = None;
        true
    }

    fn apply_commander_draw(
        &mut self,
        player: PlayerId,
        to_command_zone: bool,
        events: &mut Vec<Event>,
    ) {
        let Some(Pending::CommanderDraw {
            object,
            remaining,
            resume,
            ..
        }) = self.state.pending.take()
        else {
            return;
        };
        let target_zone = if to_command_zone {
            Zone::Command
        } else {
            Zone::Hand
        };
        if let Some(mut moved) = self.detach_object(object) {
            moved.tapped = false;
            moved.damage = 0;
            moved.summoning_sick = false;
            moved.pump_power = 0;
            moved.pump_toughness = 0;
            let card = moved.card;
            let new_id = self.insert_object(moved, target_zone);
            if to_command_zone {
                events.push(Event::StateBasedAction {
                    description: format!(
                        "P{player} replaces the draw and puts commander {new_id} into the command zone (CR 903.9b)"
                    ),
                });
            } else {
                events.push(Event::Draw {
                    player,
                    object: new_id,
                    card,
                });
            }
        }
        match resume {
            DrawResume::Step => {
                let active = self.state.turn.active;
                self.grant_priority(active, events);
            }
            DrawResume::Effect { .. } => {
                for index in 0..remaining {
                    if self.draw_with_resume(player, resume, remaining - index - 1, events) {
                        return;
                    }
                }
            }
        }
    }

    fn apply_commander_replacement(
        &mut self,
        player: PlayerId,
        to_command_zone: bool,
        events: &mut Vec<Event>,
    ) {
        let Some(Pending::CommanderReplacement {
            commander,
            object,
            destination,
            ..
        }) = self.state.pending.take()
        else {
            return;
        };
        let target_zone = if to_command_zone {
            Zone::Command
        } else {
            destination
        };
        let Some(mut moved) = self.detach_object(object) else {
            return;
        };
        moved.tapped = false;
        moved.damage = 0;
        moved.summoning_sick = false;
        moved.pump_power = 0;
        moved.pump_toughness = 0;
        let card = moved.card;
        if target_zone == Zone::Library {
            let owner = moved.owner;
            let new_id = self.insert_object(moved, Zone::Library);
            let library = &mut self.state.players[owner as usize].library;
            library.retain(|id| *id != new_id);
            library.insert(0, new_id);
            events.push(Event::StateBasedAction {
                description: format!(
                    "P{player} puts commander {new_id} on top of their library (CR 903.9b)"
                ),
            });
        } else {
            let new_id = self.insert_object(moved, target_zone);
            events.push(Event::StateBasedAction {
                description: format!(
                    "P{player} puts commander {new_id} into the {} (CR 903.9b)",
                    target_zone.name()
                ),
            });
        }
        let _ = (commander, card);
    }

    fn move_to_top_of_library(&mut self, id: ObjectId, events: &mut Vec<Event>) {
        let Some(mut object) = self.detach_object(id) else {
            return;
        };
        let owner = object.owner;
        let card = object.card;
        object.tapped = false;
        object.damage = 0;
        object.summoning_sick = false;
        object.pump_power = 0;
        object.pump_toughness = 0;
        let new_id = self.insert_object(object, Zone::Library);
        let library = &mut self.state.players[owner as usize].library;
        library.retain(|id| *id != new_id);
        library.insert(0, new_id);
        events.push(Event::StateBasedAction {
            description: format!("{card} ({new_id}) is put on top of its owner's library"),
        });
    }

    fn move_to_exile(&mut self, id: ObjectId, events: &mut Vec<Event>) -> Option<ObjectId> {
        let object = self.detach_object(id)?;
        let card = object.card;
        let new_id = self.insert_object(object, Zone::Exile);
        events.push(Event::StateBasedAction {
            description: format!("{card} ({new_id}) is exiled"),
        });
        Some(new_id)
    }

    fn move_to_hand(&mut self, id: ObjectId, events: &mut Vec<Event>) {
        let Some(mut object) = self.detach_object(id) else {
            return;
        };
        let card = object.card;
        object.tapped = false;
        object.damage = 0;
        object.summoning_sick = false;
        object.pump_power = 0;
        object.pump_toughness = 0;
        let new_id = self.insert_object(object, Zone::Hand);
        events.push(Event::StateBasedAction {
            description: format!("{card} ({new_id}) is returned to its owner's hand"),
        });
    }

    fn move_to_graveyard(&mut self, id: ObjectId, events: &mut Vec<Event>) {
        if let Some((dying, new_id, card, controller)) = self.move_to_graveyard_inner(id, events) {
            let death = (dying, new_id, card, controller);
            self.queue_watchers_for_deaths(&death_watchers(self), &[death]);
        }
    }

    /// Move one permanent to its owner's graveyard and return the death record
    /// for the "creature dies" watcher dispatch. When `dispatch == false` the
    /// caller batches several simultaneous deaths and dispatches the watchers
    /// once for the whole batch (CR 704.3, 603.6c): a watcher that dies in the
    /// same batch must still see its fellow dying creatures.
    fn move_to_graveyard_inner(
        &mut self,
        id: ObjectId,
        events: &mut Vec<Event>,
    ) -> Option<(ObjectId, ObjectId, CardId, PlayerId)> {
        let dying = self.state.try_object(id).and_then(|object| {
            (object.zone == Zone::Battlefield).then_some((object.card, object.controller))
        });
        let token = self
            .state
            .try_object(id)
            .is_some_and(|object| object.card.is_token());
        let mut object = self.detach_object(id)?;
        let card = object.card;
        object.tapped = false;
        object.damage = 0;
        object.summoning_sick = false;
        object.pump_power = 0;
        object.pump_toughness = 0;
        let new_id = self.insert_object(object, Zone::Graveyard);
        events.push(Event::Destroyed {
            object: new_id,
            card,
        });
        if token {
            events.push(Event::StateBasedAction {
                description: format!(
                    "token {card} is put into the graveyard and will cease to exist (CR 111.7)"
                ),
            });
        }
        // CR 700.4 + 603.6c: a permanent put into a graveyard from the
        // battlefield "dies". The trigger keeps the card and its controller as
        // last known information; `source` is the graveyard object.
        if let Some((card, controller)) = dying {
            self.queue_triggers(new_id, card, controller, TriggerEvent::Dies);
            // The dying permanent's own "this creature ... dies" trigger is
            // order-independent, so it is queued here rather than in the batch.
            self.queue_self_death(new_id, card, controller);
            return Some((id, new_id, card, controller));
        }
        None
    }

    fn enter_battlefield(&mut self, id: ObjectId, controller: PlayerId, events: &mut Vec<Event>) {
        let Some(mut object) = self.detach_object(id) else {
            return;
        };
        let card = object.card;
        let is_creature = card.def().is_creature();
        object.controller = controller;
        object.tapped = false;
        object.damage = 0;
        // CR 302.6: summoning sickness applies until the controller's next
        // untap step.
        object.summoning_sick = is_creature;
        object.pump_power = 0;
        object.pump_toughness = 0;
        let new_id = self.insert_object(object, Zone::Battlefield);
        events.push(Event::PermanentEntered {
            object: new_id,
            card,
            controller,
        });
        // CR 603.6a: an enters-the-battlefield trigger fires once the permanent
        // is on the battlefield.
        self.queue_triggers(new_id, card, controller, TriggerEvent::EntersBattlefield);
        if is_creature {
            self.queue_another_creature_enters(new_id);
        }
    }

    fn draw(&mut self, p: PlayerId, events: &mut Vec<Event>) -> bool {
        self.draw_with_resume(p, DrawResume::Step, 0, events)
    }

    /// Draw one card. If the top card is a commander, CR 903.9b suspends the
    /// draw before the move and records the exact resume context.
    fn draw_with_resume(
        &mut self,
        p: PlayerId,
        resume: DrawResume,
        remaining: u8,
        events: &mut Vec<Event>,
    ) -> bool {
        let next = self.state.players[p as usize].library.first().copied();
        match next {
            Some(id) => {
                if let Some(object) = self.state.try_object(id)
                    && let Some(commander) = object.commander
                    && self.state.pending.is_none()
                {
                    let owner = self
                        .state
                        .commanders
                        .iter()
                        .find(|registry| registry.id == commander)
                        .map(|registry| registry.owner)
                        .unwrap_or(p);
                    self.state.pending = Some(Pending::CommanderDraw {
                        player: owner,
                        commander,
                        object: id,
                        remaining,
                        resume,
                    });
                    self.state.turn.priority = None;
                    return true;
                }
                let Some(mut object) = self.detach_object(id) else {
                    return false;
                };
                let card = object.card;
                object.tapped = false;
                object.damage = 0;
                object.summoning_sick = false;
                object.pump_power = 0;
                object.pump_toughness = 0;
                let new_id = self.insert_object(object, Zone::Hand);
                events.push(Event::Draw {
                    player: p,
                    object: new_id,
                    card,
                });
                false
            }
            None => {
                self.state.players[p as usize].drew_from_empty = true;
                events.push(Event::DrewFromEmpty { player: p });
                false
            }
        }
    }

    /// CR 701.13a: mill N cards. The top card of `player`'s library becomes the
    /// top card of their graveyard, one card at a time, one `Milled` event each.
    /// A milled card never entered the battlefield, so no "dies" trigger fires
    /// (CR 700.4 requires the battlefield as the origin). Milling an empty
    /// library is legal and simply does nothing: only a *draw* from an empty
    /// library loses the game (CR 704.5b), and this is not a draw.
    fn mill_cards(&mut self, player: PlayerId, count: u8, events: &mut Vec<Event>) {
        for _ in 0..count {
            let Some(next) = self.state.players[player as usize].library.first().copied() else {
                break;
            };
            let Some(card) = self.state.try_object(next).map(|object| object.card) else {
                break;
            };
            let Some(mut object) = self.detach_object(next) else {
                break;
            };
            object.tapped = false;
            object.damage = 0;
            object.summoning_sick = false;
            object.pump_power = 0;
            object.pump_toughness = 0;
            let new_id = self.insert_object(object, Zone::Graveyard);
            events.push(Event::Milled {
                player,
                object: new_id,
                card,
            });
        }
    }

    /// CR 118.2 + 119.3: a player loses life. Life loss is not damage — nothing
    /// is dealt, so damage prevention, damage replacement and "is dealt damage"
    /// triggers never see it. Only the life total (and a single `LifeChange`
    /// event) changes; CR 704.5a is then checked by the SBA pass.
    fn lose_life(&mut self, player: PlayerId, amount: i32, events: &mut Vec<Event>) {
        if amount <= 0 || !self.state.is_live(player) {
            return;
        }
        let life = self.state.players[player as usize].life - amount;
        self.state.players[player as usize].life = life;
        events.push(Event::LifeChange {
            player,
            delta: -amount,
            life,
        });
    }

    /// The dying creature's own "this creature ... dies" trigger. Blood Artist
    /// prints "this creature or another creature dies"; Zulaport Cutthroat
    /// restricts that to creatures its controller controls, which includes
    /// itself. Independent of batch order, so it is queued at the move.
    fn queue_self_death(&mut self, dying: ObjectId, card: CardId, controller: PlayerId) {
        // CR 700.4: only a creature dying is a "creature dies" event; a milled
        // creature card or a sacrificed artifact never triggers these.
        if !card.def().is_creature() {
            return;
        }
        self.queue_triggers(dying, card, controller, TriggerEvent::CreatureDies);
        self.queue_triggers(
            dying,
            card,
            controller,
            TriggerEvent::CreatureYouControlDies,
        );
    }

    /// Queue the "creature dies" watchers once for a whole batch of deaths.
    ///
    /// CR 704.3 + 603.6c: simultaneously dying creatures are one event, so every
    /// watcher that was on the battlefield when the batch happened looks at
    /// every death in it — including a watcher that died in the same batch. The
    /// caller snapshots the watchers *before* moving anything (see
    /// `death_watchers`) and passes every death record from the batch, which is
    /// what makes the result independent of object-id order.
    fn queue_watchers_for_deaths(
        &mut self,
        watchers: &[(ObjectId, CardId, PlayerId)],
        deaths: &[(ObjectId, ObjectId, CardId, PlayerId)],
    ) {
        let deaths: Vec<(ObjectId, CardId, PlayerId)> = deaths
            .iter()
            .filter(|(_, _, card, _)| card.def().is_creature())
            .map(|(dying, _, card, controller)| (*dying, *card, *controller))
            .collect();
        if deaths.is_empty() {
            return;
        }
        for (source, source_card, source_controller) in watchers.iter().copied() {
            for (dying, _death_card, death_controller) in deaths.iter().copied() {
                // The dying permanent's own "this creature ... dies" triggers
                // were already queued by `queue_self_death` at the zone change.
                // Dispatching it again through the watcher batch would double
                // every self-death trigger.
                if dying == source {
                    continue;
                }
                self.queue_triggers(
                    source,
                    source_card,
                    source_controller,
                    TriggerEvent::CreatureDies,
                );
                if source_controller == death_controller {
                    self.queue_triggers(
                        source,
                        source_card,
                        source_controller,
                        TriggerEvent::CreatureYouControlDies,
                    );
                    self.queue_triggers(
                        source,
                        source_card,
                        source_controller,
                        TriggerEvent::AnotherCreatureYouControlDies,
                    );
                }
            }
        }
    }

    // ------------------------------------------------------------ submission

    pub fn submit(&mut self, sub: &Submission) -> Result<Vec<Event>, EngineError> {
        if self.state.is_over() {
            return Err(EngineError::GameOver);
        }
        if self.state.try_player(sub.player).is_none() {
            return Err(EngineError::UnknownSeat { seat: sub.player });
        }
        if sub.revision != self.state.revision {
            return Err(EngineError::StaleRevision {
                expected: self.state.revision,
                got: sub.revision,
            });
        }
        if sub.decision != self.state.decision {
            return Err(EngineError::StaleDecision {
                expected: self.state.decision,
                got: sub.decision,
            });
        }
        let owner = self
            .state
            .decision_owner()
            .ok_or(EngineError::NoDecisionPoint)?;
        if sub.player != owner {
            return Err(EngineError::NotYourDecision {
                expected: owner,
                got: sub.player,
            });
        }
        self.validate(sub.player, &sub.action)?;
        let mut events = Vec::new();
        self.apply(sub.player, &sub.action, &mut events);
        self.state.revision += 1;
        self.state.decision += 1;
        self.settle(&mut events);
        Ok(events)
    }

    /// Convenience for drivers: submit at the current revision/decision.
    pub fn submit_action(
        &mut self,
        player: PlayerId,
        action: Action,
    ) -> Result<Vec<Event>, EngineError> {
        let sub = Submission {
            player,
            revision: self.state.revision,
            decision: self.state.decision,
            action,
        };
        self.submit(&sub)
    }

    // ------------------------------------------------------------ validation

    pub fn validate(&self, player: PlayerId, action: &Action) -> Result<(), EngineError> {
        let state = &self.state;
        if state.try_player(player).is_none() {
            return Err(EngineError::UnknownSeat { seat: player });
        }
        if let Some(pending) = &state.pending {
            if pending.player() != player {
                return Err(EngineError::NotYourDecision {
                    expected: pending.player(),
                    got: player,
                });
            }
        } else if state.turn.priority != Some(player) {
            return Err(EngineError::NoDecisionPoint);
        }
        match action {
            Action::Pass => {
                if state.pending.is_some() {
                    return Err(EngineError::illegal(
                        "pass",
                        "a pending choice must be answered; no player has priority (CR 608.2d)",
                    ));
                }
                Ok(())
            }
            Action::PlayLand { card } => self.validate_play_land(player, *card),
            Action::CastSpell { card, targets } => self.validate_cast(player, *card, targets),
            Action::ActivateManaAbility { land } => self.validate_mana_ability(player, *land),
            Action::ActivateAbility {
                source,
                ability,
                mode,
                targets,
            } => self.validate_activate_ability(player, *source, *ability, *mode, targets),
            Action::ChooseTriggerTarget { target } => match &state.pending {
                Some(Pending::TriggerTarget {
                    player: p, legal, ..
                }) if *p == player => {
                    if !legal.contains(target) {
                        return Err(EngineError::illegal(
                            "choose trigger target",
                            format!(
                                "{} is not one of the offered legal targets (CR 603.3d)",
                                self.describe_target(*target)
                            ),
                        ));
                    }
                    Ok(())
                }
                _ => Err(EngineError::illegal(
                    "choose trigger target",
                    "no triggered ability is waiting for a target choice (CR 603.3d)",
                )),
            },
            Action::AnswerTriggerChoice { .. } => match &state.pending {
                Some(Pending::TriggerChoice { player: p, .. }) if *p == player => Ok(()),
                _ => Err(EngineError::illegal(
                    "trigger choice",
                    "no optional triggered ability is waiting for its controller (CR 603.5)",
                )),
            },
            Action::DeclareAttackers {
                attackers,
                defender,
            } => self.validate_declare_attackers(player, attackers, *defender),
            Action::DeclareBlockers { blocks } => self.validate_declare_blockers(player, blocks),
            Action::Mulligan { keep } => match &state.pending {
                Some(Pending::Mulligan { player: p }) if *p == player => {
                    if !*keep
                        && state.player(player).mulligans >= mulligan_limit(state.seat_count())
                    {
                        return Err(EngineError::illegal(
                            "mulligan",
                            format!(
                                "another mulligan would require bottoming {} cards from a fresh seven; keep instead (CR 103.5)",
                                cards_to_bottom(
                                    state.seat_count(),
                                    state.player(player).mulligans + 1
                                )
                            ),
                        ));
                    }
                    Ok(())
                }
                _ => Err(EngineError::illegal(
                    "mulligan choice",
                    "no mulligan decision is pending (CR 103.5)",
                )),
            },
            Action::BottomCards { cards } => match &state.pending {
                Some(Pending::BottomCards { player: p, count }) if *p == player => {
                    let hand = &state.player(player).hand;
                    check_selection(cards, hand, *count as usize, "bottom")
                }
                _ => Err(EngineError::illegal(
                    "bottom cards",
                    "no London mulligan bottoming is pending (CR 103.5)",
                )),
            },
            Action::OrderCard { to_top } => match &state.pending {
                Some(Pending::OrderCard { player: p, .. }) if *p == player => {
                    let _ = to_top;
                    Ok(())
                }
                _ => Err(EngineError::illegal(
                    "order card",
                    "no scry/surveil decision is pending (CR 701.22a, 701.25a)",
                )),
            },
            Action::Discard { cards } => match &state.pending {
                Some(Pending::Discard { player: p, count }) if *p == player => {
                    let hand = &state.player(player).hand;
                    check_selection(cards, hand, *count as usize, "discard")
                }
                _ => Err(EngineError::illegal(
                    "discard",
                    "no cleanup discard is pending (CR 514.1)",
                )),
            },
            Action::ChooseSacrifice { permanent } => match &state.pending {
                Some(Pending::ApnapSacrifice { player: p, .. }) if *p == player => {
                    let permanents = self.controlled_permanents(player);
                    match permanent {
                        Some(id) => {
                            let object = state.try_object(*id).ok_or(EngineError::UnknownObject)?;
                            if object.zone != Zone::Battlefield || object.controller != player {
                                return Err(EngineError::WrongZone {
                                    name: object.card.name(),
                                    zone: "battlefield controlled by the choosing player",
                                });
                            }
                            if !permanents.contains(id) {
                                return Err(EngineError::illegal(
                                    "choose sacrifice",
                                    "that permanent is not controlled by the choosing seat",
                                ));
                            }
                            Ok(())
                        }
                        None if permanents.is_empty() => Ok(()),
                        None => Err(EngineError::illegal(
                            "choose sacrifice",
                            "a controlled permanent exists, so one must be chosen (CR 701.21a)",
                        )),
                    }
                }
                _ => Err(EngineError::illegal(
                    "choose sacrifice",
                    "no simultaneous sacrifice choice is pending (CR 101.4)",
                )),
            },
            Action::AnswerCommanderDraw { .. } => match &state.pending {
                Some(Pending::CommanderDraw {
                    player: owner,
                    commander,
                    object,
                    resume,
                    ..
                }) if *owner == player => {
                    let registry = state
                        .commanders
                        .iter()
                        .find(|registry| registry.id == *commander)
                        .ok_or(EngineError::InvalidCommanderSetup {
                            reason: "draw choice references an unknown commander".to_string(),
                        })?;
                    if registry.owner != player || registry.object != *object {
                        return Err(EngineError::illegal(
                            "commander draw choice",
                            "the choice no longer matches the commander registry",
                        ));
                    }
                    if state.player(player).library.first() != Some(object) {
                        return Err(EngineError::illegal(
                            "commander draw choice",
                            "the commander is no longer the top card of that library",
                        ));
                    }
                    if let DrawResume::Effect { source, cursor } = resume {
                        let Some(top) = state.stack.last() else {
                            return Err(EngineError::illegal(
                                "commander draw choice",
                                "the resolving draw source is gone",
                            ));
                        };
                        if top.object != *source
                            || top.cursor != *cursor
                            || state.resolving != Some(*source)
                        {
                            return Err(EngineError::illegal(
                                "commander draw choice",
                                "the draw source/cursor no longer matches",
                            ));
                        }
                    }
                    Ok(())
                }
                _ => Err(EngineError::illegal(
                    "commander draw choice",
                    "no commander draw replacement is pending (CR 903.9b)",
                )),
            },
            Action::AnswerCommanderReplacement { .. } => match &state.pending {
                Some(Pending::CommanderReplacement {
                    player: owner,
                    commander,
                    object,
                    from_zone,
                    destination,
                    source,
                    cursor,
                }) if *owner == player => {
                    let registry = state
                        .commanders
                        .iter()
                        .find(|registry| registry.id == *commander)
                        .ok_or(EngineError::InvalidCommanderSetup {
                            reason: "replacement choice references an unknown commander"
                                .to_string(),
                        })?;
                    if registry.owner != player
                        || registry.object != *object
                        || !matches!(destination, Zone::Hand | Zone::Library)
                        || state.object(*object).zone != *from_zone
                    {
                        return Err(EngineError::illegal(
                            "commander replacement choice",
                            "the choice no longer matches the commander registry/object",
                        ));
                    }
                    let Some(top) = state.stack.last() else {
                        return Err(EngineError::illegal(
                            "commander replacement choice",
                            "the resolving source is no longer on the stack",
                        ));
                    };
                    if top.object != *source
                        || top.cursor != *cursor
                        || state.resolving != Some(*source)
                    {
                        return Err(EngineError::illegal(
                            "commander replacement choice",
                            "the source/cursor no longer matches the suspended resolution",
                        ));
                    }
                    Ok(())
                }
                _ => Err(EngineError::illegal(
                    "commander replacement choice",
                    "no commander replacement choice is pending (CR 903.9b)",
                )),
            },
            Action::AnswerCommanderReturn { to_command_zone } => {
                let _ = to_command_zone;
                match &state.pending {
                    Some(Pending::CommanderReturn { order, cursor, .. })
                        if *cursor < order.len() && order[*cursor].player == player =>
                    {
                        let current = &order[*cursor];
                        let registry = state
                            .commanders
                            .iter()
                            .find(|registry| registry.id == current.commander)
                            .ok_or(EngineError::InvalidCommanderSetup {
                                reason: "return choice references an unknown commander".to_string(),
                            })?;
                        if registry.object != current.object
                            || !registry.return_eligible
                            || registry.owner != current.player
                        {
                            return Err(EngineError::illegal(
                                "commander return choice",
                                "the choice no longer matches the commander registry",
                            ));
                        }
                        let object = state
                            .try_object(current.object)
                            .ok_or(EngineError::UnknownObject)?;
                        if object.zone != current.zone
                            || !matches!(current.zone, Zone::Graveyard | Zone::Exile)
                        {
                            return Err(EngineError::illegal(
                                "commander return choice",
                                "the commander is no longer in that graveyard/exile object",
                            ));
                        }
                        Ok(())
                    }
                    _ => Err(EngineError::illegal(
                        "commander return choice",
                        "no commander return choice is pending (CR 903.9a)",
                    )),
                }
            }
        }
    }

    fn validate_play_land(&self, player: PlayerId, card: ObjectId) -> Result<(), EngineError> {
        // A pending choice (including CR 903.9a commander return) is not
        // priority. Reject special actions before any card/land-limit checks.
        if self.state.pending.is_some() || self.state.turn.priority != Some(player) {
            return Err(EngineError::illegal(
                "play land",
                "a pending choice is unresolved; no player has priority (CR 305.1, 608.2d)",
            ));
        }
        let obj = self
            .state
            .try_object(card)
            .ok_or(EngineError::UnknownObject)?;
        if obj.zone != Zone::Hand || obj.owner != player {
            return Err(EngineError::WrongZone {
                name: obj.card.name(),
                zone: "hand of the acting player",
            });
        }
        if !obj.card.def().is_land() {
            return Err(EngineError::illegal(
                "play land",
                format!("{} is not a land; it must be cast", obj.card.name()),
            ));
        }
        if self.state.turn.active != player {
            return Err(EngineError::illegal(
                "play land",
                "only the active player may play a land (CR 305.1, 701.18a)",
            ));
        }
        if !self.state.turn.step.is_main() {
            return Err(EngineError::illegal(
                "play land",
                "a land may be played only during your own main phase (CR 701.18a)",
            ));
        }
        if !self.state.stack.is_empty() {
            return Err(EngineError::illegal(
                "play land",
                "the stack must be empty to play a land (CR 701.18a)",
            ));
        }
        if self.state.turn.lands_played >= 1 {
            return Err(EngineError::illegal(
                "play land",
                "already played a land this turn (CR 305.2)",
            ));
        }
        Ok(())
    }

    fn validate_mana_ability(&self, player: PlayerId, land: ObjectId) -> Result<(), EngineError> {
        // CR 605.3a: a mana ability may be activated when the player has
        // priority, or while a payment is being made. None of this slice's
        // pending choices asks for a payment, so a pending choice excludes it.
        if self.state.pending.is_some() || self.state.turn.priority != Some(player) {
            return Err(EngineError::illegal(
                "mana ability",
                "mana abilities need priority and cannot be used while a choice is pending (CR 605.3a, 608.2d)",
            ));
        }
        let obj = self
            .state
            .try_object(land)
            .ok_or(EngineError::UnknownObject)?;
        if obj.zone != Zone::Battlefield || obj.controller != player {
            return Err(EngineError::WrongZone {
                name: obj.card.name(),
                zone: "battlefield",
            });
        }
        if obj.tapped {
            return Err(EngineError::illegal(
                "mana ability",
                format!("{} is already tapped", obj.card.name()),
            ));
        }
        if obj.card.def().basic_mana.is_none() {
            return Err(EngineError::NotAManaSource);
        }
        Ok(())
    }

    /// Legal target combinations for a spell in hand, deterministic order.
    pub fn target_combinations(&self, caster: PlayerId, card: ObjectId) -> Vec<Vec<Target>> {
        let def = match self.state.try_object(card) {
            Some(o) => o.card.def(),
            None => return Vec::new(),
        };
        let Some(spec) = def.spell_spec() else {
            return Vec::new();
        };
        let targets = self.legal_targets(caster, spec.target, card);
        if spec.target == TargetSpec::None {
            vec![Vec::new()]
        } else {
            targets.into_iter().map(|t| vec![t]).collect()
        }
    }

    pub fn legal_targets(
        &self,
        _caster: PlayerId,
        spec: TargetSpec,
        exclude: ObjectId,
    ) -> Vec<Target> {
        match spec {
            TargetSpec::None => Vec::new(),
            TargetSpec::AnyTarget => {
                let mut out: Vec<Target> = (0..self.state.seat_count() as PlayerId)
                    .filter(|p| self.state.is_live(*p))
                    .map(Target::Player)
                    .collect();
                out.extend(
                    self.state
                        .objects
                        .values()
                        .filter(|o| o.zone == Zone::Battlefield && o.is_creature())
                        .map(|o| Target::Permanent(o.id)),
                );
                out
            }
            // CR 115.4: "target player" is every live player, the ability's own
            // controller included (Hedron Crab, Blood Artist).
            TargetSpec::Player => (0..self.state.seat_count() as PlayerId)
                .filter(|p| self.state.is_live(*p))
                .map(Target::Player)
                .collect(),
            TargetSpec::Creature => self
                .state
                .objects
                .values()
                .filter(|o| o.zone == Zone::Battlefield && o.is_creature())
                .map(|o| Target::Permanent(o.id))
                .collect(),
            TargetSpec::OpponentCreature => self
                .state
                .objects
                .values()
                .filter(|o| {
                    o.zone == Zone::Battlefield && o.is_creature() && o.controller != _caster
                })
                .map(|o| Target::Permanent(o.id))
                .collect(),
            TargetSpec::Spell => self
                .state
                .stack
                .iter()
                // An ability on the stack is not a spell (CR 115.1, 602.2).
                .filter(|s| s.object != exclude && s.entry == StackEntry::Spell)
                .map(|s| Target::Spell(s.object))
                .collect(),
            TargetSpec::NoncreatureSpell => self
                .state
                .stack
                .iter()
                // CR 115.1: only a spell is a spell; and a creature spell is
                // excluded because the printed wording says "noncreature"
                // (CR 205.2a: the card's types on the stack are the printed
                // types).
                .filter(|s| {
                    s.object != exclude
                        && s.entry == StackEntry::Spell
                        && !self
                            .state
                            .try_object(s.object)
                            .is_some_and(|object| object.card.def().is_creature())
                })
                .map(|s| Target::Spell(s.object))
                .collect(),
        }
    }

    fn validate_cast(
        &self,
        player: PlayerId,
        card: ObjectId,
        targets: &[Target],
    ) -> Result<(), EngineError> {
        // A pending choice means no player has priority. Refuse at the action
        // validator even if a forged/unvalidated state still carries priority.
        if self.state.pending.is_some() {
            return Err(EngineError::illegal(
                "cast spell",
                "a pending choice must be answered before any spell is cast (CR 608.2d)",
            ));
        }
        let obj = self
            .state
            .try_object(card)
            .ok_or(EngineError::UnknownObject)?;
        let def = obj.card.def();
        let from_command_zone = obj.zone == Zone::Command;
        if from_command_zone {
            let Some(commander) = obj.commander else {
                return Err(EngineError::WrongZone {
                    name: def.name,
                    zone: "hand or its owner's command zone",
                });
            };
            let registry = self
                .state
                .commanders
                .iter()
                .find(|c| c.id == commander)
                .ok_or(EngineError::InvalidCommanderSetup {
                    reason: format!("object {} carries an unknown commander", card),
                })?;
            if obj.owner != player || registry.owner != player {
                return Err(EngineError::illegal(
                    "cast spell",
                    "only the commander's owner may cast it from the command zone (CR 903.8)",
                ));
            }
        } else if obj.zone != Zone::Hand || obj.owner != player {
            return Err(EngineError::WrongZone {
                name: def.name,
                zone: "hand of the acting player",
            });
        }
        let Some(spec) = def.spell_spec() else {
            return Err(EngineError::illegal(
                "cast spell",
                format!("{} is not a spell", def.name),
            ));
        };
        self.check_timing(player, def, spec)?;

        let required = if spec.target == TargetSpec::None {
            0
        } else {
            1
        };
        if targets.len() != required {
            return Err(EngineError::TargetKind { card: def.id });
        }
        if required == 1 {
            let legal = self.legal_targets(player, spec.target, card);
            if !legal.contains(&targets[0]) {
                return Err(EngineError::BadTarget {
                    card: def.id,
                    reason: format!(
                        "{} is not a legal target for {} ({})",
                        self.describe_target(targets[0]),
                        def.name,
                        target_spec_rule(spec.target)
                    ),
                });
            }
        }
        let cost = self.effective_cost(card)?;
        self.plan_payment(player, &cost)?;
        Ok(())
    }

    fn check_timing(
        &self,
        player: PlayerId,
        def: &CardDef,
        spec: SpellSpec,
    ) -> Result<(), EngineError> {
        if self.state.turn.priority != Some(player) {
            return Err(EngineError::BadTiming {
                name: def.name,
                reason: "that player does not have priority (CR 117.1a)".to_string(),
            });
        }
        // CR 702.8a: Flash is the only printed permission to cast a noninstant
        // spell at instant timing; it covers the whole of `check_timing`, not
        // just the main-phase clause.
        if spec.instant || def.has_flash() {
            return Ok(());
        }
        if self.state.turn.active != player {
            return Err(EngineError::BadTiming {
                name: def.name,
                reason: "a noninstant spell can be cast only during its controller's main phase (CR 117.1a)".to_string(),
            });
        }
        if !self.state.turn.step.is_main() {
            return Err(EngineError::BadTiming {
                name: def.name,
                reason: format!(
                    "a noninstant spell needs a main phase, current step is {} (CR 117.1a)",
                    self.state.turn.step.name()
                ),
            });
        }
        if !self.state.stack.is_empty() {
            return Err(EngineError::BadTiming {
                name: def.name,
                reason: "a noninstant spell needs an empty stack (CR 117.1a)".to_string(),
            });
        }
        Ok(())
    }

    fn validate_declare_attackers(
        &self,
        player: PlayerId,
        attackers: &[ObjectId],
        defender: Option<PlayerId>,
    ) -> Result<(), EngineError> {
        match &self.state.pending {
            Some(Pending::DeclareAttackers { player: p }) if *p == player => {}
            _ => {
                return Err(EngineError::illegal(
                    "declare attackers",
                    "it is not that player's declaration (CR 508.1a)",
                ));
            }
        }
        if attackers.is_empty() {
            if defender.is_some() {
                return Err(EngineError::illegal(
                    "declare attackers",
                    "a defending player cannot be chosen when no attackers are declared",
                ));
            }
        } else {
            let Some(defender) = defender else {
                return Err(EngineError::illegal(
                    "declare attackers",
                    "an attacking declaration must choose one defending player (CR 508.1a)",
                ));
            };
            if defender == player || !self.state.is_live(defender) {
                return Err(EngineError::illegal(
                    "declare attackers",
                    format!("P{defender} is not a legal defending player"),
                ));
            }
        }
        let legal = self.legal_attackers(player);
        let mut seen = Vec::new();
        for a in attackers {
            if seen.contains(a) {
                return Err(EngineError::illegal(
                    "declare attackers",
                    "the same creature was declared twice",
                ));
            }
            seen.push(*a);
            if self.state.try_object(*a).is_some_and(|o| o.has_defender()) {
                return Err(EngineError::illegal(
                    "declare attackers",
                    format!(
                        "{} has defender and cannot attack (CR 702.3b)",
                        self.describe_object(*a)
                    ),
                ));
            }
            if !legal.contains(a) {
                return Err(EngineError::illegal(
                    "declare attackers",
                    format!(
                        "{} cannot attack: it must be an untapped creature controlled since the turn began (CR 302.6, 508.1a)",
                        self.describe_object(*a)
                    ),
                ));
            }
        }
        Ok(())
    }

    fn validate_declare_blockers(
        &self,
        player: PlayerId,
        blocks: &[BlockDecl],
    ) -> Result<(), EngineError> {
        match &self.state.pending {
            Some(Pending::DeclareBlockers { player: p }) if *p == player => {}
            _ => {
                return Err(EngineError::illegal(
                    "declare blockers",
                    "it is not that player's declaration (CR 509.1a)",
                ));
            }
        }
        let legal = self.legal_blockers(player);
        let mut used = Vec::new();
        for block in blocks {
            if used.contains(&block.blocker) {
                return Err(EngineError::illegal(
                    "declare blockers",
                    "a creature can block only one attacker unless an effect says otherwise (CR 509.1a)",
                ));
            }
            used.push(block.blocker);
            if !legal.contains(&block.blocker) {
                return Err(EngineError::illegal(
                    "declare blockers",
                    format!(
                        "{} cannot block: it must be an untapped creature you control (CR 509.1a)",
                        self.describe_object(block.blocker)
                    ),
                ));
            }
            if !self.state.turn.attackers.contains(&block.attacker) {
                return Err(EngineError::illegal(
                    "declare blockers",
                    format!(
                        "{} is not attacking (CR 509.1a)",
                        self.describe_object(block.attacker)
                    ),
                ));
            }
            // CR 509.1b: the pair must be legal, not just each object. Flying is
            // the only evasion this slice models (CR 702.9b, 702.17b).
            let attacker = self.state.object(block.attacker);
            let blocker = self.state.object(block.blocker);
            if !crate::tokens::can_block(attacker.keywords(), blocker.keywords()) {
                return Err(EngineError::illegal(
                    "declare blockers",
                    format!(
                        "{} cannot block {} without flying or reach (CR 509.1b, 702.9b)",
                        blocker.name(),
                        attacker.name()
                    ),
                ));
            }
        }
        Ok(())
    }

    pub fn legal_attackers(&self, player: PlayerId) -> Vec<ObjectId> {
        self.state
            .objects
            .values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                    && o.controller == player
                    && o.is_creature()
                    && !o.tapped
                    && !o.summoning_sick
                    // CR 702.3b: defender can't attack.
                    && !o.has_keyword(crate::tokens::Keyword::Defender)
            })
            .map(|o| o.id)
            .collect()
    }

    pub fn legal_blockers(&self, player: PlayerId) -> Vec<ObjectId> {
        self.state
            .objects
            .values()
            .filter(|o| {
                o.zone == Zone::Battlefield
                    && o.controller == player
                    && o.is_creature()
                    && !o.tapped
            })
            .map(|o| o.id)
            .collect()
    }

    // --------------------------------------------------------------- mutation

    fn apply(&mut self, player: PlayerId, action: &Action, events: &mut Vec<Event>) {
        match action {
            Action::Pass => self.apply_pass(player, events),
            Action::PlayLand { card } => self.apply_play_land(player, *card, events),
            Action::CastSpell { card, targets } => self.apply_cast(player, *card, targets, events),
            Action::ActivateManaAbility { land } => self.apply_mana_ability(player, *land, events),
            Action::DeclareAttackers {
                attackers,
                defender,
            } => self.apply_declare_attackers(player, attackers, *defender, events),
            Action::DeclareBlockers { blocks } => {
                self.apply_declare_blockers(player, blocks, events)
            }
            Action::Mulligan { keep } => self.apply_mulligan(player, *keep, events),
            Action::BottomCards { cards } => self.apply_bottom(player, cards, events),
            Action::OrderCard { to_top } => self.apply_order_card(player, *to_top, events),
            Action::Discard { cards } => self.apply_discard(player, cards, events),
            Action::ChooseSacrifice { permanent } => {
                self.apply_apnap_sacrifice(player, *permanent, events)
            }
            Action::AnswerCommanderReturn { to_command_zone } => {
                self.apply_commander_return(player, *to_command_zone, events)
            }
            Action::AnswerCommanderReplacement { to_command_zone } => {
                self.apply_commander_replacement(player, *to_command_zone, events)
            }
            Action::AnswerCommanderDraw { to_command_zone } => {
                self.apply_commander_draw(player, *to_command_zone, events)
            }
            Action::ActivateAbility {
                source,
                ability,
                mode,
                targets,
            } => self.apply_activate_ability(player, *source, *ability, *mode, targets, events),
            Action::ChooseTriggerTarget { target } => {
                self.apply_trigger_target(player, *target, events)
            }
            Action::AnswerTriggerChoice { use_ability } => {
                self.apply_trigger_choice(player, *use_ability, events)
            }
        }
    }

    fn apply_pass(&mut self, player: PlayerId, events: &mut Vec<Event>) {
        events.push(Event::Pass { player });
        self.state.turn.passed |= 1u8 << player;
        let live_mask = self.state.live_mask();
        if self.state.turn.passed & live_mask == live_mask {
            self.state.turn.passed = 0;
            if self.state.stack.is_empty() {
                self.advance_step(events);
            } else {
                self.state.resolving = self.state.stack.last().map(|s| s.object);
                self.state.turn.priority = None;
            }
        } else {
            let next = self
                .state
                .next_live_seat_after(player)
                .expect("a live seat remains when a pass round continues");
            if self.pending_loss_exists() {
                self.state.turn.priority = None;
            } else {
                self.state.turn.priority = Some(next);
                events.push(Event::Priority { player: next });
            }
        }
    }

    fn apply_play_land(&mut self, player: PlayerId, card: ObjectId, events: &mut Vec<Event>) {
        let card_id = self.state.object(card).card;
        self.state.players[player as usize]
            .hand
            .retain(|c| *c != card);
        if let Some(obj) = self.objects_mut(card) {
            obj.zone = Zone::Battlefield;
            obj.controller = player;
            obj.tapped = false;
            obj.summoning_sick = false;
        }
        self.state.turn.lands_played += 1;
        self.state.turn.passed = 0;
        events.push(Event::LandPlayed {
            player,
            object: card,
            card: card_id,
        });
        // CR 603.6a: landfall triggers fire when the land is already on the
        // battlefield, only for permanents that player controls.
        let sources: Vec<(ObjectId, CardId)> = self
            .state
            .objects
            .values()
            .filter(|object| object.zone == Zone::Battlefield && object.controller == player)
            .filter(|object| {
                ability::triggers_on(object.card, TriggerEvent::LandEnters)
                    .next()
                    .is_some()
            })
            .map(|object| (object.id, object.card))
            .collect();
        for (source, source_card) in sources {
            self.queue_triggers(source, source_card, player, TriggerEvent::LandEnters);
        }
        self.put_triggers(events);
        events.push(Event::Priority { player });
    }

    fn apply_mana_ability(&mut self, player: PlayerId, land: ObjectId, events: &mut Vec<Event>) {
        let color = self.state.object(land).card.def().basic_mana;
        if let (Some(color), Some(obj)) = (color, self.objects_mut(land)) {
            obj.tapped = true;
            self.state.players[player as usize].mana.add(color, 1);
        }
        self.state.turn.passed = 0;
        events.push(Event::ManaTapped {
            player,
            land,
            color: color.unwrap_or(ManaColor::Colorless),
        });
        events.push(Event::Priority { player });
    }

    // --------------------------------------------------- activated abilities

    /// CR 602.2/602.5: validate one activation against the typed ability record.
    fn validate_activate_ability(
        &self,
        player: PlayerId,
        source: ObjectId,
        index: u8,
        mode: u8,
        targets: &[Target],
    ) -> Result<(), EngineError> {
        if self.state.pending.is_some() {
            return Err(EngineError::illegal(
                "activate ability",
                "a pending choice must be answered before any ability is activated (CR 608.2d)",
            ));
        }
        let object = self
            .state
            .try_object(source)
            .ok_or(EngineError::UnknownObject)?;
        if object.zone != Zone::Battlefield {
            return Err(EngineError::WrongZone {
                name: object.card.def().name,
                zone: "battlefield",
            });
        }
        if object.controller != player {
            return Err(EngineError::illegal(
                "activate ability",
                format!(
                    "only {}'s controller may activate its abilities (CR 602.2)",
                    object.card.def().name
                ),
            ));
        }
        if self.state.turn.priority != Some(player) {
            return Err(EngineError::BadTiming {
                name: object.card.def().name,
                reason: "that player does not have priority (CR 602.2)".to_string(),
            });
        }
        let Some(activated) = ability::abilities(object.card)
            .get(index as usize)
            .and_then(Ability::activated)
        else {
            return Err(EngineError::illegal(
                "activate ability",
                format!(
                    "{} has no activated ability #{index}",
                    object.card.def().name
                ),
            ));
        };
        if mode as usize >= activated.modes.len() {
            return Err(EngineError::illegal(
                "activate ability",
                format!(
                    "{} has no mode {mode} for that ability",
                    object.card.def().name
                ),
            ));
        }
        if activated.cost.tap && object.tapped {
            return Err(EngineError::illegal(
                "activate ability",
                format!(
                    "{} is already tapped, so the {{T}} cost cannot be paid (CR 602.5a)",
                    object.card.def().name
                ),
            ));
        }
        if activated.cost.tap && object.is_creature() && object.summoning_sick {
            return Err(EngineError::BadTiming {
                name: object.card.def().name,
                reason: "a creature with a {T} activation cost cannot use it until it has been under its controller's control since their most recent turn began (CR 302.6)".to_string(),
            });
        }
        if activated.once_per_turn && self.activation_used(source, index) {
            return Err(EngineError::illegal(
                "activate ability",
                format!(
                    "{} may be activated only once each turn (CR 602.5b)",
                    object.card.def().name
                ),
            ));
        }
        let required = usize::from(activated.target != TargetSpec::None);
        if targets.len() != required {
            return Err(EngineError::TargetKind { card: object.card });
        }
        if required == 1 {
            let legal = self.legal_targets(player, activated.target, source);
            if !legal.contains(&targets[0]) {
                return Err(EngineError::BadTarget {
                    card: object.card,
                    reason: format!(
                        "{} is not a legal target for this ability ({})",
                        self.describe_target(targets[0]),
                        target_spec_rule(activated.target)
                    ),
                });
            }
        }
        if let Some(mana_cost) = activated.cost.mana {
            let cost = mana::parse_cost(mana_cost);
            self.plan_payment(player, &cost)?;
        }
        Ok(())
    }

    fn activation_used(&self, source: ObjectId, index: u8) -> bool {
        self.state
            .activation_uses
            .get(&source)
            .is_some_and(|mask| mask & (1u8 << index) != 0)
    }

    fn apply_activate_ability(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        index: u8,
        _mode: u8,
        targets: &[Target],
        events: &mut Vec<Event>,
    ) {
        let card = self.state.object(source).card;
        let Some(activated) = ability::abilities(card)
            .get(index as usize)
            .and_then(Ability::activated)
            .copied()
        else {
            return;
        };
        if let Some(mana_cost) = activated.cost.mana {
            let cost = mana::parse_cost(mana_cost);
            if let Ok(payment) = self.plan_payment(player, &cost) {
                self.apply_payment(player, payment, events);
            }
        }
        let target_labels = targets
            .iter()
            .map(|t| self.describe_target(*t))
            .collect::<Vec<_>>();
        events.push(Event::AbilityActivated {
            player,
            source,
            card,
            text: activated.text.to_string(),
            targets: target_labels,
        });
        if activated.cost.tap
            && let Some(object) = self.objects_mut(source)
        {
            object.tapped = true;
        }
        if activated.once_per_turn {
            *self.state.activation_uses.entry(source).or_insert(0) |= 1u8 << index;
        }
        // CR 602.2b: the cost is paid as part of activating. Paying a
        // "sacrifice this permanent" cost moves the source, so the stack entry
        // is recorded afterwards and carries the source's card as LKI.
        if activated.cost.sacrifice_source {
            self.move_to_graveyard(source, events);
        }
        self.state.stack.push(StackItem {
            object: source,
            card,
            controller: player,
            targets: targets.to_vec(),
            cursor: 0,
            entry: StackEntry::Activated { ability: index },
            trigger_answered: false,
        });
        // CR 117.3c: activating is treated like casting for priority purposes.
        self.state.turn.passed = 0;
        self.state.turn.priority = Some(player);
        events.push(Event::Priority { player });
    }

    // --------------------------------------------------- triggered abilities

    /// Record every triggered ability of `card` that fires for `event` on the
    /// object `source`. The trigger is only queued here; it reaches the stack
    /// when a player would receive priority (CR 603.2, 603.3).
    fn queue_triggers(
        &mut self,
        source: ObjectId,
        card: CardId,
        controller: PlayerId,
        event: TriggerEvent,
    ) {
        for (index, _) in ability::triggers_on(card, event) {
            let seq = self.state.next_trigger_seq;
            self.state.next_trigger_seq += 1;
            self.state.trigger_queue.push(TriggerRef {
                source,
                card,
                controller,
                ability: index,
                seq,
            });
        }
    }

    /// CR 603.3: put every waiting triggered ability on the stack, in APNAP
    /// order (active player first, then turn order), before a player receives
    /// priority. Returns true when the queue was consumed.
    fn put_triggers(&mut self, events: &mut Vec<Event>) -> bool {
        if self.state.pending.is_some() || self.state.trigger_queue.is_empty() {
            return false;
        }
        let mut queue = std::mem::take(&mut self.state.trigger_queue);
        // A trigger whose controller left the game is never put on the stack
        // (CR 800.4a).
        queue.retain(|trigger| self.state.is_live(trigger.controller));
        if queue.is_empty() {
            return false;
        }
        let anchor = self.state.turn.active;
        let seats = self.state.seat_count();
        queue.sort_by_key(|trigger| {
            (
                (trigger.controller as usize + seats - anchor as usize) % seats,
                trigger.seq,
            )
        });
        self.stage_triggers(queue, events);
        true
    }

    /// Put triggers on the stack in the given order, asking for targets as
    /// CR 603.3d requires. A trigger with no legal target is removed instead.
    fn stage_triggers(&mut self, mut queue: Vec<TriggerRef>, events: &mut Vec<Event>) {
        while !queue.is_empty() {
            let trigger = queue.remove(0);
            let spec = ability::abilities(trigger.card)
                .get(trigger.ability as usize)
                .and_then(Ability::triggered)
                .map(|triggered| triggered.target)
                .unwrap_or(TargetSpec::None);
            if spec == TargetSpec::None {
                self.push_trigger(trigger, Vec::new(), events);
                continue;
            }
            let legal = self.legal_targets(trigger.controller, spec, trigger.source);
            match legal.len() {
                0 => events.push(Event::TriggerRemoved {
                    controller: trigger.controller,
                    card: trigger.card,
                    reason: format!(
                        "it requires a target and no legal target exists (CR 603.3d, {})",
                        target_spec_rule(spec)
                    ),
                }),
                1 => self.push_trigger(trigger, vec![legal[0]], events),
                _ => {
                    self.state.pending = Some(Pending::TriggerTarget {
                        player: trigger.controller,
                        trigger,
                        legal,
                        queued: queue,
                    });
                    self.state.turn.priority = None;
                    return;
                }
            }
        }
    }

    fn push_trigger(&mut self, trigger: TriggerRef, targets: Vec<Target>, events: &mut Vec<Event>) {
        let text = ability::abilities(trigger.card)
            .get(trigger.ability as usize)
            .map(Ability::text)
            .unwrap_or("");
        events.push(Event::TriggerStaged {
            controller: trigger.controller,
            source: trigger.source,
            card: trigger.card,
            text: text.to_string(),
        });
        self.state.stack.push(StackItem {
            object: trigger.source,
            card: trigger.card,
            controller: trigger.controller,
            targets,
            cursor: 0,
            entry: StackEntry::Triggered {
                ability: trigger.ability,
            },
            trigger_answered: false,
        });
        self.state.turn.passed = 0;
    }

    fn apply_trigger_target(&mut self, _player: PlayerId, target: Target, events: &mut Vec<Event>) {
        let Some(Pending::TriggerTarget {
            trigger, queued, ..
        }) = self.state.pending.take()
        else {
            return;
        };
        self.push_trigger(trigger, vec![target], events);
        self.stage_triggers(queued, events);
    }

    /// CR 603.5: the controller decides when an optional trigger resolves.
    fn apply_trigger_choice(
        &mut self,
        player: PlayerId,
        use_ability: bool,
        events: &mut Vec<Event>,
    ) {
        let Some(Pending::TriggerChoice { trigger, .. }) = self.state.pending.take() else {
            return;
        };
        if use_ability {
            if let Some(top) = self.state.stack.last_mut()
                && top.object == trigger.source
                && matches!(top.entry, StackEntry::Triggered { .. })
            {
                top.trigger_answered = true;
            }
            self.state.resolving = Some(trigger.source);
        } else {
            events.push(Event::TriggerDeclined {
                controller: player,
                card: trigger.card,
            });
            self.pop_ability(
                trigger.source,
                StackEntry::Triggered {
                    ability: trigger.ability,
                },
            );
            self.state.resolving = None;
        }
        self.state.turn.priority = None;
    }

    /// Remove exactly one ability stack entry, whatever happened to its source.
    fn pop_ability(&mut self, source: ObjectId, entry: StackEntry) {
        if let Some(index) = self
            .state
            .stack
            .iter()
            .rposition(|item| item.object == source && item.entry == entry)
        {
            self.state.stack.remove(index);
        }
    }

    /// CR 601.2a + 400.7: move a hand/command-zone card to the stack. A
    /// commander gets a fresh ObjectId; stale references never follow it.
    fn move_object_to_stack(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        commander: Option<crate::ids::CommanderId>,
    ) -> ObjectId {
        self.state.players[player as usize]
            .hand
            .retain(|c| *c != card);
        self.state.players[player as usize]
            .command_zone
            .retain(|c| *c != card);
        let mut object = self
            .state
            .objects
            .remove(&card)
            .expect("cast object exists in its zone");
        let new_id = if commander.is_some() {
            let id = ObjectId(self.state.next_object);
            self.state.next_object += 1;
            object.id = id;
            if let Some(registry) = self
                .state
                .commanders
                .iter_mut()
                .find(|registry| Some(registry.id) == commander)
            {
                registry.object = id;
            }
            id
        } else {
            card
        };
        object.zone = Zone::Stack;
        object.controller = player;
        object.tapped = false;
        object.damage = 0;
        object.summoning_sick = false;
        object.pump_power = 0;
        object.pump_toughness = 0;
        object.plus_one_counters = 0;
        // CR 122.2: the stack object is not the battlefield permanent, so it can
        // never carry the counters it had (defensive: only permanents do).
        object.counters.clear();
        self.state.objects.insert(new_id, object);
        new_id
    }

    fn apply_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        targets: &[Target],
        events: &mut Vec<Event>,
    ) {
        let (card_id, from_command_zone, commander_id) = {
            let obj = self.state.object(card);
            (obj.card, obj.zone == Zone::Command, obj.commander)
        };
        let cost = self
            .effective_cost(card)
            .expect("validated command-zone cost still computes");
        let payment = self
            .plan_payment(player, &cost)
            .expect("validated payment still plans");
        // CR 601.2a: move the card to the stack; CR 601.2h: pay costs.
        let stack_object = self.move_object_to_stack(player, card, commander_id);
        self.state.stack.push(StackItem {
            object: stack_object,
            card: card_id,
            controller: player,
            targets: targets.to_vec(),
            cursor: 0,
            entry: StackEntry::Spell,
            trigger_answered: false,
        });
        // CR 603.6a: "you cast" triggers are queued as the spell is cast, and
        // reach the stack above it before priority. Only the caster's
        // permanents trigger; an opponent's cast does not.
        if matches!(card_id.def().kinds, kinds if kinds.contains(&crate::card::Kind::Instant) || kinds.contains(&crate::card::Kind::Sorcery))
        {
            let sources: Vec<(ObjectId, CardId)> = self
                .state
                .objects
                .values()
                .filter(|object| object.zone == Zone::Battlefield && object.controller == player)
                .filter(|object| {
                    ability::triggers_on(object.card, TriggerEvent::SpellCast)
                        .next()
                        .is_some()
                })
                .map(|object| (object.id, object.card))
                .collect();
            for (source, card) in sources {
                self.queue_triggers(source, card, player, TriggerEvent::SpellCast);
            }
            // CR 603.3: put the "you cast" triggers on the stack above the
            // spell before any player receives priority.
            self.put_triggers(events);
        }
        if from_command_zone
            && let Some(commander) = commander_id
            && let Some(registry) = self
                .state
                .commanders
                .iter_mut()
                .find(|registry| registry.id == commander)
        {
            // CR 903.8: the count changes when the commander is cast, never at
            // resolution, and only for a command-zone cast.
            registry.casts_from_command_zone += 1;
        }
        let target_labels = targets
            .iter()
            .map(|t| self.describe_target(*t))
            .collect::<Vec<_>>();
        // CR 601.2a: the spell is on the stack before costs are paid (601.2h).
        events.push(Event::Cast {
            player,
            object: stack_object,
            card: card_id,
            targets: target_labels,
        });
        self.apply_payment(player, payment, events);
        // CR 117.3c: the caster receives priority after casting.
        self.state.turn.passed = 0;
        self.state.turn.priority = Some(player);
        events.push(Event::Priority { player });
    }

    fn apply_declare_attackers(
        &mut self,
        player: PlayerId,
        attackers: &[ObjectId],
        defender: Option<PlayerId>,
        events: &mut Vec<Event>,
    ) {
        self.state.pending = None;
        for id in attackers {
            if let Some(obj) = self.objects_mut(*id) {
                obj.tapped = true; // CR 508.1f
            }
        }
        self.state.turn.attackers = attackers.to_vec();
        self.state.turn.defender = defender;
        self.state.turn.passed = 0;
        events.push(Event::AttackersDeclared {
            player,
            attackers: attackers.to_vec(),
        });
        // CR 508.2: the active player gets priority in the declare attackers step.
        self.grant_priority(player, events);
    }

    fn apply_declare_blockers(
        &mut self,
        player: PlayerId,
        blocks: &[BlockDecl],
        events: &mut Vec<Event>,
    ) {
        self.state.pending = None;
        self.state.turn.blockers = blocks.iter().map(|b| (b.blocker, b.attacker)).collect();
        self.state.turn.blocked = blocks.iter().map(|b| b.attacker).collect();
        events.push(Event::BlockersDeclared {
            player,
            blocks: self.state.turn.blockers.clone(),
        });
        let mut blocked_attackers: Vec<ObjectId> = self
            .state
            .turn
            .blockers
            .iter()
            .map(|(_, attacker)| *attacker)
            .collect();
        blocked_attackers.sort_unstable();
        blocked_attackers.dedup();
        for attacker in blocked_attackers {
            if let Some(object) = self.state.try_object(attacker) {
                let card = object.card;
                let controller = object.controller;
                self.queue_triggers(attacker, card, controller, TriggerEvent::BecomesBlocked);
            }
        }
        self.put_triggers(events);
        // CR 509.2: the active player gets priority after blockers are declared.
        let active = self.state.turn.active;
        self.grant_priority(active, events);
    }

    fn apply_mulligan(&mut self, player: PlayerId, keep: bool, events: &mut Vec<Event>) {
        let seat_count = self.state.seat_count();
        let (order, cursor) = {
            let Some(mulligan) = self.state.mulligan.as_mut() else {
                return;
            };
            mulligan.declarations.insert(player, keep);
            mulligan.cursor += 1;
            (mulligan.order.clone(), mulligan.cursor)
        };
        if keep {
            let size = self.state.player(player).hand_size();
            events.push(Event::KeepHand { player, size });
        }
        if cursor < order.len() {
            self.state.pending = Some(Pending::Mulligan {
                player: order[cursor],
            });
            return;
        }

        // Every eligible seat has declared. Execute this round's mulligans as
        // one deterministic batch in turn order; no seat sees a new hand or
        // makes a bottom choice before the batch completes.
        let declarations = self
            .state
            .mulligan
            .as_ref()
            .map(|mulligan| mulligan.declarations.clone())
            .unwrap_or_default();
        let mulliganers: Vec<PlayerId> = order
            .iter()
            .copied()
            .filter(|seat| declarations.get(seat) == Some(&false))
            .collect();
        for seat in &mulliganers {
            let cards = std::mem::take(&mut self.state.players[*seat as usize].hand);
            let mut library = std::mem::take(&mut self.state.players[*seat as usize].library);
            for card in cards {
                if let Some(obj) = self.objects_mut(card) {
                    obj.zone = Zone::Library;
                }
                library.push(card);
            }
            self.state.rng.shuffle(&mut library);
            self.state.players[*seat as usize].library = library;
            self.state.players[*seat as usize].mulligans += 1;
            let count = self.state.players[*seat as usize].mulligans;
            events.push(Event::Mulligan {
                player: *seat,
                count,
            });
            for _ in 0..STARTING_HAND {
                self.draw(*seat, events);
            }
        }

        // London bottoming is part of taking the mulligan (CR 103.5): queue the
        // mulliganers who owe bottoms, then collect those choices without
        // disturbing the declaration-round order.
        let bottom_queue: Vec<PlayerId> = mulliganers
            .iter()
            .copied()
            .filter(|seat| cards_to_bottom(seat_count, self.state.player(*seat).mulligans) > 0)
            .collect();
        if let Some(mulligan) = self.state.mulligan.as_mut() {
            mulligan.bottom_queue = bottom_queue.clone();
        }
        if let Some(first) = bottom_queue.first().copied() {
            let count = cards_to_bottom(seat_count, self.state.player(first).mulligans);
            self.state.pending = Some(Pending::BottomCards {
                player: first,
                count,
            });
        } else {
            self.begin_next_mulligan_round(mulliganers, events);
        }
    }

    fn apply_bottom(&mut self, player: PlayerId, cards: &[ObjectId], events: &mut Vec<Event>) {
        for card in cards {
            if let Some(mut object) = self.detach_object(*card) {
                object.tapped = false;
                object.damage = 0;
                object.summoning_sick = false;
                object.pump_power = 0;
                object.pump_toughness = 0;
                self.insert_object(object, Zone::Library);
            }
        }
        events.push(Event::Bottom {
            player,
            count: cards.len(),
        });

        let remaining = {
            let Some(mulligan) = self.state.mulligan.as_mut() else {
                return;
            };
            if mulligan.bottom_queue.first() == Some(&player) {
                mulligan.bottom_queue.remove(0);
                mulligan.bottomed.push(player);
            }
            mulligan.bottom_queue.clone()
        };
        if let Some(next) = remaining.first().copied() {
            let count = cards_to_bottom(self.state.seat_count(), self.state.player(next).mulligans);
            self.state.pending = Some(Pending::BottomCards {
                player: next,
                count,
            });
        } else {
            let mulliganers = self
                .state
                .mulligan
                .as_ref()
                .map(|mulligan| {
                    mulligan
                        .order
                        .iter()
                        .copied()
                        .filter(|seat| mulligan.declarations.get(seat) == Some(&false))
                        .collect()
                })
                .unwrap_or_default();
            self.begin_next_mulligan_round(mulliganers, events);
        }
    }

    /// Start the next declaration round for exactly the seats that mulliganed;
    /// kept hands are locked out. With no mulliganers the opening procedure is
    /// complete and the first priority of turn 1 is granted.
    fn begin_next_mulligan_round(&mut self, next_order: Vec<PlayerId>, events: &mut Vec<Event>) {
        if next_order.is_empty() {
            self.state.mulligan = None;
            self.state.pending = None;
            let active = self.state.turn.active;
            self.grant_priority(active, events);
            return;
        }
        let first = {
            let mulligan = self
                .state
                .mulligan
                .as_mut()
                .expect("a mulligan round exists while starting the next one");
            for seat in &mulligan.order {
                if mulligan.declarations.get(seat) == Some(&true) && !mulligan.kept.contains(seat) {
                    mulligan.kept.push(*seat);
                }
            }
            mulligan.round += 1;
            mulligan.previous_mulliganers = next_order.clone();
            mulligan.eligible = next_order.clone();
            mulligan.order = next_order;
            mulligan.cursor = 0;
            mulligan.declarations.clear();
            mulligan.bottom_queue.clear();
            mulligan.bottomed.clear();
            mulligan.order[0]
        };
        self.state.pending = Some(Pending::Mulligan { player: first });
    }

    fn apply_order_card(&mut self, player: PlayerId, to_top: bool, events: &mut Vec<Event>) {
        let Some(Pending::OrderCard {
            viewed,
            index,
            to_top: kept_vec,
            mode,
            ..
        }) = self.state.pending.take()
        else {
            return;
        };
        let mut kept = kept_vec;
        let current = viewed[index];
        let card = self.state.object(current).card;
        if to_top {
            kept.push(current);
        } else if mode == OrderMode::Surveil {
            self.move_to_graveyard(current, events);
        }
        events.push(Event::OrderedCard {
            player,
            card,
            to_top,
            mode,
        });

        if index + 1 < viewed.len() {
            self.state.pending = Some(Pending::OrderCard {
                player,
                viewed,
                index: index + 1,
                to_top: kept,
                mode,
            });
            return;
        }

        // All cards decided: put the kept ones on top in the order they were
        // looked at, and the rest on the bottom (scry) / already in the
        // graveyard (surveil), CR 701.22a / 701.25a.
        // Surveyed cards (including a re-keyed commander) may already have left
        // the library. Never dereference a historical ObjectId; only cards still
        // in the library and not kept belong on the bottom.
        let bottom: Vec<ObjectId> = viewed
            .iter()
            .filter(|v| !kept.contains(v))
            .filter_map(|v| {
                self.state
                    .try_object(*v)
                    .filter(|object| object.zone == Zone::Library)
                    .map(|_| *v)
            })
            .collect();
        let mut library = std::mem::take(&mut self.state.players[player as usize].library);
        let mut new_library = kept;
        new_library.append(&mut library);
        new_library.extend(bottom);
        self.state.players[player as usize].library = new_library;
    }

    fn apply_discard(&mut self, player: PlayerId, cards: &[ObjectId], events: &mut Vec<Event>) {
        self.state.pending = None;
        for card in cards {
            let Some(mut object) = self.detach_object(*card) else {
                continue;
            };
            let card_id = object.card;
            object.tapped = false;
            object.damage = 0;
            object.summoning_sick = false;
            object.pump_power = 0;
            object.pump_toughness = 0;
            self.insert_object(object, Zone::Graveyard);
            events.push(Event::Discarded {
                player,
                card: card_id,
            });
        }
        self.finish_cleanup(events);
    }

    // ----------------------------------------------------------- scheduler

    fn grant_priority(&mut self, player: PlayerId, events: &mut Vec<Event>) {
        if self.pending_loss_exists() {
            self.state.turn.priority = None;
            return;
        }
        // CR 117.5/608.2: no player receives priority while a stack item is
        // still resolving. `advance_resolution` follows an item's instructions to
        // completion in one call, so this guard is a backstop: it can only fire
        // if a future change reintroduces a mid-resolution grant, and the
        // scheduler then resumes after the item finishes instead of emitting a
        // false `Priority` event (R170-1).
        if self.state.resolving.is_some() {
            self.state.turn.priority = None;
            return;
        }
        // CR 603.3: waiting triggered abilities are put on the stack before any
        // player actually receives priority.
        if self.state.pending.is_none() && self.put_triggers(events) && self.state.pending.is_some()
        {
            // A trigger still needs its controller's target choice.
            self.state.turn.priority = None;
            return;
        }
        // CR 800.4j: if the seat that would receive priority has left the game,
        // the next live seat in turn order receives it.
        let recipient = if self.state.is_live(player) {
            Some(player)
        } else {
            self.state.next_live_seat_after(player)
        };
        let Some(recipient) = recipient else {
            self.state.turn.priority = None;
            return;
        };
        self.state.turn.priority = Some(recipient);
        self.state.turn.passed = 0;
        events.push(Event::Priority { player: recipient });
    }

    /// Drive the game forward until a player must act, the game ends, or a
    /// pending choice blocks progress.
    fn settle(&mut self, events: &mut Vec<Event>) {
        for _ in 0..SETTLE_LIMIT {
            if self.state.is_over() {
                return;
            }
            // No SBA or priority runs for normal play while a choice is
            // outstanding (CR 608.2d). Externally injected sandbox departures
            // still need to purge/resume the queue; this path cannot occur
            // through the legal action interface because no effect changes life
            // during the Crack the Earth choice.
            if self.state.pending.is_some() {
                let injected_departure =
                    self.state.players.iter().enumerate().any(|(seat, player)| {
                        !player.lost
                            && (player.life <= 0
                                || player.drew_from_empty
                                || (self.state.profile.is_commander_combat()
                                    && self.state.commander_damage.iter().any(|entry| {
                                        entry.recipient as usize == seat && entry.amount >= 21
                                    })))
                    });
                if injected_departure {
                    self.check_sbas(events);
                    if self.state.is_over() || self.state.pending.is_some() {
                        return;
                    }
                    if self.state.resolving.is_some() {
                        // The departure applied surviving choices; finish the
                        // original resolution before any priority (CR 704.4).
                        continue;
                    }
                    self.grant_priority(self.state.turn.active, events);
                }
                return;
            }
            // While a spell is resolving, its instructions complete before SBAs
            // or any priority. SBAs run after resolution, before the next
            // player would actually receive priority (CR 608.2, 117.5).
            if self.state.resolving.is_some() {
                self.advance_resolution(events);
                if self.state.is_over() || self.state.pending.is_some() {
                    return;
                }
                // The spell has finished resolving: run SBAs before granting the
                // active player priority (CR 117.5, 704.4).
                self.check_sbas(events);
                if self.state.is_over() || self.state.pending.is_some() {
                    return;
                }
                let active = self.state.turn.active;
                self.grant_priority(active, events);
                continue;
            }
            self.check_sbas(events);
            if self.state.is_over() || self.state.pending.is_some() {
                return;
            }
            if self.state.turn.priority.is_none() {
                if self.state.turn.step.grants_priority() {
                    let active = self.state.turn.active;
                    self.grant_priority(active, events);
                } else {
                    self.advance_step(events);
                }
                continue;
            }
            return;
        }
        events.push(Event::Truncated {
            reason: "scheduler did not settle within its bounded iteration count".to_string(),
        });
    }

    fn advance_step(&mut self, events: &mut Vec<Event>) {
        // CR 500.5: unspent mana empties as the step or phase ends.
        self.empty_mana_pools(events);
        // CR 514.3a: when cleanup has granted priority, a completed pass round
        // repeats the same cleanup step through the normal step machinery so
        // StepBegin, mana emptying, cleanup actions and settle/SBA handling all
        // run again. The ordinary no-priority cleanup fast path still advances.
        let (next, new_turn) =
            if self.state.turn.step == Step::Cleanup && self.state.turn.priority.is_some() {
                (Step::Cleanup, false)
            } else {
                self.state.turn.step.next()
            };
        if new_turn {
            self.state.turn.number += 1;
            // CR 800.4k: a departed player's turn does not begin; select the
            // next actual live seat. Seats are never renumbered.
            match self.state.next_live_seat_after(self.state.turn.active) {
                Some(next_active) => self.state.turn.active = next_active,
                None => {
                    let reason = "no live seat remains to begin a turn (CR 104.4a)".to_string();
                    self.state.outcome = Some(Outcome::Draw {
                        reason: reason.clone(),
                    });
                    self.state.pending = None;
                    self.state.resolving = None;
                    self.state.turn.priority = None;
                    events.push(Event::GameDrawn { reason });
                    return;
                }
            }
            self.state.turn.lands_played = 0;
            self.state.turn.attackers.clear();
            self.state.turn.blocked.clear();
            self.state.turn.blockers.clear();
            self.state.turn.combat_damage_done = false;
            self.state.turn.defender = None;
            // CR 602.5b: "activate only once each turn" limits reset with the
            // new turn.
            self.state.activation_uses.clear();
            let (turn, active) = (self.state.turn.number, self.state.turn.active);
            events.push(Event::TurnBegin { turn, active });
        }
        self.state.turn.step = next;
        self.state.turn.priority = None;
        self.state.turn.passed = 0;
        events.push(Event::StepBegin { step: next });
        let active = self.state.turn.active;
        match next {
            Step::Untap => self.perform_untap(active),
            Step::Upkeep => self.grant_priority(active, events),
            Step::Draw => {
                // CR 103.8a: in a two-player game the player who plays first
                // skips the draw step of their first turn. CR 103.8c/800.7: in
                // other multiplayer games no player skips that draw.
                let first_turn_of_first_player = self.state.seat_count() == 2
                    && self.state.turn.number == 1
                    && active == self.state.first_player;
                let suspended = if !first_turn_of_first_player && self.state.is_live(active) {
                    self.draw(active, events) // CR 504.1
                } else {
                    false
                };
                if !suspended {
                    // CR 504.2: priority follows the completed draw. When a
                    // commander draw replacement is pending (903.9b), the
                    // answer handler grants it exactly once after the choice.
                    //
                    // CR 704.5b/117.5: an empty-library draw is a state-based
                    // loss. The settle loop must run SBAs (and settle GameOver)
                    // before any player receives priority. Granting priority
                    // here emitted the inherited D-1 false Priority event
                    // between the draw and the loss.
                    if !self.state.player(active).drew_from_empty {
                        self.grant_priority(active, events);
                    }
                }
            }
            Step::PrecombatMain | Step::PostcombatMain | Step::BeginCombat | Step::EndStep => {
                self.grant_priority(active, events);
            }
            Step::DeclareAttackers => {
                if self.legal_attackers(active).is_empty() {
                    self.grant_priority(active, events);
                } else {
                    self.state.pending = Some(Pending::DeclareAttackers { player: active });
                }
            }
            Step::DeclareBlockers => {
                // The defending seat was chosen in the attacker declaration.
                let defender = self.state.turn.defender;
                let defender_ready = defender.is_some_and(|d| {
                    self.state.is_live(d)
                        && !self.state.turn.attackers.is_empty()
                        && !self.legal_blockers(d).is_empty()
                });
                match (defender, defender_ready) {
                    (Some(defender), true) => {
                        self.state.pending = Some(Pending::DeclareBlockers { player: defender });
                    }
                    _ => self.grant_priority(active, events),
                }
            }
            Step::CombatDamage => {
                // CR 510.2/117.5/704.3: damage is dealt, then all applicable
                // SBAs (including CR 903.9a commander-return choices) complete
                // before any player receives priority. A pending choice grants
                // the single post-answer priority from the answer path.
                self.deal_combat_damage(events);
                self.check_sbas(events);
                if self.state.pending.is_some() || self.state.is_over() {
                    return;
                }
                self.grant_priority(active, events);
            }
            Step::EndCombat => {
                self.state.turn.attackers.clear();
                self.state.turn.blocked.clear();
                self.state.turn.blockers.clear();
                self.state.turn.defender = None;
                self.grant_priority(active, events);
            }
            Step::Cleanup => self.perform_cleanup(events),
        }
    }

    fn perform_untap(&mut self, active: PlayerId) {
        for obj in self.state.objects.values_mut() {
            if obj.zone == Zone::Battlefield && obj.controller == active {
                obj.tapped = false;
                if obj.is_creature() {
                    obj.summoning_sick = false;
                }
            }
        }
        // CR 502.4: no player receives priority during the untap step.
    }

    fn perform_cleanup(&mut self, events: &mut Vec<Event>) {
        let hand = self.state.player(self.state.turn.active).hand_size();
        if hand > MAX_HAND_SIZE {
            self.state.pending = Some(Pending::Discard {
                player: self.state.turn.active,
                count: (hand - MAX_HAND_SIZE) as u8,
            });
            return;
        }
        self.finish_cleanup(events);
    }

    fn finish_cleanup(&mut self, events: &mut Vec<Event>) {
        let mut changed = false;
        if self.state.prevent_all_combat_damage_turn.take().is_some() {
            changed = true;
        }
        // CR 514.2: until-end-of-turn effects and every prevention shield end.
        let expired_effects = self
            .state
            .effects
            .iter()
            .filter(|effect| effect.duration == EffectDuration::UntilEndOfTurn)
            .count();
        self.state
            .effects
            .retain(|effect| effect.duration != EffectDuration::UntilEndOfTurn);
        let expired_shields = self.state.shields.len();
        self.state.shields.clear();
        self.release_idle_effect_cursors();
        if expired_effects > 0 || expired_shields > 0 {
            changed = true;
        }
        for obj in self.state.objects.values_mut() {
            if obj.zone == Zone::Battlefield {
                if obj.damage > 0 {
                    changed = true;
                }
                obj.damage = 0;
            }
        }
        // The layer recomputation clears the expired 7c modifiers and re-applies
        // any characteristic-defining ability that outlives the turn.
        self.recompute_layers();
        if changed {
            events.push(Event::StateBasedAction {
                description:
                    "cleanup: all damage removed and until-end-of-turn effects ended (CR 514.2)"
                        .to_string(),
            });
        }
    }

    /// Apply CR 800.4a immediately: the player leaves the game, every object
    /// they own leaves with them into the internal `Zone::Removed`, their
    /// objects leave the stack, and combat references are pruned. The current
    /// turn is NOT ended (CR 800.4j); seats are never renumbered.
    fn leave_game(&mut self, p: PlayerId, events: &mut Vec<Event>) {
        let apnap_source = match &self.state.pending {
            Some(Pending::ApnapSacrifice { .. }) => self.state.resolving.and_then(|id| {
                let object = self.state.objects.get(&id)?;
                let cursor = self
                    .state
                    .stack
                    .iter()
                    .find(|item| item.object == id)
                    .map(|item| item.cursor)
                    .unwrap_or(1);
                Some((id, object.card, cursor, object.controller, object.owner))
            }),
            _ => None,
        };
        self.state.players[p as usize].lost = true;
        self.state.turn.passed &= !(1u8 << p);

        let owned: Vec<ObjectId> = self
            .state
            .objects
            .values()
            .filter(|o| o.owner == p)
            .map(|o| o.id)
            .collect();
        for id in &owned {
            if let Some(obj) = self.state.objects.get_mut(id) {
                obj.zone = Zone::Removed;
                obj.commander = None;
                obj.tapped = false;
                obj.damage = 0;
                obj.summoning_sick = false;
                obj.pump_power = 0;
                obj.pump_toughness = 0;
            }
        }
        let player = &mut self.state.players[p as usize];
        player.library.clear();
        player.hand.clear();
        player.graveyard.clear();
        player.exile.clear();
        player.command_zone.clear();

        // (a) Owned stack objects already left via the owned-object clause;
        // drop only their stack entries. A survivor-owned represented card is
        // NOT silently deleted here.
        self.state.stack.retain(|item| {
            self.state
                .objects
                .get(&item.object)
                .is_some_and(|object| object.zone != Zone::Removed)
        });
        // (b) Control-changing effects end: none are modeled (no card in this
        // pool grants control), so a modeled current-control fact is not
        // silently reverted and no effect-end history is claimed.
        // (c) Non-card stack objects controlled by the loser cease to exist:
        // every stack object here is represented by a card, so there are none.
        // (d) Any object still controlled by the departing player is exiled.
        let still_controlled: Vec<ObjectId> = self
            .state
            .objects
            .values()
            .filter(|o| o.owner != p && o.controller == p && o.zone != Zone::Removed)
            .map(|o| o.id)
            .collect();
        for id in still_controlled {
            self.move_to_exile(id, events);
        }
        let retired: Vec<crate::ids::CommanderId> = self
            .state
            .commanders
            .iter()
            .filter(|commander| commander.owner == p)
            .map(|commander| commander.id)
            .collect();
        self.state.retired_commanders.extend(retired);
        self.state.retired_commanders.sort_unstable();
        self.state.retired_commanders.dedup();
        self.state
            .commanders
            .retain(|commander| commander.owner != p);
        // Minimal ledger consistency (not the full 800.4a slice): the
        // departed recipient's rows are settled; historic rows for a retired
        // commander remain so later saves still validate.
        self.state.prune_commander_damage();
        let source_left_apnap = matches!(
            &self.state.pending,
            Some(Pending::ApnapSacrifice {
                source_left: Some(provenance),
                ..
            }) if self.state.resolving == Some(provenance.source)
        ) || (matches!(
            &self.state.pending,
            Some(Pending::ApnapSacrifice {
                source_left: None,
                ..
            })
        ) && self.state.resolving.is_some_and(|r| {
            self.state
                .objects
                .get(&r)
                .is_some_and(|object| object.zone == Zone::Removed)
        }));
        if !source_left_apnap
            && self.state.resolving.is_some_and(|r| {
                self.state
                    .objects
                    .get(&r)
                    .is_none_or(|object| object.zone == Zone::Removed)
                    || !self.state.stack.iter().any(|item| item.object == r)
            })
        {
            self.state.resolving = None;
        }

        let alive: Vec<ObjectId> = self
            .state
            .objects
            .values()
            .filter(|o| o.zone == Zone::Battlefield)
            .map(|o| o.id)
            .collect();
        self.state.turn.attackers.retain(|id| alive.contains(id));
        self.state
            .turn
            .blockers
            .retain(|(blocker, attacker)| alive.contains(blocker) && alive.contains(attacker));
        self.state
            .turn
            .blocked
            .retain(|id| self.state.turn.attackers.contains(id));

        if self.state.turn.priority == Some(p) {
            // CR 800.4a: priority passes to the next live player in turn order.
            self.state.turn.priority = self.state.next_live_seat_after(p);
        }
        match self.state.pending.take() {
            Some(Pending::CommanderReturn {
                order,
                cursor,
                answers,
            }) => {
                let mut kept = Vec::new();
                let mut kept_answers = Vec::new();
                let mut kept_cursor = 0;
                for (index, entry) in order.iter().enumerate() {
                    if entry.player == p {
                        continue;
                    }
                    if index < cursor {
                        kept_cursor += 1;
                        kept_answers.push(answers[index]);
                    }
                    kept.push(entry.clone());
                }
                if !kept.is_empty() {
                    if kept_cursor == kept.len() {
                        self.apply_commander_return_batch(&kept, &kept_answers, events);
                    } else {
                        self.state.pending = Some(Pending::CommanderReturn {
                            order: kept,
                            cursor: kept_cursor,
                            answers: kept_answers,
                        });
                    }
                }
            }
            Some(
                pending @ (Pending::CommanderReplacement { .. } | Pending::CommanderDraw { .. }),
            ) if pending.player() == p => {}
            Some(Pending::ApnapSacrifice {
                mut order,
                mut choices,
                source_left: existing_source_left,
                ..
            }) => {
                // CR 608.2/608.2h/704.4: an already-resolving each-player
                // instruction continues for surviving choosers even if its
                // represented source left via 800.4a. Serialize the exact
                // last-known-source provenance; a bare Removed/Exile object is
                // never accepted by validation.
                let provenance = existing_source_left.or_else(|| {
                    apnap_source.and_then(|(id, card, cursor, controller, owner)| {
                        let object = self.state.objects.get(&id)?;
                        let zone = object.zone;
                        if !matches!(zone, Zone::Removed | Zone::Exile) || controller != p {
                            return None;
                        }
                        Some(crate::state::ApnapSourceLeft {
                            source: id,
                            card,
                            cursor,
                            prior_controller: controller,
                            owner,
                            departed: p,
                            zone,
                        })
                    })
                });
                if order.contains(&p) || choices.contains_key(&p) {
                    order.retain(|seat| *seat != p);
                    choices.remove(&p);
                }
                if let Some(next) = order.first().copied() {
                    self.state.pending = Some(Pending::ApnapSacrifice {
                        player: next,
                        order,
                        choices,
                        source_left: provenance,
                    });
                } else {
                    // The final unanswered participant left. Apply the
                    // surviving public choices exactly once, then let the
                    // original resolution finish before any priority. A present
                    // source stays on the stack at its recorded cursor for
                    // settle() to complete; a departed source has no stack item
                    // and is finished here.
                    let source_on_stack = self.state.resolving.is_some_and(|resolving| {
                        self.state
                            .stack
                            .last()
                            .is_some_and(|item| item.object == resolving)
                    });
                    if source_on_stack || provenance.is_some() {
                        self.apply_apnap_choices(&choices, events);
                        self.state.pending = None;
                        if !source_on_stack {
                            self.state.resolving = None;
                        }
                    } else {
                        self.state.pending = None;
                        self.state.resolving = None;
                    }
                }
            }
            Some(pending) => {
                if pending.player() == p {
                    self.state.resolving = None;
                } else {
                    self.state.pending = Some(pending);
                }
            }
            None => {}
        }

        // The canonical resolver pointer is the serialized provenance source;
        // validation checks its card/zone/owner/controller/departure facts.
        if let Some(Pending::ApnapSacrifice {
            source_left: Some(provenance),
            ..
        }) = &self.state.pending
        {
            self.state.resolving = Some(provenance.source);
        }
        events.push(Event::StateBasedAction {
            description: format!(
                "P{p} leaves the game; all objects owned by P{p} leave the game (CR 800.4a)"
            ),
        });
    }

    /// True when a live seat already satisfies a player-loss SBA from the
    /// current state. Priority must not be granted before the next SBA check
    /// (CR 117.5); injected/sandbox positions can expose this directly.
    fn pending_loss_exists(&self) -> bool {
        self.state.players.iter().enumerate().any(|(seat, player)| {
            !player.lost
                && (player.life <= 0
                    || player.drew_from_empty
                    || (self.state.profile.is_commander_combat()
                        && self
                            .state
                            .commander_damage
                            .iter()
                            .any(|entry| entry.recipient as usize == seat && entry.amount >= 21)))
        })
    }

    fn check_sbas(&mut self, events: &mut Vec<Event>) {
        if self.state.is_over() {
            return;
        }
        loop {
            let mut changed = false;
            // CR 613: continuous effects are recomputed before state-based
            // actions are checked, so a characteristic-defining toughness of 0
            // (or a modifier that left this turn) is seen by the SBA pass.
            self.recompute_layers();
            // Reasons are captured before leave_game prunes the departed
            // recipient's ledger rows, so a commander-damage GameOver reason
            // survives the settlement.
            let mut loss_reasons: Vec<(PlayerId, String)> = Vec::new();
            for p in 0..self.state.seat_count() as PlayerId {
                if !self.state.is_live(p) {
                    continue;
                }
                let pl = self.state.player(p);
                let reason = if pl.drew_from_empty {
                    Some("attempted to draw from an empty library (CR 704.5b)".to_string())
                } else if pl.life <= 0 {
                    Some(format!("life total is {} (CR 704.5a)", pl.life))
                } else if self.state.profile.is_commander_combat() {
                    self.state
                        .commander_damage
                        .iter()
                        .filter(|entry| entry.recipient == p && entry.amount >= 21)
                        .max_by_key(|entry| entry.amount)
                        .map(|entry| {
                            format!(
                                "was dealt {} combat damage by {} (CR 903.10a)",
                                entry.amount, entry.commander
                            )
                        })
                } else {
                    None
                };
                if let Some(reason) = reason {
                    loss_reasons.push((p, reason));
                }
            }
            // CR 704.3: all applicable losses are one simultaneous SBA set from
            // the pre-SBA state. Record every reason, then depart in seat order.
            for (p, reason) in &loss_reasons {
                events.push(Event::StateBasedAction {
                    description: format!("P{p} loses: {reason}"),
                });
            }
            for (p, _) in &loss_reasons {
                self.leave_game(*p, events);
                changed = true;
            }

            // CR 704.5q: a permanent that holds both +1/+1 and -1/-1 counters
            // loses N of each, where N is the smaller count. Removing an equal
            // number of both kinds leaves power and toughness unchanged, so this
            // action neither causes nor prevents a CR 704.5f/704.5g death; it is
            // applied at this same checkpoint so no stored state keeps an
            // illegal pair. Iteration is by object id, so the trace is stable.
            let mut annihilated: Vec<(CardId, u32)> = Vec::new();
            for object in self.state.objects.values_mut() {
                if object.zone != Zone::Battlefield {
                    continue;
                }
                if let Some(removed) = object.counters.annihilate() {
                    annihilated.push((object.card, removed));
                }
            }
            if !annihilated.is_empty() {
                let affected: Vec<String> = annihilated
                    .iter()
                    .map(|(card, removed)| format!("{card} ({removed} +1/+1 and {removed} -1/-1)"))
                    .collect();
                events.push(Event::StateBasedAction {
                    description: format!(
                        "paired +1/+1 and -1/-1 counters were removed from {} (CR 704.5q)",
                        affected.join(", ")
                    ),
                });
                changed = true;
            }

            let dying: Vec<ObjectId> = self
                .state
                .objects
                .values()
                .filter(|o| o.zone == Zone::Battlefield && o.is_creature())
                .filter(|o| o.toughness() <= 0 || o.damage as i32 >= o.toughness())
                .map(|o| o.id)
                .collect();
            if !dying.is_empty() {
                // CR 704.3 + 603.6c: the whole batch is one event. Snapshot the
                // watchers before anything leaves the battlefield and dispatch
                // once for every death, so the result does not depend on which
                // dying creature happens to be moved first.
                let watchers = death_watchers(self);
                let mut deaths: Vec<(ObjectId, ObjectId, CardId, PlayerId)> = Vec::new();
                for id in &dying {
                    if let Some((dying_id, new_id, card, controller)) =
                        self.move_to_graveyard_inner(*id, events)
                    {
                        deaths.push((dying_id, new_id, card, controller));
                    }
                }
                self.queue_watchers_for_deaths(&watchers, &deaths);
                events.push(Event::StateBasedAction {
                    description: format!(
                        "{} creature(s) had lethal damage or toughness 0 and were put into the graveyard (CR 704.5f, 704.5g)",
                        dying.len()
                    ),
                });
                changed = true;
            }

            // CR 111.7: a token in a zone other than the battlefield ceases to
            // exist as a state-based action. It reached that zone first, so
            // dies/destination events and applicable triggers were recorded
            // above; CR 111.8 then prevents another zone change.
            let tokens: Vec<ObjectId> = self
                .state
                .objects
                .values()
                .filter(|object| {
                    object.card.is_token()
                        && object.zone != Zone::Battlefield
                        && object.zone != Zone::Removed
                })
                .map(|object| object.id)
                .collect();
            if !tokens.is_empty() {
                for id in tokens {
                    if let Some(object) = self.detach_object(id) {
                        let card = object.card;
                        events.push(Event::StateBasedAction {
                            description: format!("{card} token ceases to exist (CR 111.7)"),
                        });
                    }
                }
                changed = true;
            }

            // Terminal outcomes are resolved in the same SBA event before any
            // commander-return choice: once one live seat (or none) remains,
            // an irrelevant return choice must not defer GameOver.
            match self.state.live_count() {
                0 => {
                    let reason =
                        "all remaining players lost simultaneously (CR 104.4a)".to_string();
                    self.state.outcome = Some(Outcome::Draw {
                        reason: reason.clone(),
                    });
                    self.state.pending = None;
                    self.state.resolving = None;
                    self.state.turn.priority = None;
                    events.push(Event::GameDrawn { reason });
                    return;
                }
                1 => {
                    let winner = self
                        .state
                        .live_seats()
                        .next()
                        .expect("one live seat remains");
                    let loser_reason = if loss_reasons
                        .iter()
                        .any(|(_, reason)| reason.contains("903.10a"))
                    {
                        "opponent was dealt 21+ combat damage by one commander (CR 903.10a)"
                            .to_string()
                    } else if loss_reasons
                        .iter()
                        .any(|(_, reason)| reason.contains("704.5b"))
                    {
                        "opponent lost by attempting to draw from an empty library (CR 104.3c, 704.5b)".to_string()
                    } else if loss_reasons
                        .iter()
                        .any(|(_, reason)| reason.contains("704.5a"))
                    {
                        "opponents' life totals reached 0 or less (CR 704.5a)".to_string()
                    } else {
                        "all other players have left the game".to_string()
                    };
                    self.state.outcome = Some(Outcome::Win {
                        winner,
                        reason: loser_reason.clone(),
                    });
                    self.state.pending = None;
                    self.state.resolving = None;
                    self.state.turn.priority = None;
                    events.push(Event::GameOver {
                        winner,
                        reason: loser_reason,
                    });
                    return;
                }
                _ => {}
            }

            // CR 903.9a: after all other simultaneous SBAs, collect the exact
            // armed commander moves in APNAP order. No priority is granted
            // until the whole SBA event (including these choices) finishes.
            if self.state.pending.is_none() {
                let anchor = self.state.turn.active;
                let n = self.state.seat_count();
                let mut entries: Vec<crate::state::CommanderReturnEntry> = self
                    .state
                    .commanders
                    .iter()
                    .filter(|commander| commander.return_eligible)
                    .filter_map(|commander| {
                        let object = self.state.try_object(commander.object)?;
                        if matches!(object.zone, Zone::Graveyard | Zone::Exile) {
                            Some(crate::state::CommanderReturnEntry {
                                player: commander.owner,
                                commander: commander.id,
                                object: commander.object,
                                zone: object.zone,
                            })
                        } else {
                            None
                        }
                    })
                    .collect();
                if !entries.is_empty() {
                    entries.sort_by_key(|entry| (entry.player as usize + n - anchor as usize) % n);
                    self.state.pending = Some(Pending::CommanderReturn {
                        order: entries,
                        cursor: 0,
                        answers: Vec::new(),
                    });
                    self.state.turn.priority = None;
                    self.state.resolving = None;
                    return;
                }
            }
            // Restored original scheduler termination/outcome semantics: an
            // outstanding legacy pending choice (for example an injected APNAP
            // departure) must not keep this loop spinning, and it must not
            // prevent loss/outcome handling.
            if !changed {
                break;
            }
            // CR 506.4 + 704.5: anything that just left the battlefield has
            // left combat.
            self.prune_combat();

            match self.state.live_count() {
                0 => {
                    let reason =
                        "all remaining players lost simultaneously (CR 104.4a)".to_string();
                    self.state.outcome = Some(Outcome::Draw {
                        reason: reason.clone(),
                    });
                    self.state.pending = None;
                    self.state.resolving = None;
                    self.state.turn.priority = None;
                    events.push(Event::GameDrawn { reason });
                    return;
                }
                1 => {
                    let winner = self
                        .state
                        .live_seats()
                        .next()
                        .expect("one live seat remains");
                    let loser_reason = self
                        .state
                        .players
                        .iter()
                        .rfind(|player| player.lost)
                        .map(|player| {
                            if player.drew_from_empty {
                                "opponent lost by attempting to draw from an empty library (CR 104.3c, 704.5b)"
                                    .to_string()
                            } else {
                                "opponent's life total reached 0 or less (CR 704.5a)"
                                    .to_string()
                            }
                        })
                        .unwrap_or_else(|| "all other players have left the game".to_string());
                    self.state.outcome = Some(Outcome::Win {
                        winner,
                        reason: loser_reason.clone(),
                    });
                    self.state.pending = None;
                    self.state.resolving = None;
                    self.state.turn.priority = None;
                    events.push(Event::GameOver {
                        winner,
                        reason: loser_reason,
                    });
                    return;
                }
                _ => {}
            }
        }
    }

    // ------------------------------------------------------------ resolution

    fn targets_legal(&self, item: &StackItem) -> bool {
        for target in &item.targets {
            let ok = match target {
                Target::Player(p) => self.state.is_live(*p),
                Target::Permanent(id) => self
                    .state
                    .try_object(*id)
                    .is_some_and(|o| o.zone == Zone::Battlefield && o.is_creature()),
                Target::Spell(id) => self
                    .state
                    .stack
                    .iter()
                    .any(|s| s.object == *id && s.entry == StackEntry::Spell),
            };
            if !ok {
                return false;
            }
        }
        true
    }

    fn advance_resolution(&mut self, events: &mut Vec<Event>) {
        loop {
            let Some(item) = self.state.stack.last().cloned() else {
                self.state.resolving = None;
                return;
            };
            if item.entry != StackEntry::Spell {
                // CR 608.2: an ability's instructions all finish before the item
                // leaves the stack, and no player receives priority while it is
                // resolving (CR 117.5). Follow every instruction in this call
                // rather than returning to the scheduler between effects, which
                // would grant priority before the item's `AbilityResolved` event
                // and let a trigger caused by the effect be staged too early
                // (R170-1). A pending choice still suspends resolution
                // (CR 608.2d).
                let mut current = item;
                loop {
                    self.advance_ability_resolution(current, events);
                    if self.state.pending.is_some() || self.state.resolving.is_none() {
                        return;
                    }
                    let Some(next) = self.state.stack.last().cloned() else {
                        self.state.resolving = None;
                        return;
                    };
                    current = next;
                }
            }
            let card = self.state.object(item.object).card;
            let def = card.def();
            let Some(spec) = def.spell_spec() else {
                self.move_to_graveyard(item.object, events);
                self.state.resolving = None;
                return;
            };

            // CR 608.2b: legality of targets is checked once, before
            // instructions are followed.
            if item.cursor == 0 && !self.targets_legal(&item) {
                events.push(Event::NoLegalTargets {
                    card,
                    reason: "all of its targets are illegal, so it does not resolve (CR 608.2b)"
                        .to_string(),
                });
                self.move_to_graveyard(item.object, events);
                self.state.resolving = None;
                self.finish_resolution(events);
                return;
            }

            if item.cursor as usize >= spec.effects.len() {
                events.push(Event::Resolve {
                    controller: item.controller,
                    object: item.object,
                    card,
                });
                if def.is_permanent_spell() {
                    // CR 608.3: a resolving permanent spell enters the battlefield.
                    self.enter_battlefield(item.object, item.controller, events);
                } else {
                    self.move_to_graveyard(item.object, events);
                }
                self.state.resolving = None;
                self.finish_resolution(events);
                return;
            }

            let effect = spec.effects[item.cursor as usize];
            let pending_created = self.apply_effect(effect, &item, events);
            match self.state.stack.last_mut() {
                Some(top) => top.cursor += 1,
                None => return,
            }
            if pending_created {
                // The spell stays on the stack while the choice is made; no
                // player has priority (CR 608.2d).
                self.state.resolving = Some(item.object);
                self.state.turn.priority = None;
                return;
            }
        }
    }

    fn finish_resolution(&mut self, _events: &mut Vec<Event>) {
        // Priority is granted by settle() only after resolution completes and
        // SBAs have run (CR 117.5, 704.4). No Priority event is emitted here.
        self.state.turn.priority = None;
    }

    /// Resolve one activated or triggered ability (CR 608.2, 603.5, 608.2b).
    ///
    /// An ability is not a card: finishing it never puts the source into a
    /// graveyard. The source may already have left the battlefield (a "dies"
    /// trigger, or a cost that sacrificed it), so every characteristic used
    /// here comes from the stack item's recorded card and targets.
    fn advance_ability_resolution(&mut self, item: StackItem, events: &mut Vec<Event>) {
        let Some(found) = ability::abilities(item.card).get(ability_index(item.entry) as usize)
        else {
            self.pop_ability(item.object, item.entry);
            self.state.resolving = None;
            self.finish_resolution(events);
            return;
        };
        let ability = *found;
        let optional = matches!(item.entry, StackEntry::Triggered { .. })
            && ability
                .triggered()
                .is_some_and(|triggered| triggered.optional);

        // CR 603.5: the controller decides when the optional ability resolves.
        if optional && !item.trigger_answered {
            self.state.pending = Some(Pending::TriggerChoice {
                player: item.controller,
                trigger: TriggerRef {
                    source: item.object,
                    card: item.card,
                    controller: item.controller,
                    ability: ability_index(item.entry),
                    seq: 0,
                },
            });
            self.state.resolving = Some(item.object);
            self.state.turn.priority = None;
            return;
        }

        // CR 608.2b: legality is checked once, before instructions are followed.
        if item.cursor == 0 && !self.targets_legal(&item) {
            events.push(Event::AbilityFizzled {
                controller: item.controller,
                card: item.card,
                reason: "all of its targets are illegal, so it does not resolve (CR 608.2b)"
                    .to_string(),
            });
            self.pop_ability(item.object, item.entry);
            self.state.resolving = None;
            self.finish_resolution(events);
            return;
        }

        let effects = ability.effects();
        if item.cursor as usize >= effects.len() {
            events.push(Event::AbilityResolved {
                controller: item.controller,
                source: item.object,
                card: item.card,
                text: ability.text().to_string(),
            });
            self.pop_ability(item.object, item.entry);
            self.state.resolving = None;
            self.finish_resolution(events);
            return;
        }

        let effect = effects[item.cursor as usize];
        let pending_created = self.apply_effect(effect, &item, events);
        if let Some(top) = self.state.stack.last_mut()
            && top.object == item.object
            && top.entry == item.entry
        {
            top.cursor += 1;
        }
        if pending_created {
            self.state.resolving = Some(item.object);
            self.state.turn.priority = None;
        }
    }

    /// Returns true when the effect created a pending decision.
    fn controlled_permanents(&self, player: PlayerId) -> Vec<ObjectId> {
        self.state
            .objects
            .values()
            .filter(|object| object.zone == Zone::Battlefield && object.controller == player)
            .map(|object| object.id)
            .collect()
    }

    /// Circular APNAP order anchored at `anchor`, restricted to `eligible`.
    fn apnap_order_from(&self, anchor: PlayerId, eligible: &[PlayerId]) -> Vec<PlayerId> {
        let mut order = Vec::with_capacity(eligible.len());
        for offset in 0..self.state.seat_count() {
            let seat = ((anchor as usize + offset) % self.state.seat_count()) as PlayerId;
            if eligible.contains(&seat) {
                order.push(seat);
            }
        }
        order
    }

    fn begin_apnap_sacrifice(&mut self, _events: &mut Vec<Event>) {
        let anchor = self.state.turn.active;
        let eligible: Vec<PlayerId> = self.state.live_seats().collect();
        if eligible.is_empty() {
            return;
        }
        let order = self.apnap_order_from(anchor, &eligible);
        if let Some(player) = order.first().copied() {
            self.state.pending = Some(Pending::ApnapSacrifice {
                player,
                order,
                choices: BTreeMap::new(),
                source_left: None,
            });
        }
    }

    fn apply_commander_return(
        &mut self,
        player: PlayerId,
        to_command_zone: bool,
        events: &mut Vec<Event>,
    ) {
        let Some(Pending::CommanderReturn {
            order,
            cursor,
            mut answers,
        }) = self.state.pending.take()
        else {
            return;
        };
        if cursor >= order.len() || order[cursor].player != player {
            return;
        }
        answers.push(to_command_zone);
        let next = cursor + 1;
        if next < order.len() {
            // Collect every owner's answer before applying any move.
            self.state.pending = Some(Pending::CommanderReturn {
                order,
                cursor: next,
                answers,
            });
            return;
        }
        self.apply_commander_return_batch(&order, &answers, events);
        // An SBA choice is not a pass. Finish all SBAs before restoring the
        // active player's priority in the SAME step (CR 117.5, 704.3).
        self.check_sbas(events);
        if self.state.pending.is_none() && !self.state.is_over() {
            self.grant_priority(self.state.turn.active, events);
        }
    }

    fn apply_commander_return_batch(
        &mut self,
        order: &[crate::state::CommanderReturnEntry],
        answers: &[bool],
        events: &mut Vec<Event>,
    ) {
        // Final answer received: apply the whole SBA batch together in APNAP
        // order. No priority is granted between these moves.
        for (entry, accept) in order.iter().zip(answers.iter()) {
            if *accept {
                if let Some(mut object) = self.detach_object(entry.object) {
                    object.tapped = false;
                    object.damage = 0;
                    object.summoning_sick = false;
                    object.pump_power = 0;
                    object.pump_toughness = 0;
                    let new_id = self.insert_object(object, Zone::Command);
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "P{} puts commander {} into the command zone from the {} (CR 903.9a)",
                            entry.player,
                            new_id,
                            entry.zone.name()
                        ),
                    });
                }
            } else {
                if let Some(registry) = self
                    .state
                    .commanders
                    .iter_mut()
                    .find(|registry| registry.id == entry.commander)
                {
                    registry.return_eligible = false;
                }
                events.push(Event::StateBasedAction {
                    description: format!(
                        "P{} declines to return commander {} from the {} (CR 903.9a)",
                        entry.player,
                        entry.object,
                        entry.zone.name()
                    ),
                });
            }
        }
    }

    fn apply_apnap_sacrifice(
        &mut self,
        player: PlayerId,
        permanent: Option<ObjectId>,
        events: &mut Vec<Event>,
    ) {
        let Some(Pending::ApnapSacrifice {
            mut order,
            mut choices,
            source_left,
            ..
        }) = self.state.pending.take()
        else {
            return;
        };
        events.push(Event::SacrificeChosen { player, permanent });
        if order.first() == Some(&player) {
            order.remove(0);
        }
        choices.insert(player, permanent);
        match order.first().copied() {
            Some(next) => {
                self.state.pending = Some(Pending::ApnapSacrifice {
                    player: next,
                    order,
                    choices,
                    source_left,
                });
            }
            None => self.apply_apnap_choices(&choices, events),
        }
    }

    /// Apply all collected choices as one deterministic batch. No SBA or
    /// priority runs between individual sacrifices; resolution continues after
    /// the batch through the normal scheduler.
    fn apply_apnap_choices(
        &mut self,
        choices: &BTreeMap<PlayerId, Option<ObjectId>>,
        events: &mut Vec<Event>,
    ) {
        let anchor = self.state.turn.active;
        let eligible: Vec<PlayerId> = choices.keys().copied().collect();
        for seat in self.apnap_order_from(anchor, &eligible) {
            let Some(Some(id)) = choices.get(&seat) else {
                continue;
            };
            let valid = self.state.try_object(*id).is_some_and(|object| {
                object.zone == Zone::Battlefield && object.controller == seat
            });
            if valid {
                self.move_to_graveyard(*id, events);
            }
        }
        if self.state.resolving.is_some_and(|resolving| {
            self.state
                .objects
                .get(&resolving)
                .is_none_or(|object| object.zone == Zone::Removed)
        }) {
            self.state.resolving = None;
        }
    }

    fn apply_effect(&mut self, effect: Effect, item: &StackItem, events: &mut Vec<Event>) -> bool {
        let controller = item.controller;
        match effect {
            Effect::Draw(n) => {
                for index in 0..n {
                    let remaining = n - index - 1;
                    if self.draw_with_resume(
                        controller,
                        DrawResume::Effect {
                            source: item.object,
                            cursor: item.cursor + 1,
                        },
                        remaining,
                        events,
                    ) {
                        return true;
                    }
                }
                false
            }
            Effect::Scry(n) => {
                self.begin_order(controller, n, OrderMode::Scry, events);
                true
            }
            Effect::Surveil(n) => {
                self.begin_order(controller, n, OrderMode::Surveil, events);
                true
            }
            Effect::Damage(n) => {
                for target in item.targets.clone() {
                    match target {
                        Target::Player(p) => {
                            let dealt = self.deal_damage(
                                Some(item.object),
                                DamageTarget::Player(p),
                                n as u32,
                                false,
                                events,
                            );
                            if dealt == 0 {
                                continue;
                            }
                            let life = self.state.players[p as usize].life;
                            events.push(Event::Damage {
                                source: Some(item.object),
                                target: format!("P{p}"),
                                amount: dealt,
                            });
                            events.push(Event::LifeChange {
                                player: p,
                                delta: -(dealt as i32),
                                life,
                            });
                        }
                        Target::Permanent(id) => {
                            let dealt = self.deal_damage(
                                Some(item.object),
                                DamageTarget::Permanent(id),
                                n as u32,
                                false,
                                events,
                            );
                            if dealt == 0 {
                                continue;
                            }
                            events.push(Event::Damage {
                                source: Some(item.object),
                                target: self.describe_object(id),
                                amount: dealt,
                            });
                        }
                        Target::Spell(_) => {}
                    }
                }
                false
            }
            Effect::Pump(power, toughness) => {
                for target in item.targets.clone() {
                    if let Target::Permanent(id) = target {
                        self.add_modifier_effect(item, id, power, toughness, events);
                    }
                }
                self.recompute_layers();
                false
            }
            Effect::PreventNextDamage(amount) => {
                // CR 615.1: the prevention effect's controller creates a shield
                // on the spell's target. One shield per application; two
                // Mending Hands stack as two independent shields.
                //
                // "Any target" never includes a spell (CR 115.4), so
                // `legal_targets(AnyTarget)` offers only players and creatures
                // and `validate_cast` refuses a spell with `BadTarget`. The
                // spell arm below is therefore unreachable for a legal action;
                // it produces no shield rather than a silent no-op shield.
                for target in item.targets.clone() {
                    let shield_target = match target {
                        Target::Player(p) => ShieldTarget::Player(p),
                        Target::Permanent(id) => ShieldTarget::Permanent(id),
                        Target::Spell(_) => continue,
                    };
                    let card = self.state.object(item.object).card;
                    let label = shield_target.label(&self.state);
                    let timestamp = self.next_timestamp();
                    let id = self.next_effect_id();
                    self.state.shields.push(DamageShield {
                        id,
                        timestamp,
                        source: Some(item.object),
                        card,
                        controller,
                        scope: ShieldScope::AnyDamage,
                        target: shield_target,
                        remaining: Some(amount),
                        created_turn: self.state.turn.number,
                    });
                    events.push(Event::DamageShieldCreated {
                        player: controller,
                        card,
                        target: label,
                        amount: Some(amount),
                        combat_only: false,
                    });
                }
                false
            }
            Effect::EachPlayerSacrifice => {
                self.begin_apnap_sacrifice(events);
                self.state.pending.is_some()
            }
            Effect::ReturnToHand => {
                for target in &item.targets {
                    if let Target::Permanent(id) = target {
                        if self.begin_commander_replacement(*id, Zone::Hand, item) {
                            return true;
                        }
                        self.move_to_hand(*id, events);
                    }
                }
                false
            }
            Effect::PutOnTopOfLibrary => {
                for target in &item.targets {
                    if let Target::Permanent(id) = target {
                        if self.begin_commander_replacement(*id, Zone::Library, item) {
                            return true;
                        }
                        self.move_to_top_of_library(*id, events);
                    }
                }
                false
            }
            Effect::ExileAndGainLife => {
                for target in &item.targets {
                    if let Target::Permanent(id) = target {
                        let (controller, power, card, token) = match self.state.try_object(*id) {
                            Some(object) => (
                                object.controller,
                                object.power(),
                                object.card,
                                object.card.is_token(),
                            ),
                            None => continue,
                        };
                        let exiled = self.move_to_exile(*id, events);
                        self.state.players[controller as usize].life += power;
                        let life = self.state.players[controller as usize].life;
                        events.push(Event::LifeChange {
                            player: controller,
                            delta: power,
                            life,
                        });
                        // The token really enters exile; CR 111.7 removes it at
                        // the next SBA check, not as an exile replacement. Life
                        // gain uses last-known power (CR 608.2h).
                        let fate = if token {
                            "is exiled (token cessation awaits the next SBA check, CR 111.7)"
                        } else {
                            "is exiled"
                        };
                        events.push(Event::StateBasedAction {
                            description: format!(
                                "{card} {fate}; P{controller} gains {power} life (now {life})"
                            ),
                        });
                        let _ = exiled;
                    }
                }
                false
            }
            Effect::PreventAllCombatDamageThisTurn => {
                self.state.prevent_all_combat_damage_turn = Some(self.state.turn.number);
                let card = self.state.object(item.object).card;
                events.push(Event::PreventAllCombatDamage {
                    turn: self.state.turn.number,
                    card,
                });
                false
            }
            Effect::GainLife(amount) => {
                self.state.players[controller as usize].life += amount;
                let life = self.state.players[controller as usize].life;
                events.push(Event::LifeChange {
                    player: controller,
                    delta: amount,
                    life,
                });
                false
            }
            Effect::PumpSource(power, toughness) => {
                // A pump with no target applies to the ability's own source
                // (CR 611.2c). The source must still be on the battlefield.
                if self
                    .state
                    .try_object(item.object)
                    .is_some_and(|object| object.zone == Zone::Battlefield)
                {
                    self.add_modifier_effect(item, item.object, power, toughness, events);
                }
                self.recompute_layers();
                false
            }
            Effect::CreateGoblinTokens(count) => {
                self.create_goblin_tokens(controller, usize::from(count), events);
                false
            }
            Effect::CreateGoblinTokensForEachGoblin => {
                let count = self.goblin_count(controller);
                self.create_goblin_tokens(controller, count, events);
                false
            }
            Effect::CreateDrakeToken => {
                self.create_tokens(controller, CardId::DrakeToken, 1, events);
                false
            }
            Effect::MillSelf(n) => {
                self.mill_cards(controller, n, events);
                false
            }
            Effect::MillEachOpponent(n) => {
                for opponent in 0..self.state.seat_count() as PlayerId {
                    if opponent == controller || !self.state.is_live(opponent) {
                        continue;
                    }
                    self.mill_cards(opponent, n, events);
                }
                false
            }
            Effect::MillTargetPlayer(n) => {
                // CR 701.13a: "target player mills N cards" is the targeted
                // player's library, never the effect controller's.
                for target in item.targets.clone() {
                    if let Target::Player(p) = target {
                        self.mill_cards(p, n, events);
                    }
                }
                false
            }
            Effect::LoseLife(amount) => {
                // CR 118.2 + 119.3: losing life is not damage. No `Damage`
                // event, no damage replacement or prevention, no damage
                // trigger.
                for target in item.targets.clone() {
                    if let Target::Player(p) = target {
                        self.lose_life(p, amount, events);
                    }
                }
                false
            }
            Effect::LoseLifeEachOpponent(amount) => {
                for opponent in 0..self.state.seat_count() as PlayerId {
                    if opponent == controller || !self.state.is_live(opponent) {
                        continue;
                    }
                    self.lose_life(opponent, amount, events);
                }
                false
            }
            Effect::AddMana { color, amount } => {
                self.state.players[controller as usize]
                    .mana
                    .add(color, amount);
                let pool_total = self.state.players[controller as usize].mana.get(color);
                events.push(Event::ManaAdded {
                    player: controller,
                    color,
                    amount,
                    pool_total,
                });
                false
            }
            Effect::DamageEachOpponent(n) => {
                for opponent in 0..self.state.seat_count() as PlayerId {
                    if opponent == controller || !self.state.is_live(opponent) {
                        continue;
                    }
                    let dealt = self.deal_damage_from(
                        Some(item.object),
                        Some(controller),
                        DamageTarget::Player(opponent),
                        u32::from(n),
                        false,
                        events,
                    );
                    if dealt == 0 {
                        continue;
                    }
                    let life = self.state.players[opponent as usize].life;
                    events.push(Event::Damage {
                        source: Some(item.object),
                        target: format!("P{opponent}"),
                        amount: dealt,
                    });
                    events.push(Event::LifeChange {
                        player: opponent,
                        delta: -(dealt as i32),
                        life,
                    });
                }
                false
            }
            Effect::DamageTargetCreatureForEachGoblin => {
                let amount = self.goblin_count(controller) as u32;
                for target in item.targets.clone() {
                    if let Target::Permanent(id) = target {
                        let dealt = self.deal_damage_from(
                            Some(item.object),
                            Some(controller),
                            DamageTarget::Permanent(id),
                            amount,
                            false,
                            events,
                        );
                        if dealt > 0 {
                            events.push(Event::Damage {
                                source: Some(item.object),
                                target: self.describe_object(id),
                                amount: dealt,
                            });
                        }
                    }
                }
                false
            }
            Effect::DamageTargetPlayerForEachGoblin => {
                let amount = self.goblin_count(controller) as u32;
                for target in item.targets.clone() {
                    if let Target::Player(p) = target {
                        let dealt = self.deal_damage_from(
                            Some(item.object),
                            Some(controller),
                            DamageTarget::Player(p),
                            amount,
                            false,
                            events,
                        );
                        if dealt > 0 {
                            let life = self.state.players[p as usize].life;
                            events.push(Event::Damage {
                                source: Some(item.object),
                                target: format!("P{p}"),
                                amount: dealt,
                            });
                            events.push(Event::LifeChange {
                                player: p,
                                delta: -(dealt as i32),
                                life,
                            });
                        }
                    }
                }
                false
            }
            Effect::UntapAllNonlandsYouControl => {
                let ids: Vec<ObjectId> = self
                    .state
                    .objects
                    .values()
                    .filter(|o| {
                        o.zone == Zone::Battlefield
                            && o.controller == controller
                            && !o.card.def().is_land()
                    })
                    .map(|o| o.id)
                    .collect();
                for id in &ids {
                    if let Some(object) = self.state.objects.get_mut(id) {
                        object.tapped = false;
                    }
                }
                if !ids.is_empty() {
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "P{controller} untaps {} nonland permanent(s)",
                            ids.len()
                        ),
                    });
                }
                false
            }
            Effect::CounterTargetSpell => {
                let victim = item.targets.iter().find_map(|t| match t {
                    Target::Spell(id) => Some(*id),
                    _ => None,
                });
                let Some(victim) = victim else { return false };
                if let Some(index) = self.state.stack.iter().position(|s| s.object == victim) {
                    // CR 701.6a: a countered spell is removed from the stack and
                    // put into its owner's graveyard.
                    let removed = self.state.stack.remove(index);
                    let card = self.state.object(removed.object).card;
                    let source = self.state.object(item.object).card;
                    self.move_to_graveyard(removed.object, events);
                    events.push(Event::Countered {
                        object: removed.object,
                        card,
                        by: source,
                    });
                }
                false
            }
        }
    }

    fn goblin_count(&self, controller: PlayerId) -> usize {
        self.state
            .objects
            .values()
            .filter(|object| {
                object.zone == Zone::Battlefield
                    && object.controller == controller
                    && object.card.def().is_goblin()
            })
            .count()
    }

    /// Create real token objects with unique object ids. Token cards are not in
    /// the importable catalog; each token enters the battlefield under the
    /// effect's controller with summoning sickness under CR 302.6.
    fn create_goblin_tokens(
        &mut self,
        controller: PlayerId,
        count: usize,
        events: &mut Vec<Event>,
    ) {
        self.create_tokens(controller, CardId::GoblinToken, count, events);
    }

    fn create_tokens(
        &mut self,
        controller: PlayerId,
        card: CardId,
        count: usize,
        events: &mut Vec<Event>,
    ) {
        for _ in 0..count {
            let id = ObjectId(self.state.next_object);
            self.state.next_object += 1;
            let mut object = blank_object(id, card, controller);
            object.controller = controller;
            object.zone = Zone::Battlefield;
            object.summoning_sick = true;
            self.state.objects.insert(id, object);
            events.push(Event::PermanentEntered {
                object: id,
                card,
                controller,
            });
            events.push(Event::StateBasedAction {
                description: format!("P{controller} creates {card} token ({id})"),
            });
            self.queue_triggers(id, card, controller, TriggerEvent::EntersBattlefield);
            // CR 603.6a: a creature token entering is a creature entering for
            // every "another creature enters" trigger. Sharing the permanent
            // path here is what keeps Soul Warden visible to Goblin/Drake
            // tokens instead of only to represented cards.
            if card.def().is_creature() {
                self.queue_another_creature_enters(id);
            }
        }
    }

    /// Queue every "another creature enters" trigger on another battlefield
    /// permanent (CR 603.6a). The entering permanent is excluded because the
    /// printed word is "another", and the trigger's controller is the source
    /// permanent's controller, never the entering creature's controller.
    /// Shared by `enter_battlefield` and `create_tokens` so a creature token is
    /// not invisible to these triggers.
    fn queue_another_creature_enters(&mut self, entering: ObjectId) {
        let watchers: Vec<(ObjectId, CardId, PlayerId)> = self
            .state
            .objects
            .values()
            .filter(|object| object.zone == Zone::Battlefield && object.id != entering)
            .filter(|object| {
                ability::triggers_on(object.card, TriggerEvent::AnotherCreatureEnters)
                    .next()
                    .is_some()
            })
            .map(|object| (object.id, object.card, object.controller))
            .collect();
        for (source, source_card, source_controller) in watchers {
            self.queue_triggers(
                source,
                source_card,
                source_controller,
                TriggerEvent::AnotherCreatureEnters,
            );
        }
    }

    fn begin_order(
        &mut self,
        player: PlayerId,
        count: u8,
        mode: OrderMode,
        events: &mut Vec<Event>,
    ) {
        let available = self.state.player(player).library.len();
        let take = (count as usize).min(available);
        if take == 0 {
            return;
        }
        let viewed: Vec<ObjectId> = self.state.players[player as usize]
            .library
            .drain(0..take)
            .collect();
        match mode {
            OrderMode::Scry => events.push(Event::Scried {
                player,
                count: take as u8,
            }),
            OrderMode::Surveil => events.push(Event::Surveilled {
                player,
                count: take as u8,
            }),
        }
        self.state.pending = Some(Pending::OrderCard {
            player,
            viewed,
            index: 0,
            to_top: Vec::new(),
            mode,
        });
    }

    // ---------------------------------------------------------------- combat

    /// CR 506.4: a permanent that leaves the battlefield is removed from combat.
    fn prune_combat(&mut self) {
        let alive = |state: &GameState, id: ObjectId| {
            state
                .try_object(id)
                .is_some_and(|o| o.zone == Zone::Battlefield && o.is_creature())
        };
        let attackers: Vec<ObjectId> = self
            .state
            .turn
            .attackers
            .iter()
            .copied()
            .filter(|id| alive(&self.state, *id))
            .collect();
        let blocked: Vec<ObjectId> = self
            .state
            .turn
            .blocked
            .iter()
            .copied()
            .filter(|id| attackers.contains(id))
            .collect();
        let blockers: Vec<(ObjectId, ObjectId)> = self
            .state
            .turn
            .blockers
            .iter()
            .copied()
            .filter(|(blocker, attacker)| {
                alive(&self.state, *blocker) && attackers.contains(attacker)
            })
            .collect();
        self.state.turn.attackers = attackers;
        self.state.turn.blocked = blocked;
        self.state.turn.blockers = blockers;
    }

    fn deal_combat_damage(&mut self, events: &mut Vec<Event>) {
        if self.state.turn.combat_damage_done {
            return;
        }
        // Nothing that has already left the battlefield can deal or receive
        // combat damage.
        self.prune_combat();
        self.state.turn.combat_damage_done = true;
        let attackers = self.state.turn.attackers.clone();
        let blockers = self.state.turn.blockers.clone();
        let defender = self.state.turn.defender;
        let mut damage: Vec<(ObjectId, Option<ObjectId>, u32)> = Vec::new();

        for attacker in &attackers {
            let Some(atk) = self.state.try_object(*attacker) else {
                continue;
            };
            let power = atk.power();
            if power <= 0 {
                continue;
            }
            let its_blockers: Vec<ObjectId> = blockers
                .iter()
                .filter(|(_, a)| a == attacker)
                .map(|(b, _)| *b)
                .collect();
            let is_blocked = self.state.turn.blocked.contains(attacker);
            if its_blockers.is_empty() {
                // CR 509.1h: a blocked creature stays blocked even when every
                // blocker has left, so it assigns no damage to the player.
                if !is_blocked {
                    damage.push((*attacker, None, power as u32));
                }
            } else {
                // CR 510.1c: a blocked creature's controller divides its damage
                // among the blockers as they choose. This slice is narrower: it
                // assigns automatically in lethal-first order and exposes no
                // damage-assignment choice (documented in the coach and README).
                // CR 702.19b: a trampling attacker must assign lethal damage to
                // every blocker before it may assign any to the player, so with
                // trample the surplus after lethal assignment goes to the
                // defending player instead of piling onto the last blocker.
                let trample = atk.has_trample();
                let mut remaining = power;
                for (i, blocker) in its_blockers.iter().enumerate() {
                    let lethal = match self.state.try_object(*blocker) {
                        Some(b) => (b.toughness() - b.damage as i32).max(1),
                        None => remaining,
                    };
                    let assign = if trample || i + 1 != its_blockers.len() {
                        remaining.min(lethal).max(0)
                    } else {
                        remaining
                    };
                    remaining -= assign;
                    if assign > 0 {
                        damage.push((*attacker, Some(*blocker), assign as u32));
                    }
                }
                if trample && remaining > 0 {
                    damage.push((*attacker, None, remaining as u32));
                }
                for blocker in &its_blockers {
                    if let Some(b) = self.state.try_object(*blocker) {
                        let bp = b.power();
                        if bp > 0 {
                            damage.push((*blocker, Some(*attacker), bp as u32));
                        }
                    }
                }
            }
        }

        // CR 510.2: all combat damage is dealt simultaneously.
        let shield_active =
            self.state.prevent_all_combat_damage_turn == Some(self.state.turn.number);
        for (source, target, amount) in damage {
            if shield_active {
                let card = self.state.object(source).card;
                let (recipient, permanent) = match target {
                    None => (defender, None),
                    Some(victim) => (None, Some(victim)),
                };
                events.push(Event::CombatDamagePrevented {
                    source,
                    card,
                    recipient,
                    permanent,
                    amount,
                });
                continue;
            }
            match target {
                None => {
                    // CR 800.4e: combat damage is not assigned to a player who
                    // has left the game.
                    let Some(defender) = defender else {
                        continue;
                    };
                    if !self.state.is_live(defender) {
                        continue;
                    }
                    // CR 614/615: replacement and prevention run before any
                    // life change and before any commander-damage credit, so a
                    // prevented instance never credits the ledger (CR 615.6).
                    let dealt = self.deal_damage(
                        Some(source),
                        DamageTarget::Player(defender),
                        amount,
                        true,
                        events,
                    );
                    if dealt == 0 {
                        continue;
                    }
                    let life = self.state.players[defender as usize].life;
                    let card = self.state.object(source).card;
                    events.push(Event::CombatDamage {
                        attacker: source,
                        attacker_card: card,
                        blocker: None,
                        amount: dealt,
                    });
                    events.push(Event::Damage {
                        source: Some(source),
                        target: format!("P{defender}"),
                        amount: dealt,
                    });
                    events.push(Event::LifeChange {
                        player: defender,
                        delta: -(dealt as i32),
                        life,
                    });
                    // CR 903.10a: only combat damage actually dealt to a player
                    // by a commander credits the ledger, keyed by the stable
                    // CommanderId + recipient. Control changes and fresh zone
                    // ObjectIds never change the key; noncombat damage and
                    // damage to permanents never reach this branch.
                    if self.state.profile.is_commander_combat()
                        && let Some(commander) = self.state.object(source).commander
                    {
                        self.state.add_commander_damage(commander, defender, dealt);
                        let total = self.state.commander_damage_dealt(commander, defender);
                        events.push(Event::CommanderDamage {
                            commander,
                            card,
                            recipient: defender,
                            amount: dealt,
                            total,
                        });
                    }
                }
                Some(victim) => {
                    let dealt = self.deal_damage(
                        Some(source),
                        DamageTarget::Permanent(victim),
                        amount,
                        true,
                        events,
                    );
                    if dealt == 0 {
                        continue;
                    }
                    let victim_card = self.state.object(victim).card;
                    let label = format!("{} ({})", victim_card.name(), victim);
                    events.push(Event::Damage {
                        source: Some(source),
                        target: label,
                        amount: dealt,
                    });
                    events.push(Event::CombatDamage {
                        attacker: source,
                        attacker_card: self.state.object(source).card,
                        blocker: Some((victim, victim_card)),
                        amount: dealt,
                    });
                }
            }
        }
    }

    // ------------------------------------------------------------- rendering

    pub fn describe_object(&self, id: ObjectId) -> String {
        match self.state.try_object(id) {
            Some(obj) => format!("{} ({})", obj.card.name(), id),
            None => format!("unknown object {}", id.0),
        }
    }

    pub fn describe_target(&self, target: Target) -> String {
        match target {
            Target::Player(p) => format!("P{p}"),
            Target::Permanent(id) => self.describe_object(id),
            Target::Spell(id) => match self.state.try_object(id) {
                Some(o) => format!("{} on the stack", o.card.name()),
                None => "a spell that has left the stack".to_string(),
            },
        }
    }

    // -------------------------------------------------------- legal actions

    pub fn legal_actions(&self) -> Vec<LegalAction> {
        if self.state.is_over() {
            return Vec::new();
        }
        if let Some(pending) = &self.state.pending {
            return self.pending_actions(pending);
        }
        let Some(player) = self.state.turn.priority else {
            return Vec::new();
        };
        let mut out = Vec::new();
        out.push(LegalAction {
            action: Action::Pass,
            label: "Pass priority".to_string(),
            rules: vec!["117.3d", "117.4"],
        });

        // Lands (CR 305.2, 701.18a).
        for id in &self.state.player(player).hand {
            if self
                .validate(player, &Action::PlayLand { card: *id })
                .is_ok()
            {
                let name = self.state.object(*id).card.name();
                out.push(LegalAction {
                    action: Action::PlayLand { card: *id },
                    label: format!("Play {name}"),
                    rules: vec!["305.2", "701.18a"],
                });
            }
        }

        // Spells, one entry per legal target. Command-zone commanders are cast
        // by their owner (CR 903.8).
        let mut hand: Vec<ObjectId> = self.state.player(player).hand.clone();
        hand.extend(self.state.player(player).command_zone.iter().copied());
        for id in hand {
            if self.state.object(id).card.def().is_land() {
                continue;
            }
            for targets in self.target_combinations(player, id) {
                let action = Action::CastSpell {
                    card: id,
                    targets: targets.clone(),
                };
                if self.validate(player, &action).is_err() {
                    continue;
                }
                let name = self.state.object(id).card.name();
                let from_command = self.state.object(id).zone == Zone::Command;
                let label = if targets.is_empty() {
                    if from_command {
                        format!("Cast commander {name} from the command zone")
                    } else {
                        format!("Cast {name}")
                    }
                } else {
                    let rendered: Vec<String> =
                        targets.iter().map(|t| self.describe_target(*t)).collect();
                    format!("Cast {name} -> {}", rendered.join(", "))
                };
                let rules = if self.state.object(id).card.def().is_instant() {
                    vec!["117.1a"]
                } else if self.state.object(id).card.def().has_flash() {
                    vec!["117.1a", "702.8a"]
                } else {
                    vec!["117.1a", "601.2a"]
                };
                out.push(LegalAction {
                    action,
                    label,
                    rules,
                });
            }
        }

        // Mana abilities (CR 605.1a).
        for (land, color) in self.mana_lands(player) {
            out.push(LegalAction {
                action: Action::ActivateManaAbility { land },
                label: format!(
                    "Tap {} for {}",
                    self.state.object(land).card.name(),
                    color.symbol()
                ),
                rules: vec!["605.1a"],
            });
        }

        // Activated abilities (CR 602.2). Each legal target is its own option.
        for object in self.state.battlefield(player) {
            let card = object.card;
            for (index, ability) in crate::ability::abilities(card).iter().enumerate() {
                let Ability::Activated(activated) = ability else {
                    continue;
                };
                let targets: Vec<Vec<Target>> = if activated.target == TargetSpec::None {
                    vec![Vec::new()]
                } else {
                    self.legal_targets(player, activated.target, object.id)
                        .into_iter()
                        .map(|target| vec![target])
                        .collect()
                };
                for target_set in targets {
                    let action = Action::ActivateAbility {
                        source: object.id,
                        ability: index as u8,
                        mode: 0,
                        targets: target_set.clone(),
                    };
                    if self.validate(player, &action).is_err() {
                        continue;
                    }
                    let rendered = if target_set.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " -> {}",
                            target_set
                                .iter()
                                .map(|t| self.describe_target(*t))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    };
                    out.push(LegalAction {
                        action,
                        label: format!("Activate {}: {}{rendered}", card.name(), activated.text),
                        rules: vec!["602.2", "602.2b"],
                    });
                }
            }
        }
        out
    }

    fn pending_actions(&self, pending: &Pending) -> Vec<LegalAction> {
        match pending {
            Pending::Mulligan { player } => {
                let mut out = vec![LegalAction {
                    action: Action::Mulligan { keep: true },
                    label: format!(
                        "Keep hand ({} cards)",
                        self.state.player(*player).hand_size()
                    ),
                    rules: vec!["103.5"],
                }];
                if self.state.player(*player).mulligans < mulligan_limit(self.state.seat_count()) {
                    let next_count = self.state.player(*player).mulligans + 1;
                    let bottom = cards_to_bottom(self.state.seat_count(), next_count);
                    out.push(LegalAction {
                        action: Action::Mulligan { keep: false },
                        label: if bottom == 0 {
                            "Mulligan (first multiplayer mulligan is free; no cards bottomed)"
                                .to_string()
                        } else {
                            format!("Mulligan (would bottom {bottom} card(s))")
                        },
                        rules: vec!["103.5", "103.5c"],
                    });
                }
                out
            }
            Pending::BottomCards { player, count } => vec![LegalAction {
                action: Action::BottomCards {
                    cards: self.sample_bottom(*player, *count),
                },
                label: format!("Put the last {count} card(s) of hand on the bottom"),
                rules: vec!["103.5"],
            }],
            Pending::DeclareAttackers { player } => {
                let legal = self.legal_attackers(*player);
                let mut out = vec![LegalAction {
                    action: Action::DeclareAttackers {
                        attackers: Vec::new(),
                        defender: None,
                    },
                    label: "Declare no attackers".to_string(),
                    rules: vec!["508.1a"],
                }];
                if !legal.is_empty() {
                    let names: Vec<&str> = legal
                        .iter()
                        .map(|id| self.state.object(*id).card.name())
                        .collect();
                    for defender in 0..self.state.seat_count() as PlayerId {
                        if defender == *player || !self.state.is_live(defender) {
                            continue;
                        }
                        out.push(LegalAction {
                            action: Action::DeclareAttackers {
                                attackers: legal.clone(),
                                defender: Some(defender),
                            },
                            label: format!(
                                "Attack P{defender} with all eligible creatures ({})",
                                names.join(", ")
                            ),
                            rules: vec!["508.1a", "508.1f"],
                        });
                    }
                }
                out
            }
            Pending::DeclareBlockers { player } => {
                let mut out = vec![LegalAction {
                    action: Action::DeclareBlockers { blocks: Vec::new() },
                    label: "Declare no blockers".to_string(),
                    rules: vec!["509.1a"],
                }];
                let greedy = self.sample_blocks(*player);
                if !greedy.is_empty() {
                    out.push(LegalAction {
                        action: Action::DeclareBlockers {
                            blocks: greedy.clone(),
                        },
                        label: format!(
                            "Block with every available creature ({} block(s))",
                            greedy.len()
                        ),
                        rules: vec!["509.1a"],
                    });
                }
                out
            }
            Pending::OrderCard {
                viewed,
                index,
                mode,
                ..
            } => {
                let card = self.state.object(viewed[*index]).card.name();
                let verb = match mode {
                    OrderMode::Scry => "on top",
                    OrderMode::Surveil => "on top",
                };
                let off = match mode {
                    OrderMode::Scry => "on the bottom",
                    OrderMode::Surveil => "into the graveyard",
                };
                vec![
                    LegalAction {
                        action: Action::OrderCard { to_top: true },
                        label: format!("Put {card} {verb}"),
                        rules: vec![match mode {
                            OrderMode::Scry => "701.22a",
                            OrderMode::Surveil => "701.25a",
                        }],
                    },
                    LegalAction {
                        action: Action::OrderCard { to_top: false },
                        label: format!("Put {card} {off}"),
                        rules: vec![match mode {
                            OrderMode::Scry => "701.22a",
                            OrderMode::Surveil => "701.25a",
                        }],
                    },
                ]
            }
            Pending::Discard { player, count } => vec![LegalAction {
                action: Action::Discard {
                    cards: self.sample_discard(*player, *count),
                },
                label: format!("Discard the last {count} card(s) of hand"),
                rules: vec!["514.1"],
            }],
            Pending::ApnapSacrifice { player, .. } => {
                let permanents = self.controlled_permanents(*player);
                if permanents.is_empty() {
                    vec![LegalAction {
                        action: Action::ChooseSacrifice { permanent: None },
                        label: "Sacrifice nothing (no permanents)".to_string(),
                        rules: vec!["101.4", "701.21a"],
                    }]
                } else {
                    permanents
                        .into_iter()
                        .map(|id| LegalAction {
                            action: Action::ChooseSacrifice {
                                permanent: Some(id),
                            },
                            label: format!("Sacrifice {}", self.describe_object(id)),
                            rules: vec!["101.4", "701.21a"],
                        })
                        .collect()
                }
            }
            Pending::CommanderDraw { object, .. } => vec![
                LegalAction {
                    action: Action::AnswerCommanderDraw {
                        to_command_zone: true,
                    },
                    label: format!(
                        "Put commander {} into the command zone instead of drawing it (CR 903.9b)",
                        self.describe_object(*object)
                    ),
                    rules: vec!["903.9b"],
                },
                LegalAction {
                    action: Action::AnswerCommanderDraw {
                        to_command_zone: false,
                    },
                    label: format!("Draw commander {} into hand", self.describe_object(*object)),
                    rules: vec!["903.9b"],
                },
            ],
            Pending::CommanderReplacement {
                destination,
                object,
                ..
            } => vec![
                LegalAction {
                    action: Action::AnswerCommanderReplacement {
                        to_command_zone: true,
                    },
                    label: format!(
                        "Put commander {} into the command zone instead (CR 903.9b)",
                        self.describe_object(*object)
                    ),
                    rules: vec!["903.9b"],
                },
                LegalAction {
                    action: Action::AnswerCommanderReplacement {
                        to_command_zone: false,
                    },
                    label: format!(
                        "Put commander {} into the {}",
                        self.describe_object(*object),
                        destination.name()
                    ),
                    rules: vec!["903.9b"],
                },
            ],
            Pending::CommanderReturn { order, cursor, .. } => {
                let current = &order[*cursor];
                vec![
                    LegalAction {
                        action: Action::AnswerCommanderReturn {
                            to_command_zone: true,
                        },
                        label: format!(
                            "Return commander {} to the command zone (CR 903.9a)",
                            self.describe_object(current.object)
                        ),
                        rules: vec!["903.9a"],
                    },
                    LegalAction {
                        action: Action::AnswerCommanderReturn {
                            to_command_zone: false,
                        },
                        label: format!(
                            "Leave commander {} in the {}",
                            self.describe_object(current.object),
                            current.zone.name()
                        ),
                        rules: vec!["903.9a"],
                    },
                ]
            }
            Pending::TriggerTarget { legal, .. } => legal
                .iter()
                .map(|target| LegalAction {
                    action: Action::ChooseTriggerTarget { target: *target },
                    label: format!("Target {}", self.describe_target(*target)),
                    rules: vec!["603.3d"],
                })
                .collect(),
            Pending::TriggerChoice { trigger, .. } => vec![
                LegalAction {
                    action: Action::AnswerTriggerChoice { use_ability: true },
                    label: "Use the optional triggered ability".to_string(),
                    rules: vec!["603.5"],
                },
                LegalAction {
                    action: Action::AnswerTriggerChoice { use_ability: false },
                    label: format!(
                        "Decline {}'s optional triggered ability",
                        trigger.card.name()
                    ),
                    rules: vec!["603.5"],
                },
            ],
        }
    }

    fn sample_bottom(&self, player: PlayerId, count: u8) -> Vec<ObjectId> {
        let hand = &self.state.player(player).hand;
        hand.iter()
            .rev()
            .take(count as usize)
            .rev()
            .copied()
            .collect()
    }

    fn sample_discard(&self, player: PlayerId, count: u8) -> Vec<ObjectId> {
        let hand = &self.state.player(player).hand;
        hand.iter()
            .rev()
            .take(count as usize)
            .rev()
            .copied()
            .collect()
    }

    /// A deterministic default blocking plan: every untapped creature blocks
    /// the largest attacker it can, in id order.
    fn sample_blocks(&self, player: PlayerId) -> Vec<BlockDecl> {
        let mut attackers: Vec<(ObjectId, i32)> = self
            .state
            .turn
            .attackers
            .iter()
            .filter_map(|id| self.state.try_object(*id).map(|o| (*id, o.power())))
            .collect();
        attackers.sort_by_key(|(id, power)| (std::cmp::Reverse(*power), id.0));
        let mut out = Vec::new();
        for blocker in self.legal_blockers(player) {
            let Some(blocker_object) = self.state.try_object(blocker) else {
                continue;
            };
            // Only offer a pair the rules actually allow (CR 509.1b).
            let Some((attacker, _)) = attackers.iter().find(|(id, _)| {
                self.state.try_object(*id).is_some_and(|a| {
                    crate::tokens::can_block(a.keywords(), blocker_object.keywords())
                })
            }) else {
                continue;
            };
            out.push(BlockDecl {
                blocker,
                attacker: *attacker,
            });
        }
        out
    }
}

fn check_selection(
    cards: &[ObjectId],
    hand: &[ObjectId],
    count: usize,
    what: &'static str,
) -> Result<(), EngineError> {
    if cards.len() != count {
        return Err(EngineError::illegal(
            what,
            format!("expected {count} card(s), got {}", cards.len()),
        ));
    }
    let mut seen: Vec<ObjectId> = Vec::new();
    for card in cards {
        if seen.contains(card) {
            return Err(EngineError::illegal(what, "the same card was chosen twice"));
        }
        if !hand.contains(card) {
            return Err(EngineError::illegal(
                what,
                "every chosen card must be in that player's hand",
            ));
        }
        seen.push(*card);
    }
    Ok(())
}

/// Every permanent on the battlefield with a "creature dies" watcher ability,
/// snapshotted before a batch of deaths moves anything. A watcher that dies in
/// the same batch still sees the batch (CR 704.3, 603.6c).
fn death_watchers(game: &Game) -> Vec<(ObjectId, CardId, PlayerId)> {
    game.state
        .objects
        .values()
        .filter(|object| object.zone == Zone::Battlefield)
        .filter(|object| {
            ability::triggers_on(object.card, TriggerEvent::CreatureDies)
                .next()
                .is_some()
                || ability::triggers_on(object.card, TriggerEvent::CreatureYouControlDies)
                    .next()
                    .is_some()
                || ability::triggers_on(object.card, TriggerEvent::AnotherCreatureYouControlDies)
                    .next()
                    .is_some()
        })
        .map(|object| (object.id, object.card, object.controller))
        .collect()
}

fn target_spec_rule(spec: TargetSpec) -> &'static str {
    match spec {
        TargetSpec::None => "CR 601.2c",
        TargetSpec::AnyTarget => "any target: CR 115.4",
        TargetSpec::Player => "target player: CR 115.4",
        TargetSpec::Creature => "target creature: CR 115.1",
        TargetSpec::OpponentCreature => "target creature an opponent controls: CR 115.1",
        TargetSpec::Spell => "target spell: CR 115.1",
        TargetSpec::NoncreatureSpell => "target noncreature spell: CR 115.1",
    }
}

/// Ability index of a non-spell stack entry.
pub fn ability_index(entry: StackEntry) -> u8 {
    match entry {
        StackEntry::Spell => u8::MAX,
        StackEntry::Activated { ability } | StackEntry::Triggered { ability } => ability,
    }
}

/// Number of implemented cards (used by the UI status line).
pub fn pool_size() -> usize {
    catalog::pool_size()
}

// ------------------------------------------------------- sandbox table controls

/// Refuse a manual control with a user-facing reason.
fn sandbox_refused(reason: impl Into<String>) -> EngineError {
    EngineError::SandboxRefused {
        reason: reason.into(),
    }
}

impl Game {
    /// Apply one manual table control.
    ///
    /// This is deliberately **not** a legal-match action: it bypasses the turn
    /// structure, so the caller must record the returned edit as session taint.
    /// The path is deterministic (no random draws), it never runs the scheduler,
    /// it never invents a pending choice, and it bumps the revision/decision
    /// serial so an outstanding submission or an in-flight background decision
    /// is stale. Exactly one typed trace event is emitted; the caller appends it
    /// to the session log.
    pub fn sandbox(&mut self, op: SandboxOp) -> Result<SandboxOutcome, EngineError> {
        if self.state.is_over() {
            return Err(EngineError::GameOver);
        }
        if self.state.resolving.is_some() {
            return Err(sandbox_refused(
                "a stack item is mid-resolution; finish the current resolution before using a sandbox control",
            ));
        }
        if self.state.pending.is_some() {
            return Err(sandbox_refused(
                "a player has a pending choice; answer it before using a sandbox control",
            ));
        }

        let mut events: Vec<Event> = Vec::new();
        // (affected count, private cards, public detail, seat the edit is recorded against)
        let (affected, private_cards, detail, edit_seat): (u32, Vec<CardId>, String, PlayerId) =
            match op {
                SandboxOp::Draw { seat, count } => {
                    self.sandbox_seat(seat)?;
                    let take = self.sandbox_top(seat, count)?;
                    for _ in 0..take.len() {
                        self.draw(seat, &mut events);
                    }
                    let drawn = take.len() as u32;
                    (
                        drawn,
                        Vec::new(),
                        format!("P{seat} drew {drawn} card(s) outside the turn structure"),
                        seat,
                    )
                }
                SandboxOp::Untap { seat } => {
                    self.sandbox_seat(seat)?;
                    let ids: Vec<ObjectId> = self
                        .state
                        .battlefield(seat)
                        .into_iter()
                        .map(|object| object.id)
                        .collect();
                    let mut untapped = 0u32;
                    for id in ids {
                        if let Some(object) = self.state.objects.get_mut(&id) {
                            object.tapped = false;
                        }
                        untapped += 1;
                    }
                    (
                        untapped,
                        Vec::new(),
                        format!(
                            "P{seat} untapped {untapped} permanent(s); this is not an untap step, so summoning sickness is unchanged (CR 502)"
                        ),
                        seat,
                    )
                }
                SandboxOp::Mill { seat, count } => {
                    self.sandbox_seat(seat)?;
                    let take = self.sandbox_top(seat, count)?;
                    for id in &take {
                        self.sandbox_mill_one(*id);
                    }
                    let milled = take.len() as u32;
                    (
                        milled,
                        Vec::new(),
                        format!(
                            "P{seat} put the top {milled} card(s) of their library into their graveyard"
                        ),
                        seat,
                    )
                }
                SandboxOp::Scry {
                    seat,
                    count,
                    to_top,
                }
                | SandboxOp::Surveil {
                    seat,
                    count,
                    to_top,
                } => {
                    let surveil = matches!(op, SandboxOp::Surveil { .. });
                    self.sandbox_seat(seat)?;
                    let take = self.sandbox_top(seat, count)?;
                    let looked: Vec<CardId> =
                        take.iter().map(|id| self.state.object(*id).card).collect();
                    let looked_count = take.len() as u32;
                    let detail = if to_top {
                        format!(
                            "P{seat} looked at the top {looked_count} card(s) of their library and kept them on top in the same order ({} choice, CR {})",
                            if surveil { "surveil" } else { "scry" },
                            if surveil { "701.25a" } else { "701.22a" }
                        )
                    } else if surveil {
                        for id in &take {
                            self.sandbox_mill_one(*id);
                        }
                        format!(
                            "P{seat} looked at the top {looked_count} card(s) of their library and put them into their graveyard (surveil choice, CR 701.25a)"
                        )
                    } else {
                        let library = &mut self.state.players[seat as usize].library;
                        let moved: Vec<ObjectId> =
                            library.drain(0..looked_count as usize).collect();
                        library.extend(moved);
                        format!(
                            "P{seat} looked at the top {looked_count} card(s) of their library and put them on the bottom in the same order (scry choice, CR 701.22a)"
                        )
                    };
                    (looked_count, looked, detail, seat)
                }
                SandboxOp::Reveal { seat, count } => {
                    self.sandbox_seat(seat)?;
                    let take = self.sandbox_top(seat, count)?;
                    let names: Vec<&str> = take
                        .iter()
                        .map(|id| self.state.object(*id).card.name())
                        .collect();
                    let revealed = take.len() as u32;
                    (
                        revealed,
                        Vec::new(),
                        format!(
                            "P{seat} revealed the top {revealed} card(s) of their library to every seat: {}",
                            names.join(", ")
                        ),
                        seat,
                    )
                }
                SandboxOp::SetCounters { object, .. } | SandboxOp::AddCounters { object, .. } => {
                    let (target, controller, name, zone) = self.sandbox_permanent(object)?;
                    debug_assert_eq!(zone, Zone::Battlefield);
                    let current = self
                        .state
                        .object(target)
                        .plus_one_counters
                        .min(MAX_SANDBOX_COUNTERS);
                    let next = match op {
                        SandboxOp::SetCounters { count, .. } => count,
                        SandboxOp::AddCounters { delta, .. } => {
                            let wanted = current as i32 + delta;
                            if wanted < 0 {
                                return Err(sandbox_refused(format!(
                                    "{name} has {current} +1/+1 counter(s); removing {} would go below zero",
                                    -delta
                                )));
                            }
                            wanted as u32
                        }
                        _ => unreachable!("matched above"),
                    };
                    if next > MAX_SANDBOX_COUNTERS {
                        return Err(sandbox_refused(format!(
                            "the counter editor supports 0..={MAX_SANDBOX_COUNTERS} +1/+1 counters; {next} was requested"
                        )));
                    }
                    if let Some(object) = self.state.objects.get_mut(&target) {
                        object.plus_one_counters = next;
                    }
                    (
                        next,
                        Vec::new(),
                        format!(
                            "{name} now has {next} +1/+1 counter(s) (tabletop storage only: this engine's combat math does not read them)"
                        ),
                        controller,
                    )
                }
                SandboxOp::SetLife { seat, life } => {
                    self.sandbox_seat(seat)?;
                    if !(1..=MAX_SANDBOX_LIFE).contains(&life) {
                        return Err(sandbox_refused(format!(
                            "a sandbox life total must stay between 1 and {MAX_SANDBOX_LIFE}; 0 or less would end the game, which belongs to a legal match (CR 704.5a)"
                        )));
                    }
                    let previous = self.state.players[seat as usize].life;
                    self.state.players[seat as usize].life = life;
                    (
                        0,
                        Vec::new(),
                        format!(
                            "P{seat} life {previous} → {life} (tabletop life edit, not a rules life change)"
                        ),
                        seat,
                    )
                }
            };

        let kind = op.kind();
        events.push(Event::SandboxOverride {
            player: edit_seat,
            kind,
            count: affected,
            detail: detail.clone(),
            private_cards,
        });
        // A sandbox edit is a new state: an outstanding submission and any
        // in-flight background decision computed against the old serial are stale.
        self.state.revision += 1;
        self.state.decision += 1;
        let edit = SandboxEdit {
            kind,
            seat: edit_seat,
            turn: self.state.turn.number,
            step: self.state.turn.step.name().to_string(),
            detail,
        };
        Ok(SandboxOutcome { events, edit })
    }

    fn sandbox_seat(&self, seat: PlayerId) -> Result<(), EngineError> {
        self.state
            .try_player(seat)
            .map(|_| ())
            .ok_or(EngineError::UnknownSeat { seat })
    }

    /// The top `count` cards of a seat's library, validated up front so a
    /// refused control leaves the state untouched. A commander on top is
    /// refused: its CR 903.9b zone-change choice is a rules decision that a
    /// sandbox control must not answer on the owner's behalf.
    fn sandbox_top(&self, seat: PlayerId, count: u8) -> Result<Vec<ObjectId>, EngineError> {
        if count == 0 {
            return Err(sandbox_refused(
                "a sandbox control needs a count of at least one card",
            ));
        }
        let library = &self.state.player(seat).library;
        if library.len() < count as usize {
            return Err(sandbox_refused(format!(
                "P{seat}'s library holds {} card(s); {count} were requested",
                library.len()
            )));
        }
        let top: Vec<ObjectId> = library.iter().take(count as usize).copied().collect();
        if let Some(id) = top.iter().find(|id| {
            self.state
                .try_object(**id)
                .is_some_and(|o| o.commander.is_some())
        }) {
            let name = self.state.object(*id).name();
            return Err(sandbox_refused(format!(
                "{name} is a commander on top of P{seat}'s library; its CR 903.9b zone-change choice cannot be answered from a sandbox control"
            )));
        }
        Ok(top)
    }

    /// The permanent a counter edit targets. Refuses anything that is not a
    /// permanent on the battlefield, so a hidden card can never be edited by
    /// guessing its object id.
    fn sandbox_permanent(
        &self,
        object: ObjectId,
    ) -> Result<(ObjectId, PlayerId, &'static str, Zone), EngineError> {
        let found = self
            .state
            .try_object(object)
            .ok_or(EngineError::UnknownObject)?;
        let name = found.name();
        if found.zone != Zone::Battlefield {
            return Err(sandbox_refused(format!(
                "counters can only be edited on a permanent on the battlefield; {name} is in the {}",
                found.zone.name()
            )));
        }
        Ok((found.id, found.controller, name, found.zone))
    }

    /// Move one library card to its owner's graveyard without firing a "dies"
    /// trigger: a milled card never entered the battlefield (CR 701.13a).
    fn sandbox_mill_one(&mut self, id: ObjectId) {
        let Some(mut object) = self.detach_object(id) else {
            return;
        };
        object.tapped = false;
        object.damage = 0;
        object.summoning_sick = false;
        object.pump_power = 0;
        object.pump_toughness = 0;
        self.insert_object(object, Zone::Graveyard);
    }
}

/// Upper bound for the tabletop counter editor.
pub const MAX_SANDBOX_COUNTERS: u32 = 99;
/// Upper bound for a sandbox life edit.
pub const MAX_SANDBOX_LIFE: i32 = 999;
