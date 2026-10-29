//! Slice 69: Moggcatcher's "{3}, {T}: Search your library for a Goblin
//! permanent card, put it onto the battlefield, then shuffle."
//!
//! The card is fetched from its live printing (NEM 96, retrieved 2026-10-29)
//! and the record lives in `fixtures/starter-corpus/slice69-sourced.json`.
//! The behaviour under test is the search itself: a real whole-library search
//! (CR 701.23) whose eligible set is exactly the Goblin *permanent* cards
//! (CR 110.4a), the found card entering through the ordinary entry path so its
//! entry triggers fire (CR 603.6a), and nothing revealed into hand.
mod common;
use aperture_core::*;
use common::*;

fn moggcatcher_action(source: ObjectId) -> Action {
    Action::ActivateAbility {
        source,
        ability: 0,
        mode: 0,
        targets: Vec::new(),
    }
}

/// Pass priority until the activated ability resolves into its search. The
/// ability is put on the stack by the activation, so the search exists only
/// after both seats pass (CR 602.2a).
fn pass_to_search(g: &mut Game) -> (Vec<ObjectId>, LibrarySearchMode) {
    for _ in 0..20 {
        if let Some(Pending::SearchLibrary { eligible, mode, .. }) = &g.state.pending {
            return (eligible.clone(), *mode);
        }
        let p = g.decision_owner().expect("owner before the search");
        submit_ok(g, p, Action::Pass);
    }
    panic!("the search never resolved");
}

fn drain_all(g: &mut Game, events: &mut Vec<Event>) {
    for _ in 0..100 {
        if g.state.stack.is_empty()
            && g.state.trigger_queue.is_empty()
            && g.state.pending.is_none()
            && g.state.resolving.is_none()
        {
            return;
        }
        let p = g.decision_owner().expect("owner during drain");
        let action = g
            .legal_actions()
            .into_iter()
            .map(|x| x.action)
            .find(|a| {
                matches!(
                    a,
                    Action::ChooseTriggerTarget {
                        target: Target::Player(P1)
                    }
                )
            })
            .unwrap_or(Action::Pass);
        assert!(g.legal_actions().iter().any(|x| x.action == action));
        events.extend(g.submit_action(p, action).unwrap());
        g.state.validate().unwrap();
    }
    panic!("drain cap");
}

#[test]
fn moggcatcher_searches_a_goblin_permanent_onto_the_battlefield() {
    let mut g = fixture(6901)
        .battlefield(
            P0,
            &[
                CardId::Moggcatcher,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
            ],
        )
        .library(
            P0,
            &[
                CardId::GrizzlyBears,
                CardId::SiegeGangCommander,
                CardId::Mountain,
                CardId::GrizzlyBears,
            ],
        )
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&g, P0, CardId::Moggcatcher);
    let action = moggcatcher_action(source);
    assert!(
        g.legal_actions().iter().any(|x| x.action == action),
        "the printed {{3}}, {{T}} activation is offered"
    );
    let library_before = g.state.player(P0).library.len();
    submit_ok(&mut g, P0, action);
    assert!(
        g.state.objects[&source].tapped,
        "{{T}} is part of the cost and the source is tapped"
    );
    assert_eq!(
        g.state.player(P0).mana.total(),
        0,
        "the {{3}} was paid, not left in the pool"
    );
    let (eligible, mode) = pass_to_search(&mut g);
    match &g.state.pending {
        Some(Pending::SearchLibrary {
            player, remaining, ..
        }) => {
            assert_eq!(*player, P0);
            assert_eq!(*remaining, 1);
        }
        other => panic!("expected a library search, got {other:?}"),
    }
    assert_eq!(
        mode,
        aperture_core::LibrarySearchMode::GoblinPermanentToBattlefield
    );
    let eligible_names = names(&g, &eligible);
    assert_eq!(
        eligible_names,
        vec!["Siege-Gang Commander"],
        "only the Goblin permanent card in the library is eligible"
    );

    // A forged pick outside the offered set is refused without mutating state.
    let bears = g
        .state
        .player(P0)
        .library
        .iter()
        .copied()
        .find(|id| g.state.object(*id).card == CardId::GrizzlyBears)
        .expect("a Bears in the library");
    let revision = g.state.revision;
    let digest = aperture_core::digest::digest(&g.state);
    let error = submit_err(&mut g, P0, Action::SearchLibraryPick { card: bears });
    assert!(
        matches!(error, EngineError::Illegal { .. }),
        "a non-Goblin pick is refused"
    );
    assert_eq!(
        g.state.revision, revision,
        "refusal leaves the revision alone"
    );
    assert_eq!(
        aperture_core::digest::digest(&g.state),
        digest,
        "refusal leaves the state untouched"
    );

    let seize = eligible[0];
    let events = submit_ok(&mut g, P0, Action::SearchLibraryPick { card: seize });
    // Finishing the search moves the chosen card; the search itself suspends on
    // the pick, so finish it and then drain the stack (the entry trigger).
    submit_ok(&mut g, P0, Action::SearchLibraryDone);
    let mut log = events;
    drain_all(&mut g, &mut log);

    assert!(
        g.state
            .battlefield(P0)
            .iter()
            .any(|o| o.card == CardId::SiegeGangCommander),
        "the searched card entered the battlefield"
    );
    let tokens = g
        .state
        .battlefield(P0)
        .into_iter()
        .filter(|o| o.card == CardId::GoblinToken)
        .count();
    assert_eq!(
        tokens, 3,
        "Siege-Gang Commander's entry trigger fired (CR 603.6a)"
    );
    assert!(
        !g.state
            .player(P0)
            .hand
            .iter()
            .any(|id| { g.state.object(*id).card == CardId::SiegeGangCommander }),
        "the searched card was not put into hand"
    );
    assert_eq!(
        g.state.player(P0).library.len(),
        library_before - 1,
        "exactly one card left the library"
    );
    assert!(g.state.pending.is_none());
    g.state
        .validate()
        .expect("the finished search state is valid");
}

#[test]
fn moggcatcher_is_not_offered_when_tapped_sick_or_unpaid() {
    // Tapped: the {T} cost cannot be paid.
    let mut tapped = fixture(6902)
        .battlefield(
            P0,
            &[
                CardId::Moggcatcher,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
            ],
        )
        .library(P0, &[CardId::GrizzlyBears, CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&tapped, P0, CardId::Moggcatcher);
    object_mut(&mut tapped.state, source).tapped = true;
    assert!(
        !tapped
            .legal_actions()
            .iter()
            .any(|x| x.action == moggcatcher_action(source)),
        "a tapped Moggcatcher is not offered"
    );

    // Summoning sickness: a creature that arrived this turn cannot pay {T}.
    let sick = fixture(6903)
        .battlefield_sick(P0, &[CardId::Moggcatcher])
        .battlefield(P0, &[CardId::Mountain, CardId::Mountain, CardId::Mountain])
        .library(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&sick, P0, CardId::Moggcatcher);
    assert!(
        !sick
            .legal_actions()
            .iter()
            .any(|x| x.action == moggcatcher_action(source)),
        "a summoning-sick Moggcatcher is not offered"
    );

    // No mana: only two Mountains, so {3} cannot be paid.
    let poor = fixture(6904)
        .battlefield(
            P0,
            &[CardId::Moggcatcher, CardId::Mountain, CardId::Mountain],
        )
        .library(P0, &[CardId::GrizzlyBears])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&poor, P0, CardId::Moggcatcher);
    assert!(
        !poor
            .legal_actions()
            .iter()
            .any(|x| x.action == moggcatcher_action(source)),
        "an unpayable activation is not offered"
    );
}

#[test]
fn moggcatcher_pending_search_saves_and_continues_identically() {
    let mut g = fixture(6905)
        .battlefield(
            P0,
            &[
                CardId::Moggcatcher,
                CardId::Mountain,
                CardId::Mountain,
                CardId::Mountain,
            ],
        )
        .library(
            P0,
            &[
                CardId::SiegeGangCommander,
                CardId::Mountain,
                CardId::GrizzlyBears,
            ],
        )
        .library(P1, &[CardId::Forest; 10])
        .main(P0, Step::PrecombatMain)
        .build();
    let source = battlefield_card(&g, P0, CardId::Moggcatcher);
    submit_ok(&mut g, P0, moggcatcher_action(source));
    let (eligible, _mode) = pass_to_search(&mut g);
    assert_eq!(eligible.len(), 1, "one Goblin permanent card is offered");
    g.state.validate().expect("pending search is valid");

    // Real capture -> JSON -> load through the session save path.
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        vec![deck_a(), deck_b()],
        AiSettings::default(),
    )
    .unwrap();
    let save = SessionSave::capture(&g, replay, Vec::new(), true);
    let json = save.to_json();
    let mut reloaded = SessionSave::from_json(&json)
        .expect("the pending search loads")
        .game();

    // Same answers on both states; the digests must agree afterwards.
    for game in [&mut g, &mut reloaded] {
        let pick = match &game.state.pending {
            Some(Pending::SearchLibrary { eligible, .. }) => eligible[0],
            other => panic!("expected a search, got {other:?}"),
        };
        submit_ok(game, P0, Action::SearchLibraryPick { card: pick });
        submit_ok(game, P0, Action::SearchLibraryDone);
        let mut log = Vec::new();
        drain_all(game, &mut log);
    }
    assert_eq!(
        aperture_core::digest::digest(&g.state),
        aperture_core::digest::digest(&reloaded.state),
        "the reloaded search continues to the same state"
    );
    assert_eq!(
        g.state
            .battlefield(P0)
            .iter()
            .filter(|o| o.card == CardId::GoblinToken)
            .count(),
        3,
        "the reloaded continuation also resolves the entry trigger"
    );
}
