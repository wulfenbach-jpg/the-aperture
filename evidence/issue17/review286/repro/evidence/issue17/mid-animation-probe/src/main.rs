//! Evidence-only demonstration driver for slice 29, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. Its preference is "activate the Stuffed Bear
//! animation, then cast Stuffed Bear, then play a land, then the biggest
//! creature", which is what makes the four-seat journey exercise the printed
//! animation inside a real game rather than only in a unit test.
//!
//! The driver additionally audits the printed invariant after *every* accepted
//! action in the whole game: a Stuffed Bear on the battlefield is a 4/4 green
//! Bear artifact creature exactly while its animation effect applies, and a
//! plain colourless Artifact otherwise. That is a live assertion over a real
//! replayable game, not a unit-test fixture.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, ManaColor, Replay,
    ReplayStep, SessionSave, Target, Zone,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice29.txt";
const SEED: u64 = 2929;

/// The printed animation, checked against the live state.
fn audit_animation(game: &Game) -> (usize, usize) {
    let mut animated = 0;
    let mut plain = 0;
    for object in game.state.objects.values() {
        if object.card != CardId::StuffedBear || object.zone != Zone::Battlefield {
            continue;
        }
        let effect_applies = game.state.effects.iter().any(|effect| {
            effect.target == object.id
                && effect.layer == aperture_core::layer::Layer::Type
                && matches!(
                    effect.kind,
                    aperture_core::layer::ContinuousKind::AddCreatureSubtypes(_)
                )
        });
        let creature = game.is_creature(object);
        let green = game.has_color(object.id, ManaColor::Green);
        if effect_applies {
            assert!(creature, "an animated Stuffed Bear must be a creature ({})", object.id);
            assert!(green, "an animated Stuffed Bear must be green ({})", object.id);
            assert!(
                object.card.def().is_artifact(),
                "the animation keeps the Artifact type"
            );
            assert_eq!(
                (object.power(), object.toughness()),
                (4, 4),
                "an animated Stuffed Bear is base 4/4"
            );
            animated += 1;
        } else {
            assert!(!creature, "an unanimated Stuffed Bear is not a creature ({})", object.id);
            assert!(!green, "an unanimated Stuffed Bear is colourless ({})", object.id);
            plain += 1;
        }
    }
    (animated, plain)
}

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice29", &text).unwrap();
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
    let mut steps = 0; let mut mid_checked = false;
    // (animated, plain) sightings accumulated over the whole game.
    let mut animated_sightings = 0usize;
    let mut plain_sightings = 0usize;
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
                        .find(|legal| {
                            matches!(
                                &legal.action,
                                Action::ActivateAbility { source, .. } if view
                                    .own_battlefield
                                    .iter()
                                    .any(|c| c.id == *source && c.card == CardId::StuffedBear)
                            )
                        })
                        .or_else(|| {
                            view.legal.iter().find(|legal| match &legal.action {
                                Action::CastSpell { card, .. } => view
                                    .own_hand
                                    .iter()
                                    .any(|c| c.id == *card && c.card == CardId::StuffedBear),
                                _ => false,
                            })
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
        if !mid_checked && game.state.effects.iter().any(|e| matches!(e.kind, aperture_core::layer::ContinuousKind::AddCreatureSubtypes(_))) {
            let saved = SessionSave::capture(&game, replay.clone(), Vec::new(), false).to_json();
            let loaded = SessionSave::from_json(&saved).expect("real mid-animation save must validate");
            assert_eq!(loaded.game, game.state);
            assert!(game.state.objects.values().any(|o| o.card == CardId::StuffedBear && game.is_creature(o)));
            mid_checked = true;
            println!("MID_ANIMATION_SESSION_VALIDATION_PASS steps={}", steps + 1);
        }
        steps += 1;
        let (animated, plain) = audit_animation(&game);
        animated_sightings += animated;
        plain_sightings += plain;
    }
    assert!(game.state.is_over(), "capped, not terminal"); assert!(mid_checked);
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
    assert_eq!(
        loaded.game, game.state,
        "fresh save preserves the exact state"
    );
    let out = root.join("evidence/issue17/slice29");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("demo-save.json"), save_text).expect("write save");
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .expect("write replay");

    let mut casts = BTreeMap::<String, usize>::new();
    let mut animates = 0usize;
    for event in &events {
        match event {
            Event::Cast { card, .. } => {
                *casts.entry(card.name().to_string()).or_default() += 1;
            }
            Event::ContinuousEffectStarted { card, layer, .. }
                if *card == CardId::StuffedBear && layer == "4" =>
            {
                animates += 1;
            }
            _ => {}
        }
    }
    assert!(
        casts.get("Stuffed Bear").copied().unwrap_or(0) > 0,
        "Stuffed Bear must be cast"
    );
    assert!(animates > 0, "the printed animation must be activated");
    assert!(
        animated_sightings > 0,
        "the animated invariant must be observed"
    );
    assert!(
        plain_sightings > 0,
        "and the plain Artifact invariant as well"
    );
    let report = serde_json::json!({
        "driver": "evidence-only masked activation/cast-preferring demonstrator; NOT product heuristic or strength evidence",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Jasmine Boreal injected for all four seats by this demonstrator",
        "snapshots_exact": true,
        "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts,
        "animations_activated": animates,
        "animated_sightings": animated_sightings,
        "plain_artifact_sightings": plain_sightings,
        "animation_audit": "animated: creature + green + artifact + 4/4; plain: noncreature + colourless, asserted after every accepted action",
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
