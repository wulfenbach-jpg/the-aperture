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

/// CR 506.5 live-state discriminator: after the first of two attackers dies,
/// the second IS attacking alone even though it did not ATTACK alone.
/// Witness says "was attacking", so its last-known pre-departure state qualifies.
#[test]
fn attacker_killed_after_its_companion_left_combat_is_attacking_alone() {
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

    // The survivor is now attacking alone: the second death qualifies even
    // though both creatures were originally declared together.
    assert_eq!(g.decision_owner(), Some(P0));
    submit_ok(&mut g, P0, Action::Pass);
    assert_eq!(g.decision_owner(), Some(P1));
    cast_at_creature(&mut g, P1, CardId::LightningStrike, second);
    // The first spell did not trigger; the second death must trigger once.
    settle_through_combat(&mut g);
    assert_eq!(g.state.object(first).zone, Zone::Graveyard);
    assert_eq!(g.state.object(second).zone, Zone::Exile);
    assert!(g.state.turn.attackers.is_empty());
    assert_eq!(exiled_cards(&g, P0), vec![CardId::GrizzlyBears]);
    assert_eq!(clue_tokens(&g, P0).len(), 1);
    assert!(g.state.stack.is_empty());
    assert!(g.state.trigger_queue.is_empty());
    assert!(g.state.resolving.is_none());
    assert!(g.state.pending.is_none());
}

/// On the blocking side, after the first blocker dies to a spell the survivor
/// IS blocking alone when combat damage kills it. Only the second death qualifies.
#[test]
fn blocker_killed_after_its_companion_left_combat_is_blocking_alone() {
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
    assert_eq!(g.state.object(first).zone, Zone::Graveyard);
    assert_eq!(g.state.object(second).zone, Zone::Exile);
    assert_eq!(exiled_cards(&g, P1), vec![CardId::GrizzlyBears]);
    assert_eq!(clue_tokens(&g, P0).len(), 1);
    assert!(g.state.stack.is_empty());
    assert!(g.state.trigger_queue.is_empty());
    assert!(g.state.resolving.is_none());
    assert!(g.state.pending.is_none());
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

/// A creature put onto the battlefield attacking counts in LIVE combat even
/// though it never attacked. Cub is declared alone but no longer IS attacking
/// alone once its put joins it. After Cub dies, the put creature IS alone.
#[test]
fn cub_put_attacker_counts_without_a_declaration() {
    let mut g = fixture(5001)
        .battlefield(P0, &[witness(), CardId::OwlbearCub])
        .battlefield(P1, &[CardId::Forest; 8])
        .battlefield(P1, &[CardId::Mountain; 4])
        .hand(P1, &[CardId::LightningStrike; 2])
        .library(
            P0,
            &[
                CardId::GrizzlyBears,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .library(P1, &[CardId::Forest; 8])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let cub = battlefield_card(&g, P0, CardId::OwlbearCub);
    let put = g.state.player(P0).library[0];
    g.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![cub],
            defender: Some(P1),
        },
    );
    for _ in 0..16 {
        if g.state.pending.is_some() {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(matches!(
        g.state.pending,
        Some(Pending::SearchLibrary { .. })
    ));
    let pick = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::SearchLibraryPick { card } if card == put))
        .expect("the real Cub put is offered")
        .action;
    submit_ok(&mut g, P0, pick);
    submit_ok(&mut g, P0, Action::SearchLibraryDone);
    assert_eq!(g.state.turn.declared_attackers, vec![cub]);
    assert!(g.state.turn.attackers.contains(&put));
    assert_eq!(g.state.turn.attackers.len(), 2);
    assert!(g.state.object(put).tapped);
    assert_eq!(g.decision_owner(), Some(P0));
    submit_ok(&mut g, P0, Action::Pass);
    cast_at_creature(&mut g, P1, CardId::LightningStrike, cub);
    assert_eq!(g.state.object(cub).zone, Zone::Graveyard);
    assert!(
        clue_tokens(&g, P0).is_empty(),
        "declared alone is NOT attacking alone"
    );
    assert_eq!(g.state.turn.attackers, vec![put]);
    submit_ok(&mut g, P0, Action::Pass);
    cast_at_creature(&mut g, P1, CardId::LightningStrike, put);
    settle_through_combat(&mut g);
    assert_eq!(g.state.object(put).zone, Zone::Exile);
    assert_eq!(
        clue_tokens(&g, P0).len(),
        1,
        "undeclared put attacker was attacking alone"
    );
    assert!(g.state.stack.is_empty());
    assert!(g.state.resolving.is_none());
    assert!(g.state.trigger_queue.is_empty());
    assert!(g.state.pending.is_none());
}

/// Legacy saves lack declared_* fields. The serialized public live combat
/// membership is sufficient for Witness; no declaration record is consulted.
#[test]
fn absent_legacy_declaration_does_not_suppress_a_live_alone_death() {
    let mut g = fixture(5002)
        .battlefield(P0, &[witness(), CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::OrdinaryBear])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let blocker = battlefield_card(&g, P1, CardId::OrdinaryBear);
    attack_and_block(
        &mut g,
        &[bear],
        vec![BlockDecl {
            blocker,
            attacker: bear,
        }],
    );
    // Model the serde(default) absent-field shape, then actually round-trip it.
    let mut value = serde_json::to_value(&g.state).unwrap();
    value["turn"]
        .as_object_mut()
        .unwrap()
        .remove("declared_attackers");
    value["turn"]
        .as_object_mut()
        .unwrap()
        .remove("declared_blockers");
    g.state = serde_json::from_value(value).unwrap();
    assert!(g.state.turn.declared_attackers.is_empty());
    assert_eq!(g.state.turn.attackers, vec![bear]);
    g.state.validate().unwrap();
    settle_through_combat(&mut g);
    assert_eq!(g.state.object(bear).zone, Zone::Exile);
    assert_eq!(clue_tokens(&g, P0).len(), 1);
}

/// CR 509.1g: a blocker remains blocking after its attacker leaves combat.
/// Losing the pair is NOT removal of the blocking creature from combat.
#[test]
fn blocker_still_blocks_alone_after_its_attacker_is_bounced() {
    let mut g = fixture(5003)
        .hand(P0, &[CardId::Unsummon, CardId::LightningStrike])
        .battlefield(
            P0,
            &[
                witness(),
                CardId::OrdinaryBear,
                CardId::Island,
                CardId::Mountain,
                CardId::Mountain,
            ],
        )
        .battlefield(P1, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let attacker = battlefield_card(&g, P0, CardId::OrdinaryBear);
    let blocker = battlefield_card(&g, P1, CardId::GrizzlyBears);
    attack_and_block(&mut g, &[attacker], vec![BlockDecl { blocker, attacker }]);
    cast_at_creature(&mut g, P0, CardId::Unsummon, attacker);
    assert_eq!(g.state.object(attacker).zone, Zone::Hand);
    assert!(g.state.turn.attackers.is_empty());
    assert_eq!(g.state.object(blocker).zone, Zone::Battlefield);
    g.state.validate().unwrap();
    assert_eq!(g.state.turn.unpaired_blockers, vec![blocker]);
    let mut loaded = fixture_save_round_trip(&g);
    cast_at_creature(&mut loaded, P0, CardId::LightningStrike, blocker);
    settle_through_combat(&mut loaded);
    cast_at_creature(&mut g, P0, CardId::LightningStrike, blocker);
    settle_through_combat(&mut g);
    assert_eq!(g.state.object(blocker).zone, Zone::Exile);
    assert_eq!(
        clue_tokens(&g, P0).len(),
        1,
        "the blocker never left combat before dying"
    );
    assert_eq!(
        g.state, loaded.state,
        "actual saved-state load continues identically"
    );
    assert!(g.state.turn.unpaired_blockers.is_empty(), "combat ended");
}

// Fixture saves exercise the actual loader and continuation. Dummy replay decks
// do not claim these hand-built initial states are reproducible legal replays.
fn fixture_save(g: &Game) -> SessionSave {
    let decks = vec![DeckList::parse_practice("fixture", "40 Forest\n").unwrap(); 2];
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        decks,
        AiSettings::default(),
    )
    .unwrap();
    SessionSave::capture(g, replay, vec![], true)
}
fn fixture_save_round_trip(g: &Game) -> Game {
    let loaded = SessionSave::from_json(&fixture_save(g).to_json()).unwrap();
    Game { state: loaded.game }
}

#[test]
fn unpaired_blocker_and_paired_blocker_are_not_blocking_alone() {
    let mut g = fixture(5004)
        .hand(
            P0,
            &[
                CardId::Unsummon,
                CardId::LightningStrike,
                CardId::LightningStrike,
            ],
        )
        .battlefield(
            P0,
            &[
                witness(),
                CardId::OrdinaryBear,
                CardId::OrdinaryBear,
                CardId::Island,
            ],
        )
        .battlefield(P0, &[CardId::Mountain; 4])
        .battlefield(P1, &[CardId::GrizzlyBears; 2])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let attackers: Vec<_> = g
        .state
        .battlefield(P0)
        .iter()
        .filter(|o| o.card == CardId::OrdinaryBear)
        .map(|o| o.id)
        .collect();
    let blockers: Vec<_> = g.state.battlefield(P1).iter().map(|o| o.id).collect();
    attack_and_block(
        &mut g,
        &attackers,
        vec![
            BlockDecl {
                blocker: blockers[0],
                attacker: attackers[0],
            },
            BlockDecl {
                blocker: blockers[1],
                attacker: attackers[1],
            },
        ],
    );
    cast_at_creature(&mut g, P0, CardId::Unsummon, attackers[0]);
    assert_eq!(g.state.turn.unpaired_blockers, vec![blockers[0]]);
    cast_at_creature(&mut g, P0, CardId::LightningStrike, blockers[1]);
    assert_eq!(g.state.object(blockers[1]).zone, Zone::Graveyard);
    assert!(
        clue_tokens(&g, P0).is_empty(),
        "the unpaired blocker still counts beside it"
    );
    cast_at_creature(&mut g, P0, CardId::LightningStrike, blockers[0]);
    settle_through_combat(&mut g);
    assert_eq!(g.state.object(blockers[0]).zone, Zone::Exile);
    assert_eq!(clue_tokens(&g, P0).len(), 1);
}

#[test]
fn bounced_and_recast_unpaired_blocker_is_not_still_blocking() {
    let mut g = fixture(5005)
        .hand(
            P0,
            &[CardId::Unsummon, CardId::Unsummon, CardId::LightningStrike],
        )
        .battlefield(
            P0,
            &[
                witness(),
                CardId::OrdinaryBear,
                CardId::Island,
                CardId::Island,
                CardId::Mountain,
                CardId::Mountain,
            ],
        )
        .battlefield(P1, &[CardId::AshcoatBear, CardId::Forest, CardId::Forest])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let attacker = battlefield_card(&g, P0, CardId::OrdinaryBear);
    let blocker = battlefield_card(&g, P1, CardId::AshcoatBear);
    attack_and_block(&mut g, &[attacker], vec![BlockDecl { blocker, attacker }]);
    cast_at_creature(&mut g, P0, CardId::Unsummon, attacker);
    assert_eq!(g.state.turn.unpaired_blockers, vec![blocker]);
    cast_at_creature(&mut g, P0, CardId::Unsummon, blocker);
    assert!(g.state.turn.unpaired_blockers.is_empty());
    submit_ok(&mut g, P0, Action::Pass);
    let recast = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::CastSpell { card, .. } if card == blocker))
        .expect("same card's Flash recast is offered")
        .action;
    submit_ok(&mut g, P1, recast);
    for _ in 0..8 {
        if g.state.stack.is_empty() {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert_eq!(g.state.object(blocker).zone, Zone::Battlefield);
    assert_eq!(g.decision_owner(), Some(P0));
    cast_at_creature(&mut g, P0, CardId::LightningStrike, blocker);
    settle_through_combat(&mut g);
    assert_eq!(g.state.object(blocker).zone, Zone::Graveyard);
    assert!(
        clue_tokens(&g, P0).is_empty(),
        "same ObjectId is not the old blocking incarnation"
    );
}

#[test]
fn malformed_unpaired_blockers_fail_validation_and_real_save_load() {
    let base = fixture(5006)
        .battlefield(P0, &[witness(), CardId::OrdinaryBear])
        .battlefield(P1, &[CardId::GrizzlyBears, CardId::Forest])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::DeclareBlockers)
        .build();
    let blocker = battlefield_card(&base, P1, CardId::GrizzlyBears);
    let own = battlefield_card(&base, P0, CardId::OrdinaryBear);
    let land = battlefield_card(&base, P1, CardId::Forest);
    for case in 0..8 {
        let mut g = Game {
            state: base.state.clone(),
        };
        g.state.turn.defender = Some(P1);
        g.state.turn.unpaired_blockers = vec![blocker];
        match case {
            0 => g.state.turn.unpaired_blockers.push(blocker),
            1 => g.state.turn.unpaired_blockers = vec![ObjectId(u32::MAX)],
            2 => g.state.turn.unpaired_blockers = vec![own],
            3 => g.state.turn.unpaired_blockers = vec![land],
            4 => g.state.turn.step = Step::PostcombatMain,
            5 => g.state.turn.defender = None,
            6 => g.state.turn.blockers.push((blocker, own)),
            7 => {
                let o = g.state.objects.get_mut(&blocker).unwrap();
                o.zone = Zone::Hand;
                g.state.players[P1 as usize].hand.push(blocker);
            }
            _ => unreachable!(),
        }
        assert!(
            g.state.validate().is_err(),
            "validator admitted case {case}"
        );
        assert!(
            SessionSave::from_json(&fixture_save(&g).to_json()).is_err(),
            "real loader admitted case {case}"
        );
    }
}
