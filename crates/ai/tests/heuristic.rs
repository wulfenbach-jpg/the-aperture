//! Baseline heuristic behavior: it must actually play the game.

mod common;

use aperture_ai::*;
use aperture_core::view::PlayerView;
use aperture_core::*;
use common::*;

fn decide(game: &Game, seat: PlayerId) -> Decision {
    let view = common::view(game, seat);
    let mut policy = HeuristicPolicy::new();
    policy.decide(&view)
}

#[test]
fn plays_a_land_in_its_own_main_phase() {
    let game = fixture(1)
        .hand(P0, &[CardId::Forest, CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let decision = decide(&game, P0);
    assert!(
        matches!(decision.action, Action::PlayLand { .. }),
        "{:?}",
        decision.action
    );
    assert_eq!(decision.source, DecisionSource::Heuristic);
}

#[test]
fn casts_a_creature_with_its_available_mana() {
    let game = fixture(2)
        .hand(P0, &[CardId::GrizzlyBears])
        .battlefield(P0, &[CardId::Forest, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let decision = decide(&game, P0);
    match decision.action {
        Action::CastSpell { card, targets } => {
            assert_eq!(game.state.object(card).card, CardId::GrizzlyBears);
            assert!(targets.is_empty());
        }
        other => panic!("expected a creature cast, got {other:?}"),
    }
}

#[test]
fn counterspells_the_opponents_creature_spell() {
    let mut game = fixture(3)
        .hand(P1, &[CardId::Counterspell])
        .battlefield(P1, &[CardId::Island, CardId::Island])
        .hand(P0, &[CardId::CentaurCourser])
        .battlefield(P0, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let courser = hand_card(&game, P0, CardId::CentaurCourser);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: courser,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.decision_owner(), Some(P1));
    let decision = decide(&game, P1);
    match decision.action {
        Action::CastSpell { card, targets } => {
            assert_eq!(game.state.object(card).card, CardId::Counterspell);
            assert_eq!(targets.len(), 1);
            assert!(matches!(targets[0], Target::Spell(_)));
        }
        other => panic!("expected a Counterspell, got {other:?}"),
    }
}

#[test]
fn attacks_for_lethal_when_the_defender_cannot_block() {
    let game = fixture(4)
        .battlefield(P0, &[CardId::CentaurCourser])
        .life(P1, 3)
        .declare_attackers(P0)
        .build();
    let decision = decide(&game, P0);
    let courser = battlefield_card(&game, P0, CardId::CentaurCourser);
    match decision.action {
        Action::DeclareAttackers { attackers } => assert_eq!(attackers, vec![courser]),
        other => panic!("expected an attack, got {other:?}"),
    }
}

#[test]
fn does_not_attack_into_a_lethal_blocker_when_it_can_avoid_it() {
    let game = fixture(5)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::HillGiant])
        .declare_attackers(P0)
        .build();
    let decision = decide(&game, P0);
    match decision.action {
        Action::DeclareAttackers { attackers } => {
            assert!(
                attackers.is_empty(),
                "a 2/2 should not attack into an untapped 3/3"
            );
        }
        other => panic!("expected a declaration, got {other:?}"),
    }
}

#[test]
fn blocks_when_the_trade_is_good() {
    let mut game = fixture(6)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::RuneclawBear])
        .main(P1, Step::PrecombatMain)
        .build();
    // P0 (defender) has no priority yet; drive to the declare-blockers step by
    // declaring the attack with the engine directly.
    let attacker = battlefield_card(&game, P1, CardId::RuneclawBear);
    let mut log = Vec::new();
    run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64,
    );
    submit_ok(
        &mut game,
        P1,
        Action::DeclareAttackers {
            attackers: vec![attacker],
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.pending.is_some() && g.state.turn.step == Step::DeclareBlockers,
        64
    ));
    let decision = decide(&game, P0);
    let blocker = battlefield_card(&game, P0, CardId::GrizzlyBears);
    match decision.action {
        Action::DeclareBlockers { blocks } => {
            assert_eq!(blocks.len(), 1);
            assert_eq!(blocks[0].blocker, blocker);
            assert_eq!(blocks[0].attacker, attacker);
        }
        other => panic!("expected a block, got {other:?}"),
    }
}

#[test]
fn keeps_a_three_land_hand_and_mulligans_a_land_flood() {
    let good = fixture(7)
        .hand(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Mountain,
                CardId::GrizzlyBears,
                CardId::Shock,
                CardId::CentaurCourser,
                CardId::HillGiant,
            ],
        )
        .mulligan(P0)
        .build();
    match decide(&good, P0).action {
        Action::Mulligan { keep } => assert!(keep, "3 lands and spells is a keep"),
        other => panic!("expected a mulligan choice, got {other:?}"),
    }

    let flood = fixture(8)
        .hand(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
            ],
        )
        .mulligan(P0)
        .build();
    match decide(&flood, P0).action {
        Action::Mulligan { keep } => assert!(!keep, "seven lands is a mulligan"),
        other => panic!("expected a mulligan choice, got {other:?}"),
    }
}

#[test]
fn bottoms_a_surplus_land_when_scrying() {
    let mut game = fixture(9)
        .hand(P0, &[CardId::Opt])
        .battlefield(P0, &[CardId::Island])
        .library(P0, &[CardId::Forest, CardId::Shock])
        .main(P0, Step::PrecombatMain)
        .build();
    // Give the AI an established board so an extra land is redundant.
    for _ in 0..4 {
        let id = ObjectId(game.state.next_object);
        game.state.next_object += 1;
        game.state.objects.insert(
            id,
            aperture_core::state::Object {
                id,
                card: CardId::Forest,
                owner: P0,
                controller: P0,
                zone: Zone::Battlefield,
                tapped: false,
                damage: 0,
                summoning_sick: false,
                pump_power: 0,
                pump_toughness: 0,
                plus_one_counters: 0,
            },
        );
    }
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
    assert!(matches!(
        game.state.pending,
        Some(Pending::OrderCard { .. })
    ));
    match decide(&game, P0).action {
        Action::OrderCard { to_top } => assert!(!to_top, "an 5th land goes to the bottom"),
        other => panic!("expected an order decision, got {other:?}"),
    }
}

#[test]
fn passes_when_it_has_nothing_useful_to_do() {
    let game = fixture(10)
        .hand(P0, &[CardId::Divination])
        .battlefield(P0, &[CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    assert_eq!(decide(&game, P0).action, Action::Pass);
}

#[test]
fn available_mana_counts_untapped_basics_and_pool() {
    let game = fixture(11)
        .battlefield(P0, &[CardId::Mountain, CardId::Mountain])
        .mana(
            P0,
            ManaPool {
                blue: 1,
                ..ManaPool::default()
            },
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let view: PlayerView = common::view(&game, P0);
    let mana = available_mana(&view);
    assert_eq!(mana.red, 2);
    assert_eq!(mana.blue, 1);
    assert_eq!(mana.total(), 3);
}

#[test]
fn the_policy_only_sees_masked_information() {
    let bank = fixture(12)
        .hand(P1, &[CardId::Counterspell, CardId::Shock])
        .library(P1, &[CardId::Forest, CardId::Mountain])
        .hand(P0, &[CardId::CrawWurm])
        .library(P0, &[CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let mut game = bank;
    game.state.turn.priority = Some(P1);
    let view = common::view(&game, P1);
    let json = view.to_json();
    // Own hand is visible to its owner.
    assert!(json.contains("Counterspell"));
    // Every library card and every opponent hand card stays hidden.
    let mut hidden: Vec<ObjectId> = Vec::new();
    hidden.extend(game.state.player(P0).hand.iter().copied());
    hidden.extend(game.state.player(P0).library.iter().copied());
    hidden.extend(game.state.player(P1).library.iter().copied());
    assert_eq!(hidden.len(), 4);
    for id in hidden {
        let token = format!("\"id\":{}", id.0);
        let leaked = json.match_indices(&token).any(|(at, _)| {
            json[at + token.len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_ascii_digit())
        });
        assert!(!leaked, "hidden object {} leaked into the AI's view", id.0);
    }
    assert_eq!(view.opponent_hand_count, 1);
    assert_eq!(view.opponent_library_count, 1);
    assert_eq!(view.own_library_count, 2);

    // The policy's only input is the projection.
    let mut policy = HeuristicPolicy::new();
    let decision = policy.decide(&view);
    assert_eq!(decision.source, DecisionSource::Heuristic);
}
