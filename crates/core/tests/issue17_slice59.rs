//! Slice 59: Dúnedain Rangers and The Ring.
mod common;
use aperture_core::catalog;
#[test]
fn dunedain_rangers_sourced() {
    let d = catalog::by_name("Dúnedain Rangers")
        .expect("supported")
        .def();
    assert_eq!(d.cost, Some("{3}{G}"));
    assert_eq!(d.type_line, "Creature — Human Ranger");
    assert_eq!(d.power, Some(4));
    assert_eq!(d.toughness, Some(4));
    assert_eq!(
        d.oracle,
        "Landfall — Whenever a land you control enters, if you don't control a Ring-bearer, the Ring tempts you."
    );
}

use aperture_core::*;
use common::*;
fn c59(n: &str) -> CardId {
    catalog::by_name(n).unwrap()
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
fn real_cast_landfall_tempts_and_chooses_ring_bearer() {
    let mut g = fixture(5901)
        .hand(P0, &[c59("Dúnedain Rangers"), CardId::Forest])
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::GrizzlyBears,
            ],
        )
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let spell = hand_card(&g, P0, c59("Dúnedain Rangers"));
    let cast = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action,Action::CastSpell{card,..} if *card==spell))
        .expect("cast offered")
        .action;
    submit_ok(&mut g, P0, cast);
    drain(&mut g);
    let land = hand_card(&g, P0, CardId::Forest);
    let play = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action,Action::PlayLand{card} if *card==land))
        .expect("land offered")
        .action;
    submit_ok(&mut g, P0, play);
    drain(&mut g);
    assert!(
        matches!(g.state.pending, Some(Pending::RingTemptation { .. })),
        "landfall must tempt"
    );
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let choice = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action,Action::ChooseRingBearer{bearer} if bearer==&Some(bear)))
        .expect("bearer offered")
        .action;
    submit_ok(&mut g, P0, choice);
    drain(&mut g);
    assert_eq!(g.state.rings[P0 as usize].temptations, 1);
    assert_eq!(g.state.rings[P0 as usize].bearer, Some(bear));
}

#[test]
fn ring_level_two_draw_and_discard_resumes_without_ending_combat() {
    for level in [2u8] {
        let mut g = fixture(5900 + level as u64)
            .battlefield(P0, &[CardId::GrizzlyBears])
            .library(P0, &[CardId::Forest; 8])
            .library(P1, &[CardId::Forest; 8])
            .main(P0, Step::PrecombatMain)
            .build();
        let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
        g.state.rings = vec![
            aperture_core::state::RingState {
                temptations: level,
                bearer: Some(bear),
                ..Default::default()
            },
            aperture_core::state::RingState::default(),
        ];
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
                attackers: vec![bear],
                defender: Some(P1),
            },
        );
        if level == 2 {
            for _ in 0..8 {
                if g.state.pending.is_some() {
                    break;
                }
                let p = g.decision_owner().unwrap();
                submit_ok(&mut g, p, Action::Pass);
            }
            assert!(
                matches!(g.state.pending, Some(Pending::EffectDiscard { player: P0 })),
                "level 2 draws then discards through its own resumable pending"
            );
            let cards: Vec<_> = g.state.player(P0).hand.iter().copied().take(1).collect();
            submit_ok(&mut g, P0, Action::Discard { cards });
            assert_eq!(
                g.state.turn.step,
                Step::DeclareAttackers,
                "answering the Ring discard must not end combat or the turn"
            );
            assert!(
                g.state.stack.is_empty(),
                "the Ring trigger finished resolving"
            );
            assert_eq!(g.state.turn.number, 1, "still the same turn");
        }
    }
}
