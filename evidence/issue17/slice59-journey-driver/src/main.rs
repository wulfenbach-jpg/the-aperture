//! Evidence-only demonstration driver for slice 59, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. It prefers offered nonempty attacks, lands,
//! Dúnedain Rangers casts and other creatures to exercise the new combat trigger.
//! It is a 40-card package, not a starter certificate.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::view::PendingKind;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep,
    SessionSave,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice59.txt";
const SEED: u64 = 2323;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice59", &text).unwrap();
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

    // Public-state instrumentation only; never fed to the projected policy.
    while !game.state.is_over() && steps < 20000 {
        let player = game.decision_owner().expect("decision owner");
        let view = game.view_for(player, &events);

        let ring_action = if view.pending_kind.is_some() {
            view.legal.iter().find(|a| matches!(a.action, Action::ChooseRingBearer { .. })).map(|a| a.action.clone())
        } else { None };
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
        let demo_action = ring_action.or(search_action).or_else(|| {
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
                                .any(|c| c.id == *card && c.card == CardId::DunedainRangers),
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
    let out = root.join("evidence/issue17/slice59/native");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("demo-save.json"), save_text).expect("write save");

    let mut casts = BTreeMap::<String, usize>::new();
    for event in &events { if let Event::Cast { card, .. } = event { *casts.entry(card.name().to_string()).or_default() += 1; } }
    assert!(casts.get("Dúnedain Rangers").copied().unwrap_or(0) > 0, "Rangers must be cast");
    let combat = events.iter().filter(|e| matches!(e, Event::CombatDamage { .. })).count();
    let ring_bearers = game.state.rings.iter().filter(|r| r.bearer.is_some()).count();
    assert!(ring_bearers > 0, "the Ring must have assigned a bearer");
    assert!(combat > 0, "combat must occur");
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
        "ring_bearers": ring_bearers,
        "combat_damage_events": combat,
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
