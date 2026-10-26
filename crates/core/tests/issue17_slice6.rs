//! Issue 17 slice 6: sourced death-trigger and target-player package.
//!
//! Three cards added to the supported catalog, each read from live Scryfall in
//! the run that added it (`fixtures/starter-corpus/slice6-sourced.json`):
//! Blood Artist (any creature dies → target player loses 1 life, you gain 1),
//! Zulaport Cutthroat (a creature you control dies → each opponent loses 1 life,
//! you gain 1) and Hedron Crab (landfall → target player mills three). Every
//! assertion drives a real engine action.

mod common;

use aperture_core::ability::{TriggerEvent, abilities};
use aperture_core::catalog;
use aperture_core::*;
use common::*;

const P2: PlayerId = 2;
const P3: PlayerId = 3;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

fn graveyard_cards(game: &Game, p: PlayerId) -> Vec<CardId> {
    game.state
        .player(p)
        .graveyard
        .iter()
        .map(|id| game.state.object(*id).card)
        .collect()
}

/// Drive the game with the deterministic scripted policy until `pred` holds.
fn advance_until(game: &mut Game, pred: impl Fn(&Game) -> bool) -> Vec<Event> {
    let mut log = Vec::new();
    let reached = run_until(game, &mut log, pred, 80);
    assert!(
        reached,
        "the scripted driver never reached the expected state"
    );
    log
}

/// Answer the pending CR 603.3d target choice with an explicit target and then
/// drive the rest of the resolution.
fn choose_trigger_target(game: &mut Game, player: PlayerId, target: Target) -> Vec<Event> {
    assert!(
        matches!(game.state.pending, Some(Pending::TriggerTarget { .. })),
        "a triggered ability must be waiting for its target, got {:?}",
        game.state.pending
    );
    submit_ok(game, player, Action::ChooseTriggerTarget { target })
}

#[test]
fn sourced_slice6_cards_match_their_live_printings() {
    let artist = card("Blood Artist").def();
    assert_eq!(artist.cost, Some("{1}{B}"));
    assert_eq!(artist.type_line, "Creature — Vampire");
    assert_eq!((artist.power, artist.toughness), (Some(0), Some(1)));
    assert_eq!(
        artist.oracle,
        "Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life."
    );
    assert!(artist.spell.is_none(), "a creature has no spell effects");
    let trigger = abilities(CardId::BloodArtist)
        .iter()
        .find_map(|a| a.triggered())
        .expect("Blood Artist has a triggered ability");
    assert_eq!(trigger.event, TriggerEvent::CreatureDies);
    assert_eq!(trigger.target, TargetSpec::Player);
    assert_eq!(
        trigger.effects,
        &[Effect::LoseLife(1), Effect::GainLife(1)],
        "life loss first, then the life gain (CR 608.2c)"
    );

    let zulaport = card("Zulaport Cutthroat").def();
    assert_eq!(zulaport.cost, Some("{1}{B}"));
    assert_eq!(zulaport.type_line, "Creature — Human Rogue Ally");
    assert_eq!((zulaport.power, zulaport.toughness), (Some(1), Some(1)));
    assert_eq!(
        zulaport.oracle,
        "Whenever this creature or another creature you control dies, each opponent loses 1 life and you gain 1 life."
    );
    let trigger = abilities(CardId::ZulaportCutthroat)
        .iter()
        .find_map(|a| a.triggered())
        .expect("Zulaport Cutthroat has a triggered ability");
    assert_eq!(trigger.event, TriggerEvent::CreatureYouControlDies);
    assert_eq!(trigger.target, TargetSpec::None, "no printed target");
    assert_eq!(
        trigger.effects,
        &[Effect::LoseLifeEachOpponent(1), Effect::GainLife(1)]
    );

    let crab = card("Hedron Crab").def();
    assert_eq!(crab.cost, Some("{U}"));
    assert_eq!(crab.type_line, "Creature — Crab");
    assert_eq!((crab.power, crab.toughness), (Some(0), Some(2)));
    assert_eq!(
        crab.oracle,
        "Landfall — Whenever a land you control enters, target player mills three cards. (They put the top three cards of their library into their graveyard.)"
    );
    let trigger = abilities(CardId::HedronCrab)
        .iter()
        .find_map(|a| a.triggered())
        .expect("Hedron Crab has a triggered ability");
    assert_eq!(trigger.event, TriggerEvent::LandEnters);
    assert_eq!(trigger.target, TargetSpec::Player);
    assert_eq!(trigger.effects, &[Effect::MillTargetPlayer(3)]);
}

#[test]
fn hedron_crab_mills_the_targeted_player_not_its_controller() {
    let crab = card("Hedron Crab");
    let mut game = fixture_n(601, 4)
        .hand(P0, &[CardId::Forest])
        .battlefield(P0, &[crab])
        .library(
            P1,
            &[
                CardId::Island,
                CardId::Forest,
                CardId::Mountain,
                CardId::Plains,
            ],
        )
        .library(P0, &[CardId::Plains, CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let forest = hand_card(&game, P0, CardId::Forest);
    submit_ok(&mut game, P0, Action::PlayLand { card: forest });
    let log = choose_trigger_target(&mut game, P0, Target::Player(P1));
    let log = {
        let mut all = log;
        all.extend(advance_until(&mut game, |g| {
            g.state.player(P1).library.len() == 1
        }));
        all
    };

    assert_eq!(
        graveyard_cards(&game, P1),
        vec![CardId::Island, CardId::Forest, CardId::Mountain],
        "the targeted player mills the top three cards in order"
    );
    assert_eq!(
        game.state.player(P0).library.len(),
        2,
        "the Crab's controller never mills for \"target player\""
    );
    let milled: Vec<PlayerId> = log
        .iter()
        .filter_map(|event| match event {
            Event::Milled { player, .. } => Some(*player),
            _ => None,
        })
        .collect();
    assert_eq!(milled, vec![P1, P1, P1], "one Milled event per card");
}

#[test]
fn hedron_crab_may_target_its_own_controller() {
    let crab = card("Hedron Crab");
    let mut game = fixture_n(602, 4)
        .hand(P0, &[CardId::Forest])
        .battlefield(P0, &[crab])
        .library(
            P0,
            &[
                CardId::Island,
                CardId::Mountain,
                CardId::Plains,
                CardId::Forest,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let forest = hand_card(&game, P0, CardId::Forest);
    submit_ok(&mut game, P0, Action::PlayLand { card: forest });
    let offered = game.legal_targets(P0, TargetSpec::Player, battlefield_card(&game, P0, crab));
    assert!(
        offered.contains(&Target::Player(P0)),
        "CR 115.4: \"target player\" includes the ability's own controller"
    );
    choose_trigger_target(&mut game, P0, Target::Player(P0));
    advance_until(&mut game, |g| {
        g.state
            .player(P0)
            .graveyard
            .iter()
            .filter(|id| g.state.object(**id).card != CardId::Forest)
            .count()
            == 3
    });
    let milled: Vec<CardId> = graveyard_cards(&game, P0)
        .into_iter()
        .filter(|c| *c != CardId::Forest)
        .collect();
    assert_eq!(
        milled,
        vec![CardId::Island, CardId::Mountain, CardId::Plains]
    );
}

/// A 1/1 token body for the death watchers: Goblin Instigator's enters trigger
/// is already in the pool, and `create_tokens` is the path that must tell the
/// watchers a *token* creature died (the R211-1 class of defect).
#[test]
fn creature_tokens_dying_are_seen_by_death_watchers() {
    let zulaport = card("Zulaport Cutthroat");
    let mut game = fixture_n(603, 4)
        .battlefield(
            P0,
            &[
                zulaport,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
            ],
        )
        .hand(P0, &[CardId::GoblinInstigator, CardId::LightningBolt])
        .main(P0, Step::PrecombatMain)
        .build();
    let before_p1 = game.state.player(P1).life;
    let before_p2 = game.state.player(P2).life;
    let instigator = hand_card(&game, P0, CardId::GoblinInstigator);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: instigator,
            targets: vec![],
        },
    );
    advance_until(&mut game, |g| {
        g.state
            .battlefield(P0)
            .iter()
            .any(|o| o.card == CardId::GoblinToken)
    });
    let token = game
        .state
        .battlefield(P0)
        .iter()
        .find(|o| o.card == CardId::GoblinToken)
        .map(|o| o.id)
        .expect("the token is on the battlefield");
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Permanent(token)],
        },
    );
    advance_until(&mut game, |g| {
        g.state.player(P1).life == before_p1 - 1
            && g.state.player(P2).life == before_p2 - 1
            && g.state.player(P3).life == 19
    });
    assert_eq!(
        game.state.player(P0).life,
        21,
        "the watcher's controller gained 1 life"
    );
    assert!(
        !game.state.battlefield(P0).iter().any(|o| o.id == token),
        "the token really died"
    );
}

#[test]
fn blood_artist_drains_for_an_opponents_creature_death_to_the_chosen_player() {
    let artist = card("Blood Artist");
    let mut game = fixture_n(604, 4)
        .battlefield(P0, &[artist, CardId::Mountain])
        .battlefield(P1, &[CardId::TrainedJackal])
        .hand(P0, &[CardId::LightningBolt])
        .main(P0, Step::PrecombatMain)
        .build();
    let jackal = battlefield_card(&game, P1, CardId::TrainedJackal);
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Permanent(jackal)],
        },
    );
    advance_until(&mut game, |g| {
        matches!(g.state.pending, Some(Pending::TriggerTarget { .. }))
    });
    assert!(
        !game.state.battlefield(P1).iter().any(|o| o.id == jackal),
        "the opponent's creature really died before the trigger"
    );
    choose_trigger_target(&mut game, P0, Target::Player(P2));
    advance_until(&mut game, |g| g.state.player(P2).life == 19);
    assert_eq!(
        game.state.player(P2).life,
        19,
        "the chosen player lost 1 life even though the dying creature was an opponent's"
    );
    assert_eq!(
        game.state.player(P0).life,
        21,
        "the Artist's controller gained 1"
    );
    assert_eq!(
        game.state.player(P1).life,
        20,
        "the dying creature's controller is untouched"
    );
}

#[test]
fn blood_artist_triggers_for_its_own_death_and_for_an_opponents_creature() {
    let artist = card("Blood Artist");
    let mut game = fixture_n(605, 4)
        .battlefield(P0, &[artist, CardId::Mountain])
        .hand(P0, &[CardId::LightningBolt])
        .main(P0, Step::PrecombatMain)
        .build();
    let self_id = battlefield_card(&game, P0, artist);
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Permanent(self_id)],
        },
    );
    let log = advance_until(&mut game, |g| {
        matches!(g.state.pending, Some(Pending::TriggerTarget { .. }))
    });
    assert!(
        log.iter().any(|e| matches!(e, Event::Destroyed { .. })),
        "the Artist really died before its trigger"
    );
    choose_trigger_target(&mut game, P0, Target::Player(P1));
    advance_until(&mut game, |g| g.state.player(P1).life == 19);
    assert_eq!(game.state.player(P1).life, 19, "the chosen player lost 1");
    assert_eq!(
        game.state.player(P0).life,
        21,
        "the Artist's controller gained 1"
    );
    assert_eq!(game.state.player(P2).life, 20, "other seats are untouched");
}

#[test]
fn zulaport_cutthroat_ignores_an_opponents_creature_but_drains_for_its_own_death() {
    let zulaport = card("Zulaport Cutthroat");
    // An opponent's creature dying is not "a creature you control dies".
    let mut game = fixture_n(606, 4)
        .battlefield(P0, &[zulaport, CardId::Mountain])
        .battlefield(P1, &[CardId::TrainedJackal])
        .hand(P0, &[CardId::LightningBolt])
        .main(P0, Step::PrecombatMain)
        .build();
    let jackal = battlefield_card(&game, P1, CardId::TrainedJackal);
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Permanent(jackal)],
        },
    );
    advance_until(&mut game, |g| {
        !g.state.battlefield(P1).iter().any(|o| o.id == jackal) && g.state.pending.is_none()
    });
    assert_eq!(
        game.state.player(P1).life,
        20,
        "an opponent's creature dying never triggers \"a creature you control dies\""
    );
    assert_eq!(game.state.player(P0).life, 20, "no life was gained");

    // Its own death does trigger, because the printed wording includes itself.
    let mut game = fixture_n(607, 4)
        .battlefield(P0, &[zulaport, CardId::Mountain])
        .hand(P0, &[CardId::LightningBolt])
        .main(P0, Step::PrecombatMain)
        .build();
    let self_id = battlefield_card(&game, P0, zulaport);
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Permanent(self_id)],
        },
    );
    advance_until(&mut game, |g| {
        g.state.player(P1).life == 19 && g.state.player(P2).life == 19
    });
    assert_eq!(
        game.state.player(P0).life,
        21,
        "its controller gained 1 life"
    );
    assert_eq!(game.state.player(P3).life, 19, "every opponent lost 1 life");
}

#[test]
fn life_loss_is_not_damage_and_a_damage_shield_never_stops_it() {
    let artist = card("Blood Artist");
    let mut game = fixture_n(608, 4)
        .battlefield(P0, &[artist, CardId::Mountain])
        .battlefield(P1, &[CardId::TrainedJackal, CardId::Plains])
        .hand(P0, &[CardId::LightningBolt])
        .hand(P1, &[CardId::MendingHands])
        .life(P1, 20)
        .main(P0, Step::PrecombatMain)
        .build();
    // P1 shields itself with a real prevention effect (Mending Hands prints
    // "any target", so a player is legal).
    let mending = hand_card(&game, P1, CardId::MendingHands);
    game.state.turn.priority = Some(P1);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: mending,
            targets: vec![Target::Player(P1)],
        },
    );
    advance_until(&mut game, |g| !g.state.player(P1).hand.contains(&mending));
    let shielded = game.state.player(P1).life;
    assert_eq!(shielded, 20, "casting the shield costs no life");

    game.state.turn.priority = Some(P0);
    let jackal = battlefield_card(&game, P1, CardId::TrainedJackal);
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bolt,
            targets: vec![Target::Permanent(jackal)],
        },
    );
    advance_until(&mut game, |g| {
        matches!(g.state.pending, Some(Pending::TriggerTarget { .. }))
    });
    choose_trigger_target(&mut game, P0, Target::Player(P1));
    let log = advance_until(&mut game, |g| g.state.player(P1).life < 20);
    // CR 118.2: losing life is not damage, so the prevention shield cannot
    // absorb it and no Damage event is emitted for it.
    assert_eq!(
        game.state.player(P1).life,
        19,
        "the shield did not stop the 1 life loss"
    );
    assert!(
        !log.iter()
            .any(|event| matches!(event, Event::Damage { target, .. } if target == "P1")),
        "nothing was dealt to P1: life loss is not damage"
    );
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::LifeChange { player, delta: -1, .. } if *player == P1
        )),
        "the life change is recorded as a LifeChange, not damage"
    );
}

#[test]
fn target_player_spec_offers_every_live_seat_and_refuses_a_creature() {
    let crab = card("Hedron Crab");
    let mut game = fixture_n(609, 4)
        .hand(P0, &[CardId::Forest])
        .battlefield(P0, &[crab])
        .main(P0, Step::PrecombatMain)
        .build();
    let crab_id = battlefield_card(&game, P0, crab);
    let offered = game.legal_targets(P0, TargetSpec::Player, crab_id);
    assert_eq!(
        offered,
        vec![
            Target::Player(P0),
            Target::Player(P1),
            Target::Player(P2),
            Target::Player(P3)
        ],
        "CR 115.4: every live seat, in seat order"
    );
    let forest = hand_card(&game, P0, CardId::Forest);
    submit_ok(&mut game, P0, Action::PlayLand { card: forest });
    let error = submit_err(
        &mut game,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(crab_id),
        },
    );
    assert!(
        matches!(error, EngineError::Illegal { .. }),
        "a permanent is not an offered \"target player\" (CR 603.3d), got {error:?}"
    );
    // The pending choice is unchanged, so the legal target still resolves.
    choose_trigger_target(&mut game, P0, Target::Player(P2));
}

/// CR 704.3 + 603.6c: creatures that die at the same time are one event, and a
/// watcher that dies in that batch still sees its fellow dying creatures. This
/// used to depend on object-id order: the watcher was moved to the graveyard
/// first and then missed the other death (review finding S6-R1-F1).
#[test]
fn simultaneous_deaths_each_trigger_the_watchers_regardless_of_id_order() {
    let zulaport = card("Zulaport Cutthroat");
    for (seed, artist_first) in [(610u64, true), (611, false)] {
        let mut cards = if artist_first {
            vec![zulaport, CardId::TrainedJackal]
        } else {
            vec![CardId::TrainedJackal, zulaport]
        };
        let mut game = fixture_n(seed, 4)
            .battlefield(P0, &cards)
            .main(P0, Step::PrecombatMain)
            .build();
        // Both creatures are lethally damaged before the same SBA check, so they
        // die in one batch.
        let ids: Vec<ObjectId> = game
            .state
            .battlefield(P0)
            .iter()
            .map(|object| object.id)
            .collect();
        assert_eq!(ids.len(), 2);
        for id in &ids {
            object_mut(&mut game.state, *id).damage = 5;
        }
        advance_until(&mut game, |g| {
            g.state.player(P1).life == 18 && g.state.player(P2).life == 18
        });
        assert_eq!(
            game.state.player(P0).life,
            22,
            "two deaths are two triggers: {artist_first}"
        );
        assert_eq!(game.state.player(P3).life, 18);
        cards.clear();
    }
}

fn drained(game: &Game) -> bool {
    game.state.pending.is_none()
        && game.state.resolving.is_none()
        && game.state.stack.is_empty()
        && game.state.trigger_queue.is_empty()
}

#[test]
fn dying_watcher_self_trigger_fires_exactly_once_both_death_orders() {
    for cards in [
        [CardId::ZulaportCutthroat, CardId::TrainedJackal],
        [CardId::TrainedJackal, CardId::ZulaportCutthroat],
    ] {
        let mut game = fixture_n(224, 4)
            .battlefield(P0, &cards)
            .main(P0, Step::PrecombatMain)
            .build();
        for object in game.state.objects.values_mut() {
            object.damage = 5;
        }
        submit_ok(&mut game, P0, Action::Pass);
        let mut log = Vec::new();
        assert!(run_until(&mut game, &mut log, drained, 100));
        let lives: Vec<i32> = game.state.players.iter().map(|p| p.life).collect();
        assert_eq!(
            lives,
            vec![22, 18, 18, 18],
            "two deaths must produce exactly two drains for order {cards:?}"
        );
    }
}

#[test]
fn single_dying_watcher_self_trigger_fires_exactly_once() {
    let mut game = fixture_n(226, 4)
        .battlefield(P0, &[CardId::ZulaportCutthroat])
        .main(P0, Step::PrecombatMain)
        .build();
    for object in game.state.objects.values_mut() {
        object.damage = 5;
    }
    submit_ok(&mut game, P0, Action::Pass);
    let mut log = Vec::new();
    assert!(run_until(&mut game, &mut log, drained, 100));
    let lives: Vec<i32> = game.state.players.iter().map(|p| p.life).collect();
    assert_eq!(lives, vec![21, 19, 19, 19], "one death, one drain");
}

#[test]
fn surviving_watcher_still_counts_both_deaths_once() {
    let mut game = fixture_n(225, 4)
        .battlefield(
            P0,
            &[
                CardId::ZulaportCutthroat,
                CardId::TrainedJackal,
                CardId::GrizzlyBears,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    for object in game.state.objects.values_mut() {
        if object.card != CardId::ZulaportCutthroat {
            object.damage = 5;
        }
    }
    submit_ok(&mut game, P0, Action::Pass);
    let mut log = Vec::new();
    assert!(run_until(&mut game, &mut log, drained, 100));
    let lives: Vec<i32> = game.state.players.iter().map(|p| p.life).collect();
    assert_eq!(lives, vec![22, 18, 18, 18]);
}
