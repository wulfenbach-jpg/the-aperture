//! Slice 30: Drover Grizzly and reusable saddle.
mod common;
use aperture_core::catalog;
#[test]
fn drover_grizzly_is_sourced_and_supported() {
    let def = catalog::by_name("Drover Grizzly").expect("supported").def();
    assert_eq!(def.cost, Some("{2}{G}"));
    assert_eq!(def.type_line, "Creature — Bear Mount");
    assert_eq!(def.power, Some(4));
    assert_eq!(def.toughness, Some(2));
    assert_eq!(
        def.oracle,
        "Whenever this creature attacks while saddled, creatures you control gain trample until end of turn.\nSaddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"
    );
}

use aperture_core::*;
use common::*;
fn card30(n: &str) -> CardId {
    catalog::by_name(n).unwrap()
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
            paired_with: None,
        },
    );
    id
}
fn drain(g: &mut Game) {
    for _ in 0..128 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(g, p, Action::Pass);
    }
    panic!("settle");
}

#[test]
fn saddle_uses_other_untapped_creatures_and_grants_trample_on_attack() {
    let mut g = fixture(3001)
        .battlefield(
            P0,
            &[
                card30("Drover Grizzly"),
                CardId::GrizzlyBears,
                CardId::EliteVanguard,
            ],
        )
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let mount = battlefield_card(&g, P0, card30("Drover Grizzly"));
    let helper = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let human = battlefield_card(&g, P0, CardId::EliteVanguard);
    let offers: Vec<_> = g
        .legal_actions()
        .into_iter()
        .filter(|a| matches!(&a.action,Action::SaddleMount{source,..} if *source==mount))
        .collect();
    assert!(!offers.is_empty(), "saddle must be offered");
    let labels: Vec<String> = offers.iter().map(|a| a.label.clone()).collect();
    let mut unique = labels.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        labels.len(),
        unique.len(),
        "equal subsets need distinct labels: {labels:?}"
    );
    let action = offers
        .into_iter()
        .find(|a| match &a.action {
            Action::SaddleMount { creatures, .. } => {
                creatures.len() == 2 && creatures.contains(&helper) && creatures.contains(&human)
            }
            _ => false,
        })
        .expect("multi-helper offer")
        .action;
    submit_ok(&mut g, P0, action);
    assert!(g.state.object(helper).tapped, "first helper tapped as cost");
    assert!(g.state.object(human).tapped, "second helper tapped as cost");
    drain(&mut g);
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::Saddled && e.target == mount)
    );
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
            attackers: vec![mount],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.stack.is_empty(),
        64
    ));
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::GrantsTrample && e.target == helper)
    );
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::GrantsTrample && e.target == human)
    );
}

#[test]
fn unsaddled_attack_does_not_grant_and_invalid_saddles_are_refused() {
    let mut g = fixture(3002)
        .battlefield(P0, &[card30("Drover Grizzly"), CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let mount = battlefield_card(&g, P0, card30("Drover Grizzly"));
    let helper = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let before = g.state.clone();
    let err = g
        .submit_action(
            P0,
            Action::SaddleMount {
                source: mount,
                creatures: vec![mount],
            },
        )
        .expect_err("self cannot be helper");
    assert_eq!(g.state, before);
    assert!(format!("{err:?}").contains("helpers"));
    g.state.objects.get_mut(&helper).unwrap().tapped = true;
    let before_tapped = g.state.clone();
    let err = g
        .submit_action(
            P0,
            Action::SaddleMount {
                source: mount,
                creatures: vec![helper],
            },
        )
        .expect_err("tapped helper");
    assert_eq!(g.state, before_tapped);
    assert!(format!("{err:?}").contains("helpers"));
    let mut log = Vec::new();
    assert!(run_until(
        &mut g,
        &mut log,
        |g| { g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some() },
        64
    ));
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![mount],
            defender: Some(P1),
        },
    );
    drain(&mut g);
    assert!(
        !g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::GrantsTrample)
    );
}

#[test]
fn saddle_refusals_and_save_reload_preserve_state() {
    let mut g = fixture(3003)
        .battlefield(
            P0,
            &[
                card30("Drover Grizzly"),
                CardId::GrizzlyBears,
                CardId::EliteVanguard,
            ],
        )
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let mount = battlefield_card(&g, P0, card30("Drover Grizzly"));
    let helper = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let opponent = add_perm(&mut g, P1, CardId::EliteVanguard);
    let before = g.state.clone();
    let err = g
        .submit_action(
            P0,
            Action::SaddleMount {
                source: mount,
                creatures: vec![opponent],
            },
        )
        .expect_err("opponent helper");
    assert_eq!(g.state, before);
    assert!(format!("{err:?}").contains("helpers"));
    g.state.objects.get_mut(&helper).unwrap().pump_power = -2;
    let before0 = g.state.clone();
    let err = g
        .submit_action(
            P0,
            Action::SaddleMount {
                source: mount,
                creatures: vec![helper],
            },
        )
        .expect_err("insufficient power");
    assert_eq!(g.state, before0);
    assert!(format!("{err:?}").contains("total power"));
    g.state.objects.get_mut(&helper).unwrap().pump_power = 0;
    submit_ok(
        &mut g,
        P0,
        Action::SaddleMount {
            source: mount,
            creatures: vec![helper],
        },
    );
    drain(&mut g);
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::Saddled && e.target == mount)
    );
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
    .unwrap();
    let save = SessionSave::capture(&g, replay, Vec::new(), true);
    let loaded = SessionSave::from_json(&save.to_json()).expect("saddle state loads");
    let loaded = Game::from_state(loaded.game).unwrap();
    assert_eq!(loaded.digest(), g.digest());
    assert!(
        loaded
            .state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::Saddled && e.target == mount)
    );
}

#[test]
fn animated_stuffed_bear_is_a_helper_and_trample_recipient() {
    let mut g = fixture(3004)
        .battlefield(
            P0,
            &[
                card30("Drover Grizzly"),
                CardId::StuffedBear,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let mount = battlefield_card(&g, P0, card30("Drover Grizzly"));
    let bear = battlefield_card(&g, P0, CardId::StuffedBear);
    assert!(!g.state.object(bear).is_creature());
    let anim = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::ActivateAbility { source, .. } if source == bear))
        .expect("animation offered")
        .action;
    submit_ok(&mut g, P0, anim);
    drain(&mut g);
    assert!(g.legal_actions().iter().any(|a| matches!(&a.action, Action::SaddleMount { creatures, .. } if creatures == &vec![bear])), "animated Bear is an eligible helper");
    submit_ok(
        &mut g,
        P0,
        Action::SaddleMount {
            source: mount,
            creatures: vec![bear],
        },
    );
    drain(&mut g);
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
            attackers: vec![mount],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.stack.is_empty(),
        64
    ));
    assert!(
        g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::GrantsTrample && e.target == bear),
        "animated Bear receives trample"
    );
}

#[test]
fn saddle_cleanup_and_source_left_activation_do_not_persist() {
    // Cleanup expiry.
    let mut g = fixture(3005)
        .battlefield(P0, &[card30("Drover Grizzly"), CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let mount = battlefield_card(&g, P0, card30("Drover Grizzly"));
    let helper = battlefield_card(&g, P0, CardId::GrizzlyBears);
    submit_ok(
        &mut g,
        P0,
        Action::SaddleMount {
            source: mount,
            creatures: vec![helper],
        },
    );
    drain(&mut g);
    let turn = g.state.turn.number;
    for _ in 0..1000 {
        if g.state.turn.number != turn {
            break;
        }
        if g.state.pending.is_some() {
            let a = auto_answer(&g);
            let p = g.decision_owner().unwrap();
            submit_ok(&mut g, p, a);
            continue;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(
        !g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::Saddled),
        "cleanup ends saddled"
    );

    // Source leaves while saddle is on the stack: resolves without marking a new incarnation.
    let mut g = fixture(3006)
        .hand(P0, &[CardId::Unsummon])
        .battlefield(
            P0,
            &[
                card30("Drover Grizzly"),
                CardId::GrizzlyBears,
                CardId::Island,
            ],
        )
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let mount = battlefield_card(&g, P0, card30("Drover Grizzly"));
    let helper = battlefield_card(&g, P0, CardId::GrizzlyBears);
    submit_ok(
        &mut g,
        P0,
        Action::SaddleMount {
            source: mount,
            creatures: vec![helper],
        },
    );
    let unsummon = hand_card(&g, P0, CardId::Unsummon);
    let a = g.legal_actions().into_iter().find(|a| matches!(&a.action, Action::CastSpell { card, targets } if *card==unsummon && targets==&vec![Target::Permanent(mount)])).expect("Unsummon offered").action;
    submit_ok(&mut g, P0, a);
    drain(&mut g);
    assert_eq!(g.state.object(mount).zone, Zone::Hand);
    assert!(
        !g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::Saddled && e.target == mount)
    );
}
