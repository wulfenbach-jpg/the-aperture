//! Focused tests for the budgeted information-set search module.
//!
//! Proven by execution, not by reading the module:
//! - reconstruction + rollout are reproducible,
//! - candidates come only from the engine's offer,
//! - hidden card identities never reach the analysis (identity-leak pair test),
//! - budget exhaustion and cancellation both land on the labelled fallback,
//! - pending decision points are refused with a typed reason,
//! - a real two-seat game runs the search end to end,
//! - a small PAIRED trial against the frozen heuristic reports real outcomes
//!   with its limitations stated — never a strength claim from search size.

mod common;

use aperture_ai::{
    DecisionSource, Knowledge, SearchCancellation, SearchConfig, SearchUnavailable, fallback,
    search,
};
use aperture_core::*;
use common::{deck_a, deck_b};

fn small_config() -> SearchConfig {
    SearchConfig {
        seeds: 2,
        node_budget: 24_000,
        max_rollout_nodes: 800,
        max_millis: 20_000,
        max_candidates: 6,
    }
}

fn red_pool() -> ManaPool {
    let mut pool = ManaPool::default();
    pool.add(ManaColor::Red, 2);
    pool
}

fn submit(game: &mut Game, owner: PlayerId, action: Action) {
    if let Err(err) = game.submit_action(owner, action.clone()) {
        panic!("driver rejected P{owner} {action:?}: {err}");
    }
}

/// Drive a real 2-seat Practice20 game with minimal pendings until `pred`
/// holds. Never a shortcut through hidden state — every step is an engine
/// submit.
fn drive_until(game: &mut Game, pred: impl Fn(&Game) -> bool) {
    for _ in 0..8_000 {
        if pred(game) || game.state.is_over() {
            return;
        }
        let Some(owner) = game.decision_owner() else {
            return;
        };
        let action = if game.state.pending.is_some() {
            let offers = game.legal_actions();
            let keep = offers.into_iter().find_map(|legal| match legal.action {
                Action::Mulligan { .. } | Action::BottomCards { .. } | Action::Discard { .. } => {
                    Some(legal.action)
                }
                _ => None,
            });
            keep.unwrap_or(Action::Pass)
        } else {
            Action::Pass
        };
        submit(game, owner, action);
    }
    panic!("drive_until never converged");
}

#[test]
fn reconstruction_is_reproducible_determinization() {
    let game = common::fixture(42)
        .battlefield(P0, &[CardId::Forest, CardId::Mountain])
        .hand(P0, &[CardId::GrizzlyBears, CardId::Shock])
        .library(P1, &[CardId::Island; 4])
        .hand(P1, &[CardId::Island, CardId::Island])
        .mana(P0, red_pool())
        .main(P0, aperture_core::state::Step::PrecombatMain)
        .build();
    let view = game.view_for(P0, &[]);
    let knowledge = Knowledge { own_deck: None };
    let cancel = SearchCancellation::new();
    let first = search(&game, &view, &knowledge, &small_config(), &cancel);
    assert!(
        first.is_ok(),
        "search should run on a plain priority decision: {:?}",
        first.err().map(|e| e.label())
    );
    let second = search(&game, &view, &knowledge, &small_config(), &cancel).unwrap();
    let first = first.unwrap();
    assert_eq!(
        first, second,
        "same observation + seeds must give the same report"
    );
    assert!(first.analysis.decisions_used > 0);
}

/// Every scored candidate is an action the engine offered — never an invented
/// or off-list move.
#[test]
fn candidates_are_only_offered_actions() {
    let game = common::fixture(42)
        .battlefield(P0, &[CardId::Forest, CardId::Mountain])
        .hand(P0, &[CardId::GrizzlyBears, CardId::Shock])
        .library(P1, &[CardId::Island; 4])
        .hand(P1, &[CardId::Island, CardId::Island])
        .mana(P0, red_pool())
        .main(P0, aperture_core::state::Step::PrecombatMain)
        .build();
    let view = game.view_for(P0, &[]);
    let outcome = search(
        &game,
        &view,
        &Knowledge { own_deck: None },
        &small_config(),
        &SearchCancellation::new(),
    )
    .expect("search runs");
    for candidate in &outcome.analysis.candidates {
        assert!(
            view.legal
                .iter()
                .any(|legal| legal.action == candidate.action),
            "candidate \"{}\" is not in the engine's offer",
            candidate.label
        );
    }
}

/// The identity-leak pair test: two real worlds identical except the actual
/// hidden contents of the opponent's hand must give identical reports, because
/// the search model re-deals hidden zones from the disclosed belief deck. Any
/// difference would prove a read of the real hand.
#[test]
fn hidden_hand_identity_never_leaks_into_the_analysis() {
    let base = || {
        common::fixture(7)
            .battlefield(P0, &[CardId::Forest, CardId::Mountain])
            .hand(P0, &[CardId::GrizzlyBears])
            .library(P1, &[CardId::Island; 3])
            .mana(P0, red_pool())
            .main(P0, aperture_core::state::Step::PrecombatMain)
    };
    // World 1: P1's one hidden card is strong white removal.
    let removal = base().hand(P1, &[CardId::SwordsToPlowshares]).build();
    // World 2: P1's one hidden card is a different identity entirely.
    let creature = base().hand(P1, &[CardId::GrizzlyBears]).build();
    let knowledge = Knowledge { own_deck: None };
    let cancel = SearchCancellation::new();
    let a = search(
        &removal,
        &removal.view_for(P0, &[]),
        &knowledge,
        &small_config(),
        &cancel,
    )
    .expect("world 1 runs");
    let b = search(
        &creature,
        &creature.view_for(P0, &[]),
        &knowledge,
        &small_config(),
        &cancel,
    )
    .expect("world 2 runs");
    assert_eq!(
        a.analysis.candidates, b.analysis.candidates,
        "the analysis changed although the only difference was hidden hand identity"
    );
}

#[test]
fn budget_exhaustion_labels_the_fallback() {
    let game = common::fixture(42)
        .battlefield(P0, &[CardId::Forest, CardId::Mountain])
        .hand(P0, &[CardId::GrizzlyBears])
        .library(P1, &[CardId::Island; 4])
        .mana(P0, red_pool())
        .main(P0, aperture_core::state::Step::PrecombatMain)
        .build();
    let view = game.view_for(P0, &[]);
    let cancel = SearchCancellation::new();
    let config = SearchConfig {
        seeds: 2,
        node_budget: 1,
        max_rollout_nodes: 1,
        max_millis: u64::MAX,
        max_candidates: 6,
    };
    let outcome = search(
        &game,
        &view,
        &Knowledge { own_deck: None },
        &config,
        &cancel,
    )
    .expect("search runs");
    assert_ne!(
        outcome.decision.source,
        DecisionSource::Search,
        "nothing scored, so search cannot claim the decision"
    );
    assert_eq!(outcome.decision.source, DecisionSource::Heuristic);
    assert!(
        outcome
            .analysis
            .fidelity
            .contains("deterministic legal fallback"),
        "fallback must be labelled: {}",
        outcome.analysis.fidelity
    );
}

#[test]
fn cancellation_reports_the_labelled_fallback() {
    let game = common::fixture(42)
        .battlefield(P0, &[CardId::Forest, CardId::Mountain])
        .hand(P0, &[CardId::GrizzlyBears])
        .library(P1, &[CardId::Island; 4])
        .mana(P0, red_pool())
        .main(P0, aperture_core::state::Step::PrecombatMain)
        .build();
    let view = game.view_for(P0, &[]);
    let cancel = SearchCancellation::new();
    cancel.cancel();
    let outcome = search(
        &game,
        &view,
        &Knowledge { own_deck: None },
        &small_config(),
        &cancel,
    )
    .expect("search runs");
    assert_eq!(outcome.decision.source, DecisionSource::Heuristic);
    assert!(outcome.analysis.cancelled);
    assert!(
        outcome
            .analysis
            .fidelity
            .contains("deterministic legal fallback")
    );
}

#[test]
fn pending_decisions_are_refused_with_a_typed_reason() {
    let fixture = common::fixture(3)
        .hand(P0, &[CardId::GrizzlyBears])
        .hand(P1, &[CardId::Island])
        .mulligan(P0)
        .build();
    let view = fixture.view_for(P0, &[]);
    let err = search(
        &fixture,
        &view,
        &Knowledge { own_deck: None },
        &small_config(),
        &SearchCancellation::new(),
    )
    .expect_err("a mulligan pending is not searchable");
    assert!(
        matches!(err, SearchUnavailable::PendingDecision { .. }),
        "expected the typed pending refusal, got: {}",
        err.label()
    );
    // The fallback remains the deterministic legal choice.
    let decision = fallback(&view);
    assert_eq!(decision.source, DecisionSource::Heuristic);
}

#[test]
fn declared_own_deck_is_used_for_own_draws_only() {
    let decks = [deck_a(), deck_b()];
    let mut game = Game::new(2024, &[decks[0].clone(), decks[1].clone()]).expect("game builds");
    drive_until(&mut game, |g| {
        let view = g.view_for(P0, &[]);
        view.pending_kind.is_none()
            && view.stack.is_empty()
            && !view.legal.is_empty()
            && view.turn_number >= 2
    });
    let view = game.view_for(P0, &[]);
    let knowledge = Knowledge {
        own_deck: Some(deck_a()),
    };
    let outcome = search(
        &game,
        &view,
        &knowledge,
        &small_config(),
        &SearchCancellation::new(),
    )
    .expect("search runs on a real game");
    assert!(
        outcome
            .analysis
            .fidelity
            .contains("own future draws sampled from declared deck"),
        "{}",
        outcome.analysis.fidelity
    );
    assert!(outcome.analysis.decisions_used > 0);
    assert!(
        view.legal
            .iter()
            .any(|legal| legal.action == outcome.decision.action),
        "the chosen action must be one of the offered ones"
    );
}

/// Small real PAIRED trial vs the frozen heuristic: both seat orders on the
/// same seeds. The printed report is the deliverable — raw outcomes and
/// limitations, not a strength claim.
#[test]
fn small_paired_trial_against_the_frozen_heuristic() {
    const PAIRS: u64 = 2;
    let mut search_wins = 0usize;
    let mut heuristic_wins = 0usize;
    let mut draws_or_unfinished = 0usize;
    for pair in 0..PAIRS {
        for (leg, seat) in [(0u64, P0), (1u64, P1)] {
            let seed = 0xBEF0 ^ (pair << 8) ^ leg;
            let (outcome, search_decisions, _fallback) = play_pair_with_search(seed, seat);
            match outcome {
                Some(aperture_core::state::Outcome::Win { winner, .. }) if winner == seat => {
                    search_wins += 1;
                }
                Some(aperture_core::state::Outcome::Win { .. }) => {
                    heuristic_wins += 1;
                }
                _ => draws_or_unfinished += 1,
            }
            if search_decisions == 0 {
                panic!(
                    "pair {pair} leg {leg}: the search seat never reached a searchable decision"
                );
            }
        }
    }
    println!("paired trial vs frozen heuristic (practice outcomes only):");
    println!(
        "  search-leg wins {search_wins} : heuristic-leg wins {heuristic_wins} : draws/caps {draws_or_unfinished}"
    );
    println!(
        "  limitations: {} pairs, one practice matchup, neutral-criterion opponent model, one determinization seed per decision, search disabled at pending choices. This does NOT establish stronger play; a conclusion needs a much larger mirrored budget-balanced trial.",
        PAIRS
    );
    assert_eq!(
        search_wins + heuristic_wins + draws_or_unfinished,
        2 * PAIRS as usize,
        "every leg lands in exactly one outcome class"
    );
}

/// One paired leg: `search_seat` places budgeted search decisions, every other
/// seat (and every pending choice) uses the frozen heuristic. Returns the
/// engine outcome plus how often search claimed a decision.
fn play_pair_with_search(
    seed: u64,
    search_seat: PlayerId,
) -> (Option<aperture_core::state::Outcome>, usize, usize) {
    let decks = [deck_a(), deck_b()];
    let mut game = Game::new(seed, &[decks[0].clone(), decks[1].clone()]).expect("game builds");
    let knowledge = Knowledge { own_deck: None };
    let config = SearchConfig {
        seeds: 1,
        node_budget: 800,
        max_rollout_nodes: 250,
        max_millis: 3_000,
        max_candidates: 6,
    };
    let cancel = SearchCancellation::new();
    let mut log: Vec<Event> = Vec::new();
    let mut search_decisions = 0usize;
    let mut fallback_decisions = 0usize;
    for _ in 0..8_000 {
        if game.state.is_over() {
            break;
        }
        let Some(owner) = game.decision_owner() else {
            break;
        };
        let view = game.view_for(owner, &log);
        let decision = if owner == search_seat && view.pending_kind.is_none() {
            match search(&game, &view, &knowledge, &config, &cancel) {
                Ok(outcome) if outcome.decision.source == DecisionSource::Search => {
                    search_decisions += 1;
                    Some(outcome.decision)
                }
                Ok(_) | Err(_) => {
                    let decided = fallback(&view);
                    fallback_decisions += 1;
                    Some(decided)
                }
            }
        } else {
            fallback_decisions += 1;
            Some(fallback(&view))
        };
        let Some(decision) = decision else { break };
        let action = decision.action;
        let sub = Submission {
            player: owner,
            revision: game.revision(),
            decision: game.decision(),
            action,
        };
        match game.submit(&sub) {
            Ok(events) => log.extend(events),
            Err(err) => panic!("paired-trial driver rejected P{owner} {sub:?}: {err}"),
        }
    }
    (
        game.state.outcome.clone(),
        search_decisions,
        fallback_decisions,
    )
}
