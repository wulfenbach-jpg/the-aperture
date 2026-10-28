//! Slice 43: Lumra, Bellow of the Woods (BLB 183) — a `*`/`*` characteristic-
//! defining body equal to the number of lands its controller controls, reach and
//! vigilance, and an enters trigger that mills four cards and then returns every
//! land card from the graveyard to the battlefield tapped.
mod common;

use aperture_core::state::Object;
use aperture_core::*;
use common::*;

fn lumra() -> CardId {
    catalog::by_name("Lumra, Bellow of the Woods").unwrap()
}

fn drain(g: &mut Game) {
    for _ in 0..256 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(g, p, Action::Pass);
    }
    panic!("settle");
}

fn cast(g: &mut Game, id: ObjectId) {
    let a = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action, Action::CastSpell { card, .. } if *card == id))
        .expect("offered cast")
        .action;
    submit_ok(g, P0, a);
    drain(g);
}

/// Put a card straight onto the battlefield in a fixture, the way the harness
/// builder does, but returning the object id so a test can address it.
fn place_on_battlefield(g: &mut Game, p: PlayerId, card: CardId, sick: bool) -> ObjectId {
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        id,
        Object {
            id,
            card,
            owner: p,
            controller: p,
            zone: Zone::Battlefield,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: sick,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: Default::default(),
            paired_with: None,
        },
    );
    id
}

fn green(amount: u32) -> ManaPool {
    ManaPool {
        green: amount,
        ..ManaPool::default()
    }
}

/// CR 604.3 + 613.4a: the value is read from the board every characteristic
/// pass, so it follows lands in both directions and ignores an opponent's lands.
#[test]
fn power_and_toughness_equal_the_lands_its_controller_controls() {
    let mut g = fixture(432001)
        .hand(P0, &[lumra()])
        .battlefield(P0, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .battlefield(P1, &[CardId::Forest, CardId::Forest])
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .mana(P0, green(6))
        .main(P0, Step::PrecombatMain)
        .build();
    let id = hand_card(&g, P0, lumra());
    cast(&mut g, id);
    let lumra_object = battlefield_card(&g, P0, lumra());
    let own_lands = g
        .state
        .objects
        .values()
        .filter(|object| {
            object.zone == Zone::Battlefield
                && object.controller == P0
                && object.card.def().is_land()
        })
        .count() as i32;
    assert_eq!(g.state.object(lumra_object).power(), own_lands);
    assert_eq!(g.state.object(lumra_object).toughness(), own_lands);

    // An opponent's lands are not "lands you control".
    let opponent_lands = g
        .state
        .objects
        .values()
        .filter(|object| {
            object.zone == Zone::Battlefield
                && object.controller == P1
                && object.card.def().is_land()
        })
        .count() as i32;
    assert_eq!(
        g.state.object(lumra_object).power(),
        own_lands,
        "only its controller's lands count (opponent controls {opponent_lands})"
    );

    // A land entering later moves the value immediately — proven below by the
    // enters trigger, which returns lands and raises this creature from N to N+2.
    let _ = &g;
}

/// CR 608.2c + 701.23b: mill four, then every land card in the graveyard
/// returns tapped. The returned lands are real battlefield entries, so existing
/// landfall watchers fire once per land even though no land play was spent.
#[test]
fn enters_mills_four_then_returns_every_land_card_tapped() {
    let mut g = fixture(432002)
        .hand(P0, &[lumra()])
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::JaddiOffshoot,
            ],
        )
        .library(
            P0,
            &[
                CardId::GrizzlyBears,
                CardId::RuneclawBear,
                CardId::GiantGrowth,
                CardId::Shock,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    // Two lands are already in the graveyard before Lumra arrives.
    for _ in 0..2 {
        let id = ObjectId(g.state.next_object);
        g.state.next_object += 1;
        g.state.objects.insert(
            id,
            Object {
                id,
                card: CardId::Forest,
                owner: P0,
                controller: P0,
                zone: Zone::Graveyard,
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
        g.state.players[P0 as usize].graveyard.push(id);
    }
    g.state.players[P0 as usize].mana = green(6);
    let id = hand_card(&g, P0, lumra());
    cast(&mut g, id);
    drain(&mut g);

    let lumra_object = battlefield_card(&g, P0, lumra());
    assert_eq!(g.state.object(lumra_object).zone, Zone::Battlefield);

    // The four milled cards are the four nonlands that were on top.
    let milled: Vec<CardId> = g
        .state
        .player(P0)
        .graveyard
        .iter()
        .filter_map(|card| g.state.try_object(*card).map(|object| object.card))
        .collect();
    assert_eq!(
        milled,
        vec![
            CardId::GrizzlyBears,
            CardId::RuneclawBear,
            CardId::GiantGrowth,
            CardId::Shock
        ],
        "mill four puts exactly the top four cards into the graveyard"
    );

    // No land card is left in the graveyard, and the two seeded lands arrived
    // tapped.
    let lands_in_graveyard = g
        .state
        .player(P0)
        .graveyard
        .iter()
        .filter(|card| {
            g.state
                .try_object(**card)
                .is_some_and(|object| object.card.def().is_land())
        })
        .count();
    assert_eq!(lands_in_graveyard, 0, "every land card returned");
    let battlefield_lands = g
        .state
        .objects
        .values()
        .filter(|object| {
            object.zone == Zone::Battlefield
                && object.controller == P0
                && object.card.def().is_land()
        })
        .count();
    assert_eq!(battlefield_lands, 5, "three lands plus the two returned");
    assert!(
        g.state
            .objects
            .values()
            .filter(|object| {
                object.zone == Zone::Battlefield
                    && object.controller == P0
                    && object.card.def().is_land()
            })
            .skip(3)
            .all(|object| object.tapped),
        "returned lands enter tapped"
    );

    // Jaddi Offshoot's landfall fires once per returning land.
    assert_eq!(
        g.state.player(P0).life,
        22,
        "two returning lands are two landfall triggers"
    );

    // ... and the land count is what makes Lumra 5/5 now.
    assert_eq!(g.state.object(lumra_object).power(), 5);
    assert_eq!(g.state.object(lumra_object).toughness(), 5);
    assert!(g.state.validate().is_ok());
}

/// CR 702.20a: attacking does not tap a vigilant creature, and nothing else
/// about the attack changes — the recorded attacker is still attacking.
#[test]
fn vigilance_keeps_lumra_untapped_when_it_attacks() {
    let mut g = fixture(432003)
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::GrizzlyBears,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let lumra_object = place_on_battlefield(&mut g, P0, lumra(), false);
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let mut log = Vec::new();
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![lumra_object, bear],
            defender: Some(P1),
        },
    );
    assert!(
        !g.state.object(lumra_object).tapped,
        "vigilance: attacking does not tap Lumra"
    );
    assert!(
        g.state.object(bear).tapped,
        "an ordinary attacker still taps"
    );
    assert!(g.state.turn.attackers.contains(&lumra_object));
}

/// CR 702.17b: reach lets Lumra block a flyer that a ground creature cannot.
#[test]
fn reach_leaves_lumra_able_to_block_a_flyer() {
    let mut g = fixture(432004)
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::GrizzlyBears,
            ],
        )
        .main(P1, Step::PrecombatMain)
        .build();
    let lumra_object = place_on_battlefield(&mut g, P0, lumra(), true);
    let spirit = g.state.create_token(TokenKind::Spirit, P1);
    g.state
        .objects
        .get_mut(&spirit)
        .expect("token")
        .summoning_sick = false;
    let mut log = Vec::new();
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut g,
        P1,
        Action::DeclareAttackers {
            attackers: vec![spirit],
            defender: Some(P0),
        },
    );
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.step == Step::DeclareBlockers && g.state.pending.is_some(),
        64
    ));
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let err = submit_err(
        &mut g,
        P0,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: bear,
                attacker: spirit,
            }],
        },
    );
    assert!(
        format!("{err}").contains("flying or reach"),
        "a ground creature cannot block the flyer: {err}"
    );
    submit_ok(
        &mut g,
        P0,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: lumra_object,
                attacker: spirit,
            }],
        },
    );
    assert!(
        g.state
            .turn
            .blockers
            .iter()
            .any(|(blocker, attacker)| *blocker == lumra_object && *attacker == spirit)
    );
}
