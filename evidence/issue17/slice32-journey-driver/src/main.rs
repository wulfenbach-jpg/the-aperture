//! Evidence-only demonstration driver for slice 32, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. It prefers Kamahl's Summons (so the per-seat
//! "reveal any number of creature cards" choice really resolves at a four-seat
//! table), answers that choice by revealing every creature card in hand, prefers
//! Surrak and Goreclaw (whose printed Trample this slice repaired) and nonempty
//! attacks. It is a 40-card package, not a starter certificate.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep,
    SessionSave,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice32.txt";
const SEED: u64 = 3232;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice32", &text).unwrap();
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
        let demo_action = match view.pending_kind {
            // The reveal is free (the cards stay in hand), so take the largest
            // offered subset and make the token creation really happen.
            Some(PendingKind::ApnapReveal) => view
                .legal
                .iter()
                .filter(|legal| matches!(legal.action, Action::RevealCreatureCards { .. }))
                .next_back()
                .map(|legal| legal.action.clone()),
            Some(PendingKind::DeclareAttackers) => view
                .legal
                .iter()
                .find(|legal| match &legal.action {
                    Action::DeclareAttackers { attackers, .. } => !attackers.is_empty(),
                    _ => false,
                })
                .map(|legal| legal.action.clone()),
            None
                if view.stack.is_empty()
                    && view.active == player
                    && matches!(view.step, Step::PrecombatMain | Step::PostcombatMain) =>
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
                                .any(|c| c.id == *card && c.card == CardId::KamahlsSummons),
                            _ => false,
                        })
                    })
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
                            Action::CastSpell { card, .. } => view
                                .own_hand
                                .iter()
                                .any(|c| c.id == *card && c.card.def().is_creature()),
                            _ => false,
                        })
                    })
                    .map(|legal| legal.action.clone())
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
    let out = root.join("evidence/issue17/slice32");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("demo-save.json"), save_text).expect("write save");

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    assert!(
        casts.get("Kamahl's Summons").copied().unwrap_or(0) > 0,
        "Kamahl's Summons must be cast"
    );
    assert!(
        casts.get("Surrak and Goreclaw").copied().unwrap_or(0) > 0,
        "Surrak and Goreclaw must be cast"
    );
    // Every reveal answer is public: four seats answer once per resolution.
    let reveals: Vec<(u8, u8)> = events
        .iter()
        .filter_map(|e| match e {
            Event::CardsRevealed { player, count, .. } => Some((*player, *count)),
            _ => None,
        })
        .collect();
    let reveal_seats: Vec<u8> = reveals.iter().map(|(seat, _)| *seat).collect();
    assert!(
        !reveals.is_empty(),
        "the per-seat reveal choice must be answered in the journey"
    );
    assert!(
        reveals.iter().map(|(_, count)| u32::from(*count)).sum::<u32>() > 0,
        "at least one seat reveals a creature card"
    );
    let bear_tokens = events
        .iter()
        .filter(|e| matches!(e, Event::PermanentEntered { card, .. } if *card == CardId::BearToken))
        .count();
    assert!(bear_tokens > 0, "the reveal creates Bear tokens");
    let surrak_combat = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::CombatDamage {
                    attacker_card: CardId::SurrakAndGoreclaw,
                    ..
                }
            )
        })
        .count();
    assert!(
        surrak_combat > 0,
        "the repaired card must deal combat damage in a real game"
    );
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();
    let report = serde_json::json!({
        "driver": "evidence-only masked cast/reveal/attack-preferring demonstrator; NOT product heuristic or strength evidence",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true,
        "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts,
        "reveal_answers": reveals.len(),
        "reveal_seats": reveal_seats,
        "revealed_cards_total": reveals.iter().map(|(_, count)| u32::from(*count)).sum::<u32>(),
        "bear_tokens_created": bear_tokens,
        "surrak_combat_damage_events": surrak_combat,
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
