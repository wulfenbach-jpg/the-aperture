//! Native slice45 persistence and full-instruction controls.
// Native tests include assertion REDs for missing save validation/public modal information.
mod common;
use aperture_core::*;
use common::*;

fn pending_search() -> Game {
    let mut g = fixture(4591)
        .hand(P0, &[CardId::TitaniasCommand])
        .battlefield(P0, &[CardId::Forest; 6])
        .battlefield(P0, &[CardId::JaddiOffshoot])
        .library(P0, &[CardId::Forest; 5])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let card = hand_card(&g, P0, CardId::TitaniasCommand);
    submit_ok(
        &mut g,
        P0,
        Action::CastSpellWithModes {
            card,
            modes: [1, 2],
            targets: vec![],
        },
    );
    for _ in 0..20 {
        if matches!(g.state.pending, Some(Pending::SearchLibrary { .. })) {
            return g;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    panic!("search not reached");
}
fn loads(g: &Game) -> bool {
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
    let save = SessionSave::capture(g, replay, vec![], true);
    SessionSave::from_json(&save.to_json()).is_ok()
}
fn refused(g: &Game) {
    let invalid = g.state.validate().is_err();
    let load_refused = !loads(g);
    assert!(
        invalid && load_refused,
        "forged modal search must fail validation AND real load: invalid={invalid}, load_refused={load_refused}"
    );
}
#[test]
fn search_capacity_is_exactly_two_in_validation_and_load() {
    let mut g = pending_search();
    assert!(g.state.validate().is_ok());
    assert!(loads(&g));
    if let Some(Pending::SearchLibrary { remaining, .. }) = &mut g.state.pending {
        *remaining = 3;
    }
    refused(&g);
}
#[test]
fn duplicate_and_unknown_saved_modes_are_refused() {
    for modes in [vec![1, 1], vec![1, 4], vec![], vec![0, 1, 2]] {
        let mut g = pending_search();
        g.state.stack.last_mut().unwrap().modes = modes;
        refused(&g);
    }
}
#[test]
fn any_land_pending_is_bound_to_selected_search_instruction() {
    let mut g = pending_search();
    g.state.stack.last_mut().unwrap().modes = vec![2, 3];
    refused(&g);
}
#[test]
fn search_pending_filter_cannot_be_swapped_on_save() {
    let mut g = pending_search();
    if let Some(Pending::SearchLibrary { mode, .. }) = &mut g.state.pending {
        *mode = aperture_core::state::LibrarySearchMode::BasicLandToBattlefield;
    }
    refused(&g);
}
#[test]
fn bears_created_by_earlier_mode_receive_counters_later_in_printed_order() {
    let mut g = fixture(4592)
        .hand(P0, &[CardId::TitaniasCommand])
        .battlefield(P0, &[CardId::Forest; 6])
        .battlefield(P0, &[CardId::GrizzlyBears])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .library(P0, &[CardId::Forest; 8])
        .library(P1, &[CardId::Forest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let old = battlefield_card(&g, P0, CardId::GrizzlyBears);
    g.state
        .objects
        .get_mut(&old)
        .unwrap()
        .counters
        .add(aperture_core::counters::CounterKind::PlusOnePlusOne, 1);
    let card = hand_card(&g, P0, CardId::TitaniasCommand);
    submit_ok(
        &mut g,
        P0,
        Action::CastSpellWithModes {
            card,
            modes: [3, 2],
            targets: vec![],
        },
    );
    for _ in 0..30 {
        if g.state.stack.is_empty() && g.state.pending.is_none() {
            break;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(&mut g, p, Action::Pass);
    }
    assert!(g.state.stack.is_empty());
    let bears: Vec<_> = g
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == CardId::BearToken)
        .collect();
    assert_eq!(bears.len(), 2);
    for b in bears {
        assert_eq!(
            b.counters
                .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
            2
        );
        assert_eq!((b.power(), b.toughness()), (4, 4));
    }
    assert_eq!(
        g.state
            .object(old)
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        3
    );
    assert_eq!(
        g.state
            .object(battlefield_card(&g, P1, CardId::GrizzlyBears))
            .counters
            .get(aperture_core::counters::CounterKind::PlusOnePlusOne),
        0
    );
}

fn drain(g: &mut Game) {
    for _ in 0..100 {
        if g.state.stack.is_empty()
            && g.state.trigger_queue.is_empty()
            && g.state.resolving.is_none()
            && g.state.pending.is_none()
        {
            return;
        }
        let p = g.decision_owner().expect("offered decision while draining");
        submit_ok(g, p, Action::Pass);
    }
    panic!("resolution did not fully drain");
}
#[test]
fn searched_lands_enter_tapped_and_fire_landfall_once_each() {
    let mut g = pending_search();
    let life = g.state.player(P0).life;
    let played = g.state.turn.lands_played;
    let mut picked = Vec::new();
    for _ in 0..2 {
        let action = g
            .legal_actions()
            .into_iter()
            .find(|a| matches!(a.action, Action::SearchLibraryPick { .. }))
            .unwrap()
            .action;
        if let Action::SearchLibraryPick { card } = action {
            picked.push(card);
            submit_ok(&mut g, P0, Action::SearchLibraryPick { card });
        }
    }
    submit_ok(&mut g, P0, Action::SearchLibraryDone);
    drain(&mut g);
    assert_eq!(picked.len(), 2);
    for id in picked {
        let o = g.state.object(id);
        assert_eq!(o.zone, Zone::Battlefield);
        assert!(o.tapped);
    }
    assert_eq!(g.state.player(P0).life, life + 2);
    assert_eq!(g.state.turn.lands_played, played);
}
#[test]
fn private_candidates_do_not_change_opponent_view_and_zero_pick_still_shuffles() {
    let mut g = pending_search();
    let mut hidden = g.clone();
    let id = hidden.state.player(P0).library[0];
    hidden.state.objects.get_mut(&id).unwrap().card = CardId::GrizzlyBears;
    if let Some(Pending::SearchLibrary { eligible, .. }) = &mut hidden.state.pending {
        eligible.retain(|x| *x != id);
    }
    assert_eq!(g.view_for(P1, &[]), hidden.view_for(P1, &[]));
    let initial_battlefield = g.state.battlefield(P0).len();
    let initial_library = g.state.player(P0).library.clone();
    let events = g.submit_action(P0, Action::SearchLibraryDone).unwrap();
    assert!(events.iter().any(|e|matches!(e,Event::StateBasedAction {description} if description.contains("shuffles their library after searching"))));
    assert_ne!(
        g.state.player(P0).library,
        initial_library,
        "zero picks really advances RNG shuffle"
    );
    drain(&mut g);
    assert_eq!(
        g.state.battlefield(P0).len(),
        initial_battlefield + 2,
        "only later Bear mode creates permanents"
    );
    assert!(g.state.validate().is_ok());
}
