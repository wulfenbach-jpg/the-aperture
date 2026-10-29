//! Evidence-only demonstration driver for slice 57, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. It prefers an announced-X cast of Rampaging Yao
//! Guai, then commits the largest legal aggregate target set it is offered, so a
//! real four-seat game actually exercises the new X counters and the
//! total-mana-value destruction. It is a 40-card package, not a starter
//! certificate.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep,
    SessionSave,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice57.txt";
const SEED: u64 = 5757;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice57", &text).unwrap();
    let seats = vec![deck.clone(); 4];
    let commanders: Vec<_> = (0..4).map(|p| (p, CardId::JasmineBoreal)).collect();
    let (mut game, mut events) =
        Game::new_commander_combat_logged(SEED, &seats, &commanders).unwrap();
    let setup = commanders
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
    .unwrap();
    let mut baseline = HeuristicPolicy::new();
    let mut steps = 0;
    let mut announced_x: Vec<u8> = Vec::new();
    let mut set_sizes: Vec<usize> = Vec::new();
    let mut sets_committed = 0usize;
    let mut adds_this_set: usize = 0;

    // Public-state instrumentation only; never fed to the projected policy.
    while !game.state.is_over() && steps < 20000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);

        // 1. The aggregate target set: add every legal target until only the
        //    commit is left, then commit. The driver never invents a target or a
        //    budget; both come from the projected legal list.
        let aggregate_action = if view.pending_kind == Some(PendingKind::TriggerTarget) {
            let adds: Vec<&Action> = view
                .legal
                .iter()
                .filter(|legal| matches!(legal.action, Action::ChooseTriggerTarget { .. }))
                .map(|legal| &legal.action)
                .collect();
            if !adds.is_empty() {
                Some((*adds[adds.len() - 1]).clone())
            } else {
                view.legal
                    .iter()
                    .find(|legal| matches!(legal.action, Action::FinishTriggerTargets))
                    .map(|legal| legal.action.clone())
            }
        } else {
            None
        };

        let demo_action = aggregate_action.or_else(|| {
            if view.pending_kind == Some(PendingKind::DeclareAttackers) {
                view.legal
                    .iter()
                    .find(|legal| match &legal.action {
                        Action::DeclareAttackers { attackers, .. } => !attackers.is_empty(),
                        _ => false,
                    })
                    .map(|legal| legal.action.clone())
            } else if view.pending_kind.is_none()
                && view.stack.is_empty()
                && view.active == player
                && matches!(view.step, Step::PrecombatMain | Step::PostcombatMain)
            {
                view.legal
                    .iter()
                    .find(|legal| matches!(legal.action, Action::PlayLand { .. }))
                    .or_else(|| {
                        // Largest affordable announced X, chosen only from the
                        // projected legal actions.
                        view.legal
                            .iter()
                            .filter_map(|legal| match &legal.action {
                                Action::CastSpellWithX { card, x, .. }
                                    if view.own_hand.iter().any(|c| {
                                        c.id == *card && c.card == CardId::RampagingYaoGuai
                                    }) =>
                                {
                                    Some((*x, legal))
                                }
                                _ => None,
                            })
                            .max_by_key(|(x, _)| *x)
                            .map(|(_, legal)| legal)
                    })
                    .or_else(|| {
                        view.legal.iter().find(|legal| match &legal.action {
                            Action::CastSpell { card, .. } => view.own_hand.iter().any(|c| {
                                c.id == *card
                                    && matches!(
                                        c.card,
                                        CardId::SolRing
                                            | CardId::ArcaneSignet
                                            | CardId::CommandersSphere
                                    )
                            }),
                            _ => false,
                        })
                    })
                    .or_else(|| {
                        view.legal.iter().find(|legal| match &legal.action {
                            Action::CastSpell { card, .. } => view
                                .own_hand
                                .iter()
                                .any(|c| c.id == *card && c.card.def().is_creature()),
                            _ => false,
                        })
                    })
                    .map(|legal| legal.action.clone())
            } else {
                None
            }
        });
        let action = demo_action.unwrap_or_else(|| baseline.decide(&view).action);
        // Public instrumentation of the announced X and the committed sets.
        match &action {
            Action::CastSpellWithX { x, .. } => announced_x.push(*x),
            Action::FinishTriggerTargets => {
                sets_committed += 1;
                set_sizes.push(adds_this_set);
                adds_this_set = 0;
            }
            Action::ChooseTriggerTarget { .. } => adds_this_set += 1,
            _ => {}
        }
        let revision = game.revision();
        let decision = game.decision();
        let accepted = game
            .submit_action(player, action.clone())
            .expect("all actions must be accepted; no rejection fallback");
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
    let save = SessionSave::capture(&game, replay.clone(), Vec::new(), false);
    let save_text = save.to_json();
    let loaded = SessionSave::from_json(&save_text).expect("fresh save reloads");
    assert_eq!(
        loaded.game, game.state,
        "fresh save preserves the exact state"
    );
    let out = root.join("evidence/issue17/slice57/native");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("demo-save.json"), save_text).expect("write save");

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    assert!(
        casts.get("Rampaging Yao Guai").copied().unwrap_or(0) > 0,
        "the card must actually be cast"
    );
    let destroy_lines = events
        .iter()
        .filter(|event| {
            matches!(event, Event::StateBasedAction { description } if description.contains("were destroyed (CR 701.7a)"))
        })
        .count();
    let destroyed_objects: usize = events
        .iter()
        .filter_map(|event| match event {
            Event::StateBasedAction { description }
                if description.contains("were destroyed (CR 701.7a)") =>
            {
                description
                    .split(' ')
                    .next()
                    .and_then(|n| n.parse::<usize>().ok())
            }
            _ => None,
        })
        .sum();
    let max_counters = game
        .state
        .objects
        .values()
        .filter(|object| {
            object.zone == aperture_core::Zone::Battlefield
                && object.card == CardId::RampagingYaoGuai
        })
        .map(|object| {
            object
                .counters
                .get(aperture_core::counters::CounterKind::PlusOnePlusOne)
        })
        .max()
        .unwrap_or(0);
    assert!(!announced_x.is_empty(), "an X was announced");
    assert!(sets_committed > 0, "an aggregate target set was committed");
    assert!(destroy_lines > 0, "the entry trigger destroyed permanents");
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();
    let report = serde_json::json!({
        "driver": "evidence-only masked X/aggregate-target demonstrator; NOT product heuristic or strength evidence",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true,
        "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts,
        "announced_x": announced_x,
        "aggregate_sets_committed": sets_committed,
        "aggregate_set_sizes": set_sizes,
        "destroy_instructions": destroy_lines,
        "destroyed_permanents": destroyed_objects,
        "max_plus_one_counters_on_a_yao_guai": max_counters,
        "static_measurement": "public battlefield at the final state; not policy input",
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
