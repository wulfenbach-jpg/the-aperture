//! Evidence-only demonstration driver for slice 18, NOT a product AI and NOT a
//! strength claim. It chooses only from masked legal actions projected for the
//! seat that has priority: no deck substitution, no state injection, no
//! hidden-information access. Its only preference is "prefer the new cards and
//! their printed abilities", which is what makes the journey exercise them.
use aperture_ai::{HeuristicPolicy, Policy};
use aperture_core::state::Step;
use aperture_core::{
    Action, AiSettings, CardId, CommanderSetup, DeckList, Event, Game, Replay, ReplayStep, SessionSave, Target,
};
use std::collections::BTreeMap;

const PACKAGE: &str = "decks/package-slice18.txt";
const SEED: u64 = 1616;

fn main() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let text = std::fs::read_to_string(root.join(PACKAGE)).unwrap();
    let deck = DeckList::parse_practice("package-slice18", &text).unwrap();
    let seats = vec![deck.clone(); 4];
    // A green-white commander keeps the package's colour identity legal; the
    // demonstration never relies on the commander's own abilities.
    let commanders: Vec<_> = (0..4).map(|p| (p, CardId::AyulaQueenAmongBears)).collect();
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
    let mut counters_mode = 0usize;
    let mut fight_mode = 0usize;
    let mut mode_toggle = false;
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
                    // Kill a creature first, then replay the death payoffs.
                    view.legal.iter().find(|a| match &a.action {
                        Action::CastSpell { card, targets } => {
                            view.own_hand.iter().any(|c| {
                                c.id == *card && c.card == CardId::Shock
                            }) && targets.iter().any(|target| match target {
                                Target::Permanent(id) => view.own_battlefield.iter().any(|c| {
                                    c.id == *id && c.card.def().is_creature() && !c.card.is_token()
                                }),
                                _ => false,
                            })
                        }
                        _ => false,
                    })
                })
                .or_else(|| {
                    view.legal.iter().find(|a| match &a.action {
                        Action::CastSpell { card, .. } => view.own_hand.iter().any(|c| {
                            c.id == *card
                                && (c.card == CardId::FriendlyTeddy
                                    || c.card == CardId::UlvenwaldBear
                                    || c.card == CardId::CallerOfTheClaw)
                        }),
                        _ => false,
                    })
                })
                .or_else(|| {
                    // Then the package's other permanents, so the destruction
                    // has real artifacts and enchantments to sweep.
                    view.legal.iter().find(|a| match &a.action {
                        Action::CastSpell { card, .. } => view.own_hand.iter().any(|c| {
                            c.id == *card
                                && (c.card.def().is_enchantment()
                                    || c.card.def().is_artifact()
                                    || c.card.def().is_creature())
                        }),
                        _ => false,
                    })
                })
                .map(|a| a.action.clone())
        } else {
            None
        };
        let mode_action = {
            let offered: Vec<_> = view
                .legal
                .iter()
                .filter(|legal| matches!(legal.action, Action::AnswerTriggerChoice { .. }))
                .collect();
            if offered.len() == 2 {
                mode_toggle = !mode_toggle;
                let wanted = mode_toggle;
                offered
                    .into_iter()
                    .find(|legal| {
                        matches!(legal.action, Action::AnswerTriggerChoice { use_ability } if use_ability == wanted)
                    })
                    .map(|legal| legal.action.clone())
            } else {
                None
            }
        };
        if let Some(Action::AnswerTriggerChoice { use_ability }) = &mode_action {
            if *use_ability { counters_mode += 1 } else { fight_mode += 1 }
        }
        let action = mode_action.or(demo_action).unwrap_or_else(|| baseline.decide(&view).action);
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
    let save = SessionSave::capture(&game, replay.clone(), Vec::new(), false);
    let save_text = save.to_json();
    let loaded = SessionSave::from_json(&save_text).expect("fresh save reloads");
    assert_eq!(loaded.game, game.state, "fresh save preserves the exact state");
    std::fs::write(root.join("evidence/issue17/slice18/demo-save.json"), save_text)
        .expect("write save");

    let mut casts = BTreeMap::<String, usize>::new();
    let mut destroyed = BTreeMap::<String, usize>::new();
    let mut bears = 0usize;
    let mut untap_counters = 0usize;
    let mut untap_plain = 0usize;
    for event in &events {
        match event {
            Event::Cast { card, .. } => *casts.entry(card.name().to_string()).or_default() += 1,
            Event::Destroyed { card, .. } => {
                *destroyed.entry(card.name().to_string()).or_default() += 1
            }
            Event::PermanentEntered { card, .. } if *card == CardId::BearToken => bears += 1,
            Event::StateBasedAction { description }
                if description.contains("is untapped and has") =>
            {
                untap_counters += 1
            }
            Event::StateBasedAction { description }
                if description.contains("is untapped (not a Bear)") =>
            {
                untap_plain += 1
            }
            _ => {}
        }
    }
    for name in ["Friendly Teddy", "Ulvenwald Bear", "Caller of the Claw"] {
        assert!(
            casts.get(name).copied().unwrap_or(0) > 0,
            "{name} must actually be cast"
        );
    }
    assert!(counters_mode > 0 && fight_mode > 0, "both Ayula modes must be chosen: counters={counters_mode} fight={fight_mode}");
    assert!(casts.get("Ayula, Queen Among Bears").copied().unwrap_or(0) > 0, "Ayula must be cast from the command zone");
    assert!(events.iter().any(|e| matches!(e, Event::Damage { .. })), "fight must deal noncombat damage");
    assert!(events.iter().any(|e| matches!(e, Event::StateBasedAction { description } if description.contains("+1/+1 counter effect"))), "counter mode must resolve");

    let out = root.join("evidence/issue17/slice18");
    std::fs::write(
        out.join("demo-replay.json"),
        serde_json::to_string_pretty(&replay).unwrap(),
    )
    .unwrap();
    let report = serde_json::json!({
        "driver": "evidence-only masked card-preferring demonstrator; NOT product heuristic/strength evidence",
        "seed": SEED, "seats": 4, "starting_life": 40, "status": "terminal",
        "deck": PACKAGE, "kind": "40-card package, not a desired starter",
        "commanders": "Ayula, Queen Among Bears injected for all four seats by this demonstrator",
        "snapshots_exact": true, "snapshot_sha256": aperture_core::digest::snapshot_digest(text.as_bytes()).unwrap(),
        "steps": steps, "turns": game.state.turn.number, "digest": game.digest(),
        "replay_events": replay_events.len(), "replay_digest_matches": true,
        "casts": casts, "destroyed": destroyed, "activations": activations,
        "bear_tokens_created": bears,
        "counters_chosen": counters_mode,
        "fight_chosen": fight_mode,
        "fight_damage_events": events.iter().filter(|e| matches!(e, Event::Damage { .. })).count(),
        "outcome": game.state.outcome,
    });
    std::fs::write(
        out.join("demo-journey.json"),
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}


