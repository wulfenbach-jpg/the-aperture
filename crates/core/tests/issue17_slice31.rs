//! Slice 31: Beorn the Fierce — the pool's first subtype anthem, its first
//! keyword counter and its first "at the beginning of combat on your turn"
//! trigger with an "up to one" target.
//!
//! The printed text is (HOB 119, retrieved 2026-10-27, recorded in
//! `fixtures/starter-corpus/slice31-sourced.json`):
//!
//! "Trample
//! Other Bears you control get +2/+2.
//! At the beginning of combat on your turn, put a trample counter on up to one
//! target creature you control. It becomes a Bear in addition to its other
//! types. Then if you control three or more Bears, draw two cards."
//!
//! The trigger's three instructions resolve in printed order (CR 608.2c): the
//! trample counter (CR 122.1b, a counter that grants the keyword for as long as
//! it stays), the layer 4 addition of the Bear subtype (CR 613.1d, until end of
//! turn because the ability states no duration — CR 611.2a), and the
//! conditional draw taken from the *effective* Bear count. "Up to one" means the
//! controller may decline the target (CR 115.6) and the draw still happens.
//!
//! Every assertion here goes through the engine's own offers, resolutions,
//! combat and cleanup; none reads an internal accessor.
mod common;

use aperture_core::ability::{self, Ability};
use aperture_core::catalog;
use aperture_core::counters::CounterKind;
use aperture_core::layer::SubtypeLine;
use aperture_core::*;
use common::*;

fn beorn() -> CardId {
    catalog::by_name("Beorn the Fierce").expect("Beorn the Fierce must be supported")
}

fn stats(g: &Game, id: ObjectId) -> (i32, i32) {
    let object = g.state.object(id);
    (object.power(), object.toughness())
}

fn trample_counters(g: &Game, id: ObjectId) -> u32 {
    g.state.object(id).counters.get(CounterKind::Trample)
}

/// Pass priority until the table reaches the beginning of combat step and the
/// printed trigger is waiting for its target (or until `limit` passes).
fn pass_to_combat_trigger(g: &mut Game, limit: usize) -> Vec<Event> {
    let mut log = Vec::new();
    for _ in 0..limit {
        if g.state.turn.step == Step::BeginCombat
            && matches!(g.state.pending, Some(Pending::TriggerTarget { .. }))
        {
            return log;
        }
        assert!(!g.state.is_over(), "the game ended before combat began");
        let owner = g.decision_owner().expect("a seat decides while advancing");
        log.extend(submit_ok(g, owner, Action::Pass));
    }
    panic!(
        "the beginning-of-combat trigger never waited for a target (step {:?})",
        g.state.turn.step
    );
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
    panic!("the stack did not settle");
}

#[test]
fn beorn_is_sourced_and_supported() {
    let def = beorn().def();
    assert_eq!(def.name, "Beorn the Fierce");
    assert_eq!(def.cost, Some("{3}{G}{G}"));
    assert_eq!(
        def.type_line,
        "Legendary Creature — Bear Shapeshifter Warrior"
    );
    assert_eq!((def.power, def.toughness), (Some(6), Some(6)));
    assert!(def.has_trample(), "printed Trample");
    assert_eq!(
        def.oracle,
        "Trample\nOther Bears you control get +2/+2.\nAt the beginning of combat on your turn, put a trample counter on up to one target creature you control. It becomes a Bear in addition to its other types. Then if you control three or more Bears, draw two cards."
    );
    let triggered: Vec<_> = ability::abilities(beorn())
        .iter()
        .filter_map(Ability::triggered)
        .collect();
    assert_eq!(triggered.len(), 1, "one printed triggered ability");
    assert_eq!(triggered[0].event, ability::TriggerEvent::BeginningOfCombat);
    assert_eq!(triggered[0].target, TargetSpec::UpToOneCreatureYouControl);
    assert!(!triggered[0].optional, "the trigger is mandatory");
    assert_eq!(triggered[0].effects.len(), 3);
    let statics = ability::static_abilities(beorn());
    assert_eq!(statics.len(), 1);
    assert_eq!(statics[0].text(), "Other Bears you control get +2/+2.");
}

#[test]
fn trigger_offers_up_to_one_target_including_declining() {
    let mut g = fixture(3101)
        .battlefield(P0, &[beorn(), CardId::EliteVanguard])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let beorn_id = battlefield_card(&g, P0, beorn());
    let other = battlefield_card(&g, P0, CardId::EliteVanguard);
    let log = pass_to_combat_trigger(&mut g, 32);
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::StepBegin {
                step: Step::BeginCombat
            }
        )),
        "the trigger waits for its target at the beginning of combat step: {log:?}"
    );
    assert!(
        matches!(
            g.state.pending,
            Some(Pending::TriggerTarget { player: P0, .. })
        ),
        "the controller chooses the target: {:?}",
        g.state.pending
    );
    let offers: Vec<Action> = g.legal_actions().into_iter().map(|a| a.action).collect();
    assert!(
        offers.contains(&Action::ChooseTriggerTarget {
            target: Target::Permanent(other)
        }),
        "the other creature is a legal target"
    );
    assert!(
        offers.contains(&Action::ChooseTriggerTarget {
            target: Target::Permanent(beorn_id)
        }),
        "the source itself is a legal target"
    );
    assert!(
        offers.contains(&Action::DeclineTriggerTarget),
        "an \"up to one\" target must offer declining (CR 115.6)"
    );
    // Declining is legal and does nothing to the board; only one Bear is
    // controlled (Beorn), so the conditional draw does not happen.
    let hand_before = g.state.player(P0).hand.len();
    submit_ok(&mut g, P0, Action::DeclineTriggerTarget);
    drain(&mut g);
    assert_eq!(trample_counters(&g, other), 0);
    assert_eq!(stats(&g, other), (2, 1), "no counter, no Bear, no anthem");
    assert_eq!(g.state.player(P0).hand.len(), hand_before);
}

#[test]
fn chosen_creature_gets_a_trample_counter_and_becomes_a_bear() {
    let mut g = fixture(3102)
        .battlefield(P0, &[beorn(), CardId::EliteVanguard])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let beorn_id = battlefield_card(&g, P0, beorn());
    let other = battlefield_card(&g, P0, CardId::EliteVanguard);
    assert_eq!(
        stats(&g, beorn_id),
        (6, 6),
        "the anthem skips its own source"
    );
    pass_to_combat_trigger(&mut g, 32);
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(other),
        },
    );
    drain(&mut g);
    assert_eq!(
        trample_counters(&g, other),
        1,
        "one trample counter (CR 122.1b)"
    );
    // It is a Bear now, so the anthem applies: 2/1 + 2/2 = 4/3. Beorn still
    // does not pump itself.
    assert_eq!(stats(&g, other), (4, 3));
    assert_eq!(stats(&g, beorn_id), (6, 6));
    // The trample counter is a counter, not a continuous effect: nothing was
    // added to `state.effects` for it, and the only type effect is the Bear
    // addition the ability created.
    assert!(
        !g.state
            .effects
            .iter()
            .any(|e| e.kind == ContinuousKind::GrantsTrample && e.target == other),
        "the keyword comes from the counter, not a layer 6 effect"
    );
    assert!(g.state.effects.iter().any(|e| {
        e.kind == ContinuousKind::AddCreatureSubtypes(SubtypeLine::Bear) && e.target == other
    }));
}

#[test]
fn three_bears_draw_two_after_the_target_becomes_a_bear() {
    let mut g = fixture(3103)
        .battlefield(P0, &[beorn(), CardId::EliteVanguard, CardId::RuneclawBear])
        .library(P0, &[CardId::Forest; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let other = battlefield_card(&g, P0, CardId::EliteVanguard);
    let hand_before = g.state.player(P0).hand.len();
    let library_before = g.state.player(P0).library.len();
    // Two Bears before the trigger resolves (Beorn and Runeclaw Bear); the
    // targeted Human becomes the third, so the draw happens.
    pass_to_combat_trigger(&mut g, 32);
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(other),
        },
    );
    drain(&mut g);
    assert_eq!(g.state.player(P0).hand.len(), hand_before + 2);
    assert_eq!(g.state.player(P0).library.len(), library_before - 2);
}

#[test]
fn two_bears_do_not_draw() {
    let mut g = fixture(3104)
        .battlefield(P0, &[beorn(), CardId::EliteVanguard])
        .library(P0, &[CardId::Forest; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let other = battlefield_card(&g, P0, CardId::EliteVanguard);
    let hand_before = g.state.player(P0).hand.len();
    pass_to_combat_trigger(&mut g, 32);
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(other),
        },
    );
    drain(&mut g);
    // Beorn + the newly made Bear = 2 Bears, fewer than three.
    assert_eq!(g.state.player(P0).hand.len(), hand_before);
}

#[test]
fn trample_counter_lets_the_creature_trample_over_a_blocker() {
    let mut g = fixture(3105)
        .battlefield(P0, &[beorn(), CardId::EliteVanguard])
        .battlefield(P1, &[CardId::WillowElf])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let other = battlefield_card(&g, P0, CardId::EliteVanguard);
    let elf = battlefield_card(&g, P1, CardId::WillowElf);
    pass_to_combat_trigger(&mut g, 32);
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(other),
        },
    );
    let mut log = drain(&mut g);
    log.clear();
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![other],
            defender: Some(P1),
        },
    );
    // The defending seat really blocks with the 1/1; an unblocked attacker would
    // deal all four damage and prove nothing about trample.
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.step == Step::DeclareBlockers && g.state.pending.is_some(),
        64
    ));
    submit_ok(
        &mut g,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: elf,
                attacker: other,
            }],
        },
    );
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.combat_damage_done,
        64
    ));
    let life = g.state.player(P1).life;
    assert_eq!(life, 17, "1 damage kills the 1/1 and 3 tramples over (4/3)");
    assert!(
        g.state.player(P1).graveyard.iter().any(|id| g
            .state
            .try_object(*id)
            .is_some_and(|object| object.card == CardId::WillowElf)),
        "the blocking 1/1 dies"
    );
}

#[test]
fn the_bear_type_ends_at_cleanup_but_the_counter_stays() {
    let mut g = fixture(3106)
        .battlefield(P0, &[beorn(), CardId::EliteVanguard])
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let other = battlefield_card(&g, P0, CardId::EliteVanguard);
    pass_to_combat_trigger(&mut g, 32);
    submit_ok(
        &mut g,
        P0,
        Action::ChooseTriggerTarget {
            target: Target::Permanent(other),
        },
    );
    drain(&mut g);
    assert_eq!(stats(&g, other), (4, 3));
    // Advance into the next turn (P1's), which runs P0's cleanup step.
    let start = g.state.turn.number;
    let mut log = Vec::new();
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.number > start,
        512
    ));
    assert_eq!(
        stats(&g, other),
        (2, 1),
        "the layer 4 Bear addition ended with the turn (CR 611.2a, 514.2)"
    );
    assert_eq!(
        trample_counters(&g, other),
        1,
        "the trample counter is permanent (CR 122.1b)"
    );
}

#[test]
fn trigger_fires_only_on_its_controllers_turn() {
    let mut g = fixture(3107)
        .battlefield(P0, &[beorn()])
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let beorn_id = battlefield_card(&g, P0, beorn());
    // Decline on P0's turn so the turn can advance.
    pass_to_combat_trigger(&mut g, 32);
    submit_ok(&mut g, P0, Action::DeclineTriggerTarget);
    drain(&mut g);
    // Walk to P1's turn and through P1's beginning of combat: Beorn is not the
    // active player's permanent, so it must not trigger.
    let start = g.state.turn.number;
    let mut log = Vec::new();
    assert!(run_until(
        &mut g,
        &mut log,
        |g| g.state.turn.number > start && g.state.turn.step == Step::BeginCombat,
        512
    ));
    assert!(
        !log.iter().any(|event| matches!(
            event,
            Event::TriggerStaged { source, .. } if *source == beorn_id
        )),
        "Beorn triggers only during its controller's turn: {log:?}"
    );
    assert_eq!(trample_counters(&g, beorn_id), 0);
}
