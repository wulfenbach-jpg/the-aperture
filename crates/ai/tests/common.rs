//! Test support for the policy crate: a precise state fixture (no validation
//! path) plus the bundled practice decks.
#![allow(dead_code)]

use std::collections::BTreeMap;

use aperture_core::state::{Object, Player, Turn};
use aperture_core::*;

pub const DECK_A: &str = include_str!("../../../decks/temur-beats.txt");
pub const DECK_B: &str = include_str!("../../../decks/izzet-control.txt");

pub fn deck_a() -> DeckList {
    DeckList::parse_practice("Temur Beats", DECK_A).expect("deck A parses")
}

pub fn deck_b() -> DeckList {
    DeckList::parse_practice("Izzet Control", DECK_B).expect("deck B parses")
}

pub struct Fixture {
    state: GameState,
}

fn new_player(id: PlayerId) -> Player {
    Player {
        id,
        name: format!("P{id}"),
        deck_name: format!("fixture-{id}"),
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
    Fixture {
        state: GameState {
            seed,
            first_player: P0,
            rng: aperture_core::rng::Pcg32::new(seed),
            objects: BTreeMap::new(),
            creature_died_this_turn: false,
            draw_replacements: Vec::new(),
            nontoken_creature_deaths_this_turn: Vec::new(),
            commanders: Vec::new(),
            profile: Profile::Practice20,
            commander_damage: Vec::new(),
            retired_commanders: Vec::new(),
            prevent_all_combat_damage_turn: None,
            players: vec![new_player(P0), new_player(P1)],
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
            skip_next_untap: Default::default(),
            effects: Vec::new(),
            shields: Vec::new(),
            regeneration_shields: Vec::new(),
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
                paired_with: None,
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

    pub fn hand(self, p: PlayerId, cards: &[CardId]) -> Self {
        cards.iter().fold(self, |f, c| f.add(p, Zone::Hand, *c))
    }

    pub fn library(self, p: PlayerId, cards: &[CardId]) -> Self {
        cards.iter().fold(self, |f, c| f.add(p, Zone::Library, *c))
    }

    pub fn battlefield(self, p: PlayerId, cards: &[CardId]) -> Self {
        cards
            .iter()
            .fold(self, |f, c| f.add(p, Zone::Battlefield, *c))
    }

    pub fn life(mut self, p: PlayerId, life: i32) -> Self {
        self.state.players[p as usize].life = life;
        self
    }

    pub fn main(mut self, active: PlayerId, step: Step) -> Self {
        self.state.turn.active = active;
        self.state.turn.step = step;
        self.state.turn.priority = Some(active);
        self.state.turn.passed = 0;
        self
    }

    /// Enter the declare-attackers step with the active player's declaration
    /// pending (as the scheduler would).
    pub fn declare_attackers(mut self, active: PlayerId) -> Self {
        self.state.turn.active = active;
        self.state.turn.step = Step::DeclareAttackers;
        self.state.turn.priority = None;
        self.state.pending = Some(Pending::DeclareAttackers { player: active });
        self
    }

    /// Opening-hand decision pending for `player`.
    pub fn mulligan(mut self, player: PlayerId) -> Self {
        self.state.pending = Some(Pending::Mulligan { player });
        self.state.turn.priority = None;
        self
    }

    pub fn mana(mut self, p: PlayerId, pool: ManaPool) -> Self {
        self.state.players[p as usize].mana = pool;
        self
    }

    pub fn priority(mut self, p: Option<PlayerId>) -> Self {
        self.state.turn.priority = p;
        self
    }

    pub fn build(self) -> Game {
        Game { state: self.state }
    }
}

pub fn hand_card(game: &Game, p: PlayerId, card: CardId) -> ObjectId {
    game.state
        .player(p)
        .hand
        .iter()
        .copied()
        .find(|id| game.state.object(*id).card == card)
        .unwrap_or_else(|| panic!("{card:?} not in P{p} hand"))
}

pub fn battlefield_card(game: &Game, p: PlayerId, card: CardId) -> ObjectId {
    game.state
        .battlefield(p)
        .into_iter()
        .find(|o| o.card == card)
        .map(|o| o.id)
        .unwrap_or_else(|| panic!("{card:?} not on P{p} battlefield"))
}

pub fn submit_ok(game: &mut Game, player: PlayerId, action: Action) -> Vec<Event> {
    match game.submit_action(player, action) {
        Ok(events) => events,
        Err(err) => panic!("expected {player} to act, got: {err}"),
    }
}

/// Advance the game (answering pendings with "none"/"keep") until `pred`.
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
        let action = if let Some(pending) = &game.state.pending {
            match pending {
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
                Pending::ApnapReveal { .. } => game
                    .legal_actions()
                    .into_iter()
                    .rfind(|legal| matches!(legal.action, Action::RevealCreatureCards { .. }))
                    .map(|legal| legal.action)
                    .expect("an APNAP reveal option is offered"),
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
                Pending::TriggerTarget { .. }
                | Pending::TriggerChoice { .. }
                | Pending::TriggerMode { .. }
                | Pending::TopCardChoice { .. }
                | Pending::SearchLibrary { .. } => game
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
                    .expect("sample action offered"),
            }
        } else {
            Action::Pass
        };
        match game.submit_action(owner, action) {
            Ok(events) => log.extend(events),
            Err(err) => panic!("driver rejected: {err}"),
        }
    }
    false
}

pub fn view(game: &Game, seat: PlayerId) -> aperture_core::view::PlayerView {
    game.view_for(seat, &[])
}
