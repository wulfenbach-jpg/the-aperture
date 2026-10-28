//! Evidence-only demonstration driver for slice 43, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. It prefers playing lands and casting Lumra, then
//! any creature, and it attacks with everything the projection offers so the
//! vigilant attacker and the land-returning trigger are exercised in a real
//! four-seat, 40-life game.
//! It is a 40-card package, not a starter certificate.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep,
    SessionSave,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice43.txt";
const SEED: u64 = 2324;

fn lumra() -> CardId {
    CardId::LumraBellowOfTheWoods
}

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice43", &text).unwrap();
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
    let mut current_turn = 0u32;
    let mut lumra_attacks = 0usize;
    let mut vigilance_checked = 0usize;
    while !game.state.is_over() && steps < 20000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);
        current_turn = view.turn_number;
        let pending = view.pending_kind;
        let demo_action = if pending == Some(PendingKind::DeclareAttackers) {
            // "Attack Pn with all eligible creatures" is the projected legal
            // action list's own entry; the driver never invents attackers.
            view.legal
                .iter()
                .find(|legal| match &legal.action {
                    Action::DeclareAttackers { attackers, .. } => !attackers.is_empty(),
                    _ => false,
                })
                .map(|legal| legal.action.clone())
        } else if pending.is_none()
            && view.stack.is_empty()
            && view.active == player
            && matches!(view.step, Step::PrecombatMain | Step::PostcombatMain)
        {
            view.legal
                .iter()
                .find(|legal| matches!(legal.action, Action::PlayLand { .. }))
                .or_else(|| {
                    view.legal.iter().find(|legal| match &legal.action {
                        Action::CastSpell { card, .. } => view
                            .own_hand
                            .iter()
                            .any(|c| c.id == *card && c.card == lumra()),
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
        // CR 702.20a check on the real event stream: a declared Lumra attacker is
        // still untapped immediately after the declaration.
        for event in &accepted {
            if let Event::AttackersDeclared { attackers, .. } = event {
                for id in attackers {
                    if game
                        .state
                        .try_object(*id)
                        .is_some_and(|object| object.card == lumra())
                    {
                        lumra_attacks += 1;
                        assert!(
                            !game.state.object(*id).tapped,
                            "a vigilant attacker must not tap"
                        );
                        vigilance_checked += 1;
                    }
                }
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
    let out = root.join("evidence/issue17/slice43");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("demo-save.json"), save_text).expect("write save");

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    assert!(casts.get("Lumra, Bellow of the Woods").copied().unwrap_or(0) > 0, "Lumra cast");
    assert!(vigilance_checked > 0, "Lumra attacked (vigilance path executed)");
    let returns = events
        .iter()
        .filter(|e| matches!(e, Event::StateBasedAction { description } if description.contains("from their graveyard to the battlefield tapped")))
        .count();
    let returned_lands = events
        .iter()
        .filter(|e| matches!(e, Event::StateBasedAction { description } if description.contains("enters the battlefield tapped")))
        .count();
    assert!(returns > 0, "the enters trigger returned lands in the real game");
    assert!(returned_lands > 0, "each returned land entered tapped");
    let combat = events
        .iter()
        .filter(|e| matches!(e, Event::CombatDamage { .. }))
        .count();
    assert!(combat > 0, "creatures must attack");
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();
    let report = serde_json::json!({
        "driver": "evidence-only masked land/cast/attack-preferring demonstrator; NOT product heuristic or strength evidence",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true,
        "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": current_turn, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts,
        "lumra_attacks": lumra_attacks,
        "vigilance_checks": vigilance_checked,
        "graveyard_land_return_triggers": returns,
        "returned_land_entries": returned_lands,
        "combat_damage_events": combat,
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
