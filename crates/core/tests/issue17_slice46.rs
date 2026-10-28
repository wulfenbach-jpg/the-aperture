//! Slice 47: Thijarian Witness — "another creature dies, if it was attacking or
//! blocking alone", exile it and investigate (WHO 111).
//!
//! The behavior asserted here is the printed text, not an approximation of it:
//! the dying creature must be the only attacker (or the only blocker) of the
//! combat, the card that is exiled is *that* card, the Clue token is a real
//! artifact token with a real "{2}, Sacrifice this token: Draw a card" ability,
//! and a death with no combat meaning (a burn spell in a main phase, or a death
//! beside other attackers) does nothing at all.
//!
//! The combat tests drive the real engine: the declare-blockers step begins only
//! after both players pass on an empty stack (CR 508.2), so the harness passes
//! priority instead of asserting an intermediate `pending` the turn structure has
//! not reached yet.

mod common;

use aperture_core::action::BlockDecl;
use aperture_core::*;
use common::*;

fn c47(name: &str) -> CardId {
    catalog::by_name(name).expect("supported card")
}

fn witness() -> CardId {
    c47("Thijarian Witness")
}

fn clue_tokens(g: &Game, p: PlayerId) -> Vec<ObjectId> {
    g.state
        .battlefield(p)
        .iter()
        .filter(|o| o.card == CardId::ClueToken)
        .map(|o| o.id)
        .collect()
}

fn exiled_cards(g: &Game, p: PlayerId) -> Vec<CardId> {
    g.state
        .player(p)
        .exile
        .iter()
        .map(|id| g.state.object(*id).card)
        .collect()
}

/// Pass priority with whoever currently holds it until the turn has left combat
/// with an empty stack. Combat is driven the way the rules drive it — priority
/// rounds between the steps — so the death trigger really resolves through the
/// engine's own stack rather than from a hand-built state.
fn settle_through_combat(g: &mut Game) -> Vec<Event> {
    let mut log = Vec::new();
    for _ in 0..64 {
        if g.state.pending.is_none()
            && g.state.stack.is_empty()
            && g.state.turn.step == Step::PostcombatMain
        {
            return log;
        }
        if g.state.is_over() {
            return log;
        }
        let Some(p) = g.decision_owner() else {
            return log;
        };
        log.extend(submit_ok(g, p, Action::Pass));
    }
    panic!("combat never settled: {:?}", g.state.turn.step);
}

/// Attack with `attackers` from P0, block with `blocks` from P1, then leave the
/// declare-blockers step. Asserts the declare-blockers step is actually reached.
fn attack_and_block(g: &mut Game, attackers: &[ObjectId], blocks: Vec<BlockDecl>) {
    g.state.turn.attackers.clear();
    g.state.turn.blockers.clear();
    g.state.turn.blocked.clear();
    g.state.turn.defender = Some(P1);
    g.state.turn.priority = None;
    g.state.turn.passed = 0;
    g.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    submit_ok(
        g,
        P0,
        Action::DeclareAttackers {
            attackers: attackers.to_vec(),
            defender: Some(P1),
        },
    );
    // CR 508.2: both players must pass before the declare-blockers step begins.
    submit_ok(g, P0, Action::Pass);
    submit_ok(g, P1, Action::Pass);
    assert!(
        matches!(
            g.state.pending,
            Some(Pending::DeclareBlockers { player: P1 })
        ),
        "the declare-blockers step is pending for the defending seat: {:?}",
        g.state.pending
    );
    submit_ok(g, P1, Action::DeclareBlockers { blocks });
}

/// Cast `spell` from `caster` at one battlefield permanent and pass priority
/// until the spell has left the stack. The loop stops as soon as the stack is
/// empty, so it never passes an extra time and accidentally advances the step.
fn cast_at_creature(g: &mut Game, caster: PlayerId, spell: CardId, target: ObjectId) {
    assert_eq!(
        g.decision_owner(),
        Some(caster),
        "the caster must hold priority before casting"
    );
    let action = g
        .legal_actions()
        .into_iter()
        .find(|a| {
            matches!(
                &a.action,
                Action::CastSpell { card, targets }
                    if g.state.object(*card).card == spell
                        && targets == &vec![Target::Permanent(target)]
            )
        })
        .map(|a| a.action)
        .expect("the spell is offered at that permanent");
    submit_ok(g, caster, action);
    for _ in 0..8 {
        if g.state.stack.is_empty() {
            return;
        }
        let p = g
            .decision_owner()
            .expect("someone holds priority while a spell is on the stack");
        submit_ok(g, p, Action::Pass);
    }
    panic!("the spell never left the stack");
}

/// Attacker is the only creature declared as an attacker (CR 508.1, 506.5) and
/// dies to its blocker's combat damage: the printed trigger fires, exiles
/// exactly that card from its owner's graveyard and creates one Clue token for
/// the watcher's controller.
#[test]
fn sole_attacker_dying_in_combat_is_exiled_and_investigated() {
    let mut g = fixture(4701)
        .battlefield(P0, &[witness(), CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::OrdinaryBear])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let bears = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let bear = battlefield_card(&g, P1, CardId::OrdinaryBear);
    object_mut(&mut g.state, bears).summoning_sick = false;
    object_mut(&mut g.state, bear).summoning_sick = false;

    attack_and_block(
        &mut g,
        &[bears],
        vec![BlockDecl {
            blocker: bear,
            attacker: bears,
        }],
    );
    assert_eq!(g.state.turn.attackers, vec![bears], "one attacker, alone");
    let log = settle_through_combat(&mut g);

    // The 2/2 attacker took lethal damage and died; the 4/5 blocker survived.
    assert_ne!(
        g.state.object(bears).zone,
        Zone::Battlefield,
        "the attacker must have left the battlefield"
    );
    assert_eq!(g.state.object(bear).zone, Zone::Battlefield);
    assert!(
        !g.state
            .player(P0)
            .graveyard
            .iter()
            .any(|id| g.state.object(*id).card == CardId::GrizzlyBears),
        "the dead attacker was exiled, so it is not also in the graveyard"
    );
    assert!(
        exiled_cards(&g, P0).contains(&CardId::GrizzlyBears),
        "the dead attacker is exiled from its owner's graveyard: {:?}",
        exiled_cards(&g, P0)
    );
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::StateBasedAction { description }
                if description.contains("Grizzly Bears") && description.contains("is exiled")
        )),
        "the exile is a real event: {log:?}"
    );
    let clues = clue_tokens(&g, P0);
    assert_eq!(clues.len(), 1, "one investigate, one Clue token");
    assert!(
        g.state.object(clues[0]).card.is_token(),
        "the Clue is a token"
    );
    assert_eq!(g.state.object(clues[0]).controller, P0);
}

/// A creature that blocked alone and died to the attacker's combat damage also
/// satisfies the printed condition, whichever player controls the watcher: P0's
/// Witness exiles P1's blocker from P1's graveyard and P0 investigates.
#[test]
fn sole_blocker_dying_in_combat_is_exiled_and_investigated() {
    let mut g = fixture(4702)
        .battlefield(P0, &[witness(), CardId::OrdinaryBear])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let attacker = battlefield_card(&g, P0, CardId::OrdinaryBear);
    let blocker = battlefield_card(&g, P1, CardId::GrizzlyBears);
    object_mut(&mut g.state, attacker).summoning_sick = false;
    object_mut(&mut g.state, blocker).summoning_sick = false;

    attack_and_block(&mut g, &[attacker], vec![BlockDecl { blocker, attacker }]);
    settle_through_combat(&mut g);

    assert_eq!(
        g.state.object(attacker).zone,
        Zone::Battlefield,
        "the 4/5 attacker survives the 2/2 blocker"
    );
    assert_ne!(
        g.state.object(blocker).zone,
        Zone::Battlefield,
        "the blocker took lethal damage and died"
    );
    assert!(
        exiled_cards(&g, P1).contains(&CardId::GrizzlyBears),
        "the blocker is exiled from its owner's graveyard: {:?}",
        exiled_cards(&g, P1)
    );
    assert!(
        !g.state
            .player(P1)
            .graveyard
            .iter()
            .any(|id| g.state.object(*id).card == CardId::GrizzlyBears),
        "a blocking-alone death is exiled, not left in the graveyard"
    );
    assert_eq!(
        clue_tokens(&g, P0).len(),
        1,
        "the trigger's controller investigates, not the dead creature's controller"
    );
    assert!(
        clue_tokens(&g, P1).is_empty(),
        "the dead creature's controller gets no Clue"
    );
}

/// Two attackers: the same death is not "attacking alone", so nothing is exiled
/// and no Clue exists (the control that makes the first test discriminate).
#[test]
fn death_beside_another_attacker_does_not_trigger() {
    let mut g = fixture(4703)
        .battlefield(P0, &[witness(), CardId::GrizzlyBears, CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::OrdinaryBear, CardId::OrdinaryBear])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let first = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let second = g
        .state
        .battlefield(P0)
        .iter()
        .filter(|o| o.card == CardId::GrizzlyBears)
        .map(|o| o.id)
        .find(|id| *id != first)
        .expect("second attacker");
    let b1 = battlefield_card(&g, P1, CardId::OrdinaryBear);
    let b2 = g
        .state
        .battlefield(P1)
        .iter()
        .filter(|o| o.card == CardId::OrdinaryBear)
        .map(|o| o.id)
        .find(|id| *id != b1)
        .expect("second blocker");
    for id in [first, second, b1, b2] {
        object_mut(&mut g.state, id).summoning_sick = false;
    }

    attack_and_block(
        &mut g,
        &[first, second],
        vec![
            BlockDecl {
                blocker: b1,
                attacker: first,
            },
            BlockDecl {
                blocker: b2,
                attacker: second,
            },
        ],
    );
    assert_eq!(g.state.turn.attackers, vec![first, second]);
    settle_through_combat(&mut g);

    for id in [first, second] {
        assert_ne!(
            g.state.object(id).zone,
            Zone::Battlefield,
            "both 2/2 attackers die to their 4/5 blockers"
        );
    }
    assert!(
        !exiled_cards(&g, P0).contains(&CardId::GrizzlyBears),
        "no alone-attacker death, so nothing is exiled: {:?}",
        exiled_cards(&g, P0)
    );
    assert!(
        clue_tokens(&g, P0).is_empty(),
        "no alone-attacker death, so no investigate"
    );
}

/// R325-1 discriminator: two creatures are declared as attackers and the first
/// is killed by a burn spell; only then is the second killed by a *separate*
/// spell in the same combat. The survivor is the only creature left in the live
/// attacker list, so a "live state" read calls it attacking alone and fires —
/// but it was declared beside the other, so under CR 506.5 it never attacked
/// alone. Nothing is exiled and no Clue is created. The shipped two-in-one-batch
/// control cannot tell the two readings apart, because there both attackers die
/// before either leaves combat.
#[test]
fn attacker_killed_after_its_companion_left_combat_is_not_alone() {
    let mut g = fixture(4706)
        .hand(P1, &[CardId::LightningStrike, CardId::LightningStrike])
        .battlefield(P0, &[witness(), CardId::GrizzlyBears, CardId::GrizzlyBears])
        .battlefield(
            P1,
            &[
                CardId::OrdinaryBear,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
            ],
        )
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let first = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let second = g
        .state
        .battlefield(P0)
        .iter()
        .filter(|o| o.card == CardId::GrizzlyBears)
        .map(|o| o.id)
        .find(|id| *id != first)
        .expect("second attacker");
    for id in [first, second] {
        object_mut(&mut g.state, id).summoning_sick = false;
    }

    // P1 has a legal blocker and declines to use it, so the declare-blockers
    // step really happens and the bolts are cast in a real combat.
    attack_and_block(&mut g, &[first, second], vec![]);
    assert_eq!(g.state.turn.attackers, vec![first, second]);
    assert_eq!(
        g.state.turn.declared_attackers,
        vec![first, second],
        "both attackers are in this combat's declaration record"
    );

    // CR 509.2: the active player holds priority after blockers are declared.
    assert_eq!(g.decision_owner(), Some(P0));
    submit_ok(&mut g, P0, Action::Pass);
    assert_eq!(g.decision_owner(), Some(P1));
    cast_at_creature(&mut g, P1, CardId::LightningStrike, first);
    assert_eq!(
        g.state.object(first).zone,
        Zone::Graveyard,
        "the first attacker is dead"
    );
    assert_eq!(
        g.state.turn.step,
        Step::DeclareBlockers,
        "the first death happens inside the same combat"
    );
    assert_eq!(
        g.state.turn.attackers,
        vec![second],
        "the live attacker list now holds only the second attacker"
    );

    // The discriminating death: the survivor is alone in the live list, but was
    // declared beside the first attacker, so the printed condition is false.
    assert_eq!(g.decision_owner(), Some(P0));
    submit_ok(&mut g, P0, Action::Pass);
    assert_eq!(g.decision_owner(), Some(P1));
    cast_at_creature(&mut g, P1, CardId::LightningStrike, second);
    assert_eq!(
        g.state.object(second).zone,
        Zone::Graveyard,
        "the second attacker is dead"
    );
    assert!(
        g.state.turn.attackers.is_empty(),
        "no attacker is left in combat"
    );

    assert!(
        !exiled_cards(&g, P0).contains(&CardId::GrizzlyBears),
        "neither death was an alone-attack death, so nothing is exiled: {:?}",
        exiled_cards(&g, P0)
    );
    assert!(
        clue_tokens(&g, P0).is_empty(),
        "no alone-attack death, so the watcher does not investigate"
    );
}

/// The same declaration reading on the blocking side: two creatures are declared
/// as blockers, one is killed by a burn spell in the declare-blockers step, and
/// the other then dies to combat damage. The survivor is the only blocker left
/// in the live list but was declared beside the other, so it was not "blocking
/// alone" and nothing triggers.
#[test]
fn blocker_killed_after_its_companion_left_combat_is_not_alone() {
    let mut g = fixture(4707)
        .hand(P0, &[CardId::LightningStrike])
        .battlefield(
            P0,
            &[
                witness(),
                CardId::OrdinaryBear,
                CardId::Mountain,
                CardId::Mountain,
            ],
        )
        .battlefield(P1, &[CardId::GrizzlyBears, CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let attacker = battlefield_card(&g, P0, CardId::OrdinaryBear);
    object_mut(&mut g.state, attacker).summoning_sick = false;
    let first = battlefield_card(&g, P1, CardId::GrizzlyBears);
    let second = g
        .state
        .battlefield(P1)
        .iter()
        .filter(|o| o.card == CardId::GrizzlyBears)
        .map(|o| o.id)
        .find(|id| *id != first)
        .expect("second blocker");
    for id in [first, second] {
        object_mut(&mut g.state, id).summoning_sick = false;
    }

    attack_and_block(
        &mut g,
        &[attacker],
        vec![
            BlockDecl {
                blocker: first,
                attacker,
            },
            BlockDecl {
                blocker: second,
                attacker,
            },
        ],
    );
    assert_eq!(
        g.state.turn.declared_blockers,
        vec![first.min(second), first.max(second)],
        "both blockers are in this combat's declaration record"
    );

    // CR 509.2: the active player holds priority after blockers are declared.
    assert_eq!(g.decision_owner(), Some(P0));
    cast_at_creature(&mut g, P0, CardId::LightningStrike, first);
    assert_eq!(
        g.state.object(first).zone,
        Zone::Graveyard,
        "the first blocker is dead"
    );
    settle_through_combat(&mut g);

    assert_ne!(
        g.state.object(second).zone,
        Zone::Battlefield,
        "the second blocker died to combat damage"
    );
    assert!(
        !exiled_cards(&g, P1).contains(&CardId::GrizzlyBears),
        "neither death was a blocking-alone death, so nothing is exiled: {:?}",
        exiled_cards(&g, P1)
    );
    assert!(
        clue_tokens(&g, P0).is_empty(),
        "no blocking-alone death, so the watcher does not investigate"
    );
}

/// A creature that dies outside combat never satisfies "attacking or blocking
/// alone", even though the same watcher is on the battlefield.
#[test]
fn death_outside_combat_does_not_trigger() {
    let mut g = fixture(4704)
        .hand(P1, &[CardId::LightningStrike])
        .battlefield(P0, &[witness(), CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::Mountain; 2])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .main(P0, Step::PrecombatMain)
        .build();
    let bears = battlefield_card(&g, P0, CardId::GrizzlyBears);

    // CR 117.1d: the active player holds priority first. Passing hands it to the
    // opponent, which is the only moment P1 can cast the burn spell — asking for
    // P1's offers while P0 still holds priority finds nothing at all.
    assert_eq!(g.decision_owner(), Some(P0));
    submit_ok(&mut g, P0, Action::Pass);
    assert_eq!(
        g.decision_owner(),
        Some(P1),
        "the opponent now holds priority"
    );

    let bolt = g
        .legal_actions()
        .into_iter()
        .find(|a| {
            matches!(
                &a.action,
                Action::CastSpell { card, targets }
                    if targets == &vec![Target::Permanent(bears)]
                        && g.state.object(*card).card == CardId::LightningStrike
            )
        })
        .map(|a| a.action)
        .expect("the burn spell is offered at the sole creature target");
    submit_ok(&mut g, P1, bolt);
    for _ in 0..8 {
        if g.state.object(bears).zone != Zone::Battlefield {
            break;
        }
        let Some(p) = g.decision_owner() else { break };
        submit_ok(&mut g, p, Action::Pass);
    }

    assert_ne!(
        g.state.object(bears).zone,
        Zone::Battlefield,
        "the burn spell killed the creature"
    );
    assert!(
        !exiled_cards(&g, P0).contains(&CardId::GrizzlyBears),
        "a creature that was not in combat is not exiled: {:?}",
        exiled_cards(&g, P0)
    );
    assert!(clue_tokens(&g, P0).is_empty(), "and does not investigate");
}

/// The Clue token is a real artifact token whose printed ability sacrifices it
/// and draws a card (CR 111.10, 701.16a) — the payoff of investigate.
#[test]
fn clue_token_sacrifices_itself_to_draw_a_card() {
    let mut g = fixture(4705)
        .battlefield(P0, &[CardId::ClueToken, CardId::Forest, CardId::Forest])
        .library(P0, &[CardId::Forest; 4])
        .library(P1, &[CardId::Forest; 6])
        .main(P0, Step::PrecombatMain)
        .build();
    let clue = battlefield_card(&g, P0, CardId::ClueToken);
    object_mut(&mut g.state, clue).summoning_sick = false;
    let before = g.state.player(P0).hand.len();
    let activate = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action, Action::ActivateAbility { source, .. } if *source == clue))
        .map(|a| a.action)
        .expect("the Clue token offers its printed activated ability");
    submit_ok(&mut g, P0, activate);
    for _ in 0..10 {
        let Some(p) = g.decision_owner() else { break };
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(g.state.player(P0).hand.len(), before + 1, "drew one card");
    assert!(
        g.state.try_object(clue).is_none(),
        "the sacrificed token ceased to exist (CR 111.7)"
    );
}

/// Source provenance and registration: the card is a real catalog creature with
/// the printed WHO 111 text, and the Clue token is a token, not a card.
#[test]
fn witness_and_clue_are_catalogued_as_printed_and_token_is_not_a_card() {
    let def = witness().def();
    assert_eq!(def.name, "Thijarian Witness");
    assert_eq!(
        def.mana_cost().map(|c| (c.generic, c.pips.len())),
        Some((1, 1)),
        "{{1}}{{G}}"
    );
    assert_eq!(def.type_line, "Creature — Alien Cleric");
    assert_eq!((def.power, def.toughness), (Some(0), Some(4)));
    assert!(def.flash, "printed Flash (CR 702.8a)");
    assert_eq!(
        def.oracle,
        "Flash\nBear Witness — Whenever another creature dies, if it was attacking or blocking alone, exile it and investigate. (Create a Clue token. It's an artifact with \"{2}, Sacrifice this token: Draw a card.\")"
    );
    assert!(!witness().is_token());
    assert!(CardId::ClueToken.is_token(), "the Clue is a token card");
    assert!(
        catalog::by_name("Clue Token").is_none(),
        "a token is never importable"
    );
    assert_eq!(
        CardId::ClueToken.def().oracle,
        "{2}, Sacrifice this token: Draw a card."
    );
}
