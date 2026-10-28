//! Slice 44: Spirit of the Aldergard snow search and other-snow power.
mod common;
use aperture_core::state::LibrarySearchMode;
use aperture_core::*;
use common::*;
fn card44(n: &str) -> CardId {
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
fn cast44(g: &mut Game, id: ObjectId) {
    let a = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(&a.action,Action::CastSpell{card,..} if *card==id))
        .expect("offered cast")
        .action;
    submit_ok(g, P0, a);
    drain(g);
}

#[test]
fn spirit_casts_and_etb_search_reveals_snow_land_to_hand() {
    let mut g = fixture(4401)
        .hand(P0, &[card44("Spirit of the Aldergard")])
        .battlefield(P0, &[CardId::Forest; 5])
        .library(
            P0,
            &[
                CardId::Forest,
                card44("Snow-Covered Forest"),
                CardId::GrizzlyBears,
            ],
        )
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let spell = hand_card(&g, P0, card44("Spirit of the Aldergard"));
    cast44(&mut g, spell);
    assert!(matches!(
        g.state.pending,
        Some(Pending::SearchLibrary { remaining: 1, .. })
    ));
    let (eligible, mode) = match &g.state.pending {
        Some(Pending::SearchLibrary { eligible, mode, .. }) => (eligible.clone(), *mode),
        other => panic!("{other:?}"),
    };
    assert_eq!(mode, LibrarySearchMode::SnowLandToHand);
    assert_eq!(eligible.len(), 1);
    assert_eq!(
        g.state.object(eligible[0]).card,
        card44("Snow-Covered Forest")
    );
    assert!(g.view_for(P1, &[]).pending.is_none(), "search is private");
    let pick = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::SearchLibraryPick { .. }))
        .expect("snow land offered")
        .action;
    submit_ok(&mut g, P0, pick);
    submit_ok(&mut g, P0, Action::SearchLibraryDone);
    drain(&mut g);
    let id = eligible[0];
    assert_eq!(g.state.object(id).zone, Zone::Hand);
    assert_eq!(g.state.player(P0).hand.last(), Some(&id));
    assert!(
        !g.state.battlefield(P0).iter().any(|o| o.id == id),
        "snow search does not put land onto battlefield"
    );
    assert!(g.state.pending.is_none());
}

#[test]
fn static_counts_other_current_snow_permanents_only_and_excludes_self() {
    let base = |snow: usize, opp: &[CardId]| {
        let mut b = vec![CardId::Forest; 5];
        for _ in 0..snow {
            b.push(card44("Snow-Covered Forest"));
        }
        let mut g = fixture(4402)
            .hand(P0, &[card44("Spirit of the Aldergard")])
            .battlefield(P0, &b)
            .battlefield(P1, opp)
            .library(P0, &[CardId::Forest; 8])
            .library(P1, &[CardId::Forest; 8])
            .main(P0, Step::PrecombatMain)
            .build();
        let spirit = hand_card(&g, P0, card44("Spirit of the Aldergard"));
        cast44(&mut g, spirit);
        drain(&mut g);
        let spirit = battlefield_card(&g, P0, card44("Spirit of the Aldergard"));
        (
            g.state.object(spirit).power(),
            g.state.object(spirit).toughness(),
        )
    };
    assert_eq!(base(0, &[]), (0, 4));
    assert_eq!(base(1, &[]), (1, 4));
    assert_eq!(
        base(2, &[card44("Snow-Covered Forest")]),
        (2, 4),
        "self and opponent snow excluded"
    );
}

#[test]
fn snow_search_fails_to_find_and_postpick_save_continues() {
    let mut g = fixture(4403)
        .hand(P0, &[card44("Spirit of the Aldergard")])
        .battlefield(P0, &[CardId::Forest; 5])
        .library(P0, &[CardId::Forest, CardId::GrizzlyBears])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let spell = hand_card(&g, P0, card44("Spirit of the Aldergard"));
    cast44(&mut g, spell);
    assert!(matches!(
        g.state.pending,
        Some(Pending::SearchLibrary { remaining: 1, .. })
    ));
    let eligible = match &g.state.pending {
        Some(Pending::SearchLibrary { eligible, .. }) => eligible.clone(),
        _ => panic!(),
    };
    assert!(eligible.is_empty(), "no snow land exists in this library");
    let library_before = g.state.player(P0).library.clone();
    submit_ok(&mut g, P0, Action::SearchLibraryDone);
    drain(&mut g);
    assert!(g.state.pending.is_none());
    assert_eq!(g.state.player(P0).hand.len(), 0, "no card added");
    assert_eq!(g.state.player(P0).library.len(), library_before.len());
    // Real eligible path: pick, save before Done, reload and finish identically.
    let mut g = fixture(4404)
        .hand(P0, &[card44("Spirit of the Aldergard")])
        .battlefield(P0, &[CardId::Forest; 5])
        .library(P0, &[card44("Snow-Covered Forest"), CardId::Forest])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let spell = hand_card(&g, P0, card44("Spirit of the Aldergard"));
    cast44(&mut g, spell);
    let pick = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::SearchLibraryPick { .. }))
        .expect("snow pick")
        .action;
    submit_ok(&mut g, P0, pick);
    assert!(matches!(
        g.state.pending,
        Some(Pending::SearchLibrary { remaining: 0, .. })
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
    let loaded = SessionSave::from_json(&save.to_json()).expect("postpick snow search loads");
    let mut loaded = Game::from_state(loaded.game).unwrap();
    submit_ok(&mut g, P0, Action::SearchLibraryDone);
    drain(&mut g);
    submit_ok(&mut loaded, P0, Action::SearchLibraryDone);
    drain(&mut loaded);
    assert_eq!(loaded.state, g.state);
    assert_eq!(loaded.digest(), g.digest());
}
