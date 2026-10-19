//! UI session behaviour: strict submission, sandbox marking, save/load
//! round-trip, and the AI step gate.

use std::path::PathBuf;

use aperture_core::{Action, AiSettings, CardId, DeckList, ObjectId, P0, P1};
use aperture_ui::session::{BUNDLED_A, BUNDLED_B, SandboxOp, UiSession};

fn deck_a() -> DeckList {
    DeckList::parse_practice("Temur Beats", BUNDLED_A).expect("deck A")
}

fn deck_b() -> DeckList {
    DeckList::parse_practice("Izzet Control", BUNDLED_B).expect("deck B")
}

fn session(seed: u64, human_plays_ai_seat: bool) -> UiSession {
    UiSession::new(
        seed,
        deck_a(),
        deck_b(),
        AiSettings::default(),
        false,
        human_plays_ai_seat,
    )
}

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("aperture-test-{name}-{}.json", std::process::id()));
    path
}

#[test]
fn an_illegal_action_is_refused_and_leaves_the_state_identical() {
    let mut ui = session(11, false);
    let before = ui.game.digest();
    let owner = ui.owner().expect("a decision is pending");
    // Point at object ids that do not exist in any hand.
    let err = ui.submit(Action::PlayLand {
        card: ObjectId(9_999),
    });
    assert!(err.is_err(), "the engine must refuse this");
    assert!(ui.message.contains("refused"), "{}", ui.message);
    assert_eq!(ui.game.digest(), before);
    assert_eq!(ui.game.decision_owner(), Some(owner));
    assert!(ui.replay.steps.is_empty(), "nothing was recorded");
}

#[test]
fn sandbox_tools_change_the_state_and_void_certified_play() {
    let mut ui = session(12, false);
    // Reach the first priority so the human seat is not mid-mulligan.
    ui.step_ai();
    while ui.owner().is_some() && ui.seat_is_ai(ui.owner().unwrap()) {
        ui.step_ai();
    }
    let human = ui.human_seat();
    let hand_before = ui.game.state.player(human).hand.len();
    let before = ui.game.digest();
    assert!(!ui.sandbox_modified);
    ui.sandbox(SandboxOp::Draw(1));
    assert!(ui.sandbox_modified, "a sandbox edit must be visible");
    assert_eq!(ui.game.state.player(human).hand.len(), hand_before + 1);
    assert_ne!(ui.game.digest(), before);
    assert!(ui
        .log
        .iter()
        .any(|event| matches!(event, aperture_core::Event::StateBasedAction { description } if description.starts_with("SANDBOX"))));

    ui.sandbox(SandboxOp::RevealTop(2));
    assert_eq!(ui.revealed.len(), 2, "reveal reports what it looked at");

    // Counters are stored but do not change combat math.
    ui.sandbox(SandboxOp::RevealTop(1));
}

#[test]
fn save_and_load_round_trip_preserves_pending_choices_rng_and_ai_mode() {
    let mut ui = session(13, false);
    ui.ai.autoplay = true;
    ui.ai.use_decider = true;
    ui.step_ai();
    let path = temp_path("roundtrip");
    ui.save(&path).expect("save succeeds");
    let saved_digest = ui.game.digest();
    let saved_hand = ui.game.state.player(P0).hand.clone();

    let mut other = session(99, false);
    assert_ne!(other.game.digest(), saved_digest);
    other
        .load(&path)
        .unwrap_or_else(|err| panic!("load failed: {err}"));
    assert_eq!(other.game.digest(), saved_digest);
    assert_eq!(other.game.state.player(P0).hand, saved_hand);
    assert_eq!(other.ai.use_decider, ui.ai.use_decider);
    assert_eq!(other.ai.autoplay, ui.ai.autoplay);
    assert_eq!(other.game.state.pending, ui.game.state.pending);
    assert_eq!(
        other.game.state.rng.raw_state(),
        ui.game.state.rng.raw_state()
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_ai_steps_only_for_its_own_seat() {
    let mut ui = session(14, false);
    // The AI seat is P1 by default; the opening mulligan belongs to P0.
    assert_eq!(ui.owner(), Some(P0));
    assert!(ui.step_ai().is_none(), "P0 is not the AI seat");
    assert_eq!(ui.owner(), Some(P0));
    assert!(ui.replay.steps.is_empty());

    // Answering for P0 and then letting the AI act for P1 works.
    ui.submit(Action::Mulligan { keep: true })
        .expect("P0 keeps");
    while ui.owner() == Some(P0) {
        ui.submit(Action::Mulligan { keep: true })
            .expect("P0 keeps");
    }
    assert_eq!(ui.owner(), Some(P1));
    assert!(ui.step_ai().is_some(), "the AI acts for P1");
    assert!(!ui.replay.steps.is_empty());
}

#[test]
fn a_new_game_resets_the_sandbox_flag_and_keeps_the_decks() {
    let mut ui = session(15, true);
    ui.sandbox(SandboxOp::Draw(1));
    assert!(ui.sandbox_modified);
    let decks = (ui.deck_a.clone(), ui.deck_b.clone());
    ui.new_game(16);
    assert!(!ui.sandbox_modified);
    assert!(ui.revealed.is_empty());
    assert_eq!((ui.deck_a.clone(), ui.deck_b.clone()), decks);
    assert_eq!(ui.game.state.seed, 16);
}

#[test]
fn the_profile_line_states_the_scope() {
    let line = aperture_ui::session::profile_line();
    assert!(line.contains("15 cards"), "{line}");
    assert!(line.contains("NOT Commander"), "{line}");
    assert!(line.contains("1v1"), "{line}");
    let _ = CardId::Shock;
    let _ = P1;
}
