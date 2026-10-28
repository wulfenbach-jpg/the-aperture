//! Slice 53: Ruxa, Patient Professor (C21 66) — the whole printed card.
//!
//! Printed text (fetched from Scryfall on 2026-10-28, recorded in
//! `fixtures/starter-corpus/slice53-sourced.json`):
//!
//! > Whenever Ruxa enters or attacks, return target creature card with no
//! > abilities from your graveyard to your hand.
//! > Creatures you control with no abilities get +1/+1.
//! > For each creature you control with no abilities, you may have that creature
//! > assign its combat damage as though it weren't blocked.
//!
//! Three separate surfaces are asserted here, each against the printed words:
//!
//! 1. **Clause 1** is a *card* target in a graveyard (CR 110.1, 400.1), filtered
//!    by the shared "no abilities" predicate, moved by the ordinary zone-change
//!    path (never the draw path), with the CR 903.9b command-zone replacement
//!    offered when the card is a commander.
//! 2. **Clause 2** is a layer 7c additive static whose recipient set is that same
//!    predicate.
//! 3. **Clause 3** is a real choice made while combat damage is being assigned
//!    (CR 510.1): the answered creature's whole damage goes to the defending
//!    player and none to its blockers, while the blockers still deal theirs.
//!
//! Everything the engine can be forged into is refused by BOTH
//! `GameState::validate` and a real `SessionSave::from_json` load, because the
//! slice-49 lesson (review 329 R329-3) is that a validator alone proves nothing.

mod common;

use aperture_core::action::{BlockDecl, Target};
use aperture_core::*;
use common::*;

fn ruxa() -> CardId {
    catalog::by_name("Ruxa, Patient Professor").expect("Ruxa is in the pool")
}

/// A creature with genuinely **no** abilities: 2/2 vanilla.
fn vanilla() -> CardId {
    CardId::GrizzlyBears
}

/// A creature with a printed triggered ability, so it must never be eligible.
fn with_abilities() -> CardId {
    CardId::ElvishVisionary
}

/// A creature whose only printed ability is a keyword (trample).
fn keyword_creature() -> CardId {
    CardId::Owlbear
}

/// `Fixture` has no graveyard builder; this keeps the fixture chain readable.
trait GraveyardExt {
    fn graveyard(self, p: PlayerId, cards: &[CardId]) -> Self;
}

impl GraveyardExt for Fixture {
    fn graveyard(self, p: PlayerId, cards: &[CardId]) -> Self {
        cards
            .iter()
            .fold(self, |fixture, card| fixture.add(p, Zone::Graveyard, *card))
    }
}

fn green(n: u8) -> ManaPool {
    ManaPool {
        green: u32::from(n),
        ..Default::default()
    }
}

fn colorless(n: u8) -> ManaPool {
    ManaPool {
        colorless: u32::from(n),
        ..Default::default()
    }
}

fn add_mana(mut pool: ManaPool, extra: ManaPool) -> ManaPool {
    pool.white += extra.white;
    pool.blue += extra.blue;
    pool.black += extra.black;
    pool.red += extra.red;
    pool.green += extra.green;
    pool.colorless += extra.colorless;
    pool
}

fn save(g: &Game) -> SessionSave {
    // Fixture-state round trips; these dummy decks do not replay the fixture.
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

fn loads(g: &Game) -> bool {
    SessionSave::from_json(&save(g).to_json()).is_ok()
}

fn round_trip(g: &Game) -> Game {
    let loaded = SessionSave::from_json(&save(g).to_json()).expect("fixture state loads");
    Game { state: loaded.game }
}

fn graveyard_cards(g: &Game, p: PlayerId) -> Vec<CardId> {
    g.state
        .player(p)
        .graveyard
        .iter()
        .map(|id| g.state.object(*id).card)
        .collect()
}

fn hand_cards(g: &Game, p: PlayerId) -> Vec<CardId> {
    g.state
        .player(p)
        .hand
        .iter()
        .map(|id| g.state.object(*id).card)
        .collect()
}

/// Cast Ruxa from P0's hand (cost {2}{G}{G}) and stop as soon as the enters
/// trigger asks for its target, or the stack empties.
fn cast_ruxa(g: &mut Game) {
    let card = hand_card(g, P0, ruxa());
    submit_ok(
        g,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
    for _ in 0..24 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return;
        }
        let p = g
            .decision_owner()
            .expect("someone holds priority while Ruxa resolves");
        submit_ok(g, p, Action::Pass);
    }
    panic!("Ruxa never resolved");
}

/// The legal graveyard targets offered for the pending trigger, as cards.
fn offered_targets(g: &Game) -> Vec<Target> {
    match &g.state.pending {
        Some(Pending::TriggerTarget { legal, .. }) => legal.clone(),
        other => panic!("no trigger target pending: {other:?}"),
    }
}

fn choose(g: &mut Game, target: Target) {
    let owner = g
        .decision_owner()
        .expect("the trigger's controller decides");
    submit_ok(g, owner, Action::ChooseTriggerTarget { target });
}

/// Pass priority while anything is pending on the stack until it settles; unlike
/// `settle_through_combat` this stops at any pending choice it did not expect.
fn settle_quiet(g: &mut Game) -> Vec<Event> {
    let mut log = Vec::new();
    for _ in 0..64 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return log;
        }
        let Some(p) = g.decision_owner() else {
            return log;
        };
        log.extend(submit_ok(g, p, Action::Pass));
    }
    panic!("never settled: {:?}", g.state.turn.step);
}

/// Attack with `attackers` from P0 at P1, block with `blocks` from P1.
fn attack_and_block(g: &mut Game, attackers: &[ObjectId], blocks: Vec<BlockDecl>) {
    g.state.turn.attackers.clear();
    g.state.turn.blockers.clear();
    g.state.turn.blocked.clear();
    g.state.turn.unpaired_blockers.clear();
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
    // CR 508.2: both players pass before the declare-blockers step begins.
    submit_ok(g, P0, Action::Pass);
    submit_ok(g, P1, Action::Pass);
    assert!(
        matches!(
            g.state.pending,
            Some(Pending::DeclareBlockers { player: P1 })
        ),
        "the declare-blockers step is pending: {:?}",
        g.state.pending
    );
    submit_ok(g, P1, Action::DeclareBlockers { blocks });
}

/// Pass priority until the combat damage assignment choice appears (or settle
/// out, returning `false`). It stops as soon as the damage has been dealt, so a
/// long pass loop can never walk into the next turn's combat.
fn run_to_damage_choice(g: &mut Game) -> bool {
    for _ in 0..32 {
        if matches!(
            g.state.pending,
            Some(Pending::CombatDamageAssignment { .. })
        ) {
            return true;
        }
        if g.state.turn.combat_damage_done {
            return false;
        }
        if g.state.pending.is_some() || g.state.is_over() {
            return false;
        }
        let Some(p) = g.decision_owner() else {
            return false;
        };
        submit_ok(g, p, Action::Pass);
    }
    false
}

/// Answer the pending assignment choice.
fn answer_assignment(g: &mut Game, as_though_unblocked: bool) -> Vec<Event> {
    let owner = g
        .decision_owner()
        .expect("the attacker's controller decides");
    submit_ok(
        g,
        owner,
        Action::AnswerCombatDamageAssignment {
            as_though_unblocked,
        },
    )
}

// ---------------------------------------------------------------- registration

#[test]
fn ruxa_is_registered_with_its_printed_text_and_three_implemented_surfaces() {
    let def = catalog::by_name("Ruxa, Patient Professor")
        .expect("in the pool")
        .def();
    assert_eq!(def.name, "Ruxa, Patient Professor");
    assert_eq!(def.cost, Some("{2}{G}{G}"));
    assert_eq!(def.type_line, "Legendary Creature — Bear Druid");
    assert_eq!(def.power, Some(4));
    assert_eq!(def.toughness, Some(4));
    assert!(
        def.oracle.starts_with(
            "Whenever Ruxa enters or attacks, return target creature card with no abilities from your graveyard to your hand."
        ),
        "printed trigger line: {}",
        def.oracle
    );
    assert!(
        def.oracle
            .contains("Creatures you control with no abilities get +1/+1."),
        "printed anthem line: {}",
        def.oracle
    );
    assert!(
        def.oracle.contains(
            "For each creature you control with no abilities, you may have that creature assign its combat damage as though it weren't blocked."
        ),
        "printed damage line: {}",
        def.oracle
    );
    // "enters or attacks" is two trigger events, so the table carries two
    // entries with the same printed sentence; both must be typed.
    assert_eq!(abilities(ruxa()).len(), 2);
    assert_eq!(ability::static_abilities(ruxa()).len(), 2);
    for ability in abilities(ruxa()) {
        assert!(
            def.oracle.contains(ability.text()),
            "typed ability text is not the printed text: {:?}",
            ability.text()
        );
    }
    for static_ability in ability::static_abilities(ruxa()) {
        assert!(
            def.oracle.contains(static_ability.text()),
            "typed static text is not the printed text: {:?}",
            static_ability.text()
        );
    }
}

// ------------------------------------------------------------------- clause 1

/// The enters trigger asks for a creature *card* in the graveyard and returns
/// exactly the chosen one; the other stays put.
#[test]
fn the_enters_trigger_returns_the_chosen_no_ability_creature_card() {
    let mut g = fixture(5301)
        .hand(P0, &[ruxa()])
        .mana(P0, add_mana(green(2), colorless(2)))
        .graveyard(P0, &[vanilla(), vanilla()])
        .library(P0, &[CardId::Forest; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let first = *g.state.player(P0).graveyard.first().unwrap();
    let second = *g.state.player(P0).graveyard.last().unwrap();
    assert_ne!(first, second);

    cast_ruxa(&mut g);

    let offered = offered_targets(&g);
    assert_eq!(
        offered,
        vec![Target::Card(first), Target::Card(second)],
        "both vanilla creature cards in the graveyard are legal targets, as cards"
    );
    choose(&mut g, Target::Card(first));
    settle_quiet(&mut g);

    assert_eq!(
        graveyard_cards(&g, P0),
        vec![vanilla()],
        "only the chosen card leaves the graveyard"
    );
    assert_eq!(
        hand_cards(&g, P0),
        vec![vanilla()],
        "the returned card is in its owner's hand"
    );
    assert!(
        g.state.stack.is_empty(),
        "the trigger has resolved off the stack"
    );
}

/// A creature card WITH abilities is not a legal target: printed abilities, a
/// keyword ability and an intrinsic mana ability all disqualify it.
#[test]
fn a_creature_card_with_any_ability_is_never_offered() {
    let mut g = fixture(5302)
        .hand(P0, &[ruxa()])
        .mana(P0, add_mana(green(2), colorless(2)))
        .graveyard(P0, &[with_abilities(), keyword_creature()])
        .library(P0, &[CardId::Forest; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();

    cast_ruxa(&mut g);
    settle_quiet(&mut g);

    assert!(
        g.state.pending.is_none(),
        "a target-only trigger with no legal target is removed, not asked (CR 603.3d): {:?}",
        g.state.pending
    );
    assert_eq!(
        graveyard_cards(&g, P0),
        vec![with_abilities(), keyword_creature()],
        "no ineligible card may be returned"
    );
    assert!(hand_cards(&g, P0).is_empty());
}

/// The card target is a *card*, not a permanent: naming the same graveyard
/// object as `Target::Permanent` is refused, and so is a card in an opponent's
/// graveyard.
#[test]
fn a_graveyard_card_cannot_be_named_as_a_permanent_or_from_an_opponent() {
    let mut g = fixture(5303)
        .hand(P0, &[ruxa()])
        .mana(P0, add_mana(green(2), colorless(2)))
        .graveyard(P0, &[vanilla(), vanilla()])
        .graveyard(P1, &[vanilla()])
        .library(P0, &[CardId::Forest; 10])
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    // Two vanillas exist across the two graveyards; only P0's is legal.
    let mine = *g.state.player(P0).graveyard.first().unwrap();
    let theirs = *g.state.player(P1).graveyard.first().unwrap();

    cast_ruxa(&mut g);

    let offered = offered_targets(&g);
    assert_eq!(offered.len(), 2, "only P0's two vanilla cards are offered");
    assert!(offered.contains(&Target::Card(mine)));
    assert!(
        !offered.contains(&Target::Card(theirs)),
        "\"from your graveyard\" is not an opponent's graveyard"
    );
    assert!(
        !offered.contains(&Target::Permanent(mine)),
        "a card in a graveyard is not a permanent (CR 110.1)"
    );

    // A forged cast/choose of the wrong shape is refused by the engine, not
    // silently accepted.
    assert!(
        g.validate(
            P0,
            &Action::ChooseTriggerTarget {
                target: Target::Permanent(mine)
            }
        )
        .is_err(),
        "Target::Permanent may not name a graveyard card"
    );
    assert!(
        g.validate(
            P0,
            &Action::ChooseTriggerTarget {
                target: Target::Card(theirs)
            }
        )
        .is_err(),
        "another player's graveyard card is not a legal target"
    );
    assert_eq!(
        graveyard_cards(&g, P1),
        vec![vanilla()],
        "the refused choices changed nothing"
    );
}

/// The attack trigger returns a card too, and the returned card comes from the
/// ordinary zone-change path: no draw event, no `drew_from_empty`, and the
/// library is untouched.
#[test]
fn the_attack_trigger_returns_a_card_without_touching_the_draw_path() {
    let mut g = fixture(5304)
        .battlefield(P0, &[ruxa(), vanilla()])
        .graveyard(P0, &[vanilla()])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let ruxa_id = battlefield_card(&g, P0, ruxa());
    object_mut(&mut g.state, ruxa_id).summoning_sick = false;
    let library_before: Vec<CardId> = g
        .state
        .player(P0)
        .library
        .iter()
        .map(|id| g.state.object(*id).card)
        .collect();

    let mut log = Vec::new();
    g.state.turn.attackers.clear();
    g.state.turn.defender = Some(P1);
    g.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![ruxa_id],
            defender: Some(P1),
        },
    );
    // Both players pass; the attack trigger is the only thing left to resolve.
    for _ in 0..32 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            break;
        }
        let p = g.decision_owner().expect("priority");
        log.extend(submit_ok(&mut g, p, Action::Pass));
    }
    // One legal target, so the engine chooses it (CR 603.3d, legal.len() == 1).
    settle_quiet(&mut g);

    assert_eq!(
        graveyard_cards(&g, P0),
        Vec::<CardId>::new(),
        "the card left the graveyard"
    );
    assert!(
        hand_cards(&g, P0).contains(&vanilla()),
        "the attack trigger returned the card to hand"
    );
    let library_after: Vec<CardId> = g
        .state
        .player(P0)
        .library
        .iter()
        .map(|id| g.state.object(*id).card)
        .collect();
    assert_eq!(
        library_before, library_after,
        "returning a card from a graveyard is not a draw"
    );
    assert!(
        !log.iter().any(|event| matches!(event, Event::Draw { .. })),
        "no Draw event may be emitted for a graveyard return"
    );
    assert!(!g.state.player(P0).drew_from_empty);
}

// ------------------------------------------------------------------- clause 2

/// The anthem is additive layer 7c and its recipient set is exactly "creatures
/// you control with no abilities": the vanilla creature grows, a creature with a
/// triggered ability does not, an opponent's vanilla does not, and Ruxa itself
/// (which has abilities) does not.
#[test]
fn the_anthem_pumps_only_your_creatures_with_no_abilities() {
    let g = fixture(5305)
        .battlefield(
            P0,
            &[ruxa(), vanilla(), with_abilities(), keyword_creature()],
        )
        .battlefield(P1, &[vanilla()])
        .build();
    let ruxa_id = battlefield_card(&g, P0, ruxa());
    let bear = battlefield_card(&g, P0, vanilla());
    let visionary = battlefield_card(&g, P0, with_abilities());
    let owlbear = battlefield_card(&g, P0, keyword_creature());
    let opp_bear = battlefield_card(&g, P1, vanilla());

    // The layer pass runs on any state recomputation; a real action is the
    // honest trigger for it.
    let mut g = g;
    g.state.turn.active = P0;
    g.state.turn.step = Step::PrecombatMain;
    set_priority(&mut g.state, Some(P0));
    submit_ok(&mut g, P0, Action::Pass);
    let _ = settle_quiet(&mut g);

    let power = |id| g.state.object(id).power();
    let toughness = |id| g.state.object(id).toughness();
    assert_eq!(power(bear), 3, "a 2/2 with no abilities becomes 3/3");
    assert_eq!(toughness(bear), 3);
    assert_eq!(power(ruxa_id), 4, "Ruxa has abilities, so it is not pumped");
    assert_eq!(toughness(ruxa_id), 4);
    assert_eq!(
        power(visionary),
        1,
        "a creature with a triggered ability is not pumped"
    );
    assert_eq!(power(owlbear), 4, "a trample creature is not pumped");
    assert_eq!(
        power(opp_bear),
        2,
        "another player's creature is not pumped by this Ruxa"
    );
}

// ------------------------------------------------------------------- clause 3

/// Answering "yes" sends the blocked attacker's whole combat damage to the
/// defending player and none to the blocker, while the blocker still deals its
/// own damage to the attacker (CR 510.1, 510.2).
#[test]
fn assigning_as_though_unblocked_deals_all_damage_to_the_player() {
    let mut g = fixture(5306)
        .battlefield(P0, &[ruxa(), vanilla()])
        .battlefield(P1, &[vanilla()])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let bear = battlefield_card(&g, P0, vanilla());
    let blocker = battlefield_card(&g, P1, vanilla());
    object_mut(&mut g.state, bear).summoning_sick = false;
    object_mut(&mut g.state, blocker).summoning_sick = false;
    attack_and_block(
        &mut g,
        &[bear],
        vec![BlockDecl {
            blocker,
            attacker: bear,
        }],
    );
    assert!(
        run_to_damage_choice(&mut g),
        "a blocked no-ability attacker under Ruxa must be asked: {:?}",
        g.state.pending
    );
    assert_eq!(
        g.state.player(P1).life,
        20,
        "nothing is dealt before the choice is answered"
    );
    let log = answer_assignment(&mut g, true);

    assert_eq!(
        g.state.player(P1).life,
        17,
        "all three damage goes to the defending player"
    );
    assert_eq!(
        g.state.object(blocker).damage,
        0,
        "the blocker receives no damage when the attacker assigns as though unblocked"
    );
    assert_eq!(
        g.state.object(bear).damage,
        2,
        "the blocker still deals its own combat damage to the attacker"
    );
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::CombatDamageAssignment {
                as_though_unblocked: true,
                ..
            }
        )),
        "the choice itself is recorded, not only its consequence"
    );
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::CombatDamage {
                blocker: None,
                amount: 3,
                ..
            }
        )),
        "the damage to the player is three"
    );
}

/// Declining keeps the ordinary lethal-first assignment: the blocker takes the
/// damage and dies, and the defending player loses nothing.
#[test]
fn declining_keeps_the_ordinary_assignment() {
    let mut g = fixture(5307)
        .battlefield(P0, &[ruxa(), vanilla()])
        .battlefield(P1, &[vanilla()])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let bear = battlefield_card(&g, P0, vanilla());
    let blocker = battlefield_card(&g, P1, vanilla());
    object_mut(&mut g.state, bear).summoning_sick = false;

    attack_and_block(
        &mut g,
        &[bear],
        vec![BlockDecl {
            blocker,
            attacker: bear,
        }],
    );
    assert!(run_to_damage_choice(&mut g));
    let log = answer_assignment(&mut g, false);

    assert_eq!(g.state.player(P1).life, 20, "no damage reaches the player");
    assert_eq!(
        graveyard_cards(&g, P1),
        vec![vanilla()],
        "the 3/3 kills the 2/2 blocker under the ordinary assignment"
    );
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::CombatDamageAssignment {
                as_though_unblocked: false,
                ..
            }
        )),
        "the declined choice is recorded"
    );
}

/// Two blocked no-ability attackers are asked one after the other in APNAP order
/// and the whole batch is dealt once, after the last answer.
#[test]
fn two_eligible_attackers_are_asked_in_order_and_damage_is_dealt_once() {
    let mut g = fixture(5308)
        .battlefield(P0, &[ruxa(), vanilla(), vanilla()])
        .battlefield(P1, &[vanilla(), vanilla()])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let attackers: Vec<ObjectId> = g
        .state
        .battlefield(P0)
        .iter()
        .filter(|o| o.card == vanilla())
        .map(|o| o.id)
        .collect();
    let blockers: Vec<ObjectId> = g
        .state
        .battlefield(P1)
        .iter()
        .filter(|o| o.card == vanilla())
        .map(|o| o.id)
        .collect();
    assert_eq!(attackers.len(), 2);
    assert_eq!(blockers.len(), 2);
    for id in &attackers {
        object_mut(&mut g.state, *id).summoning_sick = false;
    }

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
    assert!(run_to_damage_choice(&mut g));
    // APNAP order: one seat, so the order is the attacker ids, ascending.
    let order = match &g.state.pending {
        Some(Pending::CombatDamageAssignment {
            order,
            cursor,
            answers,
            ..
        }) => {
            assert_eq!(*cursor, 0);
            assert!(answers.is_empty());
            order.clone()
        }
        other => panic!("no assignment pending: {other:?}"),
    };
    assert_eq!(order.len(), 2);
    assert_eq!(order[0].attacker, attackers[0]);
    assert_eq!(order[1].attacker, attackers[1]);
    assert!(order.iter().all(|choice| choice.player == P0));

    answer_assignment(&mut g, true);
    assert_eq!(
        g.state.player(P1).life,
        20,
        "the batch is not dealt until the last answer (CR 510.2)"
    );
    answer_assignment(&mut g, false);

    assert_eq!(
        g.state.player(P1).life,
        17,
        "only the first attacker's three damage reaches the player"
    );
    assert_eq!(
        g.state.object(blockers[0]).damage,
        0,
        "the first attacker assigned as though unblocked"
    );
}

/// An unblocked attacker is never asked: it already assigns as though it weren't
/// blocked, so the permission would have no consequence.
#[test]
fn an_unblocked_attacker_is_never_asked() {
    let mut g = fixture(5309)
        .battlefield(P0, &[ruxa(), vanilla()])
        .battlefield(P1, &[vanilla()])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let bear = battlefield_card(&g, P0, vanilla());
    object_mut(&mut g.state, bear).summoning_sick = false;

    attack_and_block(&mut g, &[bear], Vec::new());
    assert!(
        !run_to_damage_choice(&mut g),
        "no choice is created without a blocker"
    );
    assert_eq!(
        g.state.player(P1).life,
        17,
        "the unblocked 3/3 deals its damage to the player"
    );
}

/// A blocked attacker with abilities is never asked, even under Ruxa: the
/// permission is only for creatures with no abilities.
#[test]
fn a_blocked_attacker_with_abilities_is_never_asked() {
    let mut g = fixture(5310)
        .battlefield(P0, &[ruxa(), with_abilities()])
        .battlefield(P1, &[vanilla()])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let visionary = battlefield_card(&g, P0, with_abilities());
    let blocker = battlefield_card(&g, P1, vanilla());
    object_mut(&mut g.state, visionary).summoning_sick = false;

    attack_and_block(
        &mut g,
        &[visionary],
        vec![BlockDecl {
            blocker,
            attacker: visionary,
        }],
    );
    assert!(
        !run_to_damage_choice(&mut g),
        "a creature with abilities is outside Ruxa's permission: {:?}",
        g.state.pending
    );
    assert_eq!(
        g.state.player(P1).life,
        20,
        "the 1/1 is blocked and deals nothing to the player"
    );
}

// ------------------------------------------------------- forged-state refusals

/// A valid mid-choice state is only reachable inside the combat damage step with
/// a live blocked attacker. Every deviation is refused by `validate` AND by a
/// real `SessionSave::from_json`, and the honest state round-trips.
#[test]
fn forged_assignment_pendings_are_refused_by_validate_and_by_a_real_load() {
    let mut g = fixture(5311)
        .battlefield(P0, &[ruxa(), vanilla()])
        .battlefield(P1, &[vanilla()])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let bear = battlefield_card(&g, P0, vanilla());
    let blocker = battlefield_card(&g, P1, vanilla());
    object_mut(&mut g.state, bear).summoning_sick = false;
    attack_and_block(
        &mut g,
        &[bear],
        vec![BlockDecl {
            blocker,
            attacker: bear,
        }],
    );
    assert!(run_to_damage_choice(&mut g));

    // The honest state validates and loads.
    assert!(g.state.validate().is_ok(), "the honest pending is valid");
    assert!(loads(&g), "the honest pending loads through SessionSave");
    assert_eq!(
        round_trip(&g).state.pending,
        g.state.pending,
        "the choice survives a save/load unchanged"
    );

    let honest = match g.state.pending.clone() {
        Some(pending) => pending,
        None => panic!("pending"),
    };
    let ruxa_id = battlefield_card(&g, P0, ruxa());

    // (a) an attacker that is not in this combat
    let mut forged = g.state.clone();
    forged.pending = Some(match &honest {
        Pending::CombatDamageAssignment {
            order,
            cursor,
            answers,
            ..
        } => {
            let mut order = order.clone();
            order[0].attacker = ruxa_id;
            Pending::CombatDamageAssignment {
                order,
                cursor: *cursor,
                answers: answers.clone(),
            }
        }
        _ => unreachable!(),
    });
    assert!(
        forged.validate().is_err(),
        "Ruxa is not in this combat, so it cannot be the chosen attacker"
    );
    assert!(
        !loads(&Game { state: forged }),
        "a real load refuses the same state"
    );

    // (b) a source that is not the printed permission
    let mut forged = g.state.clone();
    forged.pending = Some(match &honest {
        Pending::CombatDamageAssignment {
            order,
            cursor,
            answers,
            ..
        } => {
            let mut order = order.clone();
            order[0].source = bear;
            Pending::CombatDamageAssignment {
                order,
                cursor: *cursor,
                answers: answers.clone(),
            }
        }
        _ => unreachable!(),
    });
    assert!(
        forged.validate().is_err(),
        "the permission must come from Ruxa on the battlefield"
    );
    assert!(!loads(&Game { state: forged }));

    // (c) answers/cursor not a valid prefix
    let mut forged = g.state.clone();
    forged.pending = Some(match &honest {
        Pending::CombatDamageAssignment { order, .. } => Pending::CombatDamageAssignment {
            order: order.clone(),
            cursor: 0,
            answers: vec![true],
        },
        _ => unreachable!(),
    });
    assert!(forged.validate().is_err());
    assert!(!loads(&Game { state: forged }));

    // (d) the pending outside the combat damage step
    let mut forged = g.state.clone();
    forged.turn.step = Step::PrecombatMain;
    assert!(
        forged.validate().is_err(),
        "the choice exists only in the combat damage step (CR 510.1)"
    );
    assert!(!loads(&Game { state: forged }));

    // (e) the pending after combat damage has already been dealt
    let mut forged = g.state.clone();
    forged.turn.combat_damage_done = true;
    assert!(forged.validate().is_err());
    assert!(!loads(&Game { state: forged }));
}

/// A pending naming an attacker that does have abilities is refused, because the
/// permission could not have produced it.
#[test]
fn a_forged_pending_for_a_creature_with_abilities_is_refused() {
    let mut g = fixture(5312)
        .battlefield(P0, &[ruxa(), with_abilities()])
        .battlefield(P1, &[vanilla()])
        .library(P0, &[CardId::Forest; 6])
        .library(P1, &[CardId::Forest; 6])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let visionary = battlefield_card(&g, P0, with_abilities());
    let blocker = battlefield_card(&g, P1, vanilla());
    let ruxa_id = battlefield_card(&g, P0, ruxa());
    object_mut(&mut g.state, visionary).summoning_sick = false;
    attack_and_block(
        &mut g,
        &[visionary],
        vec![BlockDecl {
            blocker,
            attacker: visionary,
        }],
    );
    settle_quiet(&mut g);
    assert!(
        g.state.pending.is_none(),
        "the engine never offers this choice"
    );

    // A save that invents it anyway is refused by both paths.
    let mut forged = g.state.clone();
    forged.pending = Some(Pending::CombatDamageAssignment {
        order: vec![aperture_core::state::CombatDamageChoice {
            player: P0,
            attacker: visionary,
            source: ruxa_id,
        }],
        cursor: 0,
        answers: Vec::new(),
    });
    assert!(
        forged.validate().is_err(),
        "the chosen attacker has abilities"
    );
    assert!(!loads(&Game { state: forged }));
}

/// The commander destination prediction is extended to the graveyard effect: a
/// commander card returned from a graveyard to hand is offered the command zone
/// (CR 903.9b), and the off-by-one destination cannot be forged.
#[test]
fn a_commander_card_in_a_graveyard_is_offered_the_command_zone() {
    let decks = vec![DeckList::parse_practice("fixture", "40 Forest\n").unwrap(); 2];
    let mut g = Game::new_command_zone(
        5313,
        &decks,
        &[
            (P0, CardId::IsamaruHoundOfKonda),
            (P1, CardId::IsamaruHoundOfKonda),
        ],
    )
    .expect("command-zone setup");
    // Get past the pre-game mulligan choices so the only pendings below are the
    // ones this test creates.
    let mut log = Vec::new();
    start_game(&mut g, &mut log);
    // Move P0's commander, a vanilla legendary creature, to its graveyard. That
    // is a legal game state: the CR 903.9a choice is armed for a commander in a
    // graveyard.
    let commander = g.state.commanders[0].object;
    {
        let players = &mut g.state.players;
        players[P0 as usize]
            .command_zone
            .retain(|id| *id != commander);
        players[P0 as usize].graveyard.push(commander);
        if let Some(object) = g.state.objects.get_mut(&commander) {
            object.zone = Zone::Graveyard;
        }
        g.state.commanders[0].return_eligible = true;
    }
    // Decline the CR 903.9a return so the card stays in the graveyard. The arming
    // is only read by an SBA check, so a pass is needed to reach the choice.
    for _ in 0..8 {
        match g.state.pending.clone() {
            Some(Pending::CommanderReturn { .. }) => {
                submit_ok(
                    &mut g,
                    P0,
                    Action::AnswerCommanderReturn {
                        to_command_zone: false,
                    },
                );
                break;
            }
            Some(_) => break,
            None => {
                let Some(p) = g.decision_owner() else { break };
                submit_ok(&mut g, p, Action::Pass);
            }
        }
    }
    assert!(
        !g.state.commanders[0].return_eligible,
        "the CR 903.9a choice was declined and is no longer armed"
    );
    assert!(g.state.player(P0).graveyard.contains(&commander));

    // Put Ruxa on the battlefield for P0 without an enters event, then attack so
    // the attack trigger asks for the commander card.
    let ruxa_id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        ruxa_id,
        aperture_core::state::Object {
            id: ruxa_id,
            card: ruxa(),
            owner: P0,
            controller: P0,
            zone: Zone::Battlefield,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: aperture_core::counters::Counters::default(),
            paired_with: None,
        },
    );
    g.state.turn.active = P0;
    g.state.turn.step = Step::DeclareAttackers;
    g.state.turn.attackers.clear();
    g.state.turn.defender = Some(P1);
    g.state.turn.priority = None;
    g.state.turn.passed = 0;
    g.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![ruxa_id],
            defender: Some(P1),
        },
    );
    for _ in 0..64 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            break;
        }
        let Some(p) = g.decision_owner() else { break };
        submit_ok(&mut g, p, Action::Pass);
    }
    // The trigger offers the commander card as a graveyard card.
    if let Some(Pending::TriggerTarget { .. }) = &g.state.pending {
        let offered = offered_targets(&g);
        assert_eq!(offered, vec![Target::Card(commander)]);
        choose(&mut g, Target::Card(commander));
    }
    for _ in 0..16 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            break;
        }
        let Some(p) = g.decision_owner() else { break };
        submit_ok(&mut g, p, Action::Pass);
    }

    match g.state.pending.clone() {
        Some(Pending::CommanderReplacement {
            player,
            object,
            from_zone,
            destination,
            ..
        }) => {
            assert_eq!(player, P0);
            assert_eq!(object, commander);
            assert_eq!(
                from_zone,
                Zone::Graveyard,
                "the replacement is anchored to the graveyard move"
            );
            assert_eq!(destination, Zone::Hand);
            assert!(
                g.state.validate().is_ok(),
                "the commander replacement the engine produced validates"
            );
            // A `SessionSave` round trip is asserted for the new pending in
            // `forged_assignment_pendings_are_refused_by_validate_and_by_a_real_load`;
            // here the dummy replay decks cannot contain a commander card, so the
            // save's deck/commander cross-check would fail for a reason unrelated
            // to this pending.
            // Accepting puts the commander into the command zone.
            submit_ok(
                &mut g,
                P0,
                Action::AnswerCommanderReplacement {
                    to_command_zone: true,
                },
            );
            assert!(
                g.state.player(P0).command_zone.contains(&object)
                    || g.state
                        .objects
                        .values()
                        .any(|o| o.card == CardId::IsamaruHoundOfKonda && o.zone == Zone::Command),
                "the commander went to the command zone instead of the hand"
            );
        }
        other => panic!("expected the CR 903.9b replacement, got {other:?}"),
    }
}
