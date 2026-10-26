//! Issue 17 slice 8: eight sourced missing cards with existing/new bounded primitives.

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

fn add_permanent(game: &mut Game, controller: PlayerId, id: CardId) -> ObjectId {
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

#[test]
fn sourced_slice8_cards_match_printings() {
    let soul = card("Soul's Attendant").def();
    assert_eq!(soul.cost, Some("{W}"));
    assert_eq!(soul.type_line, "Creature — Human Cleric");
    assert_eq!(
        soul.oracle,
        "Whenever another creature enters, you may gain 1 life."
    );
    let bear = card("Razorclaw Bear").def();
    assert_eq!(bear.cost, Some("{2}{G}{G}"));
    assert_eq!((bear.power, bear.toughness), (Some(3), Some(3)));
    assert_eq!(
        bear.oracle,
        "Whenever this creature becomes blocked, it gets +2/+2 until end of turn."
    );
    let volley = card("Volley Veteran").def();
    assert_eq!(volley.cost, Some("{3}{R}"));
    assert_eq!(
        volley.oracle,
        "When this creature enters, it deals damage to target creature an opponent controls equal to the number of Goblins you control."
    );
    let bright = card("Brightstone Ritual").def();
    assert_eq!(bright.oracle, "Add {R} for each Goblin on the battlefield.");
    let aether = card("Aetherize").def();
    assert_eq!(
        aether.oracle,
        "Return all attacking creatures to their owner's hand."
    );
    let reversal = card("Dramatic Reversal").def();
    assert_eq!(reversal.oracle, "Untap all nonland permanents you control.");
    let teddy = card("Friendly Teddy").def();
    assert_eq!(teddy.type_line, "Artifact Creature — Bear Toy");
    assert_eq!(
        teddy.oracle,
        "When this creature dies, each player draws a card."
    );
}

#[test]
fn souls_attendant_optional_another_creature_enters_gain_life() {
    for accept in [true, false] {
        let mut game = fixture(81)
            .hand(P0, &[CardId::GrizzlyBears])
            .battlefield(
                P0,
                &[CardId::Forest, CardId::Forest, card("Soul's Attendant")],
            )
            .library(P0, &[CardId::Forest; 3])
            .main(P0, Step::PrecombatMain)
            .build();
        let bear = hand_card(&game, P0, CardId::GrizzlyBears);
        let life = game.state.player(P0).life;
        submit_ok(
            &mut game,
            P0,
            Action::CastSpell {
                card: bear,
                targets: Vec::new(),
            },
        );
        for _ in 0..8 {
            if drained(&game) {
                break;
            }
            if matches!(game.state.pending, Some(Pending::TriggerChoice { .. })) {
                submit_ok(
                    &mut game,
                    P0,
                    Action::AnswerTriggerChoice {
                        use_ability: accept,
                    },
                );
                continue;
            }
            let player = game.decision_owner().expect("priority");
            submit_ok(&mut game, player, Action::Pass);
        }
        assert!(drained(&game));
        assert_eq!(
            game.state.player(P0).life,
            if accept { life + 1 } else { life }
        );
    }
}

#[test]
fn razorclaw_bear_gets_plus_two_when_blocked_and_reverts() {
    let mut game = fixture(82)
        .battlefield(P0, &[card("Razorclaw Bear")])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, card("Razorclaw Bear"));
    let blocker = battlefield_card(&game, P1, CardId::GrizzlyBears);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![bear],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::DeclareBlockers { .. })),
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker,
                attacker: bear,
            }],
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.pending.is_none()
            && g.state.stack.is_empty()
            && g.state.trigger_queue.is_empty(),
        64
    ));
    assert_eq!(game.state.object(bear).power(), 5);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.number == 2 && g.state.turn.step == Step::Upkeep,
        256
    ));
    assert_eq!(game.state.object(bear).power(), 3);
}

#[test]
fn volley_veteran_damages_opponent_creature_for_each_goblin() {
    let mut game = fixture(83)
        .hand(P0, &[card("Volley Veteran")])
        .battlefield(
            P0,
            &[
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
            ],
        )
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    add_permanent(&mut game, P0, CardId::GoblinInstigator);
    add_permanent(&mut game, P0, CardId::GoblinInstigator);
    let veteran = hand_card(&game, P0, card("Volley Veteran"));
    let target = battlefield_card(&game, P1, CardId::GrizzlyBears);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: veteran,
            targets: Vec::new(),
        },
    );
    for _ in 0..12 {
        if drained(&game) {
            break;
        }
        if matches!(game.state.pending, Some(Pending::TriggerTarget { .. })) {
            submit_ok(
                &mut game,
                P0,
                Action::ChooseTriggerTarget {
                    target: Target::Permanent(target),
                },
            );
            continue;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    assert!(drained(&game));
    // Three Goblins (two Instigators + Volley Veteran) -> 3 damage to a 2/2.
    assert!(
        game.state.try_object(target).is_none()
            || game.state.object(target).zone == Zone::Graveyard
    );
}

#[test]
fn brightstone_ritual_adds_red_for_each_goblin_on_battlefield() {
    let mut game = fixture(84)
        .hand(P0, &[card("Brightstone Ritual")])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    add_permanent(&mut game, P0, CardId::GoblinInstigator);
    add_permanent(&mut game, P1, CardId::GoblinInstigator);
    add_permanent(&mut game, P1, CardId::GoblinInstigator);
    let spell = hand_card(&game, P0, card("Brightstone Ritual"));
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: spell,
            targets: Vec::new(),
        },
    );
    for _ in 0..6 {
        if drained(&game) {
            break;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    assert_eq!(game.state.player(P0).mana.red, 3);
}

#[test]
fn aetherize_returns_all_attackers_and_dramatic_reversal_untaps_nonlands() {
    let mut game = fixture(85)
        .battlefield(P0, &[CardId::GrizzlyBears, CardId::GrizzlyBears])
        .hand(P1, &[card("Aetherize")])
        .battlefield(
            P1,
            &[
                CardId::Island,
                CardId::Island,
                CardId::Island,
                CardId::Island,
                CardId::GrizzlyBears,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let bears: Vec<ObjectId> = game
        .state
        .battlefield(P0)
        .into_iter()
        .map(|o| o.id)
        .collect();
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: bears.clone(),
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::DeclareBlockers { .. })),
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers { blocks: Vec::new() },
    );
    submit_ok(&mut game, P0, Action::Pass);
    let aether = hand_card(&game, P1, card("Aetherize"));
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: aether,
            targets: Vec::new(),
        },
    );
    for _ in 0..8 {
        if drained(&game) {
            break;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    assert!(
        game.state
            .player(P0)
            .hand
            .iter()
            .any(|id| game.state.object(*id).card == CardId::GrizzlyBears)
    );

    let mut game = fixture(86)
        .hand(P0, &[card("Dramatic Reversal")])
        .battlefield(P0, &[CardId::Island, CardId::Island, CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    game.state.objects.get_mut(&bear).unwrap().tapped = true;
    let spell = hand_card(&game, P0, card("Dramatic Reversal"));
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: spell,
            targets: Vec::new(),
        },
    );
    for _ in 0..6 {
        if drained(&game) {
            break;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    assert!(
        !game.state.object(bear).tapped,
        "nonland permanent untapped"
    );
    assert!(
        game.state
            .battlefield(P0)
            .iter()
            .find(|o| o.card == CardId::Island)
            .unwrap()
            .tapped,
        "lands stay tapped for the cost"
    );
}

#[test]
fn friendly_teddy_death_draws_for_each_player_once() {
    let mut game = fixture(87)
        .hand(P0, &[CardId::Shock])
        .battlefield(P0, &[CardId::Mountain])
        .library(P0, &[CardId::Forest; 3])
        .library(P1, &[CardId::Forest; 3])
        .main(P0, Step::PrecombatMain)
        .build();
    let teddy = add_permanent(&mut game, P1, card("Friendly Teddy"));
    let shock = hand_card(&game, P0, CardId::Shock);
    let before = [
        game.state.player(P0).hand.len(),
        game.state.player(P1).hand.len(),
    ];
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Permanent(teddy)],
        },
    );
    for _ in 0..10 {
        if drained(&game) {
            break;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    assert!(drained(&game));
    assert_eq!(
        game.state.player(P0).hand.len(),
        before[0],
        "cast Shock (-1), Teddy draw (+1)"
    );
    assert_eq!(game.state.player(P1).hand.len(), before[1] + 1);
}

#[test]
fn goblin_war_strike_damages_target_player_for_each_goblin() {
    let mut game = fixture(88)
        .hand(P0, &[card("Goblin War Strike")])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    add_permanent(&mut game, P0, CardId::GoblinInstigator);
    add_permanent(&mut game, P0, CardId::GoblinInstigator);
    add_permanent(&mut game, P1, CardId::GoblinInstigator);
    let spell = hand_card(&game, P0, card("Goblin War Strike"));
    let life = game.state.player(P1).life;
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: spell,
            targets: vec![Target::Player(P1)],
        },
    );
    for _ in 0..8 {
        if drained(&game) {
            break;
        }
        let player = game.decision_owner().expect("priority");
        submit_ok(&mut game, player, Action::Pass);
    }
    assert_eq!(game.state.player(P1).life, life - 2);
}
