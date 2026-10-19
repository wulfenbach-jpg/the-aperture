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
use crate::ids::{ObjectId, P0, P1, PlayerId, Zone, opponent};
use crate::mana::{self, ManaCost, ManaPool, Payment, Sources};
use crate::rng::Pcg32;
use crate::state::{GameState, Object, OrderMode, Outcome, Pending, Player, StackItem, Step, Turn};

/// Safety bound on scheduler iterations inside one submission. Reaching it is a
/// truncation event, never a silently invented result.
const SETTLE_LIMIT: usize = 20_000;
const STARTING_HAND: usize = 7;
/// More mulligans than this could never be bottomed from a seven-card hand.
const MAX_MULLIGANS: u8 = 7;
const STARTING_LIFE: i32 = 20;
const MAX_HAND_SIZE: usize = 7;

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
        tapped: false,
        damage: 0,
        summoning_sick: false,
        pump_power: 0,
        pump_toughness: 0,
        plus_one_counters: 0,
    }
}

impl Game {
    /// Set up a game: build and shuffle both libraries from the deck lists,
    /// deal seven, roll for the first player, then await mulligan decisions.
    pub fn new(seed: u64, deck_a: &DeckList, deck_b: &DeckList) -> Game {
        Game::new_logged(seed, deck_a, deck_b).0
    }

    pub fn new_logged(seed: u64, deck_a: &DeckList, deck_b: &DeckList) -> (Game, Vec<Event>) {
        let mut state = GameState {
            seed,
            first_player: P0,
            rng: Pcg32::new(seed),
            objects: BTreeMap::new(),
            players: [
                Player {
                    id: P0,
                    name: "P0".to_string(),
                    deck_name: deck_a.name.clone(),
                    library: Vec::new(),
                    hand: Vec::new(),
                    graveyard: Vec::new(),
                    exile: Vec::new(),
                    life: STARTING_LIFE,
                    mana: ManaPool::default(),
                    mulligans: 0,
                    drew_from_empty: false,
                    lost: false,
                },
                Player {
                    id: P1,
                    name: "P1".to_string(),
                    deck_name: deck_b.name.clone(),
                    library: Vec::new(),
                    hand: Vec::new(),
                    graveyard: Vec::new(),
                    exile: Vec::new(),
                    life: STARTING_LIFE,
                    mana: ManaPool::default(),
                    mulligans: 0,
                    drew_from_empty: false,
                    lost: false,
                },
            ],
            stack: Vec::new(),
            turn: Turn {
                number: 1,
                active: P0,
                step: Step::Untap,
                priority: None,
                passes: 0,
                lands_played: 0,
                attackers: Vec::new(),
                blocked: Vec::new(),
                blockers: Vec::new(),
                combat_damage_done: false,
            },
            pending: None,
            resolving: None,
            next_object: 1,
            revision: 0,
            decision: 0,
            outcome: None,
            rules_revision: crate::RULES_REVISION.to_string(),
        };

        for (player, deck) in [(P0, deck_a), (P1, deck_b)] {
            let mut ids = Vec::with_capacity(deck.size());
            for card in deck.expand() {
                let id = ObjectId(state.next_object);
                state.next_object += 1;
                state.objects.insert(id, blank_object(id, card, player));
                ids.push(id);
            }
            state.rng.shuffle(&mut ids);
            state.players[player as usize].library = ids;
        }

        let mut game = Game { state };
        let mut events = Vec::new();
        for player in [P0, P1] {
            for _ in 0..STARTING_HAND {
                game.draw(player, &mut events);
            }
        }
        let first = game.state.rng.below(2) as PlayerId;
        game.state.turn.active = first;
        game.state.first_player = first;
        // Turn 1 exists before the first step change, so it is announced here.
        events.push(Event::GameStart { seed, first });
        events.push(Event::TurnBegin {
            turn: 1,
            active: first,
        });
        game.state.pending = Some(Pending::Mulligan { player: P0 });
        (game, events)
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

    fn apply_payment(&mut self, p: PlayerId, payment: Payment, events: &mut Vec<Event>) {
        self.tap_for(&payment.taps, events);
        self.state.players[p as usize].mana = payment.remaining;
    }

    fn empty_mana_pools(&mut self, events: &mut Vec<Event>) {
        for p in [P0, P1] {
            let amount = self.state.players[p as usize].mana.total();
            if amount > 0 {
                self.state.players[p as usize].mana = ManaPool::default();
                events.push(Event::ManaPoolEmptied { player: p, amount });
            }
        }
    }

    fn move_to_graveyard(&mut self, id: ObjectId, events: &mut Vec<Event>) {
        let (owner, card) = match self.state.try_object(id) {
            Some(o) => (o.owner, o.card),
            None => return,
        };
        if let Some(obj) = self.objects_mut(id) {
            obj.zone = Zone::Graveyard;
            obj.tapped = false;
            obj.damage = 0;
            obj.summoning_sick = false;
            obj.pump_power = 0;
            obj.pump_toughness = 0;
        }
        self.state.players[owner as usize].graveyard.push(id);
        events.push(Event::Destroyed { object: id, card });
    }

    fn enter_battlefield(&mut self, id: ObjectId, controller: PlayerId, events: &mut Vec<Event>) {
        let card = match self.state.try_object(id) {
            Some(o) => o.card,
            None => return,
        };
        let is_creature = card.def().is_creature();
        if let Some(obj) = self.objects_mut(id) {
            obj.zone = Zone::Battlefield;
            obj.controller = controller;
            obj.tapped = false;
            obj.damage = 0;
            // CR 302.6: summoning sickness applies until the controller's next
            // untap step.
            obj.summoning_sick = is_creature;
            obj.pump_power = 0;
            obj.pump_toughness = 0;
        }
        events.push(Event::PermanentEntered {
            object: id,
            card,
            controller,
        });
    }

    fn draw(&mut self, p: PlayerId, events: &mut Vec<Event>) {
        let next = self.state.players[p as usize].library.first().copied();
        match next {
            Some(id) => {
                self.state.players[p as usize].library.remove(0);
                self.state.players[p as usize].hand.push(id);
                if let Some(obj) = self.objects_mut(id) {
                    obj.zone = Zone::Hand;
                }
                let card = self.state.object(id).card;
                events.push(Event::Draw {
                    player: p,
                    object: id,
                    card,
                });
            }
            None => {
                self.state.players[p as usize].drew_from_empty = true;
                events.push(Event::DrewFromEmpty { player: p });
            }
        }
    }

    // ------------------------------------------------------------ submission

    pub fn submit(&mut self, sub: &Submission) -> Result<Vec<Event>, EngineError> {
        if self.state.is_over() {
            return Err(EngineError::GameOver);
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
            Action::DeclareAttackers { attackers } => {
                self.validate_declare_attackers(player, attackers)
            }
            Action::DeclareBlockers { blocks } => self.validate_declare_blockers(player, blocks),
            Action::Mulligan { keep } => match &state.pending {
                Some(Pending::Mulligan { player: p }) if *p == player => {
                    if !*keep && state.player(player).mulligans >= MAX_MULLIGANS {
                        return Err(EngineError::illegal(
                            "mulligan",
                            format!(
                                "another mulligan would require bottoming {} cards from a fresh seven; keep instead (CR 103.5)",
                                state.player(player).mulligans + 1
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
                let mut out = vec![Target::Player(P0), Target::Player(P1)];
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
        if obj.zone != Zone::Hand || obj.owner != player {
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
        let cost = def.mana_cost().unwrap_or_else(ManaCost::empty);
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
            Action::DeclareAttackers { attackers } => {
                self.apply_declare_attackers(player, attackers, events)
            }
            Action::DeclareBlockers { blocks } => {
                self.apply_declare_blockers(player, blocks, events)
            }
            Action::Mulligan { keep } => self.apply_mulligan(player, *keep, events),
            Action::BottomCards { cards } => self.apply_bottom(player, cards, events),
            Action::OrderCard { to_top } => self.apply_order_card(player, *to_top, events),
            Action::Discard { cards } => self.apply_discard(player, cards, events),
        }
    }

    fn apply_pass(&mut self, player: PlayerId, events: &mut Vec<Event>) {
        events.push(Event::Pass { player });
        self.state.turn.passes += 1;
        if self.state.turn.passes >= 2 {
            self.state.turn.passes = 0;
            if self.state.stack.is_empty() {
                self.advance_step(events);
            } else {
                self.state.resolving = self.state.stack.last().map(|s| s.object);
                self.state.turn.priority = None;
            }
        } else {
            let next = opponent(player);
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
        self.state.turn.passes = 0;
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
        self.state.turn.passes = 0;
        events.push(Event::ManaTapped {
            player,
            land,
            color: color.unwrap_or(ManaColor::Colorless),
        });
        events.push(Event::Priority { player });
    }

    fn apply_cast(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        targets: &[Target],
        events: &mut Vec<Event>,
    ) {
        let (card_id, cost) = {
            let obj = self.state.object(card);
            (
                obj.card,
                obj.card.mana_cost().unwrap_or_else(ManaCost::empty),
            )
        };
        let payment = self
            .plan_payment(player, &cost)
            .expect("validated payment still plans");
        // CR 601.2a: move the card to the stack; CR 601.2h: pay costs.
        self.state.players[player as usize]
            .hand
            .retain(|c| *c != card);
        if let Some(obj) = self.objects_mut(card) {
            obj.zone = Zone::Stack;
        }
        self.state.stack.push(StackItem {
            object: card,
            controller: player,
            targets: targets.to_vec(),
            cursor: 0,
        });
        let target_labels = targets
            .iter()
            .map(|t| self.describe_target(*t))
            .collect::<Vec<_>>();
        // CR 601.2a: the spell is on the stack before costs are paid (601.2h).
        events.push(Event::Cast {
            player,
            object: card,
            card: card_id,
            targets: target_labels,
        });
        self.apply_payment(player, payment, events);
        // CR 117.3c: the caster receives priority after casting.
        self.state.turn.passes = 0;
        self.state.turn.priority = Some(player);
        events.push(Event::Priority { player });
    }

    fn apply_declare_attackers(
        &mut self,
        player: PlayerId,
        attackers: &[ObjectId],
        events: &mut Vec<Event>,
    ) {
        self.state.pending = None;
        for id in attackers {
            if let Some(obj) = self.objects_mut(*id) {
                obj.tapped = true; // CR 508.1f
            }
        }
        self.state.turn.attackers = attackers.to_vec();
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
        if keep {
            let size = self.state.player(player).hand_size();
            events.push(Event::KeepHand { player, size });
            let mulligans = self.state.player(player).mulligans;
            if mulligans > 0 {
                self.state.pending = Some(Pending::BottomCards {
                    player,
                    count: mulligans,
                });
            } else {
                self.advance_mulligan_order(player, events);
            }
        } else {
            let cards: Vec<ObjectId> = self.state.players[player as usize].hand.drain(..).collect();
            let mut library = std::mem::take(&mut self.state.players[player as usize].library);
            for card in cards {
                if let Some(obj) = self.objects_mut(card) {
                    obj.zone = Zone::Library;
                }
                library.push(card);
            }
            self.state.rng.shuffle(&mut library);
            self.state.players[player as usize].library = library;
            self.state.players[player as usize].mulligans += 1;
            let count = self.state.players[player as usize].mulligans;
            events.push(Event::Mulligan { player, count });
            for _ in 0..STARTING_HAND {
                self.draw(player, events);
            }
            self.state.pending = Some(Pending::Mulligan { player });
        }
    }

    fn apply_bottom(&mut self, player: PlayerId, cards: &[ObjectId], events: &mut Vec<Event>) {
        let mut library = std::mem::take(&mut self.state.players[player as usize].library);
        for card in cards {
            self.state.players[player as usize]
                .hand
                .retain(|c| c != card);
            if let Some(obj) = self.objects_mut(*card) {
                obj.zone = Zone::Library;
            }
            library.push(*card);
        }
        self.state.players[player as usize].library = library;
        events.push(Event::Bottom {
            player,
            count: cards.len(),
        });
        self.state.pending = None;
        self.advance_mulligan_order(player, events);
    }

    /// Mulligan decisions are prompted in seat order (P0 then P1). Real Magic
    /// resolves them simultaneously; the difference is documented.
    fn advance_mulligan_order(&mut self, player: PlayerId, events: &mut Vec<Event>) {
        if player == P0 {
            self.state.pending = Some(Pending::Mulligan { player: P1 });
        } else {
            self.state.pending = None;
            events.push(Event::Priority {
                player: self.state.turn.active,
            });
        }
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
            let card_id = self.state.object(*card).card;
            self.state.players[player as usize]
                .hand
                .retain(|c| c != card);
            if let Some(obj) = self.objects_mut(*card) {
                obj.zone = Zone::Graveyard;
            }
            self.state.players[player as usize].graveyard.push(*card);
            events.push(Event::Discarded {
                player,
                card: card_id,
            });
        }
        self.finish_cleanup(events);
    }

    // ----------------------------------------------------------- scheduler

    fn grant_priority(&mut self, player: PlayerId, events: &mut Vec<Event>) {
        self.state.turn.priority = Some(player);
        self.state.turn.passes = 0;
        events.push(Event::Priority { player });
    }

    /// Drive the game forward until a player must act, the game ends, or a
    /// pending choice blocks progress.
    fn settle(&mut self, events: &mut Vec<Event>) {
        for _ in 0..SETTLE_LIMIT {
            self.check_sbas(events);
            if self.state.is_over() || self.state.pending.is_some() {
                return;
            }
            if self.state.resolving.is_some() {
                self.advance_resolution(events);
                continue;
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
            self.state.turn.active = opponent(self.state.turn.active);
            self.state.turn.lands_played = 0;
            self.state.turn.attackers.clear();
            self.state.turn.blocked.clear();
            self.state.turn.blockers.clear();
            self.state.turn.combat_damage_done = false;
            let (turn, active) = (self.state.turn.number, self.state.turn.active);
            events.push(Event::TurnBegin { turn, active });
        }
        self.state.turn.step = next;
        self.state.turn.priority = None;
        self.state.turn.passes = 0;
        events.push(Event::StepBegin { step: next });
        let active = self.state.turn.active;
        match next {
            Step::Untap => self.perform_untap(active),
            Step::Upkeep => self.grant_priority(active, events),
            Step::Draw => {
                // CR 103.8a: in a two-player game the player who plays first
                // skips the draw step of their first turn.
                let first_turn_of_first_player =
                    self.state.turn.number == 1 && active == self.state.first_player;
                if !first_turn_of_first_player {
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
                let defender = opponent(active);
                if self.state.turn.attackers.is_empty() || self.legal_blockers(defender).is_empty()
                {
                    self.grant_priority(active, events);
                } else {
                    self.state.pending = Some(Pending::DeclareBlockers { player: defender });
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

    fn check_sbas(&mut self, events: &mut Vec<Event>) {
        loop {
            let mut changed = false;
            for p in [P0, P1] {
                let pl = self.state.player(p);
                if pl.lost {
                    continue;
                }
                let reason = if pl.drew_from_empty {
                    Some("attempted to draw from an empty library (CR 704.5b)".to_string())
                } else if pl.life <= 0 {
                    Some(format!("life total is {} (CR 704.5a)", pl.life))
                } else {
                    None
                };
                if let Some(reason) = reason {
                    self.state.players[p as usize].lost = true;
                    events.push(Event::StateBasedAction {
                        description: format!("P{p} loses: {reason}"),
                    });
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

            if !changed {
                break;
            }
            // CR 506.4 + 704.5: anything that just died has left combat.
            self.prune_combat();
            let p0 = self.state.players[P0 as usize].lost;
            let p1 = self.state.players[P1 as usize].lost;
            if p0 || p1 {
                let (winner, loser) = if p0 { (P1, P0) } else { (P0, P1) };
                let reason = if self.state.players[loser as usize].drew_from_empty {
                    "opponent lost by attempting to draw from an empty library (CR 104.3c, 704.5b)"
                        .to_string()
                } else {
                    "opponent's life total reached 0 or less (CR 704.5a)".to_string()
                };
                self.state.outcome = Some(Outcome::Win {
                    winner,
                    reason: reason.clone(),
                });
                self.state.pending = None;
                self.state.resolving = None;
                self.state.turn.priority = None;
                events.push(Event::GameOver { winner, reason });
                return;
            }
        }
    }

    // ------------------------------------------------------------ resolution

    fn targets_legal(&self, item: &StackItem) -> bool {
        for target in &item.targets {
            let ok = match target {
                Target::Player(_) => true,
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

    fn finish_resolution(&mut self, events: &mut Vec<Event>) {
        // CR 117.3b: the active player receives priority after a spell resolves.
        let active = self.state.turn.active;
        self.grant_priority(active, events);
    }

    /// Returns true when the effect created a pending decision.
    fn apply_effect(&mut self, effect: Effect, item: &StackItem, events: &mut Vec<Event>) -> bool {
        let controller = item.controller;
        match effect {
            Effect::Draw(n) => {
                for _ in 0..n {
                    self.draw(controller, events);
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
        let active = self.state.turn.active;
        let defender = opponent(active);
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
                // CR 510.1c: damage is divided among blockers, lethal first.
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

        // Spells, one entry per legal target.
        let hand: Vec<ObjectId> = self.state.player(player).hand.clone();
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
                let label = if targets.is_empty() {
                    format!("Cast {name}")
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
                if self.state.player(*player).mulligans < MAX_MULLIGANS {
                    out.push(LegalAction {
                        action: Action::Mulligan { keep: false },
                        label: format!(
                            "Mulligan (would bottom {} card(s))",
                            self.state.player(*player).mulligans + 1
                        ),
                        rules: vec!["103.5"],
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
                    },
                    label: "Declare no attackers".to_string(),
                    rules: vec!["508.1a"],
                }];
                if !legal.is_empty() {
                    let names: Vec<&str> = legal
                        .iter()
                        .map(|id| self.state.object(*id).card.name())
                        .collect();
                    out.push(LegalAction {
                        action: Action::DeclareAttackers {
                            attackers: legal.clone(),
                        },
                        label: format!("Attack with all eligible creatures ({})", names.join(", ")),
                        rules: vec!["508.1a", "508.1f"],
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
            Pending::Discard { count, .. } => vec![LegalAction {
                action: Action::Discard {
                    cards: self.sample_discard(*count),
                },
                label: format!("Discard the last {count} card(s) of hand"),
                rules: vec!["514.1"],
            }],
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

    fn sample_discard(&self, count: u8) -> Vec<ObjectId> {
        let active = self.state.turn.active;
        let hand = &self.state.player(active).hand;
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
