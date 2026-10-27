//! Evidence-only demonstration driver for slice 33, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. It prefers, in this order, playing a land,
//! activating Beorn's Hospitality's animation, casting Beorn's Hospitality,
//! casting Kamahl's Summons (answering its reveal by revealing every creature
//! card in hand) and casting creatures, and it always attacks with everything
//! legal.
//!
//! After *every* accepted action it re-asserts the slice-33 invariant on every
//! animated Beorn's Hospitality on the battlefield: the layer 7b/7c delta equals
//! the number of lands its controller controls, and the effective power equals
//! that number plus any +1/+1 counters. A snapshot at resolution (layer 7b)
//! would fail this as soon as the next land enters. It is a 40-card package, not
//! a starter certificate.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, ContinuousKind, CounterKind, DeckList, Event, Game,
    Replay, ReplayStep, SessionSave, Zone,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice33.txt";
const SEED: u64 = 3333;

fn lands(g: &Game, player: u8) -> i32 {
    g.state
        .objects
        .values()
        .filter(|object| {
            object.zone == Zone::Battlefield
                && object.controller == player
                && object.card.def().is_land()
        })
        .count() as i32
}

/// The characteristic-defining invariant, checked after every action.
fn assert_cda_invariant(g: &Game, animations: &mut usize, sightings: &mut usize) {
    for object in g.state.objects.values() {
        if object.card != CardId::BeornsHospitality || object.zone != Zone::Battlefield {
            continue;
        }
        let animated = g.state.effects.iter().any(|effect| {
            effect.target == object.id
                && effect.kind == ContinuousKind::PowerToughnessEqualToLands
        });
        if !animated {
            continue;
        }
        *animations += 1;
        let lands_now = lands(g, object.controller);
        let counters: i32 = object.counters.get(CounterKind::PlusOnePlusOne) as i32;
        assert!(
            g.is_creature(object),
            "an animated enchantment must be a creature"
        );
        assert_eq!(
            object.pump_power, lands_now,
            "CR 613.4a: the 7a delta must follow the current land count, not a snapshot"
        );
        assert_eq!(
            object.power(),
            lands_now + counters,
            "effective power is the land count plus counters (CR 122.1 after 613.4a)"
        );
        assert_eq!(object.toughness(), lands_now + counters);
        *sightings += 1;
    }
}

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice33", &text).unwrap();
    assert_eq!(deck.size(), 40, "the package is exactly forty cards");
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
    let mut on_turn_sightings = 0usize;
    let mut off_turn_sightings = 0usize;
    while !game.state.is_over() && steps < 20000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);
        let demo_action = match view.pending_kind {
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
                // 1. animate an enchantment that is not animated yet
                let animate = view
                    .legal
                    .iter()
                    .find(|legal| match &legal.action {
                        Action::ActivateAbility { source, .. } => game
                            .state
                            .try_object(*source)
                            .is_some_and(|object| {
                                object.card == CardId::BeornsHospitality
                                    && object.zone == Zone::Battlefield
                                    && !game.is_creature(object)
                            }),
                        _ => false,
                    })
                    .map(|legal| legal.action.clone());
                animate
                    .or_else(|| {
                        view.legal
                            .iter()
                            .find(|legal| matches!(legal.action, Action::PlayLand { .. }))
                            .map(|legal| legal.action.clone())
                    })
                    .or_else(|| {
                        view.legal
                            .iter()
                            .find(|legal| match &legal.action {
                                Action::CastSpell { card, .. } => view
                                    .own_hand
                                    .iter()
                                    .any(|c| c.id == *card && c.card == CardId::BeornsHospitality),
                                _ => false,
                            })
                            .map(|legal| legal.action.clone())
                    })
                    .or_else(|| {
                        view.legal
                            .iter()
                            .find(|legal| match &legal.action {
                                Action::CastSpell { card, .. } => view
                                    .own_hand
                                    .iter()
                                    .any(|c| c.id == *card && c.card == CardId::KamahlsSummons),
                                _ => false,
                            })
                            .map(|legal| legal.action.clone())
                    })
                    .or_else(|| {
                        view.legal
                            .iter()
                            .find(|legal| match &legal.action {
                                Action::CastSpell { card, .. } => view
                                    .own_hand
                                    .iter()
                                    .any(|c| c.id == *card && c.card.def().is_creature()),
                                _ => false,
                            })
                            .map(|legal| legal.action.clone())
                    })
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
        let mut animated_now = 0;
        let mut sightings_now = 0;
        assert_cda_invariant(&game, &mut animated_now, &mut sightings_now);
        if sightings_now > 0 {
            if game.state.turn.active == player {
                on_turn_sightings += sightings_now;
            } else {
                off_turn_sightings += sightings_now;
            }
        }
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
    let out = root.join("evidence/issue17/slice33");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("demo-save.json"), save_text).expect("write save");

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events {
        if let Event::Cast { card, .. } = event {
            *casts.entry(card.name().to_string()).or_default() += 1;
        }
    }
    assert!(
        casts.get("Beorn's Hospitality").copied().unwrap_or(0) > 0,
        "Beorn's Hospitality must be cast"
    );
    let animations = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::AbilityActivated { card: CardId::BeornsHospitality, .. }
            )
        })
        .count();
    assert!(animations > 0, "the animation must be activated");
    let landfall_triggers = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::TriggerStaged { card: CardId::BeornsHospitality, .. }
            )
        })
        .count();
    assert!(landfall_triggers > 0, "landfall must trigger in a real game");
    let continuous = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::ContinuousEffectStarted { card: CardId::BeornsHospitality, .. }
            )
        })
        .count();
    assert!(
        continuous >= 2 * animations,
        "one layer 4 and one layer 7a effect per animation"
    );
    assert!(
        on_turn_sightings + off_turn_sightings > 0,
        "the characteristic-defining invariant must be observed"
    );
    let counters_in_play: u32 = game
        .state
        .objects
        .values()
        .filter(|object| object.zone == Zone::Battlefield)
        .map(|object| object.counters.get(CounterKind::PlusOnePlusOne))
        .sum();
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();
    let report = serde_json::json!({
        "driver": "evidence-only masked animate/landfall/cast/attack-preferring demonstrator; NOT product heuristic or strength evidence",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true,
        "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts,
        "animation_activations": animations,
        "continuous_effects_started": continuous,
        "landfall_triggers_staged": landfall_triggers,
        "cda_invariant_on_turn_sightings": on_turn_sightings,
        "cda_invariant_off_turn_sightings": off_turn_sightings,
        "plus_one_counters_in_play_at_end": counters_in_play,
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
