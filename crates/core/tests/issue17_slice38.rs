//! Slice 38: Spider-Ham, Peter Porker (SPM 114) — the pool's first *artifact*
//! token and its first list-filtered subtype anthem.
//!
//! Printed text (Scryfall SPM 114, immutable id
//! `41f18f42-b86b-4a12-9f0d-76b761571195`, oracle id
//! `00b50215-e832-417f-9c80-2bc3050b6ebb`, retrieved 2026-10-28; raw response and
//! the retrieval stamp in `evidence/issue17/slice38/sources/`, fixture
//! `fixtures/starter-corpus/slice38-sourced.json`):
//!
//! "When Spider-Ham enters, create a Food token. (It's an artifact with \"{2},
//! {T}, Sacrifice this token: You gain 3 life.\")
//! Animal May-Ham — Other Spiders, Boars, Bats, Bears, Birds, Cats, Dogs, Frogs,
//! Jackals, Lizards, Mice, Otters, Rabbits, Raccoons, Rats, Squirrels, Turtles,
//! and Wolves you control get +1/+1."
//!
//! Every assertion below drives the engine through its own legal actions,
//! resolutions and state-based actions; none reads a test-only shortcut.
mod common;

use aperture_core::ability::{self, Ability};
use aperture_core::catalog;
use aperture_core::*;
use common::*;

fn spider_ham() -> CardId {
    catalog::by_name("Spider-Ham, Peter Porker").expect("Spider-Ham must be supported")
}

fn stats(g: &Game, id: ObjectId) -> (i32, i32) {
    let object = g.state.object(id);
    (object.power(), object.toughness())
}

fn pass_until_settled(g: &mut Game, limit: usize) -> Vec<Event> {
    let mut log = Vec::new();
    for _ in 0..limit {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return log;
        }
        let Some(player) = g.decision_owner() else {
            return log;
        };
        log.extend(submit_ok(g, player, Action::Pass));
    }
    panic!("the stack did not settle");
}

fn pay(g: &mut Game, seat: PlayerId, green: u32, colorless: u32) {
    let pool = &mut g.state.players[seat as usize].mana;
    pool.add(ManaColor::Green, green);
    pool.add(ManaColor::Colorless, colorless);
}

fn cast_spider_ham(g: &mut Game) {
    pay(g, P0, 1, 1);
    let card = hand_card(g, P0, spider_ham());
    submit_ok(
        g,
        P0,
        Action::CastSpell {
            card,
            targets: Vec::new(),
        },
    );
}

fn activate_food(g: &mut Game) -> Vec<Event> {
    let food = battlefield_card(g, P0, CardId::FoodToken);
    pay(g, P0, 0, 2);
    submit_ok(
        g,
        P0,
        Action::ActivateAbility {
            source: food,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    )
}

#[test]
fn spider_ham_is_sourced_and_supported() {
    let def = spider_ham().def();
    assert_eq!(def.name, "Spider-Ham, Peter Porker");
    assert_eq!(def.cost, Some("{1}{G}"));
    assert_eq!(def.type_line, "Legendary Creature — Spider Boar Hero");
    assert_eq!((def.power, def.toughness), (Some(2), Some(2)));
    assert_eq!(
        def.oracle,
        "When Spider-Ham enters, create a Food token. (It's an artifact with \"{2}, {T}, Sacrifice this token: You gain 3 life.\")\nAnimal May-Ham — Other Spiders, Boars, Bats, Bears, Birds, Cats, Dogs, Frogs, Jackals, Lizards, Mice, Otters, Rabbits, Raccoons, Rats, Squirrels, Turtles, and Wolves you control get +1/+1."
    );

    let triggered: Vec<_> = ability::abilities(spider_ham())
        .iter()
        .filter_map(Ability::triggered)
        .collect();
    assert_eq!(triggered.len(), 1);
    assert_eq!(triggered[0].event, ability::TriggerEvent::EntersBattlefield);
    assert_eq!(triggered[0].target, TargetSpec::None);
    assert_eq!(triggered[0].effects, &[Effect::CreateFoodToken]);

    let statics = ability::static_abilities(spider_ham());
    assert_eq!(statics.len(), 1);
    assert_eq!(statics[0].layer(), Some(Layer::Modifying));
    assert!(
        statics[0]
            .text()
            .starts_with("Animal May-Ham — Other Spiders")
    );

    // The Food token is a token card, not a pool card: an artifact with no
    // power/toughness and exactly the printed activated ability (CR 111.10).
    let food = CardId::FoodToken.def();
    assert_eq!(food.type_line, "Token Artifact — Food");
    assert!(!food.is_creature() && food.is_artifact());
    assert_eq!((food.power, food.toughness), (None, None));
    let food_ability = ability::abilities(CardId::FoodToken);
    assert_eq!(food_ability.len(), 1);
    let activated = food_ability[0]
        .activated()
        .expect("Food's ability is activated");
    assert_eq!(
        activated.text,
        "{2}, {T}, Sacrifice this token: You gain 3 life."
    );
    assert!(activated.cost.mana == Some("{2}"));
    assert!(activated.cost.tap, "the cost taps the token (CR 107.5)");
    assert!(
        activated.cost.sacrifice_source,
        "the cost sacrifices the token itself"
    );
    assert_eq!(activated.effects, &[Effect::GainLife(3)]);
}

#[test]
fn entering_creates_one_food_token_that_is_an_artifact_and_not_a_creature() {
    let mut g = fixture(3801)
        .hand(P0, &[spider_ham()])
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .main(P0, Step::PrecombatMain)
        .build();
    cast_spider_ham(&mut g);
    let log = pass_until_settled(&mut g, 64);
    let ham = battlefield_card(&g, P0, spider_ham());
    assert_eq!(stats(&g, ham), (2, 2), "Spider-Ham never pumps himself");

    let food = battlefield_card(&g, P0, CardId::FoodToken);
    let object = g.state.object(food);
    assert!(
        !g.is_creature(object),
        "Food is an artifact, not a creature"
    );
    assert!(
        object.card.def().has_subtype("Food"),
        "the token really carries the Food subtype"
    );
    assert_eq!(
        g.state.players[P0 as usize].life, 20,
        "creating costs no life"
    );
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::PermanentEntered { card, .. } if *card == CardId::FoodToken
        )),
        "the token's entry is a real event: {log:?}"
    );

    // The token's printed ability is offered by the engine's own legal actions
    // once its mana cost can be paid from the pool.
    pay(&mut g, P0, 0, 2);
    let offers: Vec<Action> = g.legal_actions().into_iter().map(|a| a.action).collect();
    assert!(
        offers.contains(&Action::ActivateAbility {
            source: food,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        }),
        "an untapped Food with two mana available is activatable: {offers:?}"
    );
}

#[test]
fn activating_the_food_gains_three_life_and_sacrifices_the_token() {
    let mut g = fixture(3802)
        .hand(P0, &[spider_ham()])
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .main(P0, Step::PrecombatMain)
        .build();
    cast_spider_ham(&mut g);
    pass_until_settled(&mut g, 64);
    let food = battlefield_card(&g, P0, CardId::FoodToken);

    let activation = activate_food(&mut g);
    assert!(
        activation.iter().any(|event| matches!(
            event,
            Event::AbilityActivated { source, .. } if *source == food
        )),
        "the activation is a real event: {activation:?}"
    );
    // CR 602.2b: the sacrifice half of the cost is paid as the ability is
    // activated, before it resolves — the effect has not happened yet, and the
    // token has already left the battlefield for its owner's graveyard.
    assert_eq!(
        g.state.players[P0 as usize].life, 20,
        "the ability is on the stack: no life has been gained yet"
    );
    assert!(
        activation.iter().any(|event| matches!(
            event,
            Event::Destroyed { card, .. } if *card == CardId::FoodToken
        )),
        "the token's zone change happened while activating, not on resolution: {activation:?}"
    );
    assert!(
        activation.iter().any(|event| matches!(
            event,
            Event::StateBasedAction { description }
                if description.contains("Food Token")
                    && description.contains("will cease to exist (CR 111.7)")
        )),
        "the sacrifice records the token's pending cessation (CR 111.7): {activation:?}"
    );
    assert!(
        activation.iter().any(|event| matches!(
            event,
            Event::StateBasedAction { description }
                if description == "Food Token token ceases to exist (CR 111.7)"
        )),
        "the state-based action really removes the token (CR 111.7): {activation:?}"
    );
    assert!(
        g.state.try_object(food).is_none(),
        "CR 111.7: the sacrificed token ceases to exist — it is not parked in a zone as a card"
    );
    assert_eq!(
        g.state.players[P0 as usize].mana.colorless, 0,
        "the {{2}} half of the cost was really spent"
    );
    let log = pass_until_settled(&mut g, 64);
    assert_eq!(
        g.state.players[P0 as usize].life, 23,
        "resolving the ability gains exactly 3 life"
    );
    assert!(
        log.iter().any(|event| matches!(
            event,
            Event::LifeChange { player, delta, .. } if *player == P0 && *delta == 3
        )),
        "the life gain is a real event: {log:?}"
    );
    assert!(
        !g.state
            .battlefield(P0)
            .iter()
            .any(|object| object.id == food),
        "the sacrificed Food token is no longer a battlefield permanent"
    );
    assert!(
        !g.state.players[P0 as usize].graveyard.contains(&food),
        "the token is not left in the graveyard list: {:?}",
        g.state.players[P0 as usize].graveyard
    );
    let view = g.view_for(P0, &log);
    assert!(
        !view
            .own_graveyard
            .iter()
            .any(|card| card.card == CardId::FoodToken),
        "the public graveyard view shows no Food card: {:?}",
        view.own_graveyard
            .iter()
            .map(|card| card.card)
            .collect::<Vec<_>>()
    );
    let offers: Vec<Action> = g.legal_actions().into_iter().map(|a| a.action).collect();
    assert!(
        !offers.iter().any(|action| matches!(
            action,
            Action::ActivateAbility { source, .. } if *source == food
        )),
        "a token that left the battlefield is no longer activatable"
    );
}

#[test]
fn sacrificed_food_tokens_do_not_count_as_graveyard_cards_for_threshold() {
    // The cross-card consequence review 301 demonstrated: threshold counts
    // graveyard *objects* (`engine.rs` ThresholdPowerToughness), so a token that
    // wrongly survives in the graveyard inflates it. Three real cards plus four
    // sacrificed Foods must leave three objects and a 1/1 Werebear, because the
    // printed condition is seven or more *cards*.
    let mut g = fixture(3809)
        .battlefield(
            P0,
            &[
                CardId::Werebear,
                CardId::FoodToken,
                CardId::FoodToken,
                CardId::FoodToken,
                CardId::FoodToken,
            ],
        )
        .add(P0, Zone::Graveyard, CardId::Forest)
        .add(P0, Zone::Graveyard, CardId::Forest)
        .add(P0, Zone::Graveyard, CardId::Forest)
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .main(P0, Step::PrecombatMain)
        .build();
    let werebear = battlefield_card(&g, P0, CardId::Werebear);
    let foods: Vec<ObjectId> = g
        .state
        .battlefield(P0)
        .iter()
        .filter(|object| object.card == CardId::FoodToken)
        .map(|object| object.id)
        .collect();
    assert_eq!(foods.len(), 4, "four Food tokens on the battlefield");
    assert_eq!(
        stats(&g, werebear),
        (1, 1),
        "three cards is below the printed threshold of seven"
    );

    for food in foods {
        pay(&mut g, P0, 0, 2);
        let activation = submit_ok(
            &mut g,
            P0,
            Action::ActivateAbility {
                source: food,
                ability: 0,
                mode: 0,
                targets: Vec::new(),
            },
        );
        pass_until_settled(&mut g, 64);
        assert!(
            activation.iter().any(|event| matches!(
                event,
                Event::StateBasedAction { description }
                    if description == "Food Token token ceases to exist (CR 111.7)"
            )),
            "each sacrificed Food ceases to exist: {activation:?}"
        );
        assert!(
            g.state.try_object(food).is_none(),
            "the sacrificed token is gone from the state"
        );
    }

    assert_eq!(
        g.state.players[P0 as usize].graveyard.len(),
        3,
        "three real cards, and no phantom Food objects: {:?}",
        g.state.players[P0 as usize].graveyard
    );
    assert!(
        g.state.players[P0 as usize].graveyard.iter().all(|id| g
            .state
            .try_object(*id)
            .is_some_and(|o| o.card == CardId::Forest)),
        "every graveyard object is one of the three Forests"
    );
    assert_eq!(
        stats(&g, werebear),
        (1, 1),
        "seven graveyard *cards* are required; four vanished tokens do not count"
    );
}

#[test]
fn every_token_card_is_registered_as_a_token() {
    // Review 301's blocking defect was a registration gap, not a rules gap: the
    // Food token was creatable but absent from `CardId::is_token()` and
    // `aperture_core::card::TOKEN_CARDS`, so every CR 111.7 path treated it as an ordinary
    // card. `CardId::is_token()` is now *derived* from `card::TOKEN_CARDS`, which
    // makes that registry the single source of the predicate; token
    // characteristics are still declared in two tables (`tokens::TOKEN_CARDS` for
    // Soldier/Spirit and the catalog's `TOKENS`), so this test keeps both tables
    // and the registry in step for every variant the engine can create — the next
    // token cannot repeat the mistake silently, and the registry half of this test
    // is load-bearing rather than documentary.
    let mut declared: Vec<CardId> = aperture_core::tokens::TOKEN_CARDS
        .iter()
        .map(|def| def.id)
        .chain(catalog::TOKENS.iter().map(|def| def.id))
        .collect();
    declared.sort();
    declared.dedup();
    assert!(
        declared.len() >= 8,
        "the engine declares its token variants in two tables: {declared:?}"
    );
    for id in &declared {
        assert!(
            id.is_token(),
            "{id:?} has token characteristics but CardId::is_token() says it is a card"
        );
        assert!(
            aperture_core::card::TOKEN_CARDS.contains(id),
            "{id:?} is missing from aperture_core::card::TOKEN_CARDS"
        );
        assert!(
            !catalog::all().iter().any(|def| def.id == *id),
            "{id:?} is a token and must never be an importable pool card"
        );
        assert_eq!(
            catalog::by_name(id.name()),
            None,
            "{id:?} must not be importable by name"
        );
    }
    for id in aperture_core::card::TOKEN_CARDS {
        assert!(id.is_token(), "{id:?} is registered but not a token");
        assert!(
            declared.contains(id),
            "{id:?} is registered but has no characteristics in either token table"
        );
    }
    assert!(
        CardId::FoodToken.is_token(),
        "the Food token is a token (CR 111.10)"
    );
    assert!(
        aperture_core::card::TOKEN_CARDS.contains(&CardId::FoodToken),
        "the Food token is in the token registry"
    );
}

#[test]
fn the_food_ability_is_refused_without_its_printed_cost() {
    let mut g = fixture(3803)
        .hand(P0, &[spider_ham()])
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .main(P0, Step::PrecombatMain)
        .build();
    cast_spider_ham(&mut g);
    pass_until_settled(&mut g, 64);
    let food = battlefield_card(&g, P0, CardId::FoodToken);

    // No mana: the engine never offers the activation and refuses a forged one.
    let offers: Vec<Action> = g.legal_actions().into_iter().map(|a| a.action).collect();
    assert!(
        !offers.iter().any(|action| matches!(
            action,
            Action::ActivateAbility { source, .. } if *source == food
        )),
        "an unpayable {{2}} is not offered: {offers:?}"
    );
    submit_err(
        &mut g,
        P0,
        Action::ActivateAbility {
            source: food,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert_eq!(g.state.players[P0 as usize].life, 20, "nothing was paid");
    assert!(g.state.try_object(food).is_some(), "nothing was sacrificed");

    // Tapped: the {T} half of the cost is unpayable even with the mana.
    object_mut(&mut g.state, food).tapped = true;
    pay(&mut g, P0, 0, 2);
    submit_err(
        &mut g,
        P0,
        Action::ActivateAbility {
            source: food,
            ability: 0,
            mode: 0,
            targets: Vec::new(),
        },
    );
    assert_eq!(g.state.players[P0 as usize].life, 20);
    assert!(g.state.try_object(food).is_some());
}

#[test]
fn the_anthem_pumps_listed_subtypes_only_and_never_the_source() {
    let mut g = fixture(3804)
        .battlefield(P0, &[CardId::GrizzlyBears, CardId::ElvishVisionary])
        .hand(P0, &[spider_ham()])
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .main(P0, Step::PrecombatMain)
        .build();
    let bears = battlefield_card(&g, P0, CardId::GrizzlyBears);
    let elf = battlefield_card(&g, P0, CardId::ElvishVisionary);
    assert_eq!(
        stats(&g, bears),
        (2, 2),
        "printed, before the anthem exists"
    );

    cast_spider_ham(&mut g);
    pass_until_settled(&mut g, 64);
    let ham = battlefield_card(&g, P0, spider_ham());
    assert_eq!(
        stats(&g, bears),
        (3, 3),
        "a Bear is in the printed list: 2/2 + 1/+1"
    );
    assert_eq!(
        stats(&g, elf),
        (1, 1),
        "an Elf is not in the printed list: unchanged"
    );
    assert_eq!(
        stats(&g, ham),
        (2, 2),
        "\"Other\" excludes the source itself"
    );
}

#[test]
fn the_anthem_reads_the_effective_subtype_and_stacks_in_layer_7c() {
    // Circle of the Moon Druid is a 4/2 Bear during its controller's turn only
    // (layer 4, CR 613.1d), and its printed subtype line is Human Elf Druid. The
    // anthem must therefore read the *effective* subtype: 4/2 + 1/+1 = 5/3. A
    // printed-line read would leave it at 4/2.
    let mut g = fixture(3805)
        .battlefield(P0, &[CardId::CircleOfTheMoonDruid])
        .hand(P0, &[spider_ham(), CardId::GiantGrowth])
        .library(P0, &[CardId::Forest; 20])
        .library(P1, &[CardId::Forest; 20])
        .main(P0, Step::PrecombatMain)
        .build();
    let druid = battlefield_card(&g, P0, CardId::CircleOfTheMoonDruid);
    assert_eq!(stats(&g, druid), (2, 4), "printed 2/4, no anthem yet");

    cast_spider_ham(&mut g);
    pass_until_settled(&mut g, 64);
    assert_eq!(
        stats(&g, druid),
        (5, 3),
        "Bear Form 4/2 plus the anthem's +1/+1"
    );

    pay(&mut g, P0, 1, 0);
    let growth = hand_card(&g, P0, CardId::GiantGrowth);
    submit_ok(
        &mut g,
        P0,
        Action::CastSpell {
            card: growth,
            targets: vec![Target::Permanent(druid)],
        },
    );
    pass_until_settled(&mut g, 64);
    assert_eq!(
        stats(&g, druid),
        (8, 6),
        "counters and pumps sit on top of the layer 7c static (4/2 +1/+1 +3/+3)"
    );
}
