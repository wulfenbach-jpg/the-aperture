//! Issue 17 slice 16: two sourced Ayula-list cards that need nothing the pool
//! did not already have except a real "each player who controls ..." token
//! effect, a simultaneous mass destruction, and a targeted untap:
//!
//! * **Fade from History** `{2}{G}{G}` sorcery — "Each player who controls an
//!   artifact or enchantment creates a 2/2 green Bear creature token. Then
//!   destroy all artifacts and enchantments."
//! * **Little Bear** `{2}{G}` 3/2 Bear with flash — "When this creature enters,
//!   untap another target creature you control. If that creature is a Bear, put
//!   a +1/+1 counter on it."
//!
//! Every test drives the real engine: legal actions, validate, cost payment and
//! resolution. Nothing here reads a handoff.

mod common;

use aperture_core::catalog;
use aperture_core::counters::CounterKind;
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

/// Pass priority until the stack and the trigger queue are both empty, keeping
/// every event the engine emitted.
fn drain_stack(game: &mut Game, log: &mut Vec<Event>) {
    for _ in 0..64 {
        if game.state.stack.is_empty() && game.state.trigger_queue.is_empty() {
            return;
        }
        let Some(owner) = game.decision_owner() else {
            return;
        };
        log.extend(submit_ok(game, owner, Action::Pass));
    }
    panic!("stack never drained");
}

fn bears_on(game: &Game, p: PlayerId) -> Vec<ObjectId> {
    game.state
        .battlefield(p)
        .into_iter()
        .filter(|object| object.card == CardId::BearToken)
        .map(|object| object.id)
        .collect()
}

fn counters_on(game: &Game, id: ObjectId) -> u32 {
    game.state
        .object(id)
        .counters
        .get(CounterKind::PlusOnePlusOne)
}

#[test]
fn sourced_slice16_cards_match_their_printings() {
    let fade = card("Fade from History").def();
    assert_eq!(fade.cost, Some("{2}{G}{G}"));
    assert_eq!(fade.type_line, "Sorcery");
    assert_eq!(
        fade.oracle,
        "Each player who controls an artifact or enchantment creates a 2/2 green Bear creature token. Then destroy all artifacts and enchantments."
    );

    let little = card("Little Bear").def();
    assert_eq!(little.cost, Some("{2}{G}"));
    assert_eq!(little.type_line, "Creature — Bear");
    assert_eq!(little.power, Some(3));
    assert_eq!(little.toughness, Some(2));
    assert!(little.has_flash());
    assert!(little.has_subtype("Bear"));
    assert_eq!(
        little.oracle,
        "Flash\nWhen this creature enters, untap another target creature you control. If that creature is a Bear, put a +1/+1 counter on it."
    );
}

#[test]
fn fade_from_history_gives_a_bear_to_each_player_with_an_artifact_or_enchantment_then_destroys_them()
 {
    let fade = card("Fade from History");
    let ring = card("Sol Ring");
    let bearscape = card("Bearscape");
    let grizzly = card("Grizzly Bears");
    let mut game = fixture_n(1616, 4)
        .main(P0, Step::PrecombatMain)
        .mana(P0, green(2, 2))
        .hand(P0, &[fade])
        .battlefield(P0, &[ring])
        .battlefield(P1, &[bearscape])
        .battlefield(2u8, &[grizzly])
        .build();
    let ring_id = battlefield_card(&game, P0, ring);
    let bearscape_id = battlefield_card(&game, P1, bearscape);
    let grizzly_id = battlefield_card(&game, 2u8, grizzly);

    let spell = hand_card(&game, P0, fade);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: spell,
            targets: Vec::new(),
        },
    );
    drain_stack(&mut game, &mut log);

    // CR 111.2: the token is created under the control of the player who
    // controlled the artifact/enchantment, so it is that seat's Bear.
    assert_eq!(bears_on(&game, P0).len(), 1, "P0 controlled Sol Ring");
    assert_eq!(bears_on(&game, P1).len(), 1, "P1 controlled Bearscape");
    assert_eq!(bears_on(&game, 2u8).len(), 0, "P2 controlled no artifact");
    assert_eq!(bears_on(&game, 3u8).len(), 0, "P3 controlled nothing");

    // The tokens are creatures, not artifacts or enchantments, so the second
    // instruction leaves them alone.
    assert!(game.state.is_over() == false);

    // Both permanents really left the battlefield for their owners' graveyards.
    assert!(
        game.state
            .player(P0)
            .graveyard
            .iter()
            .any(|id| game.state.object(*id).card == ring),
        "Sol Ring must be destroyed"
    );
    assert!(
        game.state
            .player(P1)
            .graveyard
            .iter()
            .any(|id| game.state.object(*id).card == bearscape),
        "Bearscape must be destroyed"
    );
    // The engine keeps a non-commander object's id across a zone change, so the
    // honest assertion is the zone, not the id's absence.
    assert_eq!(game.state.object(ring_id).zone, Zone::Graveyard);
    assert_eq!(game.state.object(bearscape_id).zone, Zone::Graveyard);
    // A creature that is neither artifact nor enchantment survives.
    assert_eq!(game.state.object(grizzly_id).zone, Zone::Battlefield);

    // Order matters (CR 608.2c): the token creation resolves before the
    // destruction, so each seat's Bear is not swept away with the permanents.
    let created = log
        .iter()
        .position(|event| matches!(event, Event::PermanentEntered { card, .. } if *card == CardId::BearToken))
        .expect("a Bear token was created");
    let destroyed = log
        .iter()
        .position(|event| matches!(event, Event::Destroyed { card, .. } if *card == ring))
        .expect("Sol Ring was destroyed");
    assert!(created < destroyed, "tokens are created before destruction");
}

#[test]
fn fade_from_history_is_a_sorcery_that_needs_its_printed_cost() {
    let fade = card("Fade from History");
    let mut game = fixture_n(1617, 2)
        .main(P0, Step::PrecombatMain)
        .mana(P0, green(1, 2))
        .hand(P0, &[fade])
        .build();
    let spell = hand_card(&game, P0, fade);
    let err = submit_err(
        &mut game,
        P0,
        Action::CastSpell {
            card: spell,
            targets: Vec::new(),
        },
    );
    assert!(
        matches!(
            err,
            EngineError::CannotPay { .. } | EngineError::Illegal { .. }
        ),
        "one generic short must be refused, got {err}"
    );
    assert_eq!(
        game.state.object(spell).zone,
        Zone::Hand,
        "a refused cast leaves the card in hand"
    );
}

#[test]
fn little_bear_untaps_another_bear_and_counters_it() {
    let little = card("Little Bear");
    let grizzly = card("Grizzly Bears");
    let mut game = fixture_n(1618, 2)
        .main(P0, Step::PrecombatMain)
        .mana(P0, green(2, 1))
        .hand(P0, &[little])
        .battlefield(P0, &[grizzly])
        .build();
    let grizzly_id = battlefield_card(&game, P0, grizzly);
    object_mut(&mut game.state, grizzly_id).tapped = true;

    let spell = hand_card(&game, P0, little);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: spell,
            targets: Vec::new(),
        },
    );
    drain_stack(&mut game, &mut log);

    let bear = game.state.object(grizzly_id);
    assert!(
        !bear.tapped,
        "the only other Bear is untapped by the trigger"
    );
    assert_eq!(
        counters_on(&game, grizzly_id),
        1,
        "a Bear target gets one +1/+1 counter"
    );
    assert!(
        log.iter()
            .any(|event| matches!(event, Event::PermanentEntered { card, .. } if *card == little)),
        "Little Bear really entered the battlefield"
    );
}

#[test]
fn little_bear_untaps_a_non_bear_without_countering_it() {
    let little = card("Little Bear");
    let visionary = card("Elvish Visionary");
    let mut game = fixture_n(1619, 2)
        .main(P0, Step::PrecombatMain)
        .mana(P0, green(2, 1))
        .hand(P0, &[little])
        .battlefield(P0, &[visionary])
        .build();
    let elf_id = battlefield_card(&game, P0, visionary);
    object_mut(&mut game.state, elf_id).tapped = true;

    let spell = hand_card(&game, P0, little);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: spell,
            targets: Vec::new(),
        },
    );
    drain_stack(&mut game, &mut log);

    assert!(!game.state.object(elf_id).tapped);
    assert_eq!(
        counters_on(&game, elf_id),
        0,
        "\"If that creature is a Bear\" must be read, not assumed"
    );
}

#[test]
fn little_bear_cannot_target_a_creature_an_opponent_controls() {
    let little = card("Little Bear");
    let grizzly = card("Grizzly Bears");
    let mut game = fixture_n(1620, 2)
        .main(P0, Step::PrecombatMain)
        .mana(P0, green(2, 1))
        .hand(P0, &[little])
        .battlefield(P1, &[grizzly])
        .build();
    let opponent_bear = battlefield_card(&game, P1, grizzly);

    let spell = hand_card(&game, P0, little);
    let mut log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: spell,
            targets: Vec::new(),
        },
    );
    drain_stack(&mut game, &mut log);

    // CR 603.3d: the trigger requires a target and none of P0's creatures is
    // one, so it is removed instead of touching the opponent's Bear.
    assert!(
        log.iter().any(|event| matches!(event, Event::TriggerRemoved { reason, .. } if reason.contains("CR 603.3d"))),
        "the trigger is removed for want of a legal target"
    );
    assert_eq!(game.state.object(opponent_bear).zone, Zone::Battlefield);
    assert_eq!(counters_on(&game, opponent_bear), 0);
}

#[test]
fn little_bear_flash_casts_at_instant_timing() {
    let little = card("Little Bear");
    let mut game = fixture_n(1621, 2)
        .main(P1, Step::PrecombatMain)
        .mana(P0, green(2, 1))
        .hand(P0, &[little])
        .build();
    set_priority(&mut game.state, Some(P0));
    let spell = hand_card(&game, P0, little);
    let log = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: spell,
            targets: Vec::new(),
        },
    );
    assert!(
        log.iter()
            .any(|event| matches!(event, Event::Cast { card, .. } if *card == little)),
        "flash lets a creature spell be cast while another seat's main phase is active"
    );
}
