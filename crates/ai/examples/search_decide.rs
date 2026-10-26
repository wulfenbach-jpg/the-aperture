//! Real end-to-end journey for the budgeted information-set search:
//! a fresh 2-seat practice game is driven with the frozen heuristic to a P0
//! turn-2 priority decision, the search evaluates every engine-offered
//! candidate under budgets, and the printed report shows the chosen offered
//! action, the per-candidate scores and the determinization fidelity
//! disclosure. Run with:
//! `cargo run -p aperture-ai --example search_decide`

use aperture_ai::{DecisionSource, Knowledge, SearchCancellation, SearchConfig, fallback, search};
use aperture_core::state::Pending;
use aperture_core::*;

const SEED: u64 = 0xC0FFEE;

fn deck_a() -> DeckList {
    DeckList::parse_practice(
        "Temur Beats",
        include_str!("../../../decks/temur-beats.txt"),
    )
    .expect("deck A parses")
}

fn deck_b() -> DeckList {
    DeckList::parse_practice(
        "Izzet Control",
        include_str!("../../../decks/izzet-control.txt"),
    )
    .expect("deck B parses")
}

fn main() {
    let decks = [deck_a(), deck_b()];
    let mut game = Game::new(SEED, &[decks[0].clone(), decks[1].clone()]).expect("game builds");
    let mut log: Vec<Event> = Vec::new();

    // Drive to a real mid-game priority decision for P0.
    let mut reached = false;
    for _ in 0..8_000 {
        if game.state.is_over() {
            panic!("the game ended before turn 2; use a different seed");
        }
        let Some(owner) = game.decision_owner() else {
            panic!("no decision available with a live game");
        };
        let view = game.view_for(owner, &log);
        let action = if game.state.pending.is_some() {
            match &game.state.pending {
                Some(Pending::Mulligan { .. })
                | Some(Pending::BottomCards { .. })
                | Some(Pending::Discard { .. }) => Action::Mulligan { keep: true },
                Some(Pending::DeclareAttackers { .. }) => Action::DeclareAttackers {
                    attackers: Vec::new(),
                    defender: None,
                },
                Some(Pending::DeclareBlockers { .. }) => {
                    Action::DeclareBlockers { blocks: Vec::new() }
                }
                _ => view
                    .legal
                    .first()
                    .map(|legal| legal.action.clone())
                    .unwrap_or(Action::Pass),
            }
        } else {
            Action::Pass
        };
        if owner == P0
            && game.state.pending.is_none()
            && !view.legal.is_empty()
            && view.turn_number >= 2
            && matches!(view.step, aperture_core::state::Step::PrecombatMain)
        {
            reached = true;
            break;
        }
        let sub = Submission {
            player: owner,
            revision: game.revision(),
            decision: game.decision(),
            action: action.clone(),
        };
        match game.submit(&sub) {
            Ok(events) => log.extend(events),
            Err(err) => panic!("journey driver rejected P{owner} {action:?}: {err}"),
        }
    }
    assert!(reached, "did not reach a P0 priority decision");
    let view = game.view_for(P0, &log);
    println!(
        "real practice game (seed {SEED}); decision point: turn {}, P0, {:?}, stack {}, offer {} (engine-generated, masked view)",
        view.turn_number,
        view.step,
        if view.stack.is_empty() {
            "empty"
        } else {
            "occupied"
        },
        view.legal.len(),
    );

    // Budgeted search. The viewer's own deck is legitimately known here
    // because this driver set it up; the opponent is modeled by the
    // disclosed neutral belief deck.
    let knowledge = Knowledge {
        own_deck: Some(decks[0].clone()),
    };
    let config = SearchConfig::default();
    let cancel = SearchCancellation::new();
    match search(&game, &view, &knowledge, &config, &cancel) {
        Ok(outcome) => {
            println!(
                "\nsearch decision: {} (source: {})\nengine decisions used: {}{}\n",
                outcome.decision.source.label(),
                match outcome.decision.source {
                    DecisionSource::Search => "info-set search".to_string(),
                    _ => format!("fallback — {}", outcome.decision.source.label()),
                },
                outcome.analysis.decisions_used,
                if outcome.analysis.cancelled {
                    " (CANCELLED)"
                } else {
                    ""
                }
            );
            println!("candidates (root offer order):");
            for c in &outcome.analysis.candidates {
                println!(
                    "  {:<24} mean {:>6}  scored {}  no-result {}  refused {}",
                    c.label,
                    c.mean
                        .map_or_else(|| "-".to_string(), |v| format!("{v:+.3}")),
                    c.scored_samples,
                    c.rollouts_without_result,
                    c.refused_times,
                );
            }
            println!(
                "\ndeterminization disclosure: {}",
                outcome.analysis.fidelity
            );
            println!(
                "actions higher mean here are practice-outcome estimates under a neutral opponent model — not a measured strength claim."
            );
        }
        Err(unavailable) => {
            println!("\nsearch unavailable here: {}", unavailable.label());
            let decision = fallback(&view);
            println!(
                "deterministic legal fallback: {} (source: {})",
                decision.source.label(),
                decision.source.label()
            );
        }
    }
}
