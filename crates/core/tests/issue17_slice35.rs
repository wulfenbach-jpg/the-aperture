//! Slice 35: Words of Wilding (ONS 305) — the pool's first draw replacement.
//!
//! The printed text is (ONS 305, retrieved 2026-10-27T21:06:29Z, recorded in
//! `fixtures/starter-corpus/slice35-sourced.json`):
//!
//! "{1}: The next time you would draw a card this turn, create a 2/2 green Bear
//! creature token instead."
//!
//! The new rules surface is a *replacement* of a draw event (CR 614.1a): the
//! resolving ability arms exactly one one-shot replacement for its controller,
//! the next draw that controller would perform this turn is replaced by a Bear
//! token (no card moves, and an empty library is not a loss because nothing was
//! drawn), and an unspent replacement ends in cleanup (CR 514.2). The ability
//! prints no timing restriction and no once-per-turn limit, so several can be
//! armed in one turn; each consumes exactly one draw.
//!
//! Every assertion is made through the engine's own offers, resolutions and
//! events, never through a test-only shortcut.
mod common;

use aperture_core::*;
use common::*;

fn words() -> CardId {
    catalog::by_name("Words of Wilding").expect("Words of Wilding must be supported")
}

fn bears() -> CardId {
    catalog::by_name("Grizzly Bears").expect("Grizzly Bears must be supported")
}

fn divination() -> CardId {
    catalog::by_name("Divination").expect("Divination must be supported")
}

/// {1} in the colourless slot.
fn one_mana() -> ManaPool {
    ManaPool {
        colorless: 1,
        ..ManaPool::default()
    }
}

fn drain(game: &mut Game) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..128 {
        if game.state.pending.is_some() || game.state.stack.is_empty() {
            return out;
        }
        let Some(player) = game.decision_owner() else {
            return out;
        };
        out.extend(submit_ok(game, player, Action::Pass));
    }
    out
}

/// Activate "{1}: …" on the named source and resolve it.
fn activate_words(game: &mut Game, player: PlayerId, source: ObjectId) -> Vec<Event> {
    // Hold this seat's priority so the offer set is the seat's own: the ability
    // prints no timing restriction, so any seat with priority may activate it.
    set_priority(&mut game.state, Some(player));
    let action = game
        .legal_actions()
        .into_iter()
        .find(|entry| {
            matches!(
                entry.action,
                Action::ActivateAbility { source: s, .. } if s == source
            )
        })
        .expect("{1} is payable, so the ability must be offered")
        .action;
    let mut events = submit_ok(game, player, action);
    events.extend(drain(game));
    events
}

fn cast(game: &mut Game, player: PlayerId, hand: ObjectId) -> Vec<Event> {
    set_priority(&mut game.state, Some(player));
    let action = game
        .legal_actions()
        .into_iter()
        .find(|entry| matches!(&entry.action, Action::CastSpell { card, .. } if *card == hand))
        .expect("the spell must be castable")
        .action;
    let mut events = submit_ok(game, player, action);
    events.extend(drain(game));
    events
}

fn bear_count(game: &Game, player: PlayerId) -> usize {
    game.state
        .battlefield(player)
        .into_iter()
        .filter(|object| object.card == CardId::BearToken)
        .count()
}

// ---------------------------------------------------------------------------
// The replacement itself.
// ---------------------------------------------------------------------------

#[test]
fn the_armed_replacement_turns_that_turns_draw_step_into_a_bear() {
    let mut game = fixture(3535)
        .battlefield(P0, &[words()])
        .library(P0, &[bears(), bears(), bears()])
        .mana(P0, one_mana())
        .main(P0, Step::Upkeep)
        .build();
    // CR 103.8a: in a two-seat game the play-first seat skips its first draw
    // step, so this fixture is put past turn 1 for a real draw to happen.
    game.state.turn.number = 3;
    let source = battlefield_card(&game, P0, words());
    let mut log = activate_words(&mut game, P0, source);
    assert!(
        game.state.stack.is_empty(),
        "the ability resolves before the draw step"
    );
    let library_before = game.state.player(P0).library.len();
    let hand_before = game.state.player(P0).hand_size();
    assert_eq!(
        bear_count(&game, P0),
        0,
        "arming creates no token by itself"
    );

    assert!(
        run_until(
            &mut game,
            &mut log,
            |g| g.state.turn.step != Step::Upkeep,
            64
        ),
        "the turn reaches the draw step"
    );

    assert_eq!(
        game.state.player(P0).library.len(),
        library_before,
        "no card is drawn, so the library is unchanged"
    );
    assert_eq!(
        game.state.player(P0).hand_size(),
        hand_before,
        "no card is drawn, so the hand is unchanged"
    );
    assert_eq!(
        bear_count(&game, P0),
        1,
        "exactly one 2/2 green Bear token replaces the draw"
    );
    let drew = log
        .iter()
        .filter(|event| matches!(event, Event::Draw { player: P0, .. }))
        .count();
    assert_eq!(drew, 0, "the replaced draw emits no Draw event");
    assert!(
        !game.state.is_over(),
        "a replaced draw is not a loss (nothing was drawn)"
    );
}

#[test]
fn one_armed_replacement_replaces_exactly_one_of_two_draws() {
    let mut game = fixture(3535)
        .battlefield(P0, &[words()])
        .hand(P0, &[divination()])
        .library(P0, &[bears(), bears(), bears(), bears()])
        .mana(
            P0,
            ManaPool {
                colorless: 3,
                blue: 1,
                ..ManaPool::default()
            },
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&game, P0, words());
    activate_words(&mut game, P0, source);
    assert_eq!(bear_count(&game, P0), 0);

    let hand_before = game.state.player(P0).hand_size();
    let spell = hand_card(&game, P0, divination());
    let events = cast(&mut game, P0, spell);

    assert_eq!(
        game.state.player(P0).hand_size(),
        hand_before - 1 + 1,
        "Divination leaves the hand and only the second draw adds a card"
    );
    assert_eq!(
        bear_count(&game, P0),
        1,
        "the first of the two draws becomes one Bear"
    );
    let drew = events
        .iter()
        .filter(|event| matches!(event, Event::Draw { player: P0, .. }))
        .count();
    assert_eq!(drew, 1, "only the un-replaced draw emits a Draw event");
}

#[test]
fn two_armed_replacements_replace_both_draws_and_protect_an_empty_library() {
    let mut game = fixture(3535)
        .battlefield(P0, &[words()])
        .hand(P0, &[divination()])
        .mana(
            P0,
            ManaPool {
                colorless: 6,
                blue: 1,
                ..ManaPool::default()
            },
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&game, P0, words());
    activate_words(&mut game, P0, source);
    activate_words(&mut game, P0, source);
    assert_eq!(
        bear_count(&game, P0),
        0,
        "two activations still create no token by themselves"
    );

    let hand_before = game.state.player(P0).hand_size();
    let spell = hand_card(&game, P0, divination());
    let events = cast(&mut game, P0, spell);

    assert_eq!(
        bear_count(&game, P0),
        2,
        "each armed replacement is one Bear"
    );
    assert_eq!(
        game.state.player(P0).hand_size(),
        hand_before - 1,
        "neither draw adds a card"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::DrewFromEmpty { player: P0 })),
        "a replaced draw from an empty library is not a draw from an empty library"
    );
    assert!(
        !game.state.is_over(),
        "nothing was drawn, so an empty library is not a loss here"
    );
    assert!(
        !game.state.player(P0).drew_from_empty,
        "the seat is not marked as having drawn from an empty library"
    );
}

// ---------------------------------------------------------------------------
// Duration (CR 514.2) and the printed limit.
// ---------------------------------------------------------------------------

#[test]
fn an_unspent_replacement_ends_with_the_turn() {
    // Armed after this turn's draw step, so nothing can consume it before
    // cleanup: the next turn's draw must be an ordinary draw again.
    let mut game = fixture(3535)
        .battlefield(P0, &[words()])
        .library(P0, &[bears(); 12])
        .library(P1, &[bears(); 12])
        .mana(P0, one_mana())
        .main(P0, Step::PostcombatMain)
        .build();
    game.state.turn.number = 3;
    let source = battlefield_card(&game, P0, words());
    activate_words(&mut game, P0, source);

    let mut log = Vec::new();
    let reached = run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.active == P0 && g.state.turn.step == Step::Draw && g.state.turn.number > 1,
        2048,
    );
    assert!(reached, "the game reaches P0's next draw step");

    let bears_now = bear_count(&game, P0);
    let drew = log
        .iter()
        .filter(|event| matches!(event, Event::Draw { player: P0, .. }))
        .count();
    assert!(
        drew >= 1,
        "P0 draws a card on its next turn instead of the expired replacement ({drew})"
    );
    assert_eq!(
        bears_now, 0,
        "the unspent replacement was cleared in cleanup and created no token"
    );
}

#[test]
fn the_replacement_only_applies_to_its_controllers_draws() {
    // P1 owns and activates the enchantment during P0's turn. P0's draw step in
    // that same turn must be an ordinary draw; the armed replacement belongs to
    // P1's draws only, and it ends with P0's turn (CR 514.2).
    let mut game = fixture(3535)
        .battlefield(P1, &[words()])
        .library(P0, &[bears(), bears(), bears(), bears()])
        .library(P1, &[bears(), bears(), bears(), bears()])
        .mana(P1, one_mana())
        .main(P0, Step::Upkeep)
        .build();
    game.state.turn.number = 3;
    let source = battlefield_card(&game, P1, words());
    activate_words(&mut game, P1, source);

    let mut log = Vec::new();
    assert!(
        run_until(
            &mut game,
            &mut log,
            |g| g.state.turn.step != Step::Upkeep,
            64
        ),
        "the turn reaches the draw step"
    );
    assert_eq!(
        log.iter()
            .filter(|event| matches!(event, Event::Draw { player: P0, .. }))
            .count(),
        1,
        "P0 draws a card normally"
    );
    assert_eq!(bear_count(&game, P0), 0, "P0 gets no token");
    assert_eq!(bear_count(&game, P1), 0, "P1 has no token from P0's draw");
}

#[test]
fn the_ability_is_not_offered_without_its_cost() {
    let mut game = fixture(3535)
        .battlefield(P0, &[words()])
        .library(P0, &[bears(); 4])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&game, P0, words());
    set_priority(&mut game.state, Some(P0));
    assert!(
        !game.legal_actions().iter().any(
            |entry| matches!(entry.action, Action::ActivateAbility { source: s, .. } if s == source)
        ),
        "no mana means no offer"
    );
}
