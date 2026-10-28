//! Slice 45: Titania's Command modal pair matrix.
mod common;
use aperture_core::*;
use common::*;
fn c(n: &str) -> CardId {
    catalog::by_name(n).unwrap()
}
fn gy(g: &mut Game, p: PlayerId, c_: CardId) {
    let id = ObjectId(g.state.next_object);
    g.state.next_object += 1;
    g.state.objects.insert(
        id,
        aperture_core::state::Object {
            id,
            card: c_,
            owner: p,
            controller: p,
            zone: Zone::Graveyard,
            commander: None,
            tapped: false,
            damage: 0,
            summoning_sick: false,
            pump_power: 0,
            pump_toughness: 0,
            plus_one_counters: 0,
            counters: Default::default(),
            paired_with: None,
        },
    );
    g.state.players[p as usize].graveyard.push(id);
}
fn settle(g: &mut Game) {
    for _ in 0..200 {
        if let Some(Pending::SearchLibrary { .. }) = g.state.pending {
            let a = g
                .legal_actions()
                .into_iter()
                .find(|a| matches!(a.action, Action::SearchLibraryPick { .. }))
                .map(|a| a.action)
                .unwrap_or(Action::SearchLibraryDone);
            let p = g.decision_owner().unwrap();
            submit_ok(g, p, a);
            continue;
        }
        if g.state.stack.is_empty() {
            return;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(g, p, Action::Pass);
    }
    panic!("settle");
}

#[test]
fn pairs_complete_printed_modes_independently() {
    let pairs = [[0u8, 1], [0, 2], [0, 3], [1, 2], [1, 3], [2, 3]];
    for pair in pairs {
        let mut g = fixture(4500 + pair[0] as u64 * 10 + pair[1] as u64)
            .hand(P0, &[c("Titania's Command")])
            .battlefield(P0, &[CardId::Forest; 6])
            .battlefield(P0, &[CardId::GrizzlyBears])
            .library(P0, &[CardId::Forest; 6])
            .library(P1, &[CardId::Forest; 8])
            .main(P0, Step::PrecombatMain)
            .build();
        gy(&mut g, P1, CardId::GrizzlyBears);
        gy(&mut g, P1, CardId::GrizzlyBears);
        let bear = battlefield_card(&g, P0, CardId::GrizzlyBears);
        let initial_lands = g
            .state
            .battlefield(P0)
            .iter()
            .filter(|o| o.card.def().is_land())
            .count();
        let initial_life = g.state.player(P0).life;
        let spell = hand_card(&g, P0, c("Titania's Command"));
        let a = g
            .legal_actions()
            .into_iter()
            .find(|a| match &a.action {
                Action::CastSpellWithModes {
                    card,
                    modes,
                    targets,
                } => {
                    *card == spell
                        && *modes == pair
                        && (!modes.contains(&0) || targets == &vec![Target::Player(P1)])
                }
                _ => false,
            })
            .expect("mode pair offered")
            .action;
        submit_ok(&mut g, P0, a);
        settle(&mut g);
        let has = |m| pair.contains(&m);
        if has(0) {
            assert!(g.state.player(P1).graveyard.is_empty(), "exile graveyard");
            assert_eq!(g.state.player(P0).life, initial_life + 2);
        }
        if has(1) {
            let lands = g
                .state
                .battlefield(P0)
                .iter()
                .filter(|o| o.card.def().is_land())
                .count();
            assert!(lands >= initial_lands + 2, "two lands entered");
        }
        if has(2) {
            let bears = g
                .state
                .battlefield(P0)
                .iter()
                .filter(|o| o.card == CardId::BearToken)
                .count();
            assert_eq!(bears, 2);
        }
        if has(3) {
            assert_eq!(
                g.state
                    .object(bear)
                    .counters
                    .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
                2
            );
        }
        assert!(g.state.pending.is_none());
        assert!(g.state.stack.is_empty());
    }
}

#[test]
fn modal_refusals_and_postpick_search_save_continue_identically() {
    let mut g = fixture(4511)
        .hand(P0, &[c("Titania's Command")])
        .battlefield(P0, &[CardId::Forest; 6])
        .library(P0, &[CardId::Forest; 4])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let spell = hand_card(&g, P0, c("Titania's Command"));
    let before = g.state.clone();
    assert!(
        g.submit_action(
            P0,
            Action::CastSpellWithModes {
                card: spell,
                modes: [0, 0],
                targets: vec![Target::Player(P1)]
            }
        )
        .is_err()
    );
    assert_eq!(g.state, before);
    assert!(
        g.submit_action(
            P0,
            Action::CastSpellWithModes {
                card: spell,
                modes: [0, 1],
                targets: Vec::new()
            }
        )
        .is_err()
    );
    assert_eq!(g.state, before);
    assert!(
        g.submit_action(
            P0,
            Action::CastSpellWithModes {
                card: spell,
                modes: [1, 2],
                targets: vec![Target::Player(P1)]
            }
        )
        .is_err()
    );
    assert_eq!(g.state, before);
    assert!(
        g.submit_action(
            P0,
            Action::CastSpell {
                card: spell,
                targets: Vec::new()
            }
        )
        .is_err()
    );
    assert_eq!(g.state, before);
    let a=g.legal_actions().into_iter().find(|a|matches!(&a.action,Action::CastSpellWithModes{card,modes,..} if *card==spell && *modes==[1,2])).expect("search+bears offered").action;
    submit_ok(&mut g, P0, a);
    for _ in 0..32 {
        if let Some(Pending::SearchLibrary { .. }) = g.state.pending {
            let pick = g
                .legal_actions()
                .into_iter()
                .find(|a| matches!(a.action, Action::SearchLibraryPick { .. }))
                .unwrap()
                .action;
            submit_ok(&mut g, P0, pick);
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(matches!(
        g.state.pending,
        Some(Pending::SearchLibrary { remaining: 1, .. })
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
    let loaded = SessionSave::from_json(&save.to_json()).expect("modal postpick save loads");
    let mut loaded = Game::from_state(loaded.game).unwrap();
    settle(&mut g);
    settle(&mut loaded);
    assert_eq!(loaded.state, g.state);
    assert_eq!(loaded.digest(), g.digest());
    assert_eq!(
        g.state
            .battlefield(P0)
            .iter()
            .filter(|o| o.card == CardId::BearToken)
            .count(),
        2
    );
    assert_eq!(
        g.state
            .battlefield(P0)
            .iter()
            .filter(|o| o.card.def().is_land())
            .count(),
        8
    );
}
