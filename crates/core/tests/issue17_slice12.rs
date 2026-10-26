//! Issue 17 slice 12: artifact mana rocks with immediate mana abilities and
//! stacked sacrifice/draw abilities.

mod common;

use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

fn drained(game: &Game) -> bool {
    game.state.pending.is_none()
        && game.state.resolving.is_none()
        && game.state.stack.is_empty()
        && game.state.trigger_queue.is_empty()
}

fn add_artifact(game: &mut Game, controller: PlayerId, id: CardId) -> ObjectId {
    let oid = ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        oid,
        aperture_core::state::Object {
            id: oid,
            card: id,
            owner: controller,
            controller,
            zone: Zone::Battlefield,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: Default::default(),
        },
    );
    oid
}

fn resolve_all(game: &mut Game) {
    for _ in 0..16 {
        if drained(game) {
            return;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(game, player, Action::Pass);
    }
    assert!(drained(game));
}

#[test]
fn sourced_artifacts_match_printings_and_are_noncreature_permanents() {
    let sol = card("Sol Ring").def();
    assert_eq!(sol.cost, Some("{1}"));
    assert_eq!(sol.type_line, "Artifact");
    assert!(sol.kinds.contains(&Kind::Artifact));
    assert!(!sol.is_creature() && !sol.is_land());
    assert_eq!(sol.oracle, "{T}: Add {C}{C}.");
    let mind = card("Mind Stone").def();
    assert_eq!(
        mind.oracle,
        "{T}: Add {C}.\n{1}, {T}, Sacrifice this artifact: Draw a card."
    );
    let hedron = card("Hedron Archive").def();
    assert_eq!(
        hedron.oracle,
        "{T}: Add {C}{C}.\n{2}, {T}, Sacrifice this artifact: Draw two cards."
    );
    assert!(
        catalog::by_name("Commander's Sphere").is_none(),
        "explicitly excluded"
    );
}

#[test]
fn sol_ring_taps_immediately_for_two_colorless_and_cannot_repeat() {
    let mut game = fixture(120)
        .main(P0, Step::PrecombatMain)
        .library(P0, &[CardId::Forest; 2])
        .build();
    let sol = add_artifact(&mut game, P0, card("Sol Ring"));
    let priority = game.state.turn.priority;
    submit_ok(&mut game, P0, Action::ActivateManaAbility { land: sol });
    assert_eq!(game.state.player(P0).mana.colorless, 2);
    assert!(game.state.object(sol).tapped);
    assert!(game.state.stack.is_empty(), "mana ability uses no stack");
    assert_eq!(game.state.turn.priority, priority, "no priority transfer");
    let before = aperture_core::digest::digest(&game.state);
    let err = submit_err(&mut game, P0, Action::ActivateManaAbility { land: sol });
    assert!(err.to_string().contains("tapped"), "{err}");
    assert_eq!(aperture_core::digest::digest(&game.state), before);
}

#[test]
fn mind_stone_mana_then_sacrifice_draws_on_the_stack() {
    let mut game = fixture(121)
        .main(P0, Step::PrecombatMain)
        .library(P0, &[CardId::Forest; 3])
        .build();
    let stone = add_artifact(&mut game, P0, card("Mind Stone"));
    game.state.players[P0 as usize]
        .mana
        .add(ManaColor::Colorless, 1);
    // {1} + tap + sacrifice uses the ordinary activated-ability stack.
    let hand_before = game.state.player(P0).hand.len();
    assert!(game.legal_actions().iter().any(|a| matches!(
        a.action,
        Action::ActivateAbility { source, .. } if source == stone
    )));
    submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source: stone,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert_eq!(
        game.state.object(stone).zone,
        Zone::Graveyard,
        "sacrifice cost paid"
    );
    assert!(
        !game.state.stack.is_empty(),
        "ordinary activated ability uses the stack"
    );
    resolve_all(&mut game);
    assert_eq!(game.state.player(P0).hand.len(), hand_before + 1);
    assert!(
        game.state.player(P0).mana.is_empty(),
        "the {{1}} cost was paid"
    );
}

#[test]
fn hedron_archive_mana_and_sacrifice_draw_two_with_source_left() {
    let mut game = fixture(122)
        .main(P0, Step::PrecombatMain)
        .library(P0, &[CardId::Forest; 4])
        .build();
    let archive = add_artifact(&mut game, P0, card("Hedron Archive"));
    game.state.players[P0 as usize]
        .mana
        .add(ManaColor::Colorless, 2);
    let hand_before = game.state.player(P0).hand.len();
    submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source: archive,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert_eq!(game.state.object(archive).zone, Zone::Graveyard);
    // Source has left; the ability still resolves on the stack.
    resolve_all(&mut game);
    assert_eq!(game.state.player(P0).hand.len(), hand_before + 2);
    assert!(game.state.player(P0).mana.is_empty());
}

#[test]
fn mind_and_hedron_mana_abilities_produce_printed_amounts() {
    let mut game = fixture(124).main(P0, Step::PrecombatMain).build();
    let stone = add_artifact(&mut game, P0, card("Mind Stone"));
    let archive = add_artifact(&mut game, P0, card("Hedron Archive"));
    submit_ok(&mut game, P0, Action::ActivateManaAbility { land: stone });
    submit_ok(&mut game, P0, Action::ActivateManaAbility { land: archive });
    assert_eq!(game.state.player(P0).mana.colorless, 3);
    assert!(game.state.object(stone).tapped && game.state.object(archive).tapped);
    let before = aperture_core::digest::digest(&game.state);
    assert!(
        submit_err(&mut game, P0, Action::ActivateManaAbility { land: archive })
            .to_string()
            .contains("tapped")
    );
    assert_eq!(aperture_core::digest::digest(&game.state), before);
}

#[test]
fn insufficient_generic_mana_refusal_preserves_state() {
    let mut game = fixture(123)
        .main(P0, Step::PrecombatMain)
        .library(P0, &[CardId::Forest; 2])
        .build();
    let stone = add_artifact(&mut game, P0, card("Mind Stone"));
    let before = aperture_core::digest::digest(&game.state);
    let err = submit_err(
        &mut game,
        P0,
        Action::ActivateAbility {
            source: stone,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert!(err.to_string().to_lowercase().contains("pay"), "{err}");
    assert_eq!(aperture_core::digest::digest(&game.state), before);
}
