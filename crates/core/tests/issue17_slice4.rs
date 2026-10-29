//! Issue 17 slice 4: sourced Ayula/Isamaru package.
//!
//! Four cards added to the supported catalog — Ordinary Bear (vanilla),
//! Ashcoat Bear (Flash), Owlbear (Trample plus an enters trigger) and Soul
//! Warden ("another creature enters") — together with the two core primitives
//! they need. Every assertion drives a real engine action; nothing here reads a
//! field back to itself.

mod common;

use aperture_core::ability::{TriggerEvent, abilities, triggers_on};
use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

#[test]
fn sourced_slice4_cards_match_their_live_printings() {
    let bear = card("Ordinary Bear").def();
    assert_eq!(bear.cost, Some("{3}{G}"));
    assert_eq!(bear.type_line, "Creature — Bear");
    assert_eq!((bear.power, bear.toughness), (Some(4), Some(5)));
    assert_eq!(bear.oracle, "", "Ordinary Bear prints no rules text");

    let ashcoat = card("Ashcoat Bear").def();
    assert_eq!(ashcoat.cost, Some("{1}{G}"));
    assert_eq!(
        ashcoat.oracle,
        "Flash (You may cast this spell any time you could cast an instant.)"
    );
    assert!(ashcoat.has_flash(), "printed Flash is modelled");
    assert!(!ashcoat.has_trample());
    assert_eq!((ashcoat.power, ashcoat.toughness), (Some(2), Some(2)));

    let owlbear = card("Owlbear").def();
    assert_eq!(owlbear.cost, Some("{3}{G}{G}"));
    assert_eq!(owlbear.type_line, "Creature — Bird Bear");
    assert_eq!(
        owlbear.oracle,
        "Trample\nKeen Senses — When this creature enters, draw a card."
    );
    assert!(owlbear.has_trample(), "printed Trample is modelled");
    assert!(!owlbear.has_flash());
    assert_eq!((owlbear.power, owlbear.toughness), (Some(4), Some(4)));

    let warden = card("Soul Warden").def();
    assert_eq!(warden.cost, Some("{W}"));
    assert_eq!(
        warden.oracle,
        "Whenever another creature enters, you gain 1 life."
    );
    assert_eq!((warden.power, warden.toughness), (Some(1), Some(1)));
    assert!(warden.power.is_some() && warden.spell.is_none());
}

#[test]
fn ordinary_bear_is_a_vanilla_creature_with_no_abilities() {
    let bear = card("Ordinary Bear");
    assert!(abilities(bear).is_empty(), "vanilla means no typed ability");
    assert_eq!(
        bear.def().spell_spec().map(|s| s.target),
        Some(TargetSpec::None)
    );
    let mut game = fixture(70)
        .hand(P0, &[bear])
        .battlefield(P0, &[CardId::Forest; 6])
        .main(P0, Step::PrecombatMain)
        .build();
    let id = hand_card(&game, P0, bear);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: id,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    let permanent = battlefield_card(&game, P0, bear);
    assert_eq!(game.state.object(permanent).power(), 4);
    assert_eq!(game.state.object(permanent).toughness(), 5);
}

#[test]
fn printed_flash_allows_a_creature_spell_outside_the_own_main_phase() {
    // P1 is the active player in their precombat main; P0 receives priority
    // there, which is an instant-timing window (CR 117.1a).
    let mut game = fixture(71)
        .hand(P0, &[CardId::AshcoatBear])
        .battlefield(P0, &[CardId::Forest; 4])
        .main(P1, Step::PrecombatMain)
        .build();
    set_priority(&mut game.state, Some(P0));
    let flash_bear = hand_card(&game, P0, CardId::AshcoatBear);
    let events = submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: flash_bear,
            targets: vec![],
        },
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::Cast { card, .. } if card.name() == "Ashcoat Bear")),
        "the flash creature spell reached the stack outside P0's main phase"
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    let permanent = battlefield_card(&game, P0, CardId::AshcoatBear);
    assert_eq!(game.state.object(permanent).zone, Zone::Battlefield);
    assert_eq!(game.state.object(permanent).power(), 2);
}

#[test]
fn a_creature_without_flash_is_refused_at_instant_timing() {
    let mut game = fixture(72)
        .hand(P0, &[CardId::GrizzlyBears])
        .battlefield(P0, &[CardId::Forest; 4])
        .main(P1, Step::PrecombatMain)
        .build();
    set_priority(&mut game.state, Some(P0));
    let bears = hand_card(&game, P0, CardId::GrizzlyBears);
    let err = submit_err(
        &mut game,
        P0,
        Action::CastSpell {
            card: bears,
            targets: vec![],
        },
    );
    assert!(matches!(err, EngineError::BadTiming { .. }), "{err}");
    assert_eq!(
        game.state.object(bears).zone,
        Zone::Hand,
        "the refused spell never left the hand"
    );
}

#[test]
fn trample_assigns_only_lethal_damage_to_the_blocker() {
    // Owlbear is a 4/4 with trample; the 1/1 blocker dies and the remaining 3
    // damage goes to the defending player (CR 702.19b, 510.1c).
    let mut game = fixture(73)
        .battlefield(P0, &[CardId::Owlbear])
        .battlefield(P1, &[CardId::WillowElf])
        .main(P0, Step::PrecombatMain)
        .build();
    let owlbear = battlefield_card(&game, P0, CardId::Owlbear);
    let elf = battlefield_card(&game, P1, CardId::WillowElf);
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
            attackers: vec![owlbear],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.pending.is_some() && g.state.turn.step == Step::DeclareBlockers,
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: elf,
                attacker: owlbear,
            }],
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.player(P1).life == 17,
        64
    ));
    assert_eq!(
        game.state.battlefield(P1).len(),
        0,
        "the 1/1 blocker took lethal damage"
    );
    assert!(
        log.iter().any(|e| matches!(
            e,
            Event::CombatDamage {
                amount: 1,
                blocker: Some(_),
                ..
            }
        )),
        "exactly lethal damage was assigned to the blocker"
    );
    assert!(
        log.iter().any(|e| matches!(
            e,
            Event::CombatDamage {
                amount: 3,
                blocker: None,
                ..
            }
        )),
        "the 3 excess damage trampled over to the player"
    );
}

#[test]
fn a_non_trample_attacker_never_damages_the_player_through_a_blocker() {
    // Control for the trample rule: the same 4 power without trample assigns all
    // of it to the blocker and none to the player.
    let mut game = fixture(74)
        .battlefield(P0, &[CardId::OrdinaryBear])
        .battlefield(P1, &[CardId::WillowElf])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::OrdinaryBear);
    let elf = battlefield_card(&game, P1, CardId::WillowElf);
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
            attackers: vec![bear],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.pending.is_some() && g.state.turn.step == Step::DeclareBlockers,
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: elf,
                attacker: bear,
            }],
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.battlefield(P1).is_empty(),
        64
    ));
    assert_eq!(
        game.state.player(P1).life,
        20,
        "no trample, no player damage"
    );
    assert!(
        log.iter()
            .filter(|e| matches!(e, Event::CombatDamage { blocker: None, .. }))
            .all(|e| !matches!(e, Event::CombatDamage { amount: 4, .. })),
        "the 4 damage was assigned to the blocker, not trampled over"
    );
}

#[test]
fn owlbear_draws_a_card_when_it_enters() {
    let mut game = fixture(75)
        .hand(P0, &[CardId::Owlbear])
        .battlefield(P0, &[CardId::Forest; 6])
        .library(P0, &[CardId::Forest; 4])
        .main(P0, Step::PrecombatMain)
        .build();
    let before = game.state.player(P0).hand_size();
    let id = hand_card(&game, P0, CardId::Owlbear);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: id,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.player(P0).hand_size() == before,
        8
    ));
    assert!(
        log.iter()
            .any(|e| matches!(e, Event::Draw { player, .. } if *player == P0)),
        "the enters trigger drew a card"
    );
    let permanent = battlefield_card(&game, P0, CardId::Owlbear);
    assert_eq!(game.state.object(permanent).zone, Zone::Battlefield);
}

#[test]
fn soul_warden_gains_life_only_for_another_creature() {
    let warden = card("Soul Warden");
    assert!(
        triggers_on(warden, TriggerEvent::AnotherCreatureEnters)
            .next()
            .is_some(),
        "Soul Warden prints an another-creature-enters trigger"
    );
    // Casting a creature while a Warden is already on the battlefield gains 1.
    let mut game = fixture(76)
        .hand(P0, &[CardId::GrizzlyBears])
        .battlefield(
            P0,
            &[CardId::Forest, CardId::Forest, CardId::Forest, warden],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    assert_eq!(game.state.player(P0).life, 20);
    let bears = hand_card(&game, P0, CardId::GrizzlyBears);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: bears,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.player(P0).life == 21,
        64
    ));
    assert!(
        log.iter()
            .any(|e| matches!(e, Event::LifeChange { delta: 1, .. }))
    );
}

#[test]
fn soul_warden_does_not_trigger_for_itself() {
    let warden = card("Soul Warden");
    let mut game = fixture(77)
        .hand(P0, &[warden])
        .battlefield(P0, &[CardId::Plains])
        .main(P0, Step::PrecombatMain)
        .build();
    let id = hand_card(&game, P0, warden);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: id,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.battlefield(P0).iter().any(|o| o.card == warden),
        64
    ));
    assert_eq!(
        game.state.player(P0).life,
        20,
        "the printed word is 'another': the Warden never triggers for itself"
    );
}

#[test]
fn soul_warden_triggers_for_an_opponents_creature_too() {
    // The trigger belongs to the Warden's controller, not the entering
    // creature's controller.
    let mut game = fixture(78)
        .battlefield(P0, &[card("Soul Warden")])
        .hand(P1, &[CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::Forest; 3])
        .main(P1, Step::PrecombatMain)
        .build();
    let bears = hand_card(&game, P1, CardId::GrizzlyBears);
    submit_ok(
        &mut game,
        P1,
        Action::CastSpell {
            card: bears,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P1, Action::Pass);
    submit_ok(&mut game, P0, Action::Pass);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.player(P0).life == 21,
        64
    ));
    assert_eq!(
        game.state.player(P1).life,
        20,
        "the entering creature's controller gains nothing"
    );
}

// ---------------------------------------------------------------- review R211-1

/// R211-1: a creature token entering is "another creature enters" for Soul
/// Warden. Goblin Instigator's own entry and its Goblin token are two separate
/// entries, so a Warden on the battlefield gains exactly 2 life.
#[test]
fn soul_warden_triggers_for_creature_tokens_too() {
    let warden = card("Soul Warden");
    let mut game = fixture(79)
        .hand(P0, &[CardId::GoblinInstigator])
        .battlefield(P0, &[CardId::Mountain, CardId::Mountain, warden])
        .main(P0, Step::PrecombatMain)
        .build();
    let instigator = hand_card(&game, P0, CardId::GoblinInstigator);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: instigator,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.player(P0).life == 22,
        64
    ));
    let goblins = game
        .state
        .battlefield(P0)
        .iter()
        .filter(|object| object.card == CardId::GoblinToken)
        .count();
    assert_eq!(goblins, 1, "the Instigator made one Goblin token");
    let gains = log
        .iter()
        .filter(|event| {
            matches!(
                event,
                Event::LifeChange {
                    player: P0,
                    delta: 1,
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        gains, 2,
        "one trigger for the Instigator and one for its Goblin token"
    );
}

/// Trample with two blockers: lethal to each (1 + 1) and the remaining 2 damage
/// tramples over to the player.
#[test]
fn trample_carries_surplus_across_two_blockers() {
    let mut game = fixture(80)
        .battlefield(P0, &[CardId::Owlbear])
        .battlefield(P1, &[CardId::WillowElf, CardId::WillowElf])
        .main(P0, Step::PrecombatMain)
        .build();
    let owlbear = battlefield_card(&game, P0, CardId::Owlbear);
    let blockers: Vec<ObjectId> = game.state.battlefield(P1).iter().map(|o| o.id).collect();
    assert_eq!(blockers.len(), 2);
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
            attackers: vec![owlbear],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.pending.is_some() && g.state.turn.step == Step::DeclareBlockers,
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: blockers
                .iter()
                .map(|blocker| BlockDecl {
                    blocker: *blocker,
                    attacker: owlbear,
                })
                .collect(),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.player(P1).life == 18,
        64
    ));
    assert!(
        game.state.battlefield(P1).is_empty(),
        "both 1/1 blockers took lethal damage"
    );
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::CombatDamage {
                amount: 2,
                blocker: None,
                ..
            }
        )),
        "the 2 surplus damage trampled over after lethal was assigned twice"
    );
}

/// Trample accounts for damage already marked on a blocker: a 2/2 with 1 damage
/// needs only 1 more, so 3 damage tramples over.
#[test]
fn trample_counts_damage_already_marked_on_the_blocker() {
    let mut game = fixture(81)
        .battlefield(P0, &[CardId::Owlbear])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let owlbear = battlefield_card(&game, P0, CardId::Owlbear);
    let bears = battlefield_card(&game, P1, CardId::GrizzlyBears);
    object_mut(&mut game.state, bears).damage = 1;
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
            attackers: vec![owlbear],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.pending.is_some() && g.state.turn.step == Step::DeclareBlockers,
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker: bears,
                attacker: owlbear,
            }],
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| g.state.player(P1).life == 17,
        64
    ));
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::CombatDamage {
                amount: 3,
                blocker: None,
                ..
            }
        )),
        "only 1 damage was needed for lethal, so 3 trampled over"
    );
}
