//! Evidence-only demonstration driver for slice 15, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. Its only preference is "prefer the new cards and
//! their printed abilities", which is what makes the journey exercise them.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice15.txt";
const SEED: u64 = 1515;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice15", &text).unwrap();
    let seats = vec![deck.clone(); 4];
    // A green-white commander keeps the package's colour identity legal; the
    // demonstration never relies on the commander's own abilities.
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
    let mut activations = BTreeMap::<String, usize>::new();
    let mut steps = 0;
    while !game.state.is_over() && steps < 20000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);
        let demo_action = if view.pending_kind.is_none()
            && view.stack.is_empty()
            && view.active == player
            && matches!(view.step, Step::PrecombatMain | Step::PostcombatMain)
        {
            view.legal
                .iter()
                .find(|a| matches!(a.action, Action::PlayLand { .. }))
                .or_else(|| {
                    // Cast the package's own cards first (enchantments and the
                    // graveyard-costed bear), then ordinary creatures.
                    view.legal.iter().find(|a| match &a.action {
                        Action::CastSpell { card, .. } => view.own_hand.iter().any(|c| {
                            c.id == *card
                                && (c.card.def().is_enchantment() || c.card == CardId::MotherBear)
                        }),
                        _ => false,
                    })
                })
                .or_else(|| {
                    // The abilities this slice adds. Any offered costed
                    // activation is legal by construction; the driver only
                    // prefers it over passing.
                    view.legal
                        .iter()
                        .find(|a| matches!(a.action, Action::ActivateCostedAbility { .. }))
                })
                .or_else(|| {
                    // Mother Bear's graveyard activation: its source is not a
                    // battlefield permanent of this seat.
                    view.legal.iter().find(|a| match &a.action {
                        Action::ActivateAbility { source, .. } => {
                            !view.own_battlefield.iter().any(|c| c.id == *source)
                        }
                        _ => false,
                    })
                })
                .or_else(|| {
                    view.legal.iter().find(|a| match &a.action {
                        Action::CastSpell { card, .. } => view
                            .own_hand
                            .iter()
                            .any(|c| c.id == *card && c.card.def().is_creature()),
                        _ => false,
                    })
                })
                .map(|a| a.action.clone())
        } else {
            None
        };
        let action = demo_action.unwrap_or_else(|| baseline.decide(&view).action);
        let activation_source = match &action {
            Action::ActivateCostedAbility { source, .. } => Some(*source),
            Action::ActivateAbility { source, .. } => Some(*source),
            _ => None,
        };
        let activation_name = activation_source.and_then(|source| {
            view.own_battlefield
                .iter()
                .find(|c| c.id == source)
                .map(|c| c.name.to_string())
                .or_else(|| {
                    view.own_graveyard
                        .iter()
                        .find(|c| c.id == source)
                        .map(|c| c.name.to_string())
                })
        });
        let revision = game.revision();
        let decision = game.decision();
        let accepted = game
            .submit_action(player, action.clone())
            .expect("all actions must be accepted; no rejection fallback");
        if let Some(name) = activation_name {
            *activations.entry(name).or_default() += 1;
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
    let mut bears = 0usize;
    for event in &events {
        match event {
            Event::Cast { card, .. } => *casts.entry(card.name().to_string()).or_default() += 1,
            Event::PermanentEntered { card, .. } if *card == CardId::BearToken => bears += 1,
            _ => {}
        }
    }
    for name in ["Ayula's Influence", "Bearscape", "Mother Bear"] {
        assert!(
            casts.get(name).copied().unwrap_or(0) > 0,
            "{name} must actually be cast"
        );
    }
    let costed = activations
        .get("Ayula's Influence")
        .copied()
        .unwrap_or(0)
        + activations.get("Bearscape").copied().unwrap_or(0);
    assert!(
        costed > 0,
        "at least one card-costed ability must be activated"
    );
    assert!(bears >= 2, "Bear tokens must actually be created: {bears}");

    let out = root.join("evidence/issue17/slice15");
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();
    let report = serde_json::json!({
        "driver": "evidence-only masked card-preferring demonstrator; NOT product heuristic/strength evidence",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true, "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts, "activations": activations, "bear_tokens_created": bears,
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
