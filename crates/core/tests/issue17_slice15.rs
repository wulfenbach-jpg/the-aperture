//! Issue 17 slice 15: sourced Bear-token package whose costs are paid with
//! cards — Ayula's Influence (discard a land card), Bearscape (exile two cards
//! from your graveyard) and Mother Bear (exile this card from your graveyard,
//! only as a sorcery). Every test below drives the real engine: legal actions,
//! `validate`, cost payment, and resolution. Nothing here reads the handoff.

mod common;

use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn card(name: &str) -> CardId {
    catalog::by_name(name).unwrap_or_else(|| panic!("{name} must be supported"))
}

/// Pass priority until the stack and the trigger queue are empty. The engine
/// grants priority only after resolution, so this is the honest way to let an
/// activated ability finish.
fn drain_stack(game: &mut Game) {
    for _ in 0..32 {
        if game.state.stack.is_empty() && game.state.trigger_queue.is_empty() {
            return;
        }
        let Some(owner) = game.decision_owner() else {
            return;
        };
        submit_ok(game, owner, Action::Pass);
    }
    panic!("stack never drained");
}

fn costed_offers(game: &Game, source: ObjectId) -> Vec<Action> {
    game.legal_actions()
        .into_iter()
        .filter(|legal| {
            matches!(
                &legal.action,
                Action::ActivateCostedAbility { source: s, .. } if *s == source
            )
        })
        .map(|legal| legal.action)
        .collect()
}

/// Every activation offered for one source, whatever its cost shape. Mother
/// Bear names no cards in `cost_cards` (it exiles itself), so its only offer is
/// the plain activation.
fn activation_offers(game: &Game, source: ObjectId) -> Vec<Action> {
    game.legal_actions()
        .into_iter()
        .filter(|legal| match &legal.action {
            Action::ActivateCostedAbility { source: s, .. } if *s == source => true,
            Action::ActivateAbility { source: s, .. } if *s == source => true,
            _ => false,
        })
        .map(|legal| legal.action)
        .collect()
}

fn graveyard_card(game: &Game, p: PlayerId, card: CardId) -> ObjectId {
    *game
        .state
        .player(p)
        .graveyard
        .iter()
        .find(|id| game.state.object(**id).card == card)
        .unwrap_or_else(|| panic!("{card:?} not in P{p}'s graveyard"))
}

fn bears_on_battlefield(game: &Game, p: PlayerId) -> Vec<ObjectId> {
    game.state
        .battlefield(p)
        .into_iter()
        .filter(|object| object.card == CardId::BearToken)
        .map(|object| object.id)
        .collect()
}

#[test]
fn sourced_slice15_cards_match_their_printings() {
    let influence = card("Ayula's Influence").def();
    assert_eq!(influence.cost, Some("{G}{G}{G}"));
    assert_eq!(influence.type_line, "Enchantment");
    assert!(influence.is_enchantment() && !influence.is_creature());
    assert_eq!(
        influence.oracle,
        "Discard a land card: Create a 2/2 green Bear creature token."
    );

    let bearscape = card("Bearscape").def();
    assert_eq!(bearscape.cost, Some("{1}{G}{G}"));
    assert_eq!(bearscape.type_line, "Enchantment");
    assert_eq!(
        bearscape.oracle,
        "{1}{G}, Exile two cards from your graveyard: Create a 2/2 green Bear creature token."
    );

    let mother = card("Mother Bear").def();
    assert_eq!(mother.cost, Some("{1}{G}"));
    assert_eq!(mother.type_line, "Creature — Bear");
    assert_eq!((mother.power, mother.toughness), (Some(2), Some(2)));
    assert_eq!(
        mother.oracle,
        "{3}{G}{G}, Exile this card from your graveyard: Create two 2/2 green Bear creature tokens. Activate only as a sorcery."
    );

    let bear = catalog::def(CardId::BearToken);
    assert_eq!(bear.name, "Bear Token");
    assert_eq!(bear.type_line, "Token Creature — Bear");
    assert!(bear.is_creature());
    assert_eq!((bear.power, bear.toughness), (Some(2), Some(2)));
    assert_eq!(bear.colors, &[card::ManaColor::Green]);
}

#[test]
fn ayula_influence_discards_exactly_the_named_land_card() {
    let mut game = fixture(1510)
        .hand(
            P0,
            &[CardId::Forest, CardId::Mountain, CardId::GrizzlyBears],
        )
        .battlefield(P0, &[CardId::AyulasInfluence])
        .main(P0, Step::PrecombatMain)
        .build();
    let influence = battlefield_card(&game, P0, CardId::AyulasInfluence);

    // One offered activation per land card in hand — never one per non-land.
    let offers = costed_offers(&game, influence);
    assert_eq!(
        offers.len(),
        2,
        "one option per land card in hand: {offers:?}"
    );

    // A creature card cannot pay the cost.
    let creature = hand_card(&game, P0, CardId::GrizzlyBears);
    let err = submit_err(
        &mut game,
        P0,
        Action::ActivateCostedAbility {
            source: influence,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
            cost_cards: vec![creature],
        },
    );
    assert!(err.to_string().contains("land"), "{err}");
    assert_eq!(
        game.state.object(creature).zone,
        Zone::Hand,
        "a refused cost mutates nothing"
    );

    let forest = hand_card(&game, P0, CardId::Forest);
    submit_ok(
        &mut game,
        P0,
        Action::ActivateCostedAbility {
            source: influence,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
            cost_cards: vec![forest],
        },
    );
    // CR 118.3: the cost is paid as part of activating.
    assert_eq!(game.state.object(forest).zone, Zone::Graveyard);
    assert!(bears_on_battlefield(&game, P0).is_empty(), "no token yet");
    drain_stack(&mut game);
    let bears = bears_on_battlefield(&game, P0);
    assert_eq!(bears.len(), 1, "one 2/2 green Bear token");
    assert!(
        hand_card_optional(&game, P0, CardId::Mountain).is_some(),
        "the Mountain in hand was not discarded"
    );
    assert_eq!(game.state.player(P0).hand.len(), 2, "one land discarded");
}

fn hand_card_optional(game: &Game, p: PlayerId, card: CardId) -> Option<ObjectId> {
    game.state
        .player(p)
        .hand
        .iter()
        .copied()
        .find(|id| game.state.object(*id).card == card)
}

#[test]
fn bearscape_needs_two_graveyard_cards_and_exiles_exactly_them() {
    let mut game = fixture(1511)
        .add(P0, Zone::Graveyard, CardId::GrizzlyBears)
        .battlefield(P0, &[CardId::Bearscape, CardId::Forest, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let bearscape = battlefield_card(&game, P0, CardId::Bearscape);

    // One card in the graveyard cannot pay "exile two cards".
    assert!(
        costed_offers(&game, bearscape).is_empty(),
        "the cost is not payable with a single graveyard card"
    );

    let second = fixture(1512)
        .add(P0, Zone::Graveyard, CardId::GrizzlyBears)
        .add(P0, Zone::Graveyard, CardId::Island)
        .battlefield(P0, &[CardId::Bearscape, CardId::Forest, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let mut game2 = second;
    let bearscape2 = battlefield_card(&game2, P0, CardId::Bearscape);
    let offers = costed_offers(&game2, bearscape2);
    assert_eq!(offers.len(), 1, "one pairing of two graveyard cards");
    let chosen = match &offers[0] {
        Action::ActivateCostedAbility { cost_cards, .. } => cost_cards.clone(),
        other => panic!("unexpected offer {other:?}"),
    };
    assert_eq!(chosen.len(), 2);

    submit_ok(
        &mut game2,
        P0,
        Action::ActivateCostedAbility {
            source: bearscape2,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
            cost_cards: chosen.clone(),
        },
    );
    let graveyard = game2.state.player(P0).graveyard.clone();
    let exile = game2.state.player(P0).exile.clone();
    assert_eq!(
        graveyard.len(),
        0,
        "both graveyard cards were exiled as the cost"
    );
    assert_eq!(exile.len(), 2);
    for id in &chosen {
        assert_eq!(
            game2.state.object(*id).zone,
            Zone::Exile,
            "the named card is exiled as the cost"
        );
    }
    assert_eq!(
        game2.state.object(bearscape2).zone,
        Zone::Battlefield,
        "the enchantment is not sacrificed to pay the ability"
    );
    assert!(!game2.state.object(bearscape2).tapped);
    drain_stack(&mut game2);
    assert_eq!(bears_on_battlefield(&game2, P0).len(), 1);

    // The same pairing submitted twice is a duplicate cost, not two payments.
    let only = graveyard_card(&game, P0, CardId::GrizzlyBears);
    let err = submit_err(
        &mut game,
        P0,
        Action::ActivateCostedAbility {
            source: bearscape,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
            cost_cards: vec![only, only],
        },
    );
    assert!(
        err.to_string().contains("graveyard") || err.to_string().contains("twice"),
        "{err}"
    );
}

#[test]
fn mother_bear_is_activated_only_from_the_graveyard_and_only_as_a_sorcery() {
    // On the battlefield the card offers nothing: its only ability is the
    // graveyard one.
    let on_board = fixture(1513)
        .battlefield(P0, &[CardId::MotherBear])
        .main(P0, Step::PrecombatMain)
        .build();
    let on_board_id = battlefield_card(&on_board, P0, CardId::MotherBear);
    assert!(
        costed_offers(&on_board, on_board_id).is_empty()
            && !on_board
                .legal_actions()
                .iter()
                .any(|legal| matches!(legal.action, Action::ActivateAbility { source, .. } if source == on_board_id)),
        "Mother Bear's ability is not usable from the battlefield"
    );

    // In the graveyard during an opponent's turn: refused.
    let mut opponent_turn = fixture(1514)
        .add(P0, Zone::Graveyard, CardId::MotherBear)
        .battlefield(P0, &[CardId::Forest; 5])
        .main(P1, Step::PrecombatMain)
        .build();
    set_priority(&mut opponent_turn.state, Some(P0));
    let mother = graveyard_card(&opponent_turn, P0, CardId::MotherBear);
    let err = submit_err(
        &mut opponent_turn,
        P0,
        Action::ActivateAbility {
            source: mother,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert!(
        err.to_string().contains("main phase") || err.to_string().contains("sorcery"),
        "{err}"
    );

    // Own main phase, empty stack: offered and executable.
    let mut game = fixture(1515)
        .add(P0, Zone::Graveyard, CardId::MotherBear)
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .main(P0, Step::PrecombatMain)
        .build();
    let mother = graveyard_card(&game, P0, CardId::MotherBear);
    assert_eq!(activation_offers(&game, mother).len(), 1);

    submit_ok(
        &mut game,
        P0,
        Action::ActivateAbility {
            source: mother,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    // CR 118.3: the card itself left the graveyard as the cost, and the ability
    // is on the stack carrying its card as last known information.
    assert_eq!(game.state.player(P0).graveyard.len(), 0);
    assert_eq!(game.state.player(P0).exile.len(), 1);
    assert_eq!(game.state.stack.len(), 1);
    assert_eq!(
        game.state.object(mother).zone,
        Zone::Exile,
        "the source card itself is exiled"
    );
    drain_stack(&mut game);
    assert_eq!(
        bears_on_battlefield(&game, P0).len(),
        2,
        "two 2/2 green Bear tokens"
    );
}

#[test]
fn enchantment_permanents_cast_from_hand_and_stay_on_the_battlefield() {
    let mut game = fixture(1516)
        .hand(P0, &[CardId::AyulasInfluence])
        .battlefield(P0, &[CardId::Forest, CardId::Forest, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let card_id = hand_card(&game, P0, CardId::AyulasInfluence);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: card_id,
            targets: Vec::new(),
        },
    );
    drain_stack(&mut game);
    let enchantment = battlefield_card(&game, P0, CardId::AyulasInfluence);
    assert_eq!(game.state.object(enchantment).zone, Zone::Battlefield);
    assert!(game.state.player(P0).graveyard.is_empty(), "not a sorcery");
}

#[test]
fn bear_tokens_enter_as_creatures_and_trigger_another_creature_enters() {
    // Soul Warden watches "another creature enters": a Bear token is a real
    // creature entering, so the token path must feed the same watcher.
    let mut game = fixture(1517)
        .hand(P0, &[CardId::Forest])
        .battlefield(P0, &[CardId::AyulasInfluence, CardId::SoulWarden])
        .life(P0, 20)
        .main(P0, Step::PrecombatMain)
        .build();
    let influence = battlefield_card(&game, P0, CardId::AyulasInfluence);
    let forest = hand_card(&game, P0, CardId::Forest);
    submit_ok(
        &mut game,
        P0,
        Action::ActivateCostedAbility {
            source: influence,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
            cost_cards: vec![forest],
        },
    );
    drain_stack(&mut game);
    let bears = bears_on_battlefield(&game, P0);
    assert_eq!(bears.len(), 1);
    let bear = game.state.object(bears[0]);
    assert_eq!(bear.zone, Zone::Battlefield);
    assert_eq!(bear.controller, P0);
    assert!(bear.summoning_sick, "CR 302.6");
    assert_eq!(game.state.player(P0).life, 21, "Soul Warden saw the token");
}
