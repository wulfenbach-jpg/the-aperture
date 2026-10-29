//! Evidence-only demonstration driver for slice 28, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. Its preference is "cast Circle of the Moon Druid,
//! then play a land, then the biggest creature", which is what makes the
//! four-seat journey exercise the printed Bear Form inside a real game rather
//! than only in a unit test.
//!
//! The driver additionally audits the printed invariant after *every* accepted
//! action in the whole game: a Circle of the Moon Druid on the battlefield is
//! base 4/2 while its controller is the active player and 2/4 otherwise. That
//! is a live assertion over a real replayable game, not a unit-test fixture.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep,
    SessionSave, Target, Zone,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice28.txt";
const SEED: u64 = 2828;

/// The printed Bear Form, checked against the live state.
fn audit_bear_form(game: &Game) -> (usize, usize) {
    let mut on_own_turn = 0;
    let mut off_turn = 0;
    for object in game.state.objects.values() {
        if object.card != CardId::CircleOfTheMoonDruid || object.zone != Zone::Battlefield {
            continue;
        }
        let (power, toughness) = (object.power(), object.toughness());
        if object.controller == game.state.turn.active {
            assert_eq!(
                (power, toughness),
                (4, 2),
                "printed Bear Form must set base 4/2 during its controller's turn ({})",
                object.id
            );
            on_own_turn += 1;
        } else {
            assert_eq!(
                (power, toughness),
                (2, 4),
                "printed Bear Form must not apply during another seat's turn ({})",
                object.id
            );
            off_turn += 1;
        }
    }
    (on_own_turn, off_turn)
}

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice28", &text).unwrap();
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
    // (on-own-turn, off-turn) sightings accumulated over the whole game.
    let mut own_turn_sightings = 0usize;
    let mut off_turn_sightings = 0usize;
    while !game.state.is_over() && steps < 20000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);
        let demo_action = match view.pending_kind {
            Some(PendingKind::TriggerChoice) => view
                .legal
                .iter()
                .find(|a| matches!(a.action, Action::AnswerTriggerChoice { use_ability: true }))
                .map(|a| a.action.clone()),
            Some(PendingKind::TriggerTarget) => view
                .legal
                .iter()
                .find(|a| {
                    matches!(
                        &a.action,
                        Action::ChooseTriggerTarget {
                            target: Target::Permanent(_)
                        }
                    )
                })
                .map(|a| a.action.clone()),
            Some(PendingKind::DeclareAttackers) => view
                .legal
                .iter()
                .find(|legal| match &legal.action {
                    Action::DeclareAttackers { attackers, .. } => !attackers.is_empty(),
                    _ => false,
                })
                .map(|legal| legal.action.clone()),
            None => {
                if view.pending_kind.is_none()
                    && view.stack.is_empty()
                    && view.active == player
                    && matches!(view.step, Step::PrecombatMain | Step::PostcombatMain)
                {
                    view.legal
                        .iter()
                        .find(|legal| match &legal.action {
                            Action::CastSpell { card, .. } => view
                                .own_hand
                                .iter()
                                .any(|c| c.id == *card && c.card == CardId::CircleOfTheMoonDruid),
                            _ => false,
                        })
                        .or_else(|| {
                            view.legal
                                .iter()
                                .find(|legal| matches!(legal.action, Action::PlayLand { .. }))
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
            }
            _ => None,
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
        let (own, off) = audit_bear_form(&game);
        own_turn_sightings += own;
        off_turn_sightings += off;
    }
    assert!(game.state.is_over(), "capped, not terminal");
    assert_eq!(replay.seats, seats, "exact deck snapshots");
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, Event::Truncated { .. })),
        "no rejected actions"
    );
    replay.outcome = game.state.outcome.clone();
    replay.final_digest = game.digest();
    let (replayed, replay_events) = replay.run().unwrap();
    assert_eq!(replayed.digest(), game.digest());
    let save = SessionSave::capture(&game, replay.clone(), Vec::new(), false);
    let save_text = save.to_json();
    let loaded = SessionSave::from_json(&save_text).expect("fresh save reloads");
    assert_eq!(loaded.game, game.state, "fresh save preserves the exact state");
    let out = root.join("evidence/issue17/slice28");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("demo-save.json"), save_text).expect("write save");
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .expect("write replay");

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    assert!(
        casts
            .get("Circle of the Moon Druid")
            .copied()
            .unwrap_or(0)
            > 0,
        "Circle of the Moon Druid must be cast"
    );
    assert!(
        own_turn_sightings > 0,
        "the Bear Form invariant must be observed on its controller's turn"
    );
    assert!(
        off_turn_sightings > 0,
        "and observed as the printed 2/4 on another seat's turn"
    );
    let report = serde_json::json!({
        "driver": "evidence-only masked cast-preferring demonstrator; NOT product heuristic or strength evidence",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true,
        "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts,
        "bear_form_checked_on_own_turn": own_turn_sightings,
        "bear_form_checked_off_turn": off_turn_sightings,
        "bear_form_audit": "printed 4/2 on the controller's turn and 2/4 otherwise, asserted after every accepted action",
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
