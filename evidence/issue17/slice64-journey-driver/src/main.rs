//! Evidence-only demonstration driver for slice 64, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. It prefers the four slice-64 cards, the offered
//! library-search pick, and offered nonempty attacks, so the journey exercises
//! what this slice enabled. It is a 40-card package, not a starter certificate.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep,
    SessionSave,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice64.txt";
const SEED: u64 = 2424;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice64", &text).unwrap();
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
    let mut search_picks = 0usize;
    let mut ability_activations = 0usize;

    // Public-state instrumentation only; never fed to the projected policy.
    while !game.state.is_over() && steps < 20000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);

        let search_action = if view.pending_kind.is_some() {
            view.legal
                .iter()
                .find(|a| matches!(a.action, Action::SearchLibraryPick { .. }))
                .or_else(|| {
                    view.legal
                        .iter()
                        .find(|a| matches!(a.action, Action::SearchLibraryDone))
                })
                .map(|a| a.action.clone())
        } else {
            None
        };
        let demo_action = search_action.or_else(|| {
            if view.pending_kind == Some(PendingKind::DeclareBlockers) {
                // Take an offered declaration (empty blocks included); the driver
                // never constructs a pair the projection did not offer.
                view.legal
                    .iter()
                    .find(|legal| matches!(legal.action, Action::DeclareBlockers { .. }))
                    .map(|legal| legal.action.clone())
            } else if view.pending_kind == Some(PendingKind::DeclareAttackers) {
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
                    .map(|legal| legal.action.clone())
                    .or_else(|| {
                        // Prefer the slice-64 cards, in a fixed order, then any
                        // offered creature cast.
                        let wanted = [
                            CardId::GoblinMatron,
                            CardId::KrenkoTinStreetKingpin,
                            CardId::HobgoblinBanditLord,
                            CardId::GoblinSharpshooter,
                            CardId::GoblinInstigator,
                        ];
                        view.legal
                            .iter()
                            .find(|legal| match &legal.action {
                                Action::CastSpell { card, .. } => view
                                    .own_hand
                                    .iter()
                                    .any(|c| c.id == *card && wanted.contains(&c.card)),
                                _ => false,
                            })
                            .map(|legal| legal.action.clone())
                            .or_else(|| {
                                view.legal
                                    .iter()
                                    .find(|legal| match &legal.action {
                                        Action::CastSpell { card, .. } => view.own_hand.iter().any(
                                            |c| c.id == *card && c.card.def().is_creature(),
                                        ),
                                        _ => false,
                                    })
                                    .map(|legal| legal.action.clone())
                            })
                    })
            } else {
                None
            }
        });
        let action = demo_action.unwrap_or_else(|| baseline.decide(&view).action);
        if matches!(action, Action::SearchLibraryPick { .. }) {
            search_picks += 1;
        }
        if matches!(action, Action::ActivateAbility { .. }) {
            ability_activations += 1;
        }
        let revision = game.revision();
        let decision = game.decision();
        let accepted = match game.submit_action(player, action.clone()) {
            Ok(events) => events,
            Err(error) => {
                panic!("all actions must be accepted; no rejection fallback: {error:?}");
            }
        };
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
    assert!(
        !save.sandbox_modified,
        "journey save is not sandbox-modified"
    );
    let save_text = save.to_json();
    let loaded_save = SessionSave::from_json(&save_text).expect("journey save loads");
    let loaded_game = Game::from_state(loaded_save.game).expect("journey state restores");
    assert_eq!(loaded_game.digest(), game.digest(), "save roundtrip digest");
    let loaded = SessionSave::from_json(&save_text).expect("fresh save reloads");
    assert_eq!(
        loaded.game, game.state,
        "fresh save preserves the exact state"
    );
    let out = root.join("evidence/issue17/slice64");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("demo-save.json"), save_text).expect("write save");

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    for card in [
        "Goblin Matron",
        "Krenko, Tin Street Kingpin",
        "Hobgoblin Bandit Lord",
        "Goblin Sharpshooter",
    ] {
        assert!(
            casts.get(card).copied().unwrap_or(0) > 0,
            "{card} was cast in the journey"
        );
    }
    let goblin_tokens = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::PermanentEntered {
                    card: CardId::GoblinToken,
                    ..
                }
            )
        })
        .count();
    let searched = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::StateBasedAction { description }
                    if description.contains("puts it into their hand (CR 701.23)")
            )
        })
        .count();
    let attacks = events
        .iter()
        .filter(|event| matches!(event, Event::AttackersDeclared { .. }))
        .count();
    assert!(attacks > 0, "the driver declared actual attacks");
    let combat = events
        .iter()
        .filter(|e| matches!(e, Event::CombatDamage { .. }))
        .count();
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();
    let report = serde_json::json!({
        "driver": "projected legal-offer evidence driver; explicitly not product AI",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true,
        "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts,
        "goblin_tokens_entered": goblin_tokens,
        "goblin_search_reveals": searched,
        "search_picks": search_picks,
        "ability_activations": ability_activations,
        "attack_declarations": attacks,
        "combat_damage_events": combat,
        "prefers_slice64_cards": ["Goblin Matron", "Krenko, Tin Street Kingpin",
                                  "Hobgoblin Bandit Lord", "Goblin Sharpshooter"],
        "static_measurement": "public battlefield after each accepted action; not policy input; repeated observations, not unique events",
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
