//! Evidence-only demonstration driver, NOT a product AI or a strength claim.
//! Chooses from masked legal actions; no deck substitution/state injection.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep,
};
use std::collections::BTreeMap;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    let text = std::fs::read_to_string(root.join("decks/package-slice14.txt")).unwrap();
    let deck = DeckList::parse_practice("package-slice14", &text).unwrap();
    let seats = vec![deck.clone(); 4];
    let commanders: Vec<_> = (0..4).map(|p| (p, CardId::KrenkoMobBoss)).collect();
    let (mut game, mut events) =
        Game::new_commander_combat_logged(1414, &seats, &commanders).unwrap();
    let setup = commanders
        .iter()
        .map(|&(seat, card)| CommanderSetup { seat, card })
        .collect();
    let mut replay = Replay::new_commander_combat(
        1414,
        game.state.first_player,
        seats.clone(),
        setup,
        AiSettings::default(),
    )
    .unwrap();
    let mut baseline = HeuristicPolicy::new();
    let mut mana_counts = BTreeMap::<String, usize>::new();
    let mut draw_counts = BTreeMap::<String, usize>::new();
    let mut steps = 0;
    while !game.state.is_over() && steps < 12000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);
        // Evidence-only override uses only publicly projected cards/legal offers.
        let demo_action = if view.pending_kind.is_none()
            && view.stack.is_empty()
            && view.active == player
            && matches!(view.step, Step::PrecombatMain | Step::PostcombatMain)
        {
            view.legal
                .iter()
                .find(|a| matches!(a.action, Action::PlayLand { .. }))
                .or_else(|| view.legal.iter().find(|a| match &a.action {
                    Action::CastSpell { card, .. } => view
                        .own_hand
                        .iter()
                        .any(|c| c.id == *card && c.card.def().is_creature()),
                    _ => false,
                }))
                .or_else(|| {
                    view.legal.iter().find(|a| match &a.action {
                        Action::CastSpell { card, .. } => view
                            .own_hand
                            .iter()
                            .any(|c| c.id == *card && c.card.def().is_artifact()),
                        _ => false,
                    })
                })
                .or_else(|| {
                    view.legal.iter().find(|a| match &a.action {
                        Action::ActivateManaAbility { land } => view
                            .own_battlefield
                            .iter()
                            .any(|c| c.id == *land && c.name == "Werebear" && !c.tapped),
                        _ => false,
                    })
                })
                .or_else(|| {
                    view.legal.iter().find(|a| match &a.action {
                        Action::ActivateColorManaAbility { .. } => true,
                        _ => false,
                    })
                })
                .or_else(|| {
                    view.legal.iter().find(|a| match &a.action {
                        Action::ActivateAbility { source, .. } => {
                            view.turn_number >= 10
                                && view.own_battlefield.iter().any(|c| c.id == *source)
                                && view.own_library_count >= 5
                                && view
                                    .own_battlefield
                                    .iter()
                                    .any(|c| c.id == *source && c.card.def().is_artifact())
                        }
                        _ => false,
                    })
                })
                .or_else(|| {
                    view.legal.iter().find(|a| match &a.action {
                        Action::ActivateManaAbility { land } => view
                            .own_battlefield
                            .iter()
                            .any(|c| c.id == *land),
                        _ => false,
                    })
                })
                .map(|a| a.action.clone())
        } else {
            None
        };
        let action = demo_action.unwrap_or_else(|| baseline.decide(&view).action);
        let mana_source = match action {
            Action::ActivateColorManaAbility { source, .. } => view
                .own_battlefield
                .iter()
                .find(|c| c.id == source)
                .map(|c| c.name.to_string()),
            Action::ActivateManaAbility { land } => view
                .own_battlefield
                .iter()
                .find(|c| c.id == land)
                .map(|c| c.name.to_string()),
            _ => None,
        };
        let draw_source = match action {
            Action::ActivateAbility { source, .. } => view
                .own_battlefield
                .iter()
                .find(|c| c.id == source && c.card.def().is_artifact())
                .map(|c| c.name.to_string()),
            _ => None,
        };
        let revision = game.revision();
        let decision = game.decision();
        let accepted = game
            .submit_action(player, action.clone())
            .expect("all actions must be accepted; no rejection fallback");
        if let Some(name) = mana_source {
            *mana_counts.entry(name).or_default() += 1;
        }
        if let Some(name) = draw_source {
            *draw_counts.entry(name).or_default() += 1;
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
    replay.outcome = game.state.outcome.clone();
    replay.final_digest = game.digest();
    let (replayed, replay_events) = replay.run().unwrap();
    assert_eq!(replayed.digest(), game.digest());
    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    for name in ["Arcane Signet", "Fellwar Stone", "Commander's Sphere", "Werebear"] {
        assert!(
            casts.get(name).copied().unwrap_or(0) > 0,
            "{name} must actually be cast"
        );
        assert!(
            mana_counts.get(name).copied().unwrap_or(0) > 0,
            "{name} must actually produce mana"
        );
    }
    assert!(
        draw_counts.get("Commander's Sphere").copied().unwrap_or(0) > 0,
        "Commander's Sphere sacrifice draw must resolve"
    );
    let out = root.join("evidence/issue17/slice14");
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();
    let report = serde_json::json!({
        "driver": "evidence-only masked artifact-preferring demonstrator; NOT product heuristic/strength evidence",
        "seed": 1414, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": "decks/package-slice14.txt", "kind": "40-card package, not a desired starter",
        "snapshots_exact": true, "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "artifact_casts": casts, "artifact_mana_activations": mana_counts, "artifact_draw_activations": draw_counts,
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
