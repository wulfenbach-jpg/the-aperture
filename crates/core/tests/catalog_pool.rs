//! The pool contract: every card in the catalog is fully implemented, and the
//! bundled practice decks are exactly 40 supported cards.

mod common;

use aperture_core::*;
use common::*;

#[test]
fn pool_size_is_bounded_and_names_are_unique() {
    let pool = aperture_core::catalog::all();
    assert!(
        pool.len() >= 16,
        "the accepted 16-card practice corpus must remain supported, got {}",
        pool.len()
    );
    let mut names: Vec<&str> = pool.iter().map(|c| c.name).collect();
    names.sort_unstable();
    let unique = names.len();
    names.dedup();
    assert_eq!(names.len(), unique, "duplicate card names in the catalog");
    for card in pool {
        assert_eq!(
            aperture_core::catalog::by_name(card.name),
            Some(card.id),
            "{} does not round-trip by name",
            card.name
        );
    }
}

#[test]
fn every_printed_ability_maps_to_implemented_semantics() {
    for card in aperture_core::catalog::all() {
        if card.is_land() {
            assert!(
                card.basic_mana.is_some(),
                "{} is a land but has no mana ability",
                card.name
            );
            assert!(
                card.cost.is_none(),
                "{} should have no mana cost",
                card.name
            );
            assert!(card.spell.is_none());
            continue;
        }
        let spec = match card.spell {
            Some(spec) => spec,
            None => {
                // No spell definition means a vanilla permanent: it must have
                // no printed ability text at all.
                assert!(
                    card.oracle.trim().is_empty(),
                    "{} prints abilities but has no implementation",
                    card.name
                );
                assert!(card.is_creature(), "{} has no rules text at all", card.name);
                assert!(card.power.is_some() && card.toughness.is_some());
                continue;
            }
        };
        assert!(!spec.effects.is_empty(), "{} does nothing", card.name);
        assert!(card.cost.is_some(), "{} has no cost", card.name);
        if card.is_creature() {
            assert!(
                card.power.is_some() && card.toughness.is_some(),
                "{}",
                card.name
            );
            assert_eq!(
                spec.target,
                TargetSpec::None,
                "{} has no spell text",
                card.name
            );
        } else {
            assert!(card.power.is_none() && card.toughness.is_none());
        }
        for effect in spec.effects {
            match effect {
                Effect::Draw(n) | Effect::Scry(n) | Effect::Surveil(n) | Effect::Damage(n) => {
                    assert!(*n > 0, "{} has a no-op effect", card.name)
                }
                Effect::Pump(p, t) => assert!(*p != 0 || *t != 0),
                Effect::CounterTargetSpell => {
                    assert_eq!(spec.target, TargetSpec::Spell, "{}", card.name)
                }
                Effect::EachPlayerSacrifice => {
                    assert_eq!(spec.target, TargetSpec::None, "{}", card.name)
                }
                Effect::ReturnToHand | Effect::PutOnTopOfLibrary | Effect::ExileAndGainLife => {
                    assert_eq!(spec.target, TargetSpec::Creature, "{}", card.name)
                }
            }
        }
        // The card's own reminder text is consistent with its cost.
        if let Some(cost) = card.mana_cost() {
            assert!(cost.mana_value() >= 1, "{} costs nothing", card.name);
        }
    }
}

#[test]
fn bundled_practice_decks_are_exactly_forty_supported_cards() {
    for (name, text) in [("Temur Beats", DECK_A), ("Izzet Control", DECK_B)] {
        let deck = DeckList::parse_practice(name, text)
            .unwrap_or_else(|e| panic!("{name} must parse: {e}"));
        assert_eq!(deck.size(), 40, "{name} must be a 40-card practice deck");
        assert!(deck.lands() >= 16, "{name} needs enough lands to function");
        let creatures: u8 = deck
            .cards
            .iter()
            .filter(|(c, _)| c.def().is_creature())
            .map(|(_, n)| *n)
            .sum();
        assert!(creatures >= 4, "{name} needs creatures, got {creatures}");
        for (card, count) in &deck.cards {
            if !card.def().is_land() {
                assert!(
                    *count <= 4,
                    "{name}: {count} copies of {} breaks the 4-copy practice rule",
                    card.name()
                );
            }
        }
    }
}

#[test]
fn unsupported_card_names_are_rejected_by_the_deck_parser() {
    let err = DeckList::parse_practice("bad", "4 Llanowar Elves").unwrap_err();
    assert!(err.contains("Llanowar Elves"), "{err}");
    assert!(err.contains("not in the supported pool"), "{err}");
}

#[test]
fn game_setup_uses_twenty_life_and_forty_card_decks() {
    let game = Game::new(1, &[deck_a(), deck_b()]).expect("game");
    assert_eq!(game.state.player(P0).life, 20);
    assert_eq!(game.state.player(P1).life, 20);
    assert_eq!(game.state.player(P0).hand_size(), 7);
    assert_eq!(game.state.player(P0).library.len(), 33);
    assert_eq!(game.state.rules_revision, RULES_REVISION);
}
