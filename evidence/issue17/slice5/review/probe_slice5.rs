//! Independent review probe for issue 17 slice 5 (review-only; never part of the crate).
//!
//! Scenario-level checks that the shipped focused test file does *not* cover:
//! short/empty mill, mill vs. dies/enters triggers, opponent landfall,
//! Negate vs. an ability on the stack, mana emptying, and Preordain ordering
//! with a real bottoming decision.
//!
//! Build:
//!   rustc --edition 2021 -L dependency=target/debug/deps \
//!     --extern aperture_core=target/debug/libaperture_core.rlib \
//!     probe_slice5.rs -o probe_slice5.exe
//! Run from the repository root so `include_str!` deck paths resolve.

#[path = "../../../../crates/core/tests/common/mod.rs"]
mod common;

use aperture_core::ability::{TriggerEvent, triggers_on};
use aperture_core::state::{Commander, Pending, StackEntry, StackItem};
use aperture_core::*;
use common::*;

const P2: PlayerId = 2;
const P3: PlayerId = 3;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

fn grave(game: &Game, p: PlayerId) -> Vec<CardId> {
    game.state
        .player(p)
        .graveyard
        .iter()
        .map(|id| game.state.object(*id).card)
        .collect()
}

fn lib(game: &Game, p: PlayerId) -> Vec<CardId> {
    game.state
        .player(p)
        .library
        .iter()
        .map(|id| game.state.object(*id).card)
        .collect()
}

fn count_milled(log: &[Event]) -> usize {
    log.iter()
        .filter(|e| matches!(e, Event::Milled { .. }))
        .count()
}

fn drama(log: &[Event]) -> usize {
    log.iter()
        .filter(|e| {
            matches!(
                e,
                Event::Destroyed { .. }
                    | Event::PermanentEntered { .. }
                    | Event::AbilityResolved { .. }
                    | Event::LifeChange { .. }
            )
        })
        .count()
}

fn settle(game: &mut Game, log: &mut Vec<Event>, cap: usize) {
    let ok = run_until(
        game,
        log,
        |g| g.state.stack.is_empty() && g.state.pending.is_none(),
        cap,
    );
    assert!(ok, "the stack never emptied");
}

// ---------------------------------------------------------------- 1. short library
fn short_library_mill() {
    let supplier = card("Stitcher's Supplier");
    let mut game = fixture_n(201, 2)
        .hand(P0, &[supplier])
        .battlefield(P0, &[CardId::Swamp])
        .library(P0, &[CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let id = hand_card(&game, P0, supplier);
    submit_ok(&mut game, P0, Action::CastSpell { card: id, targets: vec![] });
    let mut log = Vec::new();
    settle(&mut game, &mut log, 200);
    assert_eq!(grave(&game, P0), vec![CardId::Island], "mill must stop at the library's end");
    assert!(lib(&game, P0).is_empty(), "library empty after milling its only card");
    assert_eq!(count_milled(&log), 1, "exactly one card was milled");
    assert_eq!(game.state.player(P0).life, 20, "a mill is not life loss");
    assert!(!game.state.is_over(), "milling the last card is not a loss (CR 704.5b)");
    assert!(
        !log.iter().any(|e| matches!(e, Event::DrewFromEmpty { .. })),
        "no draw happened"
    );
    println!("PASS short_library_mill (milled 1 of 3, no loss)");
}

// ---------------------------------------------------------------- 2. empty library
fn empty_library_mill() {
    let supplier = card("Stitcher's Supplier");
    let mut game = fixture_n(202, 2)
        .hand(P0, &[supplier])
        .battlefield(P0, &[CardId::Swamp])
        .main(P0, Step::PrecombatMain)
        .build();
    let id = hand_card(&game, P0, supplier);
    submit_ok(&mut game, P0, Action::CastSpell { card: id, targets: vec![] });
    let mut log = Vec::new();
    settle(&mut game, &mut log, 200);
    assert_eq!(count_milled(&log), 0, "milling an empty library mills nothing");
    assert!(grave(&game, P0).is_empty());
    assert_eq!(game.state.player(P0).life, 20);
    assert!(!game.state.is_over(), "an empty-library mill is legal and harmless");
    assert!(!game.state.player(P0).drew_from_empty);
    println!("PASS empty_library_mill (0 milled, game continues)");
}

// ------------------------------------------- 3. milled creatures fire no battlefield triggers
fn mill_fires_no_battlefield_triggers() {
    let supplier = card("Stitcher's Supplier");
    let mut game = fixture_n(203, 2)
        .hand(P0, &[supplier])
        .battlefield(P0, &[CardId::Swamp])
        .battlefield(P1, &[CardId::SoulWarden])
        .library(
            P0,
            &[
                CardId::GrizzlyBears,
                supplier,
                CardId::Forest,
                CardId::Mountain,
                CardId::Plains,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let id = hand_card(&game, P0, supplier);
    submit_ok(&mut game, P0, Action::CastSpell { card: id, targets: vec![] });
    let mut log = Vec::new();
    settle(&mut game, &mut log, 200);

    assert_eq!(
        grave(&game, P0),
        vec![CardId::GrizzlyBears, supplier, CardId::Forest],
        "top three, in library order"
    );
    assert_eq!(count_milled(&log), 3, "one Milled event per card, and no cascade");
    assert_eq!(game.state.player(P1).life, 21, "only the Supplier itself entered the battlefield");
    let destroyed = log
        .iter()
        .filter(|e| matches!(e, Event::Destroyed { .. }))
        .count();
    assert_eq!(destroyed, 0, "a milled card never dies (CR 700.4)");
    let gains = log
        .iter()
        .filter(|e| matches!(e, Event::LifeChange { delta, .. } if *delta > 0))
        .count();
    assert_eq!(gains, 1, "milled creatures do not 'enter' for Soul Warden");
    println!("PASS mill_fires_no_battlefield_triggers (3 milled, 0 dies, 1 legit ETB gain)");
}

// ------------------------------------------- 3b. the printed "or dies" mill really fires
fn supplier_mills_again_when_it_dies() {
    let supplier = card("Stitcher's Supplier");
    let mut game = fixture_n(211, 2)
        .hand(P0, &[CardId::LightningBolt])
        .battlefield(P0, &[CardId::Mountain, supplier])
        .library(
            P0,
            &[
                CardId::Island,
                CardId::Forest,
                CardId::Mountain,
                CardId::Plains,
                CardId::GrizzlyBears,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let permanent = battlefield_card(&game, P0, supplier);
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell { card: bolt, targets: vec![Target::Permanent(permanent)] },
    );
    let mut log = Vec::new();
    settle(&mut game, &mut log, 400);
    assert_eq!(
        grave(&game, P0),
        vec![
            CardId::LightningBolt,
            supplier,
            CardId::Island,
            CardId::Forest,
            CardId::Mountain
        ],
        "Bolt, the dead Supplier, then the three cards milled by the dies trigger"
    );
    assert_eq!(
        lib(&game, P0),
        vec![CardId::Plains, CardId::GrizzlyBears],
        "the dies trigger milled exactly three"
    );
    assert_eq!(count_milled(&log), 3, "exactly one mill-3 trigger fired");
    let destroyed = log
        .iter()
        .filter(|e| matches!(e, Event::Destroyed { card, .. } if *card == supplier))
        .count();
    assert_eq!(destroyed, 1, "one battlefield departure of the Supplier, one dies trigger");
    println!("PASS supplier_mills_again_when_it_dies (dies trigger mills 3, once)");
}

// ------------------------------------------------------------- 4. Ruin Crab landfall
fn ruin_crab_landfall_scopes() {
    // (a) an opponent's land must not trigger the crab
    let crab = card("Ruin Crab");
    let mut game = fixture_n(204, 2)
        .hand(P0, &[CardId::Forest])
        .battlefield(P1, &[crab])
        .library(P1, &[CardId::Island; 4])
        .library(P0, &[CardId::Island; 4])
        .main(P0, Step::PrecombatMain)
        .build();
    let land = hand_card(&game, P0, CardId::Forest);
    submit_ok(&mut game, P0, Action::PlayLand { card: land });
    let mut log = Vec::new();
    settle(&mut game, &mut log, 100);
    assert!(
        !log.iter().any(|e| matches!(e, Event::Milled { .. })),
        "landfall is 'a land you control', not any land"
    );
    assert_eq!(game.state.player(P1).graveyard.len(), 0);
    println!("PASS ruin_crab_ignores_opponent_landfall");

    // (b) the controller's land triggers it; opponents mill, controller does not
    let mut game = fixture_n(205, 4)
        .hand(P1, &[CardId::Forest])
        .battlefield(P1, &[crab])
        .library(P0, &[CardId::Island; 4])
        .library(P2, &[CardId::Island; 4])
        .library(P3, &[CardId::Island; 4])
        .library(P1, &[CardId::Island; 4])
        .main(P1, Step::PrecombatMain)
        .build();
    game.state.players[P3 as usize].lost = true; // eliminated seat must be skipped
    let land = hand_card(&game, P1, CardId::Forest);
    submit_ok(&mut game, P1, Action::PlayLand { card: land });
    let mut log = Vec::new();
    settle(&mut game, &mut log, 100);
    assert_eq!(grave(&game, P1), Vec::<CardId>::new(), "the crab's controller never mills");
    assert_eq!(game.state.player(P0).graveyard.len(), 3);
    assert_eq!(game.state.player(P2).graveyard.len(), 3);
    assert_eq!(
        game.state.player(P3).graveyard.len(),
        0,
        "an eliminated opponent is not milled"
    );
    assert_eq!(count_milled(&log), 6);
    let order: Vec<PlayerId> = log
        .iter()
        .filter_map(|e| match e {
            Event::Milled { player, .. } => Some(*player),
            _ => None,
        })
        .collect();
    println!("  mill event order (active = P1): {order:?}");
    println!("PASS ruin_crab_mills_each_live_opponent_only");

    // (c) two crabs stack (CR 603.2c: each trigger resolves separately)
    let mut game = fixture_n(206, 2)
        .hand(P1, &[CardId::Forest])
        .battlefield(P1, &[crab, crab])
        .library(P0, &[CardId::Island; 8])
        .main(P1, Step::PrecombatMain)
        .build();
    let land = hand_card(&game, P1, CardId::Forest);
    submit_ok(&mut game, P1, Action::PlayLand { card: land });
    let mut log = Vec::new();
    settle(&mut game, &mut log, 100);
    assert_eq!(game.state.player(P0).graveyard.len(), 6, "two crabs mill three each");
    println!("PASS ruin_crab_stacks");

    // (d) an opponent with an empty library is milled harmlessly, not killed
    let mut game = fixture_n(213, 3)
        .hand(P1, &[CardId::Forest])
        .battlefield(P1, &[crab])
        .library(P2, &[CardId::Island; 4])
        .main(P1, Step::PrecombatMain)
        .build();
    let land = hand_card(&game, P1, CardId::Forest);
    submit_ok(&mut game, P1, Action::PlayLand { card: land });
    let mut log = Vec::new();
    settle(&mut game, &mut log, 100);
    assert_eq!(game.state.player(P0).graveyard.len(), 0, "P0's library was empty");
    assert_eq!(game.state.player(P2).graveyard.len(), 3);
    assert_eq!(count_milled(&log), 3);
    assert!(!game.state.is_over(), "milling an empty library never ends the game");
    println!("PASS ruin_crab_mill_short_libraries_are_harmless");
}

// ------------------------------------------------------------ 5. Negate restrictions
fn negate_forged_creature_spell_changes_nothing() {
    let negate = card("Negate");
    let mut game = fixture(212)
        .hand(P0, &[negate])
        .mana(P0, ManaPool { blue: 2, ..ManaPool::default() })
        .hand(P1, &[CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::Forest, CardId::Forest])
        .main(P1, Step::PrecombatMain)
        .build();
    let bears = hand_card(&game, P1, CardId::GrizzlyBears);
    submit_ok(&mut game, P1, Action::CastSpell { card: bears, targets: vec![] });
    submit_ok(&mut game, P1, Action::Pass);
    let on_stack = game.state.stack.last().expect("Bears is on the stack").object;
    assert!(
        !game
            .legal_targets(P0, TargetSpec::NoncreatureSpell, ObjectId(u32::MAX))
            .contains(&Target::Spell(on_stack)),
        "a creature spell is never offered"
    );
    let id = hand_card(&game, P0, negate);
    let err = submit_err(
        &mut game,
        P0,
        Action::CastSpell { card: id, targets: vec![Target::Spell(on_stack)] },
    );
    assert!(matches!(err, EngineError::BadTarget { .. }), "got {err:?}");
    assert_eq!(game.state.player(P0).hand.len(), 1, "Negate stayed in hand");
    assert_eq!(game.state.player(P0).mana.blue, 2, "no mana was spent");
    assert_eq!(game.state.stack.len(), 1, "the creature spell is still on the stack");
    println!("PASS negate_forged_creature_spell_changes_nothing");
}

fn negate_refuses_an_ability_on_the_stack() {
    let negate = card("Negate");
    let mut game = fixture(207)
        .hand(P0, &[negate])
        .mana(P0, ManaPool { blue: 2, ..ManaPool::default() })
        .battlefield(P1, &[CardId::SoulWarden])
        .main(P0, Step::PrecombatMain)
        .build();
    let warden = battlefield_card(&game, P1, CardId::SoulWarden);
    // A triggered ability on the stack is not a spell (CR 115.1).
    game.state.stack.push(StackItem {
        object: warden,
        card: CardId::SoulWarden,
        controller: P1,
        targets: Vec::new(),
        cursor: 0,
        entry: StackEntry::Triggered { ability: 0 },
        trigger_answered: true,
    });
    let offered = game.legal_targets(P0, TargetSpec::NoncreatureSpell, ObjectId(u32::MAX));
    assert!(
        !offered.contains(&Target::Spell(warden)),
        "an ability is not a noncreature spell"
    );
    let id = hand_card(&game, P0, negate);
    let err = submit_err(
        &mut game,
        P0,
        Action::CastSpell { card: id, targets: vec![Target::Spell(warden)] },
    );
    assert!(matches!(err, EngineError::BadTarget { .. }), "got {err:?}");
    assert_eq!(game.state.player(P0).hand.len(), 1, "the refused cast changed nothing");
    assert_eq!(game.state.player(P0).mana.blue, 2, "no mana was spent");
    assert_eq!(game.state.stack.len(), 1, "the ability is still on the stack");

    // A noncreature spell is a legal target; a creature spell is not.
    println!("PASS negate_refuses_an_ability_on_the_stack");
}

// ---------------------------------------------------------------- 6. AddMana timing
fn ritual_mana_is_spendable_and_empties_at_step_end() {
    let ritual = card("Pyretic Ritual");
    let mut game = fixture(208)
        .hand(P0, &[ritual, CardId::LightningBolt])
        .mana(P0, ManaPool { red: 2, ..ManaPool::default() })
        .main(P0, Step::PrecombatMain)
        .build();
    let id = hand_card(&game, P0, ritual);
    submit_ok(&mut game, P0, Action::CastSpell { card: id, targets: vec![] });
    let mut log = Vec::new();
    settle(&mut game, &mut log, 200);
    assert_eq!(game.state.player(P0).mana.red, 3, "cost 2 paid, 3 added");
    let added = log
        .iter()
        .find_map(|e| match e {
            Event::ManaAdded { amount, pool_total, .. } => Some((*amount, *pool_total)),
            _ => None,
        })
        .expect("ManaAdded emitted");
    assert_eq!(added, (3, 3), "pool_total is the pool after the addition");

    // CR 500.4: the mana survives priority passing inside the same step.
    let mut log_pre = Vec::new();
    let ok = run_until(&mut game, &mut log_pre, |g| g.state.turn.priority == Some(P0), 32);
    assert!(ok, "P0 never regained priority");
    assert_eq!(
        game.state.player(P0).mana.red,
        3,
        "mana must not be emptied before the step ends"
    );

    // Spend it: Bolt costs {R} out of the pool.
    let bolt = hand_card(&game, P0, CardId::LightningBolt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell { card: bolt, targets: vec![Target::Player(P1)] },
    );
    assert_eq!(game.state.player(P0).mana.red, 2, "the ritual mana really pays a cost");
    let mut log2 = Vec::new();
    settle(&mut game, &mut log2, 200);

    // Now drive to the end of the step and confirm the pool empties with the step.
    let mut log3 = Vec::new();
    let ok = run_until(
        &mut game,
        &mut log3,
        |g| g.state.turn.step != Step::PrecombatMain,
        64,
    );
    assert!(ok, "the step never ended");
    assert!(log3.iter().any(|e| matches!(
        e,
        Event::ManaPoolEmptied { player: P0, amount: 2 }
    )), "2 leftover red must empty with the step (CR 500.4)");
    assert_eq!(game.state.player(P0).mana.red, 0);
    println!("PASS ritual_mana_spendable_then_empties_at_step_end");
}

// -------------------------------------------------------------- 7. Preordain ordering
fn preordain_orders_then_draws() {
    let preordain = card("Preordain");
    let mut game = fixture(209)
        .hand(P0, &[preordain])
        .mana(P0, ManaPool { blue: 1, ..ManaPool::default() })
        .library(P0, &[CardId::Island, CardId::Forest, CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let id = hand_card(&game, P0, preordain);
    submit_ok(&mut game, P0, Action::CastSpell { card: id, targets: vec![] });
    let mut log = Vec::new();
    // Put the first viewed card on the bottom and the second on top, answering
    // the scry decisions in resolution order; everything else uses the scripted
    // policy.
    let mut answers = vec![false, true];
    for _ in 0..40 {
        if game.state.is_over() {
            break;
        }
        let Some(owner) = game.decision_owner() else {
            break;
        };
        let action = match &game.state.pending {
            Some(Pending::OrderCard { .. }) if !answers.is_empty() => {
                Action::OrderCard { to_top: answers.remove(0) }
            }
            _ => auto_answer(&game),
        };
        let events = submit_ok(&mut game, owner, action);
        log.extend(events);
    }
    println!("---- preordain event sequence ({}) ----", log.len());
    for (i, e) in log.iter().enumerate() {
        println!("  [{i}] {e:?}");
    }
    println!("---- library after: {:?} ----", lib(&game, P0));

    let hand: Vec<CardId> = game
        .state
        .player(P0)
        .hand
        .iter()
        .map(|o| game.state.object(*o).card)
        .collect();
    assert_eq!(
        hand,
        vec![CardId::Forest],
        "the scry resolved before the draw: Forest was put on top, Island on the bottom"
    );
    assert_eq!(lib(&game, P0), vec![CardId::Mountain, CardId::Island]);
    let first_scry = log.iter().position(|e| matches!(e, Event::Scried { .. }));
    let first_draw = log.iter().position(|e| matches!(e, Event::Draw { .. }));
    assert!(
        first_scry < first_draw,
        "Scried must precede Draw: scry {first_scry:?} draw {first_draw:?}"
    );
    println!("PASS preordain_orders_then_draws");
}

// -------------------------------------------------- 8. a milled commander is offered CR 903.9a
fn milled_commander_is_offered_the_command_zone() {
    let supplier = card("Stitcher's Supplier");
    let jasmine = CardId::JasmineBoreal;
    let mut game = fixture_n(210, 2)
        .hand(P0, &[supplier])
        .battlefield(P0, &[CardId::Swamp])
        .library(P0, &[jasmine, CardId::Island, CardId::Forest, CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    // Register the top library card as P0's commander.
    let cmdr_id = game.state.player(P0).library[0];
    {
        let obj = object_mut(&mut game.state, cmdr_id);
        obj.commander = Some(CommanderId(0));
    }
    game.state.commanders.push(Commander {
        id: CommanderId(0),
        owner: P0,
        card: jasmine,
        object: cmdr_id,
        casts_from_command_zone: 0,
        return_eligible: false,
    });
    let id = hand_card(&game, P0, supplier);
    submit_ok(&mut game, P0, Action::CastSpell { card: id, targets: vec![] });
    let mut log = Vec::new();
    let saw_choice = run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::CommanderReturn { .. })),
        200,
    );
    assert!(
        saw_choice,
        "HANDOFF.md claims a commander milled out of a library is offered CR 903.9a"
    );
    assert!(grave(&game, P0).contains(&jasmine));
    println!("PASS milled_commander_is_offered_the_command_zone");
}

// ----------------------------------------------------- 9. pool and importer statements
fn pool_statements() {
    assert_eq!(catalog::all().len(), 103, "the pool is 103 cards");
    assert_eq!(catalog::all().len(), 103);
    for name in [
        "Stitcher's Supplier",
        "Ruin Crab",
        "Negate",
        "Preordain",
        "Pyretic Ritual",
        "Seething Song",
    ] {
        assert!(catalog::by_name(name).is_some(), "{name} importable");
    }
    assert!(catalog::by_name("Hedron Crab").is_none(), "target-player mill stays refused");
    let err = DeckList::parse_practice("probe", "1 Hedron Crab\n").unwrap_err();
    assert!(err.to_string().contains("Hedron Crab"));
    assert_eq!(abilities_len("Stitcher's Supplier"), 2);
    assert_eq!(abilities_len("Ruin Crab"), 1);
    println!("PASS pool_statements (103 cards, Hedron Crab refused)");
}

fn abilities_len(name: &str) -> usize {
    aperture_core::ability::abilities(card(name)).len()
}

fn main() {
    short_library_mill();
    empty_library_mill();
    mill_fires_no_battlefield_triggers();
    supplier_mills_again_when_it_dies();
    ruin_crab_landfall_scopes();
    negate_forged_creature_spell_changes_nothing();
    negate_refuses_an_ability_on_the_stack();
    ritual_mana_is_spendable_and_empties_at_step_end();
    preordain_orders_then_draws();
    milled_commander_is_offered_the_command_zone();
    pool_statements();
    println!("\nall probes passed");
}
