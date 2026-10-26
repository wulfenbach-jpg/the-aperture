//! Supervisor acceptance fixes 3 and 4: two-seat import vertical path, no
//! silent deck substitution, and background workers that never apply a stale
//! result to a replaced game.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use aperture_ai::{DeciderConfig, Decision};
use aperture_core::state::Step;
use aperture_core::{Action, AiSettings, DeckList, P0, P1, Pending};
use aperture_ui::session::{
    BUNDLED_A, BUNDLED_B, UiSession, import_seat_file, import_seat_text, load_explicit_deck,
};

fn session(seed: u64) -> UiSession {
    let deck_a = DeckList::parse_practice("Temur Beats", BUNDLED_A).expect("deck A");
    let deck_b = DeckList::parse_practice("Izzet Control", BUNDLED_B).expect("deck B");
    UiSession::new(
        seed,
        vec![deck_a, deck_b],
        AiSettings::default(),
        false,
        false,
    )
    .expect("session")
}

fn temp_file(name: &str, contents: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("aperture-import-{name}-{}.txt", std::process::id()));
    std::fs::write(&path, contents).expect("write temp deck");
    path
}

#[test]
fn two_distinct_imported_lists_enter_the_game_and_the_replay() {
    let mut ui = session(31);
    // Seat P0 imports one list, seat P1 a different one, through the same
    // per-seat entry point the UI uses.
    let a = import_seat_text(P0, "seat0 list", BUNDLED_A).expect("P0 imports");
    let b = import_seat_text(P1, "seat1 list", BUNDLED_B).expect("P1 imports");
    assert!(a.certified() && b.certified());
    assert_ne!(a.name, b.name, "the two seats must be distinct lists");
    ui.seat_import[0] = Some(a);
    ui.seat_import[1] = Some(b);
    assert!(ui.can_start_imported());

    let names = ui.start_imported_match(77).expect("both seats certified");
    assert_eq!(ui.seats[0].name, names[0]);
    assert_eq!(ui.seats[1].name, names[1]);
    // The exact planned lists are embedded as replay snapshots.
    assert_eq!(ui.replay.seats[0], ui.seats[0]);
    assert_eq!(ui.replay.seats[1], ui.seats[1]);
    assert_eq!(ui.replay.seed, 77);
    // And they are the lists actually in play (7-card opening hand dealt).
    assert_eq!(ui.game.state.players[0].deck_name, names[0]);
    assert_eq!(ui.game.state.players[1].deck_name, names[1]);
    assert_eq!(ui.game.state.players[0].library.len(), 33);
    assert_eq!(ui.game.state.players[1].library.len(), 33);
    assert_eq!(ui.game.revision(), 0);
    assert!(ui.message.contains("no substitutions"), "{}", ui.message);
}

#[test]
fn an_unsupported_import_cannot_start_an_imported_match() {
    let mut ui = session(32);
    ui.seat_import[0] = Some(import_seat_text(P0, "ok", BUNDLED_A).expect("P0 imports"));
    let bad = import_seat_text(P1, "unsupported", "40 Dark Ritual\n").expect("the list parses");
    assert!(!bad.certified(), "an out-of-pool list is not certified");
    assert!(bad.refusal.is_some());
    ui.seat_import[1] = Some(bad);

    assert!(!ui.can_start_imported(), "unsupported input must not start");
    let before = ui.game.digest();
    assert!(ui.start_imported_match(78).is_err());
    assert_eq!(ui.game.digest(), before, "the refusal changed nothing");
}

#[test]
fn text_fallback_certifies_both_seats_and_rejects_bad_input() {
    let a = import_seat_text(P0, "a", BUNDLED_A).expect("P0 text");
    let b = import_seat_text(P1, "b", BUNDLED_B).expect("P1 text");
    assert!(a.certified() && b.certified());

    // A malformed line is refused with a real reason, not silently dropped.
    let bad = import_seat_text(P0, "bad", "not-a-quantity Grizzly Bears\n");
    assert!(bad.is_err());
}

#[test]
fn a_missing_or_invalid_explicit_deck_file_is_refused_not_substituted() {
    let missing = Path::new("definitely-not-here-9999.txt");
    let (list, err) = load_explicit_deck(missing, "Explicit");
    assert!(list.cards.is_empty(), "no bundled list was substituted");
    let err = err.expect("missing file is an error");
    assert!(err.contains("Refusing to substitute"), "{err}");

    let path = temp_file("invalid", "1 NotARealCard\n");
    let (list, err) = load_explicit_deck(&path, "Explicit");
    assert!(list.cards.is_empty());
    assert!(
        err.expect("invalid content is an error")
            .contains("Refusing to substitute")
    );
    let _ = std::fs::remove_file(&path);

    // A valid explicit file is used verbatim.
    let valid = temp_file("valid", BUNDLED_A);
    let (list, err) = load_explicit_deck(&valid, "Explicit");
    assert!(err.is_none());
    assert_eq!(list.size(), 40);
    let _ = std::fs::remove_file(&valid);
}

#[test]
fn import_worker_result_is_applied_through_the_poll() {
    let mut ui = session(33);
    let text = BUNDLED_B.to_string();
    assert!(ui.request_import(P1, move || { import_seat_text(P1, "worker list", &text) }));
    let deadline = Instant::now() + Duration::from_secs(10);
    while ui.import_busy[1] && Instant::now() < deadline {
        ui.poll_workers();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!ui.import_busy[1], "the worker finished");
    assert!(
        ui.seat_import[1]
            .as_ref()
            .expect("result stored")
            .certified()
    );
    assert!(ui.seat_error[1].is_none());
}

#[test]
fn file_worker_reports_a_missing_path_instead_of_guessing() {
    let mut ui = session(34);
    assert!(ui.request_import(P0, || {
        import_seat_file(P0, Path::new("no-such-deck-file.txt"))
    }));
    let deadline = Instant::now() + Duration::from_secs(10);
    while ui.import_busy[0] && Instant::now() < deadline {
        ui.poll_workers();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(ui.seat_import[0].is_none());
    assert!(
        ui.seat_error[0]
            .as_deref()
            .unwrap_or("")
            .contains("could not read")
    );
}

#[test]
fn a_stale_worker_completion_never_changes_the_replacement_game() {
    let mut ui = session(35);
    // Give the AI whichever seat owns the opening declaration.
    let opening = ui.owner().expect("opening declarer");
    ui.ai.human_seat = if opening == P0 { P1 } else { P0 };
    ui.ai.ai_seats = vec![opening];
    assert_eq!(ui.owner(), Some(opening));

    let (release_tx, release_rx) = channel::<()>();
    let (running_tx, running_rx) = channel::<()>();
    let started = Arc::new(AtomicBool::new(false));
    let started_flag = started.clone();
    let started_request = ui.request_ai_decision(move |_view| {
        started_flag.store(true, Ordering::SeqCst);
        let _ = running_tx.send(());
        // Block until the test releases this worker.
        let _ = release_rx.recv_timeout(Duration::from_secs(10));
        Decision::heuristic(Action::Mulligan { keep: true })
    });
    assert!(started_request);
    running_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("the worker started");

    // Polling while the worker is blocked must return promptly and apply nothing.
    let poll_start = Instant::now();
    let applied = ui.poll_workers();
    assert!(applied.is_empty(), "nothing is ready yet: {applied:?}");
    assert!(
        poll_start.elapsed() < Duration::from_millis(500),
        "poll_workers blocked the frame for {:?}",
        poll_start.elapsed()
    );

    // Replace the game while the worker is still in flight — same seed, so the
    // digest is identical; only the generation token distinguishes the run.
    ui.new_game(35);
    let replacement_digest = ui.game.digest();
    assert_eq!(replacement_digest, ui.game.digest());

    // Release the stale worker and let its message land.
    release_tx.send(()).expect("release the worker");
    let deadline = Instant::now() + Duration::from_secs(10);
    while ui.busy_note.is_none() && Instant::now() < deadline {
        ui.poll_workers();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        ui.busy_note.as_deref().unwrap_or("").contains("stale"),
        "the stale result must be discarded: {:?}",
        ui.busy_note
    );
    // The replacement game is untouched: same digest, nothing recorded.
    assert_eq!(ui.game.digest(), replacement_digest);
    assert_eq!(ui.game.revision(), 0);
    assert_eq!(
        ui.game.state.pending,
        Some(Pending::Mulligan { player: opening })
    );
    assert!(ui.replay.steps.is_empty(), "no action was recorded");
}

#[test]
fn a_fresh_worker_completion_is_applied() {
    let mut ui = session(36);
    let opening = ui.owner().expect("opening declarer");
    ui.ai.human_seat = if opening == P0 { P1 } else { P0 };
    ui.ai.ai_seats = vec![opening];
    let ran = Arc::new(AtomicBool::new(false));
    let flag = ran.clone();
    assert!(ui.request_ai_decision(move |_view| {
        flag.store(true, Ordering::SeqCst);
        Decision::heuristic(Action::Mulligan { keep: true })
    }));
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut applied = Vec::new();
    while applied.is_empty() && Instant::now() < deadline {
        applied = ui.poll_workers();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(ran.load(Ordering::SeqCst), "the worker ran");
    assert!(!applied.is_empty(), "a fresh completion is applied");
    assert!(ui.game.revision() > 0, "the action reached the engine");
    assert!(!ui.ai_busy);
}

#[test]
fn the_default_async_decider_reports_a_single_legal_option_as_forced_without_http() {
    let mut ui = session(39);
    let opening = ui.owner().expect("opening declarer");
    ui.ai.human_seat = if opening == P0 { P1 } else { P0 };
    ui.ai.ai_seats = vec![opening];

    // Strip the table to a position with exactly one legal action (Pass).
    ui.game.state.pending = None;
    ui.game.state.objects.clear();
    for player in &mut ui.game.state.players {
        player.hand.clear();
        player.library.clear();
        player.graveyard.clear();
        player.exile.clear();
        player.command_zone.clear();
    }
    ui.game.state.stack.clear();
    ui.game.state.turn.active = opening;
    ui.game.state.turn.step = Step::PrecombatMain;
    ui.game.state.turn.priority = Some(opening);
    ui.game.state.turn.passed = 0;
    assert_eq!(ui.owner(), Some(opening));
    assert_eq!(ui.view(opening).legal.len(), 1, "the fixture is forced");

    // Any request would be visible here; a forced choice must not contact it.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
    let port = listener.local_addr().unwrap().port();
    let contacted = Arc::new(AtomicBool::new(false));
    let flag = contacted.clone();
    std::thread::spawn(move || {
        if listener.accept().is_ok() {
            flag.store(true, Ordering::SeqCst);
        }
    });
    ui.ai.use_decider = true;
    ui.decider_config = Some(DeciderConfig {
        base_url: format!("http://127.0.0.1:{port}"),
        timeout_ms: 300,
        ..DeciderConfig::default()
    });
    assert!(ui.request_ai_decision_default());
    let deadline = Instant::now() + Duration::from_secs(5);
    while ui.note.is_none() && Instant::now() < deadline {
        ui.poll_workers();
        std::thread::sleep(Duration::from_millis(5));
    }
    let note = ui.note.clone().unwrap_or_default();
    assert!(
        note.contains("forced"),
        "a single legal option must be labelled Forced, got: {note}"
    );
    assert!(
        !note.contains("too_few_options"),
        "the forced path must not be reported as a prepare fallback: {note}"
    );
    assert!(
        !contacted.load(Ordering::SeqCst),
        "a forced choice must not contact the model endpoint"
    );
    assert!(!ui.ai_busy);
}

#[test]
fn an_ai_completion_outside_the_live_offer_is_refused_and_a_legal_offer_completes_it() {
    let mut ui = session(38);
    let opening = ui.owner().expect("opening declarer");
    ui.ai.human_seat = if opening == P0 { P1 } else { P0 };
    ui.ai.ai_seats = vec![opening];
    let offered = ui
        .view(opening)
        .legal
        .first()
        .cloned()
        .expect("the opening decision offers at least one action");
    // Pass is not a legal answer to the opening mulligan decision, so the
    // completion itself must be refused. Since #34 a refused completion is not a
    // terminal state for a legal position: the engine's own offer is applied.
    assert!(ui.request_ai_decision(|_view| Decision::heuristic(Action::Pass)));
    let deadline = Instant::now() + Duration::from_secs(10);
    while ui.busy_note.is_none() && Instant::now() < deadline {
        ui.poll_workers();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        ui.busy_note
            .as_deref()
            .unwrap_or("")
            .contains("illegal_choice"),
        "the refusal needs a stable diagnostic: {:?}",
        ui.busy_note
    );
    assert!(
        ui.busy_note
            .as_deref()
            .unwrap_or("")
            .contains("applied the engine's legal offer"),
        "the table must not be left waiting on a refused answer: {:?}",
        ui.busy_note
    );
    assert_eq!(
        ui.game.revision(),
        1,
        "exactly one legal completion was applied"
    );
    assert_eq!(ui.replay.steps.len(), 1, "exactly one action was recorded");
    assert_eq!(
        ui.replay.steps[0].action, offered.action,
        "the recorded action must be the engine's offered action"
    );
    assert_ne!(
        ui.replay.steps[0].action,
        Action::Pass,
        "the refused completion must never be recorded as played"
    );
    assert!(!ui.ai_busy);
}

#[test]
fn the_ui_cannot_submit_an_action_for_the_other_seat() {
    let mut ui = session(37);
    // P0 (the human seat) keeps, which hands the decision to the AI seat P1.
    while ui.owner() == Some(P0) {
        ui.submit(Action::Mulligan { keep: true })
            .expect("P0 keeps");
    }
    assert_eq!(ui.owner(), Some(P1));
    let before = ui.game.digest();
    let err = ui.submit_human(Action::Mulligan { keep: true });
    assert!(err.is_err(), "the human must not act for P1");
    assert!(ui.message.contains("not your seat"), "{}", ui.message);
    assert_eq!(
        ui.game.digest(),
        before,
        "the refused submit changed nothing"
    );
    assert!(ui.replay.steps.iter().all(|step| step.player == P0));
}

fn temp_save(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("aperture-save-{name}-{}.json", std::process::id()));
    path
}

#[test]
fn save_and_load_run_on_workers() {
    let mut ui = session(38);
    while ui.owner() == Some(P0) {
        ui.submit(Action::Mulligan { keep: true })
            .expect("P0 keeps");
    }
    let digest = ui.game.digest();
    let path = temp_save("worker-roundtrip");
    let _ = std::fs::remove_file(&path);
    ui.request_save(path.clone());
    let deadline = Instant::now() + Duration::from_secs(10);
    while ui.io_busy && Instant::now() < deadline {
        ui.poll_workers();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!ui.io_busy, "the save worker finished");
    assert!(path.exists(), "the save file was written");
    assert!(ui.message.contains("saved to"), "{}", ui.message);

    // Move elsewhere, then load the saved position back through a worker.
    ui.new_game(999);
    assert_ne!(ui.game.digest(), digest);
    ui.request_load(path.clone());
    let deadline = Instant::now() + Duration::from_secs(10);
    while ui.io_busy && Instant::now() < deadline {
        ui.poll_workers();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(ui.game.digest(), digest, "the load worker applied the save");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_load_completed_after_the_game_changed_is_discarded() {
    let mut ui = session(39);
    let path = temp_save("stale-load");
    ui.request_save(path.clone());
    let deadline = Instant::now() + Duration::from_secs(10);
    while ui.io_busy && Instant::now() < deadline {
        ui.poll_workers();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(path.exists());

    // Request a load, then replace the game before it is applied.
    ui.request_load(path.clone());
    ui.new_game(41);
    let replacement = ui.game.digest();
    let deadline = Instant::now() + Duration::from_secs(10);
    while ui.busy_note.is_none() && Instant::now() < deadline {
        ui.poll_workers();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        ui.busy_note.as_deref().unwrap_or("").contains("stale load"),
        "{:?}",
        ui.busy_note
    );
    assert_eq!(
        ui.game.digest(),
        replacement,
        "the stale load changed nothing"
    );
    let _ = std::fs::remove_file(&path);
}
