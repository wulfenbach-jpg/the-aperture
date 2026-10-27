//! Issue 17 slice 22 — Dancing from Dark to Dawn.
//!
//! Sourced printing: HOB 123, `{3}{G}{G}` Enchantment (see
//! `fixtures/starter-corpus/slice22-sourced.json`). The card prints two separate
//! triggered abilities:
//!
//! * "Whenever you cast a creature spell, put X +1/+1 counters on target creature
//!   you control, where X is that spell's mana value."
//! * "Landfall — Whenever a land you control enters, create a 2/2 green Bear
//!   creature token."
//!
//! The first is the pool's first "you cast a creature spell" trigger and the
//! first effect sized by the triggering spell's mana value, so the tests below
//! discriminate it from the older instant/sorcery cast trigger, from an
//! opponent's cast and from a noncreature permanent spell, and prove the value is
//! captured at cast time by saving and reloading while the target is still being
//! chosen.

mod common;

use aperture_core::catalog;
use aperture_core::counters::CounterKind;
use aperture_core::replay::{Replay, SessionSave};
use aperture_core::state::{Pending, Step};
use aperture_core::*;
use common::*;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

fn green(generic: u32, pips: u32) -> ManaPool {
    ManaPool {
        green: pips,
        colorless: generic,
        ..ManaPool::default()
    }
}

fn counters_on(game: &Game, id: ObjectId) -> u32 {
    game.state
        .object(id)
        .counters
        .get(CounterKind::PlusOnePlusOne)
}

fn drain(game: &mut Game, log: &mut Vec<Event>) {
    for _ in 0..128 {
        if game.state.pending.is_some() || game.state.stack.is_empty() {
            return;
        }
        let Some(p) = game.decision_owner() else {
            return;
        };
        log.extend(submit_ok(game, p, Action::Pass));
    }
    panic!("the stack did not settle");
}

fn reload(game: &Game) -> Game {
    let decks = vec![
        DeckList::parse_practice("A", "40 Forest\n").unwrap(),
        DeckList::parse_practice("B", "40 Forest\n").unwrap(),
    ];
    let replay = Replay::new(
        game.state.seed,
        game.state.first_player,
        decks,
        AiSettings::default(),
    )
    .expect("replay");
    let save = SessionSave::capture(game, replay, Vec::new(), true);
    let loaded = SessionSave::from_json(&save.to_json()).expect("save loads");
    Game::from_state(loaded.game).expect("loaded game")
}

fn triggered_by_dancing(log: &[Event]) -> usize {
    log.iter()
        .filter(|event| {
            matches!(
                event,
                Event::TriggerStaged {
                    card: CardId::DancingFromDarkToDawn,
                    ..
                }
            )
        })
        .count()
}

/// P0 has the enchantment and two 2/2 creatures (so the trigger has a genuine
/// target choice), P1 has one 2/2 of their own, and both seats hold enough green
/// mana for the creatures these tests cast.
fn board(seed: u64, hand: &[CardId]) -> Game {
    fixture(seed)
        .battlefield(
            P0,
            &[
                CardId::DancingFromDarkToDawn,
                CardId::GrizzlyBears,
                CardId::RuneclawBear,
            ],
        )
        .battlefield(P1, &[CardId::RuneclawBear])
        .hand(P0, hand)
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .mana(P0, green(8, 4))
        .mana(P1, green(8, 4))
        .main(P0, Step::PrecombatMain)
        .build()
}

#[test]
fn casting_a_creature_puts_its_mana_value_in_counters_on_the_chosen_creature() {
    // {1}{G} Grizzly Bears has mana value 2, so the trigger adds exactly two.
    let mut game = board(2201, &[CardId::GrizzlyBears]);
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let cast = hand_card(&game, P0, CardId::GrizzlyBears);
    let mut log = Vec::new();

    log.extend(submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: cast,
            targets: Vec::new(),
        },
    ));
    // Two creatures are legal targets (the Bear already on the battlefield and
    // the resolving spell's own permanent), so the engine must ask.
    let Some(Pending::TriggerTarget { player, legal, .. }) = game.state.pending.clone() else {
        panic!(
            "the cast trigger must ask for its target: {:?}",
            game.state.pending
        );
    };
    assert_eq!(player, P0);
    assert!(
        legal.contains(&Target::Permanent(bears)),
        "legal: {legal:?}"
    );
    assert_eq!(
        triggered_by_dancing(&log),
        0,
        "nothing is staged before the target is chosen (CR 603.3d)"
    );

    log.extend(submit_ok(
        &mut game,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(bears),
        },
    ));
    drain(&mut game, &mut log);

    assert_eq!(
        counters_on(&game, bears),
        2,
        "X is the cast spell's mana value"
    );
    assert_eq!(game.state.object(bears).power(), 4, "2/2 + two counters");
    assert_eq!(game.state.object(bears).toughness(), 4);
    assert_eq!(triggered_by_dancing(&log), 1, "exactly one cast trigger");
}

#[test]
fn the_counter_count_tracks_a_six_mana_creature() {
    // {4}{G}{G} Craw Wurm has mana value 6: the size must follow the printed
    // cost rather than a hard-coded number.
    let mut game = fixture(2202)
        .battlefield(
            P0,
            &[
                CardId::DancingFromDarkToDawn,
                CardId::GrizzlyBears,
                CardId::RuneclawBear,
            ],
        )
        .battlefield(P1, &[CardId::RuneclawBear])
        .hand(P0, &[CardId::CrawWurm])
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .mana(P0, green(8, 4))
        .mana(P1, green(8, 4))
        .main(P0, Step::PrecombatMain)
        .build();
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let wurm = hand_card(&game, P0, CardId::CrawWurm);
    let mut log = Vec::new();
    log.extend(submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: wurm,
            targets: Vec::new(),
        },
    ));
    log.extend(submit_ok(
        &mut game,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(bears),
        },
    ));
    drain(&mut game, &mut log);
    assert_eq!(counters_on(&game, bears), 6);
}

#[test]
fn an_opponents_creature_spell_does_not_trigger_it() {
    // "you cast" is controller-scoped: P1 casting on their own turn with P0's
    // enchantment watching must not trigger it.
    let mut game = fixture(2203)
        .battlefield(P0, &[CardId::DancingFromDarkToDawn, CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::RuneclawBear])
        .hand(P1, &[CardId::RuneclawBear])
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .mana(P1, green(2, 1))
        .main(P1, Step::PrecombatMain)
        .build();
    let spell = hand_card(&game, P1, CardId::RuneclawBear);
    let mut log = Vec::new();
    log.extend(submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: spell,
            targets: Vec::new(),
        },
    ));
    drain(&mut game, &mut log);
    assert_eq!(
        triggered_by_dancing(&log),
        0,
        "P1's cast must not trigger P0's enchantment"
    );
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    assert_eq!(counters_on(&game, bears), 0);
}

#[test]
fn a_noncreature_permanent_spell_does_not_trigger_it() {
    // Bearscape is an enchantment spell: neither an instant/sorcery (the older
    // cast trigger) nor a creature spell.
    let mut game = board(2204, &[CardId::Bearscape]);
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let enchantment = hand_card(&game, P0, CardId::Bearscape);
    let mut log = Vec::new();
    log.extend(submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: enchantment,
            targets: Vec::new(),
        },
    ));
    drain(&mut game, &mut log);
    assert_eq!(
        triggered_by_dancing(&log),
        0,
        "a noncreature permanent spell must not trigger it"
    );
    assert_eq!(counters_on(&game, bears), 0);
    assert!(
        game.state
            .objects
            .values()
            .any(|o| o.card == CardId::Bearscape && o.zone == Zone::Battlefield),
        "the enchantment still resolved normally"
    );
}

#[test]
fn landfall_creates_one_bear_token_and_adds_no_counters() {
    // The landfall half creates a real Bear (the subtype the Bear-matters cards
    // read), and it is not the cast trigger.
    let mut game = board(2205, &[CardId::Forest]);
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let land = hand_card(&game, P0, CardId::Forest);
    let before = game
        .state
        .objects
        .values()
        .filter(|o| o.card == CardId::BearToken && o.zone == Zone::Battlefield)
        .count();
    let mut log = Vec::new();
    log.extend(submit_ok(&mut game, P0, Action::PlayLand { card: land }));
    drain(&mut game, &mut log);

    let tokens: Vec<ObjectId> = game
        .state
        .objects
        .values()
        .filter(|o| o.card == CardId::BearToken && o.zone == Zone::Battlefield)
        .map(|o| o.id)
        .collect();
    assert_eq!(tokens.len(), before + 1, "exactly one Bear token: {log:?}");
    let token = tokens[0];
    assert_eq!(game.state.object(token).power(), 2);
    assert_eq!(game.state.object(token).toughness(), 2);
    assert!(game.state.object(token).card.def().has_subtype("Bear"));
    assert_eq!(
        counters_on(&game, bears),
        0,
        "landfall is not the cast trigger"
    );
}

#[test]
fn an_opponents_land_does_not_trigger_the_landfall_half() {
    let mut game = fixture(2206)
        .battlefield(P0, &[CardId::DancingFromDarkToDawn])
        .battlefield(P1, &[CardId::RuneclawBear])
        .hand(P1, &[CardId::Forest])
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P1, Step::PrecombatMain)
        .build();
    let land = hand_card(&game, P1, CardId::Forest);
    let before = game
        .state
        .objects
        .values()
        .filter(|o| o.card == CardId::BearToken)
        .count();
    let mut log = Vec::new();
    log.extend(submit_ok(&mut game, P1, Action::PlayLand { card: land }));
    drain(&mut game, &mut log);
    let after = game
        .state
        .objects
        .values()
        .filter(|o| o.card == CardId::BearToken)
        .count();
    assert_eq!(before, after, "P1's land must not trigger P0's landfall");
}

#[test]
fn the_captured_mana_value_survives_a_save_and_reload_while_targeting() {
    // The mana value is captured when the spell is cast and travels on the
    // serialized stack item, so a save taken while the target is still being
    // chosen must resume with the same counters.
    let mut game = board(2207, &[CardId::GrizzlyBears]);
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let cast = hand_card(&game, P0, CardId::GrizzlyBears);
    let mut log = Vec::new();
    log.extend(submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: cast,
            targets: Vec::new(),
        },
    ));
    assert!(matches!(
        game.state.pending,
        Some(Pending::TriggerTarget { .. })
    ));
    let mut resumed = reload(&game);
    assert_eq!(resumed.digest(), game.digest(), "the save is lossless");

    let mut resumed_log = Vec::new();
    resumed_log.extend(submit_ok(
        &mut resumed,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(bears),
        },
    ));
    drain(&mut resumed, &mut resumed_log);
    assert_eq!(
        counters_on(&resumed, bears),
        2,
        "the reloaded trigger still knows the cast spell's mana value"
    );
}

#[test]
fn mana_value_reads_the_printed_cost() {
    assert_eq!(card("Dancing from Dark to Dawn").def().mana_value(), 5);
    assert_eq!(card("Grizzly Bears").def().mana_value(), 2);
    assert_eq!(card("Craw Wurm").def().mana_value(), 6);
    assert_eq!(card("Shock").def().mana_value(), 1);
    assert_eq!(card("Sol Ring").def().mana_value(), 1);
    assert_eq!(card("Forest").def().mana_value(), 0, "lands have no cost");
}
