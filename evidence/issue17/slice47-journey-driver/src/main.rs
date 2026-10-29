//! Evidence-only demonstration driver for slice 46, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority plus that seat's own public battlefield: no deck
//! substitution, no state injection, no hidden-information access.
//!
//! Two deliberate choices make the printed trigger reachable in a real game
//! rather than only in a fixture. It attacks with the **weakest** eligible
//! creature it controls that is not the 0/4 Witness, so an attack that is
//! declared alone actually trades and dies, and it cracks Clue tokens for the
//! card. Both are legal rules actions composed from public state; the driver
//! asserts every submission is accepted and never falls back on a rejection.
//!
//! It is a 40-card package, not a starter certificate.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep,
    SessionSave,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice46.txt";
const SEED: u64 = 2323;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice46", &text).unwrap();
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
    let mut alone_attacks = 0usize;
    while !game.state.is_over() && steps < 20000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);
        // Projected, masked actions only; `own_battlefield` is public information
        // about this seat's own permanents.
        let attack_action = if view.pending_kind == Some(PendingKind::DeclareAttackers) {
            view.legal.iter().find_map(|legal| {
                let Action::DeclareAttackers { attackers, defender } = &legal.action else {
                    return None;
                };
                if attackers.is_empty() {
                    return None;
                }
                let power_of = |id: &aperture_core::ObjectId| {
                    view.own_battlefield
                        .iter()
                        .find(|card| card.id == *id)
                        .map(|card| card.power.unwrap_or(i32::MAX))
                        .unwrap_or(i32::MAX)
                };
                let is_witness = |id: &aperture_core::ObjectId| {
                    view.own_battlefield
                        .iter()
                        .any(|card| card.id == *id && card.card == CardId::ThijarianWitness)
                };
                let chosen = attackers
                    .iter()
                    .copied()
                    .filter(|id| !is_witness(id))
                    .min_by_key(power_of)
                    .or_else(|| attackers.first().copied())?;
                Some(Action::DeclareAttackers {
                    attackers: vec![chosen],
                    defender: *defender,
                })
            })
        } else {
            None
        };
        if attack_action.is_some() {
            alone_attacks += 1;
        }
        // One Clue token's printed ability is "{2}, Sacrifice this token: Draw a
        // card"; cracking it is a legal activation offered on the Clue itself.
        let clue_action = if view.pending_kind.is_none() && view.stack.is_empty() {
            view.legal
                .iter()
                .find(|legal| match &legal.action {
                    Action::ActivateAbility { source, .. } => view
                        .own_battlefield
                        .iter()
                        .any(|card| card.id == *source && card.card == CardId::ClueToken),
                    _ => false,
                })
                .map(|legal| legal.action.clone())
        } else {
            None
        };
        let demo_action = attack_action.or(clue_action).or_else(|| {
            if view.pending_kind.is_none()
                && view.stack.is_empty()
                && view.active == player
                && matches!(view.step, Step::PrecombatMain | Step::PostcombatMain)
            {
                view.legal
                    .iter()
                    .find(|legal| matches!(legal.action, Action::PlayLand { .. }))
                    .or_else(|| {
                        // The Witness first, so its trigger is actually on the
                        // battlefield when a combat death happens.
                        view.legal.iter().find(|legal| match &legal.action {
                            Action::CastSpell { card, .. } => view
                                .own_hand
                                .iter()
                                .any(|c| c.id == *card && c.card == CardId::ThijarianWitness),
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
    let out = root.join("evidence/issue17/slice47");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("demo-save.json"), save_text).expect("write save");

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    let witnesses = casts.get("Thijarian Witness").copied().unwrap_or(0);
    assert!(witnesses > 0, "Thijarian Witness must be cast");
    let clues = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::PermanentEntered {
                    card: CardId::ClueToken,
                    ..
                }
            )
        })
        .count();
    assert!(clues > 0, "investigate must create a real Clue token");
    let exiles = events
        .iter()
        .filter(|e| {
            matches!(e, Event::StateBasedAction { description }
                if description.contains("is exiled")
                    && ["Grizzly Bears", "Elite Vanguard", "Centaur Courser", "Thijarian Witness"]
                        .iter()
                        .any(|name| description.contains(name)))
        })
        .count();
    assert!(
        exiles > 0,
        "the printed trigger must exile a creature that died alone in combat"
    );
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();
    let report = serde_json::json!({
        "driver": "evidence-only masked alone-attacking demonstrator; NOT product heuristic or strength evidence",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true,
        "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts,
        "solo_attacks_declared": alone_attacks,
        "clue_tokens_created": clues,
        "exile_events": exiles,
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
