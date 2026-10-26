//! #34 D1 — a cleanup discard (CR 514.1) must never stall an all-AI table.
//!
//! The packaged defect: with both seats on AI autoplay the session dropped the
//! AI's discard answer as `illegal_choice` (the offer contained only one
//! hard-coded sample) and then re-asked forever, so the game stopped being a
//! game at the cleanup step. These tests use the real session path — request on
//! a worker thread, `poll_workers` on the main thread — with no GUI.

use std::time::{Duration, Instant};

use aperture_ai::{Decision, HeuristicPolicy, Policy};
use aperture_core::{Action, AiSettings, DeckList, ObjectId, Pending, PlayerId};
use aperture_ui::session::{BUNDLED_A, BUNDLED_B, UiSession, WorkerMessage};

fn decks() -> Vec<DeckList> {
    vec![
        DeckList::parse_practice("Temur Beats", BUNDLED_A).expect("deck A"),
        DeckList::parse_practice("Izzet Control", BUNDLED_B).expect("deck B"),
    ]
}

/// The packaged configuration: both seats driven by the AI, no human seat.
fn all_ai(seed: u64) -> UiSession {
    let ai = AiSettings {
        enabled: true,
        autoplay: true,
        ai_seats: vec![0, 1],
        ..AiSettings::default()
    };
    UiSession::new(seed, decks(), ai, false, false).expect("two-seat session")
}

/// One UI frame: request the AI answer for the current owner, then drain the
/// worker channel until the answer has been consumed. Returns the applied log
/// lines, exactly like `app::update` does.
fn frame(ui: &mut UiSession) -> Vec<String> {
    if let Some(seat) = ui.owner()
        && ui.seat_is_ai(seat)
        && !ui.ai_busy
    {
        assert!(
            ui.request_ai_decision_default(),
            "the session refused to ask the AI for P{seat}"
        );
    }
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let applied = ui.poll_workers();
        if !ui.ai_busy {
            return applied;
        }
        assert!(
            Instant::now() < deadline,
            "the AI worker never answered within 60 s"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn pending_discard(ui: &UiSession) -> Option<(PlayerId, u8)> {
    match ui.game.state.pending {
        Some(Pending::Discard { player, count }) => Some((player, count)),
        _ => None,
    }
}

/// Answer every decision with the baseline policy on the main thread. This is
/// the mixed table a human plays against: the same actions a human can submit.
fn drive(ui: &mut UiSession, pred: impl Fn(&UiSession) -> bool, cap: usize) -> bool {
    for _ in 0..cap {
        if pred(ui) {
            return true;
        }
        if ui.game.state.is_over() {
            return false;
        }
        let Some(seat) = ui.owner() else {
            return false;
        };
        let decision = HeuristicPolicy::new().decide(&ui.view(seat));
        if ui.submit(decision.action).is_err() {
            return false;
        }
    }
    false
}

/// The human seat picks the cards: any legal set works, not only the engine's
/// default sample, and only the entitled viewer can submit it.
#[test]
fn the_human_seat_can_discard_the_cards_they_choose() {
    let human_ai = || AiSettings {
        enabled: true,
        autoplay: true,
        ai_seats: vec![1],
        ..AiSettings::default()
    };
    let mut found = None;
    for seed in 34..=72u64 {
        let mut ui = UiSession::new(seed, decks(), human_ai(), false, false).expect("session");
        let reached = drive(
            &mut ui,
            |u| {
                matches!(
                    u.game.state.pending,
                    Some(Pending::Discard { player: 0, .. })
                )
            },
            900,
        );
        if reached {
            found = Some(ui);
            break;
        }
    }
    let mut ui = found.expect("no probed seed gave the human seat a cleanup discard");
    let count = match ui.game.state.pending {
        Some(Pending::Discard { count, .. }) => count as usize,
        other => panic!("expected a cleanup discard, got {other:?}"),
    };
    let hand = ui.view(0).own_hand.clone();
    // Deliberately the *first* cards, where the engine's historical sample was
    // the last cards of the hand: the human is not limited to that sample.
    let chosen: Vec<ObjectId> = hand.iter().take(count).map(|card| card.id).collect();
    let mut expected: Vec<&'static str> = hand.iter().take(count).map(|card| card.name).collect();
    expected.sort_unstable();
    // Only the entitled viewer may answer: the same submission is refused when
    // the session's human seat is the other seat.
    ui.ai.human_seat = 1;
    let refused = ui.submit_human(Action::Discard {
        cards: chosen.clone(),
    });
    assert!(
        refused.is_err(),
        "a seat that does not own the choice must not be able to answer it"
    );
    assert_eq!(
        ui.game.state.pending,
        Some(Pending::Discard {
            player: 0,
            count: count as u8
        }),
        "a refused answer must leave the choice pending"
    );
    ui.ai.human_seat = 0;
    ui.submit_human(Action::Discard { cards: chosen })
        .expect("the entitled human seat may discard the cards it chose");
    assert!(
        ui.game.state.pending.is_none(),
        "the chosen discard must complete the cleanup step"
    );
    let mut discarded: Vec<&'static str> = ui
        .log
        .iter()
        .filter_map(|event| match event {
            aperture_core::Event::Discarded { player: 0, card } => Some(card.name()),
            _ => None,
        })
        .collect();
    discarded.sort_unstable();
    assert_eq!(
        discarded, expected,
        "the human's own choice must be what left the hand"
    );
}

/// The defect itself: at a cleanup discard the AI's answer must be applied.
#[test]
fn the_cleanup_discard_answer_is_applied_instead_of_dropped() {
    // The first cleanup discard of a practice game needs a hand over seven, so
    // probe the bundled decks until one is reached; any stall before that is a
    // failure of the same class, not a reason to skip.
    let mut found = None;
    for seed in 34..=72u64 {
        let mut ui = all_ai(seed);
        let mut guard = 0;
        while pending_discard(&ui).is_none() && ui.game.state.outcome.is_none() {
            guard += 1;
            assert!(
                guard < 800,
                "seed {seed} stalled before a cleanup discard (turn {}, pending {:?}, busy_note {:?})",
                ui.game.state.turn.number,
                ui.game.state.pending,
                ui.busy_note
            );
            frame(&mut ui);
        }
        if pending_discard(&ui).is_some() {
            found = Some(ui);
            break;
        }
    }
    let mut ui = found.expect("no probed seed reached a cleanup discard");
    let (seat, count) = pending_discard(&ui).expect("a pending cleanup discard");
    assert_eq!(
        ui.owner(),
        Some(seat),
        "the seat that must discard owns the choice"
    );
    let decision_before = ui.game.decision();
    let view = ui.view(seat);

    // The AI's own answer has to be one of the actions the engine actually
    // offers at this decision point — that is what makes it legal to apply.
    let wanted = HeuristicPolicy::new().decide(&view);
    let offered = view
        .legal
        .iter()
        .filter(|legal| matches!(legal.action, Action::Discard { .. }))
        .count();
    assert!(
        view.legal.iter().any(|legal| legal.action == wanted.action),
        "the AI's discard answer is not among the {offered} offered action(s): {:?}",
        wanted.action
    );
    assert!(
        offered >= 1 && (count > 1 || offered == view.own_hand.len()),
        "a count-{count} cleanup discard over {} cards offered {offered} action(s)",
        view.own_hand.len()
    );

    let applied = frame(&mut ui);
    assert!(
        !applied.is_empty(),
        "the AI's discard answer was dropped: busy_note={:?}, still pending {:?}",
        ui.busy_note,
        pending_discard(&ui)
    );
    assert!(
        ui.busy_note
            .as_deref()
            .unwrap_or("")
            .find("illegal_choice")
            .is_none(),
        "the live position refused its own offer: {:?}",
        ui.busy_note
    );
    assert_ne!(
        ui.game.decision(),
        decision_before,
        "the decision point did not move past the cleanup discard"
    );
    assert!(
        pending_discard(&ui).is_none(),
        "the table is still waiting on the cleanup discard"
    );
    let discarded = ui
        .log
        .iter()
        .any(|event| matches!(event, aperture_core::Event::Discarded { .. }));
    assert!(discarded, "no discard event was recorded");
}

/// The release criterion the deadlock blocked: a whole game, both seats on the
/// AI, reaching a terminal result with a replay that reproduces the digest.
#[test]
fn both_seats_on_ai_autoplay_reach_a_terminal_result_with_a_verified_replay() {
    let mut ui = all_ai(30);
    let mut guard = 0;
    while ui.game.state.outcome.is_none() {
        guard += 1;
        assert!(
            guard < 8_000,
            "the table never finished: turn {} pending {:?} busy_note {:?}",
            ui.game.state.turn.number,
            ui.game.state.pending,
            ui.busy_note
        );
        let applied = frame(&mut ui);
        assert!(
            !ui.busy_note
                .as_deref()
                .unwrap_or("")
                .contains("illegal_choice"),
            "step {guard} dropped the AI's own legal answer: {:?} ({applied:?})",
            ui.busy_note
        );
    }
    assert!(
        !ui.replay.steps.is_empty(),
        "a finished game must have recorded a replay"
    );
    let digest = ui.game.digest().to_string();
    let (replayed, _log) = ui
        .replay
        .run()
        .expect("the recorded replay must re-run cleanly");
    assert_eq!(
        replayed.digest(),
        digest,
        "the replay does not reproduce the live final position"
    );
    assert_eq!(
        replayed.state.outcome, ui.game.state.outcome,
        "the replay does not reproduce the same result"
    );
}

/// A stale/foreign answer must still be refused — the fallback may not become a
/// way to apply another seat's decision. The worker message is queued directly
/// so the check is deterministic (no thread race).
#[test]
fn an_answer_bound_to_the_wrong_seat_is_still_refused() {
    let mut ui = all_ai(52);
    let owner = ui.owner().expect("a pending decision");
    let other = 1 - owner;
    let digest = ui.game.digest().to_string();
    let decision_serial = ui.game.decision();
    let revision = ui.game.revision();
    ui.worker_tx
        .send(WorkerMessage::Ai {
            generation: ui.generation,
            revision,
            decision_serial,
            seat: other,
            decision: Decision::heuristic(Action::Mulligan { keep: true }),
        })
        .expect("queue the misaddressed answer");
    let applied = ui.poll_workers();
    assert!(
        applied.is_empty(),
        "a misaddressed answer was applied: {applied:?}"
    );
    assert!(
        ui.busy_note.as_deref().unwrap_or("").contains("wrong_seat"),
        "expected a wrong_seat refusal, got {:?}",
        ui.busy_note
    );
    assert_eq!(
        ui.game.digest(),
        digest,
        "a refusal must not change the state"
    );
    assert_eq!(
        ui.game.decision(),
        decision_serial,
        "a refusal must not move the choice"
    );
}
