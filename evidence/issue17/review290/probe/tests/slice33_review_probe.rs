//! Reviewer probe for slice 33 (Beorn's Hospitality). Out-of-tree; copies of the
//! engine only. Each test asks a behaviour the slice's own tests do not cover.
mod common;

use aperture_core::ability::{self, Ability};
use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn hospitality() -> CardId {
    catalog::by_name("Beorn's Hospitality").expect("supported")
}

fn seven_mana() -> ManaPool {
    ManaPool {
        colorless: 5,
        green: 2,
        ..ManaPool::default()
    }
}

fn drain(g: &mut Game) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..128 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return out;
        }
        let Some(player) = g.decision_owner() else {
            return out;
        };
        out.extend(submit_ok(g, player, Action::Pass));
    }
    out
}

fn animate(g: &mut Game, player: PlayerId, source: ObjectId) -> Vec<Event> {
    let action = g
        .legal_actions()
        .into_iter()
        .find(|offered| {
            matches!(&offered.action, Action::ActivateAbility { source: s, .. } if *s == source)
        })
        .expect("animation offered")
        .action;
    let mut events = submit_ok(g, player, action);
    events.extend(drain(g));
    events
}

fn stats(g: &Game, id: ObjectId) -> (i32, i32) {
    let object = g.state.object(id);
    (object.power(), object.toughness())
}

/// CR 603.3d: a mandatory "target creature you control" trigger with no legal
/// target must be removed, not parked in a pending choice that no one can answer.
#[test]
fn landfall_with_no_legal_target_is_removed_instead_of_pending() {
    let mut game = fixture(9301)
        .battlefield(P0, &[hospitality(), CardId::Forest])
        .hand(P0, &[CardId::Forest])
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build();
    let land = hand_card(&game, P0, CardId::Forest);
    let mut log = submit_ok(&mut game, P0, Action::PlayLand { card: land });
    log.extend(drain(&mut game));

    assert!(
        game.state.pending.is_none(),
        "no pending choice may survive a targetless landfall trigger: {:?}",
        game.state.pending
    );
    let removed = log
        .iter()
        .find_map(|event| match event {
            Event::TriggerRemoved { card, reason, .. } if *card == hospitality() => {
                Some(reason.clone())
            }
            _ => None,
        })
        .expect("the trigger is reported as removed");
    assert!(
        removed.contains("603.3d"),
        "the removal cites the real rule: {removed}"
    );
    assert!(
        !log.iter().any(|event| matches!(
            event,
            Event::TriggerStaged { card, .. } if *card == hospitality()
        )),
        "nothing was staged on the stack"
    );
    // The game is still playable afterwards.
    assert!(!game.state.is_over());
    let owner = game.decision_owner().expect("a seat still decides");
    let _ = game.submit_action(owner, auto_answer(&game));
}

/// The 7a value is a *base*: a 7c anthem and CR 122 counters both sit on top of
/// it, and a later land still moves the base underneath them.
#[test]
fn cda_is_a_base_under_anthem_and_counters() {
    let mut game = fixture(9302)
        .battlefield(
            P0,
            &[
                hospitality(),
                CardId::BeornTheFierce,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .hand(P0, &[CardId::Forest])
        .mana(P0, seven_mana())
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build();
    let id = battlefield_card(&game, P0, hospitality());
    animate(&mut game, P0, id);
    assert_eq!(
        stats(&game, id),
        (5, 5),
        "3 lands (7a) plus Beorn the Fierce's Other Bears anthem (7c)"
    );

    object_mut(&mut game.state, id)
        .counters
        .add(CounterKind::PlusOnePlusOne, 2);
    assert_eq!(stats(&game, id), (7, 7), "two counters on top of 5/5");

    let land = hand_card(&game, P0, CardId::Forest);
    submit_ok(&mut game, P0, Action::PlayLand { card: land });
    let _ = drain(&mut game);
    assert_eq!(
        stats(&game, id),
        (8, 8),
        "the fourth land raises the base to 4, anthem +2, counters +2"
    );
}

/// Activating twice is rules-legal here (no once-per-turn clause) and must not
/// double the value or drop the earlier effect.
#[test]
fn duplicate_animation_is_idempotent() {
    let mut game = fixture(9303)
        .battlefield(
            P0,
            &[
                hospitality(),
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .mana(P0, seven_mana())
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build();
    let id = battlefield_card(&game, P0, hospitality());
    animate(&mut game, P0, id);
    assert_eq!(stats(&game, id), (4, 4));
    game.state.players[0].mana = seven_mana();
    animate(&mut game, P0, id);
    assert_eq!(stats(&game, id), (4, 4), "four lands, not eight");
    let effects = game
        .state
        .effects
        .iter()
        .filter(|effect| effect.target == id)
        .count();
    assert_eq!(effects, 4, "two activations, two effects each");
    assert!(
        game.state
            .effects
            .iter()
            .all(|effect| effect.duration == EffectDuration::WhileSourceOnBattlefield),
        "the printed duration is kept on every copy"
    );
}

fn P0_id() -> PlayerId {
    0
}

/// Once animated, the enchantment is itself a "creature you control", so by the
/// printed wording a lone animated Hospitality is a legal target of its own
/// landfall trigger. The engine filters the source out of
/// `TargetSpec::CreatureYouControl` and therefore removes the trigger instead.
#[test]
fn lone_animated_hospitality_loses_its_own_landfall_trigger() {
    let mut game = fixture(9304)
        .battlefield(P0, &[hospitality(), CardId::Forest, CardId::Forest])
        .hand(P0, &[CardId::Forest])
        .mana(P0, seven_mana())
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .life(P0, 40)
        .life(P1, 40)
        .main(P0, Step::PrecombatMain)
        .build();
    let id = battlefield_card(&game, P0, hospitality());
    animate(&mut game, P0, id);
    assert_eq!(stats(&game, id), (2, 2));
    assert!(game.is_creature(game.state.object(id)));

    let land = hand_card(&game, P0, CardId::Forest);
    let mut log = submit_ok(&mut game, P0, Action::PlayLand { card: land });
    log.extend(drain(&mut game));
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::TriggerRemoved { card, .. } if *card == hospitality()
        )),
        "DEFECT: the rules-legal self target is not offered, so the trigger is removed"
    );
    assert_eq!(
        game.state
            .object(id)
            .counters
            .get(CounterKind::PlusOnePlusOne),
        0,
        "no counter was placed"
    );
    assert_eq!(stats(&game, id), (3, 3), "three lands, no counter");
}

/// The typed ability text is the printed text, and the ability list order is the
/// printed order (trigger first, activated second) so the log reads like the card.
#[test]
fn ability_text_and_order_match_the_printing() {
    let abilities = ability::abilities(hospitality());
    assert!(matches!(abilities[0], Ability::Triggered(_)));
    assert!(matches!(abilities[1], Ability::Activated(_)));
    assert_eq!(
        abilities[1].text(),
        "{5}{G}{G}: This enchantment becomes a Bear creature in addition to its other types and gains \"This creature's power and toughness are each equal to the number of lands you control.\" (This effect doesn't end.)"
    );
}
