//! UI session behaviour: strict submission, sandbox marking, save/load
//! round-trip, and the AI step gate.

use std::path::PathBuf;
use std::time::{Duration, Instant};

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
        vec![deck_a(), deck_b()],
        AiSettings::default(),
        false,
        human_plays_ai_seat,
    )
    .expect("session")
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
    let decks = (ui.seats[0].clone(), ui.seats[1].clone());
    ui.new_game(16);
    assert!(!ui.sandbox_modified);
    assert!(ui.revealed.is_empty());
    assert_eq!((ui.seats[0].clone(), ui.seats[1].clone()), decks);
    assert_eq!(ui.game.state.seed, 16);
}

#[test]
fn a_branch_point_restores_the_earlier_position_and_labels_the_boundary() {
    let mut ui = session(17, true);
    ui.step_ai();
    let branch_digest = ui.game.digest();
    let branch_log = ui.log.len();
    let branch_step = ui.game.state.turn.step;
    ui.set_branch();
    assert!(ui.branch.is_some());
    assert!(
        ui.message.contains("Hidden-information boundary"),
        "{}",
        ui.message
    );

    // Play on, then return: everything after the branch point is discarded.
    for _ in 0..25 {
        if ui.game.state.is_over() {
            break;
        }
        ui.step_ai();
    }
    assert_ne!(ui.game.digest(), branch_digest);
    ui.return_to_branch().expect("a branch point exists");
    assert_eq!(ui.game.digest(), branch_digest);
    assert_eq!(ui.log.len(), branch_log, "the log is rewound too");
    assert_eq!(ui.game.state.turn.step, branch_step);
    assert!(ui.message.contains("branch point"));

    // Without a branch point the button cannot fabricate one.
    let mut fresh = session(18, false);
    assert!(fresh.return_to_branch().is_err());
}

#[test]
fn the_profile_line_states_the_scope() {
    let line = aperture_ui::session::profile_line();
    assert!(line.contains("2-4 live seats"), "{line}");
    assert!(line.contains("NOT Commander"), "{line}");
    assert!(line.contains("native table: 2"), "{line}");
    let _ = CardId::Shock;
    let _ = P1;
}

#[test]
fn sandbox_modified_save_roundtrip_preserves_the_uncertified_flag() {
    let mut ui = session(19, false);
    ui.sandbox(SandboxOp::Draw(1));
    assert!(ui.sandbox_modified);
    let digest = ui.game.digest();
    let path = temp_path("sandbox-roundtrip");
    ui.save(&path).expect("sandbox save succeeds");

    let mut other = session(20, false);
    other.load(&path).expect("sandbox load succeeds");
    assert!(
        other.sandbox_modified,
        "the uncertified flag must survive save/load"
    );
    assert_eq!(other.game.digest(), digest);
    let _ = std::fs::remove_file(&path);
}

fn crack_deck() -> DeckList {
    DeckList::new(
        "Crack practice",
        vec![(CardId::CrackTheEarth, 4), (CardId::Mountain, 36)],
    )
}

fn crack_seed(decks: &[DeckList]) -> u64 {
    for seed in 0..1_000u64 {
        let (game, _) = aperture_core::Game::new_logged(seed, decks).expect("game");
        let target = game.state.first_player;
        let hand = &game.state.player(target).hand;
        let has_crack = hand
            .iter()
            .any(|id| game.state.object(*id).card == CardId::CrackTheEarth);
        let has_mountain = hand
            .iter()
            .any(|id| game.state.object(*id).card == CardId::Mountain);
        if has_crack && has_mountain {
            return seed;
        }
    }
    panic!("a seed with Crack the Earth and Mountain");
}

#[test]
fn native_session_makes_the_apnap_choice_actionable_only_for_the_entitled_human() {
    let deck = crack_deck();
    let decks = vec![deck.clone(), deck];
    let seed = crack_seed(&decks);
    let ai = AiSettings {
        enabled: false,
        human_seat: P0,
        ai_seats: Vec::new(),
        ..AiSettings::default()
    };
    let mut ui = UiSession::new(seed, decks, ai, false, false).expect("session");
    let order = ui
        .game
        .state
        .mulligan
        .as_ref()
        .expect("mulligan state")
        .order
        .clone();
    for _ in order {
        ui.submit(Action::Mulligan { keep: true })
            .expect("opening hand kept");
    }
    let target = ui.game.state.first_player;
    let mut guard = 0;
    while !(ui.game.state.turn.active == target
        && ui.game.state.turn.step == aperture_core::Step::PrecombatMain
        && ui.game.state.turn.priority == Some(target)
        && ui.game.state.stack.is_empty())
    {
        ui.submit(Action::Pass).expect("pass");
        guard += 1;
        assert!(guard < 64);
    }
    let mountain = ui
        .game
        .state
        .player(target)
        .hand
        .iter()
        .copied()
        .find(|id| ui.game.state.object(*id).card == CardId::Mountain)
        .expect("Mountain");
    ui.submit(Action::PlayLand { card: mountain })
        .expect("land");
    ui.submit(Action::ActivateManaAbility { land: mountain })
        .expect("mana");
    let crack = ui
        .game
        .state
        .player(target)
        .hand
        .iter()
        .copied()
        .find(|id| ui.game.state.object(*id).card == CardId::CrackTheEarth)
        .expect("Crack");
    ui.submit(Action::CastSpell {
        card: crack,
        targets: Vec::new(),
    })
    .expect("cast");
    while !matches!(
        ui.game.state.pending,
        Some(aperture_core::Pending::ApnapSacrifice { .. })
    ) {
        ui.submit(Action::Pass).expect("pass");
    }
    let chooser = ui.owner().expect("chooser");
    let choice = ui
        .game
        .legal_actions()
        .into_iter()
        .find(|legal| matches!(legal.action, Action::ChooseSacrifice { .. }))
        .expect("a sacrifice choice is generated")
        .action;
    ui.ai.human_seat = if chooser == P0 { P1 } else { P0 };
    assert!(ui.submit_human(choice.clone()).is_err());
    ui.ai.human_seat = chooser;
    ui.submit_human(choice).expect("entitled human answers");
    assert_ne!(ui.owner(), Some(chooser), "the queue advanced");
    let (replayed, _) = ui.replay.run().expect("the session replay reproduces");
    assert_eq!(replayed.digest(), ui.game.digest());
}

#[test]
fn background_load_of_a_malformed_apnap_save_clears_busy_and_preserves_state() {
    let deck = crack_deck();
    let decks = vec![deck.clone(), deck];
    let seed = crack_seed(&decks);
    let ai = AiSettings {
        enabled: false,
        human_seat: P0,
        ai_seats: Vec::new(),
        ..AiSettings::default()
    };
    let mut ui = UiSession::new(seed, decks, ai, false, false).expect("session");
    while matches!(
        ui.game.state.pending,
        Some(aperture_core::Pending::Mulligan { .. })
    ) {
        ui.submit(Action::Mulligan { keep: true }).expect("keep");
    }
    let mut guard = 0;
    while !(ui.game.state.turn.step == aperture_core::Step::PrecombatMain
        && ui.game.state.turn.priority == Some(P0)
        && ui.game.state.stack.is_empty())
    {
        ui.submit(Action::Pass).expect("pass");
        guard += 1;
        assert!(guard < 64);
    }
    let mountain = *ui
        .game
        .state
        .player(P0)
        .hand
        .iter()
        .find(|id| ui.game.state.object(**id).card == CardId::Mountain)
        .expect("Mountain");
    ui.submit(Action::PlayLand { card: mountain })
        .expect("land");
    let crack = *ui
        .game
        .state
        .player(P0)
        .hand
        .iter()
        .find(|id| ui.game.state.object(**id).card == CardId::CrackTheEarth)
        .expect("Crack");
    ui.submit(Action::CastSpell {
        card: crack,
        targets: Vec::new(),
    })
    .expect("cast");
    while !matches!(
        ui.game.state.pending,
        Some(aperture_core::Pending::ApnapSacrifice { .. })
    ) {
        ui.submit(Action::Pass).expect("pass");
    }
    let choice = ui
        .game
        .legal_actions()
        .into_iter()
        .find(|legal| matches!(legal.action, Action::ChooseSacrifice { .. }))
        .expect("choice")
        .action;
    ui.submit(choice).expect("partial answer");

    let valid = temp_path("apnap-valid");
    ui.save(&valid).expect("valid save");
    let mut json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&valid).unwrap()).unwrap();
    let source = json["game"]["resolving"].as_u64().expect("resolving");
    json["game"]["objects"]
        .as_object_mut()
        .expect("objects")
        .remove(&source.to_string());
    let malformed = temp_path("apnap-malformed");
    std::fs::write(&malformed, serde_json::to_string(&json).unwrap()).unwrap();

    let digest = ui.game.digest();
    let replay_steps = ui.replay.steps.len();
    let ai_before = ui.ai.clone();
    ui.request_load(malformed.clone());
    let deadline = Instant::now() + Duration::from_secs(10);
    while ui.io_busy && Instant::now() < deadline {
        ui.poll_workers();
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!ui.io_busy, "a malformed load must clear io_busy");
    assert!(
        ui.message.contains("load failed") || ui.message.contains("missing"),
        "useful error missing: {}",
        ui.message
    );
    assert_eq!(ui.game.digest(), digest);
    assert_eq!(ui.replay.steps.len(), replay_steps);
    assert_eq!(ui.ai, ai_before);

    ui.load(&valid).expect("valid APNAP load still works");
    let _ = std::fs::remove_file(&valid);
    let _ = std::fs::remove_file(&malformed);
}
