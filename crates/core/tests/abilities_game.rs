//! Real gameplay + replay smoke for issue 14: a full scripted practice game
//! that actually activates an ability and resolves a triggered ability, then
//! reproduces the same state digest from the recorded replay.

mod common;

use aperture_core::*;
use common::auto_answer;

const DECK_ABILITIES_A: &str = r#"// Ability practice A (40 cards, supported pool only)
9 Forest
8 Island
4 Prodigal Sorcerer
4 Elvish Visionary
3 Rootwalla
3 Brindle Boar
4 Festering Goblin
5 Grizzly Bears
"#;

const DECK_ABILITIES_B: &str = r#"// Ability practice B (40 cards, supported pool only)
9 Island
8 Forest
4 Oculus
4 Festering Goblin
4 Runeclaw Bear
4 Centaur Courser
4 Hill Giant
3 Unsummon
"#;

fn deck_a() -> DeckList {
    DeckList::parse_practice("Ability practice A", DECK_ABILITIES_A).expect("deck A parses")
}

fn deck_b() -> DeckList {
    DeckList::parse_practice("Ability practice B", DECK_ABILITIES_B).expect("deck B parses")
}

/// Deterministic policy: use an activated ability when one is offered, then
/// develop the board, otherwise pass. Pending choices use the shared scripted
/// answer, which accepts optional triggers and declares targets.
fn ability_policy(game: &Game) -> Action {
    if game.state.pending.is_some() {
        return auto_answer(game);
    }
    let legal = game.legal_actions();
    for wanted in [
        |action: &Action| matches!(action, Action::ActivateAbility { .. }),
        |action: &Action| matches!(action, Action::PlayLand { .. }),
        |action: &Action| matches!(action, Action::CastSpell { .. }),
    ] {
        if let Some(option) = legal.iter().find(|legal| wanted(&legal.action)) {
            return option.action.clone();
        }
    }
    Action::Pass
}

#[test]
fn real_game_activates_and_triggers_abilities_and_replays_identically() {
    let (mut game, mut log) = Game::new_logged(4242, &[deck_a(), deck_b()]).expect("game");
    let mut steps = Vec::new();

    for _ in 0..600 {
        if game.state.is_over() {
            break;
        }
        let Some(owner) = game.decision_owner() else {
            break;
        };
        let action = ability_policy(&game);
        let step = ReplayStep {
            player: owner,
            revision: game.revision(),
            decision: game.decision(),
            action: action.clone(),
        };
        match game.submit_action(owner, action) {
            Ok(events) => {
                log.extend(events);
                steps.push(step);
            }
            Err(err) => panic!("scripted step rejected: {err}"),
        }
    }

    let activations = log
        .iter()
        .filter(|event| matches!(event, Event::AbilityActivated { .. }))
        .count();
    let triggers = log
        .iter()
        .filter(|event| matches!(event, Event::TriggerStaged { .. }))
        .count();
    let resolved = log
        .iter()
        .filter(|event| matches!(event, Event::AbilityResolved { .. }))
        .count();
    assert!(
        activations > 0,
        "a real game must actually activate an ability (log lines: {})",
        log.len()
    );
    assert!(
        triggers > 0,
        "a real game must actually trigger an ability (log lines: {})",
        log.len()
    );
    assert!(
        resolved > 0,
        "an activated or triggered ability must resolve in the real game"
    );
    // The game really progressed: lands were played and turns taken.
    assert!(game.state.turn.number >= 3, "the game reached turn 3");

    // R170-1: no `Priority` event may appear inside an ability's own
    // resolution (CR 117.5, 608.2). Every completed ability must be reported
    // before the next priority grant, so no `AbilityResolved` may be preceded by
    // a priority event.
    let resolved_indices: Vec<usize> = log
        .iter()
        .enumerate()
        .filter(|(_, event)| matches!(event, Event::AbilityResolved { .. }))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        resolved_indices.len(),
        resolved,
        "the audit covers every resolution in the real game"
    );
    for index in resolved_indices {
        assert!(
            index > 0 && !matches!(log[index - 1], Event::Priority { .. }),
            "no Priority may be granted while an ability is still resolving: {:?}",
            &log[index.saturating_sub(2)..=index]
        );
    }

    let mut replay = Replay::new(
        4242,
        game.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .expect("replay");
    for step in steps {
        replay.record(step);
    }
    replay.outcome = game.state.outcome.clone();
    replay.final_digest = game.digest();

    let (replayed, replayed_log) = replay.run().expect("the replay reproduces the game");
    assert_eq!(replayed.digest(), game.digest(), "state digest reproduces");
    assert_eq!(
        replayed_log
            .iter()
            .filter(|event| matches!(event, Event::AbilityActivated { .. }))
            .count(),
        activations,
        "the replay produces the same ability events"
    );

    // A save round-trip preserves the new ability state exactly.
    let save = SessionSave::capture(&game, replay.clone(), log.clone(), false);
    let restored = SessionSave::from_json(&save.to_json()).expect("save round-trips");
    assert_eq!(restored.game().digest(), game.digest());
    assert_eq!(restored.log, log);

    println!(
        "abilities game: {} steps, {} activations, {} triggers, {} resolved, turn {}, digest {}",
        replay.steps.len(),
        activations,
        triggers,
        resolved,
        game.state.turn.number,
        game.digest()
    );
}
