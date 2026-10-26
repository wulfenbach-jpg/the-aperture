//! Issue 17 slice 17: Ayula death-history package.

mod common;

use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}
fn drained(g: &Game) -> bool {
    g.state.pending.is_none()
        && g.state.resolving.is_none()
        && g.state.stack.is_empty()
        && g.state.trigger_queue.is_empty()
}
fn drain(g: &mut Game) {
    for _ in 0..32 {
        if drained(g) {
            return;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(g, p, Action::Pass);
    }
    assert!(drained(g));
}
fn add_perm(g: &mut Game, p: PlayerId, c: CardId) -> ObjectId {
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card: c,
            owner: p,
            controller: p,
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
    id
}
fn next_turn(g: &mut Game) {
    let n = g.state.turn.number;
    for _ in 0..600 {
        if g.state.turn.number != n || g.state.is_over() {
            return;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(g, p, Action::Pass);
    }
    panic!("turn did not end");
}
fn kill(g: &mut Game, target: ObjectId) {
    add_perm(g, P0, CardId::Mountain);
    let shock = card("Shock");
    let sid = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        sid,
        aperture_core::state::Object {
            id: sid,
            card: shock,
            owner: P0,
            controller: P0,
            zone: Zone::Hand,
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
    g.state.players[P0 as usize].hand.push(sid);
    submit_ok(
        g,
        P0,
        Action::CastSpell {
            card: sid,
            targets: vec![Target::Permanent(target)],
        },
    );
    drain(g);
    assert!(
        !g.state.battlefield(P1).iter().any(|o| o.id == target)
            && !g.state.battlefield(P0).iter().any(|o| o.id == target)
    );
}

#[test]
fn sourced_cards_match() {
    let t = card("Friendly Teddy").def();
    assert_eq!(t.cost, Some("{2}"));
    assert_eq!(t.type_line, "Artifact Creature — Bear Toy");
    assert!(t.is_artifact() && t.is_creature());
    assert_eq!(
        t.oracle,
        "When this creature dies, each player draws a card."
    );
    let u = card("Ulvenwald Bear").def();
    assert_eq!(u.cost, Some("{2}{G}"));
    assert_eq!(
        u.oracle,
        "Morbid — When this creature enters, if a creature died this turn, put two +1/+1 counters on target creature."
    );
    let c = card("Caller of the Claw").def();
    assert_eq!(c.cost, Some("{2}{G}"));
    assert!(c.has_flash());
    assert_eq!(
        c.oracle,
        "Flash\nWhen this creature enters, create a 2/2 green Bear creature token for each nontoken creature put into your graveyard from the battlefield this turn."
    );
}

#[test]
fn friendly_teddy_dies_each_live_player_draws_after_source_left() {
    let mut g = fixture(207)
        .hand(P0, &[card("Friendly Teddy")])
        .battlefield(P0, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .library(P0, &[CardId::Forest; 3])
        .library(P1, &[CardId::Forest; 3])
        .main(P0, Step::PrecombatMain)
        .build();
    let t = hand_card(&g, P0, card("Friendly Teddy"));
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card: t,
            targets: Vec::new(),
        },
    );
    drain(&mut g);
    let perm = g
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == card("Friendly Teddy"))
        .unwrap()
        .id;
    let b = [g.state.player(P0).hand.len(), g.state.player(P1).hand.len()];
    kill(&mut g, perm);
    assert_eq!(g.state.player(P0).hand.len(), b[0] + 1);
    assert_eq!(g.state.player(P1).hand.len(), b[1] + 1);
}

#[test]
fn ulvenwald_morbid_needs_a_real_death_and_targets_any_controller() {
    // Control: a real Ulvenwald enters with no death this turn. Its actual
    // battlefield object must have zero counters; an empty trigger queue alone
    // would also be true after an incorrect trigger had already resolved.
    let mut g = fixture(208)
        .hand(P0, &[card("Ulvenwald Bear")])
        .battlefield(P0, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let u = hand_card(&g, P0, card("Ulvenwald Bear"));
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card: u,
            targets: Vec::new(),
        },
    );
    drain(&mut g);
    let entered: Vec<_> = g
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == card("Ulvenwald Bear"))
        .collect();
    assert_eq!(
        entered.len(),
        1,
        "Ulvenwald must actually be on the battlefield"
    );
    assert_eq!(
        entered[0]
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        0,
        "no death this turn means no counters"
    );

    // Positive: a different creature dies, then the pre-existing opposing
    // Runeclaw Bear is chosen from the actual TriggerTarget offer.
    let mut g = fixture(209)
        .hand(P0, &[card("Ulvenwald Bear")])
        .battlefield(P0, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .battlefield(P1, &[CardId::GrizzlyBears, CardId::RuneclawBear])
        .main(P0, Step::PrecombatMain)
        .build();
    let victim = battlefield_card(&g, P1, CardId::GrizzlyBears);
    let target = battlefield_card(&g, P1, CardId::RuneclawBear);
    kill(&mut g, victim);
    let u = hand_card(&g, P0, card("Ulvenwald Bear"));
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card: u,
            targets: Vec::new(),
        },
    );
    let mut answered = false;
    for _ in 0..12 {
        if let Some(Pending::TriggerTarget { legal, .. }) = &g.state.pending {
            assert!(
                legal.contains(&Target::Permanent(target)),
                "the pre-existing opposing creature must be an offered target"
            );
            submit_ok(
                &mut g,
                P0,
                Action::ChooseTriggerTarget {
                    target: Target::Permanent(target),
                },
            );
            answered = true;
            break;
        }
        if drained(&g) {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(answered, "morbid must offer its creature target");
    drain(&mut g);
    let target = g.state.object(target);
    assert_eq!(target.zone, Zone::Battlefield, "target must still exist");
    assert_eq!(
        target
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        2,
        "exactly two +1/+1 counters"
    );
}

#[test]
fn token_only_death_enables_morbid_without_counting_as_a_nontoken_death() {
    let mut g = fixture(212)
        .hand(P0, &[card("Ulvenwald Bear")])
        .battlefield(P0, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .battlefield(P1, &[CardId::RuneclawBear])
        .main(P0, Step::PrecombatMain)
        .build();
    let token = add_perm(&mut g, P0, CardId::BearToken);
    let target = battlefield_card(&g, P1, CardId::RuneclawBear);
    kill(&mut g, token);
    assert_eq!(
        g.state
            .nontoken_creature_deaths_this_turn
            .get(P0 as usize)
            .copied()
            .unwrap_or(0),
        0
    );
    let u = hand_card(&g, P0, card("Ulvenwald Bear"));
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card: u,
            targets: Vec::new(),
        },
    );
    let mut answered = false;
    for _ in 0..12 {
        if let Some(Pending::TriggerTarget { legal, .. }) = &g.state.pending {
            assert!(legal.contains(&Target::Permanent(target)));
            submit_ok(
                &mut g,
                P0,
                Action::ChooseTriggerTarget {
                    target: Target::Permanent(target),
                },
            );
            answered = true;
            break;
        }
        if drained(&g) {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(
        answered,
        "a token death is still a creature death for morbid"
    );
    drain(&mut g);
    assert_eq!(
        g.state
            .object(target)
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        2
    );
    assert_eq!(
        g.state
            .nontoken_creature_deaths_this_turn
            .get(P0 as usize)
            .copied()
            .unwrap_or(0),
        0
    );
}

#[test]
fn caller_counts_owned_nontoken_battlefield_deaths_even_if_card_later_leaves() {
    let mut g = fixture(210)
        .hand(P0, &[card("Caller of the Claw")])
        .battlefield(P0, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .library(P0, &[CardId::Forest; 4])
        .main(P0, Step::PrecombatMain)
        .build();
    let a = add_perm(&mut g, P0, CardId::GrizzlyBears);
    let b = add_perm(&mut g, P1, CardId::RuneclawBear);
    // b is controlled by P1 but owned by P1 — not P0's graveyard; change owner for ownership distinction.
    g.state.objects.get_mut(&b).unwrap().owner = P0;
    let tok = add_perm(&mut g, P0, CardId::BearToken);
    kill(&mut g, a);
    kill(&mut g, b);
    kill(&mut g, tok);
    // A milled/discarded creature is not a battlefield death.
    let milled = add_perm(&mut g, P0, CardId::CrawWurm);
    g.state.objects.get_mut(&milled).unwrap().zone = Zone::Graveyard;
    g.state.players[P0 as usize].graveyard.push(milled);
    // One dead card leaves the graveyard before Caller resolves; count remains.
    let moved = g.state.players[P0 as usize]
        .graveyard
        .iter()
        .copied()
        .find(|id| g.state.object(*id).card == CardId::GrizzlyBears)
        .unwrap();
    g.state.players[P0 as usize]
        .graveyard
        .retain(|id| *id != moved);
    g.state.objects.get_mut(&moved).unwrap().zone = Zone::Exile;
    let c = hand_card(&g, P0, card("Caller of the Claw"));
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card: c,
            targets: Vec::new(),
        },
    );
    drain(&mut g);
    let bears: Vec<_> = g
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.name() == "Bear Token")
        .collect();
    assert_eq!(
        bears.len(),
        2,
        "two P0-owned nontoken deaths; token and mill excluded"
    );
}

#[test]
fn death_history_is_reset_at_the_actual_turn_boundary() {
    let mut g = fixture(211)
        .hand(
            P0,
            &[card("Caller of the Claw"), card("Caller of the Claw")],
        )
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .main(P0, Step::PrecombatMain)
        .build();
    let a = add_perm(&mut g, P0, CardId::GrizzlyBears);
    let b = add_perm(&mut g, P1, CardId::RuneclawBear);
    g.state.objects.get_mut(&b).unwrap().owner = P0;
    kill(&mut g, a);
    kill(&mut g, b);
    let first = hand_card(&g, P0, card("Caller of the Claw"));
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card: first,
            targets: Vec::new(),
        },
    );
    drain(&mut g);
    let after_first = g
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == CardId::BearToken)
        .count();
    assert_eq!(after_first, 2);
    next_turn(&mut g);
    for _ in 0..100 {
        if matches!(g.state.turn.step, Step::PrecombatMain)
            && g.decision_owner() == Some(P0)
            && g.state.stack.is_empty()
        {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(!g.state.creature_died_this_turn);
    assert!(g.state.nontoken_creature_deaths_this_turn.is_empty());
    let second = hand_card(&g, P0, card("Caller of the Claw"));
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card: second,
            targets: Vec::new(),
        },
    );
    drain(&mut g);
    assert_eq!(
        g.state
            .battlefield(P0)
            .into_iter()
            .filter(|o| o.card == CardId::BearToken)
            .count(),
        after_first,
        "no deaths this turn"
    );
}
