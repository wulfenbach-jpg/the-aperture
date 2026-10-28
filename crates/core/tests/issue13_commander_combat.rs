//! Issue 13 bounded Commander-combat profile: serialized profile, ledger and
//! loss behavior. Behavior tests are written against the existing public
//! serialized interface so a failure is an assertion failure, not a compile
//! failure for a merely absent API.

mod common;

use aperture_core::state::{Object, StackItem};
use aperture_core::*;
use common::*;

const P2: PlayerId = 2;

fn submit(game: &mut Game, player: PlayerId, action: Action) -> Vec<Event> {
    match game.submit_action(player, action) {
        Ok(events) => events,
        Err(error) => panic!("expected P{player} action to be accepted: {error}"),
    }
}

fn forest_deck(name: &str) -> DeckList {
    DeckList::parse_practice(name, "40 Forest\n").expect("deck parses")
}

fn commander_setups() -> Vec<CommanderSetup> {
    vec![
        CommanderSetup {
            seat: P0,
            card: CardId::IsamaruHoundOfKonda,
        },
        CommanderSetup {
            seat: P1,
            card: CardId::IsamaruHoundOfKonda,
        },
    ]
}

/// A v-current replay whose serialized `profile` field explicitly requests the
/// bounded 40-life Commander-combat profile. The profile is authoritative: it
/// must not be ignored and silently reinterpreted as one of the 20-life
/// practice profiles.
fn serialized_commander_combat_replay() -> String {
    let replay = Replay::new_command_zone(
        1303,
        P0,
        vec![forest_deck("A"), forest_deck("B")],
        commander_setups(),
        AiSettings::default(),
    )
    .expect("command-zone replay");
    let mut json: serde_json::Value = serde_json::from_str(&replay.to_json()).expect("replay JSON");
    json.as_object_mut().expect("replay object").insert(
        "profile".to_string(),
        serde_json::json!("CommanderCombat40"),
    );
    serde_json::to_string(&json).expect("serialized replay")
}

#[test]
fn serialized_commander_combat_profile_reconstructs_40_life() {
    let text = serialized_commander_combat_replay();
    let loaded = Replay::from_json(&text).expect("declared bounded profile is supported");
    let (game, _) = loaded.run().expect("replay rebuilds");
    assert_eq!(
        game.state.player(P0).life,
        40,
        "the explicit CommanderCombat40 profile starts at 40, not a silent 20"
    );
    assert_eq!(game.state.player(P1).life, 40);
}

/// The profile/schema delta makes the previous serialized versions explicitly
/// unsupported. At the accepted base these hardcoded previous versions are the
/// *current* versions and load successfully, so this is a real assertion RED
/// for the version bump (not a compile failure).
#[test]
fn previous_replay_schema_version_is_rejected() {
    let replay = Replay::new_command_zone(
        1304,
        P0,
        vec![forest_deck("A"), forest_deck("B")],
        commander_setups(),
        AiSettings::default(),
    )
    .expect("command-zone replay");
    let mut json: serde_json::Value = serde_json::from_str(&replay.to_json()).unwrap();
    json.as_object_mut()
        .unwrap()
        .insert("version".into(), serde_json::json!(4));
    let text = serde_json::to_string(&json).unwrap();
    match Replay::from_json(&text) {
        Err(ReplayError::Version { got, expected }) => {
            assert_eq!(got, 4);
            assert_eq!(expected, aperture_core::replay::REPLAY_VERSION);
        }
        other => panic!("previous replay schema must be rejected explicitly, got {other:?}"),
    }
}

#[test]
fn previous_save_schema_version_is_rejected() {
    let replay = Replay::new_command_zone(
        1303,
        P0,
        vec![forest_deck("A"), forest_deck("B")],
        commander_setups(),
        AiSettings::default(),
    )
    .expect("command-zone replay");
    let game = replay.run().expect("replay runs").0;
    let save = SessionSave::capture(&game, replay, Vec::new(), false);
    let mut json: serde_json::Value = serde_json::from_str(&save.to_json()).unwrap();
    json.as_object_mut()
        .unwrap()
        .insert("version".into(), serde_json::json!(8));
    let text = serde_json::to_string(&json).unwrap();
    match SessionSave::from_json(&text) {
        Err(ReplayError::Version { got, expected }) => {
            assert_eq!(got, 8);
            assert_eq!(expected, aperture_core::replay::SAVE_VERSION);
        }
        other => panic!("previous save schema must be rejected explicitly, got {other:?}"),
    }
}

/// Named 20-life contracts are controls: the new profile must not change the
/// ordinary or accepted command-zone practice starting life.
#[test]
fn named_20_life_profiles_are_unchanged() {
    let decks = vec![forest_deck("A"), forest_deck("B")];
    let ordinary = Game::new(1306, &decks).expect("ordinary practice");
    assert_eq!(ordinary.state.player(P0).life, 20);
    assert_eq!(ordinary.state.player(P1).life, 20);
    assert!(ordinary.state.commanders.is_empty());

    let command = Game::new_command_zone(
        1307,
        &decks,
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
        ],
    )
    .expect("named command-zone practice");
    assert_eq!(command.state.player(P0).life, 20);
    assert_eq!(command.state.player(P1).life, 20);
    assert_eq!(command.state.commanders.len(), 2);
}

/// Ready a CommanderCombat40 game with P0's commander on the battlefield and
/// P1 as the only defending seat. Direct state setup is the same public-state
/// style the existing command-zone tests use; no new API is assumed.
fn ready_commander_combat(seed: u64) -> (Game, ObjectId) {
    let decks = vec![forest_deck("A"), forest_deck("B")];
    let specs = [
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
    ];
    let mut game = Game::new_commander_combat(seed, &decks, &specs).expect("bounded profile");
    game.state.mulligan = None;
    game.state.pending = None;
    let commander = game
        .state
        .commanders
        .iter()
        .find(|c| c.owner == P0)
        .expect("P0 commander registry")
        .object;
    game.state.players[P0 as usize]
        .command_zone
        .retain(|id| *id != commander);
    let object = game
        .state
        .objects
        .get_mut(&commander)
        .expect("commander object");
    object.zone = Zone::Battlefield;
    object.controller = P0;
    object.tapped = false;
    object.summoning_sick = false;
    game.state.turn.number = 2;
    game.state.turn.active = P0;
    (game, commander)
}

/// Declare the commander as the only attacker, then pass both priority rounds
/// (declare blockers + combat damage) so real combat damage is dealt.
fn attack_once(game: &mut Game, commander: ObjectId) -> Vec<Event> {
    attack_as(game, commander, P0, P1)
}

fn attack_as(
    game: &mut Game,
    commander: ObjectId,
    attacker: PlayerId,
    defender: PlayerId,
) -> Vec<Event> {
    {
        let object = game
            .state
            .objects
            .get_mut(&commander)
            .expect("commander object");
        object.tapped = false;
        object.summoning_sick = false;
    }
    game.state.turn.active = attacker;
    game.state.turn.step = Step::DeclareAttackers;
    game.state.turn.priority = None;
    game.state.turn.passed = 0;
    game.state.turn.attackers.clear();
    game.state.turn.blocked.clear();
    game.state.turn.blockers.clear();
    game.state.turn.combat_damage_done = false;
    game.state.turn.defender = None;
    game.state.pending = Some(Pending::DeclareAttackers { player: attacker });
    let mut log = submit(
        game,
        attacker,
        Action::DeclareAttackers {
            attackers: vec![commander],
            defender: Some(defender),
        },
    );
    let pass_round = game.state.live_count();
    for _ in 0..2 {
        for _ in 0..pass_round {
            if game.state.is_over() {
                return log;
            }
            let player = game.decision_owner().expect("a seat has priority");
            log.extend(submit(game, player, Action::Pass));
        }
    }
    log
}

/// CR 903.10a must be visible in the public projection and credit only combat
/// damage actually dealt to a player, keyed by the stable commander identity.
/// The base build ignores the concept entirely, so this is a genuine assertion
/// RED (the projection has no `commander_damage` field).
#[test]
fn commander_combat_damage_is_publicly_credited() {
    let (mut game, commander) = ready_commander_combat(1401);
    let commander_id = game
        .state
        .commanders
        .iter()
        .find(|c| c.owner == P0)
        .expect("registry")
        .id;
    let log = attack_once(&mut game, commander);
    assert_eq!(game.state.player(P1).life, 38, "2 combat damage was dealt");
    let view: serde_json::Value =
        serde_json::from_str(&game.view_for(P0, &log).to_json()).expect("view JSON");
    let rows = view
        .get("commander_damage")
        .and_then(|value| value.as_array())
        .expect("public commander damage projection exists");
    assert_eq!(rows.len(), 1, "one commander, one recipient: {rows:?}");
    assert_eq!(rows[0]["recipient"], serde_json::json!(P1));
    assert_eq!(rows[0]["amount"], serde_json::json!(2));
    assert_eq!(
        rows[0]["commander"],
        serde_json::json!(commander_id.0),
        "keyed by the stable CommanderId"
    );
}

/// Two-power hits: 20 cumulative damage is not a loss, the 11th hit reaches 22
/// and is. Exact 19/20/21 boundaries are covered separately after the minimal
/// ledger scaffold; this pure-combat case proves real damage reaches the SBA.
#[test]
fn twenty_two_commander_damage_is_a_loss() {
    let (mut game, commander) = ready_commander_combat(1402);
    for hit in 1..=10 {
        assert!(!game.state.is_over(), "no loss before the threshold");
        let _ = attack_once(&mut game, commander);
        assert_eq!(game.state.player(P1).life, 40 - 2 * hit, "hit {hit} life");
        assert!(!game.state.is_over(), "20 commander damage is not a loss");
    }
    let log = attack_once(&mut game, commander);
    assert_eq!(game.state.player(P1).life, 18);
    match &game.state.outcome {
        Some(Outcome::Win { winner, reason }) => {
            assert_eq!(*winner, P0);
            assert!(
                reason.contains("903.10a"),
                "commander-damage loss reason: {reason}"
            );
        }
        other => panic!("21 commander damage must end the game, got {other:?} after {log:?}"),
    }
}

/// Direct-restored ledger injected through the public serialized GameState
/// shape (unknown at the accepted base, parsed after the ledger delta). The
/// final damage still comes from one real combat hit.
fn game_with_p0_ledger(seed: u64, recipient: PlayerId, prior: u32) -> (Game, ObjectId) {
    let (game, commander) = ready_commander_combat(seed);
    let commander_id = game
        .state
        .commanders
        .iter()
        .find(|c| c.owner == P0)
        .expect("registry")
        .id
        .0;
    let mut value = serde_json::to_value(&game.state).expect("state JSON");
    value.as_object_mut().expect("state object").insert(
        "commander_damage".to_string(),
        serde_json::json!([{
            "commander": commander_id,
            "recipient": recipient,
            "amount": prior,
        }]),
    );
    let state: GameState = serde_json::from_value(value).expect("state parses");
    let game = Game::from_state(state).expect("state is valid");
    (game, commander)
}

/// CR 903.10a exact boundaries with a real 2-damage combat hit on top of a
/// direct-restored prior total: 17+2=19 no loss, 18+2=20 no loss, 19+2=21 loss,
/// 20+2=22 loss. At the accepted base no ledger/threshold exists, so the 21/22
/// cases fail behaviorally (RED), not by missing symbols.
#[test]
fn exact_commander_damage_boundaries() {
    for (prior, loses) in [(17, false), (18, false), (19, true), (20, true)] {
        let (mut game, commander) = game_with_p0_ledger(1600 + prior as u64, P1, prior);
        let log = attack_once(&mut game, commander);
        assert_eq!(
            game.state.player(P1).life,
            38,
            "one real 2-damage hit at prior {prior}"
        );
        if loses {
            match &game.state.outcome {
                Some(Outcome::Win { winner, reason }) => {
                    assert_eq!(*winner, P0, "prior {prior}");
                    assert!(reason.contains("903.10a"), "reason: {reason}");
                }
                other => panic!("prior {prior} + 2 must lose, got {other:?} after {log:?}"),
            }
            assert_eq!(
                log.iter()
                    .filter(|event| matches!(event, Event::GameOver { .. }))
                    .count(),
                1,
                "exactly one terminal event at prior {prior}"
            );
        } else {
            assert!(
                game.state.outcome.is_none(),
                "prior {prior} + 2 is below 21: {log:?}"
            );
        }
    }
}

/// A modeled current-control change must not change the ledger bucket: the key
/// is the stable CommanderId + recipient, never the controller or ObjectId.
/// P0 owns the commander, P1 controls it, P2 is dealt the damage.
#[test]
fn commander_damage_identity_survives_a_modeled_control_change() {
    let decks = vec![forest_deck("A"), forest_deck("B"), forest_deck("C")];
    let specs = [
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
        (P2, CardId::IsamaruHoundOfKonda),
    ];
    let mut game = Game::new_commander_combat(1701, &decks, &specs).expect("bounded profile");
    game.state.mulligan = None;
    game.state.pending = None;
    let commander_id = game
        .state
        .commanders
        .iter()
        .find(|c| c.owner == P0)
        .expect("registry")
        .id
        .0;
    let commander = game
        .state
        .commanders
        .iter()
        .find(|c| c.owner == P0)
        .expect("registry")
        .object;
    game.state.players[P0 as usize]
        .command_zone
        .retain(|id| *id != commander);
    {
        let object = game.state.objects.get_mut(&commander).expect("commander");
        object.zone = Zone::Battlefield;
        object.controller = P1;
        object.tapped = false;
        object.summoning_sick = false;
    }
    let mut value = serde_json::to_value(&game.state).expect("state JSON");
    value.as_object_mut().expect("state object").insert(
        "commander_damage".to_string(),
        serde_json::json!([{
            "commander": commander_id,
            "recipient": P2,
            "amount": 19,
        }]),
    );
    let state: GameState = serde_json::from_value(value).expect("state parses");
    let mut game = Game::from_state(state).expect("state is valid");
    game.state.turn.number = 2;
    game.state.turn.active = P1;
    let log = attack_as(&mut game, commander, P1, P2);
    assert!(game.state.player(P2).lost, "P2 took 21 commander damage");
    assert!(game.state.outcome.is_none(), "P0 and P1 remain live");
    assert!(log.iter().any(|event| matches!(
        event,
        Event::CommanderDamage {
            commander: _,
            recipient: r,
            amount: 2,
            total: 21,
            ..
        } if *r == P2
    )));
    assert_eq!(
        log.iter()
            .filter(|event| matches!(event, Event::StateBasedAction { description } if description.contains("P2 loses") && description.contains("903.10a")))
            .count(),
        1
    );
    game.state
        .validate()
        .expect("ledger stays valid after the loss");
}

/// Malformed ledger rows must be typed refusals, never silently ignored or
/// laundered through serde defaults. At the accepted base the field is unknown
/// and ignored, which is the RED.
#[test]
fn malformed_commander_damage_ledgers_are_refused() {
    let (game, _) = ready_commander_combat(1801);
    let commander = game.state.commanders[0].id.0;
    let base = serde_json::to_value(&game.state).expect("state JSON");
    let with_rows = |rows: serde_json::Value| {
        let mut value = base.clone();
        value
            .as_object_mut()
            .expect("state object")
            .insert("commander_damage".to_string(), rows);
        let state: GameState = serde_json::from_value(value).expect("state shape");
        state.validate()
    };
    for (label, rows) in [
        (
            "unknown commander",
            serde_json::json!([{"commander": 999, "recipient": P1, "amount": 2}]),
        ),
        (
            "duplicate row",
            serde_json::json!([
                {"commander": commander, "recipient": P1, "amount": 2},
                {"commander": commander, "recipient": P1, "amount": 3}
            ]),
        ),
        (
            "zero amount",
            serde_json::json!([{"commander": commander, "recipient": P1, "amount": 0}]),
        ),
        (
            "settled live 21",
            serde_json::json!([{"commander": commander, "recipient": P1, "amount": 21}]),
        ),
    ] {
        assert!(
            with_rows(rows).is_err(),
            "malformed ledger case must be refused: {label}"
        );
    }
}

/// Two independent CommanderIds below 21 dealt to the same recipient do not
/// pool; 20+20 is not a loss. This is a control once the ledger exists and a
/// silent-accept RED at the accepted base only for malformed shapes, so it is
/// phrased as the eventual contract.
#[test]
fn independent_commanders_do_not_pool() {
    let decks = vec![forest_deck("A"), forest_deck("B"), forest_deck("C")];
    let specs = [
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
        (P2, CardId::IsamaruHoundOfKonda),
    ];
    let mut game = Game::new_commander_combat(1802, &decks, &specs).expect("bounded profile");
    game.state.mulligan = None;
    game.state.pending = None;
    let first = game.state.commanders[0].id.0;
    let second = game.state.commanders[1].id.0;
    let mut value = serde_json::to_value(&game.state).expect("state JSON");
    value.as_object_mut().expect("state object").insert(
        "commander_damage".to_string(),
        serde_json::json!([
            {"commander": first, "recipient": P2, "amount": 20},
            {"commander": second, "recipient": P2, "amount": 20}
        ]),
    );
    let state: GameState = serde_json::from_value(value).expect("state shape");
    let mut game = Game::from_state(state).expect("20+20 from different commanders is valid");
    game.state.turn.number = 2;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    let _ = submit(&mut game, P0, Action::Pass);
    if !game.state.is_over() {
        let _ = submit(&mut game, P1, Action::Pass);
    }
    if !game.state.is_over() {
        let _ = submit(&mut game, P2, Action::Pass);
    }
    assert!(
        game.state.outcome.is_none(),
        "independent commanders must not pool into a 21-damage loss"
    );
}

fn provenance_decks() -> Vec<DeckList> {
    vec![forest_deck("A"), forest_deck("B")]
}

fn provenance_specs() -> Vec<CommanderSetup> {
    commander_setups()
}

fn provenance_game(seed: u64) -> (Game, Replay) {
    let decks = provenance_decks();
    let game = Game::new_commander_combat(
        seed,
        &decks,
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
        ],
    )
    .expect("combat game");
    let replay = Replay::new_commander_combat(
        seed,
        game.state.first_player,
        decks,
        provenance_specs(),
        AiSettings::default(),
    )
    .expect("combat replay");
    (game, replay)
}

/// The external retired-id probe's exact defect: a fabricated retired
/// CommanderId must never launder a ledger row. At the accepted frontier the
/// retired list only checks sortedness/disjointness, so this validates Ok (RED).
#[test]
fn fabricated_retired_commander_identity_is_refused() {
    let (game, _) = provenance_game(2001);
    let mut value = serde_json::to_value(&game.state).expect("state JSON");
    value
        .as_object_mut()
        .expect("state object")
        .insert("retired_commanders".to_string(), serde_json::json!([999]));
    value.as_object_mut().expect("state object").insert(
        "commander_damage".to_string(),
        serde_json::json!([{ "commander": 999, "recipient": P1, "amount": 5 }]),
    );
    let state: GameState = serde_json::from_value(value).expect("state shape");
    assert!(
        state.validate().is_err(),
        "fabricated retired CommanderId must be refused"
    );
}

/// The same forged identity through a direct-restored sandbox save: the save
/// must refuse provenance laundering, while an untampered sandbox save remains
/// valid under its documented contract.
#[test]
fn forged_retired_identity_is_refused_by_sandbox_save_load() {
    let (game, replay) = provenance_game(2002);
    let clean = SessionSave::capture(&game, replay.clone(), Vec::new(), true);
    assert!(
        SessionSave::from_json(&clean.to_json()).is_ok(),
        "untampered direct-restored sandbox save is valid"
    );

    let mut forged_state = game.state.clone();
    forged_state.retired_commanders = vec![CommanderId(999)];
    forged_state.commander_damage = vec![CommanderDamageEntry {
        commander: CommanderId(999),
        recipient: P1,
        amount: 5,
    }];
    let forged = SessionSave {
        version: aperture_core::replay::SAVE_VERSION,
        digest: aperture_core::digest::digest(&forged_state),
        game: forged_state,
        seats: clean.seats.clone(),
        ai: clean.ai.clone(),
        replay,
        log: Vec::new(),
        sandbox_modified: true,
        sandbox: aperture_core::SandboxRecord::default(),
    };
    assert!(
        SessionSave::from_json(&forged.to_json()).is_err(),
        "sandbox save must not launder a fabricated retired CommanderId"
    );
}

/// A live, in-range CommanderId cannot be listed as retired.
#[test]
fn live_commander_cannot_be_listed_as_retired() {
    let (game, _) = provenance_game(2003);
    let live = game.state.commanders[0].id;
    let mut value = serde_json::to_value(&game.state).expect("state JSON");
    value.as_object_mut().expect("state object").insert(
        "retired_commanders".to_string(),
        serde_json::json!([live.0]),
    );
    let state: GameState = serde_json::from_value(value).expect("state shape");
    assert!(state.validate().is_err(), "live {live} cannot be retired");
}

/// A real departure retires exactly that CommanderId, keeps a historic damage
/// row to a surviving recipient, and leaves the live/retired partition complete.
#[test]
fn real_departure_retires_identity_and_keeps_historic_row() {
    let (game, _) = provenance_game(2004);
    let commander_id = game.state.commanders[0].id;
    let mut value = serde_json::to_value(&game.state).expect("state JSON");
    value.as_object_mut().expect("state object").insert(
        "commander_damage".to_string(),
        serde_json::json!([{ "commander": commander_id.0, "recipient": P1, "amount": 5 }]),
    );
    let state: GameState = serde_json::from_value(value).expect("state shape");
    let mut game = Game::from_state(state).expect("valid ledger");
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.number = 2;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    game.state.players[P0 as usize].life = 0;
    let _ = submit(&mut game, P0, Action::Pass);
    assert!(game.state.player(P0).lost);
    assert_eq!(game.state.retired_commanders, vec![commander_id]);
    assert!(
        game.state
            .commander_damage
            .iter()
            .any(|entry| entry.commander == commander_id
                && entry.recipient == P1
                && entry.amount == 5),
        "historic row to the survivor must remain"
    );
    let live: Vec<u32> = game.state.commanders.iter().map(|c| c.id.0).collect();
    let mut partition = live.clone();
    partition.extend(game.state.retired_commanders.iter().map(|id| id.0));
    partition.sort_unstable();
    assert_eq!(
        partition,
        (1..=game.state.seat_count() as u32).collect::<Vec<_>>()
    );
    assert!(game.state.validate().is_ok());
}

/// A departed identity omitted from `retired_commanders` breaks the partition
/// and must be refused. At the accepted frontier the union is not checked, so
/// this validates Ok (RED).
#[test]
fn departed_identity_omitted_from_retired_is_refused() {
    let (game, _) = provenance_game(2005);
    let departed_id = game.state.commanders[0].id;
    let mut value = serde_json::to_value(&game.state).expect("state JSON");
    value.as_object_mut().expect("state object").insert(
        "commander_damage".to_string(),
        serde_json::json!([{ "commander": departed_id.0, "recipient": P1, "amount": 5 }]),
    );
    let state: GameState = serde_json::from_value(value).expect("state shape");
    let mut game = Game::from_state(state).expect("valid ledger");
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.number = 2;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    game.state.players[P0 as usize].life = 0;
    let _ = submit(&mut game, P0, Action::Pass);
    assert!(game.state.player(P0).lost);
    let mut broken = game.state.clone();
    broken.retired_commanders.clear();
    broken.commander_damage.clear();
    assert!(
        broken.validate().is_err(),
        "the live/retired partition must cover every original commander identity"
    );
}

fn plains_deck(name: &str) -> DeckList {
    DeckList::parse_practice(name, "40 Plains\n").expect("plains deck")
}

fn policy_action(game: &Game) -> Action {
    if let Some(pending) = &game.state.pending {
        match pending {
            Pending::Mulligan { .. } => return Action::Mulligan { keep: true },
            Pending::DeclareAttackers { player } => {
                let attackers = if *player == P0 {
                    game.legal_attackers(P0)
                } else {
                    Vec::new()
                };
                let defender = if attackers.is_empty() {
                    None
                } else {
                    game.state.next_live_seat_after(*player)
                };
                return Action::DeclareAttackers {
                    attackers,
                    defender,
                };
            }
            Pending::DeclareBlockers { .. } => {
                return Action::DeclareBlockers { blocks: Vec::new() };
            }
            _ => {
                if let Some(legal) = game.legal_actions().into_iter().next() {
                    return legal.action;
                }
            }
        }
    }
    if game.state.pending.is_none() && game.decision_owner() == Some(P0) {
        let legal = game.legal_actions();
        if let Some(land) = legal
            .iter()
            .find(|candidate| matches!(candidate.action, Action::PlayLand { .. }))
        {
            return land.action.clone();
        }
        if let Some(cast) = legal.iter().find(|candidate| {
            matches!(&candidate.action, Action::CastSpell { card, .. } if game.state.object(*card).commander.is_some())
        }) {
            return cast.action.clone();
        }
    }
    Action::Pass
}

fn drive_until(game: &mut Game, replay: &mut Replay, log: &mut Vec<Event>, target: u32) -> bool {
    for _ in 0..20_000 {
        let damage = game
            .state
            .commander_damage
            .iter()
            .filter(|entry| entry.recipient == P1)
            .map(|entry| entry.amount)
            .max()
            .unwrap_or(0);
        if damage >= target || game.state.is_over() {
            return true;
        }
        let Some(player) = game.decision_owner() else {
            return false;
        };
        let action = policy_action(game);
        let step = ReplayStep {
            player,
            revision: game.state.revision,
            decision: game.state.decision,
            action: action.clone(),
        };
        match game.submit_action(player, action) {
            Ok(events) => log.extend(events),
            Err(error) => panic!("accepted-action driver rejected {step:?}: {error}"),
        }
        replay.record(step);
        replay.final_digest = game.digest();
    }
    false
}

/// Real accepted actions reach 20 commander damage; a non-sandbox save/replay
/// round-trips mid-ledger, and continuing with one more real hit settles the
/// 21-damage loss exactly once with a typed credit. This is persistence
/// evidence, not a direct-restored state.
#[test]
fn accepted_action_midledger_save_and_replay_roundtrip() {
    let decks = vec![plains_deck("A"), plains_deck("B")];
    let specs = [
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
    ];
    let mut game = Game::new_commander_combat(2101, &decks, &specs).expect("combat game");
    let setups: Vec<CommanderSetup> = specs
        .iter()
        .map(|(seat, card)| CommanderSetup {
            seat: *seat,
            card: *card,
        })
        .collect();
    let mut replay = Replay::new_commander_combat(
        2101,
        game.state.first_player,
        decks.clone(),
        setups,
        AiSettings::default(),
    )
    .expect("replay setup");
    let mut log = Vec::new();
    assert!(
        drive_until(&mut game, &mut replay, &mut log, 20),
        "reach 20"
    );
    assert_eq!(
        game.state
            .commander_damage
            .iter()
            .find(|entry| entry.recipient == P1)
            .map(|entry| entry.amount),
        Some(20),
        "exactly 20 before the saved boundary"
    );
    let save = SessionSave::capture(&game, replay.clone(), log.clone(), false);
    let loaded = SessionSave::from_json(&save.to_json())
        .expect("non-sandbox mid-ledger save must round-trip");
    assert_eq!(loaded.game, game.state);
    assert_eq!(loaded.digest, game.digest());

    let mut loaded_game = loaded.game();
    let mut loaded_replay = loaded.replay.clone();
    let mut loaded_log = Vec::new();
    assert!(
        drive_until(&mut loaded_game, &mut loaded_replay, &mut loaded_log, 21),
        "reach the terminal boundary"
    );
    match &loaded_game.state.outcome {
        Some(Outcome::Win { winner, reason }) => {
            assert_eq!(*winner, P0);
            assert!(reason.contains("903.10a"), "reason: {reason}");
        }
        other => panic!("accepted-action 21 loss missing: {other:?}"),
    }
    assert_eq!(
        loaded_log
            .iter()
            .filter(|event| matches!(event, Event::GameOver { .. }))
            .count(),
        1
    );
    assert!(loaded_log.iter().any(|event| matches!(
        event,
        Event::CommanderDamage {
            recipient: r,
            amount: 2,
            total,
            ..
        } if *r == P1 && *total >= 21
    )));
}

fn add_card(
    game: &mut Game,
    owner: PlayerId,
    zone: Zone,
    card: CardId,
    controller: PlayerId,
) -> ObjectId {
    let id = ObjectId(game.state.next_object);
    game.state.next_object += 1;
    game.state.objects.insert(
        id,
        Object {
            id,
            card,
            owner,
            controller,
            zone,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: aperture_core::counters::Counters::default(),
            paired_with: None,
        },
    );
    match zone {
        Zone::Hand => game.state.players[owner as usize].hand.push(id),
        Zone::Battlefield => {}
        _ => {}
    }
    id
}

/// Noncombat damage never credits a commander: P1's Shock to P0 changes life
/// only, with no ledger row and no typed CommanderDamage event.
#[test]
fn noncombat_damage_never_credits_the_ledger() {
    let (mut game, _) = ready_commander_combat(2201);
    let _mountain = add_card(&mut game, P1, Zone::Battlefield, CardId::Mountain, P1);
    let shock = add_card(&mut game, P1, Zone::Hand, CardId::Shock, P1);
    game.state.turn.number = 2;
    game.state.turn.active = P1;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P1);
    let mut log = submit(
        &mut game,
        P1,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P0)],
        },
    );
    log.extend(submit(&mut game, P1, Action::Pass));
    log.extend(submit(&mut game, P0, Action::Pass));
    assert_eq!(
        game.state.player(P0).life,
        38,
        "Shock dealt 2 noncombat damage"
    );
    assert!(
        game.state.commander_damage.is_empty(),
        "no commander credit"
    );
    assert!(
        !log.iter()
            .any(|event| matches!(event, Event::CommanderDamage { .. })),
        "no typed commander-damage event for noncombat damage"
    );
}

/// Combat damage to a creature never credits the player ledger, even when the
/// source is a commander: blocked Isamaru damages the blocking Bear only.
#[test]
fn commander_damage_to_a_creature_never_credits_a_player() {
    let (mut game, commander) = ready_commander_combat(2202);
    let bear = add_card(&mut game, P1, Zone::Battlefield, CardId::GrizzlyBears, P1);
    game.state.turn.step = Step::DeclareAttackers;
    game.state.turn.priority = None;
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    let mut log = submit(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![commander],
            defender: Some(P1),
        },
    );
    log.extend(submit(&mut game, P0, Action::Pass));
    log.extend(submit(&mut game, P1, Action::Pass));
    assert!(
        matches!(
            game.state.pending,
            Some(Pending::DeclareBlockers { player: P1 })
        ),
        "blocker declaration is pending"
    );
    log.extend(submit(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: bear,
                attacker: commander,
            }],
        },
    ));
    for _ in 0..2 {
        if game.state.is_over() {
            break;
        }
        let player = game.decision_owner().expect("priority");
        log.extend(submit(&mut game, player, Action::Pass));
    }
    assert_eq!(game.state.player(P1).life, 40, "no damage to the player");
    assert!(game.state.commander_damage.is_empty(), "no player credit");
    assert!(
        !log.iter()
            .any(|event| matches!(event, Event::CommanderDamage { .. }))
    );
}

/// A real Unsummon + CR 903.9b command-zone replacement + recast must keep the
/// same (CommanderId, recipient) bucket across a fresh ObjectId.
#[test]
fn zone_return_and_recast_keep_the_same_commander_bucket() {
    let (mut game, first_object) = ready_commander_combat(2203);
    let commander_id = game.state.commanders[0].id;
    let _ = attack_once(&mut game, first_object);
    assert_eq!(
        game.state
            .commander_damage
            .iter()
            .find(|entry| entry.commander == commander_id && entry.recipient == P1)
            .map(|entry| entry.amount),
        Some(2)
    );
    let life_after_first = game.state.player(P1).life;

    let _island = add_card(&mut game, P1, Zone::Battlefield, CardId::Island, P1);
    let unsummon = add_card(&mut game, P1, Zone::Hand, CardId::Unsummon, P1);
    game.state.turn.number = 2;
    game.state.turn.active = P1;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P1);
    let mut log = submit(
        &mut game,
        P1,
        Action::CastSpell {
            card: unsummon,
            targets: vec![Target::Permanent(first_object)],
        },
    );
    log.extend(submit(&mut game, P1, Action::Pass));
    log.extend(submit(&mut game, P0, Action::Pass));
    assert!(
        matches!(
            game.state.pending,
            Some(Pending::CommanderReplacement { .. })
        ),
        "903.9b replacement choice is pending: {log:?}"
    );
    log.extend(submit(
        &mut game,
        P0,
        Action::AnswerCommanderReplacement {
            to_command_zone: true,
        },
    ));
    let second_object = game.state.players[P0 as usize].command_zone[0];
    assert_ne!(
        second_object, first_object,
        "zone change creates a fresh ObjectId"
    );
    assert_eq!(game.state.commanders[0].object, second_object);

    let _plains = add_card(&mut game, P0, Zone::Battlefield, CardId::Plains, P0);
    game.state.turn.number = 3;
    game.state.turn.active = P0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    let mut cast_log = submit(
        &mut game,
        P0,
        Action::CastSpell {
            card: second_object,
            targets: Vec::new(),
        },
    );
    cast_log.extend(submit(&mut game, P0, Action::Pass));
    cast_log.extend(submit(&mut game, P1, Action::Pass));
    let recast_id = game
        .state
        .battlefield(P0)
        .into_iter()
        .find(|object| object.commander == Some(commander_id))
        .expect("recast commander on the battlefield")
        .id;
    assert_ne!(recast_id, first_object);
    {
        let object = game.state.objects.get_mut(&recast_id).expect("commander");
        object.tapped = false;
        object.summoning_sick = false;
    }
    let _ = attack_once(&mut game, recast_id);
    assert_eq!(game.state.player(P1).life, life_after_first - 2);
    let entry = game
        .state
        .commander_damage
        .iter()
        .find(|entry| entry.commander == commander_id && entry.recipient == P1)
        .expect("same bucket after zone return/recast");
    assert_eq!(entry.amount, 4, "damage accumulates in the stable bucket");
    assert_eq!(
        game.state.commander_damage.len(),
        1,
        "no ObjectId-keyed row"
    );
}

/// SessionSave must bind each live CommanderId to the ORIGINAL replay setup
/// entry at index id-1 (owner+card), not merely check a 1..=n universe. Swapping
/// two live ids keeps the partition valid, so the accepted frontier wrongly
/// loads it (genuine assertion RED).
#[test]
fn sandbox_save_rejects_swapped_live_commander_identity_binding() {
    let (game, replay) = provenance_game(2301);
    let mut swapped = game.state.clone();
    for registry in &mut swapped.commanders {
        registry.id = CommanderId(3 - registry.id.0);
        swapped
            .objects
            .get_mut(&registry.object)
            .expect("commander object")
            .commander = Some(registry.id);
    }
    swapped
        .validate()
        .expect("a swapped 1..=2 universe is structurally valid");
    let forged = Game::from_state(swapped).expect("structurally valid state");
    let save = SessionSave::capture(&forged, replay, Vec::new(), true);
    assert!(
        SessionSave::from_json(&save.to_json()).is_err(),
        "sandbox save must bind commander identity to the original setup owner/card"
    );
}

/// Controls: the normal setup and a REVERSED setup order both load, because ids
/// enumerate the supplied replay setup order, not seat+1.
#[test]
fn sandbox_save_accepts_normal_and_reversed_original_setup_binding() {
    let (game, replay) = provenance_game(2302);
    let normal = SessionSave::capture(&game, replay, Vec::new(), true);
    SessionSave::from_json(&normal.to_json()).expect("normal setup binds");

    let decks = provenance_decks();
    let reversed_setup = vec![
        CommanderSetup {
            seat: P1,
            card: CardId::IsamaruHoundOfKonda,
        },
        CommanderSetup {
            seat: P0,
            card: CardId::IsamaruHoundOfKonda,
        },
    ];
    let specs: Vec<_> = reversed_setup.iter().map(|s| (s.seat, s.card)).collect();
    let (reversed_game, log) =
        Game::new_commander_combat_logged(2303, &decks, &specs).expect("reversed setup game");
    let reversed_replay = Replay::new_commander_combat(
        2303,
        reversed_game.state.first_player,
        decks,
        reversed_setup,
        AiSettings::default(),
    )
    .expect("reversed replay");
    let reversed_save = SessionSave::capture(&reversed_game, reversed_replay, log, true);
    SessionSave::from_json(&reversed_save.to_json())
        .expect("reversed original setup order is valid");
}

fn cast_fog(game: &mut Game, seed_tag: u64) -> Vec<Event> {
    let _forest = add_card(game, P0, Zone::Battlefield, CardId::Forest, P0);
    let fog = add_card(game, P0, Zone::Hand, CardId::Fog, P0);
    game.state.turn.number = 2;
    game.state.turn.active = P0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    let mut log = submit(
        game,
        P0,
        Action::CastSpell {
            card: fog,
            targets: Vec::new(),
        },
    );
    log.extend(submit(game, P0, Action::Pass));
    log.extend(submit(game, P1, Action::Pass));
    let _ = seed_tag;
    log
}

/// Pre-Fog behavioral RED: with only the card scaffold, an accepted-action Fog
/// cast must still leave combat damage on the player and commander ledger.
/// The attack log is retained so the GREEN typed-event assertions cannot repeat
/// the 35 log-capture defect.
#[test]
fn fog_prevents_combat_damage_before_it_reaches_life_or_ledger() {
    let (mut game, commander) = ready_commander_combat(2401);
    let _forest = add_card(&mut game, P0, Zone::Battlefield, CardId::Forest, P0);
    let fog = add_card(&mut game, P0, Zone::Hand, CardId::Fog, P0);
    game.state.turn.number = 2;
    game.state.turn.active = P0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    let mut log = submit(
        &mut game,
        P0,
        Action::CastSpell {
            card: fog,
            targets: Vec::new(),
        },
    );
    log.extend(submit(&mut game, P0, Action::Pass));
    log.extend(submit(&mut game, P1, Action::Pass));
    log.extend(attack_once(&mut game, commander));
    assert_eq!(
        game.state.player(P1).life,
        40,
        "prevented damage must not reduce life"
    );
    assert!(
        game.state.commander_damage.is_empty(),
        "no commander credit"
    );
    assert!(log.iter().any(|event| matches!(
        event,
        Event::PreventAllCombatDamage {
            turn: 2,
            card: CardId::Fog
        }
    )));
    assert!(log.iter().any(|event| matches!(
        event,
        Event::CombatDamagePrevented {
            recipient: Some(P1),
            amount: 2,
            ..
        }
    )));
}

#[test]
fn fog_prevents_combat_damage_to_creatures() {
    let (mut game, commander) = ready_commander_combat(2402);
    let _ = cast_fog(&mut game, 2402);
    let bear = add_card(&mut game, P1, Zone::Battlefield, CardId::GrizzlyBears, P1);
    game.state.turn.step = Step::DeclareAttackers;
    game.state.turn.priority = None;
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    let mut log = submit(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![commander],
            defender: Some(P1),
        },
    );
    log.extend(submit(&mut game, P0, Action::Pass));
    log.extend(submit(&mut game, P1, Action::Pass));
    log.extend(submit(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: bear,
                attacker: commander,
            }],
        },
    ));
    for _ in 0..2 {
        if game.state.is_over() {
            break;
        }
        let p = game.decision_owner().expect("priority");
        log.extend(submit(&mut game, p, Action::Pass));
    }
    let commander_object = game.state.object(commander);
    let bear_object = game.state.object(bear);
    assert_eq!(
        commander_object.damage, 0,
        "combat damage to creature prevented"
    );
    assert_eq!(bear_object.damage, 0, "combat damage to creature prevented");
    assert_eq!(game.state.player(P1).life, 40);
    assert!(game.state.commander_damage.is_empty());
    assert!(log.iter().any(|e| matches!(
        e,
        Event::CombatDamagePrevented {
            permanent: Some(_),
            ..
        }
    )));
}

#[test]
fn fog_does_not_prevent_noncombat_damage() {
    let (mut game, _) = ready_commander_combat(2403);
    let _ = cast_fog(&mut game, 2403);
    let _mountain = add_card(&mut game, P1, Zone::Battlefield, CardId::Mountain, P1);
    let shock = add_card(&mut game, P1, Zone::Hand, CardId::Shock, P1);
    game.state.turn.number = 3;
    game.state.turn.active = P1;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P1);
    let mut log = submit(
        &mut game,
        P1,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P0)],
        },
    );
    log.extend(submit(&mut game, P1, Action::Pass));
    log.extend(submit(&mut game, P0, Action::Pass));
    assert_eq!(
        game.state.player(P0).life,
        38,
        "Fog does not prevent Shock: 40 - 2"
    );
    assert!(
        !log.iter()
            .any(|e| matches!(e, Event::CombatDamagePrevented { .. }))
    );
}

#[test]
fn fog_shield_expires_after_cleanup() {
    let (mut game, commander) = ready_commander_combat(2404);
    let _ = cast_fog(&mut game, 2404);
    let _ = attack_once(&mut game, commander);
    assert_eq!(game.state.player(P1).life, 40);
    let mut guard = 0;
    while (game.state.turn.number < 3 || game.state.turn.active != P0) && guard < 500 {
        if game.state.is_over() {
            break;
        }
        let p = game.decision_owner().expect("priority");
        let action = match &game.state.pending {
            Some(Pending::DeclareAttackers { .. }) => Action::DeclareAttackers {
                attackers: Vec::new(),
                defender: None,
            },
            Some(Pending::DeclareBlockers { .. }) => Action::DeclareBlockers { blocks: Vec::new() },
            Some(_) => game
                .legal_actions()
                .into_iter()
                .next()
                .map(|a| a.action)
                .unwrap_or(Action::Pass),
            None => Action::Pass,
        };
        let _ = submit(&mut game, p, action);
        guard += 1;
    }
    assert!(guard < 500, "reached P0's next turn");
    assert_eq!(
        game.state.prevent_all_combat_damage_turn, None,
        "shield expired in cleanup"
    );
    let _ = attack_once(&mut game, commander);
    assert_eq!(
        game.state.player(P1).life,
        38,
        "next turn combat damage is normal"
    );
}

#[test]
fn fog_shield_survives_sandbox_save_and_malformed_shield_is_refused() {
    let (mut game, _) = ready_commander_combat(2405);
    let _ = cast_fog(&mut game, 2405);
    assert_eq!(game.state.prevent_all_combat_damage_turn, Some(2));
    let (_, replay) = provenance_game(2405);
    let save = SessionSave::capture(&game, replay, Vec::new(), true);
    let loaded = SessionSave::from_json(&save.to_json()).expect("mid-shield sandbox save loads");
    assert_eq!(loaded.game.prevent_all_combat_damage_turn, Some(2));

    let mut malformed = game.state.clone();
    malformed.prevent_all_combat_damage_turn = Some(99);
    assert!(
        malformed.validate().is_err(),
        "stale-turn shield must be refused"
    );
}

#[test]
fn fog_is_a_sourced_catalog_card() {
    let fog = catalog::by_name("Fog").expect("Fog is supported");
    assert_eq!(fog.name(), "Fog");
    assert_eq!(fog.def().cost, Some("{G}"));
    assert!(fog.def().is_instant());
    // The pool grew from 22 to 28 when issue 14 added the ability-bearing
    // starter cards, to 31 when issue 15 added the replacement/prevention/layer
    // starter cards, to 79 when issue 17 added the 47 vanilla starter
    // creatures plus Jasmine Boreal, to 85 with the Krenko token package, to 86 with Talrand, to 88 with Tatyova/Jaddi Offshoot, to 93 with the cross-corpus package, to 97 with the slice-4 Ayula/Isamaru package (Ordinary Bear, Ashcoat Bear, Owlbear, Soul Warden), to 103 with the slice-5 mill/counter/ritual package, and to 106 with the slice-6 death-watcher package (Blood Artist, Zulaport Cutthroat, Hedron Crab);
    // Fog's own sourced semantics are unchanged.
    // ... and to 129 with the slice-15 Bear-token package (Ayula's Influence,
    // Bearscape, Mother Bear), to 131 with the slice-16 pair (Fade from
    // History, Little Bear), to 134 with the slice-17 death-history trio, to
    // 135 with the slice-18 commander Ayula, Queen Among Bears, and to 136 with
    // the slice-19 attack-trigger bear Spectral Bears.
    // ... and to 139 with the slice-22 enchantment Dancing from Dark to Dawn,
    // and to 140 with the slice-23 Bear lord Goreclaw, Terror of Qal Sisma, and
    // to 143 with slice 24's Surrak and Goreclaw, slice 25's Exuberant Wolfbear
    // and slice 26's Ursine Champion, to 144 with slice 28's Circle of the
    // Moon Druid, to 145 with slice 29's Stuffed Bear, and to 147 with slice
    // 30's Drover Grizzly and slice 31's Beorn the Fierce, and to 148 with
    // slice 32's Kamahl's Summons, and to 149 with slice 33's Beorn's
    // Hospitality, and to 151 with slice 37's Druid's Familiar, and to 152 with
    // slice 38's Spider-Ham, Peter Porker, and to 160 with the Ayula-list cards
    // of slices 41-46 (the last being Thijarian Witness).
    assert_eq!(aperture_core::catalog::pool_size(), 160);
    assert!(aperture_core::catalog::by_name("Prodigal Sorcerer").is_some());
    assert!(aperture_core::catalog::by_name("Festering Goblin").is_some());
}

/// CR 800.4a step 4: a survivor-owned represented card still controlled by the
/// departing player must be exiled, and a represented stack card must not leave
/// a phantom stack entry. At the current bytes the blanket
/// `stack.retain(controller != p)` drops the entry while the object stays in
/// Stack, so validation fails (RED).
#[test]
fn departing_controller_stack_card_is_exiled_without_a_phantom_entry() {
    let (mut game, _) = ready_commander_combat(2601);
    let shock = add_card(&mut game, P1, Zone::Stack, CardId::Shock, P0);
    game.state.stack.push(StackItem {
        object: shock,
        card: CardId::Shock,
        controller: P0,
        targets: vec![Target::Player(P1)],
        modes: Vec::new(),
        affected: None,
        dead_object: None,
        source_pt_lki: None,
        search_amount: 0,
        cursor: 0,
        entry: StackEntry::Spell,
        trigger_answered: false,
        spell_mana_value: 0,
    });
    game.state.players[P0 as usize].life = 0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    let _ = submit(&mut game, P0, Action::Pass);
    assert!(game.state.player(P0).lost);
    assert_eq!(
        game.state.object(shock).zone,
        Zone::Exile,
        "still-controlled survivor-owned card is exiled"
    );
    assert!(game.state.stack.is_empty(), "no phantom stack entry");
    assert!(
        game.state.validate().is_ok(),
        "state validates after 800.4a exile"
    );
}

/// CR 800.4a step 4 for a battlefield permanent: survivor-owned but still
/// controlled by the departing player must be exiled, not left under a lost
/// controller. Current leave_game leaves it on the battlefield (RED).
#[test]
fn departing_controller_battlefield_permanent_is_exiled() {
    let (mut game, _) = ready_commander_combat(2602);
    let bear = add_card(&mut game, P1, Zone::Battlefield, CardId::GrizzlyBears, P0);
    game.state.players[P0 as usize].life = 0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    let _ = submit(&mut game, P0, Action::Pass);
    assert!(game.state.player(P0).lost);
    assert_eq!(
        game.state.object(bear).zone,
        Zone::Exile,
        "still-controlled survivor-owned permanent is exiled"
    );
    assert!(game.state.validate().is_ok());
}

#[test]
fn simultaneous_losses_settle_once_in_seat_order() {
    let decks = vec![plains_deck("A"), plains_deck("B"), plains_deck("C")];
    let specs = [
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
        (P2, CardId::IsamaruHoundOfKonda),
    ];
    let mut game = Game::new_commander_combat(2701, &decks, &specs).expect("3-seat game");
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.number = 2;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.active = P0;
    game.state.turn.priority = Some(P0);
    game.state.players[P0 as usize].life = 0;
    game.state.players[P1 as usize].drew_from_empty = true;
    let log = submit(&mut game, P0, Action::Pass);
    assert!(game.state.player(P0).lost && game.state.player(P1).lost);
    assert!(!game.state.player(P2).lost);
    match &game.state.outcome {
        Some(Outcome::Win { winner, .. }) => assert_eq!(*winner, P2),
        other => panic!("one survivor must win: {other:?}"),
    }
    assert_eq!(
        log.iter()
            .filter(|e| matches!(e, Event::GameOver { .. }))
            .count(),
        1
    );
    assert_eq!(
        log.iter()
            .filter(|e| matches!(e, Event::GameDrawn { .. }))
            .count(),
        0
    );
    assert!(!log.iter().any(|e| matches!(e, Event::Priority { .. })));
    let position = |needle: &str| {
        log.iter()
            .position(|event| matches!(event, Event::StateBasedAction { description } if description.contains(needle)))
            .unwrap_or_else(|| panic!("missing SBA {needle}: {log:?}"))
    };
    let p0_loss = position("P0 loses");
    let p1_loss = position("P1 loses");
    let p0_leave = position("P0 leaves");
    let p1_leave = position("P1 leaves");
    assert!(p0_loss < p1_loss, "loss records follow seat order: {log:?}");
    assert!(
        p1_loss < p0_leave,
        "ALL loss reasons precede the first departure: {log:?}"
    );
    assert!(
        p0_leave < p1_leave,
        "departures are sequential in stable seat order: {log:?}"
    );
    assert_eq!(
        log.iter().filter(|e| matches!(e, Event::StateBasedAction { description } if description.contains("loses"))).count(),
        2,
        "exactly the two detected losers are recorded once"
    );
    assert!(game.state.validate().is_ok());
}

#[test]
fn all_remaining_players_losing_is_one_draw_not_a_win() {
    let (mut game, _) = ready_commander_combat(2702);
    game.state.players[P0 as usize].life = 0;
    game.state.players[P1 as usize].life = 0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    let log = submit(&mut game, P0, Action::Pass);
    assert!(matches!(game.state.outcome, Some(Outcome::Draw { .. })));
    assert_eq!(
        log.iter()
            .filter(|e| matches!(e, Event::GameDrawn { .. }))
            .count(),
        1
    );
    assert_eq!(
        log.iter()
            .filter(|e| matches!(e, Event::GameOver { .. }))
            .count(),
        0
    );
    assert!(!log.iter().any(|e| matches!(e, Event::Priority { .. })));
    assert!(game.state.validate().is_ok());
}

#[test]
fn departed_active_turn_continues_and_pending_declaration_is_cancelled() {
    let decks = vec![plains_deck("A"), plains_deck("B"), plains_deck("C")];
    let specs = [
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
        (P2, CardId::IsamaruHoundOfKonda),
    ];
    let mut game = Game::new_commander_combat(2703, &decks, &specs).expect("3-seat game");
    game.state.mulligan = None;
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    game.state.turn.number = 2;
    game.state.turn.step = Step::DeclareAttackers;
    game.state.turn.active = P0;
    game.state.turn.priority = None;
    game.state.players[P0 as usize].life = 0;
    let log = submit(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: Vec::new(),
            defender: None,
        },
    );
    assert!(game.state.player(P0).lost);
    assert_eq!(
        game.state.turn.active, P0,
        "turn continues without an active player"
    );
    assert!(
        game.state.pending.is_none(),
        "departed player has no pending choice"
    );
    assert!(game.state.turn.priority.is_some_and(|p| p != P0));
    assert!(
        !log.iter()
            .any(|e| matches!(e, Event::Priority { player } if *player == P0))
    );
    assert!(game.state.validate().is_ok());
}

/// Same-turn 20-life command-zone control: Fog is legal and its shield does not
/// touch noncombat damage, without ever leaving a stale-turn shield.
#[test]
fn fog_same_turn_noncombat_control_in_20_life_profile() {
    let decks = vec![forest_deck("A"), forest_deck("B")];
    let specs = [
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
    ];
    let mut game = Game::new_command_zone(2801, &decks, &specs).expect("named 20-life profile");
    game.state.mulligan = None;
    game.state.pending = None;
    assert_eq!(game.state.player(P0).life, 20);
    let _forest = add_card(&mut game, P0, Zone::Battlefield, CardId::Forest, P0);
    let fog = add_card(&mut game, P0, Zone::Hand, CardId::Fog, P0);
    game.state.turn.number = 2;
    game.state.turn.active = P0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    let mut log = submit(
        &mut game,
        P0,
        Action::CastSpell {
            card: fog,
            targets: Vec::new(),
        },
    );
    log.extend(submit(&mut game, P0, Action::Pass));
    log.extend(submit(&mut game, P1, Action::Pass));
    assert_eq!(game.state.prevent_all_combat_damage_turn, Some(2));
    let _mountain = add_card(&mut game, P1, Zone::Battlefield, CardId::Mountain, P1);
    let shock = add_card(&mut game, P1, Zone::Hand, CardId::Shock, P1);
    game.state.turn.active = P1;
    game.state.turn.priority = Some(P1);
    log.extend(submit(
        &mut game,
        P1,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Player(P0)],
        },
    ));
    log.extend(submit(&mut game, P1, Action::Pass));
    log.extend(submit(&mut game, P0, Action::Pass));
    assert_eq!(
        game.state.player(P0).life,
        18,
        "noncombat Shock is not prevented"
    );
    assert!(game.state.validate().is_ok());
}

/// A commander replacement choice answered by an owner who is simultaneously at
/// lethal life: the answer applies, then the SBA departure clears state and
/// settles exactly one terminal outcome with no false priority.
#[test]
fn commander_replacement_answer_then_owner_departure_is_atomic() {
    let (mut game, commander) = ready_commander_combat(2901);
    let _island = add_card(&mut game, P1, Zone::Battlefield, CardId::Island, P1);
    let unsummon = add_card(&mut game, P1, Zone::Hand, CardId::Unsummon, P1);
    game.state.turn.number = 2;
    game.state.turn.active = P1;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P1);
    let mut log = submit(
        &mut game,
        P1,
        Action::CastSpell {
            card: unsummon,
            targets: vec![Target::Permanent(commander)],
        },
    );
    log.extend(submit(&mut game, P1, Action::Pass));
    log.extend(submit(&mut game, P0, Action::Pass));
    assert!(matches!(
        game.state.pending,
        Some(Pending::CommanderReplacement { .. })
    ));
    let (_, replay) = provenance_game(2901);
    let pending_save = SessionSave::capture(&game, replay, log.clone(), true);
    SessionSave::from_json(&pending_save.to_json())
        .expect("mid-pending elimination sandbox save loads");
    game.state.players[P0 as usize].life = 0;
    log.extend(submit(
        &mut game,
        P0,
        Action::AnswerCommanderReplacement {
            to_command_zone: true,
        },
    ));
    assert!(game.state.player(P0).lost);
    assert!(game.state.pending.is_none());
    assert_eq!(
        log.iter()
            .filter(|e| matches!(e, Event::GameOver { .. }))
            .count(),
        1
    );
    let loss = log
        .iter()
        .position(|e| matches!(e, Event::StateBasedAction { description } if description.contains("P0 loses")))
        .expect("loss SBA");
    assert!(
        !log[loss..]
            .iter()
            .any(|e| matches!(e, Event::Priority { .. })),
        "no priority after the terminal loss"
    );
    assert!(game.state.validate().is_ok());
}

/// Cleanup discard pending while the active player is at lethal life: the
/// discard answer applies, the departure clears the pending choice, and exactly
/// one terminal outcome settles without false priority.
#[test]
fn discard_pending_then_active_departure_settles_once() {
    let decks = vec![forest_deck("A"), forest_deck("B")];
    let specs = [
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
    ];
    let mut game = Game::new_commander_combat(2902, &decks, &specs).expect("game");
    game.state.mulligan = None;
    game.state.turn.number = 3;
    game.state.turn.active = P0;
    game.state.turn.step = Step::Cleanup;
    game.state.turn.priority = None;
    let first = game.state.players[P0 as usize].hand[0];
    game.state.pending = Some(Pending::Discard {
        player: P0,
        count: 1,
    });
    game.state.players[P0 as usize].life = 0;
    let log = submit(&mut game, P0, Action::Discard { cards: vec![first] });
    assert!(game.state.player(P0).lost);
    assert!(game.state.pending.is_none());
    assert_eq!(
        log.iter()
            .filter(|e| matches!(e, Event::GameOver { .. }))
            .count(),
        1
    );
    assert!(!log.iter().any(|e| matches!(e, Event::Priority { .. })));
    assert!(game.state.validate().is_ok());
}

/// A CommanderDraw replacement pending for an owner who is simultaneously at
/// lethal life: the answer is accepted, then departure settles one terminal
/// outcome with no pending choice and no false priority.
#[test]
fn commander_draw_answer_then_owner_departure_is_atomic() {
    let decks = vec![forest_deck("A"), forest_deck("B")];
    let specs = [
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
    ];
    let mut game = Game::new_commander_combat(2903, &decks, &specs).expect("game");
    game.state.mulligan = None;
    game.state.pending = None;
    let commander_id = game.state.commanders[0].id;
    let object = game.state.commanders[0].object;
    game.state.players[P0 as usize]
        .command_zone
        .retain(|id| *id != object);
    {
        let o = game.state.objects.get_mut(&object).expect("commander");
        o.zone = Zone::Library;
    }
    game.state.players[P0 as usize].library.insert(0, object);
    game.state.commanders[0].return_eligible = false;
    game.state.turn.number = 3;
    game.state.turn.active = P0;
    game.state.turn.step = Step::Draw;
    game.state.turn.priority = None;
    game.state.prevent_all_combat_damage_turn = None;
    game.state.pending = Some(Pending::CommanderDraw {
        player: P0,
        commander: commander_id,
        object,
        remaining: 0,
        resume: DrawResume::Step,
    });
    assert!(
        game.state.validate().is_ok(),
        "constructed CommanderDraw state is valid"
    );
    let (_, replay) = provenance_game(2903);
    let draw_save = SessionSave::capture(&game, replay, Vec::new(), true);
    SessionSave::from_json(&draw_save.to_json()).expect("mid-CommanderDraw sandbox save loads");
    game.state.players[P0 as usize].life = 0;
    let log = submit(
        &mut game,
        P0,
        Action::AnswerCommanderDraw {
            to_command_zone: true,
        },
    );
    assert!(game.state.player(P0).lost);
    assert!(game.state.pending.is_none());
    assert_eq!(
        log.iter()
            .filter(|e| matches!(e, Event::GameOver { .. }))
            .count(),
        1
    );
    let loss = log.iter().position(|e| matches!(e, Event::StateBasedAction { description } if description.contains("P0 loses"))).expect("loss");
    assert!(
        !log[loss..]
            .iter()
            .any(|e| matches!(e, Event::Priority { .. }))
    );
    assert!(game.state.validate().is_ok());
}

/// A CommanderReturn queue whose first owner departs before answering: that
/// owner's entry is removed, remaining owners keep the canonical order, and the
/// batch still applies without a pending choice for the removed player.
#[test]
fn commander_return_queue_survives_a_departing_owner() {
    let decks = vec![forest_deck("A"), forest_deck("B"), forest_deck("C")];
    let specs = [
        (P0, CardId::IsamaruHoundOfKonda),
        (P1, CardId::IsamaruHoundOfKonda),
        (P2, CardId::IsamaruHoundOfKonda),
    ];
    let mut game = Game::new_commander_combat(2904, &decks, &specs).expect("3-seat game");
    game.state.mulligan = None;
    game.state.pending = None;
    game.state.turn.number = 2;
    game.state.turn.active = P0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = None;
    let mut order = Vec::new();
    for registry in &mut game.state.commanders {
        let object = registry.object;
        game.state.players[registry.owner as usize]
            .command_zone
            .retain(|id| *id != object);
        let o = game.state.objects.get_mut(&object).expect("commander");
        o.zone = Zone::Graveyard;
        game.state.players[registry.owner as usize]
            .graveyard
            .push(object);
        registry.return_eligible = true;
        order.push(aperture_core::state::CommanderReturnEntry {
            player: registry.owner,
            commander: registry.id,
            object,
            zone: Zone::Graveyard,
        });
    }
    game.state.pending = Some(Pending::CommanderReturn {
        order,
        cursor: 0,
        answers: Vec::new(),
    });
    assert!(
        game.state.validate().is_ok(),
        "constructed return queue is valid"
    );
    let setups: Vec<CommanderSetup> = specs
        .iter()
        .map(|(seat, card)| CommanderSetup {
            seat: *seat,
            card: *card,
        })
        .collect();
    let replay = Replay::new_commander_combat(
        2904,
        game.state.first_player,
        decks.clone(),
        setups,
        AiSettings::default(),
    )
    .expect("3-seat replay");
    let return_save = SessionSave::capture(&game, replay, Vec::new(), true);
    SessionSave::from_json(&return_save.to_json()).expect("mid-CommanderReturn sandbox save loads");
    game.state.players[P0 as usize].life = 0;
    let log = submit(
        &mut game,
        P0,
        Action::AnswerCommanderReturn {
            to_command_zone: true,
        },
    );
    assert!(game.state.player(P0).lost);
    match &game.state.pending {
        Some(Pending::CommanderReturn {
            order,
            cursor,
            answers,
        }) => {
            assert_eq!(*cursor, 0);
            assert!(answers.is_empty());
            assert_eq!(order.len(), 2);
            assert_eq!(order[0].player, P1);
            assert_eq!(order[1].player, P2);
        }
        other => panic!("remaining owners keep a pending queue: {other:?}"),
    }
    assert!(game.state.validate().is_ok());
    submit(
        &mut game,
        P1,
        Action::AnswerCommanderReturn {
            to_command_zone: true,
        },
    );
    submit(
        &mut game,
        P2,
        Action::AnswerCommanderReturn {
            to_command_zone: true,
        },
    );
    assert!(game.state.pending.is_none());
    assert!(game.state.outcome.is_none(), "two live seats remain");
    assert_eq!(game.state.players[P1 as usize].command_zone.len(), 1);
    assert_eq!(game.state.players[P2 as usize].command_zone.len(), 1);
    assert!(game.state.validate().is_ok());
    let _ = log;
}

/// Scry pending while its owner is at lethal life: the answer resolves first,
/// then the SBA departure leaves no stale pending/resolving/object reference and
/// exactly one terminal outcome.
#[test]
fn order_card_pending_then_owner_departure_leaves_no_stale_choice() {
    let (mut game, _) = ready_commander_combat(2905);
    let _island = add_card(&mut game, P0, Zone::Battlefield, CardId::Island, P0);
    let opt = add_card(&mut game, P0, Zone::Hand, CardId::Opt, P0);
    game.state.turn.number = 2;
    game.state.turn.active = P0;
    game.state.turn.step = Step::PrecombatMain;
    game.state.turn.priority = Some(P0);
    let mut log = submit(
        &mut game,
        P0,
        Action::CastSpell {
            card: opt,
            targets: Vec::new(),
        },
    );
    log.extend(submit(&mut game, P0, Action::Pass));
    log.extend(submit(&mut game, P1, Action::Pass));
    assert!(
        matches!(game.state.pending, Some(Pending::OrderCard { .. })),
        "scry pending: {log:?}"
    );
    game.state.players[P0 as usize].life = 0;
    log.extend(submit(&mut game, P0, Action::OrderCard { to_top: true }));
    assert!(game.state.player(P0).lost);
    assert!(game.state.pending.is_none());
    assert!(game.state.resolving.is_none());
    assert_eq!(
        log.iter()
            .filter(|e| matches!(e, Event::GameOver { .. }))
            .count(),
        1
    );
    assert!(game.state.validate().is_ok());
}

/// Co-loser pending reassignment: P0 is the current APNAP chooser and P1 is a
/// later chooser in the same simultaneous loss set. Both leave together; the
/// collected P0 answer is not reassigned to the still-live P2, and the terminal
/// outcome discards the queue exactly once (CR 101.4, 800.4a/g/h, 704.3).
#[test]
fn co_loser_apnap_chooser_is_not_reassigned_and_terminal_discards_queue() {
    let mut game = fixture_n(2805, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain, CardId::Forest])
        .battlefield(1, &[CardId::GrizzlyBears])
        .battlefield(2, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let p0_forest = battlefield_card(&game, P0, CardId::Forest);
    let mut events = Vec::new();
    let crack = hand_card(&game, P0, CardId::CrackTheEarth);
    events.extend(submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: crack,
            targets: Vec::new(),
        },
    ));
    let mut guard = 0;
    while !matches!(game.state.pending, Some(Pending::ApnapSacrifice { .. })) {
        let owner = game.decision_owner().expect("priority");
        events.extend(submit_ok(&mut game, owner, Action::Pass));
        guard += 1;
        assert!(guard < 16);
    }
    game.state.players[P0 as usize].life = 0;
    game.state.players[1].life = 0;
    events.extend(submit_ok(
        &mut game,
        P0,
        Action::ChooseSacrifice {
            permanent: Some(p0_forest),
        },
    ));
    assert!(game.state.player(P0).lost && game.state.player(1).lost);
    assert!(matches!(
        game.state.outcome,
        Some(Outcome::Win { winner: 2, .. })
    ));
    assert!(
        game.state.pending.is_none(),
        "terminal outcome discards the queue"
    );
    let loss = events
        .iter()
        .position(|e| {
            matches!(e, Event::StateBasedAction { description } if description.contains("P0 loses"))
        })
        .expect("loss SBA");
    assert!(
        !events[loss..]
            .iter()
            .any(|e| matches!(e, Event::Priority { .. })),
        "no priority after the simultaneous loss"
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::GameOver { .. }))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::GameDrawn { .. }))
            .count(),
        0
    );
    assert!(game.state.validate().is_ok());
}

/// Four-seat co-loser APNAP source departure: P0 (the spell's owner/current
/// chooser) and later chooser P1 leave together. The resolving Crack the Earth
/// leaves with P0, so the remaining each-player queue is cancelled rather than
/// resumed without a source; survivors continue without sacrifices/terminal.
/// RED 76 showed the prior orphan likely-pending invalid state.
#[test]
fn four_seat_co_loser_apnap_source_departure_continues_survivors() {
    let mut game = fixture_n(2806, 4)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain, CardId::Forest])
        .battlefield(1, &[CardId::GrizzlyBears])
        .battlefield(2, &[CardId::Forest])
        .battlefield(3, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let p0_forest = battlefield_card(&game, P0, CardId::Forest);
    let p2_forest = battlefield_card(&game, 2, CardId::Forest);
    let p3_forest = battlefield_card(&game, 3, CardId::Forest);
    let crack = hand_card(&game, P0, CardId::CrackTheEarth);
    let mut events = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: crack,
            targets: Vec::new(),
        },
    );
    let mut guard = 0;
    while !matches!(game.state.pending, Some(Pending::ApnapSacrifice { .. })) {
        let owner = game.decision_owner().expect("priority");
        events.extend(submit_ok(&mut game, owner, Action::Pass));
        guard += 1;
        assert!(guard < 16);
    }
    game.state.players[P0 as usize].life = 0;
    game.state.players[1].life = 0;
    events.extend(submit_ok(
        &mut game,
        P0,
        Action::ChooseSacrifice {
            permanent: Some(p0_forest),
        },
    ));
    assert!(game.state.player(P0).lost && game.state.player(1).lost);
    assert!(game.state.outcome.is_none(), "two survivors remain");
    assert!(game.state.outcome.is_none(), "two survivors remain");
    match &game.state.pending {
        Some(Pending::ApnapSacrifice {
            player,
            order,
            choices,
            ..
        }) => {
            assert_eq!(*player, 2, "queue advances to the next live chooser");
            assert_eq!(order, &vec![2, 3]);
            assert!(
                choices.is_empty(),
                "departed answer is purged, not reassigned"
            );
        }
        other => panic!("survivor continuation must keep the queue: {other:?}"),
    }
    assert_eq!(
        game.state.object(p2_forest).zone,
        Zone::Battlefield,
        "no sacrifice applied before the last answer"
    );
    assert_eq!(game.state.object(p3_forest).zone, Zone::Battlefield);
    assert!(
        game.state.validate().is_ok(),
        "source-left provenance validates: {:?}",
        game.state.validate()
    );
    events.extend(submit_ok(
        &mut game,
        2,
        Action::ChooseSacrifice {
            permanent: Some(p2_forest),
        },
    ));
    events.extend(submit_ok(
        &mut game,
        3,
        Action::ChooseSacrifice {
            permanent: Some(p3_forest),
        },
    ));
    assert!(game.state.pending.is_none());
    assert!(game.state.resolving.is_none());
    assert_eq!(game.state.object(p2_forest).zone, Zone::Graveyard);
    assert_eq!(game.state.object(p3_forest).zone, Zone::Graveyard);
    assert!(game.state.validate().is_ok());
    let _ = events;
}

/// Survivor-owned Crack the Earth controlled by the departing P0 is exiled by
/// 800.4a step 4; survivors P1/P2 still continue the each-player queue on the
/// recorded last-known source, with P0's answer purged.
#[test]
fn survivor_owned_source_exiled_but_queue_continues() {
    let mut game = fixture_n(2807, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain, CardId::Forest])
        .battlefield(1, &[CardId::GrizzlyBears])
        .battlefield(2, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let p0_forest = battlefield_card(&game, P0, CardId::Forest);
    let p1_bear = battlefield_card(&game, 1, CardId::GrizzlyBears);
    let p2_forest = battlefield_card(&game, 2, CardId::Forest);
    let crack = hand_card(&game, P0, CardId::CrackTheEarth);
    let mut events = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: crack,
            targets: Vec::new(),
        },
    );
    let mut guard = 0;
    while !matches!(game.state.pending, Some(Pending::ApnapSacrifice { .. })) {
        let owner = game.decision_owner().expect("priority");
        events.extend(submit_ok(&mut game, owner, Action::Pass));
        guard += 1;
        assert!(guard < 16);
    }
    // Model a survivor (P1) owning the represented source while departed P0
    // still controls it. This is a direct-restored control fact, not a modeled
    // continuous control-effect producer.
    let source = game.state.resolving.expect("resolving source");
    {
        let object = game.state.objects.get_mut(&source).expect("source object");
        object.owner = 1;
        object.controller = P0;
    }
    game.state.players[0].life = 0;
    events.extend(submit_ok(
        &mut game,
        P0,
        Action::ChooseSacrifice {
            permanent: Some(p0_forest),
        },
    ));
    assert!(game.state.player(P0).lost);
    assert_eq!(
        game.state.object(source).zone,
        Zone::Exile,
        "still-controlled survivor-owned source exiled"
    );
    match &game.state.pending {
        Some(Pending::ApnapSacrifice {
            player,
            order,
            choices,
            ..
        }) => {
            assert_eq!(*player, 1);
            assert_eq!(order, &vec![1, 2]);
            assert!(choices.is_empty());
        }
        other => panic!("survivor continuation expected: {other:?}"),
    }
    eprintln!(
        "DBG resolving={:?} pending={:?} source={:?}",
        game.state.resolving,
        game.state.pending,
        game.state
            .objects
            .get(&source)
            .map(|o| (o.zone, o.owner, o.controller, o.card))
    );
    assert!(
        game.state.validate().is_ok(),
        "exiled-source provenance validates: {:?}",
        game.state.validate()
    );
    events.extend(submit_ok(
        &mut game,
        1,
        Action::ChooseSacrifice {
            permanent: Some(p1_bear),
        },
    ));
    events.extend(submit_ok(
        &mut game,
        2,
        Action::ChooseSacrifice {
            permanent: Some(p2_forest),
        },
    ));
    assert!(game.state.pending.is_none());
    assert!(game.state.resolving.is_none());
    assert_eq!(game.state.object(p1_bear).zone, Zone::Graveyard);
    assert_eq!(game.state.object(p2_forest).zone, Zone::Graveyard);
    assert!(game.state.validate().is_ok());
    let _ = events;
}

/// R166-1 source-present branch: the final unanswered participant leaves after
/// two survivors recorded choices; apply them exactly once, finish the original
/// Crack the Earth resolution (no orphan stack/cursor), then grant priority.
#[test]
fn last_unanswered_departure_applies_surviving_choices_before_priority() {
    let mut game = fixture_n(166, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain, CardId::Forest])
        .battlefield(1, &[CardId::GrizzlyBears])
        .battlefield(2, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let forest = battlefield_card(&game, P0, CardId::Forest);
    let bear = battlefield_card(&game, 1, CardId::GrizzlyBears);
    let crack = hand_card(&game, P0, CardId::CrackTheEarth);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: crack,
            targets: Vec::new(),
        },
    );
    for _ in 0..8 {
        if matches!(game.state.pending, Some(Pending::ApnapSacrifice { .. })) {
            break;
        }
        let p = game.decision_owner().unwrap();
        submit_ok(&mut game, p, Action::Pass);
    }
    submit_ok(
        &mut game,
        P0,
        Action::ChooseSacrifice {
            permanent: Some(forest),
        },
    );
    game.state.players[2].life = 0;
    let events = submit_ok(
        &mut game,
        1,
        Action::ChooseSacrifice {
            permanent: Some(bear),
        },
    );
    assert!(game.state.player(2).lost);
    assert_eq!(game.state.object(forest).zone, Zone::Graveyard);
    assert_eq!(game.state.object(bear).zone, Zone::Graveyard);
    assert!(game.state.pending.is_none());
    assert!(game.state.resolving.is_none());
    assert!(game.state.stack.is_empty(), "no orphan Crack stack/cursor");
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(
                e,
                Event::Destroyed {
                    card: CardId::CrackTheEarth,
                    ..
                }
            ))
            .count(),
        1
    );
    let resolved = events
        .iter()
        .position(|e| {
            matches!(
                e,
                Event::Resolve {
                    card: CardId::CrackTheEarth,
                    ..
                }
            )
        })
        .expect("Crack resolved");
    let priority = events
        .iter()
        .position(|e| matches!(e, Event::Priority { .. }))
        .expect("priority");
    assert!(resolved < priority, "resolution completes before priority");
    assert!(game.state.validate().is_ok());
}

/// R166-1 source-left branch: source is survivor-owned and exiled while P0
/// departs; the final unanswered P2 leaves after P1 records a choice. Apply the
/// surviving choice once, complete the recorded instruction, settle one terminal
/// outcome with no stack/resolving orphan.
#[test]
fn last_unanswered_departure_with_departed_source_applies_choices_once() {
    let mut game = fixture_n(167, 3)
        .hand(P0, &[CardId::CrackTheEarth])
        .battlefield(P0, &[CardId::Mountain, CardId::Forest])
        .battlefield(1, &[CardId::GrizzlyBears])
        .battlefield(2, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let forest = battlefield_card(&game, P0, CardId::Forest);
    let bear = battlefield_card(&game, 1, CardId::GrizzlyBears);
    let crack = hand_card(&game, P0, CardId::CrackTheEarth);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: crack,
            targets: Vec::new(),
        },
    );
    for _ in 0..8 {
        if matches!(game.state.pending, Some(Pending::ApnapSacrifice { .. })) {
            break;
        }
        let p = game.decision_owner().unwrap();
        submit_ok(&mut game, p, Action::Pass);
    }
    let source = game.state.resolving.expect("resolving Crack");
    {
        let object = game.state.objects.get_mut(&source).expect("source");
        object.owner = 1;
        object.controller = P0;
    }
    game.state.players[0].life = 0;
    let mut events = submit_ok(
        &mut game,
        P0,
        Action::ChooseSacrifice {
            permanent: Some(forest),
        },
    );
    assert!(game.state.player(P0).lost);
    assert_eq!(game.state.object(source).zone, Zone::Exile);
    game.state.players[2].life = 0;
    events.extend(submit_ok(
        &mut game,
        1,
        Action::ChooseSacrifice {
            permanent: Some(bear),
        },
    ));
    assert!(game.state.player(2).lost);
    assert!(matches!(
        game.state.outcome,
        Some(Outcome::Win { winner: 1, .. })
    ));
    assert_eq!(game.state.object(bear).zone, Zone::Graveyard);
    assert_eq!(
        game.state.object(forest).zone,
        Zone::Removed,
        "departed P0 permanent left the game"
    );
    assert!(game.state.pending.is_none());
    assert!(game.state.resolving.is_none());
    assert!(game.state.stack.is_empty());
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::GameOver { .. }))
            .count(),
        1
    );
    let terminal = events
        .iter()
        .position(|e| matches!(e, Event::GameOver { .. }))
        .expect("terminal");
    assert!(
        !events[terminal..]
            .iter()
            .any(|e| matches!(e, Event::Priority { .. }))
    );
    assert!(game.state.validate().is_ok());
}
