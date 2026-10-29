//! Slice 62 tracer bullet: Goblin Chieftain static and Searslicer Raid.
mod common;
use aperture_core::*;
use common::*;

fn goblin_token_count(g: &Game, p: PlayerId) -> usize {
    g.state
        .battlefield(p)
        .into_iter()
        .filter(|o| o.card == CardId::GoblinToken)
        .count()
}

#[test]
fn goblin_chieftain_anthem_and_haste_are_other_owned_goblins_only() {
    let mut g = fixture(6201)
        .battlefield_sick(
            P0,
            &[
                CardId::GoblinChieftain,
                CardId::GoblinInstigator,
                CardId::GrizzlyBears,
            ],
        )
        .battlefield_sick(P1, &[CardId::GoblinInstigator])
        .main(P0, Step::PrecombatMain)
        .build();
    let chief = battlefield_card(&g, P0, CardId::GoblinChieftain);
    let own_goblin = battlefield_card(&g, P0, CardId::GoblinInstigator);
    let non_goblin = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let enemy_goblin = battlefield_card(&g, P1, CardId::GoblinInstigator);
    submit_ok(&mut g, P0, Action::Pass);
    assert_eq!(
        g.state.object(own_goblin).power(),
        2,
        "other own Goblin gets +1/+1"
    );
    assert_eq!(g.state.object(own_goblin).toughness(), 2);
    assert_eq!(
        g.state.object(chief).power(),
        2,
        "the anthem never pumps itself"
    );
    assert_eq!(g.state.object(chief).toughness(), 2);
    assert_eq!(
        g.state.object(non_goblin).power(),
        2,
        "non-Goblin unchanged"
    );
    assert_eq!(
        g.state.object(enemy_goblin).power(),
        1,
        "opponent Goblin unchanged"
    );
    let attackers = g.legal_attackers(P0);
    assert!(
        attackers.contains(&chief),
        "printed Haste lets the Chieftain attack while sick"
    );
    assert!(attackers.contains(&own_goblin), "other Goblins gain haste");
    assert!(
        !attackers.contains(&non_goblin),
        "non-Goblins do not gain haste"
    );
}

#[test]
fn searslicer_casts_and_raid_end_step_makes_a_real_token_after_attackers_leave() {
    let mut g = fixture(6202)
        .battlefield(
            P0,
            &[
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
                CardId::GrizzlyBears,
            ],
        )
        .hand(P0, &[CardId::SearslicerGoblin])
        .library(P0, &[CardId::Mountain; 8])
        .battlefield(P1, &[CardId::SearslicerGoblin])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let card = g.state.player(P0).hand[0];
    let events = submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            Event::Cast {
                card: CardId::SearslicerGoblin,
                ..
            }
        )),
        "the printed card is cast through the normal stack"
    );
    for _ in 0..8 {
        if g.state
            .battlefield(P0)
            .into_iter()
            .any(|o| o.card == CardId::SearslicerGoblin)
        {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    let attacker = battlefield_card(&g, P0, CardId::GrizzlyBears);
    // Exercise an actual declaration, then remove the attacker: the Raid fact
    // must survive the departure for the whole turn.
    g.state.turn.attackers = vec![attacker];
    g.state.turn.defender = Some(P1);
    g.state.attacked_this_turn = vec![P0];
    if let Some(object) = g.state.objects.get_mut(&attacker) {
        object.zone = Zone::Graveyard;
    }
    g.state.players[P0 as usize].graveyard.push(attacker);
    assert_eq!(goblin_token_count(&g, P0), 0);
    g.state.turn.step = Step::PostcombatMain;
    g.state.turn.priority = Some(P0);
    g.state.turn.passed = 0;
    for _ in 0..12 {
        if g.state
            .stack
            .iter()
            .any(|item| item.card == CardId::SearslicerGoblin)
        {
            break;
        }
        if let Some(pending) = &g.state.pending {
            let p = pending.player();
            let action = auto_answer(&g);
            submit_ok(&mut g, p, action);
        } else if let Some(p) = g.decision_owner() {
            submit_ok(&mut g, p, Action::Pass);
        } else {
            break;
        }
    }
    assert!(
        g.state
            .stack
            .iter()
            .any(|item| item.card == CardId::SearslicerGoblin),
        "the Raid trigger waits on the stack"
    );
    let slicer = battlefield_card(&g, P0, CardId::SearslicerGoblin);
    if let Some(object) = g.state.objects.get_mut(&slicer) {
        object.zone = Zone::Graveyard;
    }
    g.state.players[P0 as usize].graveyard.push(slicer);
    for _ in 0..20 {
        if goblin_token_count(&g, P0) > 0 {
            break;
        }
        if let Some(pending) = &g.state.pending {
            let p = pending.player();
            let action = auto_answer(&g);
            submit_ok(&mut g, p, action);
        } else if let Some(p) = g.decision_owner() {
            submit_ok(&mut g, p, Action::Pass);
        } else {
            break;
        }
    }
    assert_eq!(
        goblin_token_count(&g, P0),
        1,
        "Raid creates one real 1/1 red Goblin token on resolution"
    );
    assert_eq!(
        goblin_token_count(&g, P1),
        0,
        "only the active seat's Searslicer triggers on that seat's end step"
    );
}

#[test]
fn raid_does_not_trigger_without_an_attack_this_turn() {
    let mut g = fixture(6203)
        .battlefield(P0, &[CardId::SearslicerGoblin])
        .library(P0, &[CardId::Mountain; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::EndStep)
        .build();
    let prior = g.state.turn.number;
    for _ in 0..10 {
        if g.state.pending.is_none() && g.state.stack.is_empty() {
            if g.state.turn.step == Step::EndStep {
                let p = g.decision_owner().unwrap();
                submit_ok(&mut g, p, Action::Pass);
            } else {
                break;
            }
        } else if let Some(p) = g.decision_owner() {
            submit_ok(&mut g, p, Action::Pass);
        } else {
            break;
        }
    }
    assert_eq!(goblin_token_count(&g, P0), 0);
    assert!(g.state.turn.number > prior || g.state.turn.step != Step::EndStep);
}

#[test]
fn raid_history_resets_on_turn_transition_and_roundtrips_a_save() {
    let mut g = fixture(6204)
        .battlefield(P0, &[CardId::SearslicerGoblin])
        .library(P0, &[CardId::Mountain; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::EndStep)
        .build();
    g.state.attacked_this_turn = vec![P0];
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        vec![
            DeckList::parse_practice("fixture", "40 Forest\n").unwrap(),
            DeckList::parse_practice("fixture", "40 Forest\n").unwrap(),
        ],
        AiSettings::default(),
    )
    .unwrap();
    let non_sandbox = SessionSave::capture(&g, replay.clone(), Vec::new(), false).to_json();
    assert!(
        SessionSave::from_json(&non_sandbox).is_err(),
        "a fabricated history that the embedded replay did not produce is refused"
    );
    let json = SessionSave::capture(&g, replay, Vec::new(), true).to_json();
    let loaded = SessionSave::from_json(&json).expect("sandbox history save is valid");
    assert_eq!(loaded.game.attacked_this_turn, vec![P0]);
    g.state.turn.step = Step::PostcombatMain;
    g.state.turn.priority = Some(P0);
    g.state.turn.passed = 0;
    // Normal step advancement reaches cleanup and rolls the turn over.
    for _ in 0..24 {
        if g.state.attacked_this_turn.is_empty() {
            break;
        }
        let Some(p) = g.decision_owner() else {
            break;
        };
        let _ = g.submit_action(p, Action::Pass);
    }
    assert!(
        g.state.attacked_this_turn.is_empty(),
        "history resets only at the turn transition"
    );
}

#[test]
fn actual_attack_declaration_records_the_turn_fact_after_the_attacker_leaves() {
    let mut g = fixture(6205)
        .battlefield(P0, &[CardId::GoblinChieftain, CardId::GrizzlyBears])
        .library(P0, &[CardId::Mountain; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::DeclareAttackers)
        .build();
    let chief = battlefield_card(&g, P0, CardId::GoblinChieftain);
    g.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    g.state.turn.priority = None;
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![chief],
            defender: Some(P1),
        },
    );
    assert_eq!(g.state.attacked_this_turn, vec![P0]);
    if let Some(object) = g.state.objects.get_mut(&chief) {
        object.zone = Zone::Graveyard;
    }
    g.state.players[P0 as usize].graveyard.push(chief);
    g.state.turn.attackers.clear();
    assert_eq!(
        g.state.attacked_this_turn,
        vec![P0],
        "Raid reads the declaration fact, not current attackers"
    );
}
