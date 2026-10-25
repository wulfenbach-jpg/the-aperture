//! Focused issue23 UI-session controls on the native table state:
//! - a four-seat save taken with a stack item mid-resolution and an APNAP
//!   sacrifice pending reloads the same pending choice, chooser and stack;
//! - a stale branch result is discarded by the generation guard;
//! - branching a recorded line at an explicit action point, then diverging,
//!   is exposed with a public accepted-action comparison;
//! - the postgame omniscient review is separate from every policy input.

use std::path::PathBuf;

use aperture_core::{Action, AiSettings, CardId, DeckList, P0, Pending};
use aperture_ui::session::{BranchOutcome, SandboxOp, UiSession, WorkerMessage};

fn crack_deck() -> DeckList {
    DeckList::new(
        "Crack practice",
        vec![(CardId::CrackTheEarth, 4), (CardId::Mountain, 36)],
    )
}

fn crack_seed(decks: &[DeckList]) -> u64 {
    for seed in 0..10_000u64 {
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

fn four_seat_apnap_ui() -> UiSession {
    let deck = crack_deck();
    let decks = vec![deck.clone(), deck.clone(), deck.clone(), deck];
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
        assert!(guard < 256, "never reached the precombat main phase");
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
    let mut guard = 0;
    while !matches!(ui.game.state.pending, Some(Pending::ApnapSacrifice { .. })) {
        ui.submit(Action::Pass).expect("pass");
        guard += 1;
        assert!(guard < 64, "never reached the APNAP sacrifice choice");
    }
    ui
}

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("aperture-test-{name}-{}.json", std::process::id()));
    path
}

fn first_legal(ui: &UiSession) -> Action {
    ui.game
        .legal_actions()
        .into_iter()
        .next()
        .expect("the engine always offers at least one legal action at a live decision")
        .action
}

#[test]
fn a_four_seat_save_taken_with_a_resolving_spell_and_apnap_pending_restores_the_boundary() {
    let mut ui = four_seat_apnap_ui();
    let owner = ui.owner().expect("chooser");
    let serial = ui.game.decision();
    let stack_len = ui.game.state.stack.len();
    let digest = ui.game.digest();

    let path = temp_path("issue23-pending");
    ui.save(&path).expect("save succeeds");

    let mut reload = four_seat_apnap_ui();
    reload.load(&path).expect("load restores");
    assert_eq!(reload.game.digest(), digest);
    assert_eq!(reload.game.decision(), serial);
    assert_eq!(
        reload.game.state.stack.len(),
        stack_len,
        "the resolving item is still on the stack"
    );
    assert_eq!(reload.game.state.pending, ui.game.state.pending);
    assert_eq!(reload.owner(), Some(owner));
    assert!(matches!(
        reload.game.state.pending,
        Some(Pending::ApnapSacrifice { .. })
    ));
    drop(std::fs::remove_file(&path));
}

#[test]
fn a_stale_branch_result_is_discarded_by_the_generation_guard() {
    let mut ui = four_seat_apnap_ui();

    // A Branch message captured with the generation seen BEFORE the next
    // accepted action must be discarded when that action lands first. The
    // accepted action here is the pending APNAP choice (a pass would be
    // illegal while a pending choice must be answered); the queued outcome is
    // deliberately harmless (the unchanged session) so only the guard itself
    // is under test.
    let stale_generation = ui.generation;
    let outcome = BranchOutcome {
        parent: ui.replay.clone(),
        branched: ui.replay.clone(),
        game: ui.game.clone(),
        log: ui.log.clone(),
    };
    ui.worker_tx
        .send(WorkerMessage::Branch {
            generation: stale_generation,
            path: PathBuf::from("stale.json"),
            keep: 0,
            result: Ok(Box::new(outcome)),
        })
        .expect("queue the stale branch");
    let answer = first_legal(&ui);
    ui.submit(answer).expect("the pending choice is accepted");
    let digest = ui.game.digest();
    let decision = ui.game.decision();
    let log_len = ui.log.len();
    let replay_len = ui.replay.steps.len();
    let applied = ui.poll_workers();
    assert!(
        ui.busy_note
            .as_deref()
            .unwrap_or("")
            .contains("stale branch"),
        "expected a stale-branch reason, got {:?}",
        ui.busy_note
    );
    assert!(
        applied
            .iter()
            .all(|line| !line.starts_with("branched from")),
        "a stale branch must not report success: {applied:?}"
    );
    assert_eq!(ui.game.digest(), digest, "state was overwritten");
    assert_eq!(ui.game.decision(), decision, "decision moved");
    assert_eq!(
        ui.log.len(),
        log_len,
        "a discarded stale branch must not log anything"
    );
    assert_eq!(
        ui.replay.steps.len(),
        replay_len,
        "a discarded stale branch must not record anything"
    );
}

#[test]
fn a_replay_branches_at_a_chosen_action_and_the_comparison_stays_public() {
    let mut ui = four_seat_apnap_ui();
    // Answer the pending APNAP choice until every owed sacrifice is given (a
    // pass is not legal while a pending choice must be answered), then ride
    // back to a plain priority boundary so the tail actions are clean passes.
    let mut guard = 0;
    while ui.game.state.pending.is_some() {
        ui.submit(first_legal(&ui))
            .expect("the engine accepts its own offer");
        guard += 1;
        assert!(guard < 64, "the pending choice never resolved");
    }
    let mut guard = 0;
    while !(ui.game.state.turn.active == ui.game.state.first_player
        && ui.game.state.turn.step == aperture_core::Step::PrecombatMain
        && ui.game.state.turn.priority == Some(ui.game.state.first_player)
        && ui.game.state.stack.is_empty()
        && ui.game.state.pending.is_none())
    {
        ui.submit(Action::Pass).expect("pass");
        guard += 1;
        assert!(guard < 512, "never got back to priority");
    }
    // Record one more action so the line is longer than the branch point.
    ui.submit(Action::Pass).expect("pass");
    let path = temp_path("issue23-branch");
    ui.save(&path).expect("save succeeds");
    let base = ui.replay.steps.len();
    assert!(base >= 6, "a usable recorded line, got {base}");
    let keep = base - 1usize;

    let mut fresh = four_seat_apnap_ui();
    fresh.request_replay_branch(path.clone(), keep);
    let mut applied = Vec::new();
    let mut guard = 0;
    while guard < 2000 {
        applied.extend(fresh.poll_workers());
        if !fresh.io_busy {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        guard += 1;
    }
    assert!(!fresh.io_busy, "the branch worker never finished");
    fresh.poll_workers();
    assert!(
        applied.iter().any(|line| line.starts_with("branched from")),
        "{applied:?}"
    );
    assert!(fresh.branch_source.is_some(), "the base line is remembered");
    assert_eq!(fresh.replay.steps.len(), keep);

    // The recorded prefix replays to exactly the state the base line had there.
    let (prefix_game, _) = {
        let mut prefix = fresh.branch_source.clone().unwrap();
        prefix.steps.truncate(keep);
        prefix.final_digest.clear();
        prefix.run_recorded().expect("prefix replays")
    };
    assert_eq!(fresh.game.digest(), prefix_game.digest());
    assert_eq!(fresh.game.decision(), prefix_game.decision());

    let diff_before = fresh.branch_comparison().expect("a comparison exists");
    assert_eq!(diff_before.diverged_at, None, "nothing new played yet");

    // First replay the same accepted action the base recorded at the
    // boundary (its state is bit-identical and deterministic), then play one
    // action beyond the recorded base tail — the histories stop agreeing the
    // moment the branch says something the base line never recorded.
    let base_step = &fresh.branch_source.as_ref().unwrap().steps[keep];
    fresh
        .submit(base_step.action.clone())
        .unwrap_or_else(|err| panic!("the base's own action replays on the branch: {err}"));
    assert!(
        fresh.branches_agree(),
        "the replayed move is still the base move"
    );
    let beyond = first_legal(&fresh);
    let original = fresh.game.digest();
    fresh
        .submit(beyond.clone())
        .unwrap_or_else(|err| panic!("the branch accepts its own action: {err}"));
    assert_ne!(
        fresh.game.digest(),
        original,
        "the branch actually moved past the recorded tail"
    );

    let diff = fresh.branch_comparison().expect("a comparison exists");
    assert_eq!(
        diff.diverged_at,
        Some(fresh.replay.steps.len() - 1),
        "the histories stop agreeing exactly when the branch leaves the recorded line"
    );
    let rendered = diff.render();
    assert!(
        rendered.contains("(nothing — line has ended here)"),
        "the base tail after the branch point stays unrevealed: {rendered}"
    );
    assert!(
        !rendered.contains("Crack the Earth") && !rendered.contains("Mountain"),
        "the comparison must stay on the public action axis: {rendered}"
    );
    assert_eq!(
        fresh.replay.steps.len(),
        keep + 1 + 1,
        "the branch only carries its own history"
    );
    drop(std::fs::remove_file(&path));
}

#[test]
fn the_postgame_review_is_omniscient_read_only_and_refused_while_live() {
    let mut ui = four_seat_apnap_ui();
    // While a live decision is outstanding, review must be refused.
    let err = ui.enter_review().expect_err("review refused while live");
    assert!(
        err.contains("only available once the game has actually ended"),
        "{err}"
    );

    // Play the four-seat mountain deck out: every seat eventually draws from
    // an empty library and leaves (CR 104.3c/704.5b), so the game ends. Each
    // decision is answered with the engine's own first legal offer.
    let mut guard = 0usize;
    while !ui.game.state.is_over() {
        let Some(_) = ui.owner() else {
            break;
        };
        let _ = ui.submit(first_legal(&ui));
        guard += 1;
        assert!(guard < 200_000, "the game did not end");
    }
    assert!(ui.game.state.is_over(), "the exhausted deck ends the game");
    ui.enter_review()
        .expect("review opens once the game is over");
    let review = ui.review_cache.clone().expect("cached review");
    assert!(review.starts_with("POSTGAME OMNISCIENT REVIEW"), "{review}");
    // Omniscient: every seat's hand is named, including the opponents'.
    assert!(review.contains("P1 '"), "opponent seat present: {review}");
    assert!(review.contains("hand ("), "hands are listed: {review}");
    // Read-only: acting is refused in review mode.
    let before = ui.game.digest();
    assert!(
        ui.submit(Action::Pass).is_err(),
        "review refuses submissions"
    );
    assert!(
        ui.message.contains("read-only"),
        "the refusal is reported to the user: {}",
        ui.message
    );
    ui.sandbox(SandboxOp::Draw(1));
    assert_eq!(
        ui.game.digest(),
        before,
        "sandbox is also blocked in review"
    );
    ui.leave_review();
    assert!(
        !ui.review_mode && ui.review_cache.is_none(),
        "review closes cleanly"
    );
}
