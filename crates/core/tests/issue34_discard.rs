//! #34 D1 — the cleanup discard (CR 514.1) must be a real, bounded choice, and
//! an invalid submission must still be refused with the state untouched.
//!
//! Regression origin: with one hard-coded "last N cards of hand" offer, an AI
//! policy that could not match the sample had its answer dropped by the UI and
//! the table stopped at the cleanup step forever.

mod common;

use aperture_core::engine::MAX_DISCARD_OFFER;
use aperture_core::*;
use common::*;

/// Seven lands plus one spell: nine cards, so cleanup discards `count == 1`.
fn count_one_hand() -> Vec<CardId> {
    vec![
        CardId::Forest,
        CardId::Forest,
        CardId::Forest,
        CardId::Forest,
        CardId::Forest,
        CardId::Forest,
        CardId::Forest,
        CardId::Shock,
    ]
}

fn reach_cleanup_discard(seed: u64, hand: &[CardId]) -> Game {
    let mut game = fixture(seed)
        .hand(P0, hand)
        .main(P0, Step::PrecombatMain)
        .build();
    let mut log = Vec::new();
    let reached = run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::Discard { .. })),
        64,
    );
    assert!(reached, "the cleanup discard was never reached");
    assert_eq!(
        game.state.turn.priority, None,
        "no priority while a cleanup choice is pending"
    );
    game
}

fn discard_sets(actions: &[LegalAction]) -> Vec<Vec<ObjectId>> {
    actions
        .iter()
        .filter_map(|legal| match &legal.action {
            Action::Discard { cards } => Some(cards.clone()),
            _ => None,
        })
        .collect()
}

/// CR 514.1: "each player discards down to their maximum hand size". When only
/// one card is discarded the choice is genuinely per-card, so the pending offer
/// must contain one action for every card in hand — not a single sample.
#[test]
fn single_card_cleanup_discard_offers_every_hand_card() {
    let game = reach_cleanup_discard(3401, &count_one_hand());
    let hand = game.state.player(P0).hand.clone();
    assert_eq!(hand.len(), 8);
    let offer = game.legal_actions();
    let sets = discard_sets(&offer);
    assert_eq!(
        sets.len(),
        hand.len(),
        "expected one discard offer per hand card, got {}",
        sets.len()
    );
    for card in &hand {
        assert!(
            sets.contains(&vec![*card]),
            "no offer discards the single card {card:?}"
        );
    }
    let mut unique = sets.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), sets.len(), "offers must be distinct sets");
    for legal in &offer {
        assert!(
            matches!(legal.action, Action::Discard { .. }),
            "a cleanup choice must offer only discards: {:?}",
            legal.action
        );
        assert!(
            legal.rules.contains(&"514.1"),
            "the offer must cite CR 514.1: {:?}",
            legal.rules
        );
        assert!(
            !legal.label.is_empty(),
            "every offer needs a readable label"
        );
    }
}

/// A multi-card discard cannot enumerate every combination, so the offer is a
/// bounded deterministic set: at least the historical sample, never more than
/// the documented cap, every entry a legal `count`-card set.
#[test]
fn multi_card_cleanup_discard_offer_is_bounded_and_legal() {
    let game = reach_cleanup_discard(
        3402,
        &[
            CardId::Forest,
            CardId::Forest,
            CardId::Forest,
            CardId::Forest,
            CardId::Forest,
            CardId::Forest,
            CardId::Forest,
            CardId::Shock,
            CardId::Shock,
            CardId::Forest,
            CardId::Forest,
        ],
    );
    let hand = game.state.player(P0).hand.clone();
    assert_eq!(hand.len(), 11);
    let sets = discard_sets(&game.legal_actions());
    assert!(sets.len() >= 2, "a real choice needs at least two offers");
    assert!(
        sets.len() <= MAX_DISCARD_OFFER,
        "the offer must stay bounded: {} > {MAX_DISCARD_OFFER}",
        sets.len()
    );
    let mut unique = sets.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), sets.len(), "offers must be distinct sets");
    for set in &sets {
        assert_eq!(set.len(), 4, "every offer discards exactly count cards");
        assert!(
            set.iter().all(|card| hand.contains(card)),
            "every offered card must be in that player's hand"
        );
        let mut sorted = set.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), set.len(), "no offer repeats a card");
    }
    // The same position must always produce the same offer.
    let again = reach_cleanup_discard(
        3402,
        &[
            CardId::Forest,
            CardId::Forest,
            CardId::Forest,
            CardId::Forest,
            CardId::Forest,
            CardId::Forest,
            CardId::Forest,
            CardId::Shock,
            CardId::Shock,
            CardId::Forest,
            CardId::Forest,
        ],
    );
    assert_eq!(
        discard_sets(&again.legal_actions()),
        sets,
        "the bounded offer must be deterministic"
    );
}

/// `check_selection` stays authoritative: a short, over-count, duplicated or
/// foreign set is refused and nothing about the game moves.
#[test]
fn invalid_discard_submissions_are_refused_with_the_state_untouched() {
    let mut game = reach_cleanup_discard(3403, &count_one_hand());
    let hand = game.state.player(P0).hand.clone();
    let opponent_card = game.state.player(P1).hand.first().copied();
    let digest = game.digest();
    let revision = game.revision();
    let decision = game.decision();
    let log_len = game.state.objects.len();

    let empty: Vec<ObjectId> = Vec::new();
    let two = vec![hand[0], hand[1]];
    let duplicated = vec![hand[0], hand[0]];
    let err = submit_err(&mut game, P0, Action::Discard { cards: empty }).to_string();
    assert!(err.contains("expected 1"), "{err}");
    let err = submit_err(&mut game, P0, Action::Discard { cards: two }).to_string();
    assert!(err.contains("expected 1"), "{err}");
    let err = submit_err(&mut game, P0, Action::Discard { cards: duplicated }).to_string();
    assert!(err.contains("twice") || err.contains("expected 1"), "{err}");
    if let Some(foreign) = opponent_card {
        let err = submit_err(
            &mut game,
            P0,
            Action::Discard {
                cards: vec![foreign],
            },
        )
        .to_string();
        assert!(err.contains("hand"), "{err}");
    }
    let err = submit_err(
        &mut game,
        P0,
        Action::Discard {
            cards: vec![ObjectId(9_999)],
        },
    )
    .to_string();
    assert!(err.contains("hand"), "{err}");

    assert_eq!(game.digest(), digest, "a refusal must not change the state");
    assert_eq!(
        game.revision(),
        revision,
        "a refusal must not bump revision"
    );
    assert_eq!(
        game.decision(),
        decision,
        "a refusal must not move the choice"
    );
    assert_eq!(
        game.state.objects.len(),
        log_len,
        "a refusal must not create or remove objects"
    );
    assert!(
        matches!(game.state.pending, Some(Pending::Discard { count: 1, .. })),
        "the cleanup choice must still be pending after refusals"
    );
    // And the position is still completable.
    submit_ok(
        &mut game,
        P0,
        Action::Discard {
            cards: vec![hand[7]],
        },
    );
    assert!(
        game.state.pending.is_none(),
        "the legal answer must finish the cleanup step"
    );
}

/// The offer belongs to the seat that has to choose: an unequal hand on the
/// inactive seat must not leak into it.
#[test]
fn the_offer_only_lists_the_choosing_seats_cards() {
    let mut game = fixture(3404)
        .hand(P0, &count_one_hand())
        .hand(
            P1,
            &[
                CardId::Island,
                CardId::Island,
                CardId::Island,
                CardId::Counterspell,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::Discard { count: 1, .. })),
        64
    ));
    let own = game.state.player(P0).hand.clone();
    let other = game.state.player(P1).hand.clone();
    let sets = discard_sets(&game.legal_actions());
    assert_eq!(sets.len(), own.len());
    for set in sets {
        for card in set {
            assert!(own.contains(&card), "the offer leaked a card not in hand");
            assert!(
                !other.contains(&card),
                "the offer leaked the other seat's card"
            );
        }
    }
}
