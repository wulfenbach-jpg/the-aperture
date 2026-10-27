//! Reviewer probe #2 for slice 33: the CDA survives a state round-trip, and the
//! landfall target list is audited against the printed wording.
mod common;

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

/// A mid-animation state must survive JSON and still answer the layered queries.
#[test]
fn animated_state_round_trips_through_json() {
    let mut game = fixture(9311)
        .battlefield(
            P0,
            &[hospitality(), CardId::Forest, CardId::Forest, CardId::Forest],
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
    assert_eq!(stats(&game, id), (3, 3));

    let json = serde_json::to_string(&game.state).expect("state serialises");
    let loaded: aperture_core::state::GameState =
        serde_json::from_str(&json).expect("state deserialises");
    let reloaded = Game { state: loaded };
    assert_eq!(
        reloaded.state.effects.len(),
        game.state.effects.len(),
        "both continuous effects survive the round-trip"
    );
    assert!(
        reloaded.state.effects.iter().any(|effect| {
            effect.kind == ContinuousKind::PowerToughnessEqualToLands
                && effect.layer == Layer::CharacteristicDefining
                && effect.duration == EffectDuration::WhileSourceOnBattlefield
        }),
        "the 7a effect survives with its layer and printed duration"
    );
    assert!(
        reloaded.is_creature(reloaded.state.object(id)),
        "still a creature after reload"
    );
    assert_eq!(
        stats(&reloaded, id),
        (3, 3),
        "the reloaded state still reads the land count"
    );
}

/// Repro of a conformance gap this probe found: `TargetSpec::CreatureYouControl`
/// excludes the ability's own source, so an animated Beorn's Hospitality is not a
/// legal target for its own landfall trigger — the printed text says only "target
/// creature you control", with no "another".
#[test]
fn landfall_wrongly_excludes_its_own_animated_source() {
    let mut game = fixture(9312)
        .battlefield(
            P0,
            &[
                hospitality(),
                CardId::GrizzlyBears,
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
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    animate(&mut game, P0, id);
    assert!(
        game.is_creature(game.state.object(id)),
        "the source is a creature at this point"
    );

    let offered = game.legal_targets(P0, TargetSpec::CreatureYouControl, id);
    println!("legal targets offered to the trigger: {offered:?}");
    assert!(
        !offered.contains(&Target::Permanent(id)),
        "DEFECT: the animated source is filtered out by the `exclude` argument"
    );
    assert!(offered.contains(&Target::Permanent(bears)));

    let land = hand_card(&game, P0, CardId::Forest);
    let mut log = submit_ok(&mut game, P0, Action::PlayLand { card: land });
    log.extend(drain(&mut game));
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::TriggerStaged { card, .. } if *card == hospitality()
        )),
        "the trigger still resolves against the only object the engine offers it"
    );
    assert_eq!(
        game.state
            .object(bears)
            .counters
            .get(CounterKind::PlusOnePlusOne),
        1,
        "the counter lands on the bears because the source was not offered"
    );
    assert_eq!(
        game.state
            .object(id)
            .counters
            .get(CounterKind::PlusOnePlusOne),
        0,
        "DEFECT: the card could not put its own counter on itself"
    );
}
