//! Evidence-only demonstration driver for slice 45, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. It prefers offered search picks, nonempty attacks,
//! lands, Titania's Command casts and other creatures, so selected exile/search,
//! Bear and counter modes plus normal combat are exercised.
//! It is a 40-card package, not a starter certificate.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep,
    SessionSave, Target, P1,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice45.txt";
const SEED: u64 = 2323;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice45", &text).unwrap();
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
    let mut modal_round = 0usize;
    // Public-state instrumentation only; never fed to the projected policy.
    let mut spirit_static_samples = 0usize;
    let mut boosted_spirit_samples = 0usize;
    while !game.state.is_over() && steps < 20000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);
        let desired_modes = if modal_round % 2 == 0 { [0u8, 1] } else { [2u8, 3] };
        let modal_action = if view.pending_kind.is_none() && view.stack.is_empty() && view.active == player {
            view.legal.iter().find(|a| match &a.action {
                Action::CastSpellWithModes { modes, targets, .. } => {
                    *modes == desired_modes
                        && (!modes.contains(&0) || targets == &vec![Target::Player(P1)])
                }
                _ => false,
            }).map(|a| a.action.clone())
        } else { None };
        if modal_action.is_some() { modal_round += 1; }
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
        let demo_action = modal_action.or(search_action).or_else(|| {
            if view.pending_kind == Some(PendingKind::DeclareAttackers) {
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
                                .any(|c| c.id == *card && c.card == CardId::TitaniasCommand),
                            _ => false,
                        })
                    })
                    .or_else(|| {
                        view.legal.iter().find(|legal| match &legal.action {
                            Action::CastSpell { card, .. } => view
                                .own_hand
                                .iter()
                                .any(|c| c.id == *card && c.card == CardId::EliteVanguard),
                            _ => false,
                        })
                    })
                    .or_else(|| {
                        view.legal.iter().find(|legal| match &legal.action {
                            Action::CastSpell { card, .. } => view.own_hand.iter().any(|c| {
                                c.id == *card
                                    && c.card.def().is_creature()
                                    && c.card.def().power >= Some(4)
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
            }
        });
        let action = demo_action.unwrap_or_else(|| baseline.decide(&view).action);
        let revision = game.revision();
        let decision = game.decision();
        let accepted = game
            .submit_action(player, action.clone())
            .expect("all actions must be accepted; no rejection fallback");
        events.extend(accepted);
        for spirit in game.state.objects.values().filter(|o| {
            o.zone == aperture_core::Zone::Battlefield && o.card == CardId::TitaniasCommand
        }) {
            let other_snow = game
                .state
                .objects
                .values()
                .filter(|o| {
                    o.zone == aperture_core::Zone::Battlefield
                        && o.controller == spirit.controller
                        && o.id != spirit.id
                        && o.card.def().is_snow()
                })
                .count() as i32;
            // This exact package has no other P/T-changing effects.
            assert_eq!(spirit.power(), other_snow);
            assert_eq!(spirit.toughness(), 4);
            spirit_static_samples += 1;
            if other_snow > 0 {
                boosted_spirit_samples += 1;
            }
        }
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
    let out = root.join("evidence/issue17/slice45");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("demo-save.json"), save_text).expect("write save");

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events { if let Event::Cast { card, .. } = event { *casts.entry(card.name().to_string()).or_default() += 1; } }
    assert!(casts.get("Titania's Command").copied().unwrap_or(0) > 0, "Titania's Command must be cast");
    let exiles = events.iter().filter(|e| matches!(e, Event::StateBasedAction { description } if description.contains("exiles P1's graveyard"))).count();
    let searches = events.iter().filter(|e| matches!(e, Event::StateBasedAction { description } if description.contains("onto the battlefield tapped"))).count();
    let bears = events.iter().filter(|e| matches!(e, Event::PermanentEntered { card: CardId::BearToken, .. })).count();
    let counters = events.iter().filter(|e| matches!(e, Event::StateBasedAction { description } if description.contains("gets two +1/+1 counters"))).count();
    assert!(exiles > 0, "exile mode must resolve");
    assert!(searches > 0, "land search mode must resolve");
    assert!(bears > 0, "Bear mode must resolve");
    assert!(counters > 0, "counter mode must resolve");
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
        "exile_grave_events": exiles,
        "searched_land_entries": searches,
        "bear_tokens": bears,
        "counter_events": counters,
        "spirit_static_samples": spirit_static_samples,
        "boosted_spirit_samples": boosted_spirit_samples,
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
