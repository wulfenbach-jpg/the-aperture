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

use crate::action::{Action, BlockDecl, LegalAction, Submission, Target};
use crate::card::{CardDef, CardId, Effect, ManaColor, SpellSpec, TargetSpec};
use crate::catalog;
use crate::deck::DeckList;
use crate::digest;
use crate::error::EngineError;
use crate::event::Event;
use crate::ids::{ObjectId, P0, PlayerId, Zone};
use crate::mana::{self, ManaCost, ManaPool, Payment, Sources};
use crate::rng::Pcg32;
use crate::state::{
    DrawResume, GameState, MulliganRound, Object, OrderMode, Outcome, Pending, Player, StackItem,
    Step, Turn,
};

/// Safety bound on scheduler iterations inside one submission. Reaching it is a
/// truncation event, never a silently invented result.
const SETTLE_LIMIT: usize = 20_000;
const STARTING_HAND: usize = 7;
const STARTING_LIFE: i32 = 20;
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

#[derive(Debug)]
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
        let (mut game, events) = Game::new_logged(seed, decks)?;
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

        game.state.command_zone_practice = true;
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
                life: STARTING_LIFE,
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
            command_zone_practice: false,
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
        game.state.validate()?;
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
        let Some(mut object) = self.detach_object(id) else {
            return;
        };
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
            TargetSpec::Creature => self
                .state
                .objects
                .values()
                .filter(|o| o.zone == Zone::Battlefield && o.is_creature())
                .map(|o| Target::Permanent(o.id))
                .collect(),
            TargetSpec::Spell => self
                .state
                .stack
                .iter()
                .filter(|s| s.object != exclude)
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
        if spec.instant {
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
            self.state.turn.priority = Some(next);
            events.push(Event::Priority { player: next });
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
            controller: player,
            targets: targets.to_vec(),
            cursor: 0,
        });
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
        let bottom: Vec<ObjectId> = viewed
            .iter()
            .filter(|v| !kept.contains(v) && self.state.object(**v).zone == Zone::Library)
            .copied()
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
            // No SBA or priority runs for normal play while a choice is
            // outstanding (CR 608.2d). Externally injected sandbox departures
            // still need to purge/resume the queue; this path cannot occur
            // through the legal action interface because no effect changes life
            // during the Crack the Earth choice.
            if self.state.pending.is_some() {
                let injected_departure = self
                    .state
                    .players
                    .iter()
                    .any(|player| !player.lost && (player.life <= 0 || player.drew_from_empty));
                if injected_departure {
                    self.check_sbas(events);
                    if self.state.pending.is_none() && !self.state.is_over() {
                        self.grant_priority(self.state.turn.active, events);
                    }
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
                self.advance_step(events);
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
        let (next, new_turn) = self.state.turn.step.next();
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
                if !first_turn_of_first_player && self.state.is_live(active) {
                    self.draw(active, events); // CR 504.1
                }
                self.grant_priority(active, events);
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
                self.deal_combat_damage(events);
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
        for obj in self.state.objects.values_mut() {
            if obj.zone == Zone::Battlefield {
                if obj.damage > 0 || obj.pump_power != 0 || obj.pump_toughness != 0 {
                    changed = true;
                }
                obj.damage = 0;
                obj.pump_power = 0;
                obj.pump_toughness = 0;
            }
        }
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

        let controlled: Vec<ObjectId> = self
            .state
            .stack
            .iter()
            .filter(|s| s.controller == p)
            .map(|s| s.object)
            .collect();
        self.state.stack.retain(|s| s.controller != p);
        self.state
            .commanders
            .retain(|commander| commander.owner != p);
        if self
            .state
            .resolving
            .is_some_and(|r| controlled.contains(&r))
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
                ..
            }) => {
                if order.contains(&p) || choices.contains_key(&p) {
                    order.retain(|seat| *seat != p);
                    choices.remove(&p);
                    if let Some(next) = order.first().copied() {
                        self.state.pending = Some(Pending::ApnapSacrifice {
                            player: next,
                            order,
                            choices,
                        });
                    } else {
                        // The departing seat was the last required answer:
                        // apply the remaining public choices as one batch.
                        self.apply_apnap_choices(&choices, events);
                    }
                } else {
                    self.state.pending = Some(Pending::ApnapSacrifice {
                        player: order.first().copied().unwrap_or(p),
                        order,
                        choices,
                    });
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

        events.push(Event::StateBasedAction {
            description: format!(
                "P{p} leaves the game; all objects owned by P{p} leave the game (CR 800.4a)"
            ),
        });
    }

    fn check_sbas(&mut self, events: &mut Vec<Event>) {
        loop {
            let mut changed = false;
            for p in 0..self.state.seat_count() as PlayerId {
                if !self.state.is_live(p) {
                    continue;
                }
                let pl = self.state.player(p);
                let reason = if pl.drew_from_empty {
                    Some("attempted to draw from an empty library (CR 704.5b)".to_string())
                } else if pl.life <= 0 {
                    Some(format!("life total is {} (CR 704.5a)", pl.life))
                } else {
                    None
                };
                if let Some(reason) = reason {
                    events.push(Event::StateBasedAction {
                        description: format!("P{p} loses: {reason}"),
                    });
                    self.leave_game(p, events);
                    changed = true;
                }
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
                for id in &dying {
                    self.move_to_graveyard(*id, events);
                }
                events.push(Event::StateBasedAction {
                    description: format!(
                        "{} creature(s) had lethal damage or toughness 0 and were put into the graveyard (CR 704.5f, 704.5g)",
                        dying.len()
                    ),
                });
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
                    let loser_reason = self
                        .state
                        .players
                        .iter()
                        .rfind(|player| player.lost)
                        .map(|player| {
                            if player.drew_from_empty {
                                "opponent lost by attempting to draw from an empty library (CR 104.3c, 704.5b)".to_string()
                            } else {
                                "opponent's life total reached 0 or less (CR 704.5a)".to_string()
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
                Target::Spell(id) => self.state.stack.iter().any(|s| s.object == *id),
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
            let card = self.state.object(item.object).card;
            let def = card.def();
            let Some(spec) = def.spell_spec() else {
                self.move_to_graveyard(item.object, events);
                self.state.stack.pop();
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
                self.state.stack.pop();
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
                self.state.stack.pop();
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
                for target in &item.targets {
                    match target {
                        Target::Player(p) => {
                            self.state.players[*p as usize].life -= n as i32;
                            let life = self.state.players[*p as usize].life;
                            events.push(Event::Damage {
                                source: Some(item.object),
                                target: format!("P{p}"),
                                amount: n as u32,
                            });
                            events.push(Event::LifeChange {
                                player: *p,
                                delta: -(n as i32),
                                life,
                            });
                        }
                        Target::Permanent(id) => {
                            if let Some(obj) = self.objects_mut(*id) {
                                obj.damage += n as u32;
                            }
                            events.push(Event::Damage {
                                source: Some(item.object),
                                target: self.describe_object(*id),
                                amount: n as u32,
                            });
                        }
                        Target::Spell(_) => {}
                    }
                }
                false
            }
            Effect::Pump(power, toughness) => {
                for target in &item.targets {
                    if let Target::Permanent(id) = target
                        && let Some(obj) = self.objects_mut(*id)
                    {
                        obj.pump_power += power;
                        obj.pump_toughness += toughness;
                    }
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
                        let (controller, power, card) = match self.state.try_object(*id) {
                            Some(object) => (object.controller, object.power(), object.card),
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
                        events.push(Event::StateBasedAction {
                            description: format!(
                                "{card} is exiled; P{controller} gains {power} life (now {life})"
                            ),
                        });
                        let _ = exiled;
                    }
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
                let mut remaining = power;
                for (i, blocker) in its_blockers.iter().enumerate() {
                    let assign = if i + 1 == its_blockers.len() {
                        remaining
                    } else {
                        let lethal = match self.state.try_object(*blocker) {
                            Some(b) => (b.toughness() - b.damage as i32).max(1),
                            None => remaining,
                        };
                        remaining.min(lethal).max(0)
                    };
                    remaining -= assign;
                    if assign > 0 {
                        damage.push((*attacker, Some(*blocker), assign as u32));
                    }
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
        for (source, target, amount) in damage {
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
                    self.state.players[defender as usize].life -= amount as i32;
                    let life = self.state.players[defender as usize].life;
                    let card = self.state.object(source).card;
                    events.push(Event::CombatDamage {
                        attacker: source,
                        attacker_card: card,
                        blocker: None,
                        amount,
                    });
                    events.push(Event::Damage {
                        source: Some(source),
                        target: format!("P{defender}"),
                        amount,
                    });
                    events.push(Event::LifeChange {
                        player: defender,
                        delta: -(amount as i32),
                        life,
                    });
                }
                Some(victim) => {
                    if let Some(obj) = self.objects_mut(victim) {
                        obj.damage += amount;
                    }
                    let victim_card = self.state.object(victim).card;
                    let label = format!("{} ({})", victim_card.name(), victim);
                    events.push(Event::Damage {
                        source: Some(source),
                        target: label,
                        amount,
                    });
                    events.push(Event::CombatDamage {
                        attacker: source,
                        attacker_card: self.state.object(source).card,
                        blocker: Some((victim, victim_card)),
                        amount,
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
            let Some((attacker, _)) = attackers.first() else {
                break;
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

fn target_spec_rule(spec: TargetSpec) -> &'static str {
    match spec {
        TargetSpec::None => "CR 601.2c",
        TargetSpec::AnyTarget => "any target: CR 115.4",
        TargetSpec::Creature => "target creature: CR 115.1",
        TargetSpec::Spell => "target spell: CR 115.1",
    }
}

/// Number of implemented cards (used by the UI status line).
pub fn pool_size() -> usize {
    catalog::pool_size()
}
