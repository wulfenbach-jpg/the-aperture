//! Typed counters (CR 122), creature tokens (CR 111) and library operations
//! (CR 401, 701.13, 701.16, 701.20, 701.26): characteristics, state-based
//! actions, zone changes, evasion, visibility, determinism and save/load.
//!
//! The 28-card practice pool prints no keywords and has no card that creates a
//! counter or a token, so these mechanics are driven through the rules
//! primitives (`GameState::create_token`, `Object::counters`, `GameState::mill`,
//! `GameState::reveal_top`) and then executed by real engine actions with real
//! pool cards (Unsummon, Shock, combat). That is a deliberate bounded scope, not
//! a claim of general counter/token coverage.

mod common;

use aperture_core::counters::CounterKind;
use aperture_core::*;
use common::*;

fn add_counters(game: &mut Game, id: ObjectId, kind: CounterKind, count: u32) {
    game.state
        .objects
        .get_mut(&id)
        .expect("object is in the state")
        .counters
        .add(kind, count);
}

fn counters_of(game: &Game, id: ObjectId) -> String {
    game.state.object(id).counters.describe()
}

#[test]
fn power_and_toughness_counters_change_characteristics_and_projection() {
    // CR 122.1a, 613.4c
    let mut game = fixture(60)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    assert_eq!(
        (
            game.state.object(bear).power(),
            game.state.object(bear).toughness()
        ),
        (2, 2)
    );

    add_counters(&mut game, bear, CounterKind::PlusOnePlusOne, 2);
    assert_eq!(
        (
            game.state.object(bear).power(),
            game.state.object(bear).toughness()
        ),
        (4, 4)
    );

    add_counters(&mut game, bear, CounterKind::MinusOneMinusOne, 1);
    assert_eq!(
        (
            game.state.object(bear).power(),
            game.state.object(bear).toughness()
        ),
        (3, 3),
        "+1/+1 and -1/-1 counters apply together (CR 122.1a)"
    );
    game.state.validate().expect("counter state is legal");

    // Counters are public information and appear in every projection.
    let view = game.view_for(P1, &[]);
    let shown = view
        .opponent_view(P0)
        .expect("P1 sees P0")
        .battlefield
        .iter()
        .find(|card| card.id == bear)
        .expect("the bear is on P0's visible battlefield");
    assert_eq!(shown.power, Some(3));
    assert_eq!(shown.toughness, Some(3));
    assert_eq!(
        shown.counters,
        vec![("+1/+1", 2), ("-1/-1", 1)],
        "counters are canonical and public"
    );
    assert_eq!(
        shown.plus_one_counters, 0,
        "legacy sandbox storage is not the rules layer"
    );
}

#[test]
fn minus_counters_reduce_toughness_to_zero_and_the_creature_dies() {
    // CR 704.5g: a creature with toughness 0 or less is put into its owner's
    // graveyard as a state-based action.
    let mut game = fixture(61)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    add_counters(&mut game, bear, CounterKind::MinusOneMinusOne, 2);
    assert_eq!(game.state.object(bear).toughness(), 0);

    let mut log = Vec::new();
    assert!(
        run_until(
            &mut game,
            &mut log,
            |g| g.state.zone_of(bear) == Some(Zone::Graveyard),
            64
        ),
        "the 0-toughness creature is put into a graveyard; log={log:?}"
    );
    assert!(
        game.state.object(bear).counters.is_empty(),
        "counters cease with the permanent that carried them (CR 122.2)"
    );
    assert_eq!(game.state.player(P0).graveyard.len(), 1);
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::StateBasedAction { description } if description.contains("704.5g")
        )),
        "the death is a cited state-based action: {log:?}"
    );
    game.state.validate().expect("post-SBA state is legal");
}

#[test]
fn counters_do_not_follow_the_card_out_of_the_battlefield() {
    // CR 122.2: counters cease to exist with the object that carried them.
    let mut game = fixture(62)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .hand(P1, &[CardId::Unsummon])
        .battlefield(P1, &[CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    add_counters(&mut game, bear, CounterKind::PlusOnePlusOne, 3);

    submit_ok(&mut game, P0, Action::Pass);
    let unsummon = hand_card(&game, P1, CardId::Unsummon);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: unsummon,
            targets: vec![Target::Permanent(bear)],
        },
    );
    submit_ok(&mut game, P1, Action::Pass);
    submit_ok(&mut game, P0, Action::Pass);

    let returned = game.state.player(P0).hand[0];
    assert_eq!(game.state.object(returned).zone, Zone::Hand);
    assert!(
        game.state.object(returned).counters.is_empty(),
        "counters must not travel with the card"
    );
    assert_eq!(counters_of(&game, returned), "");
    game.state.validate().expect("bounced state is legal");
}

#[test]
fn combat_damage_uses_counter_modified_power() {
    // CR 510.1a: damage equal to the creature's power, counters included.
    let mut game = fixture(63)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let attacker = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let blocker = battlefield_card(&game, P1, CardId::GrizzlyBears);
    add_counters(&mut game, attacker, CounterKind::PlusOnePlusOne, 1);
    assert_eq!(game.state.object(attacker).power(), 3);

    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![attacker],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareBlockers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl { blocker, attacker }],
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.player(P1).graveyard.len() == 1,
        64
    ));
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::CombatDamage {
                amount: 3,
                blocker: Some((_, CardId::GrizzlyBears)),
                ..
            }
        )),
        "3 damage from a 2/2 with one +1/+1 counter: {log:?}"
    );
    assert_eq!(
        game.state.object(attacker).toughness(),
        3,
        "the 3/3 attacker survives two damage"
    );
    assert_eq!(game.state.object(attacker).damage, 2);
}

#[test]
fn a_created_token_has_its_own_characteristics_ownership_and_visibility() {
    // CR 111.1, 111.2, 111.3, 302.6
    let mut game = fixture(64).main(P0, Step::PrecombatMain).build();
    let spirit = game.state.create_token(TokenKind::Spirit, P1);
    {
        let token = game.state.object(spirit);
        assert_eq!(token.zone, Zone::Battlefield);
        assert_eq!(token.owner, P1, "the creator owns the token (CR 111.2)");
        assert_eq!(token.controller, P1, "the creator controls it (CR 111.3)");
        assert!(token.is_token() && token.is_creature());
        assert_eq!((token.power(), token.toughness()), (1, 1));
        assert!(token.has_keyword(Keyword::Flying));
        assert!(
            token.summoning_sick,
            "a new creature token is summoning sick (CR 302.6)"
        );
    }
    assert!(game.state.player(P1).graveyard.is_empty());
    assert!(
        game.state.player(P1).library.is_empty() && game.state.player(P1).hand.is_empty(),
        "a token is never a card in a hidden zone"
    );
    game.state.validate().expect("token state is legal");

    let view = game.view_for(P0, &[]);
    let seen = view
        .opponent_view(P1)
        .expect("P0 sees P1")
        .battlefield
        .iter()
        .find(|card| card.id == spirit)
        .expect("the token is visible on the battlefield");
    assert!(seen.is_token);
    assert_eq!(seen.name, "Spirit");
    assert_eq!(seen.power, Some(1));
}

#[test]
fn a_flying_token_can_only_be_blocked_by_flying_or_reach() {
    // CR 509.1b, 702.9b, 702.17b
    let mut game = fixture(65)
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let spirit = game.state.create_token(TokenKind::Spirit, P0);
    game.state
        .objects
        .get_mut(&spirit)
        .expect("token")
        .summoning_sick = false;

    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers: vec![spirit],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.turn.step == Step::DeclareBlockers && g.state.pending.is_some(),
        64
    ));

    let bear = battlefield_card(&game, P1, CardId::GrizzlyBears);
    let err = submit_err(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: bear,
                attacker: spirit,
            }],
        },
    );
    assert!(
        format!("{err}").contains("flying or reach"),
        "a ground creature cannot block a flyer: {err}"
    );
    // The engine never offers an illegal pair either.
    let offered: Vec<Action> = game
        .legal_actions()
        .into_iter()
        .map(|legal| legal.action)
        .filter(|action| matches!(action, Action::DeclareBlockers { blocks } if !blocks.is_empty()))
        .collect();
    assert!(
        offered.is_empty(),
        "no illegal block is offered: {offered:?}"
    );

    // A second flying creature may block it.
    let blocker_spirit = game.state.create_token(TokenKind::Spirit, P1);
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: blocker_spirit,
                attacker: spirit,
            }],
        },
    );
}

#[test]
fn a_destroyed_token_ceases_to_exist_and_never_enters_a_graveyard() {
    // CR 111.7
    let mut game = fixture(66)
        .hand(P0, &[CardId::Shock])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let spirit = game.state.create_token(TokenKind::Spirit, P1);

    let shock = hand_card(&game, P0, CardId::Shock);
    let events = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: shock,
            targets: vec![Target::Permanent(spirit)],
        },
    );
    assert!(events.iter().any(|e| matches!(e, Event::Cast { .. })));
    let resolve = submit_ok(&mut game, P0, Action::Pass);
    let back = submit_ok(&mut game, P1, Action::Pass);

    let all: Vec<Event> = resolve.into_iter().chain(back).collect();
    assert!(
        game.state.try_object(spirit).is_none(),
        "the token no longer exists"
    );
    assert!(
        game.state.player(P1).graveyard.is_empty(),
        "a token never stays in a graveyard (CR 111.7)"
    );
    assert!(
        all.iter().any(|event| matches!(
            event,
            Event::StateBasedAction { description } if description.contains("ceases to exist")
        )),
        "the log states the token ceased to exist: {all:?}"
    );
    game.state
        .validate()
        .expect("post-cessation state is legal");
}

#[test]
fn a_bounced_token_ceases_to_exist_and_never_reaches_a_hand() {
    // CR 111.7
    let mut game = fixture(67)
        .hand(P1, &[CardId::Unsummon])
        .battlefield(P1, &[CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let soldier = game.state.create_token(TokenKind::Soldier, P0);
    submit_ok(&mut game, P0, Action::Pass);
    let unsummon = hand_card(&game, P1, CardId::Unsummon);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: unsummon,
            targets: vec![Target::Permanent(soldier)],
        },
    );
    submit_ok(&mut game, P1, Action::Pass);
    submit_ok(&mut game, P0, Action::Pass);

    assert!(game.state.try_object(soldier).is_none());
    assert!(
        game.state.player(P0).hand.is_empty(),
        "the token does not become a card in hand"
    );
    game.state
        .validate()
        .expect("post-cessation state is legal");
}

#[test]
fn mill_shuffle_reveal_and_untap_are_deterministic_and_information_safe() {
    // CR 401.2, 701.13a, 701.16a, 701.20a, 701.26a
    let library = [
        CardId::Forest,
        CardId::Island,
        CardId::Mountain,
        CardId::GrizzlyBears,
        CardId::Forest,
    ];
    let mut game = fixture(68)
        .library(P0, &library)
        .library(P1, &library)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let top_three: Vec<ObjectId> = game
        .state
        .player(P0)
        .library
        .iter()
        .copied()
        .take(3)
        .collect();

    let milled = game.state.mill(P0, 2);
    assert_eq!(milled, top_three[..2].to_vec(), "mill takes from the top");
    assert_eq!(game.state.player(P0).graveyard, milled);
    assert_eq!(game.state.player(P0).library.len(), 3);
    assert!(game.state.object(milled[0]).counters.is_empty());

    // Reveal makes exactly the named card public without moving it.
    let revealed = game.state.reveal_top(P0, 1);
    assert_eq!(revealed.len(), 1);
    let opponent = game.view_for(P1, &[]);
    assert_eq!(
        opponent
            .revealed
            .iter()
            .map(|card| card.card)
            .collect::<Vec<_>>(),
        vec![game.state.object(revealed[0]).card],
        "the revealed library card is public to the opponent"
    );
    assert_eq!(
        opponent
            .opponent_view(P0)
            .expect("P1 sees P0")
            .library_count,
        library.len() - 2,
        "the milled cards left P0's library"
    );
    assert_eq!(
        game.state.zone_of(revealed[0]),
        Some(Zone::Library),
        "revealing does not move the card (CR 701.16a)"
    );
    assert!(
        !opponent
            .opponent_view(P0)
            .expect("P1 sees P0")
            .battlefield
            .iter()
            .any(|card| card.id == revealed[0]),
        "a revealed library card is not a permanent"
    );

    // Shuffle is deterministic under the same seed and keeps the multiset.
    let mut twin = fixture(68)
        .library(P0, &library)
        .library(P1, &library)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .build();
    twin.state.mill(P0, 2);
    game.state.shuffle_library(P0);
    twin.state.shuffle_library(P0);
    assert_eq!(
        game.state.player(P0).library,
        twin.state.player(P0).library,
        "the same seed shuffles identically"
    );
    let mut before: Vec<CardId> = game
        .state
        .player(P0)
        .library
        .iter()
        .map(|id| game.state.object(*id).card)
        .collect();
    before.sort_unstable();
    let mut expected = vec![CardId::Mountain, CardId::GrizzlyBears, CardId::Forest];
    expected.sort_unstable();
    assert_eq!(before, expected, "shuffling changes order, never contents");

    // Tap/untap round-trips and refuses non-permanents.
    let bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    assert!(!game.state.untap(bear), "an untapped permanent is a no-op");
    assert!(game.state.tap(bear));
    assert!(game.state.object(bear).tapped);
    assert!(game.state.untap(bear));
    assert!(!game.state.object(bear).tapped);
    assert!(
        !game.state.untap(milled[0]),
        "a graveyard card cannot be untapped"
    );
}

#[test]
fn a_real_game_keeps_its_digest_and_survives_a_save_load_round_trip() {
    // Determinism, replay and save/load must be unchanged by the new fields.
    let (mut game, mut log) = Game::new_logged(4242, &[deck_a(), deck_b()]).expect("game");
    let mid = run_until(&mut game, &mut log, |g| g.state.turn.number >= 3, 400);
    assert!(mid, "the game reaches turn 3");

    let counters_before = game
        .state
        .objects
        .values()
        .map(|object| object.counters.total())
        .sum::<u32>();
    assert_eq!(counters_before, 0, "no pool card creates counters yet");

    let digest = game.digest();
    let json = serde_json::to_string(&game.state).expect("state serializes");
    let saved: GameState = serde_json::from_str(&json).expect("state deserializes");
    saved.validate().expect("a saved state is a legal state");
    let restored = Game::from_state(saved).expect("saved state loads");
    assert_eq!(
        restored.digest(),
        digest,
        "save/load preserves the exact state"
    );
    assert!(restored.state.revealed.is_empty());

    // The same seed produces the same digest again: nothing drifted.
    let (mut twin, mut twin_log) = Game::new_logged(4242, &[deck_a(), deck_b()]).expect("game");
    assert!(run_until(
        &mut twin,
        &mut twin_log,
        |g| g.state.turn.number >= 3,
        400
    ));
    assert_eq!(twin.digest(), digest);
    assert_eq!(twin_log, log);
}

#[test]
fn counters_and_tokens_survive_save_load_exactly() {
    let mut game = fixture(69)
        .battlefield(P0, &[CardId::CentaurCourser])
        .main(P0, Step::PrecombatMain)
        .build();
    let courser = battlefield_card(&game, P0, CardId::CentaurCourser);
    add_counters(&mut game, courser, CounterKind::PlusOnePlusOne, 2);
    add_counters(&mut game, courser, CounterKind::MinusOneMinusOne, 1);
    let spirit = game.state.create_token(TokenKind::Spirit, P1);
    let digest = game.digest();

    let json = serde_json::to_string(&game.state).expect("state serializes");
    let saved: GameState = serde_json::from_str(&json).expect("state deserializes");
    let restored = Game::from_state(saved).expect("saved state loads");

    assert_eq!(restored.digest(), digest);
    assert_eq!(
        restored.state.object(courser).counters,
        game.state.object(courser).counters
    );
    assert_eq!(restored.state.object(courser).power(), 4);
    assert_eq!(restored.state.object(courser).toughness(), 4);
    assert_eq!(restored.state.object(spirit).card, CardId::SpiritToken);
    assert!(restored.state.object(spirit).has_keyword(Keyword::Flying));
    restored.state.validate().expect("loaded state is legal");
}

#[test]
fn a_saved_state_with_counters_off_the_battlefield_is_refused() {
    // CR 122.2 is an invariant, not a convention: an illegal save is rejected.
    let game = fixture(70)
        .battlefield(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let mut state = game.state.clone();
    {
        let object = state.objects.get_mut(&bear).expect("bear");
        object.zone = Zone::Graveyard;
        object.counters.add(CounterKind::PlusOnePlusOne, 1);
    }
    state.players[P0 as usize].graveyard.push(bear);
    let err = state
        .validate()
        .expect_err("counters in a graveyard are illegal");
    assert!(
        format!("{err}").contains("counter(s) in the graveyard zone"),
        "{err}"
    );
}
