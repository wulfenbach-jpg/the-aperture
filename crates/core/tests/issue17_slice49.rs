//! Slice 49: review329's private-look, attack-event and saved-instruction repairs.
//! The first five tests retain the reviewer's discriminating cases. The land
//! departure is a controlled valid fixture mutation, not a land-removal spell.
mod common;
use aperture_core::state::LibrarySearchMode;
use aperture_core::*;
use common::*;

fn cub_pending(cards: &[CardId]) -> Game {
    let mut g = fixture(32901)
        .battlefield(P0, &[CardId::OwlbearCub])
        .battlefield(P1, &[CardId::Forest; 8])
        .library(P0, cards)
        .library(P1, &[CardId::Forest; 5])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let owl = battlefield_card(&g, P0, CardId::OwlbearCub);
    g.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![owl],
            defender: Some(P1),
        },
    );
    settle_to_pending(&mut g);
    g
}
fn settle_to_pending(g: &mut Game) {
    for _ in 0..24 {
        if g.state.pending.is_some() || g.state.stack.is_empty() {
            return;
        }
        let p = g.decision_owner().unwrap();
        submit_ok(g, p, Action::Pass);
    }
    panic!("did not settle");
}
fn done(g: &mut Game) {
    let p = g.decision_owner().unwrap();
    submit_ok(g, p, Action::SearchLibraryDone);
}
fn save(g: &Game) -> SessionSave {
    // Fixture-state round trips, not a claim that these dummy decks replay the fixture.
    let decks = vec![DeckList::parse_practice("fixture", "40 Forest\n").unwrap(); 2];
    let replay = Replay::new(
        g.state.seed,
        g.state.first_player,
        decks,
        AiSettings::default(),
    )
    .unwrap();
    SessionSave::capture(g, replay, vec![], true)
}
fn loads(g: &Game) -> bool {
    SessionSave::from_json(&save(g).to_json()).is_ok()
}
fn round_trip(g: &Game) -> Game {
    let loaded = SessionSave::from_json(&save(g).to_json()).unwrap();
    Game { state: loaded.game }
}
fn pending_titania() -> Game {
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
    settle_to_pending(&mut g);
    assert!(matches!(
        g.state.pending,
        Some(Pending::SearchLibrary { .. })
    ));
    g
}
#[test]
fn review329_owner_must_see_every_looked_card() {
    let cards = [
        CardId::GrizzlyBears,
        CardId::Forest,
        CardId::Island,
        CardId::Mountain,
        CardId::Plains,
        CardId::Forest,
        CardId::Island,
        CardId::Mountain,
        CardId::OrdinaryBear,
    ];
    let g = cub_pending(&cards);
    let v = g.view_for(P0, &[]);
    assert_eq!(
        v.pending_viewed.iter().map(|c| c.card).collect::<Vec<_>>(),
        cards[..8]
    );
}
#[test]
fn review329_no_creatures_still_requires_private_look() {
    let mut g = cub_pending(&[CardId::Forest; 9]);
    assert!(matches!(
        g.state.pending,
        Some(Pending::SearchLibrary { .. })
    ));
    assert_eq!(g.view_for(P0, &[]).pending_viewed.len(), 8);
    assert_eq!(g.legal_actions().len(), 1);
    assert_eq!(g.legal_actions()[0].action, Action::SearchLibraryDone);
    assert!(g.state.validate().is_ok());
    let before = g.state.player(P0).library.clone();
    let mut loaded = round_trip(&g);
    let state = g.state.clone();
    submit_err(&mut g, P0, Action::SearchLibraryPick { card: before[0] });
    assert_eq!(g.state, state, "ineligible take must refuse atomically");
    done(&mut g);
    done(&mut loaded);
    assert_eq!(g.state, loaded.state);
    assert_eq!(g.state.player(P0).library[0], before[8]);
}
#[test]
fn review329_attack_restriction_is_not_intervening_if() {
    let mut g = fixture(32902)
        .battlefield(P0, &[CardId::OwlbearCub])
        .battlefield(P1, &[CardId::Forest; 8])
        .library(P0, &[CardId::GrizzlyBears; 9])
        .library(P1, &[CardId::Forest; 5])
        .step_no_priority(P0, Step::DeclareAttackers)
        .build();
    let owl = battlefield_card(&g, P0, CardId::OwlbearCub);
    g.state.pending = Some(Pending::DeclareAttackers { player: P0 });
    submit_ok(
        &mut g,
        P0,
        Action::DeclareAttackers {
            attackers: vec![owl],
            defender: Some(P1),
        },
    );
    assert_eq!(g.state.stack.len(), 1);
    // Controlled valid-state response result, NOT a legal land-removal cast.
    let land = battlefield_card(&g, P1, CardId::Forest);
    g.state.objects.get_mut(&land).unwrap().zone = Zone::Graveyard;
    g.state.players[1].graveyard.push(land);
    assert!(g.state.validate().is_ok());
    settle_to_pending(&mut g);
    assert!(
        matches!(g.state.pending, Some(Pending::SearchLibrary { .. })),
        "an already-qualified attack must still look after the eighth land leaves"
    );
}
#[test]
fn review329_look_pending_cannot_replace_titania_search_on_load() {
    let mut g = pending_titania();
    assert!(g.state.validate().is_ok());
    assert!(loads(&g));
    let library = g.state.player(P0).library.clone();
    if let Some(Pending::SearchLibrary {
        mode,
        remaining,
        chosen,
        eligible,
        looked,
        ..
    }) = &mut g.state.pending
    {
        *mode = LibrarySearchMode::CreatureFromTopToBattlefieldAttacking;
        *remaining = 1;
        chosen.clear();
        eligible.clear();
        *looked = library;
    }
    assert!(g.state.validate().is_err());
    assert!(!loads(&g));
}
#[test]
fn review329_post_pick_valid_continuation_control() {
    let mut g = cub_pending(&[CardId::GrizzlyBears; 9]);
    let card = g.state.player(P0).library[0];
    assert!(
        g.legal_actions()
            .iter()
            .any(|a| a.action == Action::SearchLibraryPick { card })
    );
    submit_ok(&mut g, P0, Action::SearchLibraryPick { card });
    assert!(g.state.validate().is_ok());
    let mut loaded = round_trip(&g);
    done(&mut g);
    done(&mut loaded);
    assert_eq!(g.state, loaded.state);
}
#[test]
fn entire_opponent_view_and_digest_ignore_hidden_eligibility_and_pick() {
    let mut mixed = cub_pending(&[CardId::GrizzlyBears; 9]);
    let ineligible = cub_pending(&[CardId::Forest; 9]);
    assert_eq!(mixed.view_for(P1, &[]), ineligible.view_for(P1, &[]));
    assert_eq!(
        mixed.view_for(P1, &[]).digest,
        ineligible.view_for(P1, &[]).digest
    );
    let before = mixed.view_for(P1, &[]);
    let card = mixed.state.player(P0).library[0];
    submit_ok(&mut mixed, P0, Action::SearchLibraryPick { card });
    assert_eq!(mixed.view_for(P1, &[]), before);
}
#[test]
fn forged_look_states_are_refused_by_validation_and_real_load() {
    let original = cub_pending(&[CardId::GrizzlyBears; 9]);
    assert!(original.state.validate().is_ok());
    assert!(loads(&original));
    for case in 0..12 {
        let mut g = original.clone();
        match case {
            0 => g.state.stack.last_mut().unwrap().cursor = 0,
            1 => g.state.stack.last_mut().unwrap().cursor = 2,
            2 => g.state.stack.last_mut().unwrap().controller = P1,
            3 => g.state.stack.last_mut().unwrap().card = CardId::GrizzlyBears,
            4 => {
                g.state.stack.last_mut().unwrap().entry =
                    aperture_core::state::StackEntry::Triggered { ability: 1 }
            }
            5 => g.state.resolving = Some(g.state.player(P0).library[0]),
            _ => {
                if let Some(Pending::SearchLibrary {
                    looked,
                    eligible,
                    remaining,
                    mode,
                    ..
                }) = &mut g.state.pending
                {
                    match case {
                        6 => {
                            looked.pop();
                        }
                        7 => looked.swap(0, 1),
                        8 => looked[1] = looked[0],
                        9 => eligible.push(eligible[0]),
                        10 => *remaining = 2,
                        11 => *mode = LibrarySearchMode::AnyLandToBattlefield,
                        _ => unreachable!(),
                    }
                } else {
                    panic!("missing look");
                }
            }
        }
        assert!(g.state.validate().is_err(), "forged case {case} validates");
        assert!(!loads(&g), "forged case {case} loads");
    }
}
#[test]
fn short_and_empty_library_looks_round_trip_before_acknowledgement() {
    for cards in [&[][..], &[CardId::Forest; 3][..]] {
        let mut g = cub_pending(cards);
        assert_eq!(g.view_for(P0, &[]).pending_viewed.len(), cards.len());
        assert!(g.state.validate().is_ok());
        let mut loaded = round_trip(&g);
        done(&mut g);
        done(&mut loaded);
        assert_eq!(g.state, loaded.state);
    }
}
