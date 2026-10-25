//! Focused issue20 native-session coverage:
//! - undo rebuilds the exact random state and decision serial from the recorded
//!   prefix, and drops everything the undone action revealed;
//! - undo is refused once a sandbox override has tainted the recorded line;
//! - the sandbox taint is typed, survives save/load, and every result surface
//!   says the game is not a legal match result.

use std::path::PathBuf;

use aperture_core::{Action, AiSettings, CardId, DeckList, Outcome, P0, Step};
use aperture_ui::session::{BUNDLED_A, BUNDLED_B, SandboxOp, UiSession};

fn deck_a() -> DeckList {
    DeckList::parse_practice("Temur Beats", BUNDLED_A).expect("deck A")
}

fn deck_b() -> DeckList {
    DeckList::parse_practice("Izzet Control", BUNDLED_B).expect("deck B")
}

fn session(seed: u64) -> UiSession {
    UiSession::new(
        seed,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
        false,
        false,
    )
    .expect("session")
}

fn mountain_session() -> (UiSession, u64) {
    let deck = DeckList::new("All Mountains", vec![(CardId::Mountain, 40)]);
    let decks = vec![deck.clone(), deck];
    let ai = AiSettings {
        enabled: true,
        autoplay: false,
        human_seat: P0,
        ai_seats: vec![1],
        ..AiSettings::default()
    };
    for seed in 0..5_000u64 {
        let mut ui =
            UiSession::new(seed, decks.clone(), ai.clone(), false, false).expect("session");
        while ui.game.state.pending.is_some() {
            ui.submit(Action::Mulligan { keep: true }).expect("keep");
        }
        let mut guard = 0;
        while !(ui.game.state.turn.active == P0
            && ui.game.state.turn.step == Step::PrecombatMain
            && ui.game.state.turn.priority == Some(P0))
        {
            ui.submit(Action::Pass).expect("pass");
            guard += 1;
            assert!(guard < 64);
        }
        if ui
            .game
            .state
            .player(P0)
            .hand
            .iter()
            .any(|id| ui.game.state.object(*id).card == CardId::Mountain)
        {
            return (ui, seed);
        }
    }
    panic!("no seed produced a Mountain in P0's opening hand");
}

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "aperture-issue20-{name}-{}.json",
        std::process::id()
    ));
    path
}

#[test]
fn undo_restores_the_exact_random_state_and_forgets_the_undone_play() {
    let (mut ui, _) = mountain_session();
    let land = *ui
        .game
        .state
        .player(P0)
        .hand
        .iter()
        .find(|id| ui.game.state.object(**id).card == CardId::Mountain)
        .expect("Mountain in hand");
    let digest_before = ui.game.digest();
    let decision_before = ui.game.decision();
    let rng_before = ui.game.state.rng.raw_state();
    let log_before = ui.log.len();
    let steps_before = ui.replay.steps.len();
    assert!(ui.can_undo(), "a determined position is undoable");

    ui.submit(Action::PlayLand { card: land })
        .expect("play land");
    let digest_played = ui.game.digest();
    assert_ne!(digest_played, digest_before);
    assert!(
        ui.log
            .iter()
            .any(|event| matches!(event, aperture_core::Event::LandPlayed { .. })),
        "the play is in the log"
    );

    let label = ui.undo_last_action().expect("undo applies");
    assert!(label.contains("P0"), "{label}");
    assert_eq!(
        ui.game.digest(),
        digest_before,
        "the position is restored exactly"
    );
    assert_eq!(
        ui.game.decision(),
        decision_before,
        "the decision serial is restored, not guessed"
    );
    assert_eq!(
        ui.game.state.rng.raw_state(),
        rng_before,
        "the random state is restored exactly"
    );
    assert_eq!(
        ui.log.len(),
        log_before,
        "the log is rewound with the state"
    );
    assert!(
        !ui.log
            .iter()
            .any(|event| matches!(event, aperture_core::Event::LandPlayed { .. })),
        "nothing the undone play revealed is still readable"
    );
    assert_eq!(ui.replay.steps.len(), steps_before);

    // The same action from the restored position continues identically.
    let land_again = *ui
        .game
        .state
        .player(P0)
        .hand
        .iter()
        .find(|id| ui.game.state.object(**id).card == CardId::Mountain)
        .expect("Mountain is back in hand");
    ui.submit(Action::PlayLand { card: land_again })
        .expect("the same play is legal again");
    assert_eq!(
        ui.game.digest(),
        digest_played,
        "the same seed and decision serial produce the same continuation"
    );
}

#[test]
fn undo_is_refused_once_a_sandbox_override_raised_the_taint() {
    let (mut ui, _) = mountain_session();
    ui.sandbox(SandboxOp::Draw(1)).expect("draw override");
    assert!(ui.tainted());
    let digest = ui.game.digest();
    assert!(!ui.can_undo(), "undo is not offered on a tainted line");
    let err = ui.undo_last_action().expect_err("undo is refused");
    assert!(err.contains("sandbox override"), "{err}");
    assert_eq!(
        ui.game.digest(),
        digest,
        "the refusal changed nothing on the table"
    );
}

#[test]
fn the_taint_record_survives_save_load_and_every_result_is_labelled() {
    let (mut ui, _) = mountain_session();
    ui.sandbox(SandboxOp::Draw(1)).expect("draw override");
    ui.sandbox(SandboxOp::SetLife(P0, 33))
        .expect("life override");
    assert!(ui.sandbox_record.len() == 2);
    assert!(
        ui.record_line().contains("NOT a legal match result"),
        "{}",
        ui.record_line()
    );

    let path = temp_path("taint");
    ui.save(&path).expect("a tainted save is written");
    let mut loaded = session(9_001);
    loaded.load(&path).expect("a tainted save loads");
    assert!(loaded.tainted(), "the taint survives the reload");
    assert_eq!(loaded.sandbox_record.len(), 2);
    assert_eq!(loaded.game.state.player(P0).life, 33);

    // A finished sandbox game can never be read as a legal result.
    loaded.game.state.outcome = Some(Outcome::Win {
        winner: P0,
        reason: "scripted finish".to_string(),
    });
    let conclusion = loaded.conclude().expect("an outcome exists");
    assert!(
        conclusion.contains("SANDBOX-MODIFIED") && conclusion.contains("not a legal match result"),
        "{conclusion}"
    );

    // A fresh game clears the record with the position.
    loaded.new_game(9_002);
    assert!(!loaded.tainted());
    assert!(loaded.sandbox_record.is_empty());
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_pending_choice_and_an_empty_library_are_reported_not_silently_ignored() {
    let mut ui = session(31);
    // Still in the opening mulligan: an override is refused with a reason.
    let err = ui.sandbox(SandboxOp::Draw(1)).expect_err("pending choice");
    assert!(err.contains("pending choice"), "{err}");
    assert!(ui.message.contains("sandbox refused"), "{}", ui.message);
    assert!(
        !ui.tainted(),
        "a refused control does not taint the session"
    );

    while ui.game.state.pending.is_some() {
        ui.submit(Action::Mulligan { keep: true }).expect("keep");
    }
    let library = ui.game.state.player(P0).library.len() as u8;
    let err = ui
        .sandbox(SandboxOp::Mill(library + 1))
        .expect_err("oversized mill");
    assert!(err.contains("library"), "{err}");
    assert_eq!(
        ui.game.state.player(P0).library.len(),
        library as usize,
        "a refusal leaves the library untouched"
    );
}
