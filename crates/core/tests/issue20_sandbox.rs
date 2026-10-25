//! Focused issue20 engine coverage for the manual table controls and the
//! sandbox taint record:
//! - draw/untap/mill/scry/surveil/reveal/counters/life all go through one
//!   deterministic engine path that emits a typed, viewer-masked trace event;
//! - a scry or surveil look is private to the seat that looked, while a reveal
//!   is public by construction;
//! - a control is refused while a rules choice is pending, and a refusal leaves
//!   the state untouched;
//! - the taint record survives save/load and cannot be laundered by clearing
//!   the boolean flag;
//! - the shared zone inspector and search read only the masked projection.

mod common;

use aperture_core::view::render_event;
use aperture_core::{
    Action, AiSettings, CardId, Event, Game, ObjectId, P0, P1, Replay, SandboxKind, SandboxOp,
    SandboxRecord, SessionSave, Zone,
};
use common::{deck_a, deck_b};

/// A two-seat practice game driven to the first real priority (past mulligans).
fn started(seed: u64) -> (Game, Vec<Event>) {
    let (mut game, mut log) = Game::new_logged(seed, &[deck_a(), deck_b()]).expect("game");
    let mut guard = 0;
    while game.state.pending.is_some() {
        let owner = game.decision_owner().expect("pending owner");
        let action = match &game.state.pending {
            Some(aperture_core::Pending::Mulligan { .. }) => Action::Mulligan { keep: true },
            other => panic!("unexpected opening choice: {other:?}"),
        };
        match game.submit_action(owner, action) {
            Ok(events) => log.extend(events),
            Err(err) => panic!("opening choice rejected: {err}"),
        }
        guard += 1;
        assert!(guard < 40, "opening procedure did not finish");
    }
    (game, log)
}

fn top_card(game: &Game, seat: u8) -> CardId {
    let id = *game.state.player(seat).library.first().expect("library");
    game.state.object(id).card
}

fn replay_for(game: &Game) -> Replay {
    let mut replay = Replay::new(
        game.state.seed,
        game.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .expect("replay");
    replay.steps.clear();
    replay
}

#[test]
fn draw_untap_mill_and_reveal_are_traceable_and_deterministic() {
    let (mut game, _) = started(4242);
    let hand_before = game.state.player(P0).hand.len();
    let library_before = game.state.player(P0).library.len();
    let outcome = game
        .sandbox(SandboxOp::Draw { seat: P0, count: 2 })
        .expect("draw is allowed at a priority");
    assert_eq!(outcome.edit.kind, SandboxKind::Draw);
    assert_eq!(outcome.edit.seat, P0);
    assert_eq!(game.state.player(P0).hand.len(), hand_before + 2);
    assert_eq!(game.state.player(P0).library.len(), library_before - 2);
    assert_eq!(
        outcome.events.len(),
        3,
        "two draws plus the override record"
    );
    assert!(matches!(
        outcome.events.last(),
        Some(Event::SandboxOverride {
            kind: SandboxKind::Draw,
            count: 2,
            private_cards,
            ..
        }) if private_cards.is_empty()
    ));

    // Milling is a library -> graveyard move with no "dies" trigger.
    let grave_before = game.state.player(P0).graveyard.len();
    game.sandbox(SandboxOp::Mill { seat: P0, count: 1 })
        .expect("mill");
    assert_eq!(game.state.player(P0).graveyard.len(), grave_before + 1);
    assert!(
        !game
            .state
            .trigger_queue
            .iter()
            .any(|trigger| format!("{trigger:?}").contains("Dies")),
        "a milled card must not fire a dies trigger (CR 701.13a)"
    );

    // Untap clears tapped state without granting an untap step.
    let permanents: Vec<ObjectId> = game
        .state
        .battlefield(P0)
        .into_iter()
        .map(|object| object.id)
        .collect();
    if let Some(id) = permanents.first()
        && let Some(object) = game.state.objects.get_mut(id)
    {
        object.tapped = true;
    }
    let outcome = game.sandbox(SandboxOp::Untap { seat: P0 }).expect("untap");
    assert!(outcome.edit.detail.contains("not an untap step"));
    assert!(
        game.state
            .battlefield(P0)
            .iter()
            .all(|object| !object.tapped)
    );

    // Reveal is public: the names are in the public detail for every seat.
    let revealed = top_card(&game, P0);
    let outcome = game
        .sandbox(SandboxOp::Reveal { seat: P0, count: 1 })
        .expect("reveal");
    assert!(outcome.edit.detail.contains(revealed.name()));
    assert_eq!(
        game.state.player(P0).library.len(),
        library_before - 3,
        "draw 2 + mill 1 consumed three cards; reveal does not move any"
    );
}

#[test]
fn a_scry_look_is_private_to_the_seat_that_looked() {
    let (mut game, _) = started(4242);
    let looked = top_card(&game, P0);
    let outcome = game
        .sandbox(SandboxOp::Scry {
            seat: P0,
            count: 1,
            to_top: true,
        })
        .expect("scry");
    let event = outcome.events.last().expect("trace event").clone();
    let own = render_event(&game, P0, &event);
    let other = render_event(&game, P1, &event);
    assert!(
        own.contains(looked.name()),
        "the seat that looked must see the card: {own}"
    );
    assert!(
        !other.contains(looked.name()),
        "the other seat must never see a scry look: {other}"
    );
    assert!(
        other.contains("SANDBOX"),
        "the override stays visible to every seat: {other}"
    );
    // Keeping the card on top leaves the library order untouched.
    assert_eq!(top_card(&game, P0), looked);

    // Bottoming the same card moves it to the bottom in the same relative order.
    let library_len = game.state.player(P0).library.len();
    game.sandbox(SandboxOp::Scry {
        seat: P0,
        count: 1,
        to_top: false,
    })
    .expect("scry to the bottom");
    let last = *game.state.player(P0).library.last().expect("library");
    assert_eq!(
        game.state.object(last).card,
        looked,
        "the looked-at card is now on the bottom"
    );
    assert_eq!(game.state.player(P0).library.len(), library_len);
}

#[test]
fn surveil_to_the_graveyard_counter_and_life_edits_are_recorded() {
    let (mut game, _) = started(4242);
    let grave_before = game.state.player(P0).graveyard.len();
    game.sandbox(SandboxOp::Surveil {
        seat: P0,
        count: 2,
        to_top: false,
    })
    .expect("surveil");
    assert_eq!(
        game.state.player(P0).graveyard.len(),
        grave_before + 2,
        "surveil to the graveyard moves both looked-at cards"
    );

    // Counter editor: bounded, battlefield-only, storage-only.
    let creature = game
        .state
        .battlefield(P0)
        .into_iter()
        .find(|object| object.is_creature())
        .map(|object| object.id);
    if let Some(id) = creature {
        game.sandbox(SandboxOp::SetCounters {
            object: id,
            count: 2,
        })
        .expect("set counters");
        assert_eq!(game.state.object(id).plus_one_counters, 2);
        let outcome = game
            .sandbox(SandboxOp::AddCounters {
                object: id,
                delta: -1,
            })
            .expect("remove one counter");
        assert_eq!(game.state.object(id).plus_one_counters, 1);
        assert!(outcome.edit.detail.contains("storage only"));
        assert!(
            game.sandbox(SandboxOp::AddCounters {
                object: id,
                delta: -5
            })
            .is_err(),
            "a counter count can never go below zero"
        );
        assert!(
            game.sandbox(SandboxOp::SetCounters {
                object: id,
                count: 100
            })
            .is_err(),
            "the counter editor is bounded"
        );
    }
    // A card in hand is not a permanent, and a guessed id is refused outright.
    let in_hand = *game.state.player(P0).hand.first().expect("own hand");
    assert!(
        game.sandbox(SandboxOp::SetCounters {
            object: in_hand,
            count: 1
        })
        .is_err()
    );
    assert!(
        game.sandbox(SandboxOp::SetCounters {
            object: ObjectId(9_999),
            count: 1
        })
        .is_err()
    );

    // Life edits are recorded against the seat and never end the game.
    let outcome = game
        .sandbox(SandboxOp::SetLife { seat: P1, life: 31 })
        .expect("life edit");
    assert_eq!(game.state.player(P1).life, 31);
    assert_eq!(outcome.edit.seat, P1);
    assert!(game.state.outcome.is_none());
    assert!(
        game.sandbox(SandboxOp::SetLife { seat: P1, life: 0 })
            .is_err()
    );
}

#[test]
fn a_control_is_refused_while_a_rules_choice_is_pending() {
    let (mut game, _) = Game::new_logged(7, &[deck_a(), deck_b()]).expect("game");
    assert!(
        game.state.pending.is_some(),
        "the opening mulligan choice is pending"
    );
    let before = game.digest();
    let err = game
        .sandbox(SandboxOp::Draw { seat: P0, count: 1 })
        .expect_err("pending choice blocks the sandbox");
    assert!(err.to_string().contains("pending choice"), "{err}");
    assert_eq!(game.digest(), before, "a refusal must not touch the state");

    // An oversized control is refused instead of silently doing less.
    let (mut game, _) = started(4242);
    let library_len = game.state.player(P1).library.len() as u8;
    let err = game
        .sandbox(SandboxOp::Draw {
            seat: P1,
            count: library_len + 1,
        })
        .expect_err("a draw past the library is refused");
    assert!(err.to_string().contains("library"), "{err}");
    assert_eq!(
        game.state.player(P1).library.len(),
        library_len as usize,
        "the refusal left the library untouched"
    );
}

#[test]
fn the_taint_record_survives_save_load_and_cannot_be_laundered() {
    let (mut game, mut log) = started(4242);
    let mut record = SandboxRecord::default();
    assert!(!record.is_tainted());
    let outcome = game
        .sandbox(SandboxOp::Draw { seat: P0, count: 1 })
        .expect("draw");
    record.push(outcome.edit.clone());
    log.extend(outcome.events.clone());

    let save = SessionSave::capture_recorded(&game, replay_for(&game), log, record);
    assert!(save.tainted());
    assert!(save.record_line().contains("NOT a legal match result"));
    let text = save.to_json();
    let loaded = SessionSave::from_json(&text).expect("a tainted save loads");
    assert!(loaded.tainted());
    assert_eq!(loaded.sandbox.len(), 1);
    assert_eq!(loaded.sandbox.edits[0].kind, SandboxKind::Draw);
    assert_eq!(
        loaded.game, save.game,
        "the saved state round-trips exactly"
    );

    // Clearing only the flag must not launder the game into a match result.
    let mut value: serde_json::Value = serde_json::from_str(&text).expect("json");
    value["sandbox_modified"] = serde_json::json!(false);
    let laundered = serde_json::to_string(&value).expect("json");
    let err = SessionSave::from_json(&laundered).expect_err("laundering is refused");
    assert!(err.to_string().contains("sandbox"), "{err}");
}

#[test]
fn the_zone_inspector_and_search_read_only_the_masked_projection() {
    let (game, log) = started(4242);
    let view = game.view_for(P0, &log);

    // Search finds own hand cards and public cards, never an opponent hand card.
    let own_hand = view.own_hand.first().expect("own hand");
    let hits = view.search(own_hand.name, &[P0], &[Zone::Hand]);
    assert!(hits.iter().any(|hit| hit.card.id == own_hand.id));
    let hidden: Vec<ObjectId> = game.state.player(P1).hand.clone();
    let every_id: Vec<ObjectId> = view
        .search("", &[], &[])
        .into_iter()
        .map(|hit| hit.card.id)
        .collect();
    assert!(
        hidden.iter().all(|id| !every_id.contains(id)),
        "an opponent hand card is not in any view hit"
    );
    assert_eq!(view.zone_of(hidden[0]), None);

    // A command-zone card reports the command zone, never the battlefield.
    let decks = [deck_a(), deck_b()];
    let specs = vec![
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
    ];
    let (mut game_b, mut log_b) =
        Game::new_commander_combat_logged(9, &decks, &specs).expect("bounded table");
    let mut guard = 0;
    while game_b.state.pending.is_some() {
        let owner = game_b.decision_owner().expect("owner");
        if let Ok(events) = game_b.submit_action(owner, Action::Mulligan { keep: true }) {
            log_b.extend(events);
        }
        guard += 1;
        assert!(guard < 40);
    }
    let view_b = game_b.view_for(P0, &log_b);
    let commander = view_b.own_command_zone.first().expect("commander");
    assert_eq!(view_b.zone_of(commander.id), Some(Zone::Command));
    let commander_hits = view_b.search("isamaru", &[P1], &[]);
    assert_eq!(
        commander_hits.first().map(|hit| hit.zone),
        Some(Zone::Command),
        "a command-zone card is never labeled battlefield"
    );
}
