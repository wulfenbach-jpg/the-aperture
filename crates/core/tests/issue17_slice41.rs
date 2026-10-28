//! Slice 41: The Earth King ETB token and attack land search.
mod common;
use aperture_core::catalog;
#[test]
fn earth_king_is_sourced_and_supported() {
    let def = catalog::by_name("The Earth King").expect("supported").def();
    assert_eq!(def.cost, Some("{3}{G}"));
    assert_eq!(def.type_line, "Legendary Creature — Human Noble Ally");
    assert_eq!(def.power, Some(2));
    assert_eq!(def.toughness, Some(2));
    assert_eq!(
        def.oracle,
        "When The Earth King enters, create a 4/4 green Bear creature token.\nWhenever one or more creatures you control with power 4 or greater attack, search your library for up to that many basic land cards, put them onto the battlefield tapped, then shuffle."
    );
}

use aperture_core::*;
use common::*;
fn card41(n: &str) -> CardId {
    catalog::by_name(n).unwrap()
}
fn drain(g: &mut Game) {
    for _ in 0..128 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(g, p, Action::Pass);
    }
    panic!("settle");
}

#[test]
fn earth_king_really_casts_and_creates_distinct_four_four_bear() {
    let mut g = fixture(4101)
        .hand(P0, &[card41("The Earth King")])
        .battlefield(P0, &[CardId::Forest; 4])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let spell = hand_card(&g, P0, card41("The Earth King"));
    let cast = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action,Action::CastSpell{card,..} if *card==spell))
        .expect("offered cast")
        .action;
    submit_ok(&mut g, P0, cast);
    drain(&mut g);
    let king = battlefield_card(&g, P0, card41("The Earth King"));
    let token = g
        .state
        .battlefield(P0)
        .into_iter()
        .find(|o| o.card == CardId::EarthKingBearToken)
        .expect("4/4 Bear token");
    assert_eq!(token.power(), 4);
    assert_eq!(token.toughness(), 4);
    assert_eq!(
        g.state.object(token.id).card.def().colors,
        &[aperture_core::card::ManaColor::Green]
    );
    assert!(aperture_core::card::TOKEN_CARDS.contains(&CardId::EarthKingBearToken));
    assert!(catalog::by_name("Bear Token").is_none());
    assert_eq!(
        g.state
            .object(king)
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        0
    );
}

#[test]
fn attack_search_is_one_trigger_for_two_qualifiers_and_puts_basic_lands_tapped() {
    let mut g = fixture(4102)
        .battlefield(
            P0,
            &[
                card41("The Earth King"),
                card41("Craw Wurm"),
                card41("Craw Wurm"),
            ],
        )
        .library(
            P0,
            &[
                CardId::Forest,
                CardId::Island,
                CardId::GrizzlyBears,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let king = battlefield_card(&g, P0, card41("The Earth King"));
    let wurms: Vec<_> = g
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == card41("Craw Wurm"))
        .map(|o| o.id)
        .collect();
    let mut log = Vec::new();
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
            attackers: vec![king, wurms[0], wurms[1]],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut g,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::SearchLibrary { .. })),
        64
    ));
    let (remaining, eligible) = match &g.state.pending {
        Some(Pending::SearchLibrary {
            remaining,
            eligible,
            ..
        }) => (*remaining, eligible.clone()),
        other => panic!("{other:?}"),
    };
    assert_eq!(
        remaining, 2,
        "one trigger with the qualifying attacker count"
    );
    assert_eq!(eligible.len(), 4, "only basic lands are eligible");
    assert!(
        !eligible
            .iter()
            .any(|id| g.state.object(*id).card == CardId::GrizzlyBears)
    );
    assert!(
        g.view_for(P1, &[]).pending.is_none(),
        "search choice is private"
    );
    let mut picked = Vec::new();
    for _ in 0..2 {
        let a = g
            .legal_actions()
            .into_iter()
            .find(|a| matches!(a.action, Action::SearchLibraryPick { .. }))
            .expect("pick offered")
            .action;
        if let Action::SearchLibraryPick { card } = a {
            picked.push(card);
        }
        submit_ok(&mut g, P0, a);
    }
    submit_ok(&mut g, P0, Action::SearchLibraryDone);
    drain(&mut g);
    for id in picked {
        assert_eq!(g.state.object(id).zone, Zone::Battlefield);
        assert!(g.state.object(id).tapped);
    }
    assert!(g.state.pending.is_none());
}

#[test]
fn midsearch_save_reload_keeps_private_progress_and_finishes_identically() {
    let mut g = fixture(4103)
        .battlefield(P0, &[card41("The Earth King"), card41("Craw Wurm")])
        .library(
            P0,
            &[
                CardId::Forest,
                CardId::Island,
                CardId::Forest,
                CardId::Forest,
            ],
        )
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let king = battlefield_card(&g, P0, card41("The Earth King"));
    let wurm = battlefield_card(&g, P0, card41("Craw Wurm"));
    let mut log = Vec::new();
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
            attackers: vec![king, wurm],
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut g,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::SearchLibrary { .. })),
        64
    ));
    let decks = vec![
        DeckList::parse_practice("A", "40 Forest\n").unwrap(),
        DeckList::parse_practice("B", "40 Forest\n").unwrap(),
    ];
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        decks,
        AiSettings::default(),
    )
    .unwrap();
    let save = SessionSave::capture(&g, replay, Vec::new(), true);
    let loaded = SessionSave::from_json(&save.to_json()).expect("midsearch save loads");
    let mut loaded = Game::from_state(loaded.game).unwrap();
    assert_eq!(loaded.digest(), g.digest());
    for game in [&mut g, &mut loaded] {
        let pick = game
            .legal_actions()
            .into_iter()
            .find(|a| matches!(a.action, Action::SearchLibraryPick { .. }))
            .unwrap()
            .action;
        submit_ok(game, P0, pick);
        submit_ok(game, P0, Action::SearchLibraryDone);
        drain(game);
    }
    assert_eq!(loaded.state, g.state);
    assert_eq!(loaded.digest(), g.digest());
    assert!(g.state.pending.is_none());
}
