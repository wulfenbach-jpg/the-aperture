//! Issue 17, slice 11: islandwalk, sourced from two still-missing Ayula bears.
//!
//! `Pale Bears` and `River Bear` print one keyword each and nothing else, so
//! they are the whole of this slice's card work: the printed text is
//! Islandwalk, and the new behaviour is the blocking restriction of CR 702.14b
//! applied from the *defending* player's battlefield. Every test below reads the
//! engine, not the handoff.

mod common;

use aperture_core::tokens::Keyword;
use aperture_core::*;
use common::*;

fn printed(name: &str) -> &'static aperture_core::card::CardDef {
    let id =
        catalog::by_name(name).unwrap_or_else(|| panic!("{name} is not in the supported pool"));
    id.def()
}

#[test]
fn pale_bears_and_river_bear_match_their_printed_text() {
    let pale = printed("Pale Bears");
    assert_eq!(pale.cost, Some("{2}{G}"));
    assert_eq!(pale.type_line, "Creature — Bear");
    assert_eq!((pale.power, pale.toughness), (Some(2), Some(2)));
    assert!(pale.is_creature());
    assert!(pale.has_subtype("Bear"));
    assert!(
        pale.oracle.contains("Islandwalk"),
        "the oracle text must keep the printed keyword: {}",
        pale.oracle
    );
    assert!(
        !pale.oracle.contains("Trample") && !pale.oracle.contains("Flash"),
        "neither bear prints another keyword: {}",
        pale.oracle
    );

    let river = printed("River Bear");
    assert_eq!(river.cost, Some("{3}{G}"));
    assert_eq!(river.type_line, "Creature — Bear");
    assert_eq!((river.power, river.toughness), (Some(3), Some(3)));
    assert!(river.is_creature());
    assert!(river.oracle.contains("Islandwalk"), "{}", river.oracle);

    // The keyword lives in the one table the blocking path reads, and the
    // `CardDef` flag the pool guard requires agrees with it.
    assert_eq!(
        aperture_core::tokens::keywords(CardId::PaleBears),
        &[Keyword::Islandwalk]
    );
    assert_eq!(
        aperture_core::tokens::keywords(CardId::RiverBear),
        &[Keyword::Islandwalk]
    );
    assert_eq!(Keyword::Islandwalk.name(), "islandwalk");
    assert!(pale.has_islandwalk(), "the pool guard reads this flag");
    assert!(river.has_islandwalk());

    // Islandwalk is a land subtype question: only an Island land counts, and
    // neither bear is one.
    assert!(!pale.is_island(), "a Bear is not an Island land");
    assert!(printed("Island").is_island());
    assert!(
        !printed("Forest").is_island(),
        "a Forest is not an Island (CR 305.6)"
    );
}

/// One fact, one source: a card prints Islandwalk exactly when it carries the
/// flag, exactly when the blocker check's table says so. Any of the three
/// drifting from the other two is a card the engine would mis-play.
#[test]
fn the_islandwalk_flag_and_the_blocking_table_agree_with_the_printed_text() {
    for card in catalog::all() {
        let printed_keyword = card.oracle.contains("Islandwalk");
        assert_eq!(
            card.has_islandwalk(),
            printed_keyword,
            "{} prints Islandwalk but the CardDef flag disagrees",
            card.name
        );
        assert_eq!(
            aperture_core::tokens::keywords(card.id).contains(&Keyword::Islandwalk),
            printed_keyword,
            "{} prints Islandwalk but the blocking table disagrees",
            card.name
        );
    }
}

/// The real defect this slice had to avoid: islandwalk must read the defending
/// player's board, so an Island anywhere else must not grant or deny anything.
#[test]
fn an_islandwalk_attacker_cannot_be_blocked_while_the_defender_controls_an_island() {
    let mut game = fixture(1111)
        .battlefield(P0, &[CardId::PaleBears])
        .battlefield(P1, &[CardId::GrizzlyBears, CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::PaleBears);
    let blocker = battlefield_card(&game, P1, CardId::GrizzlyBears);

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
            attackers: vec![bear],
            defender: Some(P1),
        },
    );
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::DeclareBlockers { .. })),
        64
    ));

    let before = game.state.revision;
    let err = submit_err(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker,
                attacker: bear,
            }],
        },
    );
    let message = err.to_string().to_lowercase();
    assert!(message.contains("islandwalk"), "{err}");
    assert!(message.contains("island"), "{err}");
    assert_eq!(
        game.state.revision, before,
        "a refused block declaration changes no state"
    );
    assert!(game.state.turn.blockers.is_empty());

    // The engine's own greedy blocking plan must not offer the illegal pair
    // either: it is published as a legal action, so it has to be one.
    let greedy = game
        .legal_actions()
        .into_iter()
        .find(|action| action.label.contains("Block with every available"));
    assert!(
        greedy.is_none(),
        "no blocker is legal, so no blocking plan may be offered"
    );
}

#[test]
fn an_islandwalk_attacker_is_blockable_when_the_defender_controls_no_island() {
    let mut game = fixture(1112)
        .battlefield(P0, &[CardId::RiverBear])
        .battlefield(P1, &[CardId::GrizzlyBears, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::RiverBear);
    let blocker = battlefield_card(&game, P1, CardId::GrizzlyBears);

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
            attackers: vec![bear],
            defender: Some(P1),
        },
    );
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::DeclareBlockers { .. })),
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker,
                attacker: bear,
            }],
        },
    );
    assert_eq!(game.state.turn.blockers.len(), 1, "the block is legal");

    // The greedy plan is offered again once the restriction is inert.
    let mut game = fixture(1112)
        .battlefield(P0, &[CardId::RiverBear])
        .battlefield(P1, &[CardId::GrizzlyBears, CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::RiverBear);
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
            attackers: vec![bear],
            defender: Some(P1),
        },
    );
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::DeclareBlockers { .. })),
        64
    ));
    assert!(
        game.legal_actions()
            .iter()
            .any(|action| action.label.contains("Block with every available")),
        "without an Island on the defending board the pair is legal"
    );
}

/// An opponent's Island is not the defending player's Island (CR 702.14b).
#[test]
fn another_players_island_never_makes_the_attacker_unblockable() {
    let mut game = fixture_n(1113, 3)
        .battlefield(P0, &[CardId::PaleBears])
        .battlefield(P1, &[CardId::GrizzlyBears])
        .battlefield(2, &[CardId::Island])
        .main(P0, Step::PrecombatMain)
        .build();
    let bear = battlefield_card(&game, P0, CardId::PaleBears);
    let blocker = battlefield_card(&game, P1, CardId::GrizzlyBears);

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
            attackers: vec![bear],
            defender: Some(P1),
        },
    );
    let mut log = Vec::new();
    assert!(run_until(
        &mut game,
        &mut log,
        |g| matches!(g.state.pending, Some(Pending::DeclareBlockers { .. })),
        64
    ));
    submit_ok(
        &mut game,
        P1,
        Action::DeclareBlockers {
            blocks: vec![BlockDecl {
                blocker,
                attacker: bear,
            }],
        },
    );
    assert_eq!(
        game.state.turn.blockers.len(),
        1,
        "P2's Island does not restrict P1's blocks"
    );
}

/// Combat still ends correctly, and the engine's own greedy blocking plan is
/// the thing that changes: with an Island on the defending board no block is
/// legal, so the islandwalker connects for its printed power; with the same
/// board minus that Island the greedy plan blocks and the damage is prevented.
/// The two runs differ only by the Island.
#[test]
fn the_greedy_blocking_plan_only_stops_an_islandwalker_without_an_island() {
    fn play(seed: u64, defender_cards: &[CardId]) -> (i32, bool) {
        let mut game = fixture(seed)
            .battlefield(P0, &[CardId::RiverBear])
            .battlefield(P1, defender_cards)
            .main(P0, Step::PrecombatMain)
            .build();
        let bear = battlefield_card(&game, P0, CardId::RiverBear);
        let life_before = game.state.players[P1 as usize].life;

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
                attackers: vec![bear],
                defender: Some(P1),
            },
        );
        let mut log = Vec::new();
        assert!(run_until(
            &mut game,
            &mut log,
            |g| matches!(g.state.pending, Some(Pending::DeclareBlockers { .. })),
            64
        ));

        let greedy = game
            .legal_actions()
            .into_iter()
            .find(|action| action.label.contains("Block with every available"))
            .map(|action| action.action);
        let offered = greedy.is_some();
        submit_ok(
            &mut game,
            P1,
            greedy.unwrap_or(Action::DeclareBlockers { blocks: Vec::new() }),
        );

        let mut log = Vec::new();
        assert!(run_until(
            &mut game,
            &mut log,
            |g| matches!(g.state.turn.step, Step::EndCombat | Step::PostcombatMain),
            256
        ));
        (game.state.players[P1 as usize].life - life_before, offered)
    }

    // With the Island the islandwalker is unblockable: no block is offered, and
    // the full printed power lands on the defending player (CR 702.14b, 510.1c).
    let (life_with_island, offered_with_island) =
        play(1114, &[CardId::GrizzlyBears, CardId::Island]);
    assert!(
        !offered_with_island,
        "no block plan may be offered while the defender controls an Island"
    );
    assert_eq!(
        life_with_island, -3,
        "the unblocked 3/3 islandwalker deals three"
    );

    // Same board, same seed, minus the Island: the greedy plan exists and the
    // 2/2 blocker can block, so nothing reaches the player.
    let (life_without_island, offered_without_island) = play(1114, &[CardId::GrizzlyBears]);
    assert!(
        offered_without_island,
        "the block is legal without an Island"
    );
    assert_eq!(
        life_without_island, 0,
        "a blocked islandwalker deals no damage to the player"
    );
}
