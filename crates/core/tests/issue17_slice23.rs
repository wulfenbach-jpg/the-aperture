//! Issue 17 slice 23 — Goreclaw, Terror of Qal Sisma.
//!
//! Sourced printing: CMM 293, `{3}{G}` Legendary Creature — Bear 4/3 (see
//! `fixtures/starter-corpus/slice23-sourced.json`). The card prints two
//! abilities:
//!
//! * "Creature spells you cast with power 4 or greater cost {2} less to cast."
//!   — a static cost reduction (CR 601.2f / 118.7a), the pool's first.
//! * "Whenever Goreclaw attacks, each creature you control with power 4 or
//!   greater gets +1/+1 and gains trample until end of turn." — an attack
//!   trigger (CR 508.1) whose effect is a layer 7c pump plus the pool's first
//!   layer 6 ability grant (CR 613.4d).
//!
//! The tests below discriminate the reduction from "any creature spell", from
//! an opponent's Goreclaw, and from a creature below the printed power
//! threshold; and they prove the granted trample by running a real combat that
//! assigns excess damage to the defending player, plus a control combat at the
//! same board without the trigger.

mod common;

use aperture_core::catalog;
use aperture_core::layer::{ContinuousKind, Layer};
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

/// Object ids that currently carry a layer 6 trample grant.
fn trample_grants(game: &Game) -> Vec<ObjectId> {
    game.state
        .effects
        .iter()
        .filter(|effect| effect.kind == ContinuousKind::GrantsTrample)
        .map(|effect| effect.target)
        .collect()
}

fn layer_six_events(log: &[Event]) -> usize {
    log.iter()
        .filter(|event| {
            matches!(
                event,
                Event::ContinuousEffectStarted { layer, .. } if layer == Layer::Ability.label()
            )
        })
        .count()
}

/// P0 controls Goreclaw; `hand` is what P0 may cast; `mana` is P0's pool.
fn with_goreclaw(seed: u64, extra_battlefield: &[CardId], hand: &[CardId], mana: ManaPool) -> Game {
    let mut battlefield = vec![CardId::GoreclawTerrorOfQalSisma];
    battlefield.extend_from_slice(extra_battlefield);
    fixture(seed)
        .battlefield(P0, &battlefield)
        .hand(P0, hand)
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .mana(P0, mana)
        .main(P0, Step::PrecombatMain)
        .build()
}

/// The same board with no Goreclaw: the control for every cost assertion.
fn without_goreclaw(seed: u64, hand: &[CardId], mana: ManaPool) -> Game {
    fixture(seed)
        .hand(P0, hand)
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .mana(P0, mana)
        .main(P0, Step::PrecombatMain)
        .build()
}

fn cast(game: &mut Game, id: ObjectId) -> Result<Vec<Event>, EngineError> {
    let player = game.decision_owner().expect("priority");
    game.submit_action(
        player,
        Action::CastSpell {
            card: id,
            targets: Vec::new(),
        },
    )
}

#[test]
fn goreclaw_is_registered_with_its_printed_characteristics() {
    let goreclaw = card("Goreclaw, Terror of Qal Sisma");
    let def = goreclaw.def();
    assert_eq!(def.name, "Goreclaw, Terror of Qal Sisma");
    assert_eq!(def.cost, Some("{3}{G}"));
    assert_eq!(def.mana_value(), 4);
    assert_eq!(def.type_line, "Legendary Creature — Bear");
    assert_eq!(def.power, Some(4));
    assert_eq!(def.toughness, Some(3));
    assert!(def.is_creature());
    assert!(!def.has_trample(), "Goreclaw prints no trample itself");
    assert_eq!(
        def.oracle,
        "Creature spells you cast with power 4 or greater cost {2} less to cast.\n\
         Whenever Goreclaw attacks, each creature you control with power 4 or \
         greater gets +1/+1 and gains trample until end of turn."
    );
    // The static half is declared, and it is a cost-reduction ability rather
    // than a layer effect.
    let statics = aperture_core::ability::static_abilities(goreclaw);
    assert_eq!(statics.len(), 1);
    assert_eq!(statics[0].layer(), None);
}

#[test]
fn a_qualifying_creature_spell_costs_two_less_generic() {
    // Craw Wurm is {4}{G}{G}; with Goreclaw it must be castable from {2}{G}{G}.
    let mut game = with_goreclaw(2301, &[], &[CardId::CrawWurm], green(2, 2));
    let wurm = hand_card(&game, P0, CardId::CrawWurm);
    let log = cast(&mut game, wurm).expect("the reduced cost is payable");
    assert!(
        log.iter().any(|event| matches!(event, Event::Cast { .. })),
        "the reduced spell was actually cast: {log:?}"
    );

    // Control: the same pool without Goreclaw cannot pay {4}{G}{G}.
    let mut control = without_goreclaw(2301, &[CardId::CrawWurm], green(2, 2));
    let wurm = hand_card(&control, P0, CardId::CrawWurm);
    assert!(
        cast(&mut control, wurm).is_err(),
        "without the static the full six mana must be required"
    );
}

#[test]
fn the_reduction_stops_at_the_colored_pips() {
    // Alpine Grizzly is {2}{G}: a {2} reduction takes the generic part to zero,
    // so one green mana is enough (CR 118.7a never reduces a colored pip).
    let mut game = with_goreclaw(2302, &[], &[CardId::AlpineGrizzly], green(0, 1));
    let grizzly = hand_card(&game, P0, CardId::AlpineGrizzly);
    cast(&mut game, grizzly).expect("generic is fully reduced");

    let mut control = without_goreclaw(2302, &[CardId::AlpineGrizzly], green(0, 1));
    let grizzly = hand_card(&control, P0, CardId::AlpineGrizzly);
    assert!(
        cast(&mut control, grizzly).is_err(),
        "control: the printed {{2}}{{G}} cost needs three mana without the reduction"
    );
}

#[test]
fn a_creature_below_the_power_threshold_is_not_reduced() {
    // Grizzly Bears is a 2/2 for {1}{G}, so the reduction must not apply.
    let mut game = with_goreclaw(2303, &[], &[CardId::GrizzlyBears], green(0, 1));
    let bears = hand_card(&game, P0, CardId::GrizzlyBears);
    assert!(
        cast(&mut game, bears).is_err(),
        "a 2/2 is below the printed power threshold"
    );
    // The same board with the printed cost available does cast it.
    let mut paid = with_goreclaw(2303, &[], &[CardId::GrizzlyBears], green(1, 1));
    let bears = hand_card(&paid, P0, CardId::GrizzlyBears);
    cast(&mut paid, bears).expect("the unreduced cost is payable");
}

#[test]
fn the_reduction_never_applies_to_a_noncreature_spell() {
    // Dancing from Dark to Dawn is an enchantment for {3}{G}{G}: its generic 3
    // stays 3 even while its controller has Goreclaw.
    let mut game = with_goreclaw(2304, &[], &[CardId::DancingFromDarkToDawn], green(1, 2));
    let enchantment = hand_card(&game, P0, CardId::DancingFromDarkToDawn);
    assert!(
        cast(&mut game, enchantment).is_err(),
        "a noncreature spell is outside the printed wording"
    );
    let mut paid = with_goreclaw(2304, &[], &[CardId::DancingFromDarkToDawn], green(3, 2));
    let enchantment = hand_card(&paid, P0, CardId::DancingFromDarkToDawn);
    cast(&mut paid, enchantment).expect("full cost is payable");
}

#[test]
fn an_opponents_goreclaw_does_not_reduce_your_cost() {
    let mut game = fixture(2305)
        .battlefield(P1, &[CardId::GoreclawTerrorOfQalSisma])
        .hand(P0, &[CardId::CrawWurm])
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .mana(P0, green(2, 2))
        .main(P0, Step::PrecombatMain)
        .build();
    let wurm = hand_card(&game, P0, CardId::CrawWurm);
    assert!(
        cast(&mut game, wurm).is_err(),
        "the static reduces only its controller's creature spells"
    );
}

/// Declares `attackers` for P0 against P1, lets the attack trigger resolve, then
/// answers the blocker step with `blocks` (possibly none) and resolves combat
/// damage. When the defending seat controls no creature the engine skips the
/// blocker step, so this helper tolerates both paths.
fn attack_and_block(
    game: &mut Game,
    attackers: &[ObjectId],
    blocks: &[(ObjectId, ObjectId)],
) -> Vec<Event> {
    game.state.turn.active = P0;
    game.state.turn.step = Step::DeclareAttackers;
    game.state.turn.priority = None;
    game.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    let mut log = submit_ok(
        game,
        P0,
        Action::DeclareAttackers {
            attackers: attackers.to_vec(),
            defender: Some(P1),
        },
    );
    for _ in 0..12 {
        if game.state.turn.combat_damage_done {
            break;
        }
        match game.state.pending.clone() {
            Some(Pending::DeclareBlockers { .. }) => {
                log.extend(submit_ok(
                    game,
                    P1,
                    Action::DeclareBlockers {
                        blocks: blocks
                            .iter()
                            .map(|(blocker, attacker)| BlockDecl {
                                blocker: *blocker,
                                attacker: *attacker,
                            })
                            .collect(),
                    },
                ));
            }
            Some(other) => panic!("unexpected pending choice during combat: {other:?}"),
            None => {
                let Some(player) = game.decision_owner() else {
                    break;
                };
                log.extend(submit_ok(game, player, Action::Pass));
            }
        }
    }
    assert!(
        game.state.turn.combat_damage_done,
        "combat damage was reached: pending {:?}",
        game.state.pending
    );
    log
}

#[test]
fn the_attack_trigger_boosts_and_tramples_only_qualifying_creatures() {
    let mut game = fixture(2306)
        .battlefield(
            P0,
            &[
                CardId::GoreclawTerrorOfQalSisma,
                CardId::CrawWurm,
                CardId::GrizzlyBears,
            ],
        )
        .battlefield(P1, &[CardId::CrawWurm])
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let goreclaw = battlefield_card(&game, P0, CardId::GoreclawTerrorOfQalSisma);
    let wurm = battlefield_card(&game, P0, CardId::CrawWurm);
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let their_wurm = battlefield_card(&game, P1, CardId::CrawWurm);
    assert_eq!(game.state.object(wurm).power(), 6);

    let log = attack_and_block(&mut game, &[goreclaw], &[]);

    assert_eq!(
        log.iter()
            .filter(|event| matches!(
                event,
                Event::TriggerStaged {
                    card: CardId::GoreclawTerrorOfQalSisma,
                    ..
                }
            ))
            .count(),
        1,
        "one attack trigger for the attacking Goreclaw"
    );
    assert_eq!(
        layer_six_events(&log),
        2,
        "Craw Wurm and Goreclaw itself both have power 4 or greater"
    );
    // Craw Wurm is a printed 6/4, so the pump makes it 7/5.
    assert_eq!(
        game.state.object(wurm).power(),
        7,
        "+1/+1 until end of turn"
    );
    assert_eq!(game.state.object(wurm).toughness(), 5);
    assert_eq!(
        game.state.object(goreclaw).power(),
        5,
        "Goreclaw's own power is 4, so it qualifies too"
    );
    let grants = trample_grants(&game);
    assert!(grants.contains(&wurm) && grants.contains(&goreclaw));
    assert!(
        !grants.contains(&bears),
        "a 2/2 is below the threshold and gets nothing"
    );
    assert_eq!(game.state.object(bears).power(), 2);
    assert!(
        !grants.contains(&their_wurm),
        "the effect never reaches another seat's creatures"
    );
    assert_eq!(game.state.object(their_wurm).power(), 6);
}

#[test]
fn the_trigger_fires_only_when_goreclaw_itself_attacks() {
    let mut game = fixture(2307)
        .battlefield(
            P0,
            &[CardId::GoreclawTerrorOfQalSisma, CardId::GrizzlyBears],
        )
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let goreclaw = battlefield_card(&game, P0, CardId::GoreclawTerrorOfQalSisma);
    let bears = battlefield_card(&game, P0, CardId::GrizzlyBears);
    let log = attack_and_block(&mut game, &[bears], &[]);
    assert!(
        !log.iter().any(|event| matches!(
            event,
            Event::TriggerStaged {
                card: CardId::GoreclawTerrorOfQalSisma,
                ..
            }
        )),
        "another creature attacking does not trigger the printed wording"
    );
    assert!(trample_grants(&game).is_empty());
    assert_eq!(game.state.object(goreclaw).power(), 4);
}

#[test]
fn granted_trample_carries_excess_damage_to_the_defending_player() {
    // P0: Goreclaw + Craw Wurm. P1 blocks both with 2/2s, so both attackers are
    // blocked. The 7/7 trampling Wurm must assign 2 lethal to its blocker and 5
    // to P1; the blocked Goreclaw assigns nothing to P1.
    let mut game = fixture(2308)
        .battlefield(P0, &[CardId::GoreclawTerrorOfQalSisma, CardId::CrawWurm])
        .battlefield(P1, &[CardId::GrizzlyBears, CardId::GrizzlyBears])
        .life(P0, 40)
        .life(P1, 40)
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let goreclaw = battlefield_card(&game, P0, CardId::GoreclawTerrorOfQalSisma);
    let wurm = battlefield_card(&game, P0, CardId::CrawWurm);
    // Two 2/2 blockers: the first added object blocks the Wurm.
    let first_bear = game.state.players[P1 as usize].hand.iter().copied().next();
    assert!(
        first_bear.is_none(),
        "no hand cards are needed for the block"
    );
    let blockers: Vec<ObjectId> = game
        .state
        .objects
        .values()
        .filter(|object| {
            object.zone == Zone::Battlefield
                && object.controller == P1
                && object.card == CardId::GrizzlyBears
        })
        .map(|object| object.id)
        .collect();
    assert_eq!(blockers.len(), 2);
    let log = attack_and_block(
        &mut game,
        &[goreclaw, wurm],
        &[(blockers[0], wurm), (blockers[1], goreclaw)],
    );
    assert!(
        log.iter()
            .any(|event| matches!(event, Event::CombatDamage { .. })),
        "combat damage was dealt: {log:?}"
    );
    // Both attackers qualify (Goreclaw is a 4/3 pumped to 5/4, the Wurm a 6/4
    // pumped to 7/5), both are blocked by a 2/2, and both trample: 5 - 2 = 3 and
    // 7 - 2 = 5 reach P1, for 8 total.
    assert_eq!(
        game.state.player(P1).life,
        32,
        "both trampling attackers push their excess damage through"
    );
}

#[test]
fn the_same_combat_without_goreclaw_deals_no_damage_to_the_player() {
    // Control for the test above: identical board and blocks, no Goreclaw, so no
    // trample grant and every point of damage stops at the blocker.
    let mut game = fixture(2309)
        .battlefield(P0, &[CardId::CrawWurm])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .life(P0, 40)
        .life(P1, 40)
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let wurm = battlefield_card(&game, P0, CardId::CrawWurm);
    let blocker = battlefield_card(&game, P1, CardId::GrizzlyBears);
    let log = attack_and_block(&mut game, &[wurm], &[(blocker, wurm)]);
    assert!(
        log.iter()
            .any(|event| matches!(event, Event::CombatDamage { .. })),
        "the blocker is damaged"
    );
    assert_eq!(
        game.state.player(P1).life,
        40,
        "without trample the excess damage never reaches the player"
    );
}

#[test]
fn the_granted_trample_expires_in_cleanup() {
    let mut game = fixture(2310)
        .battlefield(P0, &[CardId::GoreclawTerrorOfQalSisma, CardId::CrawWurm])
        .library(P0, &[CardId::Forest; 12])
        .library(P1, &[CardId::Forest; 12])
        .main(P0, Step::PrecombatMain)
        .build();
    let goreclaw = battlefield_card(&game, P0, CardId::GoreclawTerrorOfQalSisma);
    let wurm = battlefield_card(&game, P0, CardId::CrawWurm);
    attack_and_block(&mut game, &[goreclaw], &[]);
    assert!(!trample_grants(&game).is_empty());

    // CR 514.2: the until-end-of-turn effects are gone once the cleanup step
    // begins, which happens when the end step advances.
    game.state.turn.step = Step::EndStep;
    game.state.turn.priority = Some(P0);
    game.state.turn.passed = 0;
    for _ in 0..8 {
        if game.state.turn.step == Step::Cleanup || game.state.is_over() {
            break;
        }
        let Some(player) = game.decision_owner() else {
            break;
        };
        let _ = game.submit_action(player, Action::Pass);
    }
    assert_ne!(
        game.state.turn.step,
        Step::EndStep,
        "the end step ended and cleanup ran"
    );
    assert!(
        trample_grants(&game).is_empty(),
        "no until-end-of-turn grant survives cleanup"
    );
    assert_eq!(
        game.state.object(wurm).power(),
        6,
        "the pump expires with the grant"
    );
}
