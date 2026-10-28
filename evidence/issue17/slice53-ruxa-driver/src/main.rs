//! Evidence-only demonstration driver for slice 53, NOT a product AI and NOT a
//! strength claim. Every action is composed from the masked legal actions the
//! engine projects for the seat that has priority plus that seat's own public
//! battlefield; a rejected submission is a hard failure, never a fallback.
//!
//! It deliberately drives the three printed clauses of Ruxa, Patient Professor in
//! one real four-seat 40-life Commander game:
//!
//! * it casts Ruxa as early as possible, so the anthem and the permission are on
//!   the battlefield in live combats;
//! * it attacks with **every** eligible creature, so combats are multi-attacker
//!   and the defending seats block;
//! * it answers the "assign as though it weren't blocked" choice **both ways**
//!   (alternating), so the evidence covers the accepting and the declining branch
//!   rather than one of them;
//! * creatures therefore die, which puts vanilla creature cards into graveyards,
//!   which is what the enters/attacks trigger returns.
//!
//! It is a 40-card package, not a starter certificate.

use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep, SessionSave,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice53.txt";
const SEED: u64 = 5353;
const COMMANDER: CardId = CardId::JasmineBoreal;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).expect("package deck");
    let deck = DeckList::parse_practice("package-slice53", &text).expect("package parses");
    let seats = vec![deck.clone(); 4];
    let commanders: Vec<_> = (0..4).map(|p| (p, COMMANDER)).collect();
    let (mut game, mut events) =
        Game::new_commander_combat_logged(SEED, &seats, &commanders).expect("commander setup");
    let setup: Vec<CommanderSetup> = commanders
        .iter()
        .map(|&(seat, card)| CommanderSetup { seat, card })
        .collect();
    let mut replay = Replay::new_commander_combat(
        SEED,
        game.state.first_player,
        seats.clone(),
        setup,
        AiSettings::default(),
    )
    .expect("replay");

    let mut baseline = HeuristicPolicy::new();
    let mut steps = 0usize;
    let mut assignments = 0usize;
    let mut assignments_accepted = 0usize;
    let mut assignments_declined = 0usize;
    let mut ruxa_casts = 0usize;
    let mut returned_to_hand = 0usize;
    let mut unblocked_damage_to_player = 0u32;
    let mut blocker_damage = 0u32;

    while !game.state.is_over() && steps < 20000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);

        // (1) Attack with every eligible attacker: the engine offers one declar-
        // ation per defending seat carrying its full legal set, so the largest
        // offer is "all of them".
        let attack = if view.pending_kind == Some(PendingKind::DeclareAttackers) {
            view.legal
                .iter()
                .filter(|legal| matches!(legal.action, Action::DeclareAttackers { .. }))
                .max_by_key(|legal| match &legal.action {
                    Action::DeclareAttackers { attackers, .. } => attackers.len(),
                    _ => 0,
                })
                .map(|legal| legal.action.clone())
        } else {
            None
        };

        // (2) Answer the damage-assignment permission both ways.
        let assignment = if matches!(
            view.pending_kind,
            Some(PendingKind::CombatDamageAssignment { .. })
        ) {
            let accept = assignments % 2 == 0;
            assignments += 1;
            if accept {
                assignments_accepted += 1;
            } else {
                assignments_declined += 1;
            }
            view.legal
                .iter()
                .find_map(|legal| match legal.action {
                    Action::AnswerCombatDamageAssignment { as_though_unblocked }
                        if as_though_unblocked == accept =>
                    {
                        Some(legal.action.clone())
                    }
                    _ => None,
                })
        } else {
            None
        };

        // (3) Cast Ruxa whenever it is castable, so the static abilities are live.
        let cast_ruxa = if view.pending_kind.is_none() && view.stack.is_empty() {
            view.legal
                .iter()
                .find(|legal| match &legal.action {
                    Action::CastSpell { card, .. } => view
                        .own_hand
                        .iter()
                        .any(|c| c.id == *card && c.card == CardId::RuxaPatientProfessor),
                    _ => false,
                })
                .map(|legal| legal.action.clone())
        } else {
            None
        };

        let action = attack
            .or(assignment)
            .or(cast_ruxa)
            .unwrap_or_else(|| baseline.decide(&view).action);

        let revision = game.revision();
        let decision = game.decision();
        let accepted = game
            .submit_action(player, action.clone())
            .expect("every submitted action must be accepted; no rejection fallback");
        for event in &accepted {
            match event {
                Event::Cast {
                    card: CardId::RuxaPatientProfessor,
                    ..
                } => ruxa_casts += 1,
                Event::StateBasedAction { description }
                    if description.contains("is returned to its owner's hand") =>
                {
                    returned_to_hand += 1;
                }
                Event::CombatDamage {
                    blocker: None,
                    amount,
                    ..
                } => unblocked_damage_to_player += u32::from(*amount),
                Event::CombatDamage {
                    blocker: Some(_),
                    amount,
                    ..
                } => blocker_damage += u32::from(*amount),
                _ => {}
            }
        }
        events.extend(accepted);
        replay.record(ReplayStep {
            player,
            revision,
            decision,
            action,
        });
        steps += 1;
    }

    assert!(game.state.is_over(), "capped, not terminal");
    assert_eq!(replay.seats, seats, "exact deck snapshots");
    assert!(
        !events.iter().any(|e| matches!(e, Event::Truncated { .. })),
        "no rejected actions"
    );

    // Every printed clause must actually have happened in this one game.
    assert!(ruxa_casts > 0, "Ruxa, Patient Professor must be cast");
    assert!(
        assignments_accepted > 0,
        "the accepting branch of the printed permission must be exercised"
    );
    assert!(
        assignments_declined > 0,
        "the declining branch of the printed permission must be exercised"
    );
    assert!(
        returned_to_hand > 0,
        "the enters/attacks trigger must return a card from a graveyard"
    );
    assert!(
        unblocked_damage_to_player > 0,
        "combat damage must reach a defending player"
    );

    replay.outcome = game.state.outcome.clone();
    replay.final_digest = game.digest();
    let (replayed, replay_events) = replay.run().expect("replay runs");
    assert_eq!(
        replay_events.len(),
        events.len(),
        "replay event count matches the live game"
    );
    assert_eq!(replayed.digest(), game.digest(), "replay digest matches");

    let save = SessionSave::capture(&game, replay.clone(), Vec::new(), false);
    let save_text = save.to_json();
    let loaded = SessionSave::from_json(&save_text).expect("fresh save reloads");
    assert_eq!(
        loaded.game, game.state,
        "fresh save preserves the exact state"
    );

    let out = root.join("evidence/issue17/slice53/ruxa");
    std::fs::create_dir_all(&out).expect("out dir");
    std::fs::write(out.join("demo-save.json"), &save_text).expect("write save");
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).expect("replay json"),
    )
    .expect("write replay");

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    let report = serde_json::json!({
        "driver": "evidence-only masked Ruxa demonstrator; NOT product heuristic or strength evidence",
        "seed": SEED,
        "seats": 4,
        "starting_life": 40,
        "profile": format!("{:?}", game.state.profile),
        "status": "terminal",
        "deck": PACKAGE,
        "kind": "40-card package, not a desired starter",
        "commander": COMMANDER.name(),
        "outcome": format!("{:?}", game.state.outcome),
        "steps": steps,
        "turns": game.state.turn.number,
        "events": events.len(),
        "digest": game.digest(),
        "replay_digest_matches": true,
        "ruxa_casts": ruxa_casts,
        "ruxa_assignments_offered": assignments,
        "ruxa_assignments_accepted": assignments_accepted,
        "ruxa_assignments_declined": assignments_declined,
        "cards_returned_from_graveyard_to_hand": returned_to_hand,
        "combat_damage_to_players": unblocked_damage_to_player,
        "combat_damage_to_creatures": blocker_damage,
        "casts": casts,
        "snapshots_exact": replay.seats == seats,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).expect("journey json"),
    )
    .expect("write journey");
    println!("{}", serde_json::to_string_pretty(&report).expect("json"));
}
