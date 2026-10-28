//! Land/cast timing, mana payment, mana-pool emptying, and the transactional
//! guarantee that a rejected action leaves the state untouched.
//! CR 117.1a, 305.2, 500.5, 601.2, 605.1a, 701.18a.

mod common;

use aperture_core::state::StackItem;
use aperture_core::*;
use common::*;

fn digest(game: &Game) -> String {
    game.digest()
}

#[test]
fn land_can_be_played_once_in_your_own_main_phase() {
    // CR 305.2, 701.18a
    let mut game = fixture(1)
        .hand(P0, &[CardId::Forest, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let first = hand_card(&game, P0, CardId::Forest);
    submit_ok(&mut game, P0, Action::PlayLand { card: first });
    assert_eq!(game.state.battlefield(P0).len(), 1);
    assert_eq!(game.state.player(P0).hand_size(), 1);
    assert_eq!(game.state.turn.lands_played, 1);

    let second = hand_card(&game, P0, CardId::Forest);
    let err = submit_err(&mut game, P0, Action::PlayLand { card: second });
    assert!(err.reason().contains("already played a land"), "{err}");
    assert_eq!(
        game.state.battlefield(P0).len(),
        1,
        "no second land appeared"
    );
}

#[test]
fn land_cannot_be_played_outside_your_main_phase_or_turn() {
    let mut game = fixture(1)
        .hand(P0, &[CardId::Forest])
        .hand(P1, &[CardId::Forest])
        .main(P0, Step::BeginCombat)
        .build();
    let forest = hand_card(&game, P0, CardId::Forest);
    let err = submit_err(&mut game, P0, Action::PlayLand { card: forest });
    assert!(err.reason().contains("main phase"), "{err}");

    // P1 has priority but is not the active player.
    let mut game = fixture(1)
        .hand(P1, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    set_priority(&mut game.state, Some(P1));
    let forest = hand_card(&game, P1, CardId::Forest);
    let err = submit_err(&mut game, P1, Action::PlayLand { card: forest });
    assert!(err.reason().contains("active player"), "{err}");
}

#[test]
fn land_cannot_be_played_with_a_nonempty_stack() {
    let mut game = fixture(2)
        .hand(P0, &[CardId::Forest, CardId::Shock])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let shock = hand_card(&game, P0, CardId::Shock);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P1)],
        },
    );
    let forest = hand_card(&game, P0, CardId::Forest);
    let err = submit_err(&mut game, P0, Action::PlayLand { card: forest });
    assert!(err.reason().contains("stack must be empty"), "{err}");
}

#[test]
fn sorcery_requires_your_main_phase_with_an_empty_stack() {
    // CR 117.1a
    let mut game = fixture(3)
        .hand(P0, &[CardId::Divination])
        .battlefield(P0, &[CardId::Island, CardId::Island, CardId::Island])
        .main(P0, Step::BeginCombat)
        .build();
    let div = hand_card(&game, P0, CardId::Divination);
    let err = submit_err(
        &mut game,
        P0,
        Action::CastSpell {
            card: div,
            targets: vec![],
        },
    );
    assert!(err.reason().contains("main phase"), "{err}");

    // Not your turn (P1 casts a sorcery during P0's turn).
    let mut game = fixture(3)
        .hand(P1, &[CardId::Divination])
        .battlefield(P1, &[CardId::Island, CardId::Island, CardId::Island])
        .main(P0, Step::PrecombatMain)
        .library(P1, &[CardId::Island, CardId::Island])
        .build();
    set_priority(&mut game.state, Some(P1));
    let div = hand_card(&game, P1, CardId::Divination);
    let err = submit_err(
        &mut game,
        P1,
        Action::CastSpell {
            card: div,
            targets: vec![],
        },
    );
    assert!(err.reason().contains("controller's main phase"), "{err}");
}

#[test]
fn cost_is_paid_on_cast_by_tapping_lands() {
    // CR 601.2h
    let mut game = fixture(4)
        .hand(P0, &[CardId::LightningStrike])
        .battlefield(P0, &[CardId::Mountain, CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let strike = hand_card(&game, P0, CardId::LightningStrike);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: strike,
            targets: vec![Target::Player(P1)],
        },
    );
    let tapped = game
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.tapped)
        .count();
    assert_eq!(tapped, 2, "{{1}}{{R}} needs two mana");
    assert!(game.state.player(P0).mana.is_empty(), "no mana wasted here");
}

#[test]
fn unaffordable_spell_is_rejected_and_state_is_identical() {
    let mut game = fixture(5)
        .hand(P0, &[CardId::LightningStrike])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let before = digest(&game);
    let revision = game.revision();
    let strike = hand_card(&game, P0, CardId::LightningStrike);
    let err = submit_err(
        &mut game,
        P0,
        Action::CastSpell {
            card: strike,
            targets: vec![Target::Player(P1)],
        },
    );
    assert!(matches!(err, EngineError::CannotPay { .. }), "{err}");
    assert_eq!(digest(&game), before, "rejected action must not mutate");
    assert_eq!(game.revision(), revision);
    assert!(
        !game
            .state
            .object(battlefield_card(&game, P0, CardId::Mountain))
            .tapped
    );
}

#[test]
fn illegal_action_attempts_leave_the_state_unchanged() {
    let mut game = fixture(6)
        .hand(P0, &[CardId::Shock, CardId::Divination])
        .battlefield(P0, &[CardId::Mountain, CardId::Island])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let before = digest(&game);
    let shock = hand_card(&game, P0, CardId::Shock);
    let div = hand_card(&game, P0, CardId::Divination);
    let bear = battlefield_card(&game, P1, CardId::GrizzlyBears);

    // 1. Shock with no legal target of the requested kind (a spell target).
    assert!(
        !submit_err(
            &mut game,
            P0,
            Action::CastSpell {
                card: shock,
                targets: vec![Target::Spell(shock)],
            }
        )
        .reason()
        .is_empty()
    );
    // 2. Divination with a target it does not take.
    assert!(
        !submit_err(
            &mut game,
            P0,
            Action::CastSpell {
                card: div,
                targets: vec![Target::Permanent(bear)],
            }
        )
        .reason()
        .is_empty()
    );
    // 3. A creature card played as a land.
    assert!(
        submit_err(&mut game, P0, Action::PlayLand { card: div })
            .reason()
            .contains("not a land")
    );
    // 4. Passing when a player has no priority.
    assert_eq!(
        submit_err(&mut game, P1, Action::Pass),
        EngineError::NotYourDecision {
            expected: P0,
            got: P1
        }
    );
    assert_eq!(digest(&game), before, "no failed attempt mutated the state");
}

#[test]
fn stale_submissions_are_rejected() {
    let mut game = fixture(7).main(P0, Step::PrecombatMain).build();
    let stale = Submission {
        player: P0,
        revision: 99,
        decision: game.decision(),
        action: Action::Pass,
    };
    assert!(matches!(
        game.submit(&stale),
        Err(EngineError::StaleRevision { .. })
    ));
    let stale = Submission {
        player: P0,
        revision: game.revision(),
        decision: 42,
        action: Action::Pass,
    };
    assert!(matches!(
        game.submit(&stale),
        Err(EngineError::StaleDecision { .. })
    ));
    assert_eq!(
        game.state.turn.passed, 0,
        "rejected submissions do not advance"
    );
}

#[test]
fn mana_pool_empties_when_the_step_or_phase_ends() {
    // CR 500.5
    let mut game = fixture(8)
        .battlefield(P0, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let forest = battlefield_card(&game, P0, CardId::Forest);
    submit_ok(&mut game, P0, Action::ActivateManaAbility { land: forest });
    assert_eq!(game.state.player(P0).mana.green, 1);
    assert!(
        game.state.object(forest).tapped,
        "mana ability taps the land"
    );

    let events = submit_ok(&mut game, P0, Action::Pass);
    let events2 = submit_ok(&mut game, P1, Action::Pass);
    let all: Vec<&Event> = events.iter().chain(events2.iter()).collect();
    assert!(
        all.iter()
            .any(|e| matches!(e, Event::ManaPoolEmptied { amount: 1, .. })),
        "mana emptying must be reported"
    );
    assert!(game.state.player(P0).mana.is_empty());
    assert_eq!(game.state.turn.step, Step::BeginCombat);
}

#[test]
fn mana_ability_is_only_for_untapped_basic_lands() {
    let mut game = fixture(10)
        .hand(P0, &[CardId::GrizzlyBears])
        .battlefield(P0, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let forest = battlefield_card(&game, P0, CardId::Forest);
    let bears = hand_card(&game, P0, CardId::GrizzlyBears);
    // Creatures have no mana ability in this pool.
    let err = submit_err(&mut game, P0, Action::ActivateManaAbility { land: bears });
    assert!(matches!(err, EngineError::WrongZone { .. }), "{err}");
    submit_ok(&mut game, P0, Action::ActivateManaAbility { land: forest });
    let err = submit_err(&mut game, P0, Action::ActivateManaAbility { land: forest });
    assert!(err.reason().contains("already tapped"), "{err}");
}

#[test]
fn spell_target_that_is_not_on_the_stack_is_rejected() {
    let mut game = fixture(12)
        .hand(P0, &[CardId::Counterspell])
        .battlefield(P0, &[CardId::Island, CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let counter = hand_card(&game, P0, CardId::Counterspell);
    // No spell on the stack: the target list is empty, so any spell target is
    // not a legal target.
    let err = submit_err(
        &mut game,
        P0,
        Action::CastSpell {
            card: counter,
            targets: vec![Target::Spell(counter)],
        },
    );
    assert!(matches!(err, EngineError::BadTarget { .. }), "{err}");
    assert!(
        game.legal_actions()
            .iter()
            .all(|a| !matches!(a.action, Action::CastSpell { .. })),
        "no castable Counterspell with an empty stack"
    );
    let _ = StackItem {
        object: ObjectId(999),
        card: CardId::GrizzlyBears,
        controller: P0,
        targets: Vec::new(),
        affected: None,
        source_pt_lki: None,
        search_amount: 0,
        cursor: 0,
        entry: StackEntry::Spell,
        trigger_answered: false,
        spell_mana_value: 0,
    };
}

#[test]
fn casting_a_creature_puts_it_onto_the_battlefield_with_summoning_sickness() {
    // CR 302.6, 608.3 — regression: a vanilla creature is a permanent spell
    // even though it has no printed spell text.
    let mut game = fixture(13)
        .hand(P0, &[CardId::GrizzlyBears])
        .battlefield(P0, &[CardId::Forest, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let bears = hand_card(&game, P0, CardId::GrizzlyBears);
    let events = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bears,
            targets: vec![],
        },
    );
    assert!(events.iter().any(|e| matches!(e, Event::Cast { .. })));
    assert_eq!(
        game.state.stack.len(),
        1,
        "the creature spell is on the stack"
    );
    assert_eq!(game.state.player(P0).graveyard.len(), 0);

    let mut later = submit_ok(&mut game, P0, Action::Pass);
    later.extend(submit_ok(&mut game, P1, Action::Pass));
    assert_eq!(game.state.stack.len(), 0);
    assert_eq!(game.state.player(P0).graveyard.len(), 0, "it did not die");
    assert_eq!(
        game.state.battlefield_creatures(P0).len(),
        1,
        "it entered the battlefield"
    );
    let id = battlefield_card(&game, P0, CardId::GrizzlyBears);
    assert!(game.state.object(id).summoning_sick);
    assert!(game.legal_attackers(P0).is_empty(), "summoning sick");
    assert!(
        later
            .iter()
            .any(|e| matches!(e, Event::PermanentEntered { .. })),
        "the permanent entering is reported: {later:?}"
    );
}
