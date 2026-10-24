//! Issue 13 player-loss slice: inherited D-1 and Commander-combat loss.
//!
//! D-1 (inherited from review 160): an ordinary empty-library draw step emitted
//! `Priority` before the loss/cleanup state-based action and GameOver events.
//! CR 117.5 requires state-based actions to finish before a player receives
//! priority; a terminal state must announce exactly one GameOver and no false
//! priority holder afterwards.

mod common;

use aperture_core::*;
use common::*;

fn assert_loss_events(events: &[Event], loser: PlayerId, winner: PlayerId) {
    let step_index = events
        .iter()
        .position(|e| matches!(e, Event::StepBegin { step: Step::Draw }))
        .expect("draw step began");
    let loss_index = events
        .iter()
        .position(|e| matches!(e, Event::StateBasedAction { description } if description.contains(&format!("P{loser} loses"))))
        .expect("loss SBA emitted");
    assert!(
        loss_index > step_index,
        "loss must follow the draw step: {events:?}"
    );
    let false_priority: Vec<_> = events[step_index..loss_index]
        .iter()
        .filter(|e| matches!(e, Event::Priority { .. }))
        .collect();
    assert!(
        false_priority.is_empty(),
        "no Priority may be granted before the empty-draw loss/cleanup: {false_priority:?} in {events:?}"
    );
    let game_over: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, Event::GameOver { winner: w, .. } if *w == winner))
        .collect();
    assert_eq!(
        game_over.len(),
        1,
        "exactly one GameOver for P{winner}: {events:?}"
    );
}

#[test]
fn d1_ordinary_empty_library_draw_loses_before_priority() {
    let mut game = fixture(1301).step_no_priority(P0, Step::Upkeep).build();
    // Turn 1 of the starting player skips its draw in a two-seat game
    // (CR 103.8a); use turn 2 so the empty draw is the event under test.
    game.state.turn.number = 2;
    set_priority(&mut game.state, Some(P0));

    let mut log = submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.state.turn.priority, Some(P1), "P1 receives priority");
    log.extend(submit_ok(&mut game, P1, Action::Pass));

    assert!(game.state.is_over(), "empty draw ended the game: {log:?}");
    assert_eq!(game.state.turn.priority, None);
    assert_eq!(game.state.live_count(), 1);
    assert_loss_events(&log, P0, P1);
}

/// Build a command-zone practice position whose library is empty and whose
/// commander is idle in the command zone: the same D-1 draw/loss path, but with
/// command-zone state present (the inherited defect covered ordinary and idle
/// command games).
fn idle_command_empty_library() -> Game {
    let deck = DeckList::parse_practice("empty-command", "5 Plains\n").unwrap();
    let decks = vec![deck.clone(), deck];
    let specs = [
        (0, CardId::IsamaruHoundOfKonda),
        (1, CardId::IsamaruHoundOfKonda),
    ];
    let (mut game, _) = Game::new_command_zone_logged(1302, &decks, &specs).unwrap();
    game.state.pending = None;
    game.state.mulligan = None;
    for seat in 0..2 {
        let ids = std::mem::take(&mut game.state.players[seat].library);
        for id in ids {
            let object = game.state.objects.get_mut(&id).unwrap();
            object.zone = Zone::Graveyard;
            game.state.players[seat].graveyard.push(id);
        }
        // The tiny constructed library triggered empty draws during setup;
        // clear the flag so the test premise is exactly "empty at draw step".
        game.state.players[seat].drew_from_empty = false;
    }
    game.state.turn.number = 2;
    game.state.turn.active = P0;
    game.state.turn.step = Step::Upkeep;
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
    game
}

#[test]
fn d1_idle_command_empty_library_draw_loses_before_priority() {
    let mut game = idle_command_empty_library();
    assert_eq!(
        game.state.commanders.len(),
        2,
        "both commanders are idle in the command zone"
    );
    let mut log = submit_ok(&mut game, P0, Action::Pass);
    assert_eq!(game.state.turn.priority, Some(P1));
    log.extend(submit_ok(&mut game, P1, Action::Pass));
    assert!(game.state.is_over(), "{log:?}");
    assert_eq!(game.state.turn.priority, None);
    assert_loss_events(&log, P0, P1);
}
