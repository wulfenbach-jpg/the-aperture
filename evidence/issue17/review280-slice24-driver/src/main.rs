//! Evidence-only demonstration driver for slice 24, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. Its only preference is "cast Surrak first, then
//! the biggest creature spell, then play lands", which is what makes the journey
//! exercise the printed cost reduction (a four-drop Bear lord, then six-drop
//! creatures paid with reduced generic mana) and the attack trigger that grants
//! trample and +1/+1 to everything with power 4 or greater.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::layer::ContinuousKind;
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep,
    SessionSave,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice24.txt";
const SEED: u64 = 2323;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice24", &text).unwrap();
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
        let demo_action = if view.pending_kind == Some(PendingKind::DeclareAttackers) {
            // "Attack Pn with all eligible creatures" is the projected legal
            // action list's own entry; the driver never invents attackers.
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
                    view.legal.iter().find(|legal| match &legal.action {
                        Action::CastSpell { card, .. } => view
                            .own_hand
                            .iter()
                            .any(|c| c.id == *card && c.card == CardId::SurrakAndGoreclaw),
                        _ => false,
                    })
                })
                .or_else(|| {
                    view.legal.iter().find(|legal| match &legal.action {
                        Action::CastSpell { card, .. } => view.own_hand.iter().any(|c| {
                            c.id == *card && c.card.def().is_creature() && c.card.def().power >= Some(4)
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
    let out = root.join("evidence/issue17/review280-slice24-out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("demo-save.json"), save_text).expect("write save");

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    assert!(casts.get("Surrak and Goreclaw").copied().unwrap_or(0) > 0, "Surrak must be cast");
    let entry_buffs = events.iter().filter(|e| matches!(e, Event::StateBasedAction { description } if description.contains("gets a +1/+1 counter and gains haste"))).count();
    let haste_grants = events.iter().filter(|e| matches!(e, Event::ContinuousEffectStarted { text, .. } if text.contains("haste"))).count();
    let blocked_damage = events.iter().filter(|e| matches!(e, Event::CombatDamage { attacker_card, blocker: Some(_), .. } if *attacker_card != CardId::SurrakAndGoreclaw)).count();
    assert!(entry_buffs > 0, "the nontoken entry trigger must resolve");
    assert!(haste_grants >= entry_buffs, "each buffed entrant gains haste");
    assert!(blocked_damage > 0, "a blocked creature must deal combat damage with static trample available");
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();
    let report = serde_json::json!({
        "driver": "evidence-only masked cast/attack-preferring demonstrator; NOT product heuristic or strength evidence",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true,
        "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts,
        "entry_buff_events": entry_buffs,
        "haste_grant_events": haste_grants,
        "blocked_trample_damage_events": blocked_damage,
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
