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

#[test]
fn re_saving_a_legacy_tainted_save_never_clears_the_flag() {
    // An older save can carry the boolean taint without a typed record. Loading
    // and re-saving it must not launder the game into a legal match result.
    let (mut ui, _) = mountain_session();
    ui.sandbox(SandboxOp::Draw(1)).expect("draw override");
    let path = temp_path("legacy-taint");
    ui.save(&path).expect("save");
    let text = std::fs::read_to_string(&path).expect("read");
    let mut value: serde_json::Value = serde_json::from_str(&text).expect("json");
    value["sandbox"] = serde_json::json!({ "edits": [] });
    std::fs::write(&path, serde_json::to_string(&value).expect("json")).expect("write legacy");

    let mut loaded = session(9_003);
    loaded.load(&path).expect("a legacy tainted save loads");
    assert!(loaded.tainted(), "the legacy flag is still a taint");
    assert!(!loaded.can_undo(), "a tainted line offers no undo");

    let again = temp_path("legacy-taint-resave");
    loaded.save(&again).expect("re-save");
    let save = aperture_core::SessionSave::from_json(
        &std::fs::read_to_string(&again).expect("read the re-save"),
    )
    .expect("the re-save loads");
    assert!(
        save.sandbox_modified,
        "re-saving must not clear a taint that came from the boolean flag alone"
    );
    assert!(save.record_line().contains("NOT a legal match result"));
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(again);
}

/// Applying a branch replaces the live position and clears the taint record, so
/// the branch has to prove the replacement really is the clean recorded prefix.
/// A cut that includes a step played after a sandbox override must be refused
/// (the override bumped the revision those steps were recorded against), and a
/// cut before the override must reproduce the pre-override position exactly.
#[test]
fn a_sandbox_taint_cannot_be_branched_into_a_clean_line() {
    let (mut ui, _) = mountain_session();
    let land = *ui
        .game
        .state
        .player(P0)
        .hand
        .iter()
        .find(|id| ui.game.state.object(**id).card == CardId::Mountain)
        .expect("Mountain in hand");
    ui.submit(Action::PlayLand { card: land })
        .expect("a recorded legal action before the override");
    let clean_steps = ui.replay.steps.len();
    let clean_digest = ui.game.digest();
    let clean_events = ui.log.len();
    assert!(!ui.tainted(), "the recorded line is clean so far");

    ui.sandbox(SandboxOp::Draw(1)).expect("draw override");
    assert!(ui.tainted());
    ui.submit(Action::Pass)
        .expect("a recorded legal action after the override");
    let tainted_steps = ui.replay.steps.len();
    assert_eq!(
        tainted_steps,
        clean_steps + 1,
        "the pass is one recorded step on top of the clean prefix"
    );
    let tainted_digest = ui.game.digest();
    let tainted_events = ui.log.len();

    let path = temp_path("tainted-branch");
    ui.save(&path).expect("a tainted save is written");

    // 1. A cut that reaches into the post-override tail is refused, and the
    //    refusal leaves the tainted session exactly as it was.
    ui.request_replay_branch(path.clone(), tainted_steps);
    let mut applied = Vec::new();
    let mut guard = 0;
    while ui.io_busy && guard < 2_000 {
        applied.extend(ui.poll_workers());
        std::thread::sleep(std::time::Duration::from_millis(2));
        guard += 1;
    }
    assert!(!ui.io_busy, "the branch worker never finished");
    applied.extend(ui.poll_workers());
    assert!(
        applied
            .iter()
            .any(|line| line.starts_with("branch failed") && line.contains("stale submission")),
        "a post-override cut was not refused by the revision guard: {applied:?}"
    );
    assert!(
        ui.tainted(),
        "a refused branch must not launder the sandbox taint"
    );
    assert_eq!(ui.sandbox_record.len(), 1, "the taint record is untouched");
    assert_eq!(
        ui.game.digest(),
        tainted_digest,
        "a refused branch leaves the live position alone"
    );
    assert_eq!(ui.log.len(), tainted_events);
    assert_eq!(ui.replay.steps.len(), tainted_steps);

    // 2. A cut before the override replays the clean recorded prefix exactly.
    //    That equality is what makes clearing the taint on the branch honest.
    ui.request_replay_branch(path.clone(), clean_steps);
    let mut applied = Vec::new();
    let mut guard = 0;
    while ui.io_busy && guard < 2_000 {
        applied.extend(ui.poll_workers());
        std::thread::sleep(std::time::Duration::from_millis(2));
        guard += 1;
    }
    assert!(!ui.io_busy, "the branch worker never finished");
    applied.extend(ui.poll_workers());
    assert!(
        applied.iter().any(|line| line.starts_with("branched from")),
        "the clean pre-override cut was refused: {applied:?}"
    );
    assert!(
        !ui.tainted(),
        "the branch is the clean recorded prefix, so the taint is gone with the position"
    );
    assert!(ui.sandbox_record.is_empty());
    assert_eq!(
        ui.game.digest(),
        clean_digest,
        "the branch reproduces the pre-override position exactly, not an approximation"
    );
    assert_eq!(
        ui.log.len(),
        clean_events,
        "nothing the override logged survives the branch"
    );
    assert!(
        !ui.log
            .iter()
            .any(|event| matches!(event, aperture_core::Event::SandboxOverride { .. })),
        "an override event survived into the clean line"
    );
    assert_eq!(ui.replay.steps.len(), clean_steps);
    let _ = std::fs::remove_file(path);
}
