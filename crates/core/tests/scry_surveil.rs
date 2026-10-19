//! Scry/surveil pending choices, ordering, and the draw that follows.
//! CR 701.22a, 701.25a, 608.2c, 608.2d.

mod common;

use aperture_core::*;
use common::*;

#[test]
fn opt_scries_then_draws_and_respects_the_bottom_choice() {
    let mut game = fixture(20)
        .hand(P0, &[CardId::Opt])
        .battlefield(P0, &[CardId::Island])
        .library(P0, &[CardId::Forest, CardId::Shock, CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let opt = hand_card(&game, P0, CardId::Opt);
    let forest_id = game.state.player(P0).library[0];

    let events = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: opt,
            targets: vec![],
        },
    );
    assert!(
        events.iter().any(|e| matches!(e, Event::Cast { .. })),
        "Opt is cast"
    );
    // The scry decision arrives only when Opt resolves.
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);

    assert!(
        matches!(
            game.state.pending,
            Some(Pending::OrderCard {
                index: 0,
                ref viewed,
                mode: OrderMode::Scry,
                ..
            }) if viewed == &vec![forest_id]
        ),
        "pending scry decision for the top card"
    );
    assert_eq!(
        game.state.turn.priority, None,
        "no priority during a choice"
    );
    assert_eq!(game.decision_owner(), Some(P0));
    assert_eq!(game.legal_actions().len(), 2, "top or bottom, nothing else");

    let events = submit_ok(&mut game, P0, Action::OrderCard { to_top: false });
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::OrderedCard {
                card: CardId::Forest,
                to_top: false,
                ..
            }
        )),
        "the Forest goes to the bottom"
    );
    // Resolution continues: Opt draws the next card.
    assert_eq!(game.state.pending, None);
    assert_eq!(game.state.stack.len(), 0, "Opt finished resolving");
    assert_eq!(
        names(&game, &game.state.player(P0).hand.to_vec()),
        vec!["Shock"],
        "drew the Shock under the bottomed Forest"
    );
    assert_eq!(
        names(&game, &game.state.player(P0).library.to_vec()),
        vec!["Mountain", "Forest"],
        "library order after bottoming"
    );
    assert_eq!(
        names(&game, &game.state.player(P0).graveyard.to_vec()),
        vec!["Opt"]
    );
}

#[test]
fn opt_keeping_the_card_on_top_draws_it() {
    let mut game = fixture(21)
        .hand(P0, &[CardId::Opt])
        .battlefield(P0, &[CardId::Island])
        .library(P0, &[CardId::Forest, CardId::Shock])
        .main(P0, Step::PrecombatMain)
        .build();
    let opt = hand_card(&game, P0, CardId::Opt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: opt,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    submit_ok(&mut game, P0, Action::OrderCard { to_top: true });
    assert_eq!(
        names(&game, &game.state.player(P0).hand.to_vec()),
        vec!["Forest"]
    );
    assert_eq!(
        names(&game, &game.state.player(P0).library.to_vec()),
        vec!["Shock"]
    );
}

#[test]
fn consider_surveils_into_the_graveyard_then_draws() {
    let mut game = fixture(22)
        .hand(P0, &[CardId::Consider])
        .battlefield(P0, &[CardId::Island])
        .library(P0, &[CardId::Mountain, CardId::Shock])
        .main(P0, Step::PrecombatMain)
        .build();
    let consider = hand_card(&game, P0, CardId::Consider);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: consider,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert!(matches!(
        game.state.pending,
        Some(Pending::OrderCard {
            mode: OrderMode::Surveil,
            ..
        })
    ));
    submit_ok(&mut game, P0, Action::OrderCard { to_top: false });
    assert_eq!(
        names(&game, &game.state.player(P0).graveyard.to_vec()),
        vec!["Mountain", "Consider"],
        "surveilled card first, then the resolved spell"
    );
    assert_eq!(
        names(&game, &game.state.player(P0).hand.to_vec()),
        vec!["Shock"],
        "Consider draws after surveilling"
    );
    assert!(game.state.player(P0).library.is_empty());
}

#[test]
fn a_scry_choice_cannot_be_skipped_by_passing() {
    // CR 608.2d — a resolution choice is not priority.
    let mut game = fixture(23)
        .hand(P0, &[CardId::Opt])
        .battlefield(P0, &[CardId::Island])
        .library(P0, &[CardId::Forest, CardId::Shock])
        .main(P0, Step::PrecombatMain)
        .build();
    let opt = hand_card(&game, P0, CardId::Opt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: opt,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    let err = submit_err(&mut game, P0, Action::Pass);
    assert!(matches!(err, EngineError::Illegal { .. }), "{err}");
    let err = submit_err(&mut game, P1, Action::OrderCard { to_top: true });
    assert!(matches!(err, EngineError::NotYourDecision { .. }), "{err}");
}

#[test]
fn scry_with_an_empty_library_does_nothing_and_the_draw_loses() {
    // CR 701.22b-ish (nothing to look at) and CR 704.5b for the draw.
    let mut game = fixture(24)
        .hand(P0, &[CardId::Opt])
        .battlefield(P0, &[CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let opt = hand_card(&game, P0, CardId::Opt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: opt,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert_eq!(
        game.state.pending, None,
        "no scry choice with an empty library"
    );
    match &game.state.outcome {
        Some(Outcome::Win { winner, reason }) => {
            assert_eq!(*winner, P1);
            assert!(reason.contains("704.5b"), "{reason}");
        }
        other => panic!("expected P1 to win, got {other:?}"),
    }
}
