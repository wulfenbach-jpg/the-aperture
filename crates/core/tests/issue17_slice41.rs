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

fn search_fixture(library: &[CardId], watcher: bool) -> Game {
    let mut creatures = vec![CardId::TheEarthKing, CardId::CrawWurm, CardId::CrawWurm];
    if watcher {
        creatures.push(card41("Jaddi Offshoot"));
    }
    let mut game = fixture(4141)
        .battlefield(P0, &creatures)
        .library(P0, library)
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| { g.state.turn.step == Step::DeclareAttackers && g.state.pending.is_some() },
        64
    ));
    let attackers = game
        .state
        .battlefield(P0)
        .iter()
        .filter(|o| o.card == CardId::CrawWurm)
        .map(|o| o.id)
        .collect();
    submit_ok(
        &mut game,
        P0,
        Action::DeclareAttackers {
            attackers,
            defender: Some(P1),
        },
    );
    assert!(run_until(
        &mut game,
        &mut log,
        |g| { matches!(g.state.pending, Some(Pending::SearchLibrary { .. })) },
        64
    ));
    game
}

#[test]
fn searched_lands_fire_existing_landfall_once_each_without_spending_land_play() {
    let mut game = search_fixture(&[CardId::Forest; 8], true);
    let life = game.state.player(P0).life;
    let lands_played = game.state.turn.lands_played;
    for _ in 0..2 {
        let action = game
            .legal_actions()
            .into_iter()
            .find(|a| matches!(a.action, Action::SearchLibraryPick { .. }))
            .unwrap()
            .action;
        submit_ok(&mut game, P0, action);
    }
    submit_ok(&mut game, P0, Action::SearchLibraryDone);
    drain(&mut game);
    assert_eq!(
        game.state.player(P0).life,
        life + 2,
        "each searched land entering triggers Jaddi Offshoot, even though it wasn't played"
    );
    assert_eq!(game.state.turn.lands_played, lands_played);
    assert!(
        game.state.pending.is_none()
            && game.state.stack.is_empty()
            && game.state.trigger_queue.is_empty()
            && game.state.resolving.is_none()
    );
}

#[test]
fn zero_pick_search_still_shuffles_and_forged_or_duplicate_picks_are_atomic_refusals() {
    let mut game = search_fixture(
        &[
            CardId::Forest,
            CardId::GrizzlyBears,
            CardId::Island,
            CardId::Forest,
            CardId::Mountain,
            CardId::Forest,
            CardId::Plains,
            CardId::Forest,
        ],
        false,
    );
    let nonland = game
        .state
        .player(P0)
        .library
        .iter()
        .copied()
        .find(|id| game.state.object(*id).card == CardId::GrizzlyBears)
        .unwrap();
    let before = game.state.clone();
    assert!(
        game.submit_action(P0, Action::SearchLibraryPick { card: nonland })
            .is_err()
    );
    assert_eq!(game.state, before);
    let entries_before = game.state.battlefield(P0).len();
    let events = game.submit_action(P0, Action::SearchLibraryDone).unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StateBasedAction { description }
        if description.contains("shuffles their library after searching")))
    );
    assert_eq!(game.state.battlefield(P0).len(), entries_before);
    drain(&mut game);
    assert!(game.state.pending.is_none() && game.state.resolving.is_none());

    let mut game = search_fixture(&[CardId::Forest; 8], false);
    let pick = game
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::SearchLibraryPick { .. }))
        .unwrap()
        .action;
    submit_ok(&mut game, P0, pick.clone());
    let before = game.state.clone();
    assert!(game.submit_action(P0, pick).is_err());
    assert_eq!(game.state, before);
    assert!(game.submit_action(P1, Action::SearchLibraryDone).is_err());
    assert_eq!(game.state, before);
}

#[test]
fn attack_search_resolves_from_the_captured_count_after_its_source_leaves() {
    let mut game = fixture(4143)
        .hand(P0, &[CardId::SwordsToPlowshares])
        .battlefield(
            P0,
            &[
                card41("The Earth King"),
                card41("Craw Wurm"),
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Plains,
            ],
        )
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let king = battlefield_card(&game, P0, card41("The Earth King"));
    let wurm = battlefield_card(&game, P0, card41("Craw Wurm"));
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
            attackers: vec![wurm],
            defender: Some(P1),
        },
    );
    // The trigger is on the stack; the source leaves before it resolves.
    let swords = hand_card(&game, P0, CardId::SwordsToPlowshares);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: swords,
            targets: vec![Target::Permanent(king)],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);
    assert!(
        run_until(
            &mut game,
            &mut log,
            |g| matches!(g.state.pending, Some(Pending::SearchLibrary { .. })),
            64
        ),
        "the captured search still happens with its source gone"
    );
    assert!(
        game.state
            .try_object(king)
            .is_some_and(|object| object.zone != Zone::Battlefield),
        "the source has left the battlefield"
    );
    let remaining = match &game.state.pending {
        Some(Pending::SearchLibrary { remaining, .. }) => *remaining,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        remaining, 1,
        "one qualifying attacker was captured at trigger time"
    );
    let pick = game
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::SearchLibraryPick { .. }))
        .expect("search still offered")
        .action;
    submit_ok(&mut game, P0, pick);
    submit_ok(&mut game, P0, Action::SearchLibraryDone);
    drain(&mut game);
    assert!(game.state.pending.is_none());
    assert_eq!(game.state.validate(), Ok(()));
}

#[test]
fn carved_bear_token_ceases_to_exist_after_leaving_the_battlefield() {
    let mut game = fixture(4142)
        .hand(P0, &[card41("The Earth King"), CardId::SwordsToPlowshares])
        .battlefield(
            P0,
            &[
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Forest,
                CardId::Plains,
            ],
        )
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let king = hand_card(&game, P0, card41("The Earth King"));
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: king,
            targets: Vec::new(),
        },
    );
    drain(&mut game);
    let token = game
        .state
        .battlefield(P0)
        .iter()
        .find(|o| o.card == CardId::EarthKingBearToken)
        .expect("token")
        .id;
    let swords = hand_card(&game, P0, CardId::SwordsToPlowshares);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: swords,
            targets: vec![Target::Permanent(token)],
        },
    );
    let mut events = submit_ok(&mut game, P0, Action::Pass);
    events.extend(submit_ok(&mut game, P1, Action::Pass));
    drain(&mut game);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, Event::StateBasedAction { description }
            if description.contains("ceases to exist"))),
        "the Bear token ceases to exist instead of being exiled (CR 111.7)"
    );
    assert!(game.state.try_object(token).is_none());
    assert!(!game.state.player(P0).graveyard.contains(&token));
    assert_eq!(game.state.validate(), Ok(()));
}

#[test]
fn full_opponent_projection_is_independent_of_hidden_search_candidates() {
    let a = search_fixture(&[CardId::Forest; 8], false);
    let b = search_fixture(&[CardId::GrizzlyBears; 8], false);
    assert_eq!(a.view_for(P1, &[]), b.view_for(P1, &[]));
    assert_ne!(
        a.view_for(P0, &[]).legal.len(),
        b.view_for(P0, &[]).legal.len()
    );
}

#[test]
fn midsearch_after_one_pick_save_reload_continues_identically() {
    let mut g = fixture(4104)
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
    let first = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::SearchLibraryPick { .. }))
        .unwrap()
        .action;
    submit_ok(&mut g, P0, first);
    assert!(matches!(
        g.state.pending,
        Some(Pending::SearchLibrary { remaining: 1, .. })
    ));
    g.state
        .validate()
        .expect("a legal post-pick search must validate");
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
    let loaded = SessionSave::from_json(&save.to_json()).expect("post-pick search save loads");
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
}
