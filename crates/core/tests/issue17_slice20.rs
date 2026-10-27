//! Slice 20: Vivien's Grizzly top-card look / reveal-or-bottom.
mod common;

use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn drain(g: &mut Game) {
    for _ in 0..64 {
        if g.state.pending.is_some() {
            return;
        }
        if g.state.stack.is_empty() {
            return;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(g, p, Action::Pass);
    }
    panic!("did not settle");
}

fn drain_events(g: &mut Game) -> Vec<Event> {
    let mut log = Vec::new();
    for _ in 0..64 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return log;
        }
        let p = g.decision_owner().unwrap();
        log.extend(submit_ok(g, p, Action::Pass));
    }
    panic!("did not settle");
}

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

fn vivien_game(seed: u64, library: &[CardId]) -> (Game, ObjectId) {
    let mut g = fixture(seed)
        .hand(P0, &[card("Vivien's Grizzly")])
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .library(P0, library)
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let spell = hand_card(&g, P0, card("Vivien's Grizzly"));
    let cast = g
        .legal_actions()
        .into_iter()
        .find(|legal| {
            matches!(
                legal.action,
                Action::CastSpell { card, .. } if card == spell
            )
        })
        .expect("the hand card must be in the real cast offers")
        .action;
    submit_ok(&mut g, P0, cast);
    drain(&mut g);
    let vivien = g
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == card("Vivien's Grizzly"))
        .expect("Vivien must resolve")
        .id;
    (g, vivien)
}

fn activate(g: &mut Game, vivien: ObjectId) {
    let activation = g
        .legal_actions()
        .into_iter()
        .find(|legal| {
            matches!(
                legal.action,
                Action::ActivateAbility { source, .. } if source == vivien
            )
        })
        .expect("the printed activated ability must be offered")
        .action;
    submit_ok(g, P0, activation);
    drain(g);
}

fn reload(g: &Game) -> Game {
    let decks = vec![
        DeckList::parse_practice("A", "40 Forest\n").unwrap(),
        DeckList::parse_practice("B", "40 Forest\n").unwrap(),
    ];
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        decks,
        AiSettings::default(),
    )
    .expect("replay");
    let save = SessionSave::capture(g, replay, Vec::new(), true);
    let loaded = SessionSave::from_json(&save.to_json()).expect("save loads");
    Game::from_state(loaded.game).expect("loaded game")
}

#[test]
fn vivien_grizzly_is_sourced_and_supported() {
    let card = catalog::by_name("Vivien's Grizzly")
        .expect("Vivien's Grizzly must be supported")
        .def();
    assert_eq!(card.cost, Some("{2}{G}"));
    assert_eq!(card.type_line, "Creature — Bear Spirit");
    assert_eq!(card.power, Some(2));
    assert_eq!(card.toughness, Some(3));
    assert_eq!(
        card.oracle,
        "{3}{G}: Look at the top card of your library. If it's a creature or planeswalker card, you may reveal it and put it into your hand. If you don't put the card into your hand, put it on the bottom of your library."
    );
    assert_eq!(
        aperture_core::ability::abilities(CardId::ViviensGrizzly).len(),
        1
    );
}

#[test]
fn take_reveals_only_the_chosen_card_and_puts_it_into_hand() {
    let (mut g, vivien) = vivien_game(2001, &[CardId::GrizzlyBears, CardId::RuneclawBear]);
    let top = g.state.player(P0).library[0];
    let second = g.state.player(P0).library[1];
    activate(&mut g, vivien);
    assert!(matches!(
        g.state.pending,
        Some(Pending::TopCardChoice { card, can_take: true, .. }) if card == top
    ));
    assert_eq!(g.state.turn.priority, None, "no priority while pending");
    let own = g.view_for(P0, &[]);
    assert_eq!(own.pending_viewed.len(), 1);
    assert_eq!(own.pending_viewed[0].name, "Grizzly Bears");
    let other = g.view_for(P1, &[]);
    assert!(other.pending_viewed.is_empty(), "opponent cannot look");
    assert!(!other.pending.as_deref().unwrap_or("").contains("Grizzly"));
    let hand = g.state.player(P0).hand.len();
    let events = submit_ok(
        &mut g,
        P0,
        Action::AnswerTriggerChoice { use_ability: true },
    );
    assert_eq!(g.state.player(P0).hand.len(), hand + 1);
    assert_eq!(g.state.player(P0).hand.last(), Some(&top));
    assert_eq!(g.state.object(top).zone, Zone::Hand);
    assert_eq!(g.state.player(P0).library, vec![second]);
    assert!(events.iter().any(|e| matches!(
        e,
        Event::StateBasedAction { description } if description.contains("Grizzly Bears") && description.contains("reveals")
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::AbilityResolved {
            card: CardId::ViviensGrizzly,
            ..
        }
    )));
    let after = drain_events(&mut g);
    assert!(!after.iter().any(|e| matches!(e, Event::Truncated { .. })));
    assert!(g.state.stack.is_empty());
    assert!(g.state.pending.is_none());
    assert!(g.state.resolving.is_none());
}

#[test]
fn decline_puts_the_card_on_the_bottom_without_revealing_it() {
    let (mut g, vivien) = vivien_game(
        2002,
        &[CardId::GrizzlyBears, CardId::RuneclawBear, CardId::Forest],
    );
    let top = g.state.player(P0).library[0];
    activate(&mut g, vivien);
    let hand = g.state.player(P0).hand.len();
    let events = submit_ok(
        &mut g,
        P0,
        Action::AnswerTriggerChoice { use_ability: false },
    );
    assert_eq!(g.state.player(P0).hand.len(), hand);
    assert_eq!(g.state.player(P0).library.last(), Some(&top));
    assert!(!events.iter().any(|e| matches!(
        e,
        Event::StateBasedAction { description } if description.contains("Grizzly Bears")
    )));
}

#[test]
fn ineligible_top_card_requires_private_bottom_acknowledgement() {
    let (mut g, vivien) = vivien_game(2003, &[CardId::Forest, CardId::GrizzlyBears]);
    let top = g.state.player(P0).library[0];
    let second = g.state.player(P0).library[1];
    activate(&mut g, vivien);
    assert!(matches!(
        g.state.pending,
        Some(Pending::TopCardChoice { card, can_take: false, .. }) if card == top
    ));
    let own = g.view_for(P0, &[]);
    assert_eq!(own.pending_viewed.len(), 1);
    assert_eq!(own.pending_viewed[0].name, "Forest");
    let other = g.view_for(P1, &[]);
    assert!(other.pending_viewed.is_empty(), "opponent cannot look");
    assert_eq!(
        g.legal_actions()
            .iter()
            .filter(|a| matches!(a.action, Action::AnswerTriggerChoice { .. }))
            .count(),
        1,
        "only the mandatory bottom is offered"
    );
    let before = g.state.clone();
    let err = g
        .submit_action(P0, Action::AnswerTriggerChoice { use_ability: true })
        .expect_err("forged take must be refused");
    assert_eq!(g.state, before);
    assert!(format!("{err:?}").contains("trigger choice"), "{err:?}");
    submit_ok(
        &mut g,
        P0,
        Action::AnswerTriggerChoice { use_ability: false },
    );
    assert_eq!(g.state.player(P0).library, vec![second, top]);
    assert_eq!(g.state.player(P0).hand.len(), 0);
    assert!(g.state.stack.is_empty());
    assert!(g.state.pending.is_none());
    assert!(g.state.resolving.is_none());
}

#[test]
fn empty_library_look_is_not_a_draw_or_loss() {
    let (mut g, vivien) = vivien_game(2004, &[]);
    let life = g.state.player(P0).life;
    activate(&mut g, vivien);
    assert!(g.state.pending.is_none());
    assert_eq!(g.state.player(P0).life, life);
    assert_eq!(g.state.player(P0).library.len(), 0);
    assert!(!g.state.is_over());
}

#[test]
fn pending_rejects_wrong_seat_and_wrong_action_without_mutation() {
    let (mut g, vivien) = vivien_game(2005, &[CardId::GrizzlyBears]);
    activate(&mut g, vivien);
    let before = g.state.clone();
    let decision = g.state.decision;
    let err = g
        .submit_action(P1, Action::AnswerTriggerChoice { use_ability: true })
        .expect_err("wrong seat");
    assert!(format!("{err:?}").contains("Decision"), "{err:?}");
    assert_eq!(g.state, before);
    let err = g
        .submit_action(
            P0,
            Action::ChooseTriggerTarget {
                target: Target::Permanent(vivien),
            },
        )
        .expect_err("wrong action kind");
    assert!(format!("{err:?}").contains("trigger target"), "{err:?}");
    assert_eq!(g.state, before);
    assert_eq!(g.state.decision, decision);
}

#[test]
fn save_reload_mid_choice_finishes_identically() {
    let (mut g, vivien) = vivien_game(2006, &[CardId::GrizzlyBears, CardId::RuneclawBear]);
    activate(&mut g, vivien);
    let mut loaded = reload(&g);
    assert_eq!(loaded.digest(), g.digest());
    assert!(matches!(
        loaded.state.pending,
        Some(Pending::TopCardChoice { .. })
    ));
    for game in [&mut g, &mut loaded] {
        submit_ok(game, P0, Action::AnswerTriggerChoice { use_ability: true });
    }
    assert_eq!(loaded.state, g.state);
    assert_eq!(loaded.digest(), g.digest());
}

#[test]
fn insufficient_mana_never_offers_the_paid_ability() {
    let mut g = fixture(2007)
        .battlefield(
            P0,
            &[
                CardId::ViviensGrizzly,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .library(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let vivien = g
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == CardId::ViviensGrizzly)
        .unwrap()
        .id;
    assert!(!g.legal_actions().iter().any(|a| matches!(
        a.action,
        Action::ActivateAbility { source, .. } if source == vivien
    )));
    let before = g.state.clone();
    let err = g
        .submit_action(
            P0,
            Action::ActivateAbility {
                source: vivien,
                ability: 0,
                mode: 0,
                targets: Vec::new(),
            },
        )
        .expect_err("insufficient mana must be refused");
    assert_eq!(g.state, before);
    assert!(format!("{err:?}").contains("Pay"), "{err:?}");
}

#[test]
fn two_paid_activations_save_at_top_pending_and_both_resolve_once() {
    let mut g = fixture(2008)
        .battlefield(
            P0,
            &[
                CardId::ViviensGrizzly,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .library(P0, &[CardId::GrizzlyBears, CardId::RuneclawBear])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let vivien = g
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == CardId::ViviensGrizzly)
        .unwrap()
        .id;
    for _ in 0..2 {
        let action = g
            .legal_actions()
            .into_iter()
            .find(
                |a| matches!(a.action, Action::ActivateAbility { source, .. } if source == vivien),
            )
            .expect("ability offered")
            .action;
        submit_ok(&mut g, P0, action);
    }
    assert_eq!(
        g.state.stack.len(),
        2,
        "two legal activations share one source"
    );
    submit_ok(&mut g, P0, Action::Pass);
    submit_ok(&mut g, P1, Action::Pass);
    assert!(matches!(
        g.state.pending,
        Some(Pending::TopCardChoice { .. })
    ));
    assert_eq!(g.state.stack.len(), 2, "second activation remains below");
    let mut loaded = reload(&g);
    assert_eq!(loaded.digest(), g.digest());
    for game in [&mut g, &mut loaded] {
        submit_ok(game, P0, Action::AnswerTriggerChoice { use_ability: true });
        drain(game);
        assert!(
            matches!(game.state.pending, Some(Pending::TopCardChoice { .. })),
            "second activation asks once"
        );
        submit_ok(game, P0, Action::AnswerTriggerChoice { use_ability: false });
        drain(game);
    }
    assert_eq!(loaded.state, g.state);
    assert_eq!(loaded.digest(), g.digest());
    assert!(g.state.stack.is_empty());
    assert!(g.state.pending.is_none());
    assert!(g.state.resolving.is_none());
}
