//! Shared test support: a precise state fixture and deterministic drivers.
//! Test-only: `Fixture` builds states directly, bypassing validation, so a test
//! can state its premise exactly.
#![allow(dead_code)]

use std::collections::BTreeMap;

use aperture_core::state::{Object, Player, Turn};
use aperture_core::*;

pub const DECK_A: &str = include_str!("../../../../decks/temur-beats.txt");
pub const DECK_B: &str = include_str!("../../../../decks/izzet-control.txt");

pub fn deck_a() -> DeckList {
    DeckList::parse_practice("Temur Beats", DECK_A).expect("bundled deck A parses")
}

pub fn deck_b() -> DeckList {
    DeckList::parse_practice("Izzet Control", DECK_B).expect("bundled deck B parses")
}

pub struct Fixture {
    state: GameState,
}

fn new_player(id: PlayerId, name: &str) -> Player {
    Player {
        id,
        name: name.to_string(),
        deck_name: format!("fixture-{name}"),
        library: Vec::new(),
        hand: Vec::new(),
        graveyard: Vec::new(),
        exile: Vec::new(),
        command_zone: Vec::new(),
        life: 20,
        mana: ManaPool::default(),
        mulligans: 0,
        drew_from_empty: false,
        lost: false,
    }
}

pub fn fixture(seed: u64) -> Fixture {
    fixture_n(seed, 2)
}

/// A valid 2–4 seat fixture with empty zones. Tests fill the seats they use.
pub fn fixture_n(seed: u64, seat_count: usize) -> Fixture {
    assert!((2..=4).contains(&seat_count));
    Fixture {
        state: GameState {
            seed,
            first_player: P0,
            rng: aperture_core::rng::Pcg32::new(seed),
            objects: BTreeMap::new(),
            commanders: Vec::new(),
            profile: Profile::Practice20,
            commander_damage: Vec::new(),
            retired_commanders: Vec::new(),
            prevent_all_combat_damage_turn: None,
            players: (0..seat_count as PlayerId)
                .map(|id| new_player(id, &format!("P{id}")))
                .collect(),
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
            creature_died_this_turn: false,
            nontoken_creature_deaths_this_turn: Vec::new(),
            activation_uses: BTreeMap::new(),
            skip_next_untap: Default::default(),
            effects: Vec::new(),
            shields: Vec::new(),
            next_effect: 1,
            next_timestamp: 1,
            revision: 0,
            decision: 0,
            outcome: None,
            rules_revision: RULES_REVISION.to_string(),
        },
    }
}

impl Fixture {
    pub fn add(mut self, p: PlayerId, zone: Zone, card: CardId) -> Self {
        let id = ObjectId(self.state.next_object);
        self.state.next_object += 1;
        self.state.objects.insert(
            id,
            Object {
                id,
                card,
                owner: p,
                controller: p,
                zone,
                commander: None,
                tapped: false,
                damage: 0,
                summoning_sick: false,
                pump_power: 0,
                pump_toughness: 0,
                plus_one_counters: 0,
                counters: aperture_core::counters::Counters::default(),
            },
        );
        match zone {
            Zone::Hand => self.state.players[p as usize].hand.push(id),
            Zone::Library => self.state.players[p as usize].library.push(id),
            Zone::Graveyard => self.state.players[p as usize].graveyard.push(id),
            Zone::Exile => self.state.players[p as usize].exile.push(id),
            Zone::Battlefield | Zone::Stack | Zone::Command | Zone::Removed => {}
        }
        self
    }

    /// Cards in hand, in the given order.
    pub fn hand(self, p: PlayerId, cards: &[CardId]) -> Self {
        cards.iter().fold(self, |f, c| f.add(p, Zone::Hand, *c))
    }

    /// Library, top first.
    pub fn library(self, p: PlayerId, cards: &[CardId]) -> Self {
        cards.iter().fold(self, |f, c| f.add(p, Zone::Library, *c))
    }

    /// Untapped, un-sick permanents.
    pub fn battlefield(self, p: PlayerId, cards: &[CardId]) -> Self {
        cards
            .iter()
            .fold(self, |f, c| f.add(p, Zone::Battlefield, *c))
    }

    /// Battlefield permanents that entered this turn (summoning sick).
    pub fn battlefield_sick(self, p: PlayerId, cards: &[CardId]) -> Self {
        let mut f = self;
        for card in cards {
            let before = f.state.next_object;
            f = f.add(p, Zone::Battlefield, *card);
            let id = ObjectId(before);
            if let Some(obj) = f.state.objects.get_mut(&id) {
                obj.summoning_sick = true;
            }
        }
        f
    }

    pub fn life(mut self, p: PlayerId, life: i32) -> Self {
        self.state.players[p as usize].life = life;
        self
    }

    pub fn mana(mut self, p: PlayerId, pool: ManaPool) -> Self {
        self.state.players[p as usize].mana = pool;
        self
    }

    /// Set the active seat, step, and that seat's priority.
    pub fn main(mut self, active: PlayerId, step: Step) -> Self {
        self.state.turn.active = active;
        self.state.turn.step = step;
        set_priority(&mut self.state, Some(active));
        self
    }

    pub fn step_no_priority(mut self, active: PlayerId, step: Step) -> Self {
        self.state.turn.active = active;
        self.state.turn.step = step;
        set_priority(&mut self.state, None);
        self
    }

    pub fn build(self) -> Game {
        Game { state: self.state }
    }
}

pub fn set_priority(state: &mut GameState, p: Option<PlayerId>) {
    state.turn.priority = p;
    state.turn.passed = 0;
}

pub fn object_mut(state: &mut GameState, id: ObjectId) -> &mut Object {
    state.objects.get_mut(&id).expect("object exists")
}

/// Submit and expect acceptance.
pub fn submit_ok(game: &mut Game, player: PlayerId, action: Action) -> Vec<Event> {
    match game.submit_action(player, action) {
        Ok(events) => events,
        Err(err) => panic!("expected {player} to be able to act, got: {err}"),
    }
}

/// Submit and expect rejection; returns the error.
pub fn submit_err(game: &mut Game, player: PlayerId, action: Action) -> EngineError {
    match game.submit_action(player, action) {
        Ok(_) => panic!("expected rejection"),
        Err(err) => err,
    }
}

pub fn hand_card(game: &Game, p: PlayerId, card: CardId) -> ObjectId {
    game.state
        .player(p)
        .hand
        .iter()
        .copied()
        .find(|id| game.state.object(*id).card == card)
        .unwrap_or_else(|| panic!("{card:?} not in P{p}'s hand"))
}

pub fn battlefield_card(game: &Game, p: PlayerId, card: CardId) -> ObjectId {
    game.state
        .battlefield(p)
        .into_iter()
        .find(|o| o.card == card)
        .map(|o| o.id)
        .unwrap_or_else(|| panic!("{card:?} not on P{p}'s battlefield"))
}

/// Answer whatever the engine is waiting for using a fixed deterministic
/// policy: keep hands, no attackers, no blockers, put viewed cards on top,
/// take the offered discard/bottom set, otherwise pass.
pub fn auto_answer(game: &Game) -> Action {
    if let Some(pending) = &game.state.pending {
        return match pending {
            Pending::Mulligan { .. } => Action::Mulligan { keep: true },
            Pending::DeclareAttackers { .. } => Action::DeclareAttackers {
                attackers: Vec::new(),
                defender: None,
            },
            Pending::DeclareBlockers { .. } => Action::DeclareBlockers { blocks: Vec::new() },
            Pending::ApnapSacrifice { .. } => game
                .legal_actions()
                .into_iter()
                .find(|legal| matches!(legal.action, Action::ChooseSacrifice { .. }))
                .map(|legal| legal.action)
                .expect("an APNAP sacrifice option is offered"),
            Pending::OrderCard { .. } => Action::OrderCard { to_top: true },
            Pending::CommanderReturn { .. } => Action::AnswerCommanderReturn {
                to_command_zone: true,
            },
            Pending::CommanderReplacement { .. } => Action::AnswerCommanderReplacement {
                to_command_zone: true,
            },
            Pending::CommanderDraw { .. } => Action::AnswerCommanderDraw {
                to_command_zone: true,
            },
            // Declare the first offered target; accept the first offered option
            // for a "you may" trigger (the engine lists "use it" first).
            Pending::TriggerTarget { .. }
            | Pending::TriggerChoice { .. }
            | Pending::TriggerMode { .. }
            | Pending::TopCardChoice { .. } => game
                .legal_actions()
                .into_iter()
                .find(|legal| {
                    matches!(
                        legal.action,
                        Action::ChooseTriggerTarget { .. } | Action::AnswerTriggerChoice { .. }
                    )
                })
                .map(|legal| legal.action)
                .expect("a triggered-ability option is offered"),
            Pending::BottomCards { .. } | Pending::Discard { .. } => game
                .legal_actions()
                .into_iter()
                .find(|a| {
                    matches!(
                        a.action,
                        Action::BottomCards { .. } | Action::Discard { .. }
                    )
                })
                .map(|a| a.action)
                .expect("a sample bottom/discard action is offered"),
        };
    }
    Action::Pass
}

/// Advance the game (answering pendings via `auto_answer`) until `pred` holds.
/// Returns false when the game ends or the iteration cap is hit first.
pub fn run_until(
    game: &mut Game,
    log: &mut Vec<Event>,
    pred: impl Fn(&Game) -> bool,
    cap: usize,
) -> bool {
    for _ in 0..cap {
        if pred(game) {
            return true;
        }
        if game.state.is_over() {
            return false;
        }
        let Some(owner) = game.decision_owner() else {
            return false;
        };
        let action = auto_answer(game);
        match game.submit_action(owner, action) {
            Ok(events) => log.extend(events),
            Err(err) => panic!("scripted driver was rejected: {err}"),
        }
    }
    false
}

/// Keep both opening hands (doing the London bottoming if a mulligan was taken)
/// and return once turn 1 has begun.
pub fn start_game(game: &mut Game, log: &mut Vec<Event>) {
    let ok = run_until(
        game,
        log,
        |g| g.state.pending.is_none() && g.state.turn.priority.is_some(),
        64,
    );
    assert!(ok, "game should reach the first priority of turn 1");
}

pub fn names(game: &Game, ids: &[ObjectId]) -> Vec<&'static str> {
    ids.iter()
        .map(|id| game.state.object(*id).card.name())
        .collect()
}
