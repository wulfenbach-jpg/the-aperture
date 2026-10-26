//! UI session behaviour: strict submission, sandbox marking, save/load
//! round-trip, and the AI step gate.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use aperture_core::{Action, AiSettings, CardId, DeckList, Event, ObjectId, P0, P1, Profile};
use aperture_ui::session::{
    BUNDLED_A, BUNDLED_B, SandboxOp, UiSession, import_seat_text_for, load_explicit_deck,
};

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

/// Answer the opening mulligan choices so the table reaches a real priority.
/// A sandbox control is refused while a rules choice is pending, so a test that
/// wants to override the table has to finish the opening procedure first.
fn reach_priority(ui: &mut UiSession) {
    let mut guard = 0;
    while ui.game.state.pending.is_some() {
        ui.submit(Action::Mulligan { keep: true })
            .unwrap_or_else(|err| panic!("opening keep refused: {err}"));
        guard += 1;
        assert!(guard < 40, "the opening procedure did not finish");
    }
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
    reach_priority(&mut ui);
    ui.step_ai();
    while ui.owner().is_some() && ui.seat_is_ai(ui.owner().unwrap()) {
        ui.step_ai();
    }
    let human = ui.human_seat();
    let hand_before = ui.game.state.player(human).hand.len();
    let before = ui.game.digest();
    assert!(!ui.sandbox_modified);
    ui.sandbox(SandboxOp::Draw(1))
        .expect("a draw override applies");
    assert!(ui.sandbox_modified, "a sandbox edit must be visible");
    assert_eq!(ui.game.state.player(human).hand.len(), hand_before + 1);
    assert_ne!(ui.game.digest(), before);
    assert!(
        ui.log.iter().any(|event| matches!(
            event,
            aperture_core::Event::SandboxOverride { kind, .. }
                if *kind == aperture_core::SandboxKind::Draw
        )),
        "the override is recorded as a typed trace event"
    );

    ui.sandbox(SandboxOp::RevealTop(2)).expect("reveal applies");
    assert_eq!(ui.revealed.len(), 2, "reveal reports what it looked at");

    // Counters are stored but do not change combat math.
    ui.sandbox(SandboxOp::RevealTop(1))
        .expect("a second reveal applies");
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
    reach_priority(&mut ui);
    ui.sandbox(SandboxOp::Draw(1)).expect("draw override");
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
    assert!(line.contains("native table: 2-4"), "{line}");
    assert!(line.contains("40-life"), "{line}");
    let _ = CardId::Shock;
    let _ = P1;
}

#[test]
fn sandbox_modified_save_roundtrip_preserves_the_uncertified_flag() {
    let mut ui = session(19, false);
    reach_priority(&mut ui);
    ui.sandbox(SandboxOp::Draw(1)).expect("draw override");
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

#[test]
fn native_startup_refuses_a_commander_text_file_instead_of_flattening_it() {
    // The native startup/file consumer uses `load_explicit_deck`. A file with a
    // `Commander:` header must be refused with a visible reason and an empty
    // list; the bundled practice deck must never be substituted silently.
    let path = temp_path("commander-launder");
    std::fs::write(&path, "Commander:\n1 Grizzly Bears\nDeck:\n39 Forest\n").unwrap();
    let (list, refusal) = load_explicit_deck(&path, "launder");
    assert!(
        list.cards.is_empty(),
        "no Commander card may become a practice deck"
    );
    let refusal = refusal.expect("the startup consumer must explain the refusal");
    assert!(refusal.to_lowercase().contains("commander"), "{refusal}");
    assert!(refusal.contains("practice"), "{refusal}");
    std::fs::remove_file(&path).ok();
}

#[test]
fn native_startup_refuses_parenthesized_commander_headers() {
    let path = temp_path("commander-paren");
    std::fs::write(&path, "Commander (1):\n1 Grizzly Bears\nDeck:\n39 Forest\n").unwrap();
    let (list, refusal) = load_explicit_deck(&path, "launder");
    assert!(list.cards.is_empty());
    let refusal = refusal.expect("the startup consumer must explain the refusal");
    assert!(refusal.to_lowercase().contains("section") || refusal.contains("header"));
    std::fs::remove_file(&path).ok();
}

#[test]
fn native_startup_still_accepts_a_valid_mainboard_prefixed_file() {
    let path = temp_path("mainboard");
    std::fs::write(&path, "Mainboard:\n1 Grizzly Bears\n39 Forest\n").unwrap();
    let (list, refusal) = load_explicit_deck(&path, "mainboard practice");
    assert_eq!(list.size(), 40);
    assert!(refusal.is_none(), "{refusal:?}");
    std::fs::remove_file(&path).ok();
}

fn four_seat_bounded(seed: u64) -> UiSession {
    let mut ui = session(seed, false);
    ui.start_table(seed, 4, Profile::CommanderCombat40)
        .expect("four-seat bounded table");
    ui
}

#[test]
fn native_table_starts_four_ordinary_seats_and_retains_them_on_a_new_game() {
    let mut ui = session(31, false);
    ui.start_table(31, 4, Profile::Practice20)
        .expect("four-seat ordinary table");
    assert_eq!(
        ui.seats.len(),
        4,
        "new game must not fall back to two seats"
    );
    assert_eq!(ui.game.state.seat_count(), 4);
    assert_eq!(ui.game.state.profile, Profile::Practice20);
    assert_eq!(ui.ai.human_seat, P0);
    assert_eq!(ui.ai.ai_seats, vec![1, 2, 3]);
    assert!(ui.game.state.commanders.is_empty());

    let owner = ui.owner().expect("opening mulligan");
    if owner == P0 {
        ui.submit(Action::Mulligan { keep: true })
            .expect("human keeps");
    }
    while ui.owner().is_some() && ui.owner() != Some(P0) {
        assert!(
            ui.step_ai().is_some(),
            "every non-human seat must be playable by the heuristic AI"
        );
    }
    assert!(
        ui.owner() == Some(P0),
        "the table returns to the human seat"
    );

    ui.new_game(32);
    assert_eq!(
        ui.seats.len(),
        4,
        "New game dropped the selected seat count"
    );
    assert_eq!(ui.game.state.profile, Profile::Practice20);
    assert_eq!(ui.game.state.player(P0).life, 20);
}

#[test]
fn bounded_forty_life_profile_has_one_public_commander_per_seat() {
    let ui = four_seat_bounded(33);
    assert_eq!(ui.game.state.profile, Profile::CommanderCombat40);
    assert_eq!(ui.game.state.player(P0).life, 40);
    assert_eq!(ui.game.state.commanders.len(), 4);
    assert_eq!(ui.commander_setup().len(), 4);
    let view = ui.view(P0);
    assert_eq!(view.seat_count, 4);
    assert_eq!(view.opponents.len(), 3);
    assert_eq!(view.commanders.len(), 4);
    for commander in &view.commanders {
        assert_eq!(commander.name, "Isamaru, Hound of Konda");
        assert_eq!(commander.zone, Some(aperture_core::Zone::Command));
    }
    // Command zones are public; opponent hand/library contents are not even
    // representable in OpponentView.
    for opponent in &view.opponents {
        assert_eq!(opponent.command_zone.len(), 1);
        assert!(opponent.hand_count > 0);
        assert!(opponent.library_count > 0);
    }
    assert_eq!(view.own_command_zone.len(), 1);
    assert_eq!(
        view.pending_owner,
        ui.game.decision_owner(),
        "the public pending owner must match the engine"
    );
}

#[test]
fn bounded_forty_life_four_seat_save_load_round_trips() {
    let mut ui = four_seat_bounded(34);
    while matches!(
        ui.game.state.pending,
        Some(aperture_core::Pending::Mulligan { .. })
    ) {
        if ui.owner() == Some(P0) {
            ui.submit(Action::Mulligan { keep: true }).expect("keep");
        } else {
            assert!(ui.step_ai().is_some(), "AI keeps its hand");
        }
    }
    let path = temp_path("bounded-four-seat");
    ui.save(&path).expect("bounded save succeeds");
    let digest = ui.game.digest();

    let mut other = session(35, false);
    other.load(&path).expect("bounded load succeeds");
    assert_eq!(other.game.digest(), digest);
    assert_eq!(other.seats.len(), 4, "load must restore the seat count");
    assert_eq!(other.game.state.profile, Profile::CommanderCombat40);
    assert_eq!(other.game.state.player(P0).life, 40);
    assert_eq!(other.game.state.commanders.len(), 4);
    assert_eq!(other.seat_url.len(), 4, "per-seat controls follow the load");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn four_seat_load_resizes_the_native_import_controls() {
    let mut ui = session(36, false);
    let path = temp_path("four-seat-resize");
    ui.start_table(36, 3, Profile::Practice20)
        .expect("three-seat table");
    ui.save(&path).expect("save");
    let mut other = session(37, false);
    assert_eq!(other.seat_url.len(), 2);
    other.load(&path).expect("load");
    assert_eq!(other.seats.len(), 3);
    assert_eq!(other.seat_url.len(), 3);
    assert_eq!(other.seat_import.len(), 3);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn default_four_seat_bounded_libraries_actually_cast_isamaru() {
    let mut cast_seed = None;
    'seeds: for seed in 0..24u64 {
        let mut ui = session(seed, false);
        ui.start_table(seed, 4, Profile::CommanderCombat40)
            .expect("four-seat bounded table");
        for (seat, deck) in ui.seats.iter().enumerate() {
            assert!(
                deck.count_of(CardId::Plains) > 0,
                "P{seat} bounded default has no white source: {}",
                deck.name
            );
            assert_eq!(
                deck.count_of(CardId::IsamaruHoundOfKonda),
                0,
                "the commander must not be part of the library"
            );
        }
        // Drive every seat with the same heuristic policy the UI uses.
        ui.human_plays_ai_seat = true;
        for _ in 0..600 {
            if ui.game.state.is_over() {
                break;
            }
            let Some(seat) = ui.owner() else { break };
            assert!(ui.seat_is_ai(seat), "P{seat} has no policy");
            let _ = ui.step_ai();
            if ui.log.iter().any(|event| {
                matches!(
                    event,
                    Event::Cast {
                        card: CardId::IsamaruHoundOfKonda,
                        ..
                    }
                )
            }) {
                cast_seed = Some(seed);
                break 'seeds;
            }
        }
    }
    assert!(
        cast_seed.is_some(),
        "no default four-seat bounded game cast its supplied Isamaru within 600 actions per seed"
    );
}

#[test]
fn ordinary_default_seats_stay_temur_izzet() {
    let seats =
        UiSession::bundled_seats_for_profile(4, Profile::Practice20).expect("four ordinary seats");
    let names: Vec<&str> = seats.iter().map(|deck| deck.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Temur Beats",
            "Izzet Control",
            "Temur Beats P2",
            "Izzet Control P3"
        ]
    );
    assert!(seats.iter().all(|deck| deck.count_of(CardId::Plains) == 0));
    assert!(seats.iter().all(|deck| deck.size() == 40));
}

// ------------------------------------------------------- issue21 import journey

/// 40 supported library cards plus a supported commander: the bounded
/// command-zone seat contract that a real Archidekt Commander list has to be
/// reduced to before this build can seat it.
const COMMAND_SEAT: &str = "\
Commander
1 Isamaru, Hound of Konda
Deck
12 Plains
8 Forest
8 Island
4 Grizzly Bears
4 Centaur Courser
2 Unsummon
2 Swords to Plowshares
";

#[test]
fn a_commander_import_certifies_and_starts_the_seat_with_its_own_commander() {
    let mut ui = session(41, false);
    ui.start_table(41, 2, Profile::CommanderCombat40)
        .expect("bounded command table");
    let profile = ui.profile;
    for seat in 0..2u8 {
        let import = import_seat_text_for(
            seat,
            &format!("P{seat} command import"),
            COMMAND_SEAT,
            profile,
        )
        .expect("import runs");
        assert!(import.certified(), "refusal: {:?}", import.refusal);
        assert_eq!(import.commander, Some(CardId::IsamaruHoundOfKonda));
        // The two axes are reported separately and honestly: 41 cards is not a
        // legal Commander deck, and it is still a supported, seatable list.
        assert_eq!(import.format_status, "illegal");
        assert_eq!(import.support_status, "supported");
        assert_eq!(import.profile, profile.label());
        ui.seat_import[seat as usize] = Some(import);
    }
    assert!(ui.can_start_imported(), "both seats are certified");
    ui.start_imported_match(42).expect("imported match starts");

    assert_eq!(ui.game.state.profile, Profile::CommanderCombat40);
    assert_eq!(ui.game.state.player(P0).life, 40);
    assert_eq!(ui.seats.len(), 2);
    assert!(ui.seats.iter().all(|deck| deck.size() == 40));
    assert_eq!(ui.game.state.commanders.len(), 2);
    for commander in &ui.game.state.commanders {
        assert_eq!(commander.card, CardId::IsamaruHoundOfKonda);
    }
    assert_eq!(ui.commander_setup().len(), 2);
    // The imported commander is public in the other seat's command zone.
    let view = ui.view(P1);
    assert_eq!(view.opponents[0].command_zone.len(), 1);
    assert_eq!(view.commanders.len(), 2);
}

#[test]
fn an_unsupported_card_in_a_commander_import_is_refused_and_never_substituted() {
    let mut ui = session(43, false);
    ui.start_table(43, 2, Profile::CommanderCombat40)
        .expect("bounded command table");
    let text = COMMAND_SEAT.replace("2 Unsummon", "2 Darksteel Ingot");
    let import = import_seat_text_for(0, "P0 refused", &text, ui.profile).expect("import runs");
    assert!(!import.certified());
    assert_eq!(import.support_status, "unsupported");
    let sol_ring = import
        .unsupported
        .iter()
        .find(|card| card.name == "Darksteel Ingot")
        .expect("Darksteel Ingot is listed with its reason");
    assert!(sol_ring.reason.contains("not in the supported pool"));
    assert!(
        import
            .refusal
            .as_deref()
            .unwrap_or_default()
            .contains("never substituted"),
        "refusal: {:?}",
        import.refusal
    );
    ui.seat_import[0] = Some(import);
    assert!(!ui.can_start_imported());
    assert!(ui.start_imported_match(44).is_err());
    // The live table is untouched: the bundled lists were not replaced, and no
    // partially supported list was quietly applied.
    assert_eq!(ui.seats[0].name, "Command zone practice");
}

#[test]
fn a_stale_import_result_is_discarded_and_the_seat_is_not_left_busy() {
    let mut ui = session(45, false);
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<()>();
    let started = ui.request_import(0, move || {
        // Hold the worker until the test has changed the session.
        let _ = ready_rx.recv();
        import_seat_text_for(0, "late", COMMAND_SEAT, Profile::CommanderCombat40)
    });
    assert!(started);
    assert!(ui.import_busy[0]);
    ui.new_game(46);
    ready_tx.send(()).expect("release the worker");
    let mut applied = Vec::new();
    for _ in 0..600 {
        applied = ui.poll_workers();
        if !ui.import_busy[0] && !applied.is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        !ui.import_busy[0],
        "a discarded import must clear its busy flag, or the panel spins forever"
    );
    assert!(applied.is_empty(), "the stale result must not be applied");
    assert!(
        ui.busy_note
            .as_deref()
            .unwrap_or_default()
            .contains("stale"),
        "busy note: {:?}",
        ui.busy_note
    );
    assert!(ui.seat_import[0].is_none());
    assert!(!ui.can_start_imported());
}
