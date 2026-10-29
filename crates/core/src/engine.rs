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
    CombatDamageChoice, DrawResume, GameState, MulliganRound, Object, OrderMode, Outcome, Pending,
    Player, Profile, StackEntry, StackItem, Step, TriggerRef, Turn,
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

/// One continuous effect a resolving ability starts on its own source: the
/// layer it belongs to, what it does, its printed wording for the event log and
/// how long it lasts. Grouped into a struct so the helper takes one value
/// instead of six positional arguments.
struct SourceContinuous {
    layer: Layer,
    kind: ContinuousKind,
    text: String,
    duration: EffectDuration,
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
        paired_with: None,
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
            draw_replacements: Vec::new(),
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
                declared_attackers: Vec::new(),
                declared_blockers: Vec::new(),
                unpaired_blockers: Vec::new(),
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
            creature_died_this_turn: false,
            nontoken_creature_deaths_this_turn: Vec::new(),
            activation_uses: BTreeMap::new(),
            skip_next_untap: std::collections::BTreeSet::new(),
            effects: Vec::new(),
            shields: Vec::new(),
            regeneration_shields: Vec::new(),
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
    fn count_value(
        &self,
        controller: PlayerId,
        source: ability::CountSource,
        source_id: ObjectId,
    ) -> i32 {
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
            ability::CountSource::LandsYouControl => self.lands_controlled_by(controller),
            ability::CountSource::OtherSnowPermanentsYouControl => self
                .state
                .objects
                .values()
                .filter(|object| {
                    object.zone == Zone::Battlefield
                        && object.controller == controller
                        && object.id != source_id
                        && object.card.def().is_snow()
                })
                .count() as i32,
        }
    }

    /// CR 613.4a: the number of lands `player` controls right now. A
    /// characteristic-defining value is read from the board on every
    /// characteristic pass (CR 604.3), never captured when the effect started,
    /// so a land that enters after the animation still raises the value.
    fn lands_controlled_by(&self, player: PlayerId) -> i32 {
        self.state
            .objects
            .values()
            .filter(|object| {
                object.zone == Zone::Battlefield
                    && object.controller == player
                    && object.card.def().is_land()
            })
            .count() as i32
    }

    /// CR 613: recompute every battlefield object's characteristics in layer
    /// order — 4 type, 6 ability, 7b base power/toughness, 7c modifiers. The
    /// power/toughness half covers printed count-based statics (Wight of
    /// Precinct Six's "+1/+1 for each creature card in your opponents'
    /// graveyards") and one-shot until-end-of-turn modifiers, the
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
        // CR 613.4b base P/T setters apply before counters and modifiers. A
        // later setter in timestamp order replaces an earlier base. A *printed*
        // static setter (Circle of the Moon Druid's Bear Form) has the timestamp
        // of its permanent's arrival, which precedes every effect created
        // later, so it is applied first and any one-shot setter below overrides
        // it. It also depends on whose turn it is (CR 611.3a), which the engine
        // answers against the active player rather than by storing an effect.
        for (id, card) in &battlefield {
            let controller = self
                .state
                .try_object(*id)
                .map(|object| object.controller)
                .unwrap_or(0);
            if self.state.turn.active != controller {
                continue;
            }
            for static_ability in ability::static_abilities(*card) {
                let ability::StaticAbility::SelfBasePowerToughnessDuringYourTurn {
                    power,
                    toughness,
                    ..
                } = static_ability
                else {
                    continue;
                };
                if let Some(object) = self.state.objects.get_mut(id)
                    && object.zone == Zone::Battlefield
                {
                    object.pump_power = power - object.card.def().power.unwrap_or(0);
                    object.pump_toughness = toughness - object.card.def().toughness.unwrap_or(0);
                }
            }
        }
        // CR 613.4a: characteristic-defining power/toughness, applied before any
        // 7b setter and every 7c modifier. The value is read from the board on
        // this pass, not captured when the effect started, so a land that enters
        // later changes it (Beorn's Hospitality). Counters still sit on top of
        // it through `Object::power`/`toughness` (layer 7d).
        let characteristic: Vec<ObjectId> = self
            .state
            .effects
            .iter()
            .filter(|effect| effect.layer == Layer::CharacteristicDefining)
            .filter(|effect| effect.kind == ContinuousKind::PowerToughnessEqualToLands)
            .map(|effect| effect.target)
            .collect();
        for target in characteristic {
            let Some(controller) = self
                .state
                .try_object(target)
                .filter(|object| object.zone == Zone::Battlefield)
                .map(|object| object.controller)
            else {
                continue;
            };
            let value = self.lands_controlled_by(controller);
            if let Some(object) = self.state.objects.get_mut(&target) {
                object.pump_power = value - object.card.def().power.unwrap_or(0);
                object.pump_toughness = value - object.card.def().toughness.unwrap_or(0);
            }
        }
        let mut setting: Vec<(u64, ObjectId, i32, i32)> = self
            .state
            .effects
            .iter()
            .filter(|effect| effect.layer == Layer::Setting)
            .filter_map(|effect| match effect.kind {
                ContinuousKind::SetBasePowerToughness { power, toughness } => {
                    Some((effect.timestamp, effect.target, power, toughness))
                }
                _ => None,
            })
            .collect();
        setting.sort_unstable_by_key(|(timestamp, ..)| *timestamp);
        for (_, target, power, toughness) in setting {
            if let Some(object) = self.state.objects.get_mut(&target)
                && object.zone == Zone::Battlefield
            {
                object.pump_power = power - object.card.def().power.unwrap_or(0);
                object.pump_toughness = toughness - object.card.def().toughness.unwrap_or(0);
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
                let value = self.count_value(controller, count, *id);
                if value == 0 {
                    continue;
                }
                if let Some(object) = self.state.objects.get_mut(id) {
                    object.pump_power += power * value;
                    object.pump_toughness += toughness * value;
                }
            }
        }
        // Threshold statics: +P/+T while the controller's graveyard has at
        // least the printed number of cards (Werebear).
        for (id, card) in &battlefield {
            let controller = self
                .state
                .try_object(*id)
                .map(|object| object.controller)
                .unwrap_or(0);
            let graveyard = self.state.players[controller as usize].graveyard.len();
            for static_ability in ability::static_abilities(*card) {
                let ability::StaticAbility::ThresholdPowerToughness {
                    minimum,
                    power,
                    toughness,
                    ..
                } = static_ability
                else {
                    continue;
                };
                if graveyard >= *minimum
                    && let Some(object) = self.state.objects.get_mut(id)
                {
                    object.pump_power += power;
                    object.pump_toughness += toughness;
                }
            }
        }
        // Printed layer 7c subtype anthems: "Other Bears you control get
        // +2/+2" (Beorn the Fierce). The recipient set is computed from the
        // *effective* subtype before any pump is applied, so a creature made a
        // Bear by a layer 4 effect counts (CR 613.1d) and the source never
        // pumps itself. Additive, like every other layer 7c modifier.
        for (id, card) in &battlefield {
            let controller = self
                .state
                .try_object(*id)
                .map(|object| object.controller)
                .unwrap_or(0);
            let bonus = ability::static_abilities(*card)
                .iter()
                .find_map(|static_ability| match static_ability {
                    ability::StaticAbility::OtherBearsYouControlGet {
                        power, toughness, ..
                    } => Some((*power, *toughness)),
                    _ => None,
                });
            let Some((power, toughness)) = bonus else {
                continue;
            };
            let recipients: Vec<ObjectId> = self
                .state
                .objects
                .values()
                .filter(|object| {
                    object.zone == Zone::Battlefield
                        && object.controller == controller
                        && object.id != *id
                        && self.is_creature(object)
                        && self.has_subtype(object, "Bear")
                })
                .map(|object| object.id)
                .collect();
            for recipient in recipients {
                if let Some(object) = self.state.objects.get_mut(&recipient) {
                    object.pump_power += power;
                    object.pump_toughness += toughness;
                }
            }
        }
        // Printed layer 7c "no abilities" anthem: "Creatures you control with no
        // abilities get +1/+1" (Ruxa, Patient Professor). The same additive 7c
        // layer as the subtype anthems above, but the recipient filter is the
        // shared no-abilities predicate rather than a subtype. The printed text
        // has **no** "Other", so the source itself is a candidate; it can never
        // qualify while this static is one of its abilities, and the code does
        // not special-case it — if some effect ever stripped Ruxa's abilities it
        // would pump itself, which is what the printed words say.
        for (id, card) in &battlefield {
            let controller = self
                .state
                .try_object(*id)
                .map(|object| object.controller)
                .unwrap_or(0);
            let bonus = ability::static_abilities(*card)
                .iter()
                .find_map(|static_ability| match static_ability {
                    ability::StaticAbility::ControlledCreaturesWithNoAbilitiesGet {
                        power,
                        toughness,
                        ..
                    } => Some((*power, *toughness)),
                    _ => None,
                });
            let Some((power, toughness)) = bonus else {
                continue;
            };
            let recipients: Vec<ObjectId> = self
                .state
                .objects
                .values()
                .filter(|object| {
                    object.zone == Zone::Battlefield
                        && object.controller == controller
                        && self.is_creature(object)
                        && self.state.object_has_no_abilities(object.id)
                })
                .map(|object| object.id)
                .collect();
            for recipient in recipients {
                if let Some(object) = self.state.objects.get_mut(&recipient) {
                    object.pump_power += power;
                    object.pump_toughness += toughness;
                }
            }
        }
        // Printed layer 7c *list*-filtered subtype anthems: "Other Spiders,
        // Boars, Bats, Bears, ... and Wolves you control get +1/+1"
        // (Spider-Ham, Peter Porker). Same layer, same additive behaviour and
        // the same effective-subtype read as the Bear anthem above, but the
        // recipient qualifies on *any* one of the printed subtypes, so a
        // creature made a Bear by a layer 4 effect counts and a creature with
        // none of the listed subtypes never does. The printed word is "Other",
        // so the source never pumps itself.
        for (id, card) in &battlefield {
            let controller = self
                .state
                .try_object(*id)
                .map(|object| object.controller)
                .unwrap_or(0);
            let listed = ability::static_abilities(*card)
                .iter()
                .find_map(|static_ability| match static_ability {
                    ability::StaticAbility::OtherControlledCreaturesWithAnySubtypeGet {
                        subtypes,
                        power,
                        toughness,
                        ..
                    } => Some((*subtypes, *power, *toughness)),
                    _ => None,
                });
            let Some((subtypes, power, toughness)) = listed else {
                continue;
            };
            let recipients: Vec<ObjectId> = self
                .state
                .objects
                .values()
                .filter(|object| {
                    object.zone == Zone::Battlefield
                        && object.controller == controller
                        && object.id != *id
                        && self.is_creature(object)
                        && subtypes
                            .iter()
                            .any(|subtype| self.has_subtype(object, subtype))
                })
                .map(|object| object.id)
                .collect();
            for recipient in recipients {
                if let Some(object) = self.state.objects.get_mut(&recipient) {
                    object.pump_power += power;
                    object.pump_toughness += toughness;
                }
            }
        }
        // Printed layer 7c pair static: "As long as this creature is paired with
        // another creature, each of those creatures gets +2/+2" (Druid's
        // Familiar, CR 702.94a). Like every other 7c modifier it is additive, and
        // the pairing is re-validated per source, so a pair that ended pumps
        // nothing even if a stale back-reference survived. Two paired copies of
        // the card each grant their own bonus.
        for (id, card) in &battlefield {
            let bonus = ability::static_abilities(*card)
                .iter()
                .find_map(|ability| match ability {
                    ability::StaticAbility::PairedEachGet {
                        power, toughness, ..
                    } => Some((*power, *toughness)),
                    _ => None,
                });
            let Some((power, toughness)) = bonus else {
                continue;
            };
            let Some(partner) = self.paired_with(*id) else {
                continue;
            };
            for recipient in [*id, partner] {
                if let Some(object) = self.state.objects.get_mut(&recipient) {
                    object.pump_power += power;
                    object.pump_toughness += toughness;
                }
            }
        }
        // Layer 7c one-shot effects in timestamp order (CR 613.7). Layer 6
        // ability grants carry no power/toughness, so they are filtered out here
        // and read directly by `has_trample`.
        let mut modifying: Vec<(u64, ObjectId, i32, i32)> = self
            .state
            .effects
            .iter()
            .filter(|effect| effect.layer == Layer::Modifying)
            .filter_map(|effect| match effect.kind {
                ContinuousKind::ModifyPowerToughness { power, toughness } => {
                    Some((effect.timestamp, effect.target, power, toughness))
                }
                ContinuousKind::GrantsTrample
                | ContinuousKind::GrantsHaste
                | ContinuousKind::SetBasePowerToughness { .. }
                | ContinuousKind::SetCreatureSubtypes(_)
                | ContinuousKind::AddCreatureSubtypes(_)
                | ContinuousKind::SetColor(_)
                | ContinuousKind::PowerToughnessEqualToLands
                | ContinuousKind::Saddled
                | ContinuousKind::Equipped { .. }
                | ContinuousKind::Indestructible
                | ContinuousKind::MustAttackThisCombat => None,
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
        // Evaluate creature types while animation effects are still present.
        // Taking the effects out first makes a legally animated permanent look
        // noncreature and incorrectly detaches its Equipment immediately.
        let invalid_attachments: Vec<_> = self
            .state
            .effects
            .iter()
            .filter(|effect| {
                matches!(effect.kind, ContinuousKind::Equipped { .. })
                    && !self.state.try_object(effect.target).is_some_and(|object| {
                        object.zone == Zone::Battlefield && self.is_creature(object)
                    })
            })
            .map(|effect| effect.id)
            .collect();
        self.state
            .effects
            .retain(|effect| !invalid_attachments.contains(&effect.id));
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

    /// CR 613.1d + CR 205.3: the object's *effective* subtype. A layer 4
    /// type-changing effect replaces the printed subtype line for as long as it
    /// applies (Ursine Champion's "becomes a Bear Berserker"), so every subtype
    /// question in the rules layer asks this instead of reading the printed type
    /// line directly. Successive layer 4 effects apply in timestamp order
    /// (CR 613.7), so the last one wins.
    pub fn has_subtype(&self, object: &Object, subtype: &str) -> bool {
        let mut effective: Option<crate::layer::SubtypeLine> = None;
        // CR 613.1d: an *additive* layer 4 effect (Stuffed Bear's animation)
        // adds its subtype line to the printed one instead of replacing it, so
        // it is tracked separately from the replacement line above.
        let mut added = false;
        // CR 611.3a + 613.1d: a *printed* layer 4 line can be conditional on
        // whose turn it is ("During your turn, this creature is a Bear ...").
        // Its timestamp is the permanent's arrival, so it is answered first and
        // any effect created afterwards in the same layer still wins below.
        if object.zone == Zone::Battlefield && self.state.turn.active == object.controller {
            for static_ability in crate::ability::static_abilities(object.card) {
                if let crate::ability::StaticAbility::SelfBecomesSubtypeDuringYourTurn {
                    line,
                    ..
                } = static_ability
                {
                    effective = Some(*line);
                }
            }
        }
        for effect in self
            .state
            .effects
            .iter()
            .filter(|effect| effect.layer == Layer::Type && effect.target == object.id)
        {
            match effect.kind {
                ContinuousKind::SetCreatureSubtypes(line) => effective = Some(line),
                ContinuousKind::AddCreatureSubtypes(line) if line.has(subtype) => added = true,
                _ => {}
            }
        }
        if added {
            return true;
        }
        match effective {
            Some(line) => line.has(subtype),
            None => object.card.def().has_subtype(subtype),
        }
    }

    /// CR 613.1d + CR 110.1: whether the object is a creature *right now*. A
    /// printed creature card is one; a noncreature permanent is one while a
    /// layer 4 effect has added the creature card type to it (Stuffed Bear's
    /// animation). Every rules question about "is this a creature" asks this
    /// instead of reading the printed card type, so an animated artifact can
    /// attack, block, be targeted, deal and receive damage and die like any
    /// other creature, and reverts the moment the effect ends.
    pub fn is_creature(&self, object: &Object) -> bool {
        object.card.def().is_creature()
            || self.state.effects.iter().any(|effect| {
                effect.layer == Layer::Type
                    && effect.target == object.id
                    && matches!(effect.kind, ContinuousKind::AddCreatureSubtypes(_))
            })
    }

    /// CR 613.1e + CR 202.2: the object's *effective* colour. A layer 5 effect
    /// sets it (Stuffed Bear's animation makes the artifact green); with no
    /// such effect the printed colour answers. `ManaColor::Colorless` is true
    /// only when nothing changed the colour and the card prints no colour.
    pub fn has_color(&self, id: ObjectId, color: crate::card::ManaColor) -> bool {
        let mut set: Option<crate::card::ManaColor> = None;
        for effect in self
            .state
            .effects
            .iter()
            .filter(|effect| effect.layer == Layer::Color && effect.target == id)
        {
            if let ContinuousKind::SetColor(chosen) = effect.kind {
                set = Some(chosen);
            }
        }
        match set {
            Some(chosen) => chosen == color,
            None => self
                .state
                .try_object(id)
                .is_some_and(|object| object.card.def().is_color(color)),
        }
    }

    /// CR 702.19a + 613.4d: trample printed on the card, or trample granted by
    /// an active layer 6 continuous effect. Combat damage assignment reads this
    /// instead of the printed flag, so a granted keyword is executed by the same
    /// rule as a printed one.
    fn has_trample(&self, object: &Object) -> bool {
        object.card.def().has_trample()
            // CR 122.1b: a trample counter grants trample for as long as it is
            // on the permanent, so it is a keyword source like a printed or
            // granted ability (Beorn the Fierce).
            || object
                .counters
                .get(crate::counters::CounterKind::Trample)
                > 0
            || self.state.effects.iter().any(|effect| {
                effect.kind == ContinuousKind::GrantsTrample && effect.target == object.id
            })
            || self.state.objects.values().any(|source| {
                source.zone == Zone::Battlefield
                    && source.id != object.id
                    && source.controller == object.controller
                    && crate::ability::static_abilities(source.card).iter().any(|ability| {
                        matches!(
                            ability,
                            crate::ability::StaticAbility::OtherControlledCreaturesHaveTrample { .. }
                        )
                    })
            })
    }

    fn graveyard_card_type_count(&self, player: PlayerId) -> i32 {
        let mut mask = 0u8;
        for id in &self.state.player(player).graveyard {
            let Some(object) = self.state.try_object(*id) else {
                continue;
            };
            for (bit, kind) in [
                (1u8, crate::card::Kind::Artifact),
                (2, crate::card::Kind::Creature),
                (4, crate::card::Kind::Enchantment),
                (8, crate::card::Kind::Instant),
                (16, crate::card::Kind::Land),
                (32, crate::card::Kind::Sorcery),
            ] {
                if object.card.def().kinds.contains(&kind) {
                    mask |= bit;
                }
            }
        }
        mask.count_ones() as i32
    }

    /// CR 702.12b: the destroy half of the lethal-damage state-based action does
    /// not apply to an indestructible permanent. `pub(crate)` so the coach can
    /// avoid telling the player an indestructible creature will be destroyed.
    pub(crate) fn has_indestructible(&self, id: ObjectId) -> bool {
        self.state
            .effects
            .iter()
            .any(|effect| effect.kind == ContinuousKind::Indestructible && effect.target == id)
    }

    fn forced_attack_defender(&self, id: ObjectId) -> Option<PlayerId> {
        // The engine permits one defending player per combat. Requirements with
        // different defenders are maximized by choosing one group; they cannot
        // all be satisfied simultaneously (CR 508.1d).
        self.forced_attack_defender_single(id)
    }

    fn forced_attack_defender_single(&self, id: ObjectId) -> Option<PlayerId> {
        self.state.effects.iter().find_map(|effect| {
            (effect.kind == ContinuousKind::MustAttackThisCombat && effect.target == id)
                .then_some(effect.controller)
                .filter(|p| self.state.is_live(*p))
        })
    }

    fn has_shroud(&self, id: ObjectId) -> bool {
        self.state.effects.iter().any(|effect| {
            matches!(effect.kind, ContinuousKind::Equipped { shroud: true, .. })
                && effect.target == id
                && self.state.try_object(id).is_some_and(|object| {
                    object.zone == Zone::Battlefield && self.is_creature(object)
                })
        })
    }

    fn has_hexproof(&self, id: ObjectId) -> bool {
        self.state.effects.iter().any(|effect| {
            matches!(effect.kind, ContinuousKind::Equipped { hexproof: true, .. })
                && effect.target == id
                && self.state.try_object(id).is_some_and(|object| {
                    object.zone == Zone::Battlefield && self.is_creature(object)
                })
        })
    }

    fn can_target_permanent(&self, id: ObjectId, caster: PlayerId) -> bool {
        if self.has_shroud(id) {
            return false;
        }
        if self.has_hexproof(id)
            && self
                .state
                .try_object(id)
                .is_some_and(|object| object.controller != caster)
        {
            return false;
        }
        true
    }

    fn has_saddled(&self, id: ObjectId) -> bool {
        self.state
            .effects
            .iter()
            .any(|effect| effect.kind == ContinuousKind::Saddled && effect.target == id)
    }

    fn has_haste(&self, object: &Object) -> bool {
        self.state.effects.iter().any(|effect| {
            (effect.kind == ContinuousKind::GrantsHaste
                || matches!(effect.kind, ContinuousKind::Equipped { haste: true, .. }))
                && effect.target == object.id
        })
    }

    /// Start one continuous effect on the ability's own source, for a printed
    /// ability that animates its own permanent (Stuffed Bear's three layered
    /// until-end-of-turn effects, Beorn's Hospitality's two that do not end).
    /// The source must still be a battlefield permanent. The result is an
    /// ordinary `ContinuousEffect`, so cleanup expiry (CR 514.2) and departure
    /// pruning (CR 400.7) come from the existing pipeline instead of a bespoke
    /// lifetime, and the printed duration travels with the effect.
    fn start_source_continuous(
        &mut self,
        item: &StackItem,
        controller: PlayerId,
        effect: SourceContinuous,
        events: &mut Vec<Event>,
    ) {
        let card = match self.state.try_object(item.object) {
            Some(object) if object.zone == Zone::Battlefield => object.card,
            _ => return,
        };
        let timestamp = self.next_timestamp();
        let id = self.next_effect_id();
        self.state.effects.push(ContinuousEffect {
            id,
            timestamp,
            source: Some(item.object),
            card,
            controller,
            layer: effect.layer,
            target: item.object,
            kind: effect.kind,
            duration: effect.duration,
        });
        events.push(Event::ContinuousEffectStarted {
            object: item.object,
            card,
            layer: effect.layer.label().to_string(),
            text: effect.text,
        });
        self.recompute_layers();
    }

    fn add_haste_grant(&mut self, item: &StackItem, target: ObjectId, events: &mut Vec<Event>) {
        let card = item.card;
        let timestamp = self.next_timestamp();
        let id = self.next_effect_id();
        self.state.effects.push(ContinuousEffect {
            id,
            timestamp,
            source: Some(item.object),
            card,
            controller: item.controller,
            layer: Layer::Ability,
            target,
            kind: ContinuousKind::GrantsHaste,
            duration: EffectDuration::UntilEndOfTurn,
        });
        events.push(Event::ContinuousEffectStarted {
            object: target,
            card,
            layer: Layer::Ability.label().to_string(),
            text: format!("gains haste until end of turn ({})", Layer::Ability.rules()),
        });
    }

    /// Start a layer 6 continuous effect granting trample to `target`.
    fn add_trample_grant(&mut self, item: &StackItem, target: ObjectId, events: &mut Vec<Event>) {
        let card = item.card;
        let timestamp = self.next_timestamp();
        let id = self.next_effect_id();
        self.state.effects.push(ContinuousEffect {
            id,
            timestamp,
            source: Some(item.object),
            card,
            controller: item.controller,
            layer: Layer::Ability,
            target,
            kind: ContinuousKind::GrantsTrample,
            duration: EffectDuration::UntilEndOfTurn,
        });
        events.push(Event::ContinuousEffectStarted {
            object: target,
            card,
            layer: Layer::Ability.label().to_string(),
            text: format!(
                "gains trample until end of turn ({})",
                Layer::Ability.rules()
            ),
        });
    }

    /// CR 613.1d: start a layer 4 effect that *adds* a creature subtype line to
    /// `target` until end of turn (Beorn the Fierce's "it becomes a Bear in
    /// addition to its other types"). Additive, so the printed subtypes stay;
    /// `Game::has_subtype` answers for it everywhere and cleanup ends it.
    fn add_subtype_addition_effect(
        &mut self,
        item: &StackItem,
        target: ObjectId,
        line: crate::layer::SubtypeLine,
        events: &mut Vec<Event>,
    ) {
        let card = item.card;
        let timestamp = self.next_timestamp();
        let id = self.next_effect_id();
        self.state.effects.push(ContinuousEffect {
            id,
            timestamp,
            source: Some(item.object),
            card,
            controller: item.controller,
            layer: Layer::Type,
            target,
            kind: ContinuousKind::AddCreatureSubtypes(line),
            duration: EffectDuration::UntilEndOfTurn,
        });
        events.push(Event::ContinuousEffectStarted {
            object: target,
            card,
            layer: Layer::Type.label().to_string(),
            text: format!(
                "becomes {} in addition to its other types until end of turn ({})",
                line.text(),
                Layer::Type.rules()
            ),
        });
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

    fn mana_sources(&self, p: PlayerId) -> Vec<(ObjectId, ManaColor, u8)> {
        self.state
            .objects
            .values()
            .filter(|o| o.zone == Zone::Battlefield && o.controller == p && !o.tapped)
            .filter_map(|o| {
                let def = o.card.def();
                if let Some(color) = def.basic_mana {
                    Some((o.id, color, 1))
                } else if let Some(color) = o.card.creature_mana() {
                    (!o.summoning_sick || self.has_haste(o)).then_some((o.id, color, 1))
                } else {
                    def.artifact_mana
                        .map(|(color, amount)| (o.id, color, amount))
                }
            })
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

    /// CR 601.2f: static cost reductions from permanents the caster controls.
    /// Only the *generic* part of a cost is ever reduced (CR 118.7a), so the
    /// colored pips of a printed cost stay payable. A permanent contributes its
    /// reduction only while it is on the battlefield under the caster's control.
    fn cost_reduction(&self, caster: PlayerId, card: CardId) -> u8 {
        let def = card.def();
        if !def.is_creature() {
            return 0;
        }
        // A card's power outside the battlefield is its printed power: no effect
        // in this pool changes a card's characteristics in a hidden zone.
        let power = def.power.unwrap_or(0);
        let mut reduction: u8 = 0;
        for object in self.state.objects.values() {
            if object.zone != Zone::Battlefield || object.controller != caster {
                continue;
            }
            for static_ability in ability::static_abilities(object.card) {
                if let ability::StaticAbility::CreatureSpellsYouCastWithPowerCostLess {
                    minimum_power,
                    generic,
                    ..
                } = static_ability
                    && power >= *minimum_power
                {
                    reduction = reduction.saturating_add(*generic);
                }
            }
        }
        reduction
    }

    /// Total mana cost of casting `card` from its current zone. A command-zone
    /// cast adds {2} per previous command-zone cast (CR 903.8); generic-mana
    /// overflow is a typed refusal, never a silent wrap.
    fn effective_cost(&self, card: ObjectId) -> Result<ManaCost, EngineError> {
        self.effective_cost_parts(card, 0)
    }

    /// CR 107.3a + 601.2f: the total cost of casting `card` with `x` announced.
    /// X is added to the generic part before the static cost reduction applies,
    /// because the reduction applies to the *total* generic cost — an X of 2 is
    /// reduced exactly like the printed generic mana is.
    fn effective_cost_parts(&self, card: ObjectId, x: u8) -> Result<ManaCost, EngineError> {
        let object = self.state.object(card);
        let mut cost = object.card.mana_cost().unwrap_or_else(ManaCost::empty);
        if x > 0 {
            cost.generic = cost.generic.checked_add(x).ok_or_else(|| {
                EngineError::illegal(
                    "cast spell",
                    "the announced X makes the generic cost overflow",
                )
            })?;
        }
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
        // CR 601.2f + 118.7a: a static reduction applies to the generic part of
        // the total cost, never to the colored pips, and never below zero.
        let reduction = self.cost_reduction(object.controller, object.card);
        if reduction > 0 {
            cost.generic = cost.generic.saturating_sub(reduction);
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
        let lki = self
            .state
            .objects
            .get(&id)
            .filter(|object| object.zone == Zone::Battlefield)
            .map(|object| (object.power(), object.toughness()));
        // CR 702.94a: a pair lasts only "for as long as you control both of
        // them". Every zone change goes through this one detach point, so the
        // partner's back-reference is cleared here rather than at each caller.
        let leaving_pair = self
            .state
            .objects
            .get(&id)
            .filter(|object| object.zone == Zone::Battlefield)
            .and_then(|object| object.paired_with);
        if let Some(partner) = leaving_pair
            && let Some(other) = self.state.objects.get_mut(&partner)
            && other.paired_with == Some(id)
        {
            other.paired_with = None;
        }
        let object = self.state.objects.remove(&id)?;
        // A regeneration shield belongs to one battlefield permanent, not to
        // the card identity that survives a zone change. Leaving the
        // battlefield ends it; this is not regeneration or shield consumption.
        if object.zone == Zone::Battlefield {
            if object.card == CardId::ExuberantWolfbear
                && let Some(pts) = lki
            {
                for trigger in &mut self.state.trigger_queue {
                    if trigger.source == id && trigger.source_pt_lki.is_none() {
                        trigger.source_pt_lki = Some(pts);
                    }
                }
                for item in &mut self.state.stack {
                    if item.object == id && item.source_pt_lki.is_none() {
                        item.source_pt_lki = Some(pts);
                    }
                }
                if let Some(pending) = &mut self.state.pending {
                    match pending {
                        Pending::TriggerTarget {
                            trigger, queued, ..
                        } => {
                            if trigger.source == id && trigger.source_pt_lki.is_none() {
                                trigger.source_pt_lki = Some(pts);
                            }
                            for t in queued.iter_mut() {
                                if t.source == id && t.source_pt_lki.is_none() {
                                    t.source_pt_lki = Some(pts);
                                }
                            }
                        }
                        Pending::TriggerChoice { trigger, .. } => {
                            if trigger.source == id && trigger.source_pt_lki.is_none() {
                                trigger.source_pt_lki = Some(pts);
                            }
                        }
                        Pending::TriggerMode {
                            trigger, queued, ..
                        } => {
                            if trigger.source == id && trigger.source_pt_lki.is_none() {
                                trigger.source_pt_lki = Some(pts);
                            }
                            for t in queued.iter_mut() {
                                if t.source == id && t.source_pt_lki.is_none() {
                                    t.source_pt_lki = Some(pts);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            self.state.effects.retain(|effect| effect.target != id);
            self.state
                .regeneration_shields
                .retain(|shield| shield.object != id);
            // A captured "it" from a battlefield entry is one incarnation.
            // The mandatory ability stays queued/on the stack, but its affected
            // binding is invalidated so it resolves without touching a later
            // incarnation of the same card.
            for trigger in &mut self.state.trigger_queue {
                if trigger.affected == Some(id) {
                    trigger.affected = None;
                }
            }
            for item in &mut self.state.stack {
                if item.affected == Some(id) {
                    item.affected = None;
                }
            }
            if let Some(pending) = &mut self.state.pending {
                match pending {
                    Pending::TriggerTarget {
                        trigger, queued, ..
                    } => {
                        if trigger.affected == Some(id) {
                            trigger.affected = None;
                        }
                        for trigger in queued.iter_mut() {
                            if trigger.affected == Some(id) {
                                trigger.affected = None;
                            }
                        }
                    }
                    Pending::TriggerChoice { trigger, .. } => {
                        if trigger.affected == Some(id) {
                            trigger.affected = None;
                        }
                    }
                    Pending::TriggerMode {
                        trigger, queued, ..
                    } => {
                        if trigger.affected == Some(id) {
                            trigger.affected = None;
                        }
                        for trigger in queued.iter_mut() {
                            if trigger.affected == Some(id) {
                                trigger.affected = None;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
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
        self.remove_from_combat(id);
        Some(object)
    }

    /// CR 506.4 / 509.1g: removing an attacker does not remove its blockers
    /// from combat. Retain unpaired blocking membership until those creatures
    /// themselves leave combat, not until their former target disappears.
    fn remove_from_combat(&mut self, id: ObjectId) {
        let turn = &mut self.state.turn;
        let newly_unpaired: Vec<_> = turn
            .blockers
            .iter()
            .filter(|(blocker, attacker)| *attacker == id && *blocker != id)
            .map(|(blocker, _)| *blocker)
            .collect();
        turn.attackers.retain(|c| *c != id);
        turn.blocked.retain(|c| *c != id);
        turn.blockers
            .retain(|(blocker, attacker)| *blocker != id && *attacker != id);
        turn.unpaired_blockers.retain(|c| *c != id);
        turn.unpaired_blockers.extend(newly_unpaired);
        turn.unpaired_blockers
            .retain(|c| !turn.blockers.iter().any(|(blocker, _)| blocker == c));
        turn.unpaired_blockers.sort_unstable();
        turn.unpaired_blockers.dedup();
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
            DrawResume::EachPlayer {
                source,
                cursor,
                next,
                cards_left,
                per_seat,
                done,
                live_total,
            } => {
                self.each_player_draw(
                    source, cursor, next, cards_left, per_seat, done, live_total, true, events,
                );
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
        let combat = self.capture_combat_alone();
        if let Some(record) = self.move_to_graveyard_inner(id, events, &combat) {
            self.queue_watchers_for_deaths(&death_watchers(self), &[record]);
        }
    }

    /// Move one permanent to its owner's graveyard and return the death record
    /// for the "creature dies" watcher dispatch. When `dispatch == false` the
    /// caller batches several simultaneous deaths and dispatches the watchers
    /// once for the whole batch (CR 704.3, 603.6c): a watcher that dies in the
    /// same batch must still see its fellow dying creatures.
    fn tracks_death_history(&self) -> bool {
        self.state
            .objects
            .values()
            .any(|object| matches!(object.card, CardId::UlvenwaldBear | CardId::CallerOfTheClaw))
    }

    /// CR 614.8/701.19: apply one regeneration shield to a destruction event.
    /// Returns true when destruction was replaced. Sacrifice/exile/bounce and
    /// zero-toughness moves never call this helper.
    fn try_regenerate(&mut self, id: ObjectId, events: &mut Vec<Event>) -> bool {
        let Some(position) = self
            .state
            .regeneration_shields
            .iter()
            .position(|shield| shield.object == id)
        else {
            return false;
        };
        self.state.regeneration_shields.remove(position);
        // A shield only regenerates a creature (CR 701.19a); an animated
        // artifact counts, so ask the layered query before taking the mutable
        // borrow.
        if !self
            .state
            .try_object(id)
            .is_some_and(|object| object.zone == Zone::Battlefield && self.is_creature(object))
        {
            return false;
        }
        let Some(object) = self.state.objects.get_mut(&id) else {
            return false;
        };
        object.damage = 0;
        object.tapped = true;
        let name = object.card.name();
        self.remove_from_combat(id);
        events.push(Event::StateBasedAction {
            description: format!(
                "{name} is regenerated: destruction replaced, damage removed, tapped, removed from combat (CR 701.19a)"
            ),
        });
        true
    }

    fn move_to_graveyard_inner(
        &mut self,
        id: ObjectId,
        events: &mut Vec<Event>,
        combat: &CombatAloneSnapshot,
    ) -> Option<DeathRecord> {
        // CR 700.4 + 613.1d: a permanent that dies while a layer 4 effect makes
        // it a creature *is* a creature dying, so the record keeps that answer
        // from the moment it leaves rather than the printed card type (an
        // animated Stuffed Bear is a creature when it is put into a graveyard).
        let dying = self.state.try_object(id).and_then(|object| {
            (object.zone == Zone::Battlefield)
                .then(|| (object.card, object.controller, self.is_creature(object)))
        });
        // CR 508.1 + 509.1: "was attacking or blocking alone" is read from the
        // live pre-death combat membership captured ONCE for this batch by
        // `capture_combat_alone` (CR 704.3). It must be captured before this
        // call, because `detach_object` prunes the live lists as part of the
        // zone change below. The declaration record is NOT this predicate
        // (CR 506.5 separates "attacks alone" from "is attacking alone").
        let alone_in_combat = combat.died_alone(id);
        let token = self
            .state
            .try_object(id)
            .is_some_and(|object| object.card.is_token());
        let mut object = self.detach_object(id)?;
        let card = object.card;
        let owner = object.owner;
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
        if let Some((card, controller, was_creature)) = dying {
            if was_creature && self.tracks_death_history() {
                self.state.creature_died_this_turn = true;
                if !card.is_token() {
                    let seats = self.state.seat_count();
                    self.state
                        .nontoken_creature_deaths_this_turn
                        .resize(seats, 0);
                    self.state.nontoken_creature_deaths_this_turn[owner as usize] += 1;
                }
            }
            self.queue_triggers(new_id, card, controller, TriggerEvent::Dies);
            // The dying permanent's own "this creature ... dies" trigger is
            // order-independent, so it is queued here rather than in the batch.
            self.queue_self_death(new_id, card, controller, was_creature);
            return Some(DeathRecord {
                battlefield_id: id,
                graveyard_id: new_id,
                controller,
                was_creature,
                alone_in_combat,
            });
        }
        None
    }

    fn enter_battlefield(
        &mut self,
        id: ObjectId,
        controller: PlayerId,
        events: &mut Vec<Event>,
        entering_x: u8,
    ) {
        let Some(mut object) = self.detach_object(id) else {
            return;
        };
        let card = object.card;
        let is_creature = card.def().is_creature();
        object.controller = controller;
        object.tapped = false;
        object.damage = 0;
        // CR 302.6: summoning sickness applies until the controller's next
        // untap step. A noncreature permanent that prints an ability which can
        // make it a creature (Stuffed Bear) records the same restriction when it
        // arrives, because the restriction asks whether the *permanent* has been
        // under its controller's control since their turn began, not whether it
        // was a creature at the time.
        object.summoning_sick = is_creature || crate::ability::can_become_creature(card);
        object.pump_power = 0;
        object.pump_toughness = 0;
        let new_id = self.insert_object(object, Zone::Battlefield);
        // CR 122.2: `insert_object` clears counters that belonged to the previous
        // zone (an object that leaves the battlefield takes its counters with
        // it), so the counters a replacement puts on the *entering* permanent are
        // added to the new battlefield object, after the zone change, and before
        // the entry is announced.
        if entering_x > 0
            && crate::card::enters_with_x_counters(card)
            && let Some(entered) = self.state.objects.get_mut(&new_id)
        {
            entered.counters.add(
                crate::counters::CounterKind::PlusOnePlusOne,
                u32::from(entering_x),
            );
        }
        events.push(Event::PermanentEntered {
            object: new_id,
            card,
            controller,
        });
        // CR 603.6a: an enters-the-battlefield trigger fires once the permanent
        // is on the battlefield.
        let first_new = self.state.trigger_queue.len();
        self.queue_triggers(new_id, card, controller, TriggerEvent::EntersBattlefield);
        if entering_x > 0 {
            // CR 107.3a: a printed "When this creature enters ... X" reads the X
            // announced for the spell that became this permanent, so it travels
            // with the queued trigger exactly like a cast trigger's mana value.
            for trigger in &mut self.state.trigger_queue[first_new..] {
                trigger.x_value = entering_x;
            }
        }
        if is_creature {
            self.queue_another_creature_enters(new_id);
        }
        if card.def().is_land() {
            // A searched/returned land entering is not a land play, but it is
            // the same LandEnters event for every existing landfall watcher.
            self.queue_land_enters(controller);
        }
        // CR 613.1d/613.4b: a *printed* layer 4 line or 7b base setter starts
        // applying the moment its permanent arrives (Circle of the Moon Druid's
        // Bear Form during its controller's turn). Unlike an additive layer 7c
        // printed static — which the engine recomputes at pump/untap points and
        // which is disclosed pre-existing debt — a base-setting value cannot be
        // reconstructed by adding to the printed numbers later, so it is applied
        // here rather than waiting for an unrelated recomputation.
        if crate::ability::static_abilities(card)
            .iter()
            .any(|ability| matches!(ability.layer(), Some(Layer::Type | Layer::Setting)))
        {
            self.recompute_layers();
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn each_player_draw(
        &mut self,
        source: ObjectId,
        cursor: u8,
        mut next: PlayerId,
        mut cards_left: u8,
        per_seat: u8,
        mut done: u8,
        live_total: u8,
        mut completed_draw: bool,
        events: &mut Vec<Event>,
    ) -> bool {
        loop {
            if completed_draw {
                cards_left -= 1;
                completed_draw = false;
                if cards_left == 0 {
                    done += 1;
                    if done >= live_total {
                        return false;
                    }
                    let Some(following) = self.state.next_live_seat_after(next) else {
                        return false;
                    };
                    next = following;
                    cards_left = per_seat;
                }
            }
            if !self.state.is_live(next) {
                let Some(following) = self.state.next_live_seat_after(next) else {
                    return false;
                };
                next = following;
                continue;
            }
            if self.draw_with_resume(
                next,
                DrawResume::EachPlayer {
                    source,
                    cursor,
                    next,
                    cards_left,
                    per_seat,
                    done,
                    live_total,
                },
                0,
                events,
            ) {
                return true;
            }
            completed_draw = true;
        }
    }

    fn draw(&mut self, p: PlayerId, events: &mut Vec<Event>) -> bool {
        self.draw_with_resume(p, DrawResume::Step, 0, events)
    }

    /// CR 614.1a: arm one one-shot "the next time you would draw a card this
    /// turn" replacement for `seat` (Words of Wilding).
    fn arm_draw_replacement(&mut self, seat: PlayerId) {
        let index = seat as usize;
        if self.state.draw_replacements.len() <= index {
            self.state.draw_replacements.resize(index + 1, 0);
        }
        self.state.draw_replacements[index] += 1;
    }

    /// Consume one armed replacement for `seat`. `true` means the draw event is
    /// replaced (CR 614.1a), so no card may move and nothing else may happen for
    /// that draw — not even drawing from an empty library.
    fn take_draw_replacement(&mut self, seat: PlayerId) -> bool {
        match self.state.draw_replacements.get_mut(seat as usize) {
            Some(count) if *count > 0 => {
                *count -= 1;
                true
            }
            _ => false,
        }
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
        // CR 614.1a: the replacement applies to the draw event itself, before the
        // top card is looked at. It therefore also replaces a draw from an empty
        // library, which is not a loss (nothing was drawn) and creates the token
        // instead of a card.
        if self.take_draw_replacement(p) {
            self.create_tokens(p, CardId::BearToken, 1, events);
            return false;
        }
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
    fn queue_self_death(
        &mut self,
        dying: ObjectId,
        card: CardId,
        controller: PlayerId,
        was_creature: bool,
    ) {
        // CR 700.4: only a creature dying is a "creature dies" event; a milled
        // creature card or a sacrificed artifact never triggers these.
        if !was_creature {
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
        deaths: &[DeathRecord],
    ) {
        let deaths: Vec<DeathRecord> = deaths
            .iter()
            .filter(|record| record.was_creature)
            .cloned()
            .collect();
        if deaths.is_empty() {
            return;
        }
        for (source, source_card, source_controller) in watchers.iter().copied() {
            for record in deaths.iter() {
                // The dying permanent's own "this creature ... dies" triggers
                // were already queued by `queue_self_death` at the zone change.
                // Dispatching it again through the watcher batch would double
                // every self-death trigger.
                if record.battlefield_id == source {
                    continue;
                }
                self.queue_triggers(
                    source,
                    source_card,
                    source_controller,
                    TriggerEvent::CreatureDies,
                );
                if source_controller == record.controller {
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
                // CR 603.6c: "whenever another creature dies" has no controller
                // restriction. "Was attacking or blocking alone" uses the
                // pre-death live combat state (CR 506.5), NOT "attacks alone",
                // which describes a declaration. Each death in one simultaneous
                // batch carries the same pre-departure snapshot. A companion
                // leaving in an EARLIER event can make a later death alone.
                if record.alone_in_combat {
                    self.queue_another_creature_dies(
                        source,
                        source_card,
                        source_controller,
                        record.graveyard_id,
                    );
                }
            }
        }
    }

    /// Queue one "whenever another creature dies" watcher for one death in a
    /// batch, recording the dead object so the resolution can exile that exact
    /// card (Thijarian Witness). The caller has already evaluated the printed
    /// intervening-if (CR 603.4) against the pre-zone-change combat state.
    fn queue_another_creature_dies(
        &mut self,
        source: ObjectId,
        card: CardId,
        controller: PlayerId,
        dead_object: ObjectId,
    ) {
        for (index, _) in ability::triggers_on(card, TriggerEvent::AnotherCreatureDies) {
            let seq = self.state.next_trigger_seq;
            self.state.next_trigger_seq += 1;
            self.state.trigger_queue.push(TriggerRef {
                source,
                card,
                controller,
                ability: index,
                seq,
                spell_mana_value: 0,
                affected: None,
                dead_object: Some(dead_object),
                source_pt_lki: None,
                search_amount: 0,
                x_value: 0,
            });
        }
    }

    /// CR 506.5 distinguishes "attacks alone" (declared alone) from "is
    /// attacking alone" (currently the only attacker), likewise for blockers.
    /// Witness's "was attacking or blocking alone" uses the latter, immediately
    /// before death. A creature put onto the battlefield attacking counts too;
    /// a companion removed in an earlier event no longer counts.
    ///
    /// Capture ONCE before a simultaneous batch (CR 704.3), never after each
    /// detach: otherwise the final death would falsely become alone. The
    /// declaration record remains separate and is not a Witness predicate.
    fn capture_combat_alone(&self) -> CombatAloneSnapshot {
        let turn = &self.state.turn;
        let mut blockers: Vec<_> = turn.blockers.iter().map(|block| block.0).collect();
        blockers.extend_from_slice(&turn.unpaired_blockers);
        blockers.sort_unstable();
        blockers.dedup();
        CombatAloneSnapshot {
            attackers: turn.attackers.clone(),
            blockers,
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
            Action::CastSpellWithX { card, x, targets } => {
                self.validate_cast_with_x(player, *card, *x, targets)
            }
            Action::ActivateManaAbility { land } => self.validate_mana_ability(player, *land),
            Action::ActivateColorManaAbility { source, color } => {
                self.validate_color_mana_ability(player, *source, *color)
            }
            Action::SearchLibraryPick { card } => match &state.pending {
                Some(Pending::SearchLibrary {
                    player: p,
                    remaining,
                    chosen,
                    eligible,
                    ..
                }) if *p == player => {
                    if *remaining == 0 || chosen.contains(card) || !eligible.contains(card) {
                        return Err(EngineError::illegal(
                            "search library",
                            "that card is not an available matching land for this search",
                        ));
                    }
                    Ok(())
                }
                _ => Err(EngineError::illegal(
                    "search library",
                    "no library search is waiting for this player",
                )),
            },
            Action::SearchLibraryDone => match &state.pending {
                Some(Pending::SearchLibrary { player: p, .. }) if *p == player => Ok(()),
                _ => Err(EngineError::illegal(
                    "search library",
                    "no library search is waiting for this player",
                )),
            },
            Action::CastSpellWithModes {
                card,
                modes,
                targets,
            } => self.validate_modal_cast(player, *card, modes, targets),
            Action::SaddleMount { source, creatures } => {
                self.validate_saddle_mount(player, *source, creatures)
            }
            Action::ActivateAbility {
                source,
                ability,
                mode,
                targets,
            } => self.validate_activate_ability(
                player,
                ActivationRequest {
                    source: *source,
                    ability: *ability,
                    mode: *mode,
                    targets,
                    cost_cards: &[],
                },
            ),
            Action::ActivateCostedAbility {
                source,
                ability,
                mode,
                targets,
                cost_cards,
            } => self.validate_activate_ability(
                player,
                ActivationRequest {
                    source: *source,
                    ability: *ability,
                    mode: *mode,
                    targets,
                    cost_cards,
                },
            ),
            Action::ChooseTriggerTarget { target } => match &state.pending {
                Some(Pending::TriggerTarget {
                    player: p,
                    legal,
                    aggregate,
                    chosen,
                    ..
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
                    if let Some(budget) = aggregate {
                        // CR 601.2c: adding a target to the set is legal only
                        // while the running total stays within the announced X.
                        if chosen.contains(target) {
                            return Err(EngineError::illegal(
                                "choose trigger target",
                                format!(
                                    "{} is already part of the chosen target set",
                                    self.describe_target(*target)
                                ),
                            ));
                        }
                        let mut total: u32 = 0;
                        for chosen_target in chosen.iter().chain(std::iter::once(target)) {
                            total += self.target_mana_value(*chosen_target);
                        }
                        if total > u32::from(*budget) {
                            return Err(EngineError::illegal(
                                "choose trigger target",
                                format!(
                                    "that target set would cost {total} mana value, over the announced X of {budget} (CR 601.2c)"
                                ),
                            ));
                        }
                    }
                    Ok(())
                }
                _ => Err(EngineError::illegal(
                    "choose trigger target",
                    "no triggered ability is waiting for a target choice (CR 603.3d)",
                )),
            },
            // CR 115.6: declining is legal only for a printed "up to one"
            // target and only while that trigger is waiting for its choice.
            Action::DeclineTriggerTarget => match &state.pending {
                Some(Pending::TriggerTarget {
                    player: p, trigger, ..
                }) if *p == player && trigger_target_is_optional(trigger.card, trigger.ability) => {
                    Ok(())
                }
                _ => Err(EngineError::illegal(
                    "decline trigger target",
                    "no triggered ability with an optional target is waiting for its controller (CR 115.6)",
                )),
            },
            // CR 601.2c: committing a target *set* is legal only while a
            // set-targeting trigger is waiting for its controller.
            Action::FinishTriggerTargets => match &state.pending {
                Some(Pending::TriggerTarget {
                    player: p,
                    aggregate: Some(_),
                    ..
                }) if *p == player => Ok(()),
                _ => Err(EngineError::illegal(
                    "finish trigger targets",
                    "no triggered ability is waiting for a set of targets (CR 601.2c)",
                )),
            },
            Action::AnswerTriggerChoice { use_ability } => match &state.pending {
                Some(Pending::TriggerChoice { player: p, .. })
                | Some(Pending::TriggerMode { player: p, .. })
                    if *p == player =>
                {
                    Ok(())
                }
                Some(Pending::TopCardChoice {
                    player: p,
                    can_take,
                    ..
                }) if *p == player && (!*use_ability || *can_take) => Ok(()),
                _ => Err(EngineError::illegal(
                    "trigger choice",
                    "no optional triggered ability is waiting for its controller (CR 603.5)",
                )),
            },
            Action::AnswerCombatDamageAssignment { .. } => match &state.pending {
                Some(Pending::CombatDamageAssignment { order, cursor, .. })
                    if order[*cursor].player == player =>
                {
                    Ok(())
                }
                _ => Err(EngineError::illegal(
                    "combat damage assignment choice",
                    "no creature is waiting for its controller's damage-assignment answer (CR 510.1)",
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
            // CR 701.9a + 101.4: the seat whose reveal is pending names any subset
            // of the creature cards in its own hand. An empty list is legal (the
            // instruction is "may reveal any number", so zero is a real answer).
            Action::RevealCreatureCards { cards } => match &state.pending {
                Some(Pending::ApnapReveal { player: p, .. }) if *p == player => {
                    let hand = &state.player(player).hand;
                    let mut seen: Vec<ObjectId> = Vec::new();
                    for id in cards {
                        if seen.contains(id) {
                            return Err(EngineError::illegal(
                                "reveal creature cards",
                                "a card can only be revealed once",
                            ));
                        }
                        seen.push(*id);
                        if !hand.contains(id) {
                            return Err(EngineError::illegal(
                                "reveal creature cards",
                                "only cards in the choosing seat's own hand can be revealed (CR 701.9a)",
                            ));
                        }
                        let object = state.try_object(*id).ok_or(EngineError::UnknownObject)?;
                        if !object.card.def().is_creature() {
                            return Err(EngineError::illegal(
                                "reveal creature cards",
                                "only creature cards can be revealed this way",
                            ));
                        }
                    }
                    Ok(())
                }
                _ => Err(EngineError::illegal(
                    "reveal creature cards",
                    "no \"reveal any number of creature cards\" choice is pending (CR 101.4)",
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
        if obj.card.def().basic_mana.is_none()
            && obj.card.def().artifact_mana.is_none()
            && obj.card.creature_mana().is_none()
        {
            return Err(EngineError::NotAManaSource);
        }
        if obj.card.creature_mana().is_some() && obj.summoning_sick && !self.has_haste(obj) {
            return Err(EngineError::BadTiming {
                name: obj.card.name(),
                reason: "a creature with a {T} mana ability is summoning sick (CR 302.6)"
                    .to_string(),
            });
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

    /// CR 702.94a: the creature `id` is paired with, if that pairing is still
    /// real. A pair lasts only "for as long as you control both of them", so the
    /// stored back-reference is never trusted by itself: both objects must still
    /// be on the battlefield, still controlled by the same player, and still
    /// point at each other. Every reader (pair eligibility, the +2/+2 static,
    /// the staging guard) goes through this one validated query.
    pub fn paired_with(&self, id: ObjectId) -> Option<ObjectId> {
        let object = self.state.try_object(id)?;
        if object.zone != Zone::Battlefield {
            return None;
        }
        let partner = object.paired_with?;
        let other = self.state.try_object(partner)?;
        if partner == id {
            return None;
        }
        if other.zone != Zone::Battlefield
            || other.controller != object.controller
            || other.paired_with != Some(id)
        {
            return None;
        }
        Some(partner)
    }

    /// CR 702.94a: pair two permanents. Both sides must be unpaired battlefield
    /// creatures controlled by the trigger's controller; anything else is a
    /// stale request and pairs nothing.
    fn pair_permanents(&mut self, source: ObjectId, partner: ObjectId, events: &mut Vec<Event>) {
        if source == partner {
            return;
        }
        let Some((controller, source_card)) = self
            .state
            .try_object(source)
            .filter(|object| object.zone == Zone::Battlefield)
            .map(|object| (object.controller, object.card))
        else {
            return;
        };
        let eligible = self.state.try_object(partner).is_some_and(|object| {
            object.zone == Zone::Battlefield
                && object.controller == controller
                && self.is_creature(object)
        });
        if !eligible || self.paired_with(source).is_some() || self.paired_with(partner).is_some() {
            return;
        }
        let partner_card = self.state.object(partner).card;
        if let Some(object) = self.state.objects.get_mut(&source) {
            object.paired_with = Some(partner);
        }
        if let Some(object) = self.state.objects.get_mut(&partner) {
            object.paired_with = Some(source);
        }
        events.push(Event::StateBasedAction {
            description: format!(
                "P{controller} pairs {source_card} ({source}) with {partner_card} ({partner}) (soulbond, CR 702.94a)"
            ),
        });
    }

    pub fn legal_targets(
        &self,
        _caster: PlayerId,
        spec: TargetSpec,
        exclude: ObjectId,
    ) -> Vec<Target> {
        let targets = match spec {
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
                        .filter(|o| o.zone == Zone::Battlefield && self.is_creature(o))
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
                .filter(|o| o.zone == Zone::Battlefield && self.is_creature(o))
                .map(|o| Target::Permanent(o.id))
                .collect(),
            TargetSpec::HumanYouControl => self
                .state
                .objects
                .values()
                .filter(|o| {
                    o.zone == Zone::Battlefield
                        && self.is_creature(o)
                        && o.controller == _caster
                        && self.has_subtype(o, "Human")
                })
                .map(|o| Target::Permanent(o.id))
                .collect(),
            // CR 115.6: "up to one" offers the same legal set as "target
            // creature you control"; the difference is on the choice surface,
            // which also offers declining (Beorn the Fierce).
            TargetSpec::UpToOneCreatureYouControl => self
                .state
                .objects
                .values()
                .filter(|o| {
                    o.zone == Zone::Battlefield && self.is_creature(o) && o.controller == _caster
                })
                .map(|o| Target::Permanent(o.id))
                .collect(),
            // CR 702.94a: soulbond's pairing choice. "Another unpaired creature
            // you control" is the printed set minus the ability's own source
            // (soulbond pairs with *another* creature) and minus any creature
            // that is already in a pair, because a pairing needs both sides
            // unpaired. It is not a target in the rules; the engine asks for the
            // pairing choice on the trigger's own choice surface, which also
            // offers declining the "you may".
            TargetSpec::UnpairedAnotherCreatureYouControl => self
                .state
                .objects
                .values()
                .filter(|o| {
                    o.zone == Zone::Battlefield
                        && self.is_creature(o)
                        && o.controller == _caster
                        && o.id != exclude
                        && self.paired_with(o.id).is_none()
                })
                .map(|o| Target::Permanent(o.id))
                .collect(),
            TargetSpec::Bear => self
                .state
                .objects
                .values()
                .filter(|o| {
                    o.zone == Zone::Battlefield
                        && self.is_creature(o)
                        && self.has_subtype(o, "Bear")
                })
                .map(|o| Target::Permanent(o.id))
                .collect(),
            TargetSpec::BearYouControl => self
                .state
                .objects
                .values()
                .filter(|o| {
                    o.zone == Zone::Battlefield
                        && self.is_creature(o)
                        && o.controller == _caster
                        && self.has_subtype(o, "Bear")
                })
                .map(|o| Target::Permanent(o.id))
                .collect(),
            TargetSpec::OpponentCreature => self
                .state
                .objects
                .values()
                .filter(|o| {
                    o.zone == Zone::Battlefield && self.is_creature(o) && o.controller != _caster
                })
                .map(|o| Target::Permanent(o.id))
                .collect(),
            // CR 115.1: "target creature you control" is every creature this
            // player controls — the ability's own source included, if it is a
            // creature when targets are chosen (Beorn's Hospitality can put
            // its own landfall counter on itself once animated). The printed
            // word "another" is the separate spec below.
            TargetSpec::CreatureYouControl => self
                .state
                .objects
                .values()
                .filter(|o| {
                    o.zone == Zone::Battlefield && self.is_creature(o) && o.controller == _caster
                })
                .map(|o| Target::Permanent(o.id))
                .collect(),
            // CR 115.1: "another target creature you control" — the same set
            // minus the ability's own source (Little Bear).
            TargetSpec::AnotherCreatureYouControl => self
                .state
                .objects
                .values()
                .filter(|o| {
                    o.zone == Zone::Battlefield
                        && self.is_creature(o)
                        && o.controller == _caster
                        && o.id != exclude
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
            // CR 115.1 + 110.1: "target creature card with no abilities from
            // your graveyard". The set is read from the graveyard list, not from
            // the battlefield, and each candidate must be a creature card owned
            // by the ability's controller with no abilities (the same shared
            // predicate the anthem and the damage permission use, so the three
            // clauses can never disagree about what "no abilities" means).
            TargetSpec::NoAbilityCreatureCardInYourGraveyard => self
                .state
                .player(_caster)
                .graveyard
                .iter()
                .copied()
                .filter(|id| {
                    self.state.try_object(*id).is_some_and(|object| {
                        object.zone == Zone::Graveyard
                            && object.owner == _caster
                            && object.card.def().is_creature()
                            && self.state.object_has_no_abilities(*id)
                    })
                })
                .map(Target::Card)
                .collect(),
            // CR 115.1 + 601.2c: every artifact and/or enchantment on the
            // battlefield, either controller. "Any number" is an aggregate
            // restriction, not a per-target one, so the budget is applied when
            // the set is chosen (see `Pending::TriggerTarget::aggregate`).
            TargetSpec::ArtifactOrEnchantment => self
                .state
                .objects
                .values()
                .filter(|o| {
                    o.zone == Zone::Battlefield
                        && (o.card.def().is_artifact() || o.card.def().is_enchantment())
                })
                .map(|o| Target::Permanent(o.id))
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
        };
        targets
            .into_iter()
            .filter(|target| match target {
                Target::Permanent(id) => self.can_target_permanent(*id, _caster),
                // A graveyard card is already filtered to the controller's own
                // graveyard above; shroud/hexproof are battlefield-only
                // restrictions (CR 702.11a, 702.18a), so nothing else applies.
                Target::Card(_) => true,
                _ => true,
            })
            .collect()
    }

    fn validate_modal_cast(
        &self,
        player: PlayerId,
        card: ObjectId,
        modes: &[u8; 2],
        targets: &[Target],
    ) -> Result<(), EngineError> {
        let object = self
            .state
            .try_object(card)
            .ok_or(EngineError::UnknownObject)?;
        if object.card != CardId::TitaniasCommand {
            return Err(EngineError::illegal(
                "cast modal spell",
                "that card has no modal modes",
            ));
        }
        if object.zone != Zone::Hand || object.owner != player {
            return Err(EngineError::WrongZone {
                name: "Titania's Command",
                zone: "hand of the acting player",
            });
        }
        if modes[0] == modes[1] || modes[0] > 3 || modes[1] > 3 {
            return Err(EngineError::illegal(
                "cast modal spell",
                "choose exactly two distinct printed modes",
            ));
        }
        let exile = modes.contains(&0);
        if exile {
            if targets.len() != 1 {
                return Err(EngineError::TargetKind { card: object.card });
            }
            let Target::Player(p) = targets[0] else {
                return Err(EngineError::BadTarget {
                    card: object.card,
                    reason: "exile mode requires a target player".to_string(),
                });
            };
            if !self.state.is_live(p) {
                return Err(EngineError::BadTarget {
                    card: object.card,
                    reason: "target player is not live".to_string(),
                });
            }
        } else if !targets.is_empty() {
            return Err(EngineError::TargetKind { card: object.card });
        }
        let spec = object
            .card
            .def()
            .spell_spec()
            .ok_or(EngineError::illegal("cast modal spell", "not a spell"))?;
        self.check_timing(player, object.card.def(), spec)?;
        let cost = self.effective_cost(card)?;
        self.plan_payment(player, &cost)?;
        Ok(())
    }

    fn validate_cast(
        &self,
        player: PlayerId,
        card: ObjectId,
        targets: &[Target],
    ) -> Result<(), EngineError> {
        self.validate_cast_common(player, card, None, targets)
    }

    /// CR 107.3a: a spell whose printed cost contains `{X}` is cast through
    /// `CastSpellWithX`, which announces X explicitly; a spell that does not
    /// print `{X}` cannot announce one. Everything else is the ordinary cast.
    fn validate_cast_with_x(
        &self,
        player: PlayerId,
        card: ObjectId,
        x: u8,
        targets: &[Target],
    ) -> Result<(), EngineError> {
        self.validate_cast_common(player, card, Some(x), targets)
    }

    fn validate_cast_common(
        &self,
        player: PlayerId,
        card: ObjectId,
        x: Option<u8>,
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
        if obj.card == CardId::TitaniasCommand {
            return Err(EngineError::illegal(
                "cast spell",
                "Titania's Command is modal; choose modes with CastSpellWithModes",
            ));
        }
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
        match x {
            Some(_) if !def.has_x_cost() => {
                return Err(EngineError::illegal(
                    "cast spell",
                    format!("{} does not print an {{X}} cost", def.name),
                ));
            }
            Some(value) if value > MAX_X_VALUE => {
                return Err(EngineError::illegal(
                    "cast spell",
                    format!(
                        "the announced X of {value} exceeds the supported maximum of {MAX_X_VALUE}"
                    ),
                ));
            }
            None if def.has_x_cost() => {
                return Err(EngineError::illegal(
                    "cast spell",
                    format!(
                        "{} prints an {{X}} cost; announce X with CastSpellWithX (CR 107.3a)",
                        def.name
                    ),
                ));
            }
            _ => {}
        }
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
        let cost = self.effective_cost_parts(card, x.unwrap_or(0))?;
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
        let forced: Vec<(ObjectId, PlayerId)> = legal
            .iter()
            .filter_map(|id| {
                self.forced_attack_defender(*id)
                    .map(|defender| (*id, defender))
            })
            .collect();
        // CR 508.1d: obey the greatest number of requirements possible without
        // violating restrictions. The inherited one-defender combat model can
        // satisfy only one opponent's group; choosing an unrelated opponent (or
        // a smaller group) must not evade an able creature's requirement.
        let maximum = (0..self.state.seat_count() as PlayerId)
            .map(|p| forced.iter().filter(|(_, required)| *required == p).count())
            .max()
            .unwrap_or(0);
        let obeyed = forced
            .iter()
            .filter(|(attacker, required)| {
                defender == Some(*required) && attackers.contains(attacker)
            })
            .count();
        if obeyed < maximum {
            return Err(EngineError::illegal(
                "declare attackers",
                format!(
                    "this declaration obeys {obeyed} attack requirements, but {maximum} can be obeyed (CR 508.1d)"
                ),
            ));
        }
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
            // the only evasion this slice models (CR 702.9b, 702.17b), plus
            // islandwalk on the defending player's own board (CR 702.14c).
            let attacker = self.state.object(block.attacker);
            let blocker = self.state.object(block.blocker);
            let defender_controls_island = self.controls_island(player);
            if !crate::tokens::can_block(
                attacker.keywords(),
                blocker.keywords(),
                defender_controls_island,
            ) {
                let reason = if attacker
                    .keywords()
                    .contains(&crate::tokens::Keyword::Islandwalk)
                    && defender_controls_island
                {
                    format!(
                        "{} has islandwalk and you control an Island, so it can't be blocked (CR 702.14c)",
                        attacker.name()
                    )
                } else {
                    format!(
                        "{} cannot block {} without flying or reach (CR 509.1b, 702.9b)",
                        blocker.name(),
                        attacker.name()
                    )
                };
                return Err(EngineError::illegal("declare blockers", reason));
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
                    && self.is_creature(o)
                    && !o.tapped
                    && (!o.summoning_sick || self.has_haste(o))
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
                    && self.is_creature(o)
                    && !o.tapped
            })
            .map(|o| o.id)
            .collect()
    }

    /// CR 702.14c: whether `player` controls an Island land. Islandwalk reads
    /// this from the defending player's battlefield only; the attacker's own
    /// lands never matter.
    fn controls_island(&self, player: PlayerId) -> bool {
        self.state.objects.values().any(|o| {
            o.zone == Zone::Battlefield && o.controller == player && o.card.def().is_island()
        })
    }

    /// CR 603.4: evaluate `card`'s printed intervening-if clause against the
    /// defending seat's current board. Called when an attack trigger would fire
    /// and again when it resolves, so a clause that stops being true removes the
    /// ability instead of applying it.
    fn trigger_condition_holds(&self, card: CardId, defender: PlayerId, source: ObjectId) -> bool {
        match ability::trigger_condition(card) {
            ability::TriggerCondition::Always => true,
            ability::TriggerCondition::SourceIsSaddled => self.has_saddled(source),
            ability::TriggerCondition::DefendingPlayerControlsNoBlackNontokenPermanent => {
                !self.state.objects.values().any(|object| {
                    object.zone == Zone::Battlefield
                        && object.controller == defender
                        && !object.card.is_token()
                        // CR 613.1e: the clause reads the permanent's colour,
                        // so a layer 5 effect counts (Stuffed Bear's animation
                        // can only make it green, which is not black, but the
                        // question must be asked of the layered colour and not
                        // of the printed cost).
                        && self.has_color(object.id, crate::card::ManaColor::Black)
                })
            }
        }
    }

    // --------------------------------------------------------------- mutation

    fn apply(&mut self, player: PlayerId, action: &Action, events: &mut Vec<Event>) {
        match action {
            Action::Pass => self.apply_pass(player, events),
            Action::PlayLand { card } => self.apply_play_land(player, *card, events),
            Action::CastSpell { card, targets } => {
                self.apply_cast(player, *card, 0, &[], targets, events)
            }
            Action::CastSpellWithModes {
                card,
                modes,
                targets,
            } => self.apply_cast(player, *card, 0, modes, targets, events),
            Action::CastSpellWithX { card, x, targets } => {
                self.apply_cast(player, *card, *x, &[], targets, events)
            }
            Action::ActivateManaAbility { land } => self.apply_mana_ability(player, *land, events),
            Action::ActivateColorManaAbility { source, color } => {
                self.apply_color_mana_ability(player, *source, *color, events)
            }
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
            Action::RevealCreatureCards { cards } => self.apply_apnap_reveal(player, cards, events),
            Action::AnswerCommanderReturn { to_command_zone } => {
                self.apply_commander_return(player, *to_command_zone, events)
            }
            Action::AnswerCommanderReplacement { to_command_zone } => {
                self.apply_commander_replacement(player, *to_command_zone, events)
            }
            Action::AnswerCommanderDraw { to_command_zone } => {
                self.apply_commander_draw(player, *to_command_zone, events)
            }
            Action::SearchLibraryPick { card } => {
                if let Some(Pending::SearchLibrary {
                    remaining, chosen, ..
                }) = &mut self.state.pending
                {
                    chosen.push(*card);
                    *remaining -= 1;
                }
                self.state.turn.priority = None;
            }
            Action::SearchLibraryDone => self.apply_search_library_done(player, events),
            Action::SaddleMount { source, creatures } => {
                self.apply_saddle_mount(player, *source, creatures, events)
            }
            Action::ActivateAbility {
                source,
                ability,
                mode,
                targets,
            } => self.apply_activate_ability(
                player,
                ActivationRequest {
                    source: *source,
                    ability: *ability,
                    mode: *mode,
                    targets,
                    cost_cards: &[],
                },
                events,
            ),
            Action::ActivateCostedAbility {
                source,
                ability,
                mode,
                targets,
                cost_cards,
            } => self.apply_activate_ability(
                player,
                ActivationRequest {
                    source: *source,
                    ability: *ability,
                    mode: *mode,
                    targets,
                    cost_cards,
                },
                events,
            ),
            Action::ChooseTriggerTarget { target } => {
                self.apply_trigger_target(player, *target, events)
            }
            Action::DeclineTriggerTarget => self.apply_trigger_target_none(player, events),
            Action::FinishTriggerTargets => self.apply_finish_trigger_targets(player, events),
            Action::AnswerTriggerChoice { use_ability } => {
                self.apply_trigger_choice(player, *use_ability, events)
            }
            Action::AnswerCombatDamageAssignment {
                as_though_unblocked,
            } => self.apply_combat_damage_assignment(player, *as_though_unblocked, events),
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
        self.queue_land_enters(player);
        self.put_triggers(events);
        events.push(Event::Priority { player });
    }

    /// CR 603.6a: observe actual land entry regardless of whether a land was
    /// played or put onto the battlefield by an effect. Staging remains with
    /// the caller so searches finish their entire instruction before priority.
    fn queue_land_enters(&mut self, player: PlayerId) {
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
    }

    fn dynamic_mana_colors(&self, player: PlayerId, source: ObjectId) -> Vec<ManaColor> {
        let Some(object) = self.state.try_object(source) else {
            return Vec::new();
        };
        let order = [
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
        ];
        let mut set = [false; 5];
        match object.card.dynamic_mana() {
            Some(crate::card::ManaChoice::CommanderIdentity) => {
                for commander in self
                    .state
                    .commanders
                    .iter()
                    .filter(|commander| commander.owner == player)
                {
                    for color in commander.card.commander_color_identity() {
                        if let Some(index) = order.iter().position(|candidate| candidate == color) {
                            set[index] = true;
                        }
                    }
                }
            }
            Some(crate::card::ManaChoice::OpponentLandColors) => {
                for land in self.state.objects.values() {
                    let def = land.card.def();
                    if land.zone != Zone::Battlefield
                        || land.controller == player
                        || !self.state.is_live(land.controller)
                        || !def.is_land()
                    {
                        continue;
                    }
                    if let Some(color) = def.basic_mana
                        && color != ManaColor::Colorless
                        && let Some(index) = order.iter().position(|candidate| *candidate == color)
                    {
                        set[index] = true;
                    }
                }
            }
            None => return Vec::new(),
        }
        order
            .iter()
            .zip(set)
            .filter_map(|(color, present)| present.then_some(*color))
            .collect()
    }

    fn validate_color_mana_ability(
        &self,
        player: PlayerId,
        source: ObjectId,
        color: ManaColor,
    ) -> Result<(), EngineError> {
        if self.state.pending.is_some() || self.state.turn.priority != Some(player) {
            return Err(EngineError::illegal(
                "mana ability",
                "mana abilities need priority and cannot be used while a choice is pending (CR 605.3a, 608.2d)",
            ));
        }
        let object = self
            .state
            .try_object(source)
            .ok_or(EngineError::UnknownObject)?;
        if object.zone != Zone::Battlefield || object.controller != player {
            return Err(EngineError::WrongZone {
                name: object.card.name(),
                zone: "battlefield",
            });
        }
        if object.tapped {
            return Err(EngineError::illegal(
                "mana ability",
                format!("{} is already tapped", object.card.name()),
            ));
        }
        if !self.dynamic_mana_colors(player, source).contains(&color) {
            return Err(EngineError::illegal(
                "mana ability",
                format!(
                    "{} cannot produce {} mana here",
                    object.card.name(),
                    color.name()
                ),
            ));
        }
        Ok(())
    }

    fn apply_color_mana_ability(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        color: ManaColor,
        events: &mut Vec<Event>,
    ) {
        if let Some(object) = self.objects_mut(source) {
            object.tapped = true;
        }
        self.state.players[player as usize].mana.add(color, 1);
        self.state.turn.passed = 0;
        events.push(Event::ManaTapped {
            player,
            land: source,
            color,
        });
        events.push(Event::Priority { player });
    }

    fn apply_mana_ability(&mut self, player: PlayerId, land: ObjectId, events: &mut Vec<Event>) {
        let def = self.state.object(land).card.def();
        let source = self
            .state
            .object(land)
            .card
            .creature_mana()
            .map(|color| (color, 1u8))
            .or_else(|| def.basic_mana.map(|color| (color, 1u8)))
            .or(def.artifact_mana);
        let color = source
            .map(|(color, _)| color)
            .unwrap_or(ManaColor::Colorless);
        if let (Some((_, amount)), Some(obj)) = (source, self.objects_mut(land)) {
            obj.tapped = true;
            self.state.players[player as usize]
                .mana
                .add(color, u32::from(amount));
        }
        self.state.turn.passed = 0;
        events.push(Event::ManaTapped {
            player,
            land,
            color,
        });
        events.push(Event::Priority { player });
    }

    // --------------------------------------------------- activated abilities

    /// CR 602.2/602.5: validate one activation against the typed ability record.
    fn saddle_ability_index(&self, card: CardId) -> Option<(u8, u8)> {
        crate::ability::abilities(card)
            .iter()
            .enumerate()
            .find_map(|(index, ability)| {
                ability
                    .activated()
                    .and_then(|activated| activated.cost.saddle_power)
                    .map(|power| (index as u8, power))
            })
    }

    fn validate_saddle_mount(
        &self,
        player: PlayerId,
        source: ObjectId,
        creatures: &[ObjectId],
    ) -> Result<(), EngineError> {
        let object = self
            .state
            .try_object(source)
            .ok_or(EngineError::UnknownObject)?;
        if object.zone != Zone::Battlefield
            || object.controller != player
            || !self.is_creature(object)
        {
            return Err(EngineError::illegal(
                "saddle",
                "only a Mount you control on the battlefield can be saddled (CR 702.171a)",
            ));
        }
        let Some((_, threshold)) = self.saddle_ability_index(object.card) else {
            return Err(EngineError::illegal(
                "saddle",
                "that permanent has no saddle ability",
            ));
        };
        if !self.is_sorcery_timing(player) {
            return Err(EngineError::BadTiming {
                name: object.card.def().name,
                reason: "saddle can be activated only as a sorcery (CR 702.171a)".to_string(),
            });
        }
        if creatures.is_empty() {
            return Err(EngineError::illegal(
                "saddle",
                "at least one other creature must be tapped",
            ));
        }
        let mut total = 0i64;
        let mut seen = Vec::new();
        for id in creatures {
            if seen.contains(id) {
                return Err(EngineError::illegal(
                    "saddle",
                    "a helper creature was named twice",
                ));
            }
            seen.push(*id);
            let Some(helper) = self.state.try_object(*id) else {
                return Err(EngineError::UnknownObject);
            };
            if *id == source
                || helper.zone != Zone::Battlefield
                || !self.is_creature(helper)
                || helper.controller != player
                || helper.tapped
            {
                return Err(EngineError::illegal(
                    "saddle",
                    "helpers must be other untapped creatures you control",
                ));
            }
            total += i64::from(helper.power());
        }
        if total < i64::from(threshold) {
            return Err(EngineError::illegal(
                "saddle",
                format!("helpers have total power {total}, below Saddle {threshold}"),
            ));
        }
        Ok(())
    }

    fn apply_search_library_done(&mut self, player: PlayerId, events: &mut Vec<Event>) {
        let Some(Pending::SearchLibrary {
            chosen,
            mode,
            looked,
            ..
        }) = self.state.pending.take()
        else {
            return;
        };
        // CR 701.20a/e: a "look at the top N" is not a search. Only the looked-at
        // cards move, only they are randomised, and no shuffle happens.
        if mode == crate::state::LibrarySearchMode::CreatureFromTopToBattlefieldAttacking {
            self.finish_look_at_top(chosen.first().copied(), &looked, player, events);
            return;
        }
        for id in chosen {
            match mode {
                crate::state::LibrarySearchMode::BasicLandToBattlefield
                | crate::state::LibrarySearchMode::AnyLandToBattlefield => {
                    self.enter_battlefield(id, player, events, 0);
                    if let Some(object) = self.state.objects.get_mut(&id) {
                        object.tapped = true;
                    }
                    if let Some(object) = self.state.try_object(id) {
                        events.push(Event::StateBasedAction {
                            description: format!(
                                "P{player} puts {} onto the battlefield tapped (CR 701.23)",
                                object.card.def().name
                            ),
                        });
                    }
                }
                crate::state::LibrarySearchMode::SnowLandToHand => {
                    let card = self.state.object(id).card;
                    self.move_to_hand(id, events);
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "P{player} reveals {} and puts it into their hand (CR 701.23)",
                            card.def().name
                        ),
                    });
                }
                // Not reachable: the look mode returns above, before this loop,
                // because it moves the looked-at cards rather than searching.
                crate::state::LibrarySearchMode::CreatureFromTopToBattlefieldAttacking => {}
            }
        }
        self.state
            .rng
            .shuffle(&mut self.state.players[player as usize].library);
        events.push(Event::StateBasedAction {
            description: format!("P{player} shuffles their library after searching (CR 701.23)"),
        });
        self.state.turn.priority = None;
    }

    /// Finish a "look at the top N" pending choice: put the chosen creature card
    /// onto the battlefield tapped and attacking the defending player, then put
    /// the looked-at cards that were not chosen on the bottom in a random order.
    ///
    /// CR 701.20e is the reason this is not `shuffle`: only the looked-at cards
    /// are randomised, and they go under the rest of the library, so a card that
    /// was below the look keeps its position relative to the top. CR 508.1 is the
    /// reason the new object is appended to `turn.attackers` but **not** to
    /// `turn.declared_attackers`: it is attacking, but it was never *declared* as
    /// an attacker, and CR 506.5's "attacks alone" is a fact about the
    /// declaration.
    fn finish_look_at_top(
        &mut self,
        chosen: Option<ObjectId>,
        looked: &[ObjectId],
        player: PlayerId,
        events: &mut Vec<Event>,
    ) {
        if let Some(id) = chosen {
            let card = self.state.object(id).card;
            let defender = self.state.turn.defender;
            self.enter_battlefield(id, player, events, 0);
            if let Some(object) = self.state.objects.get_mut(&id) {
                object.tapped = true;
            }
            if !self.state.turn.attackers.contains(&id) {
                self.state.turn.attackers.push(id);
            }
            events.push(Event::StateBasedAction {
                description: match defender {
                    Some(defender) => format!(
                        "P{player} puts {} onto the battlefield tapped and attacking P{defender} (CR 701.20e, 508.1)",
                        card.def().name
                    ),
                    None => format!(
                        "P{player} puts {} onto the battlefield tapped (CR 701.20e, 508.1)",
                        card.def().name
                    ),
                },
            });
        }
        let mut rest: Vec<ObjectId> = looked
            .iter()
            .copied()
            .filter(|id| Some(*id) != chosen)
            .collect();
        self.state.rng.shuffle(&mut rest);
        let library = &mut self.state.players[player as usize].library;
        library.retain(|id| !rest.contains(id));
        library.extend(rest.iter().copied());
        events.push(Event::StateBasedAction {
            description: format!(
                "P{player} puts the rest of the looked-at cards on the bottom of their library in a random order (CR 701.20e)"
            ),
        });
        self.state.turn.priority = None;
    }

    fn apply_saddle_mount(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        creatures: &[ObjectId],
        events: &mut Vec<Event>,
    ) {
        let Some((index, _)) = self
            .state
            .try_object(source)
            .and_then(|object| self.saddle_ability_index(object.card))
        else {
            return;
        };
        for id in creatures {
            if let Some(object) = self.state.objects.get_mut(id) {
                object.tapped = true;
            }
        }
        let mount_card = self.state.object(source).card;
        events.push(Event::AbilityActivated {
            player,
            source,
            card: mount_card,
            text: "Saddle 1".to_string(),
            targets: Vec::new(),
        });
        events.push(Event::StateBasedAction {
            description: format!(
                "P{player} saddles {} by tapping {} helper(s) (CR 702.171a)",
                self.describe_object(source),
                creatures.len()
            ),
        });
        let card = self.state.object(source).card;
        self.state.stack.push(StackItem {
            object: source,
            card,
            controller: player,
            targets: Vec::new(),
            modes: Vec::new(),
            affected: Some(source),
            dead_object: None,
            source_pt_lki: None,
            search_amount: 0,
            cursor: 0,
            entry: StackEntry::Activated { ability: index },
            trigger_answered: false,
            spell_mana_value: 0,
            x_value: 0,
        });
        self.state.turn.passed = 0;
        self.state.turn.priority = Some(player);
        events.push(Event::Priority { player });
    }

    fn validate_activate_ability(
        &self,
        player: PlayerId,
        request: ActivationRequest<'_>,
    ) -> Result<(), EngineError> {
        let ActivationRequest {
            source,
            ability: index,
            mode,
            targets,
            cost_cards,
        } = request;
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
        if activated.cost.saddle_power.is_some() {
            return Err(EngineError::illegal(
                "activate ability",
                "this is a saddle ability; choose its tapped helpers through SaddleMount (CR 702.171a)".to_string(),
            ));
        }
        // CR 602.2: an ability is activated from the zone its cost can pay for.
        // The pool's only graveyard ability exiles its own card as the cost, so
        // that is the only reason a non-battlefield object is ever offered.
        if activated.cost.exile_source_from_graveyard {
            if object.zone != Zone::Graveyard {
                return Err(EngineError::WrongZone {
                    name: object.card.def().name,
                    zone: "graveyard",
                });
            }
            if object.owner != player {
                return Err(EngineError::illegal(
                    "activate ability",
                    format!(
                        "only {}'s owner may activate it from their graveyard (CR 602.2)",
                        object.card.def().name
                    ),
                ));
            }
        } else {
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
        }
        if self.state.turn.priority != Some(player) {
            return Err(EngineError::BadTiming {
                name: object.card.def().name,
                reason: "that player does not have priority (CR 602.2)".to_string(),
            });
        }
        if activated.sorcery_timing && !self.is_sorcery_timing(player) {
            return Err(EngineError::BadTiming {
                name: object.card.def().name,
                reason: "that ability can be activated only as a sorcery: during its controller's main phase, with the stack empty (CR 307.5)".to_string(),
            });
        }
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
        if activated.cost.tap
            && self.is_creature(object)
            && object.summoning_sick
            && !self.has_haste(object)
        {
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
        self.validate_card_costs(player, object.card, activated.cost, cost_cards)?;
        if object.card == CardId::DragonScarredBear {
            let total: i64 = self
                .state
                .objects
                .values()
                .filter(|o| {
                    o.zone == Zone::Battlefield && o.controller == player && self.is_creature(o)
                })
                .map(|o| i64::from(o.power()))
                .sum();
            if total < 8 {
                return Err(EngineError::illegal(
                    "activate ability",
                    format!(
                        "Formidable requires creatures you control to have total power 8 or greater (current {total}; CR 207.2c; activation restriction 602.1b/602.5)"
                    ),
                ));
            }
        }
        Ok(())
    }

    /// CR 307.5: "Activate only as a sorcery" — the activator's own main phase
    /// with an empty stack.
    fn is_sorcery_timing(&self, player: PlayerId) -> bool {
        self.state.turn.active == player
            && matches!(
                self.state.turn.step,
                crate::state::Step::PrecombatMain | crate::state::Step::PostcombatMain
            )
            && self.state.stack.is_empty()
    }

    /// CR 118.3 + 602.2b: every card the activator named must exist, be unique
    /// in the list, and be in the zone the printed cost reads. Lands discarded
    /// to Ayula's Influence come from the hand and must be land cards; the
    /// cards exiled to Bearscape come from the activator's own graveyard; the
    /// exiling source of Mother Bear is named by the ability, not by the
    /// activator, so it is not part of `cost_cards`.
    fn validate_card_costs(
        &self,
        player: PlayerId,
        card: CardId,
        cost: crate::ability::AbilityCost,
        cost_cards: &[ObjectId],
    ) -> Result<(), EngineError> {
        let required = cost.card_cost_count();
        if cost_cards.len() != required {
            return Err(EngineError::illegal(
                "activate ability",
                format!(
                    "{} costs exactly {required} named card(s), but {} were supplied (CR 118.3)",
                    card.def().name,
                    cost_cards.len()
                ),
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for id in cost_cards {
            if !seen.insert(*id) {
                return Err(EngineError::illegal(
                    "activate ability",
                    format!(
                        "{} names the same card twice, so the cost cannot be paid (CR 118.3)",
                        card.def().name
                    ),
                ));
            }
        }
        let mut index = 0usize;
        if cost.discard_land {
            let id = cost_cards[index];
            index += 1;
            let in_hand = self.state.player(player).hand.contains(&id);
            let Some(object) = self.state.try_object(id) else {
                return Err(EngineError::UnknownObject);
            };
            if !in_hand || object.owner != player {
                return Err(EngineError::illegal(
                    "activate ability",
                    format!(
                        "{} must be a card in {}'s hand to be discarded (CR 118.3)",
                        card.def().name,
                        "the activator"
                    ),
                ));
            }
            if !object.card.def().is_land() {
                return Err(EngineError::illegal(
                    "activate ability",
                    format!(
                        "{} must discard a land card, and {} is not a land (CR 118.3)",
                        card.def().name,
                        object.card.def().name
                    ),
                ));
            }
        }
        let graveyard_cards = cost.exile_graveyard_cards as usize;
        if graveyard_cards > 0 {
            let graveyard = &self.state.player(player).graveyard;
            for id in &cost_cards[index..index + graveyard_cards] {
                if !graveyard.contains(id) {
                    return Err(EngineError::illegal(
                        "activate ability",
                        format!(
                            "{} must exile cards from its activator's own graveyard (CR 118.3)",
                            card.def().name
                        ),
                    ));
                }
            }
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
        request: ActivationRequest<'_>,
        events: &mut Vec<Event>,
    ) {
        let ActivationRequest {
            source,
            ability: index,
            mode: _mode,
            targets,
            cost_cards,
        } = request;
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
        // CR 602.2b: the printed card costs are paid as part of activating.
        // The named cards are already validated, so this only moves them.
        let mut paid = 0usize;
        if activated.cost.discard_land {
            self.discard_as_cost(player, cost_cards[paid], events);
            paid += 1;
        }
        let graveyard_cards = activated.cost.exile_graveyard_cards as usize;
        for id in &cost_cards[paid..paid + graveyard_cards] {
            self.move_to_exile(*id, events);
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
        // CR 602.2b + 118.3: "Exile this card from your graveyard" is paid with
        // the source itself, so the source has already left the graveyard when
        // the ability is put on the stack. The stack entry keeps the card id,
        // exactly like a sacrificed source, so resolution never needs the
        // object to still exist.
        if activated.cost.exile_source_from_graveyard {
            self.move_to_exile(source, events);
        }
        let attach = activated.effects.contains(&Effect::AttachSourceToTarget);
        self.state.stack.push(StackItem {
            object: source,
            card,
            controller: player,
            targets: targets.to_vec(),
            modes: Vec::new(),
            affected: attach.then_some(source),
            dead_object: None,
            source_pt_lki: None,
            search_amount: 0,
            cursor: 0,
            entry: StackEntry::Activated { ability: index },
            trigger_answered: false,
            spell_mana_value: 0,
            x_value: 0,
        });
        // CR 117.3c: activating is treated like casting for priority purposes.
        self.state.turn.passed = 0;
        self.state.turn.priority = Some(player);
        events.push(Event::Priority { player });
    }

    // --------------------------------------------------- triggered abilities

    /// CR 507.1: queue every "at the beginning of combat on your turn" trigger
    /// of the active player's battlefield permanents. The step exists once per
    /// turn, so each such permanent triggers exactly once per turn.
    fn queue_beginning_of_combat_triggers(&mut self, active: PlayerId, _events: &mut Vec<Event>) {
        let sources: Vec<(ObjectId, CardId, PlayerId)> = self
            .state
            .objects
            .values()
            .filter(|object| {
                object.zone == Zone::Battlefield
                    && object.controller == active
                    && crate::ability::triggers_on(object.card, TriggerEvent::BeginningOfCombat)
                        .next()
                        .is_some()
            })
            .map(|object| (object.id, object.card, object.controller))
            .collect();
        for (id, card, controller) in sources {
            self.queue_triggers(id, card, controller, TriggerEvent::BeginningOfCombat);
        }
    }

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
        if card == CardId::UrsineMonstrosity && event == TriggerEvent::BeginningOfCombat {
            let seq = self.state.next_trigger_seq;
            self.state.next_trigger_seq += 1;
            self.state.trigger_queue.push(TriggerRef {
                source,
                card,
                controller,
                ability: 0,
                seq,
                spell_mana_value: 0,
                affected: Some(source),
                source_pt_lki: None,
                search_amount: 0,
                dead_object: None,
                x_value: 0,
            });
            return;
        }
        if event == TriggerEvent::EntersBattlefield
            && card == CardId::UlvenwaldBear
            && !self.state.creature_died_this_turn
        {
            return;
        }
        if card == CardId::AyulaQueenAmongBears && event == TriggerEvent::AnotherCreatureEnters {
            let seq = self.state.next_trigger_seq;
            self.state.next_trigger_seq += 1;
            self.state.trigger_queue.push(TriggerRef {
                source,
                card,
                controller,
                ability: 0,
                seq,
                spell_mana_value: 0,
                affected: None,
                dead_object: None,
                source_pt_lki: None,
                search_amount: 0,
                x_value: 0,
            });
            return;
        }
        for (index, _) in ability::triggers_on(card, event) {
            let seq = self.state.next_trigger_seq;
            self.state.next_trigger_seq += 1;
            self.state.trigger_queue.push(TriggerRef {
                source,
                card,
                controller,
                ability: index,
                seq,
                spell_mana_value: 0,
                affected: None,
                dead_object: None,
                source_pt_lki: None,
                search_amount: 0,
                x_value: 0,
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
            if trigger.card == CardId::AyulaQueenAmongBears {
                self.stage_ayula_trigger(trigger, queue, events);
                return;
            }
            let spec = ability::abilities(trigger.card)
                .get(trigger.ability as usize)
                .and_then(Ability::triggered)
                .map(|triggered| triggered.target)
                .unwrap_or(TargetSpec::None);
            // CR 702.94a: soulbond's trigger reads "if this creature is
            // unpaired", so a source that is already in a pair has nothing to
            // pair with and the trigger does nothing.
            if trigger.card == CardId::DruidSFamiliar && self.paired_with(trigger.source).is_some()
            {
                events.push(Event::TriggerRemoved {
                    controller: trigger.controller,
                    card: trigger.card,
                    reason: "its source is already paired (CR 702.94a)".to_string(),
                });
                continue;
            }
            if spec == TargetSpec::ArtifactOrEnchantment {
                self.stage_aggregate_trigger(trigger, queue, events);
                return;
            }
            if spec == TargetSpec::None {
                self.push_trigger(trigger, Vec::new(), events);
                continue;
            }
            let legal = self.legal_targets(trigger.controller, spec, trigger.source);
            let optional = matches!(
                spec,
                TargetSpec::UpToOneCreatureYouControl
                    | TargetSpec::UnpairedAnotherCreatureYouControl
            );
            if optional {
                // CR 115.6: "up to one" is a real choice even when legal
                // targets exist — the controller may decline it — so the
                // engine always asks instead of choosing for them.
                if legal.is_empty() {
                    self.push_trigger(trigger, Vec::new(), events);
                    continue;
                }
                self.state.pending = Some(Pending::TriggerTarget {
                    player: trigger.controller,
                    trigger,
                    legal,
                    queued: queue,
                    first: None,
                    aggregate: None,
                    chosen: Vec::new(),
                });
                self.state.turn.priority = None;
                return;
            }
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
                        first: None,
                        aggregate: None,
                        chosen: Vec::new(),
                    });
                    self.state.turn.priority = None;
                    return;
                }
            }
        }
    }

    /// CR 601.2c: a triggered ability printed as "destroy any number of target
    /// artifacts and/or enchantments with total mana value X or less" chooses a
    /// *set* of targets. The budget is the X the trigger recorded (CR 107.3a),
    /// only targets that fit inside it are offered, and the empty set is a legal
    /// answer — "any number" includes zero, so the trigger is never removed for
    /// want of targets.
    fn stage_aggregate_trigger(
        &mut self,
        trigger: TriggerRef,
        queued: Vec<TriggerRef>,
        events: &mut Vec<Event>,
    ) {
        let budget = trigger.x_value;
        let legal: Vec<Target> = self
            .legal_targets(
                trigger.controller,
                TargetSpec::ArtifactOrEnchantment,
                trigger.source,
            )
            .into_iter()
            .filter(|target| self.target_mana_value(*target) <= u32::from(budget))
            .collect();
        if legal.is_empty() {
            self.push_trigger(trigger, Vec::new(), events);
            self.stage_triggers(queued, events);
            return;
        }
        self.state.pending = Some(Pending::TriggerTarget {
            player: trigger.controller,
            trigger,
            legal,
            queued,
            first: None,
            aggregate: Some(budget),
            chosen: Vec::new(),
        });
        self.state.turn.priority = None;
    }

    fn stage_ayula_trigger(
        &mut self,
        mut trigger: TriggerRef,
        queued: Vec<TriggerRef>,
        events: &mut Vec<Event>,
    ) {
        let counters = self.legal_targets(trigger.controller, TargetSpec::Bear, trigger.source);
        let fight_targets = self.legal_targets(
            trigger.controller,
            TargetSpec::BearYouControl,
            trigger.source,
        );
        let opponents = self.legal_targets(
            trigger.controller,
            TargetSpec::OpponentCreature,
            trigger.source,
        );
        let counter_ok = !counters.is_empty();
        let fight_ok = !fight_targets.is_empty() && !opponents.is_empty();
        match (counter_ok, fight_ok) {
            (false, false) => {
                events.push(Event::TriggerRemoved {
                    controller: trigger.controller,
                    card: trigger.card,
                    reason: "neither mode has legal targets (CR 603.3c, 700.2b)".to_string(),
                });
                self.stage_triggers(queued, events);
            }
            (true, false) => {
                trigger.ability = 0;
                self.stage_ayula_target(trigger, queued, 0, events);
            }
            (false, true) => {
                trigger.ability = 1;
                self.stage_ayula_target(trigger, queued, 1, events);
            }
            (true, true) => {
                self.state.pending = Some(Pending::TriggerMode {
                    player: trigger.controller,
                    trigger,
                    queued,
                });
                self.state.turn.priority = None;
            }
        }
    }

    fn stage_ayula_target(
        &mut self,
        trigger: TriggerRef,
        queued: Vec<TriggerRef>,
        mode: u8,
        events: &mut Vec<Event>,
    ) {
        let spec = if mode == 0 {
            TargetSpec::Bear
        } else {
            TargetSpec::BearYouControl
        };
        let legal = self.legal_targets(trigger.controller, spec, trigger.source);
        match legal.len() {
            0 => self.stage_triggers(queued, events),
            1 => {
                if mode == 1 {
                    self.ask_ayula_second(trigger, legal[0], queued, events);
                } else {
                    self.push_trigger(trigger, vec![legal[0]], events);
                    self.stage_triggers(queued, events);
                }
            }
            _ => {
                self.state.pending = Some(Pending::TriggerTarget {
                    player: trigger.controller,
                    trigger,
                    legal,
                    queued,
                    first: None,
                    aggregate: None,
                    chosen: Vec::new(),
                });
                self.state.turn.priority = None;
            }
        }
    }

    fn ask_ayula_second(
        &mut self,
        trigger: TriggerRef,
        first: Target,
        queued: Vec<TriggerRef>,
        events: &mut Vec<Event>,
    ) {
        let legal = self.legal_targets(
            trigger.controller,
            TargetSpec::OpponentCreature,
            trigger.source,
        );
        if legal.is_empty() {
            self.stage_triggers(queued, events);
            return;
        }
        if legal.len() == 1 {
            self.push_trigger(trigger, vec![first, legal[0]], events);
            self.stage_triggers(queued, events);
            return;
        }
        self.state.pending = Some(Pending::TriggerTarget {
            player: trigger.controller,
            trigger,
            legal,
            queued,
            first: Some(first),
            aggregate: None,
            chosen: Vec::new(),
        });
        self.state.turn.priority = None;
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
            modes: Vec::new(),
            affected: trigger.affected,
            dead_object: trigger.dead_object,
            source_pt_lki: trigger.source_pt_lki,
            search_amount: trigger.search_amount,
            cursor: 0,
            entry: StackEntry::Triggered {
                ability: trigger.ability,
            },
            trigger_answered: false,
            spell_mana_value: trigger.spell_mana_value,
            x_value: trigger.x_value,
        });
        self.state.turn.passed = 0;
    }

    fn apply_trigger_target(&mut self, _player: PlayerId, target: Target, events: &mut Vec<Event>) {
        let Some(Pending::TriggerTarget {
            trigger,
            queued,
            first,
            legal,
            aggregate,
            mut chosen,
            ..
        }) = self.state.pending.take()
        else {
            return;
        };
        // CR 601.2c: a set-targeting trigger stays open after each addition so
        // the controller can keep choosing; it is committed by
        // `Action::FinishTriggerTargets`.
        if let Some(budget) = aggregate {
            chosen.push(target);
            self.state.pending = Some(Pending::TriggerTarget {
                player: trigger.controller,
                trigger,
                legal,
                queued,
                first: None,
                aggregate: Some(budget),
                chosen,
            });
            self.state.turn.priority = None;
            return;
        }
        if trigger.card == CardId::AyulaQueenAmongBears {
            if trigger.ability == 1 && first.is_none() {
                self.ask_ayula_second(trigger, target, queued, events);
            } else if let Some(first) = first {
                self.push_trigger(trigger, vec![first, target], events);
                self.stage_triggers(queued, events);
            } else {
                self.push_trigger(trigger, vec![target], events);
                self.stage_triggers(queued, events);
            }
            return;
        }
        self.push_trigger(trigger, vec![target], events);
        self.stage_triggers(queued, events);
    }

    /// CR 601.2c: commit the chosen set for a set-targeting trigger and put the
    /// ability on the stack with exactly those targets. An empty set is legal.
    fn apply_finish_trigger_targets(&mut self, _player: PlayerId, events: &mut Vec<Event>) {
        // The action is validated before it is applied, but the guard is on the
        // state itself: a commit is meaningful only while a *set* choice is open,
        // and anything else leaves the pending untouched.
        if !matches!(
            self.state.pending,
            Some(Pending::TriggerTarget {
                aggregate: Some(_),
                ..
            })
        ) {
            return;
        }
        let Some(Pending::TriggerTarget {
            trigger,
            queued,
            chosen,
            ..
        }) = self.state.pending.take()
        else {
            return;
        };
        self.push_trigger(trigger, chosen, events);
        self.stage_triggers(queued, events);
    }

    /// CR 202.3: the mana value a "total mana value X or less" restriction reads
    /// for a target. Only a battlefield permanent is ever such a target, and its
    /// mana value is the printed one with `{X}` read as 0.
    fn target_mana_value(&self, target: Target) -> u32 {
        match target {
            Target::Permanent(id) => u32::from(
                self.state
                    .try_object(id)
                    .map_or(0, |object| object.card.def().mana_value()),
            ),
            _ => 0,
        }
    }

    /// CR 115.6: the controller of a triggered ability whose printed target is
    /// "up to one" may put it on the stack with no target. The ability still
    /// resolves; only the per-target instructions do nothing.
    fn apply_trigger_target_none(&mut self, _player: PlayerId, events: &mut Vec<Event>) {
        let Some(Pending::TriggerTarget {
            trigger, queued, ..
        }) = self.state.pending.take()
        else {
            return;
        };
        self.push_trigger(trigger, Vec::new(), events);
        self.stage_triggers(queued, events);
    }

    /// CR 603.5: the controller decides when an optional trigger resolves.
    fn apply_trigger_choice(
        &mut self,
        player: PlayerId,
        use_ability: bool,
        events: &mut Vec<Event>,
    ) {
        match self.state.pending.take() {
            Some(Pending::TriggerMode {
                trigger, queued, ..
            }) => {
                let mode = u8::from(!use_ability);
                let mut trigger = trigger;
                trigger.ability = mode;
                self.stage_ayula_target(trigger, queued, mode, events);
                self.state.turn.priority = None;
            }
            Some(Pending::TopCardChoice {
                player: owner,
                card,
                can_take,
            }) => {
                let take = use_ability && can_take;
                if take {
                    let library = &mut self.state.players[owner as usize].library;
                    if library.first() == Some(&card) {
                        library.remove(0);
                    }
                    let card_id = self.state.object(card).card;
                    if let Some(object) = self.state.objects.get_mut(&card) {
                        object.zone = Zone::Hand;
                    }
                    self.state.players[owner as usize].hand.push(card);
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "P{owner} reveals {} and puts it into their hand",
                            card_id.name()
                        ),
                    });
                } else {
                    let library = &mut self.state.players[owner as usize].library;
                    if library.first() == Some(&card) {
                        library.remove(0);
                        library.push(card);
                    }
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "P{owner} puts the looked-at card on the bottom of their library"
                        ),
                    });
                }
                self.state.turn.priority = None;
            }
            Some(Pending::TriggerChoice { trigger, .. }) => {
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
            _ => {}
        }
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
        x: u8,
        modes: &[u8],
        targets: &[Target],
        events: &mut Vec<Event>,
    ) {
        let (card_id, from_command_zone, commander_id) = {
            let obj = self.state.object(card);
            (obj.card, obj.zone == Zone::Command, obj.commander)
        };
        let cost = self
            .effective_cost_parts(card, x)
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
            modes: modes.to_vec(),
            affected: None,
            dead_object: None,
            source_pt_lki: None,
            search_amount: 0,
            cursor: 0,
            entry: StackEntry::Spell,
            trigger_answered: false,
            spell_mana_value: 0,
            x_value: x,
        });
        // CR 603.6a: "you cast" triggers are queued as the spell is cast, and
        // reach the stack above it before priority. Only the caster's
        // permanents trigger; an opponent's cast does not.
        //
        // CR 202.3 + 603.2: two printed wordings are kept apart. "You cast an
        // instant or sorcery spell" fires on instants and sorceries only, while
        // "you cast a creature spell" fires on creature spells only; a
        // noncreature permanent spell fires neither.
        let def = card_id.def();
        let cast_event = if def.kinds.contains(&crate::card::Kind::Instant)
            || def.kinds.contains(&crate::card::Kind::Sorcery)
        {
            Some(TriggerEvent::SpellCast)
        } else if def.kinds.contains(&crate::card::Kind::Creature) {
            Some(TriggerEvent::CreatureSpellCast)
        } else {
            None
        };
        if let Some(event) = cast_event {
            let sources: Vec<(ObjectId, CardId)> = self
                .state
                .objects
                .values()
                .filter(|object| object.zone == Zone::Battlefield && object.controller == player)
                .filter(|object| ability::triggers_on(object.card, event).next().is_some())
                .map(|object| (object.id, object.card))
                .collect();
            for (source, card) in sources {
                let first_new = self.state.trigger_queue.len();
                self.queue_triggers(source, card, player, event);
                if event == TriggerEvent::CreatureSpellCast {
                    // CR 202.3: the triggering spell's mana value is fixed now,
                    // not at resolution, so it travels with the trigger. An
                    // announced X is part of the mana value while the spell is
                    // on the stack.
                    let mana_value = def.mana_value().saturating_add(x);
                    for trigger in &mut self.state.trigger_queue[first_new..] {
                        trigger.spell_mana_value = mana_value;
                    }
                }
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
        // CR 117.3c: the caster receives priority after casting — unless a cast
        // trigger is waiting for a target (CR 603.3d), in which case the choice
        // is the next thing the rules do and the engine must not hold priority
        // alongside a pending choice.
        self.state.turn.passed = 0;
        if self.state.pending.is_none() {
            self.state.turn.priority = Some(player);
            events.push(Event::Priority { player });
        }
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
                // CR 702.20a: vigilance replaces the tap an attack would
                // otherwise cause (CR 508.1f). Nothing else about the attack
                // changes, so a vigilant creature can also block next turn.
                if !obj.has_keyword(crate::tokens::Keyword::Vigilance) {
                    obj.tapped = true; // CR 508.1f
                }
            }
        }
        self.state.turn.attackers = attackers.to_vec();
        // CR 506.5: the declaration record. Unlike `attackers`, this list is not
        // pruned when a creature leaves combat, so "attacks alone" keeps meaning
        // "was the only creature declared" for the rest of the combat.
        self.state.turn.declared_attackers = attackers.to_vec();
        self.state.turn.defender = defender;
        self.state.turn.passed = 0;
        events.push(Event::AttackersDeclared {
            player,
            attackers: attackers.to_vec(),
        });
        // CR 508.1 + 603.2: "whenever this creature attacks" triggers fire as
        // attackers are declared. The printed intervening-if (CR 603.4) is
        // evaluated here, before the trigger is queued, so a false clause never
        // reaches the stack; resolution re-checks it against the same defender.
        if let Some(defender) = defender {
            for id in attackers {
                let Some(object) = self.state.try_object(*id) else {
                    continue;
                };
                let (card, controller) = (object.card, object.controller);
                if card == CardId::TheEarthKing {
                    continue;
                }
                // Cub qualifies the attack event, not a resolution-time "if".
                if card == CardId::OwlbearCub
                    && self
                        .state
                        .objects
                        .values()
                        .filter(|object| {
                            object.zone == Zone::Battlefield
                                && object.controller == defender
                                && object.card.def().is_land()
                        })
                        .count()
                        < 8
                {
                    continue;
                }
                if ability::triggers_on(card, TriggerEvent::Attacks)
                    .next()
                    .is_none()
                {
                    continue;
                }
                if !self.trigger_condition_holds(card, defender, *id) {
                    events.push(Event::TriggerRemoved {
                        controller,
                        card,
                        reason: "the printed intervening-if condition is false as \
                                 attackers are declared (CR 603.4)"
                            .to_string(),
                    });
                    continue;
                }
                self.queue_triggers(*id, card, controller, TriggerEvent::Attacks);
            }
            let qualifying = attackers
                .iter()
                .filter_map(|id| self.state.try_object(*id))
                .filter(|object| {
                    object.controller == player && self.is_creature(object) && object.power() >= 4
                })
                .count() as u8;
            if qualifying > 0 {
                let kings: Vec<(ObjectId, PlayerId)> = self
                    .state
                    .objects
                    .values()
                    .filter(|object| {
                        object.zone == Zone::Battlefield
                            && object.controller == player
                            && object.card == CardId::TheEarthKing
                    })
                    .map(|object| (object.id, object.controller))
                    .collect();
                for (source, controller) in kings {
                    let seq = self.state.next_trigger_seq;
                    self.state.next_trigger_seq += 1;
                    self.state.trigger_queue.push(TriggerRef {
                        source,
                        card: CardId::TheEarthKing,
                        controller,
                        ability: 1,
                        seq,
                        spell_mana_value: 0,
                        affected: None,
                        dead_object: None,
                        source_pt_lki: None,
                        search_amount: qualifying,
                        x_value: 0,
                    });
                }
            }
            self.put_triggers(events);
        }
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
        self.state.turn.unpaired_blockers.clear();
        self.state.turn.blocked = blocks.iter().map(|b| b.attacker).collect();
        {
            // CR 506.5: "blocks alone" counts distinct blocking creatures, so a
            // single blocker blocking two attackers is still one blocker. The
            // record is not pruned when a blocker leaves combat.
            let mut declared: Vec<ObjectId> = blocks.iter().map(|b| b.blocker).collect();
            declared.sort_unstable();
            declared.dedup();
            self.state.turn.declared_blockers = declared;
        }
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

    /// Discard one card from hand as an activation cost (CR 118.3, 602.2b).
    /// Unlike the cleanup discard this never touches pending state: the card
    /// simply moves to the graveyard and the activation continues.
    fn discard_as_cost(&mut self, player: PlayerId, card: ObjectId, events: &mut Vec<Event>) {
        let Some(mut object) = self.detach_object(card) else {
            return;
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
            // CR 514/500.5: history belongs to the turn. Cleanup is still that
            // same turn even when it repeats after SBAs or priority, so reset
            // only when the turn number actually advances.
            self.state.creature_died_this_turn = false;
            self.state.nontoken_creature_deaths_this_turn.clear();
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
            self.state.turn.declared_attackers.clear();
            self.state.turn.declared_blockers.clear();
            self.state.turn.unpaired_blockers.clear();
            self.state.turn.combat_damage_done = false;
            self.state.turn.defender = None;
            // CR 602.5b: "activate only once each turn" limits reset with the
            // new turn.
            self.state.activation_uses.clear();
            // CR 611.3a + 613.1d/613.4b: a printed effect that applies "during
            // your turn" starts or stops applying when the active player
            // changes, so the layer recomputation has to run here — otherwise
            // a permanent whose base power/toughness depends on whose turn it
            // is would keep the previous turn's values.
            self.recompute_layers();
            let (turn, active) = (self.state.turn.number, self.state.turn.active);
            events.push(Event::TurnBegin { turn, active });
        }
        self.state.turn.step = next;
        self.state.turn.priority = None;
        self.state.turn.passed = 0;
        events.push(Event::StepBegin { step: next });
        let active = self.state.turn.active;
        match next {
            Step::Untap => self.perform_untap(active, events),
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
            Step::BeginCombat => {
                // CR 507.1 + 603.6a: "at the beginning of combat on your turn"
                // triggers fire as the step begins, before any player receives
                // priority. Only the active player's permanents can trigger,
                // and the step happens once per turn, so they fire once.
                self.queue_beginning_of_combat_triggers(active, events);
                self.put_triggers(events);
                if self.state.pending.is_none() && !self.state.is_over() {
                    self.grant_priority(active, events);
                }
            }
            Step::PrecombatMain | Step::PostcombatMain | Step::EndStep => {
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
                //
                // CR 510.1: a printed permission ("you may have that creature
                // assign its combat damage as though it weren't blocked") turns
                // the automatic assignment into a choice, so ask first and deal
                // the whole batch from the answer path.
                if self.begin_combat_damage_assignment() {
                    return;
                }
                self.deal_combat_damage(&[], events);
                self.check_sbas(events);
                if self.state.pending.is_some() || self.state.is_over() {
                    return;
                }
                self.grant_priority(active, events);
            }
            Step::EndCombat => {
                self.state
                    .effects
                    .retain(|effect| effect.kind != ContinuousKind::MustAttackThisCombat);
                self.state.turn.attackers.clear();
                self.state.turn.blocked.clear();
                self.state.turn.blockers.clear();
                self.state.turn.declared_attackers.clear();
                self.state.turn.declared_blockers.clear();
                self.state.turn.unpaired_blockers.clear();
                self.state.turn.defender = None;
                self.grant_priority(active, events);
            }
            Step::Cleanup => self.perform_cleanup(events),
        }
    }

    fn perform_untap(&mut self, active: PlayerId, events: &mut Vec<Event>) {
        // CR 502.3: an effect can keep a permanent tapped through its
        // controller's untap step. The marker is one-shot: this step consumes
        // it, so the permanent untaps normally a turn later. Markers belonging
        // to another seat stay queued, and markers for objects that already left
        // the battlefield are dropped instead of being carried forever.
        let pending = std::mem::take(&mut self.state.skip_next_untap);
        let mut kept = std::collections::BTreeSet::new();
        let mut skipped = Vec::new();
        for obj in self.state.objects.values_mut() {
            if obj.zone != Zone::Battlefield {
                continue;
            }
            if obj.controller != active {
                if pending.contains(&obj.id) {
                    kept.insert(obj.id);
                }
                continue;
            }
            if pending.contains(&obj.id) {
                // CR 502.3 + 302.6: it stays tapped, but it is not summoning
                // sick — it has been under its controller's control since the
                // turn began, so only its tapped state is preserved.
                obj.summoning_sick = false;
                skipped.push(obj.id);
                continue;
            }
            obj.tapped = false;
            // CR 302.6: the restriction ends for the permanent, whether or not
            // it is currently a creature (Stuffed Bear can be animated on a
            // later turn and attack; an artifact animated on the turn it
            // arrived stays sick until this step).
            obj.summoning_sick = false;
        }
        self.state.skip_next_untap = kept;
        for id in skipped {
            let name = self.state.object(id).name().to_string();
            events.push(Event::StateBasedAction {
                description: format!(
                    "{name} doesn't untap during this untap step (CR 502.3, Spectral Bears)"
                ),
            });
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
        let expired_regeneration = self.state.regeneration_shields.len();
        self.state.regeneration_shields.clear();
        if expired_regeneration > 0 {
            changed = true;
        }
        // CR 514.2: an unspent "the next time you would draw a card this turn"
        // replacement ends with the turn.
        if self.state.draw_replacements.iter().any(|count| *count > 0) {
            changed = true;
        }
        self.state.draw_replacements.clear();
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
                obj.plus_one_counters = 0;
                obj.counters.clear();
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

        self.prune_combat();

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
                .filter(|o| o.zone == Zone::Battlefield && self.is_creature(o))
                .filter(|o| {
                    o.toughness() <= 0
                        || (o.damage as i32 >= o.toughness() && !self.has_indestructible(o.id))
                })
                .map(|o| o.id)
                .collect();
            if !dying.is_empty() {
                // CR 704.3 + 603.6c: the whole batch is one event. Snapshot the
                // watchers before anything leaves the battlefield and dispatch
                // once for every death, so the result does not depend on which
                // dying creature happens to be moved first.
                let watchers = death_watchers(self);
                let combat = self.capture_combat_alone();
                let mut deaths: Vec<DeathRecord> = Vec::new();
                let mut regenerated = 0usize;
                for id in &dying {
                    let lethal_now = self.state.try_object(*id).is_some_and(|o| {
                        o.zone == Zone::Battlefield
                            && self.is_creature(o)
                            && o.toughness() > 0
                            && o.damage as i32 >= o.toughness()
                            && !self.has_indestructible(*id)
                    });
                    if lethal_now && self.try_regenerate(*id, events) {
                        regenerated += 1;
                        continue;
                    }
                    if let Some(record) = self.move_to_graveyard_inner(*id, events, &combat) {
                        deaths.push(record);
                    }
                }
                self.queue_watchers_for_deaths(&watchers, &deaths);
                events.push(Event::StateBasedAction {
                    description: if regenerated == 0 {
                        format!(
                            "{} creature(s) had lethal damage or toughness 0 and were put into the graveyard (CR 704.5f, 704.5g)",
                            dying.len()
                        )
                    } else {
                        format!(
                            "{} creature(s) had lethal damage or toughness 0; {} regenerated, {} were put into the graveyard (CR 704.5f, 704.5g)",
                            dying.len(), regenerated, deaths.len()
                        )
                    },
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
        item.targets
            .iter()
            .all(|target| self.target_is_legal(*target, item))
    }

    /// CR 608.2b: an ability whose targets are *all* illegal does not resolve.
    /// An ability with no targets at all is not "all illegal" — it resolves and
    /// does nothing, which is how an empty "destroy any number" set behaves.
    fn targets_all_illegal(&self, item: &StackItem) -> bool {
        !item.targets.is_empty()
            && item
                .targets
                .iter()
                .all(|target| !self.target_is_legal(*target, item))
    }

    fn target_is_legal(&self, target: Target, item: &StackItem) -> bool {
        match target {
            Target::Player(p) => self.state.is_live(p),
            Target::Permanent(id) => self.state.try_object(id).is_some_and(|o| {
                // CR 115.1: a permanent target must still be a permanent this
                // instruction can name. An artifact/enchantment target is not a
                // creature, so the check is by the zone plus the printed card
                // type the spec reads.
                o.zone == Zone::Battlefield
                    && if trigger_targets_a_set(item.card, ability_index(item.entry)) {
                        o.card.def().is_artifact() || o.card.def().is_enchantment()
                    } else {
                        self.is_creature(o)
                    }
            }),
            // CR 608.2b: the target is checked against the zone the printed
            // instruction names. A card that left the graveyard (or that
            // gained an ability in the meantime) is no longer legal.
            Target::Card(id) => self.state.try_object(id).is_some_and(|o| {
                o.zone == Zone::Graveyard
                    && o.owner == item.controller
                    && o.card.def().is_creature()
                    && self.state.object_has_no_abilities(id)
            }),
            Target::Spell(id) => self
                .state
                .stack
                .iter()
                .any(|s| s.object == id && s.entry == StackEntry::Spell),
        }
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

            let modal_effects: Vec<Effect> =
                if let Some(table) = crate::card::modal_spell_effects(card) {
                    let mut selected = item.modes.clone();
                    selected.sort_unstable();
                    selected
                        .into_iter()
                        .filter_map(|mode| table.get(mode as usize))
                        .flat_map(|effects| effects.iter().copied())
                        .collect()
                } else {
                    Vec::new()
                };
            let effects: &[Effect] = if modal_effects.is_empty() {
                spec.effects
            } else {
                &modal_effects
            };
            if item.cursor as usize >= effects.len() {
                events.push(Event::Resolve {
                    controller: item.controller,
                    object: item.object,
                    card,
                });
                if def.is_permanent_spell() {
                    // CR 608.3: a resolving permanent spell enters the battlefield.
                    self.enter_battlefield(item.object, item.controller, events, item.x_value);
                } else {
                    self.move_to_graveyard(item.object, events);
                }
                self.state.resolving = None;
                self.finish_resolution(events);
                return;
            }

            let effect = effects[item.cursor as usize];
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
                    spell_mana_value: item.spell_mana_value,
                    affected: item.affected,
                    dead_object: None,
                    source_pt_lki: item.source_pt_lki,
                    search_amount: item.search_amount,
                    x_value: item.x_value,
                },
            });
            self.state.resolving = Some(item.object);
            self.state.turn.priority = None;
            return;
        }

        // CR 608.2b: legality is checked once, before instructions are followed.
        let all_illegal = self.targets_all_illegal(&item);
        if item.cursor == 0
            && if trigger_targets_a_set(item.card, ability_index(item.entry)) {
                // CR 608.2b: a set-targeting ability resolves as long as at
                // least one of its targets is still legal; each illegal target
                // simply does nothing.
                all_illegal
            } else {
                !self.targets_legal(&item)
            }
        {
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

    /// Kamahl's Summons: every live seat answers in APNAP order from the active
    /// player (CR 101.4, 608.2f). Nothing is applied per answer; the batch runs
    /// once the last seat has revealed.
    fn begin_apnap_reveal(&mut self) {
        let anchor = self.state.turn.active;
        let eligible: Vec<PlayerId> = self.state.live_seats().collect();
        if eligible.is_empty() {
            return;
        }
        let order = self.apnap_order_from(anchor, &eligible);
        if let Some(player) = order.first().copied() {
            self.state.pending = Some(Pending::ApnapReveal {
                player,
                order,
                revealed: BTreeMap::new(),
            });
        }
    }

    /// Record one seat's reveal and advance the queue, or apply the batch.
    fn apply_apnap_reveal(
        &mut self,
        player: PlayerId,
        cards: &[ObjectId],
        events: &mut Vec<Event>,
    ) {
        let Some(Pending::ApnapReveal {
            mut order,
            mut revealed,
            ..
        }) = self.state.pending.take()
        else {
            return;
        };
        events.push(Event::CardsRevealed {
            player,
            cards: cards.to_vec(),
            count: u8::try_from(cards.len()).unwrap_or(u8::MAX),
        });
        if order.first() == Some(&player) {
            order.remove(0);
        }
        revealed.insert(player, cards.to_vec());
        match order.first().copied() {
            Some(next) => {
                self.state.pending = Some(Pending::ApnapReveal {
                    player: next,
                    order,
                    revealed,
                });
            }
            None => self.apply_apnap_reveal_batch(&revealed, events),
        }
    }

    /// CR 608.2f: after every seat has answered, each seat creates one 2/2 green
    /// Bear token per card it revealed, in APNAP order from the active player.
    /// The cards themselves never leave their owner's hand (CR 701.9a reveals,
    /// it does not move).
    fn apply_apnap_reveal_batch(
        &mut self,
        revealed: &BTreeMap<PlayerId, Vec<ObjectId>>,
        events: &mut Vec<Event>,
    ) {
        let anchor = self.state.turn.active;
        let eligible: Vec<PlayerId> = revealed.keys().copied().collect();
        for seat in self.apnap_order_from(anchor, &eligible) {
            let Some(cards) = revealed.get(&seat) else {
                continue;
            };
            if cards.is_empty() || !self.state.is_live(seat) {
                continue;
            }
            let count = cards.len();
            self.create_tokens(seat, CardId::BearToken, count, events);
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
                        // A graveyard card is not a damage recipient; the arm
                        // exists only for exhaustiveness (CR 115.4).
                        Target::Card(_) => {}
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
                        // A graveyard card is not a permanent and never a
                        // damage recipient, so this arm cannot be reached by a
                        // legal action (CR 115.4).
                        Target::Card(_) => continue,
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
            // CR 101.4 + 701.9a: "Each player may reveal any number of creature
            // cards from their hand" is a per-seat choice made in APNAP order;
            // the tokens wait for every answer (608.2f).
            Effect::EachPlayerMayRevealCreatureCardsForBears => {
                self.begin_apnap_reveal();
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
            // CR 400.7 + 608.2c: a card returned from a graveyard to its owner's
            // hand moves through the same zone-change path as every other move,
            // so it never travels the draw path (which would emit `Event::Draw`
            // and set `drew_from_empty`) and never fires a draw trigger. A
            // commander card is offered its CR 903.9b command-zone replacement
            // first, exactly like a battlefield permanent returned to hand.
            Effect::ReturnGraveyardCardToHand => {
                for target in &item.targets {
                    if let Target::Card(id) = target {
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
            // CR 613.1d: "becomes a Bear Berserker until end of turn". A layer 4
            // continuous effect replaces the subtype line for as long as it
            // applies; cleanup removes it and the printed line returns
            // (CR 514.2). It is created only while the source is a permanent,
            // because the effect applies to that object and a source that has
            // left the battlefield cannot keep applying it.
            Effect::SetSourceCreatureSubtypes(line) => {
                if self
                    .state
                    .try_object(item.object)
                    .is_some_and(|object| object.zone == Zone::Battlefield)
                {
                    let timestamp = self.next_timestamp();
                    let id = self.next_effect_id();
                    self.state.effects.push(ContinuousEffect {
                        id,
                        timestamp,
                        source: Some(item.object),
                        card: item.card,
                        controller,
                        layer: Layer::Type,
                        target: item.object,
                        kind: ContinuousKind::SetCreatureSubtypes(line),
                        duration: EffectDuration::UntilEndOfTurn,
                    });
                    events.push(Event::ContinuousEffectStarted {
                        object: item.object,
                        card: item.card,
                        layer: Layer::Type.label().to_string(),
                        text: format!(
                            "becomes a {} until end of turn ({})",
                            line.text(),
                            Layer::Type.rules()
                        ),
                    });
                }
                self.recompute_layers();
                false
            }
            // CR 613.1d/613.1e/613.4b: Stuffed Bear's animation. One printed
            // ability creates three until-end-of-turn continuous effects, one
            // per layer, all targeting its own source. Layer order — not
            // resolution order — decides the result, and cleanup ends all three
            // (CR 514.2), so the artifact reverts exactly when the turn does.
            Effect::AddSourceCreatureType(line) => {
                self.start_source_continuous(
                    item,
                    controller,
                    SourceContinuous {
                        layer: Layer::Type,
                        kind: ContinuousKind::AddCreatureSubtypes(line),
                        text: format!(
                            "becomes a creature with type {} until end of turn ({})",
                            line.text(),
                            Layer::Type.rules()
                        ),
                        duration: EffectDuration::UntilEndOfTurn,
                    },
                    events,
                );
                false
            }
            // CR 613.1d + 611.2b: Beorn's Hospitality's animation. The printed
            // text says the effect doesn't end, so it is not until end of turn:
            // it lasts as long as the permanent it applies to stays the same
            // object on the battlefield.
            Effect::AddSourceCreatureTypeWhileOnBattlefield(line) => {
                self.start_source_continuous(
                    item,
                    controller,
                    SourceContinuous {
                        layer: Layer::Type,
                        kind: ContinuousKind::AddCreatureSubtypes(line),
                        text: format!(
                            "becomes a creature with type {} (this effect doesn't end) ({})",
                            line.text(),
                            Layer::Type.rules()
                        ),
                        duration: EffectDuration::WhileSourceOnBattlefield,
                    },
                    events,
                );
                false
            }
            // CR 613.4a: the characteristic-defining half of the same printed
            // ability. No value is captured here — the layer pass reads the land
            // count every time characteristics are computed.
            Effect::SetSourcePowerToughnessEqualToLands => {
                self.start_source_continuous(
                    item,
                    controller,
                    SourceContinuous {
                        layer: Layer::CharacteristicDefining,
                        kind: ContinuousKind::PowerToughnessEqualToLands,
                        text: format!(
                            "power and toughness are each equal to the number of lands you control ({})",
                            Layer::CharacteristicDefining.rules()
                        ),
                        duration: EffectDuration::WhileSourceOnBattlefield,
                    },
                    events,
                );
                false
            }
            Effect::SetSourceColor(color) => {
                self.start_source_continuous(
                    item,
                    controller,
                    SourceContinuous {
                        layer: Layer::Color,
                        kind: ContinuousKind::SetColor(color),
                        text: format!(
                            "becomes {} until end of turn ({})",
                            color.name(),
                            Layer::Color.rules()
                        ),
                        duration: EffectDuration::UntilEndOfTurn,
                    },
                    events,
                );
                false
            }
            Effect::SetSourceBasePowerToughness(power, toughness) => {
                self.start_source_continuous(
                    item,
                    controller,
                    SourceContinuous {
                        layer: Layer::Setting,
                        kind: ContinuousKind::SetBasePowerToughness { power, toughness },
                        text: format!(
                            "base power/toughness {power}/{toughness} ({})",
                            Layer::Setting.rules()
                        ),
                        duration: EffectDuration::UntilEndOfTurn,
                    },
                    events,
                );
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
            Effect::ReturnLandCardsFromGraveyardTapped => {
                // CR 608.2c: "return all land cards from your graveyard" is the
                // set chosen at resolution, so the list is captured first and
                // every land then enters through the one canonical entry path.
                // That path queues the `LandEnters` event for each returning
                // land, so landfall watches fire once per land even though no
                // land play was spent (slice 41's shared hook).
                let lands: Vec<ObjectId> = self
                    .state
                    .player(controller)
                    .graveyard
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.state
                            .try_object(*id)
                            .is_some_and(|object| object.card.def().is_land())
                    })
                    .collect();
                if !lands.is_empty() {
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "P{controller} returns {} land card(s) from their graveyard to the battlefield tapped (CR 608.2c)",
                            lands.len()
                        ),
                    });
                }
                for id in lands {
                    let name = self
                        .state
                        .try_object(id)
                        .map(|object| object.card.def().name)
                        .unwrap_or("land");
                    self.enter_battlefield(id, controller, events, 0);
                    if let Some(object) = self.state.objects.get_mut(&id) {
                        object.tapped = true;
                    }
                    events.push(Event::StateBasedAction {
                        description: format!("{name} enters the battlefield tapped"),
                    });
                }
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
                    .add(color, u32::from(amount));
                let pool_total = self.state.players[controller as usize].mana.get(color);
                events.push(Event::ManaAdded {
                    player: controller,
                    color,
                    amount: u32::from(amount),
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
            Effect::AddManaForEachGoblin => {
                let count = self
                    .state
                    .objects
                    .values()
                    .filter(|o| o.zone == Zone::Battlefield && o.card.def().is_goblin())
                    .count();
                self.add_red_mana(controller, u32::try_from(count).unwrap_or(u32::MAX), events);
                false
            }
            Effect::AddManaForEachTappedOpponentLand => {
                let count = self
                    .state
                    .objects
                    .values()
                    .filter(|o| {
                        o.zone == Zone::Battlefield
                            && o.tapped
                            && o.card.def().is_land()
                            && o.controller != controller
                            && self.state.is_live(o.controller)
                    })
                    .count();
                self.add_red_mana(controller, u32::try_from(count).unwrap_or(u32::MAX), events);
                false
            }
            Effect::PutPlusOneCounterOnSource => {
                if self
                    .state
                    .try_object(item.object)
                    .is_some_and(|o| o.zone == Zone::Battlefield)
                {
                    let object = self.state.objects.get_mut(&item.object).unwrap();
                    let total = object
                        .counters
                        .add(crate::counters::CounterKind::PlusOnePlusOne, 1);
                    events.push(Event::StateBasedAction {
                        description: format!("{} has {total} +1/+1 counter(s)", object.card.name()),
                    });
                }
                false
            }
            Effect::CreateBeastToken => {
                self.create_tokens(controller, CardId::BeastToken, 1, events);
                false
            }
            Effect::CreatePestToken => {
                self.create_tokens(controller, CardId::PestToken, 1, events);
                false
            }
            Effect::CreateFoodToken => {
                self.create_tokens(controller, CardId::FoodToken, 1, events);
                false
            }
            Effect::CreateClueToken => {
                self.create_tokens(controller, CardId::ClueToken, 1, events);
                false
            }
            // CR 701.16a: "exile it and investigate". The card exiled is the one
            // the trigger watched die, recorded on the stack item; if it is no
            // longer in a graveyard the exile simply finds nothing and the
            // investigate still happens, because the printed text has no "if you
            // do" between the two instructions.
            Effect::ExileDeadCreatureAndInvestigate => {
                if let Some(dead) = item.dead_object {
                    let in_graveyard = self
                        .state
                        .try_object(dead)
                        .is_some_and(|object| object.zone == Zone::Graveyard);
                    if in_graveyard {
                        // `move_to_exile` records its own "is exiled" event.
                        self.move_to_exile(dead, events);
                    }
                }
                self.create_tokens(controller, CardId::ClueToken, 1, events);
                false
            }
            Effect::CreateBearTokens(n) => {
                self.create_tokens(controller, CardId::BearToken, u32::from(n) as usize, events);
                false
            }
            // CR 614.1a: arming is not itself a token; the replacement is
            // consumed by the next draw this controller would perform this turn.
            Effect::ReplaceNextDrawWithBearToken => {
                self.arm_draw_replacement(controller);
                false
            }
            // CR 702.94a: "You may pair this creature with another unpaired
            // creature when either enters." The chosen creature is the pairing
            // partner; if either side stopped being an unpaired creature the
            // pairing simply does not happen (nothing is created, nothing is
            // destroyed).
            Effect::PairSourceWithChosenCreature => {
                for target in item.targets.clone() {
                    if let Target::Permanent(partner) = target {
                        self.pair_permanents(item.object, partner, events);
                    }
                }
                false
            }
            // CR 111.2 + 608.2c: the token is created under the control of the
            // player who controlled the artifact/enchantment, in APNAP order
            // from the effect's controller. A seat with neither permanent gets
            // nothing, and an eliminated seat is skipped.
            Effect::BearTokenForEachPlayerWithPermanent => {
                let seats = self.state.seat_count();
                for offset in 0..seats {
                    let p = ((controller as usize + offset) % seats) as PlayerId;
                    if !self.state.is_live(p) {
                        continue;
                    }
                    let controls = self.state.battlefield(p).into_iter().any(|object| {
                        object.card.def().is_artifact() || object.card.def().is_enchantment()
                    });
                    if controls {
                        self.create_tokens(p, CardId::BearToken, 1, events);
                    }
                }
                false
            }
            // CR 701.8a + 704.3: every artifact and enchantment is destroyed at
            // the same time. All zone changes are recorded first, then the
            // dies-watchers are dispatched once for the whole batch, exactly as
            // the state-based-action path does.
            Effect::DestroyAllArtifactsAndEnchantments => {
                let doomed: Vec<ObjectId> = self
                    .state
                    .objects
                    .values()
                    .filter(|object| {
                        object.zone == Zone::Battlefield
                            && (object.card.def().is_artifact()
                                || object.card.def().is_enchantment())
                    })
                    .map(|object| object.id)
                    .collect();
                if !doomed.is_empty() {
                    let watchers = death_watchers(self);
                    let combat = self.capture_combat_alone();
                    let mut deaths = Vec::new();
                    for id in &doomed {
                        if self.has_indestructible(*id) {
                            continue;
                        }
                        if self.try_regenerate(*id, events) {
                            continue;
                        }
                        if let Some(record) = self.move_to_graveyard_inner(*id, events, &combat) {
                            deaths.push(record);
                        }
                    }
                    self.queue_watchers_for_deaths(&watchers, &deaths);
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "{} artifact(s) and/or enchantment(s) were destroyed (CR 701.8a)",
                            doomed.len()
                        ),
                    });
                }
                false
            }
            // CR 701.7a: destroy exactly the chosen artifacts/enchantments that
            // are still on the battlefield. One destroy instruction, so the whole
            // set is one simultaneous batch (CR 704.3) and the death watchers see
            // every death before any of their triggers are dispatched.
            Effect::DestroyTargetArtifactsAndEnchantments => {
                let doomed: Vec<ObjectId> = self
                    .state
                    .objects
                    .values()
                    .filter(|object| {
                        object.zone == Zone::Battlefield
                            && item.targets.contains(&Target::Permanent(object.id))
                            && (object.card.def().is_artifact()
                                || object.card.def().is_enchantment())
                    })
                    .map(|object| object.id)
                    .collect();
                if !doomed.is_empty() {
                    let watchers = death_watchers(self);
                    let combat = self.capture_combat_alone();
                    let mut deaths = Vec::new();
                    for id in &doomed {
                        if self.has_indestructible(*id) {
                            events.push(Event::StateBasedAction {
                                description: format!(
                                    "{} is indestructible and is not destroyed (CR 702.12b)",
                                    self.describe_object(*id)
                                ),
                            });
                            continue;
                        }
                        if self.try_regenerate(*id, events) {
                            continue;
                        }
                        if let Some(record) = self.move_to_graveyard_inner(*id, events, &combat) {
                            deaths.push(record);
                        }
                    }
                    self.queue_watchers_for_deaths(&watchers, &deaths);
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "{} target artifact(s) and/or enchantment(s) were destroyed (CR 701.7a)",
                            doomed.len()
                        ),
                    });
                }
                false
            }
            // CR 701.26a: untapping an untapped permanent is a no-op, so the
            // same instruction works whether or not the target was tapped. The
            // counter reads the target's own effective subtype: a layer 4
            // type-changing effect ("becomes a Bear ...") counts (CR 613.1d).
            Effect::UntapTargetAndCounterIfBear => {
                for target in item.targets.clone() {
                    let Target::Permanent(id) = target else {
                        continue;
                    };
                    let Some(probe) = self.state.try_object(id) else {
                        continue;
                    };
                    let is_bear = self.has_subtype(probe, "Bear");
                    let Some(object) = self.state.objects.get_mut(&id) else {
                        continue;
                    };
                    object.tapped = false;
                    let name = object.card.name();
                    if is_bear {
                        let total = object
                            .counters
                            .add(crate::counters::CounterKind::PlusOnePlusOne, 1);
                        events.push(Event::StateBasedAction {
                            description: format!(
                                "{name} is untapped and has {total} +1/+1 counter(s)"
                            ),
                        });
                    } else {
                        events.push(Event::StateBasedAction {
                            description: format!("{name} is untapped (not a Bear)"),
                        });
                    }
                }
                false
            }
            Effect::AddManaForRiteOfFlame => {
                let copies = self
                    .state
                    .objects
                    .values()
                    .filter(|o| o.zone == Zone::Graveyard && o.card.name() == "Rite of Flame")
                    .count();
                let amount = 2u32.saturating_add(u32::try_from(copies).unwrap_or(u32::MAX));
                self.add_red_mana(controller, amount, events);
                false
            }
            Effect::EachPlayerDraws(n) => {
                let live_total = (0..self.state.seat_count())
                    .filter(|p| self.state.is_live(*p as PlayerId))
                    .count() as u8;
                self.each_player_draw(
                    item.object,
                    item.cursor + 1,
                    self.state.turn.active,
                    n,
                    n,
                    0,
                    live_total,
                    false,
                    events,
                )
            }
            Effect::PutPlusOneCountersOnTarget(n) => {
                for target in item.targets.clone() {
                    if let Target::Permanent(id) = target
                        && let Some(object) = self.state.objects.get_mut(&id)
                        && object.zone == Zone::Battlefield
                    {
                        let name = object.card.name();
                        let total = object
                            .counters
                            .add(crate::counters::CounterKind::PlusOnePlusOne, n);
                        events.push(Event::StateBasedAction {
                            description: format!(
                                "{name} has {total} +1/+1 counter(s) from a +1/+1 counter effect"
                            ),
                        });
                    }
                }
                self.recompute_layers();
                false
            }
            // CR 202.3 + 608.2: the size was fixed when the triggering spell was
            // cast and travelled here on the stack item, so a spell that has
            // already left the stack still contributes its own mana value. A
            // zero-mana-value spell still targets (X = 0) and adds nothing.
            Effect::PutPlusOneCountersOnTargetForTriggerSpellManaValue => {
                let n = item.spell_mana_value as u32;
                if n > 0 {
                    for target in item.targets.clone() {
                        if let Target::Permanent(id) = target
                            && let Some(object) = self.state.objects.get_mut(&id)
                            && object.zone == Zone::Battlefield
                        {
                            let name = object.card.name();
                            let total = object
                                .counters
                                .add(crate::counters::CounterKind::PlusOnePlusOne, n);
                            events.push(Event::StateBasedAction {
                                description: format!(
                                    "{name} has {total} +1/+1 counter(s) from a cast triggered ability (X = {n}, CR 202.3)"
                                ),
                            });
                        }
                    }
                    self.recompute_layers();
                }
                false
            }
            // CR 613.4c + 613.4d: one pump and one trample grant per qualifying
            // creature, both until end of turn. Power is read at resolution, so a
            // creature that already carries counters or pumps qualifies on its
            // current power; a creature below the printed threshold gets nothing.
            Effect::BoostAndTrampleEachCreatureYouControlWithPowerAtLeast {
                minimum_power,
                power,
                toughness,
            } => {
                let qualifying: Vec<ObjectId> = self
                    .state
                    .objects
                    .values()
                    .filter(|object| {
                        object.zone == Zone::Battlefield
                            && object.controller == controller
                            && self.is_creature(object)
                            && object.power() >= minimum_power
                    })
                    .map(|object| object.id)
                    .collect();
                for id in qualifying {
                    self.add_modifier_effect(item, id, power, toughness, events);
                    self.add_trample_grant(item, id, events);
                }
                self.recompute_layers();
                false
            }
            Effect::SkipNextUntapStep => {
                let card = item.card;
                let on_battlefield = self
                    .state
                    .try_object(item.object)
                    .is_some_and(|object| object.zone == Zone::Battlefield);
                // CR 603.4: the intervening-if is re-read at resolution. If the
                // defending seat has gained a black nontoken permanent since
                // attackers were declared, the ability does nothing.
                let condition_met = self
                    .state
                    .turn
                    .defender
                    .map(|defender| self.trigger_condition_holds(card, defender, item.object))
                    .unwrap_or(true);
                if on_battlefield && condition_met {
                    self.state.skip_next_untap.insert(item.object);
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "{} doesn't untap during its controller's next untap step (CR 502.3)",
                            card.name()
                        ),
                    });
                } else if !condition_met {
                    events.push(Event::TriggerRemoved {
                        controller,
                        card,
                        reason: "the printed intervening-if condition is false at resolution \
                                 (CR 603.4)"
                            .to_string(),
                    });
                }
                false
            }
            Effect::CreateEarthKingBearToken => {
                self.create_tokens(controller, CardId::EarthKingBearToken, 1, events);
                false
            }
            Effect::SearchBasicLandsUpToTriggerAmount => {
                let amount = item.search_amount;
                if amount == 0 {
                    return false;
                }
                let eligible: Vec<ObjectId> = self
                    .state
                    .player(controller)
                    .library
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.state.try_object(*id).is_some_and(|object| {
                            object.zone == Zone::Library
                                && object.card.def().is_land()
                                && object.card.def().basic_mana.is_some()
                        })
                    })
                    .collect();
                self.state.pending = Some(Pending::SearchLibrary {
                    player: controller,
                    remaining: amount,
                    chosen: Vec::new(),
                    eligible,
                    mode: crate::state::LibrarySearchMode::BasicLandToBattlefield,
                    looked: Vec::new(),
                });
                self.state.turn.priority = None;
                true
            }
            Effect::LookAtTopPutCreatureAttacking => {
                if self.state.turn.defender.is_none() {
                    return false;
                }
                // The eight-land attack event already qualified. Cub prints no
                // intervening-if, so a later land departure cannot suppress it.
                let library = self.state.player(controller).library.clone();
                let looked: Vec<ObjectId> = library.into_iter().take(8).collect();
                let eligible: Vec<ObjectId> = looked
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.state
                            .try_object(*id)
                            .is_some_and(|object| object.card.def().is_creature())
                    })
                    .collect();
                events.push(Event::StateBasedAction {
                    description: format!(
                        "P{controller} looks at the top {} card(s) of their library (CR 701.20a)",
                        looked.len()
                    ),
                });
                // Even an all-ineligible look must reach its entitled viewer
                // before they acknowledge bottoming the examined cards.
                self.state.pending = Some(Pending::SearchLibrary {
                    player: controller,
                    remaining: 1,
                    chosen: Vec::new(),
                    eligible,
                    mode: crate::state::LibrarySearchMode::CreatureFromTopToBattlefieldAttacking,
                    looked,
                });
                self.state.turn.priority = None;
                true
            }
            Effect::AttachSourceToTarget => {
                let [Target::Permanent(target)] = item.targets.as_slice() else {
                    return false;
                };
                let source_ok = item.affected == Some(item.object)
                    && self.state.try_object(item.object).is_some_and(|object| {
                        object.zone == Zone::Battlefield
                            && object.controller == controller
                            && crate::ability::equipment_grants(object.card).is_some()
                    });
                let target_ok = self.state.try_object(*target).is_some_and(|object| {
                    object.zone == Zone::Battlefield
                        && object.controller == controller
                        && self.is_creature(object)
                });
                if !source_ok || !target_ok {
                    return false;
                }
                let grants = crate::ability::equipment_grants(item.card).expect("equipment grants");
                self.state.effects.retain(|effect| {
                    !(matches!(effect.kind, ContinuousKind::Equipped { .. })
                        && effect.source == Some(item.object))
                });
                let timestamp = self.next_timestamp();
                let id = self.next_effect_id();
                self.state.effects.push(ContinuousEffect {
                    id,
                    timestamp,
                    source: Some(item.object),
                    card: item.card,
                    controller,
                    layer: Layer::Ability,
                    target: *target,
                    kind: ContinuousKind::Equipped {
                        haste: grants.haste,
                        shroud: grants.shroud,
                        hexproof: grants.hexproof,
                    },
                    duration: EffectDuration::WhileSourceOnBattlefield,
                });
                self.recompute_layers();
                events.push(Event::StateBasedAction {
                    description: format!(
                        "{} is attached to {} (CR 301.5)",
                        item.card.name(),
                        self.describe_object(*target)
                    ),
                });
                false
            }
            Effect::SearchSnowLandToHand => {
                let eligible: Vec<ObjectId> = self
                    .state
                    .player(controller)
                    .library
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.state.try_object(*id).is_some_and(|object| {
                            object.zone == Zone::Library
                                && object.card.def().is_land()
                                && object.card.def().is_snow()
                        })
                    })
                    .collect();
                self.state.pending = Some(Pending::SearchLibrary {
                    player: controller,
                    remaining: 1,
                    chosen: Vec::new(),
                    eligible,
                    mode: crate::state::LibrarySearchMode::SnowLandToHand,
                    looked: Vec::new(),
                });
                self.state.turn.priority = None;
                true
            }
            Effect::ExileTargetPlayersGraveyardAndGainLife => {
                let Some(Target::Player(victim)) = item.targets.first().copied() else {
                    return false;
                };
                let graveyard = self.state.player(victim).graveyard.clone();
                let count = graveyard.len() as i32;
                for id in graveyard {
                    self.move_to_exile(id, events);
                }
                if count > 0 {
                    self.state.players[controller as usize].life += count;
                    events.push(Event::LifeChange {
                        player: controller,
                        delta: count,
                        life: self.state.player(controller).life,
                    });
                }
                events.push(Event::StateBasedAction { description: format!("P{controller} exiles P{victim}'s graveyard ({count} card(s)) and gains {count} life") });
                false
            }
            Effect::SearchAnyLandUpToTwo => {
                let eligible: Vec<ObjectId> = self
                    .state
                    .player(controller)
                    .library
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.state.try_object(*id).is_some_and(|object| {
                            object.zone == Zone::Library && object.card.def().is_land()
                        })
                    })
                    .collect();
                self.state.pending = Some(Pending::SearchLibrary {
                    player: controller,
                    remaining: 2,
                    chosen: Vec::new(),
                    eligible,
                    mode: crate::state::LibrarySearchMode::AnyLandToBattlefield,
                    looked: Vec::new(),
                });
                self.state.turn.priority = None;
                true
            }
            Effect::PutTwoCountersOnEachCreatureYouControl => {
                let creatures: Vec<ObjectId> = self
                    .state
                    .battlefield(controller)
                    .into_iter()
                    .filter(|o| self.is_creature(o))
                    .map(|o| o.id)
                    .collect();
                for id in creatures {
                    if let Some(object) = self.state.objects.get_mut(&id) {
                        let total = object
                            .counters
                            .add(crate::counters::CounterKind::PlusOnePlusOne, 2);
                        let name = object.card.def().name;
                        events.push(Event::StateBasedAction {
                            description: format!("{name} gets two +1/+1 counters (now {total})"),
                        });
                    }
                }
                false
            }
            Effect::UrsineCombatTrigger => {
                self.mill_cards(controller, 1, events);
                let opponents: Vec<PlayerId> = (0..self.state.seat_count() as PlayerId)
                    .filter(|p| *p != controller && self.state.is_live(*p))
                    .collect();
                let Some(defender) = opponents
                    .get(self.state.rng.below(opponents.len() as u32) as usize)
                    .copied()
                else {
                    return false;
                };
                // CR 608.2b: the ability resolves even when its source has left
                // the battlefield, so the mill still happens. Every grant below
                // is about that creature as a *battlefield permanent*, so it is
                // created only while the source is still one. The observable
                // defect this gate fixes was the event line: it narrated the
                // indestructible/+1/+1 grant and the attack requirement for a
                // source that no longer existed. A stale continuous effect
                // created here would have been at most transient, because
                // `recompute_layers` drops effects whose target is not a
                // battlefield permanent, so no recast incarnation was ever at
                // risk; the false narration is what the player actually saw.
                let granted = item.affected == Some(item.object)
                    && self
                        .state
                        .try_object(item.object)
                        .is_some_and(|object| object.zone == Zone::Battlefield);
                if granted {
                    let timestamp = self.next_timestamp();
                    let id = self.next_effect_id();
                    self.state.effects.push(ContinuousEffect {
                        id,
                        timestamp,
                        source: Some(item.object),
                        card: item.card,
                        controller,
                        layer: Layer::Ability,
                        target: item.object,
                        kind: ContinuousKind::Indestructible,
                        duration: EffectDuration::UntilEndOfTurn,
                    });
                    let types = self.graveyard_card_type_count(controller);
                    let id = self.next_effect_id();
                    self.state.effects.push(ContinuousEffect {
                        id,
                        timestamp,
                        source: Some(item.object),
                        card: item.card,
                        controller,
                        layer: Layer::Modifying,
                        target: item.object,
                        kind: ContinuousKind::ModifyPowerToughness {
                            power: types,
                            toughness: types,
                        },
                        duration: EffectDuration::UntilEndOfTurn,
                    });
                    let id = self.next_effect_id();
                    self.state.effects.push(ContinuousEffect {
                        id,
                        timestamp,
                        source: Some(item.object),
                        card: item.card,
                        controller: defender,
                        layer: Layer::Ability,
                        target: item.object,
                        kind: ContinuousKind::MustAttackThisCombat,
                        duration: EffectDuration::UntilEndOfTurn,
                    });
                    self.recompute_layers();
                }
                let description = if granted {
                    format!(
                        "Ursine Monstrosity mills a card, must attack P{defender} if able, and gains indestructible/+1/+1 per graveyard card type"
                    )
                } else {
                    "Ursine Monstrosity mills a card; its source has left the battlefield, so no attack requirement, indestructible or +1/+1 applies".to_string()
                };
                events.push(Event::StateBasedAction { description });
                false
            }
            Effect::SaddleSource => {
                if item.affected == Some(item.object)
                    && self
                        .state
                        .try_object(item.object)
                        .is_some_and(|object| object.zone == Zone::Battlefield)
                {
                    let timestamp = self.next_timestamp();
                    let id = self.next_effect_id();
                    self.state.effects.push(ContinuousEffect {
                        id,
                        timestamp,
                        source: Some(item.object),
                        card: item.card,
                        controller,
                        layer: Layer::Ability,
                        target: item.object,
                        kind: ContinuousKind::Saddled,
                        duration: EffectDuration::UntilEndOfTurn,
                    });
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "{} becomes saddled until end of turn",
                            item.card.name()
                        ),
                    });
                }
                false
            }
            Effect::GrantTrampleToControlledCreatures => {
                let creatures: Vec<ObjectId> = self
                    .state
                    .battlefield(controller)
                    .into_iter()
                    .filter(|o| self.is_creature(o))
                    .map(|o| o.id)
                    .collect();
                for target in creatures {
                    self.add_trample_grant(item, target, events);
                }
                false
            }
            // CR 122.1b: a trample counter is a counter, not a continuous
            // effect. It stays on the permanent after cleanup, so the keyword
            // persists until something removes the counter.
            Effect::PutTrampleCounterOnTarget => {
                for target in item.targets.clone() {
                    if let Target::Permanent(id) = target
                        && let Some(object) = self.state.objects.get_mut(&id)
                        && object.zone == Zone::Battlefield
                    {
                        let name = object.card.name();
                        let total = object
                            .counters
                            .add(crate::counters::CounterKind::Trample, 1);
                        events.push(Event::StateBasedAction {
                            description: format!(
                                "{name} has {total} trample counter(s) and therefore trample (CR 122.1b)"
                            ),
                        });
                    }
                }
                false
            }
            // CR 613.1d: "It becomes a Bear in addition to its other types."
            // No duration is printed, so the continuous effect lasts until end
            // of turn (CR 611.2a) and cleanup removes it.
            Effect::AddTargetCreatureType(line) => {
                for target in item.targets.clone() {
                    if let Target::Permanent(id) = target
                        && self
                            .state
                            .try_object(id)
                            .is_some_and(|object| object.zone == Zone::Battlefield)
                    {
                        self.add_subtype_addition_effect(item, id, line, events);
                    }
                }
                false
            }
            // "Then if you control N or more Bears, draw M cards." The count is
            // taken now, after the earlier instructions of the same resolution
            // have applied (CR 608.2c), and it reads the *effective* subtype, so
            // the creature this ability just turned into a Bear counts.
            Effect::DrawIfControlsBears { minimum, cards } => {
                let bears = self
                    .state
                    .objects
                    .values()
                    .filter(|object| {
                        object.zone == Zone::Battlefield
                            && object.controller == controller
                            && self.is_creature(object)
                            && self.has_subtype(object, "Bear")
                    })
                    .count();
                if bears < minimum as usize {
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "P{controller} controls {bears} Bear(s), fewer than {minimum}: no cards are drawn (CR 608.2c)"
                        ),
                    });
                    return false;
                }
                for index in 0..cards {
                    let remaining = cards - index - 1;
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
            Effect::SetBasePowerToughnessFromSource => {
                let [Target::Permanent(target)] = item.targets.as_slice() else {
                    return false;
                };
                let target_ok = self.state.try_object(*target).is_some_and(|object| {
                    object.zone == Zone::Battlefield
                        && self.is_creature(object)
                        && object.controller == controller
                        && self.has_subtype(object, "Human")
                });
                let source_ok = self.state.try_object(item.object).is_some_and(|object| {
                    object.zone == Zone::Battlefield && self.is_creature(object)
                });
                if !target_ok || (!source_ok && item.source_pt_lki.is_none()) {
                    return false;
                }
                let (power, toughness) = item.source_pt_lki.unwrap_or_else(|| {
                    let source = self.state.object(item.object);
                    (source.power(), source.toughness())
                });
                let timestamp = self.next_timestamp();
                let id = self.next_effect_id();
                self.state.effects.push(ContinuousEffect {
                    id,
                    timestamp,
                    source: Some(item.object),
                    card: item.card,
                    controller,
                    layer: Layer::Setting,
                    target: *target,
                    kind: ContinuousKind::SetBasePowerToughness { power, toughness },
                    duration: EffectDuration::UntilEndOfTurn,
                });
                self.recompute_layers();
                events.push(Event::ContinuousEffectStarted {
                    object: *target,
                    card: item.card,
                    layer: Layer::Setting.label().to_string(),
                    text: format!("base power/toughness {power}/{toughness}"),
                });
                false
            }
            Effect::PutCounterAndHasteOnEntered => {
                let Some(target) = item.affected else {
                    return false;
                };
                let entered_is_creature = self.state.try_object(target).is_some_and(|object| {
                    object.zone == Zone::Battlefield && self.is_creature(object)
                });
                if entered_is_creature
                    && let Some(object) = self.state.objects.get_mut(&target)
                    && object.zone == Zone::Battlefield
                {
                    object
                        .counters
                        .add(crate::counters::CounterKind::PlusOnePlusOne, 1);
                    let name = object.card.name();
                    self.add_haste_grant(item, target, events);
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "{name} gets a +1/+1 counter and gains haste until end of turn"
                        ),
                    });
                }
                false
            }
            Effect::RegenerateSource => {
                let source = item.object;
                if self.state.try_object(source).is_some_and(|object| {
                    object.zone == Zone::Battlefield && object.controller == controller
                }) {
                    self.state
                        .regeneration_shields
                        .push(crate::state::RegenerationShield { object: source });
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "{} gains a regeneration shield until cleanup (CR 701.19a)",
                            item.card.name()
                        ),
                    });
                }
                false
            }
            Effect::LookTopTakeOrBottom => {
                let Some(top) = self.state.player(controller).library.first().copied() else {
                    events.push(Event::StateBasedAction {
                        description: format!(
                            "P{controller} looks at an empty library; no card can be taken and no draw occurs"
                        ),
                    });
                    return false;
                };
                let eligible = self.state.try_object(top).is_some_and(|object| {
                    object.zone == Zone::Library
                        && (object.card.def().is_creature()
                            || object.card.def().type_line.contains("Planeswalker"))
                });
                // Looking happens even when the card cannot be taken. The
                // owner must acknowledge the mandatory bottom, and only that
                // owner's projection shows the looked-at identity.
                self.state.pending = Some(Pending::TopCardChoice {
                    player: controller,
                    card: top,
                    can_take: eligible,
                });
                self.state.turn.priority = None;
                true
            }
            Effect::FightTargets => {
                let [Target::Permanent(first), Target::Permanent(second)] = item.targets.as_slice()
                else {
                    return false;
                };
                let valid = self.state.try_object(*first).is_some_and(|o| {
                    o.zone == Zone::Battlefield
                        && self.is_creature(o)
                        && o.controller == controller
                        && self.has_subtype(o, "Bear")
                }) && self.state.try_object(*second).is_some_and(|o| {
                    o.zone == Zone::Battlefield && self.is_creature(o) && o.controller != controller
                });
                if !valid {
                    events.push(Event::StateBasedAction {
                        description: "fight does not happen because a target no longer matches the printed relation (CR 701.14b)".to_string(),
                    });
                    return false;
                }
                let (first_power, second_power) = (
                    self.state.object(*first).power().max(0) as u32,
                    self.state.object(*second).power().max(0) as u32,
                );
                let dealt_to_second = self.deal_damage(
                    Some(*first),
                    DamageTarget::Permanent(*second),
                    first_power,
                    false,
                    events,
                );
                events.push(Event::Damage {
                    source: Some(*first),
                    target: self.damage_target_label(DamageTarget::Permanent(*second)),
                    amount: dealt_to_second,
                });
                let dealt_to_first = self.deal_damage(
                    Some(*second),
                    DamageTarget::Permanent(*first),
                    second_power,
                    false,
                    events,
                );
                events.push(Event::Damage {
                    source: Some(*second),
                    target: self.damage_target_label(DamageTarget::Permanent(*first)),
                    amount: dealt_to_first,
                });
                false
            }
            Effect::CreateBearTokensForEachDeathThisTurn => {
                let count = self
                    .state
                    .nontoken_creature_deaths_this_turn
                    .get(controller as usize)
                    .copied()
                    .unwrap_or(0) as usize;
                self.create_tokens(controller, CardId::BearToken, count, events);
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
        let entering_bear = self
            .state
            .try_object(entering)
            .is_some_and(|o| o.zone == Zone::Battlefield && self.has_subtype(o, "Bear"));
        for (source, source_card, source_controller) in watchers {
            if source_card == CardId::SurrakAndGoreclaw {
                let qualifies = self.state.try_object(entering).is_some_and(|object| {
                    object.zone == Zone::Battlefield
                        && object.controller == source_controller
                        && self.is_creature(object)
                        && !object.card.is_token()
                });
                if qualifies {
                    let seq = self.state.next_trigger_seq;
                    self.state.next_trigger_seq += 1;
                    self.state.trigger_queue.push(TriggerRef {
                        source,
                        card: source_card,
                        controller: source_controller,
                        ability: 0,
                        seq,
                        spell_mana_value: 0,
                        affected: Some(entering),
                        dead_object: None,
                        source_pt_lki: None,
                        search_amount: 0,
                        x_value: 0,
                    });
                }
                continue;
            }
            if source_card == CardId::AyulaQueenAmongBears {
                let controlled = self
                    .state
                    .try_object(entering)
                    .is_some_and(|o| o.controller == source_controller);
                if !entering_bear || !controlled {
                    continue;
                }
            }
            self.queue_triggers(
                source,
                source_card,
                source_controller,
                TriggerEvent::AnotherCreatureEnters,
            );
        }
    }

    /// Add red mana with a widened, explicitly bounded pool payload.
    fn add_red_mana(&mut self, controller: PlayerId, amount: u32, events: &mut Vec<Event>) {
        self.state.players[controller as usize]
            .mana
            .add(ManaColor::Red, amount);
        events.push(Event::ManaAdded {
            player: controller,
            color: ManaColor::Red,
            amount,
            pool_total: self.state.players[controller as usize].mana.red,
        });
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
        // A permanent that left the battlefield (or stopped being a creature,
        // CR 506.4) is removed from combat; an animated artifact is a creature
        // here for exactly as long as its layer 4 effect applies.
        let alive = |game: &Game, id: ObjectId| {
            game.state
                .try_object(id)
                .is_some_and(|o| o.zone == Zone::Battlefield && game.is_creature(o))
        };
        let attackers: Vec<ObjectId> = self
            .state
            .turn
            .attackers
            .iter()
            .copied()
            .filter(|id| alive(self, *id))
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
            .filter(|(blocker, attacker)| alive(self, *blocker) && attackers.contains(attacker))
            .collect();
        let mut unpaired = self.state.turn.unpaired_blockers.clone();
        unpaired.extend(
            self.state
                .turn
                .blockers
                .iter()
                .filter(|(blocker, attacker)| {
                    alive(self, *blocker) && !attackers.contains(attacker)
                })
                .map(|(blocker, _)| *blocker),
        );
        unpaired
            .retain(|id| alive(self, *id) && !blockers.iter().any(|(blocker, _)| blocker == id));
        unpaired.sort_unstable();
        unpaired.dedup();
        self.state.turn.attackers = attackers;
        self.state.turn.blocked = blocked;
        self.state.turn.blockers = blockers;
        self.state.turn.unpaired_blockers = unpaired;
    }

    /// CR 510.1 + 509.1h: create the combat-damage-assignment choice for every
    /// blocked attacker whose controller also controls a permanent printing
    /// "... you may have that creature assign its combat damage as though it
    /// weren't blocked". An unblocked creature already assigns as though it
    /// weren't blocked, so no choice is created for it: the permission would
    /// have no consequence and asking would invent a decision the game does not
    /// actually require.
    fn begin_combat_damage_assignment(&mut self) -> bool {
        if self.state.turn.combat_damage_done {
            return false;
        }
        // A creature that has already left the battlefield neither deals nor
        // receives combat damage, so it cannot be asked either.
        self.prune_combat();
        let mut order: Vec<CombatDamageChoice> = Vec::new();
        for source in self.state.objects.values() {
            if source.zone != Zone::Battlefield {
                continue;
            }
            let grants = ability::static_abilities(source.card)
                .iter()
                .any(|static_ability| {
                    matches!(
                    static_ability,
                    ability::StaticAbility::ControlledCreaturesWithNoAbilitiesAssignAsUnblocked {
                        ..
                    }
                )
                });
            if !grants {
                continue;
            }
            let controller = source.controller;
            let source_id = source.id;
            for attacker in self.state.turn.attackers.clone() {
                let Some(object) = self.state.try_object(attacker) else {
                    continue;
                };
                if object.controller != controller
                    || !self.state.turn.blocked.contains(&attacker)
                    || !self.state.object_has_no_abilities(attacker)
                {
                    continue;
                }
                order.push(CombatDamageChoice {
                    player: controller,
                    attacker,
                    source: source_id,
                });
            }
        }
        if order.is_empty() {
            return false;
        }
        // CR 101.4: ask in APNAP order from the active player. Two permission
        // sources controlled by the same seat would otherwise offer the same
        // creature twice, so the list is deduplicated by attacker after sorting.
        let seats = self.state.seat_count();
        let anchor = self.state.turn.active;
        order.sort_by_key(|choice| {
            (
                (choice.player as usize + seats - anchor as usize) % seats,
                choice.attacker,
            )
        });
        order.dedup_by_key(|choice| choice.attacker);
        self.state.pending = Some(Pending::CombatDamageAssignment {
            order,
            cursor: 0,
            answers: Vec::new(),
        });
        self.state.turn.priority = None;
        true
    }

    fn apply_combat_damage_assignment(
        &mut self,
        _player: PlayerId,
        as_though_unblocked: bool,
        events: &mut Vec<Event>,
    ) {
        let Some(Pending::CombatDamageAssignment {
            order,
            cursor,
            mut answers,
        }) = self.state.pending.take()
        else {
            return;
        };
        answers.push(as_though_unblocked);
        let next = cursor + 1;
        if next < order.len() {
            self.state.pending = Some(Pending::CombatDamageAssignment {
                order,
                cursor: next,
                answers,
            });
            self.state.turn.priority = None;
            return;
        }
        let mut chosen: Vec<ObjectId> = Vec::new();
        for (choice, assigned) in order.iter().zip(answers.iter()) {
            events.push(Event::CombatDamageAssignment {
                player: choice.player,
                attacker: choice.attacker,
                card: self.state.object(choice.attacker).card,
                as_though_unblocked: *assigned,
            });
            if *assigned {
                chosen.push(choice.attacker);
            }
        }
        // CR 510.2: every creature assigns and deals its combat damage as one
        // simultaneous event, so the whole batch waits for the last answer.
        self.deal_combat_damage(&chosen, events);
    }

    /// CR 510.1: deal combat damage. `as_unblocked` names the attacking
    /// creatures whose controller chose "assign its combat damage as though it
    /// weren't blocked"; every other attacker keeps the automatic assignment
    /// this slice has always used.
    fn deal_combat_damage(&mut self, as_unblocked: &[ObjectId], events: &mut Vec<Event>) {
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
            if as_unblocked.contains(attacker) {
                // CR 510.1: the controller chose to have this creature assign as
                // though it weren't blocked. Its whole damage goes to the
                // defending player and none to the blockers; the blockers are
                // still blocking creatures and still deal their own damage to it.
                damage.push((*attacker, None, power as u32));
                for blocker in &its_blockers {
                    if let Some(b) = self.state.try_object(*blocker) {
                        let bp = b.power();
                        if bp > 0 {
                            damage.push((*blocker, Some(*attacker), bp as u32));
                        }
                    }
                }
            } else if its_blockers.is_empty() {
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
                let trample = self.has_trample(atk);
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
            Target::Card(id) => match self.state.try_object(id) {
                Some(o) => format!("{} in the graveyard", o.card.name()),
                None => "a card that has left the graveyard".to_string(),
            },
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
            if self.state.object(id).card == CardId::TitaniasCommand {
                for first in 0u8..4 {
                    for second in (first + 1)..4 {
                        let modes = [first, second];
                        let target_sets: Vec<Vec<Target>> = if modes.contains(&0) {
                            (0..self.state.seat_count() as PlayerId)
                                .filter(|p| self.state.is_live(*p))
                                .map(|p| vec![Target::Player(p)])
                                .collect()
                        } else {
                            vec![Vec::new()]
                        };
                        for targets in target_sets {
                            let action = Action::CastSpellWithModes {
                                card: id,
                                modes,
                                targets: targets.clone(),
                            };
                            if self.validate(player, &action).is_err() {
                                continue;
                            }
                            out.push(LegalAction {
                                action,
                                label: format!(
                                    "Cast Titania's Command ({}; {}){}",
                                    crate::card::modal_mode_name(CardId::TitaniasCommand, first)
                                        .expect("printed mode"),
                                    crate::card::modal_mode_name(CardId::TitaniasCommand, second)
                                        .expect("printed mode"),
                                    if targets.is_empty() {
                                        String::new()
                                    } else {
                                        format!(" -> {}", self.describe_target(targets[0]))
                                    }
                                ),
                                rules: vec!["601.2b", "700.2"],
                            });
                        }
                    }
                }
                continue;
            }
            if self.state.object(id).card.def().has_x_cost() {
                // CR 107.3a: an `{X}` spell is offered once per affordable X,
                // lowest first, so the announced value is a real choice and not
                // a silently-zero default. The printed cost only rises by one
                // generic mana per step, so the first unaffordable X ends the
                // scan.
                for targets in self.target_combinations(player, id) {
                    let mut x: u8 = 0;
                    loop {
                        let action = Action::CastSpellWithX {
                            card: id,
                            x,
                            targets: targets.clone(),
                        };
                        if self.validate(player, &action).is_err() {
                            break;
                        }
                        let name = self.state.object(id).card.name();
                        let from_command = self.state.object(id).zone == Zone::Command;
                        out.push(LegalAction {
                            action,
                            label: if from_command {
                                format!("Cast commander {name} from the command zone with X={x}")
                            } else {
                                format!("Cast {name} with X={x}")
                            },
                            rules: vec!["107.3a", "601.2b"],
                        });
                        if x == MAX_X_VALUE {
                            break;
                        }
                        x += 1;
                    }
                }
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
        for (land, color, amount) in self.mana_sources(player) {
            out.push(LegalAction {
                action: Action::ActivateManaAbility { land },
                label: format!(
                    "Tap {} for {}x{}",
                    self.state.object(land).card.name(),
                    color.symbol(),
                    amount
                ),
                rules: vec!["605.1a"],
            });
        }

        let mana_timing = self.state.pending.is_none() && self.state.turn.priority == Some(player);
        for object in self.state.battlefield(player) {
            if !mana_timing || object.tapped {
                continue;
            }
            for color in self.dynamic_mana_colors(player, object.id) {
                out.push(LegalAction {
                    action: Action::ActivateColorManaAbility {
                        source: object.id,
                        color,
                    },
                    label: format!("Tap {} for {}", object.card.name(), color.symbol()),
                    rules: vec!["605.1a"],
                });
            }
        }

        // Activated abilities (CR 602.2). Each legal target and each legal
        // combination of named cost cards is its own option.
        for object in self.state.battlefield(player) {
            self.push_activation_actions(player, object.id, object.card, &mut out);
            self.push_saddle_actions(player, object.id, object.card, &mut out);
        }
        // CR 602.2: an ability whose cost exiles its own card is activated from
        // the graveyard, so it is offered from that zone and offered nowhere
        // else (the validator refuses any other zone).
        let graveyard_sources: Vec<(ObjectId, CardId)> = self
            .state
            .player(player)
            .graveyard
            .clone()
            .into_iter()
            .filter_map(|id| {
                self.state
                    .try_object(id)
                    .map(|object| (object.id, object.card))
            })
            .filter(|(_, card)| {
                crate::ability::abilities(*card).iter().any(|ability| {
                    ability
                        .activated()
                        .is_some_and(|activated| activated.cost.exile_source_from_graveyard)
                })
            })
            .collect();
        for (source, card) in graveyard_sources {
            self.push_activation_actions(player, source, card, &mut out);
        }
        out
    }

    /// Every legal way to activate one permanent's (or graveyard card's)
    /// abilities: one entry per printed ability, target and legal set of named
    /// cost cards. Nothing is offered that `validate` would refuse.
    fn push_saddle_actions(
        &self,
        player: PlayerId,
        source: ObjectId,
        card: CardId,
        out: &mut Vec<LegalAction>,
    ) {
        let Some((_, threshold)) = self.saddle_ability_index(card) else {
            return;
        };
        let helpers: Vec<ObjectId> = self
            .state
            .battlefield(player)
            .into_iter()
            .filter(|object| object.id != source && self.is_creature(object) && !object.tapped)
            .map(|object| object.id)
            .collect();
        if helpers.is_empty() || helpers.len() > 10 {
            return;
        }
        for mask in 1u16..(1u16 << helpers.len()) {
            let chosen: Vec<ObjectId> = helpers
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1 << index) != 0)
                .map(|(_, id)| *id)
                .collect();
            let total: i64 = chosen
                .iter()
                .filter_map(|id| self.state.try_object(*id))
                .map(|object| i64::from(object.power()))
                .sum();
            if total < i64::from(threshold) {
                continue;
            }
            let action = Action::SaddleMount {
                source,
                creatures: chosen.clone(),
            };
            if self.validate(player, &action).is_err() {
                continue;
            }
            out.push(LegalAction {
                action,
                label: format!(
                    "Saddle {} by tapping {}",
                    card.def().name,
                    chosen
                        .iter()
                        .filter_map(|id| self.state.try_object(*id))
                        .map(|object| format!("{} #{}", object.card.def().name, object.id.0))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                rules: vec!["702.171a"],
            });
        }
    }

    fn push_activation_actions(
        &self,
        player: PlayerId,
        source: ObjectId,
        card: CardId,
        out: &mut Vec<LegalAction>,
    ) {
        for (index, ability) in crate::ability::abilities(card).iter().enumerate() {
            let Ability::Activated(activated) = ability else {
                continue;
            };
            let target_sets: Vec<Vec<Target>> = if activated.target == TargetSpec::None {
                vec![Vec::new()]
            } else {
                self.legal_targets(player, activated.target, source)
                    .into_iter()
                    .map(|target| vec![target])
                    .collect()
            };
            let cost_sets = self.legal_cost_card_sets(player, activated.cost);
            for target_set in &target_sets {
                for cost_set in &cost_sets {
                    let action = if activated.cost.card_cost_count() == 0 {
                        Action::ActivateAbility {
                            source,
                            ability: index as u8,
                            mode: 0,
                            targets: target_set.clone(),
                        }
                    } else {
                        Action::ActivateCostedAbility {
                            source,
                            ability: index as u8,
                            mode: 0,
                            targets: target_set.clone(),
                            cost_cards: cost_set.clone(),
                        }
                    };
                    if self.validate(player, &action).is_err() {
                        continue;
                    }
                    let mut rendered = String::new();
                    if !target_set.is_empty() {
                        rendered.push_str(&format!(
                            " -> {}",
                            target_set
                                .iter()
                                .map(|t| self.describe_target(*t))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    if !cost_set.is_empty() {
                        rendered.push_str(&format!(
                            " (cost: {})",
                            cost_set
                                .iter()
                                .map(|id| self.state.object(*id).card.name())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    out.push(LegalAction {
                        action,
                        label: format!("Activate {}: {}{rendered}", card.name(), activated.text),
                        rules: vec!["602.2", "602.2b"],
                    });
                }
            }
        }
    }

    /// Every combination of cards that could pay this printed cost, in a
    /// deterministic order (hand order for the discarded land, graveyard order
    /// for exiled cards). The engine still validates each combination.
    fn legal_cost_card_sets(
        &self,
        player: PlayerId,
        cost: crate::ability::AbilityCost,
    ) -> Vec<Vec<ObjectId>> {
        if cost.card_cost_count() == 0 {
            return vec![Vec::new()];
        }
        let mut prefix: Vec<Vec<ObjectId>> = vec![Vec::new()];
        if cost.discard_land {
            let lands: Vec<ObjectId> = self
                .state
                .player(player)
                .hand
                .iter()
                .copied()
                .filter(|id| {
                    self.state
                        .try_object(*id)
                        .is_some_and(|object| object.card.def().is_land())
                })
                .collect();
            prefix = lands.into_iter().map(|id| vec![id]).collect();
        }
        let wanted = cost.exile_graveyard_cards as usize;
        if wanted == 0 {
            return prefix;
        }
        let graveyard: Vec<ObjectId> = self.state.player(player).graveyard.clone();
        let mut combinations: Vec<Vec<ObjectId>> = Vec::new();
        collect_combinations(&graveyard, wanted, 0, &mut Vec::new(), &mut combinations);
        let mut out = Vec::new();
        for head in &prefix {
            for tail in &combinations {
                let mut set = head.clone();
                set.extend(tail.iter().copied());
                out.push(set);
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
                let forced: Vec<(ObjectId, PlayerId)> = legal
                    .iter()
                    .filter_map(|id| {
                        self.forced_attack_defender(*id)
                            .map(|defender| (*id, defender))
                    })
                    .collect();
                let mut out = Vec::new();
                if forced.is_empty() {
                    out.push(LegalAction {
                        action: Action::DeclareAttackers {
                            attackers: Vec::new(),
                            defender: None,
                        },
                        label: "Declare no attackers".to_string(),
                        rules: vec!["508.1a"],
                    });
                }
                for defender in 0..self.state.seat_count() as PlayerId {
                    if defender == *player || !self.state.is_live(defender) {
                        continue;
                    }
                    let attackers: Vec<ObjectId> = legal
                        .iter()
                        .copied()
                        .filter(|id| {
                            forced
                                .iter()
                                .find(|(forced_id, _)| forced_id == id)
                                .is_none_or(|(_, required)| *required == defender)
                        })
                        .collect();
                    if attackers.is_empty() {
                        continue;
                    }
                    let names: Vec<&str> = attackers
                        .iter()
                        .map(|id| self.state.object(*id).card.name())
                        .collect();
                    let action = Action::DeclareAttackers {
                        attackers,
                        defender: Some(defender),
                    };
                    if self.validate(*player, &action).is_err() {
                        continue;
                    }
                    out.push(LegalAction {
                        action,
                        label: format!(
                            "Attack P{defender} with all eligible creatures ({})",
                            names.join(", ")
                        ),
                        rules: vec!["508.1a", "508.1d", "508.1f"],
                    });
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
            Pending::CombatDamageAssignment { order, cursor, .. } => {
                let attacker = order[*cursor].attacker;
                let name = self.describe_object(attacker);
                vec![
                    LegalAction {
                        action: Action::AnswerCombatDamageAssignment {
                            as_though_unblocked: true,
                        },
                        label: format!(
                            "Have {name} assign its combat damage as though it weren't blocked"
                        ),
                        rules: vec!["510.1", "509.1h"],
                    },
                    LegalAction {
                        action: Action::AnswerCombatDamageAssignment {
                            as_though_unblocked: false,
                        },
                        label: format!("Have {name} assign its combat damage normally"),
                        rules: vec!["510.1"],
                    },
                ]
            }
            Pending::ApnapReveal { player, .. } => {
                let creatures: Vec<ObjectId> = self
                    .state
                    .player(*player)
                    .hand
                    .iter()
                    .copied()
                    .filter(|id| {
                        self.state
                            .try_object(*id)
                            .is_some_and(|object| object.card.def().is_creature())
                    })
                    .collect();
                let mut offers = vec![LegalAction {
                    action: Action::RevealCreatureCards { cards: Vec::new() },
                    label: "Reveal no creature cards".to_string(),
                    rules: vec!["101.4", "701.9a"],
                }];
                if !creatures.is_empty() {
                    offers.push(LegalAction {
                        action: Action::RevealCreatureCards {
                            cards: creatures.clone(),
                        },
                        label: format!(
                            "Reveal {} creature card(s) for {} 2/2 green Bear token(s)",
                            creatures.len(),
                            creatures.len()
                        ),
                        rules: vec!["101.4", "701.9a"],
                    });
                }
                offers
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
            Pending::TriggerTarget {
                legal,
                trigger,
                aggregate,
                chosen,
                ..
            } => {
                if let Some(budget) = aggregate {
                    // CR 601.2c: offer only the additions that still fit the
                    // announced total, plus the commit that ends the choice.
                    let spent: u32 = chosen
                        .iter()
                        .map(|target| self.target_mana_value(*target))
                        .sum();
                    let mut out: Vec<LegalAction> = legal
                        .iter()
                        .filter(|target| !chosen.contains(target))
                        .filter(|target| {
                            spent + self.target_mana_value(**target) <= u32::from(*budget)
                        })
                        .map(|target| LegalAction {
                            action: Action::ChooseTriggerTarget { target: *target },
                            label: format!(
                                "Add {} to the targets ({} of {budget} mana value spent)",
                                self.describe_target(*target),
                                spent + self.target_mana_value(*target)
                            ),
                            rules: vec!["115.1", "601.2c"],
                        })
                        .collect();
                    out.push(LegalAction {
                        action: Action::FinishTriggerTargets,
                        label: if chosen.is_empty() {
                            "Destroy nothing".to_string()
                        } else {
                            format!("Destroy the {} chosen permanent(s)", chosen.len())
                        },
                        rules: vec!["601.2c"],
                    });
                    return out;
                }
                let mut out: Vec<LegalAction> = legal
                    .iter()
                    .map(|target| LegalAction {
                        action: Action::ChooseTriggerTarget { target: *target },
                        label: format!("Target {}", self.describe_target(*target)),
                        rules: vec!["603.3d"],
                    })
                    .collect();
                if trigger_target_is_optional(trigger.card, trigger.ability) {
                    out.push(LegalAction {
                        action: Action::DeclineTriggerTarget,
                        label: "Choose no target for this trigger".to_string(),
                        rules: vec!["115.6"],
                    });
                }
                out
            }
            Pending::SearchLibrary {
                player,
                remaining,
                chosen,
                eligible,
                mode,
                ..
            } => {
                let land_kind = match mode {
                    crate::state::LibrarySearchMode::BasicLandToBattlefield => "basic land",
                    crate::state::LibrarySearchMode::SnowLandToHand => "snow land",
                    crate::state::LibrarySearchMode::AnyLandToBattlefield => "land",
                    crate::state::LibrarySearchMode::CreatureFromTopToBattlefieldAttacking => {
                        "creature card from the look"
                    }
                };
                let mut out = vec![LegalAction {
                    action: Action::SearchLibraryDone,
                    label: if *mode
                        == crate::state::LibrarySearchMode::CreatureFromTopToBattlefieldAttacking
                    {
                        "Finish look and put the rest on the bottom".to_string()
                    } else {
                        "Finish searching".to_string()
                    },
                    rules: if *mode
                        == crate::state::LibrarySearchMode::CreatureFromTopToBattlefieldAttacking
                    {
                        vec!["701.20e"]
                    } else {
                        vec!["701.23"]
                    },
                }];
                if *remaining > 0 {
                    for id in eligible.iter().filter(|id| !chosen.contains(id)) {
                        let name = self.state.object(*id).card.def().name;
                        out.push(LegalAction {
                            action: Action::SearchLibraryPick { card: *id },
                            label: format!("P{player}: choose {name} ({land_kind})"),
                            rules: vec!["701.23"],
                        });
                    }
                }
                out
            }
            Pending::TopCardChoice { can_take, .. } => {
                let mut out = Vec::new();
                if *can_take {
                    out.push(LegalAction {
                        action: Action::AnswerTriggerChoice { use_ability: true },
                        label: "Reveal the looked-at card and put it into hand".to_string(),
                        rules: vec!["701.20a", "701.20e"],
                    });
                }
                out.push(LegalAction {
                    action: Action::AnswerTriggerChoice { use_ability: false },
                    label: "Put the looked-at card on the bottom of the library".to_string(),
                    rules: vec!["701.20e"],
                });
                out
            }
            Pending::TriggerMode { .. } => vec![
                LegalAction {
                    action: Action::AnswerTriggerChoice { use_ability: true },
                    label: "Choose counters: put two +1/+1 counters on target Bear".to_string(),
                    rules: vec!["603.3c", "700.2b"],
                },
                LegalAction {
                    action: Action::AnswerTriggerChoice { use_ability: false },
                    label: "Choose fight: target Bear you control fights another creature"
                        .to_string(),
                    rules: vec!["603.3c", "700.2b", "701.14a"],
                },
            ],
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
            let defender_controls_island = self.controls_island(player);
            let Some((attacker, _)) = attackers.iter().find(|(id, _)| {
                self.state.try_object(*id).is_some_and(|a| {
                    crate::tokens::can_block(
                        a.keywords(),
                        blocker_object.keywords(),
                        defender_controls_island,
                    )
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
                || ability::triggers_on(object.card, TriggerEvent::AnotherCreatureDies)
                    .next()
                    .is_some()
        })
        .map(|object| (object.id, object.card, object.controller))
        .collect()
}

/// One creature death in a batch, recorded while the permanent is still on the
/// battlefield so facts that only hold there survive the zone change.
///
/// `battlefield_id` is the identity watched by "another creature dies" self
/// comparisons; `graveyard_id` is the object the card *is* after the move, which
/// is what the trigger's "exile it" must find (they differ only for a commander,
/// whose zone change creates a new object, CR 400.7). `alone_in_combat` is the
/// printed intervening-if evaluated in the live combat state immediately before
/// this death batch. All simultaneous deaths share the same snapshot; a creature
/// that left combat in an earlier event is absent from that snapshot.
#[derive(Clone, Copy, Debug)]
struct DeathRecord {
    battlefield_id: ObjectId,
    graveyard_id: ObjectId,
    controller: PlayerId,
    was_creature: bool,
    alone_in_combat: bool,
}

/// Live attacking and blocking creatures, snapshotted once BEFORE a death
/// batch. A previous separate departure can change the live membership, but
/// no death in the same simultaneous batch may see another's detach.
#[derive(Clone, Debug)]
struct CombatAloneSnapshot {
    attackers: Vec<ObjectId>,
    /// Distinct live blocking creatures, paired or unpaired; blocking several
    /// attackers does not make one creature count more than once.
    blockers: Vec<ObjectId>,
}

impl CombatAloneSnapshot {
    fn died_alone(&self, id: ObjectId) -> bool {
        let attacking_alone = self.attackers.len() == 1 && self.attackers[0] == id;
        let blocking_alone = self.blockers.len() == 1 && self.blockers[0] == id;
        attacking_alone || blocking_alone
    }
}

/// One activation request: which object, which printed ability and mode, which
/// targets, and which cards the activator names for a card cost. Bundling them
/// keeps validation and application from growing a parameter per printed cost
/// shape.
#[derive(Clone, Copy)]
struct ActivationRequest<'a> {
    source: ObjectId,
    ability: u8,
    mode: u8,
    targets: &'a [Target],
    cost_cards: &'a [ObjectId],
}

/// Every `wanted`-sized combination of `pool` in the pool's own order, used to
/// enumerate the legal ways to pay a "exile N cards from your graveyard" cost.
/// Order is the graveyard's, so the offered actions are deterministic.
fn collect_combinations(
    pool: &[ObjectId],
    wanted: usize,
    start: usize,
    current: &mut Vec<ObjectId>,
    out: &mut Vec<Vec<ObjectId>>,
) {
    if current.len() == wanted {
        out.push(current.clone());
        return;
    }
    for index in start..pool.len() {
        current.push(pool[index]);
        collect_combinations(pool, wanted, index + 1, current, out);
        current.pop();
    }
}

/// CR 107.3a: the largest announced X the engine accepts. A bound is needed
/// because X is carried as a `u8` and an accept-anything value would let a
/// forged action overflow the generic cost; 20 is far above any board this pool
/// can produce, and every real cast is refused earlier by `plan_payment`.
const MAX_X_VALUE: u8 = 20;

fn target_spec_rule(spec: TargetSpec) -> &'static str {
    match spec {
        TargetSpec::None => "CR 601.2c",
        TargetSpec::AnyTarget => "any target: CR 115.4",
        TargetSpec::Player => "target player: CR 115.4",
        TargetSpec::Creature => "target creature: CR 115.1",
        TargetSpec::OpponentCreature => "target creature an opponent controls: CR 115.1",
        TargetSpec::CreatureYouControl => "target creature you control: CR 115.1",
        TargetSpec::AnotherCreatureYouControl => "another target creature you control: CR 115.1",
        TargetSpec::Spell => "target spell: CR 115.1",
        TargetSpec::NoncreatureSpell => "target noncreature spell: CR 115.1",
        TargetSpec::HumanYouControl => "target Human you control: CR 115.1",
        TargetSpec::UpToOneCreatureYouControl => "up to one target creature you control: CR 115.6",
        TargetSpec::Bear => "target Bear: CR 115.1",
        TargetSpec::BearYouControl => "target Bear you control: CR 115.1",
        TargetSpec::UnpairedAnotherCreatureYouControl => {
            "another unpaired creature you control to pair with: CR 702.94a"
        }
        TargetSpec::NoAbilityCreatureCardInYourGraveyard => {
            "target creature card with no abilities in your graveyard: CR 115.1, 110.1"
        }
        TargetSpec::ArtifactOrEnchantment => {
            "any number of target artifacts and/or enchantments: CR 115.1, 601.2c"
        }
    }
}

/// `true` when this card/ability prints the aggregate "any number of target
/// artifacts and/or enchantments" specification. Used by resolution legality
/// (CR 608.2b resolves with one surviving target) and by the target check, so
/// the two can never disagree about what kind of target this ability takes.
fn trigger_targets_a_set(card: CardId, ability: u8) -> bool {
    ability::abilities(card)
        .get(ability as usize)
        .map(Ability::target_spec)
        == Some(TargetSpec::ArtifactOrEnchantment)
}

/// CR 115.6 + CR 702.94a: whether a triggered ability's choice may be declined.
/// A printed "up to one target" is the first case; soulbond's "you may pair"
/// is the second, because declining the pairing choice is how its controller
/// chooses not to pair.
fn trigger_target_is_optional(card: CardId, ability: u8) -> bool {
    matches!(
        ability::abilities(card)
            .get(ability as usize)
            .map(Ability::target_spec),
        Some(TargetSpec::UpToOneCreatureYouControl)
            | Some(TargetSpec::UnpairedAnotherCreatureYouControl)
    )
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

#[cfg(test)]
mod slice17_cleanup_history_tests {
    use super::*;

    #[test]
    fn repeated_cleanup_keeps_earlier_deaths_until_the_turn_number_advances() {
        let decks = vec![
            DeckList::parse_practice("A", "40 Forest\n").expect("deck A"),
            DeckList::parse_practice("B", "40 Forest\n").expect("deck B"),
        ];
        let mut game = Game::new(19_017, &decks).expect("game");
        game.state.creature_died_this_turn = true;
        game.state.nontoken_creature_deaths_this_turn = vec![1, 0];
        game.state.turn.step = Step::Cleanup;
        game.state.turn.priority = Some(P0);
        let mut events = Vec::new();
        game.finish_cleanup(&mut events);
        game.finish_cleanup(&mut events);
        assert!(game.state.creature_died_this_turn);
        assert_eq!(game.state.nontoken_creature_deaths_this_turn, vec![1, 0]);
        assert_eq!(game.state.turn.number, 1, "cleanup is still the same turn");
    }

    #[test]
    fn ayula_removed_trigger_does_not_drop_the_rest_of_the_batch() {
        let decks = vec![
            DeckList::parse_practice("A", "40 Forest\n").expect("deck A"),
            DeckList::parse_practice("B", "40 Forest\n").expect("deck B"),
        ];
        let mut game = Game::new(19_018, &decks).expect("game");
        let mut add = |card: CardId, zone: Zone, controller: PlayerId| {
            let id = ObjectId(game.state.next_object);
            game.state.next_object += 1;
            game.state.objects.insert(
                id,
                Object {
                    id,
                    card,
                    owner: controller,
                    controller,
                    zone,
                    commander: None,
                    tapped: false,
                    damage: 0,
                    summoning_sick: false,
                    pump_power: 0,
                    pump_toughness: 0,
                    plus_one_counters: 0,
                    counters: crate::counters::Counters::default(),
                    paired_with: None,
                },
            );
            id
        };
        let ayula = add(CardId::AyulaQueenAmongBears, Zone::Graveyard, P0);
        let warden = add(CardId::SoulsAttendant, Zone::Battlefield, P0);
        let mut events = Vec::new();
        game.stage_triggers(
            vec![
                TriggerRef {
                    source: ayula,
                    card: CardId::AyulaQueenAmongBears,
                    controller: P0,
                    ability: 0,
                    seq: 0,
                    spell_mana_value: 0,
                    affected: None,
                    dead_object: None,
                    source_pt_lki: None,
                    search_amount: 0,
                    x_value: 0,
                },
                TriggerRef {
                    source: warden,
                    card: CardId::SoulsAttendant,
                    controller: P0,
                    ability: 0,
                    seq: 1,
                    spell_mana_value: 0,
                    affected: None,
                    dead_object: None,
                    source_pt_lki: None,
                    search_amount: 0,
                    x_value: 0,
                },
            ],
            &mut events,
        );
        assert!(
            events.iter().any(|event| matches!(
                event,
                Event::TriggerRemoved {
                    card: CardId::AyulaQueenAmongBears,
                    ..
                }
            )),
            "Ayula has no Bear and must be removed"
        );
        assert!(
            game.state
                .stack
                .iter()
                .any(|item| item.card == CardId::SoulsAttendant),
            "the second trigger must still be placed"
        );
    }

    #[test]
    fn cleanup_expires_regeneration_shields() {
        let decks = vec![
            DeckList::parse_practice("A", "40 Forest\n").expect("deck A"),
            DeckList::parse_practice("B", "40 Forest\n").expect("deck B"),
        ];
        let mut game = Game::new(19_021, &decks).expect("game");
        let id = ObjectId(game.state.next_object);
        game.state.next_object += 1;
        game.state.objects.insert(
            id,
            Object {
                id,
                card: CardId::DragonScarredBear,
                owner: P0,
                controller: P0,
                zone: Zone::Battlefield,
                commander: None,
                tapped: false,
                damage: 1,
                summoning_sick: false,
                pump_power: 0,
                pump_toughness: 0,
                plus_one_counters: 0,
                counters: crate::counters::Counters::default(),
                paired_with: None,
            },
        );
        game.state
            .regeneration_shields
            .push(crate::state::RegenerationShield { object: id });
        game.finish_cleanup(&mut Vec::new());
        assert!(game.state.regeneration_shields.is_empty());
        assert_eq!(game.state.object(id).damage, 0);
    }
}
