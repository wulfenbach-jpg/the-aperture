//! Slice 48: Owlbear Cub — "Mama's Coming — Whenever this creature attacks a
//! player who controls eight or more lands, look at the top eight cards of your
//! library. You may put a creature card from among them onto the battlefield
//! tapped and attacking that player. Put the rest on the bottom of your library
//! in a random order." (CLB 246)
//!
//! What is asserted is the printed text, not an approximation: the intervening-if
//! is the *defending* player's land count and nothing else; the look is private
//! and covers exactly the top eight; only a creature card may be put, it arrives
//! tapped and attacking that player and deals combat damage; declining puts
//! nothing onto the battlefield; and the "rest" is the looked-at cards going to
//! the bottom in a random order — the rest of the library keeps its order, which
//! is what separates this from a shuffle.
//!
//! A creature put onto the battlefield attacking never "attacked" (CR 508.1), so
//! it is not part of the CR 506.5 declaration record; `declared_attackers` must
//! stay exactly the creature that was declared.

mod common;

use aperture_core::state::LibrarySearchMode;
use aperture_core::*;
use common::*;

fn cub() -> CardId {
    catalog::by_name("Owlbear Cub").expect("Owlbear Cub is in the pool")
}

/// A card that is definitely a creature in the pool, and a card that definitely
/// is not.
fn creature_card() -> CardId {
    CardId::GrizzlyBears
}

fn library_cards(g: &Game, p: PlayerId) -> Vec<CardId> {
    g.state
        .player(p)
        .library
        .iter()
        .map(|id| g.state.object(*id).card)
        .collect()
}

/// Declare `attackers` from P0 at P1 (no blocks) and let the declare-attackers
/// step run until the table holds a pending choice or the stack is empty.
fn attack_and_settle(g: &mut Game, attackers: &[ObjectId]) {
    g.state.turn.attackers.clear();
    g.state.turn.blockers.clear();
    g.state.turn.blocked.clear();
    g.state.turn.declared_attackers.clear();
    g.state.turn.declared_blockers.clear();
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
    for _ in 0..24 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return;
        }
        let p = g
            .decision_owner()
            .expect("someone holds priority after the declaration");
        submit_ok(g, p, Action::Pass);
    }
}

/// Pass priority with whoever holds it until `step` is reached with an empty
/// stack and no pending choice.
fn run_to_step(g: &mut Game, step: Step) {
    for _ in 0..128 {
        if g.state.pending.is_none() && g.state.stack.is_empty() && g.state.turn.step == step {
            return;
        }
        if g.state.is_over() {
            return;
        }
        let Some(p) = g.decision_owner() else {
            return;
        };
        submit_ok(g, p, Action::Pass);
    }
    panic!("never reached {:?}", step);
}

/// The pending look, as the tests read it.
struct Look {
    player: PlayerId,
    remaining: u8,
    chosen: Vec<ObjectId>,
    eligible: Vec<ObjectId>,
    mode: LibrarySearchMode,
}

fn search_pending(g: &Game) -> Option<Look> {
    match &g.state.pending {
        Some(Pending::SearchLibrary {
            player,
            remaining,
            chosen,
            eligible,
            mode,
            ..
        }) => Some(Look {
            player: *player,
            remaining: *remaining,
            chosen: chosen.clone(),
            eligible: eligible.clone(),
            mode: *mode,
        }),
        _ => None,
    }
}

fn pick(g: &mut Game, card: ObjectId) {
    let action = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::SearchLibraryPick { card: c } if c == card))
        .map(|a| a.action)
        .expect("the creature card is offered");
    let p = g.decision_owner().expect("the looking player decides");
    submit_ok(g, p, action);
}

fn done(g: &mut Game) {
    let p = g.decision_owner().expect("the looking player decides");
    submit_ok(g, p, Action::SearchLibraryDone);
}

/// The card exists with its printed characteristics — the trigger text is in the
/// catalog verbatim, so the ability is not authored by hand here.
#[test]
fn owlbear_cub_is_registered_with_its_printed_text() {
    let def = catalog::by_name("Owlbear Cub").expect("in the pool").def();
    assert_eq!(def.name, "Owlbear Cub");
    assert_eq!(def.cost, Some("{2}{G}"));
    assert_eq!(def.type_line, "Creature — Bird Bear");
    assert_eq!(def.power, Some(3));
    assert_eq!(def.toughness, Some(3));
    assert!(
        def.oracle
            .starts_with("Mama's Coming — Whenever this creature attacks a player"),
        "printed trigger line: {}",
        def.oracle
    );
}

/// CR 603.4: the printed intervening-if is the *defending* player's land count.
/// Seven lands means the trigger never reaches the stack, so nothing is looked
/// at and no choice is offered.
#[test]
fn a_defender_with_seven_lands_never_triggers_the_look() {
    let mut g = fixture(4801)
        .battlefield(P0, &[cub()])
        .battlefield(P1, &[CardId::Forest; 7])
        .library(P0, &[creature_card(); 12])
        .library(P1, &[CardId::Forest; 3])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let owl = battlefield_card(&g, P0, cub());
    object_mut(&mut g.state, owl).summoning_sick = false;
    let before = library_cards(&g, P0);

    attack_and_settle(&mut g, &[owl]);

    assert!(
        search_pending(&g).is_none(),
        "seven lands is not eight: nothing may be looked at"
    );
    assert!(
        g.state.stack.is_empty(),
        "no trigger may be waiting on the stack"
    );
    assert_eq!(
        library_cards(&g, P0),
        before,
        "an untriggered card may not reorder its controller's library"
    );
}

/// Eight lands triggers the look: exactly the top eight are examined, and only
/// the creature cards among them are offered — the lands in the look are not
/// "creature cards from among them".
#[test]
fn eight_lands_looks_at_the_top_eight_and_offers_only_creature_cards() {
    let top_eight = [
        CardId::Forest,
        creature_card(),
        CardId::Forest,
        CardId::Forest,
        CardId::OrdinaryBear,
        CardId::Forest,
        CardId::Forest,
        CardId::Forest,
    ];
    let mut g = fixture(4802)
        .battlefield(P0, &[cub()])
        .battlefield(P1, &[CardId::Forest; 8])
        .library(P0, &top_eight)
        .add(P0, Zone::Library, CardId::RuneclawBear)
        .library(P1, &[CardId::Forest; 3])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let owl = battlefield_card(&g, P0, cub());
    object_mut(&mut g.state, owl).summoning_sick = false;
    let library_before = g.state.player(P0).library.clone();

    attack_and_settle(&mut g, &[owl]);

    let look = search_pending(&g).expect("eight lands offers the look");
    assert_eq!(look.player, P0, "the attacking creature's controller looks");
    assert_eq!(look.remaining, 1, "at most one creature card may be put");
    assert!(look.chosen.is_empty());
    assert_eq!(
        look.mode,
        LibrarySearchMode::CreatureFromTopToBattlefieldAttacking
    );
    let eligible_cards: Vec<CardId> = look
        .eligible
        .iter()
        .map(|id| g.state.object(*id).card)
        .collect();
    assert_eq!(
        eligible_cards,
        vec![creature_card(), CardId::OrdinaryBear],
        "only the creature cards among the top eight are eligible"
    );
    // Exactly the top eight were looked at: the ninth card is not offered.
    assert!(
        !look.eligible.contains(&library_before[8]),
        "the ninth card is outside the look"
    );
}

/// Declining puts nothing onto the battlefield, and the looked-at cards go to the
/// bottom in a random order while the rest of the library keeps its order — the
/// ninth card becomes the top card, which a whole-library shuffle would not do.
#[test]
fn declining_puts_the_looked_cards_on_the_bottom_and_keeps_the_rest_in_order() {
    let mut g = fixture(4803)
        .battlefield(P0, &[cub()])
        .battlefield(P1, &[CardId::Forest; 8])
        .library(
            P0,
            &[
                CardId::Forest,
                creature_card(),
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::RuneclawBear,
                CardId::GrizzlyBears,
                CardId::OrdinaryBear,
            ],
        )
        .library(P1, &[CardId::Forest; 3])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let owl = battlefield_card(&g, P0, cub());
    object_mut(&mut g.state, owl).summoning_sick = false;
    let library_before = g.state.player(P0).library.clone();

    attack_and_settle(&mut g, &[owl]);
    assert!(search_pending(&g).is_some(), "the look is offered");
    done(&mut g);

    let library_after = g.state.player(P0).library.clone();
    assert_eq!(
        library_after.len(),
        library_before.len(),
        "no card leaves the library"
    );
    assert_eq!(
        library_after[0], library_before[8],
        "the first card below the looked-at pile becomes the top card"
    );
    assert_eq!(
        &library_after[1..3],
        &library_before[9..11],
        "everything that was not looked at keeps its relative order"
    );
    let mut looked = library_before[..8].to_vec();
    let mut moved = library_after[3..].to_vec();
    looked.sort_unstable();
    moved.sort_unstable();
    assert_eq!(
        looked, moved,
        "the rest of the look goes to the bottom as a set"
    );
    assert_eq!(
        g.state.battlefield(P0).len(),
        1,
        "declining puts no permanent onto the battlefield"
    );
}

/// Choosing a creature card puts it onto the battlefield tapped and attacking
/// *that player*, and it really deals combat damage; but it was never declared as
/// an attacker, so the CR 506.5 declaration record does not contain it.
#[test]
fn the_chosen_creature_enters_tapped_attacking_and_deals_combat_damage() {
    let mut g = fixture(4804)
        .battlefield(P0, &[cub()])
        .battlefield(P1, &[CardId::Forest; 8])
        .library(
            P0,
            &[
                CardId::Forest,
                CardId::RuneclawBear,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .library(P1, &[CardId::Forest; 3])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let owl = battlefield_card(&g, P0, cub());
    object_mut(&mut g.state, owl).summoning_sick = false;
    let bear_card = g.state.player(P0).library[1];
    let life_before = g.state.player(P1).life;

    attack_and_settle(&mut g, &[owl]);
    assert!(search_pending(&g).is_some(), "the look is offered");
    pick(&mut g, bear_card);
    done(&mut g);

    let put_on_field = g
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.id == bear_card)
        .expect("the chosen creature card is on the battlefield");
    assert_eq!(put_on_field.card, CardId::RuneclawBear);
    assert!(put_on_field.tapped, "it enters tapped");
    assert!(
        g.state.turn.attackers.contains(&bear_card),
        "it is attacking"
    );
    assert_eq!(
        g.state.turn.declared_attackers,
        vec![owl],
        "it was never declared as an attacker (CR 508.1, 506.5)"
    );

    run_to_step(&mut g, Step::PostcombatMain);
    let life_after = g.state.player(P1).life;
    assert_eq!(
        life_before - life_after,
        3 + 2,
        "the Cub (3) and the put 2/2 both connect — got {}",
        life_before - life_after
    );
}

/// The look is a look, not a search: with no creature card among the top eight
/// there is nothing to choose, the whole look goes to the bottom, and the rest of
/// the library keeps its order.
#[test]
fn a_look_with_no_creature_card_puts_everything_on_the_bottom() {
    let mut g = fixture(4805)
        .battlefield(P0, &[cub()])
        .battlefield(P1, &[CardId::Forest; 8])
        .library(
            P0,
            &[
                CardId::Forest,
                CardId::Island,
                CardId::Forest,
                CardId::Island,
                CardId::Forest,
                CardId::Island,
                CardId::Forest,
                CardId::Island,
                CardId::RuneclawBear,
            ],
        )
        .library(P1, &[CardId::Forest; 3])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let owl = battlefield_card(&g, P0, cub());
    object_mut(&mut g.state, owl).summoning_sick = false;
    let before = g.state.player(P0).library.clone();

    attack_and_settle(&mut g, &[owl]);

    assert!(
        search_pending(&g).is_none(),
        "with nothing eligible there is no choice to offer"
    );
    let after = g.state.player(P0).library.clone();
    assert_eq!(after[0], before[8], "the ninth card is now on top");
    assert_eq!(after.len(), before.len());
    let mut looked = before[..8].to_vec();
    let mut bottom = after[1..].to_vec();
    looked.sort_unstable();
    bottom.sort_unstable();
    assert_eq!(looked, bottom, "all eight looked cards are on the bottom");
}
