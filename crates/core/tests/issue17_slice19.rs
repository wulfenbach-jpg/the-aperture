//! Issue 17 slice 19 — Spectral Bears.
//!
//! The card is the pool's first attack trigger. It prints an intervening-if
//! ("if defending player controls no black nontoken permanents") and a
//! one-shot untap restriction ("it doesn't untap during your next untap step").
//! These tests assert the printed behavior, not the engine's bookkeeping:
//! whether the trigger is put on the stack at all, which permanents are still
//! tapped after a real untap step, and that the restriction ends after exactly
//! one of the controller's untap steps.

mod common;

use aperture_core::catalog;
use aperture_core::state::{Pending, Step};
use aperture_core::*;
use common::*;

fn bears(game: &Game) -> ObjectId {
    battlefield_card(game, P0, CardId::SpectralBears)
}

/// Drive the game with the only legal "no choice" answers until `done` holds.
/// Blockers are declined and discards take the first legal cards, so the test
/// never supplies a hidden or engine-chosen decision of its own.
fn run_until(game: &mut Game, log: &mut Vec<Event>, mut done: impl FnMut(&Game) -> bool) {
    let mut guard = 0;
    while !done(game) {
        guard += 1;
        assert!(guard < 5000, "the game did not reach the expected point");
        match game.state.pending.clone() {
            Some(Pending::DeclareAttackers { player }) => {
                log.extend(submit_ok(
                    game,
                    player,
                    Action::DeclareAttackers {
                        attackers: Vec::new(),
                        defender: None,
                    },
                ));
            }
            Some(Pending::DeclareBlockers { player }) => {
                log.extend(submit_ok(
                    game,
                    player,
                    Action::DeclareBlockers { blocks: Vec::new() },
                ));
            }
            Some(Pending::Discard { player, count }) => {
                let cards: Vec<ObjectId> = game
                    .state
                    .player(player)
                    .hand
                    .iter()
                    .copied()
                    .take(count as usize)
                    .collect();
                log.extend(submit_ok(game, player, Action::Discard { cards }));
            }
            _ => {
                let player = game.decision_owner().expect("a decision owner while not done");
                log.extend(submit_ok(game, player, Action::Pass));
            }
        }
    }
}

/// The first priority of `seat`'s `turn` after that turn's untap step has run.
fn reached_untap_step(game: &Game, turn: u32) -> bool {
    game.state.turn.number == turn
        && game.state.turn.active == P0
        && game.state.turn.step != Step::Untap
}

#[test]
fn spectral_bears_is_registered_from_its_printed_text() {
    let id = catalog::by_name("Spectral Bears").expect("Spectral Bears must be supported");
    assert_eq!(id, CardId::SpectralBears);
    let def = id.def();
    assert_eq!(def.name, "Spectral Bears");
    assert_eq!(def.cost, Some("{1}{G}"));
    assert_eq!(def.type_line, "Creature — Bear Spirit");
    assert_eq!(def.power, Some(3));
    assert_eq!(def.toughness, Some(3));
    assert_eq!(
        def.oracle,
        "Whenever this creature attacks, if defending player controls no black nontoken permanents, it doesn't untap during your next untap step."
    );
    assert!(def.is_creature());
    assert!(def.has_subtype("Bear"));
    // The printed cost has no black pip and the card is not a token card.
    assert!(!def.is_black());
    assert!(!id.is_token());
    // The printed ability is an attack trigger with the untap effect, not a
    // spell and not an activated ability.
    let abilities = aperture_core::ability::abilities(CardId::SpectralBears);
    assert_eq!(abilities.len(), 1);
    let trigger = abilities[0].triggered().expect("one triggered ability");
    assert_eq!(trigger.event, aperture_core::ability::TriggerEvent::Attacks);
    assert_eq!(trigger.effects, &[Effect::SkipNextUntapStep]);
    assert!(!trigger.optional);
}

#[test]
fn attacking_triggers_and_skips_exactly_one_untap_step() {
    let mut game = fixture(1901)
        .battlefield(P0, &[CardId::SpectralBears, CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::DeclareAttackers)
        .build();
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    let attacker = bears(&game);
    let bystander = battlefield_card(&game, P0, CardId::GrizzlyBears);

    let mut log = submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![attacker],
            defender: Some(P1),
        },
    );
    // CR 603.2: the trigger is queued and put on the stack in the same step.
    assert!(
        log.iter()
            .any(|event| matches!(event, Event::TriggerStaged { .. })),
        "the attack trigger must be staged, got {log:#?}"
    );
    assert!(game.state.stack.len() == 1);

    run_until(&mut game, &mut log, |game| {
        game.state.skip_next_untap.contains(&attacker)
    });
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::StateBasedAction { description }
                if description.contains("next untap step (CR 502.3)")
        )),
        "the skip must be recorded as a rules event, got {log:#?}"
    );

    // P0's next turn (turn 3): the attacker stays tapped, everything else untaps.
    run_until(&mut game, &mut log, |game| reached_untap_step(game, 3));
    assert!(
        game.state.object(attacker).tapped,
        "Spectral Bears must not untap during its controller's next untap step"
    );
    assert!(
        !game.state.object(bystander).tapped,
        "an ordinary creature must untap normally in the same step"
    );
    assert!(!game.state.object(attacker).summoning_sick);
    // The restriction is consumed by that one step.
    assert!(game.state.skip_next_untap.is_empty());

    // P0's following turn (turn 5): the restriction has expired.
    run_until(&mut game, &mut log, |game| reached_untap_step(game, 5));
    assert!(
        !game.state.object(attacker).tapped,
        "the untap restriction is one-shot and must expire"
    );
}

#[test]
fn a_black_nontoken_permanent_on_defense_stops_the_trigger() {
    // Blood Artist is a black creature ({2}{B}), so the intervening-if is false.
    let mut game = fixture(1902)
        .battlefield(P0, &[CardId::SpectralBears])
        .battlefield(P1, &[CardId::BloodArtist])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::DeclareAttackers)
        .build();
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    let attacker = bears(&game);
    assert!(CardId::BloodArtist.def().is_black());

    let mut log = submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![attacker],
            defender: Some(P1),
        },
    );
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::TriggerRemoved { card, reason, .. }
                if *card == CardId::SpectralBears && reason.contains("intervening-if")
        )),
        "a false intervening-if must stop the trigger before the stack, got {log:#?}"
    );
    run_until(&mut game, &mut log, |game| {
        game.state.stack.is_empty() && game.state.pending.is_none()
    });
    assert!(game.state.skip_next_untap.is_empty());
    // Turn 3 is P0's untap step: with no trigger, the bear untaps normally.
    run_until(&mut game, &mut log, |game| reached_untap_step(game, 3));
    assert!(!game.state.object(attacker).tapped);
}

#[test]
fn a_black_token_does_not_stop_the_trigger() {
    // The printed word is "nontoken": a black Pest token is black but must not
    // satisfy the condition that stops the trigger (CR 111.6).
    let mut game = fixture(1903)
        .battlefield(P0, &[CardId::SpectralBears])
        .battlefield(P1, &[CardId::PestToken])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::DeclareAttackers)
        .build();
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    let attacker = bears(&game);
    assert!(CardId::PestToken.def().is_black());
    assert!(CardId::PestToken.is_token());

    let mut log = submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![attacker],
            defender: Some(P1),
        },
    );
    run_until(&mut game, &mut log, |game| {
        game.state.skip_next_untap.contains(&attacker)
    });
    run_until(&mut game, &mut log, |game| reached_untap_step(game, 3));
    assert!(
        game.state.object(attacker).tapped,
        "a black token must not satisfy the nontoken condition"
    );
}

#[test]
fn a_nonattacking_bear_never_gets_the_restriction() {
    // The trigger reads "whenever this creature attacks", so simply having the
    // card on the battlefield must not mark it.
    let mut game = fixture(1904)
        .battlefield(P0, &[CardId::SpectralBears])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let idle = bears(&game);
    let mut log = Vec::new();
    run_until(&mut game, &mut log, |game| reached_untap_step(game, 3));
    assert!(!game.state.object(idle).tapped);
    assert!(game.state.skip_next_untap.is_empty());
}
