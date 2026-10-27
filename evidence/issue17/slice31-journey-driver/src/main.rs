//! Evidence-only demonstration driver for slice 31, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. It prefers Beorn the Fierce casts, accepting the
//! printed beginning-of-combat target instead of declining it, and nonempty
//! attacks, so the journey really exercises the trigger, the trample counter, the
//! layer 4 Bear addition and the printed anthem. It is a 40-card package, not a
//! starter certificate.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep,
    SessionSave,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice31.txt";
const SEED: u64 = 3131;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice31", &text).unwrap();
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
    while !game.state.is_over() && steps < 20000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);
        let demo_action = if view.pending_kind == Some(PendingKind::TriggerTarget) {
            // Accept the printed "up to one target creature you control" so the
            // counter and the Bear addition really happen; the decline path is
            // covered by the focused suite, not by this demonstrator.
            view.legal
                .iter()
                .find(|legal| matches!(legal.action, Action::ChooseTriggerTarget { .. }))
                .map(|legal| legal.action.clone())
        } else if view.pending_kind == Some(PendingKind::DeclareAttackers) {
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
                .find(|legal| match &legal.action {
                    Action::PlayLand { .. } => true,
                    _ => false,
                })
                .or_else(|| {
                    view.legal.iter().find(|legal| match &legal.action {
                        Action::CastSpell { card, .. } => view
                            .own_hand
                            .iter()
                            .any(|c| c.id == *card && c.card == CardId::BeornTheFierce),
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
        };
        let action = demo_action.unwrap_or_else(|| baseline.decide(&view).action);
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
    let out = root.join("evidence/issue17/slice31");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("demo-save.json"), save_text).expect("write save");

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    assert!(
        casts.get("Beorn the Fierce").copied().unwrap_or(0) > 0,
        "Beorn must be cast"
    );
    let triggers = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::TriggerStaged { text, .. } if text.starts_with("At the beginning of combat on your turn")
            )
        })
        .count();
    let counters = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::StateBasedAction { description } if description.contains("trample counter(s)")
            )
        })
        .count();
    let bear_additions = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::ContinuousEffectStarted { text, .. } if text.contains("in addition to its other types")
            )
        })
        .count();
    let bear_gates = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::StateBasedAction { description } if description.contains("Bear(s), fewer than")
            )
        })
        .count();
    assert!(triggers > 0, "the printed trigger must fire in the journey");
    assert!(counters > 0, "a trample counter must be put on a creature");
    assert!(bear_additions > 0, "the Bear addition must apply");
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();
    let report = serde_json::json!({
        "driver": "evidence-only masked cast/target/attack-preferring demonstrator; NOT product heuristic or strength evidence",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true,
        "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts,
        "combat_triggers_staged": triggers,
        "trample_counters_placed": counters,
        "bear_additions": bear_additions,
        "bear_count_gates_failed": bear_gates,
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
