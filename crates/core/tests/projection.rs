//! Perspective projection: hidden zones must not leak, and the log must be
//! masked per viewer. CR 401.2, 402.1, 701.22a.

mod common;

use std::collections::BTreeSet;

use aperture_core::state::StackItem;
use aperture_core::*;
use common::*;

fn all_view_ids(view: &PlayerView) -> BTreeSet<u32> {
    let mut ids = BTreeSet::new();
    for cards in [
        &view.own_hand,
        &view.own_battlefield,
        &view.own_graveyard,
        &view.own_exile,
        &view.pending_viewed,
    ] {
        for card in cards {
            ids.insert(card.id.0);
        }
    }
    for opponent in &view.opponents {
        for cards in [&opponent.battlefield, &opponent.graveyard, &opponent.exile] {
            for card in cards {
                ids.insert(card.id.0);
            }
        }
    }
    for item in &view.stack {
        ids.insert(item.object.0);
    }
    ids
}

#[test]
fn libraries_and_the_opponent_hand_are_not_projected() {
    let mut game = Game::new(7, &[deck_a(), deck_b()]).expect("game");
    let mut log = Vec::new();
    start_game(&mut game, &mut log);
    let view = game.view_for(P0, &log);
    let json = view.to_json();

    let opponent = view.opponents.first().expect("one opponent");
    assert_eq!(view.own_library_count, 33);
    assert_eq!(opponent.library_count, 33);
    assert_eq!(opponent.hand_count, 7);
    assert_eq!(view.own_hand.len(), 7);

    let visible = all_view_ids(&view);
    let mut hidden: Vec<ObjectId> = Vec::new();
    hidden.extend(game.state.player(P1).hand.iter().copied());
    hidden.extend(game.state.player(P0).library.iter().copied());
    hidden.extend(game.state.player(P1).library.iter().copied());
    for id in &hidden {
        assert!(
            !visible.contains(&id.0),
            "hidden object {} appears in P0's view",
            game.state.object(*id).card.name()
        );
        // Exact JSON token match: `"id":4,` must not be confused with `"id":42,`.
        let token = format!("\"id\":{}", id.0);
        let leaked = json.match_indices(&token).any(|(at, _)| {
            json[at + token.len()..]
                .chars()
                .next()
                .is_none_or(|c| !c.is_ascii_digit())
        });
        assert!(!leaked, "hidden object id {} leaked into the JSON", id.0);
    }
    assert_eq!(hidden.len(), 7 + 33 + 33);
}

#[test]
fn a_scry_owners_view_shows_the_looked_at_card_and_the_other_seat_does_not() {
    let mut game = fixture(60)
        .hand(P0, &[CardId::Opt])
        .battlefield(P0, &[CardId::Island])
        .library(P0, &[CardId::Forest, CardId::Shock])
        .main(P0, Step::PrecombatMain)
        .build();
    let opt = hand_card(&game, P0, CardId::Opt);
    submit_ok(
        &mut game,
        P0,
        Action::CastSpell {
            card: opt,
            targets: vec![],
        },
    );
    submit_ok(&mut game, P0, Action::Pass);
    submit_ok(&mut game, P1, Action::Pass);

    let owner = game.view_for(P0, &[]);
    assert_eq!(owner.pending_viewed.len(), 1);
    assert_eq!(owner.pending_viewed[0].name, "Forest");
    assert!(owner.pending.is_some());

    let other = game.view_for(P1, &[]);
    assert!(other.pending_viewed.is_empty());
    assert!(
        other.pending.as_deref().is_some_and(|p| p.contains("P0")),
        "the other seat sees that a choice is pending, not its contents"
    );
    assert!(!other.to_json().contains("Forest"));
}

#[test]
fn log_rendering_masks_the_opponents_draws() {
    let game = fixture(61)
        .library(P1, &[CardId::Forest])
        .main(P0, Step::PrecombatMain)
        .build();
    let hidden = game.state.player(P1).library[0];
    let hidden_card = game.state.object(hidden).card;
    let log = vec![Event::Draw {
        player: P1,
        object: hidden,
        card: hidden_card,
    }];
    let mine = game.view_for(P0, &log);
    assert_eq!(mine.log, vec!["P1 draws a card".to_string()]);
    let theirs = game.view_for(P1, &log);
    assert_eq!(theirs.log, vec!["P1 draws Forest".to_string()]);
}

#[test]
fn projection_carries_the_decision_identity_and_legal_actions() {
    let game = fixture(62)
        .hand(P0, &[CardId::Shock])
        .battlefield(P0, &[CardId::Mountain])
        .main(P0, Step::PrecombatMain)
        .build();
    let view = game.view_for(P0, &[]);
    assert!(!view.legal.is_empty());
    assert!(view.legal.iter().any(|a| matches!(a.action, Action::Pass)));
    assert!(
        view.legal
            .iter()
            .any(|a| matches!(&a.action, Action::CastSpell { .. }))
    );
    assert_eq!(view.rules_revision, RULES_REVISION);
    assert!(!view.digest.is_empty());
    // The opponent's view of the same moment offers no actions for P0's seat.
    let other = game.view_for(P1, &[]);
    assert!(
        other.legal.is_empty(),
        "P1 has no priority, so P1's projection has no legal actions"
    );
    let _ = StackItem {
        object: ObjectId(1),
        card: CardId::Shock,
        controller: P0,
        targets: Vec::new(),
        affected: None,
        source_pt_lki: None,
        search_amount: 0,
        cursor: 0,
        entry: StackEntry::Spell,
        trigger_answered: false,
        spell_mana_value: 0,
    };
}
