//! Native integration checks: the reused pending must respect its new snow mode.
//!
//! The validator and prompt regressions failed behaviorally on c94e82d before
//! the native owner's repair (slice44/native-red-search.log). Additional source
//! and land-cast controls below strengthen coverage; they are not claimed RED.
mod common;
use aperture_core::state::LibrarySearchMode;
use aperture_core::*;
use common::*;

fn snow_search() -> Game {
    let mut g = fixture(4411)
        .hand(P0, &[CardId::SpiritOfTheAldergard])
        .battlefield(P0, &[CardId::Forest; 5])
        .library(
            P0,
            &[
                CardId::Forest,
                CardId::SnowCoveredForest,
                CardId::SnowCoveredForest,
                CardId::GrizzlyBears,
            ],
        )
        .library(P1, &[CardId::SnowCoveredForest; 8])
        .main(P0, Step::PrecombatMain)
        .build();
    let a = g
        .legal_actions()
        .into_iter()
        .find(|a| matches!(a.action, Action::CastSpell { .. }))
        .unwrap()
        .action;
    submit_ok(&mut g, P0, a);
    for _ in 0..64 {
        if matches!(g.state.pending, Some(Pending::SearchLibrary { .. })) {
            return g;
        }
        let owner = g.decision_owner().unwrap();
        submit_ok(&mut g, owner, Action::Pass);
    }
    panic!("snow search not reached");
}

fn save(state: &Game) -> SessionSave {
    let decks = vec![
        DeckList::parse_practice("A", "40 Forest\n").unwrap(),
        DeckList::parse_practice("B", "40 Forest\n").unwrap(),
    ];
    let replay = Replay::new(
        state.state.seed,
        state.state.first_player,
        decks,
        AiSettings::default(),
    )
    .unwrap();
    SessionSave::capture(state, replay, Vec::new(), true)
}

#[test]
fn snow_search_validator_rejects_non_snow_basic_land_and_real_load_refuses_it() {
    let mut g = snow_search();
    assert!(g.state.validate().is_ok());
    let non_snow = *g
        .state
        .player(P0)
        .library
        .iter()
        .find(|id| g.state.object(**id).card == CardId::Forest)
        .unwrap();
    if let Some(Pending::SearchLibrary { eligible, mode, .. }) = &mut g.state.pending {
        assert_eq!(*mode, LibrarySearchMode::SnowLandToHand);
        eligible.push(non_snow);
    }
    assert!(
        g.state.validate().is_err(),
        "snow search must reject an eligible ordinary Forest, not silently reuse basic-land validation"
    );
    assert!(SessionSave::from_json(&save(&g).to_json()).is_err());
}

#[test]
fn snow_search_validator_rejects_two_picks_even_when_both_are_snow() {
    let mut g = snow_search();
    if let Some(Pending::SearchLibrary {
        eligible,
        chosen,
        remaining,
        ..
    }) = &mut g.state.pending
    {
        assert_eq!(eligible.len(), 2);
        *chosen = eligible.clone();
        *remaining = 0;
    }
    assert!(
        g.state.validate().is_err(),
        "the one-card snow search cannot serialize two chosen cards"
    );
    assert!(SessionSave::from_json(&save(&g).to_json()).is_err());
}

#[test]
fn snow_search_prompts_describe_reveal_to_hand_not_battlefield() {
    let g = snow_search();
    let description = g.state.pending.as_ref().unwrap().describe();
    assert!(
        description.contains("snow land") && description.contains("hand"),
        "{description}"
    );
    assert!(
        !description.contains("battlefield"),
        "snow search never puts the card on the battlefield"
    );
    for offered in g.legal_actions() {
        if matches!(offered.action, Action::SearchLibraryPick { .. }) {
            assert!(offered.label.contains("snow land"), "{}", offered.label);
        }
    }
}

#[test]
fn snow_search_direct_refusals_are_atomic_and_pick_remains_private_until_done() {
    let mut g = snow_search();
    let eligible = match &g.state.pending {
        Some(Pending::SearchLibrary { eligible, .. }) => eligible.clone(),
        _ => unreachable!(),
    };
    let non_snow = *g
        .state
        .player(P0)
        .library
        .iter()
        .find(|id| g.state.object(**id).card == CardId::Forest)
        .unwrap();
    let before = g.state.clone();
    assert!(
        g.submit_action(P0, Action::SearchLibraryPick { card: non_snow })
            .is_err()
    );
    assert_eq!(g.state, before);
    assert!(
        g.submit_action(P1, Action::SearchLibraryPick { card: eligible[0] })
            .is_err()
    );
    assert_eq!(g.state, before);
    let opponent_before = g.view_for(P1, &[]);
    submit_ok(&mut g, P0, Action::SearchLibraryPick { card: eligible[0] });
    assert!(g.view_for(P1, &[]).pending.is_none());
    assert_eq!(g.view_for(P1, &[]).own_hand, opponent_before.own_hand);
    let picked = g.state.clone();
    assert!(
        g.submit_action(P0, Action::SearchLibraryPick { card: eligible[0] })
            .is_err()
    );
    assert_eq!(g.state, picked);
    let events = submit_ok(&mut g, P0, Action::SearchLibraryDone);
    assert_eq!(g.state.object(eligible[0]).zone, Zone::Hand);
    assert!(
        events
            .iter()
            .any(|e| format!("{e:?}").contains("reveals Snow-Covered Forest")),
        "chosen card is publicly revealed"
    );
}
