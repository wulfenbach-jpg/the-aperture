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
            paired_with: None,
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
        catalog::by_name("Commander's Sphere").is_some(),
        "implemented later in slice 14; no longer excluded"
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

#[test]
fn all_three_artifacts_cast_from_hand_and_activate_same_turn() {
    for (name, cost, lands, mana) in [
        ("Sol Ring", 1usize, 1usize, 2u32),
        ("Mind Stone", 2, 2, 1),
        ("Hedron Archive", 4, 4, 2),
    ] {
        let mut game = fixture(130 + cost as u64)
            .hand(P0, &[card(name)])
            .battlefield(P0, &[CardId::Mountain; 6])
            .main(P0, Step::PrecombatMain)
            .build();
        let id = hand_card(&game, P0, card(name));
        let offered = game.legal_actions();
        assert!(
            offered
                .iter()
                .any(|a| matches!(a.action, Action::CastSpell { card, .. } if card == id)),
            "{name} must be castable from hand"
        );
        submit_ok(
            &mut game,
            P0,
            Action::CastSpell {
                card: id,
                targets: Vec::new(),
            },
        );
        resolve_all(&mut game);
        let (permanent, is_creature, is_artifact) = {
            let object = game
                .state
                .battlefield(P0)
                .into_iter()
                .find(|o| o.card == card(name))
                .expect("artifact resolved onto battlefield");
            (
                object.id,
                object.is_creature(),
                object.card.def().is_artifact(),
            )
        };
        assert!(!is_creature);
        assert!(is_artifact);
        let _ = (cost, lands);
        submit_ok(
            &mut game,
            P0,
            Action::ActivateManaAbility { land: permanent },
        );
        assert_eq!(game.state.player(P0).mana.get(ManaColor::Colorless), mana);
        assert!(game.state.object(permanent).tapped);
    }
}

#[test]
fn artifact_cast_requires_sorcery_timing_and_negate_sees_noncreature() {
    let mut game = fixture(140)
        .hand(P0, &[card("Sol Ring")])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::Upkeep)
        .build();
    let id = hand_card(&game, P0, card("Sol Ring"));
    let err = submit_err(
        &mut game,
        P0,
        Action::CastSpell {
            card: id,
            targets: Vec::new(),
        },
    );
    assert!(
        err.to_string().to_lowercase().contains("timing")
            || err.to_string().to_lowercase().contains("main"),
        "{err}"
    );

    let mut game = fixture(141)
        .hand(P0, &[card("Sol Ring")])
        .battlefield(P0, &[CardId::Mountain])
        .hand(P1, &[CardId::Negate])
        .battlefield(P1, &[CardId::Island, CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let sol = hand_card(&game, P0, card("Sol Ring"));
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: sol,
            targets: Vec::new(),
        },
    );
    let spell = game.state.stack.last().unwrap().object;
    submit_ok(&mut game, P0, Action::Pass);
    let negate = hand_card(&game, P1, CardId::Negate);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: negate,
            targets: vec![Target::Spell(spell)],
        },
    );
    resolve_all(&mut game);
    assert_eq!(game.state.stack.len(), 0);
    assert!(
        game.state.players[P0 as usize]
            .graveyard
            .iter()
            .any(|id| game.state.object(*id).card == card("Sol Ring")),
        "artifact spell is countered as a noncreature spell"
    );
}

#[test]
fn tapping_for_mana_does_not_untap_for_the_sacrifice_draw_ability() {
    let mut game = fixture(142)
        .hand(P0, &[card("Mind Stone")])
        .battlefield(P0, &[CardId::Mountain, CardId::Mountain])
        .library(P0, &[CardId::Forest; 2])
        .main(P0, Step::PrecombatMain)
        .build();
    let stone = hand_card(&game, P0, card("Mind Stone"));
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: stone,
            targets: Vec::new(),
        },
    );
    resolve_all(&mut game);
    let permanent = game
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == card("Mind Stone"))
        .unwrap()
        .id;
    submit_ok(
        &mut game,
        P0,
        Action::ActivateManaAbility { land: permanent },
    );
    assert_eq!(game.state.player(P0).mana.colorless, 1);
    let before = aperture_core::digest::digest(&game.state);
    let err = submit_err(
        &mut game,
        P0,
        Action::ActivateAbility {
            source: permanent,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert!(err.to_string().contains("tapped"), "{err}");
    assert_eq!(aperture_core::digest::digest(&game.state), before);
}

#[test]
fn removed_tokens_are_valid_only_as_inert_archives_of_departed_owners() {
    use aperture_core::tokens::TokenKind;
    let mut game = fixture_n(150, 2).build();
    let token = game.state.create_token(TokenKind::Spirit, P1);
    game.state.players[P1 as usize].lost = true;
    game.state.objects.get_mut(&token).unwrap().zone = Zone::Removed;
    game.state
        .validate()
        .expect("a departed owner's token archive is inert internal storage");
    assert!(!game.legal_actions().iter().any(|a| match &a.action {
        Action::ActivateAbility { source, .. } | Action::ActivateManaAbility { land: source } =>
            *source == token,
        Action::CastSpell { card, .. } => *card == token,
        _ => false,
    }));

    // Live-owner Removed and any token in a real off-battlefield zone are invalid.
    let mut game = fixture_n(151, 2).build();
    let live = game.state.create_token(TokenKind::Spirit, P1);
    game.state.objects.get_mut(&live).unwrap().zone = Zone::Removed;
    assert!(game.state.validate().is_err());

    let mut game = fixture_n(152, 2).build();
    let exiled = game.state.create_token(TokenKind::Spirit, P1);
    game.state.objects.get_mut(&exiled).unwrap().zone = Zone::Exile;
    game.state.players[P1 as usize].exile.push(exiled);
    assert!(game.state.validate().is_err());
}

fn archived_token_state(seed: u64) -> (GameState, ObjectId) {
    use aperture_core::tokens::TokenKind;
    let game = fixture_n(seed, 2).build();
    let mut state = game.state;
    let token = state.create_token(TokenKind::Spirit, P1);
    state.players[P1 as usize].lost = true;
    state.objects.get_mut(&token).unwrap().zone = Zone::Removed;
    (state, token)
}

#[test]
fn archived_tokens_cannot_anchor_live_engine_references() {
    use aperture_core::action::Target;
    use aperture_core::layer::{ContinuousEffect, DamageShield, ShieldTarget};
    use aperture_core::state::{Pending, StackEntry, StackItem, TriggerRef};

    type ArchiveCase = (&'static str, fn(&mut GameState, ObjectId));
    let cases: Vec<ArchiveCase> = vec![
        ("trigger_queue source", |state, id| {
            state.trigger_queue.push(TriggerRef {
                source: id,
                card: CardId::GoblinToken,
                controller: P1,
                ability: 0,
                seq: 0,
                spell_mana_value: 0,
                affected: None,
                source_pt_lki: None,
                search_amount: 0,
            });
        }),
        ("revealed", |state, id| state.revealed.push(id)),
        ("activation_uses", |state, id| {
            state.activation_uses.insert(id, 1);
        }),
        ("stack object", |state, id| {
            state.stack.push(StackItem {
                object: id,
                card: CardId::GoblinToken,
                controller: P1,
                targets: Vec::new(),
                affected: None,
                source_pt_lki: None,
                search_amount: 0,
                cursor: 0,
                entry: StackEntry::Activated { ability: 0 },
                trigger_answered: false,
                spell_mana_value: 0,
            });
        }),
        ("stack target", |state, id| {
            state.stack.push(StackItem {
                object: ObjectId(state.next_object),
                card: CardId::GoblinToken,
                controller: P0,
                targets: vec![Target::Permanent(id)],
                affected: None,
                source_pt_lki: None,
                search_amount: 0,
                cursor: 0,
                entry: StackEntry::Activated { ability: 0 },
                trigger_answered: false,
                spell_mana_value: 0,
            });
        }),
        ("pending trigger", |state, id| {
            state.pending = Some(Pending::TriggerChoice {
                player: P0,
                trigger: TriggerRef {
                    source: id,
                    card: CardId::GoblinToken,
                    controller: P1,
                    ability: 0,
                    seq: 0,
                    spell_mana_value: 0,
                    affected: None,
                    source_pt_lki: None,
                    search_amount: 0,
                },
            });
        }),
        ("resolving", |state, id| state.resolving = Some(id)),
        ("continuous effect target", |state, id| {
            state.effects.push(ContinuousEffect {
                id: 1,
                timestamp: 1,
                source: Some(id),
                card: CardId::GoblinToken,
                controller: P0,
                layer: aperture_core::layer::Layer::Modifying,
                target: id,
                kind: aperture_core::layer::ContinuousKind::ModifyPowerToughness {
                    power: 1,
                    toughness: 1,
                },
                duration: aperture_core::layer::EffectDuration::UntilEndOfTurn,
            });
        }),
        ("shield target", |state, id| {
            state.shields.push(DamageShield {
                id: 1,
                timestamp: 1,
                source: Some(id),
                card: CardId::GoblinToken,
                controller: P0,
                scope: aperture_core::layer::ShieldScope::AnyDamage,
                target: ShieldTarget::Permanent(id),
                remaining: Some(2),
                created_turn: 1,
            });
        }),
    ];
    for (name, apply) in cases {
        let (mut state, id) = archived_token_state(160);
        apply(&mut state, id);
        assert!(
            state.validate().is_err(),
            "archived token must not be referenced by {name}: {:?}",
            state.validate()
        );
    }
}
