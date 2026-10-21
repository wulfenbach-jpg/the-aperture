//! Regression: the 40-card practice loader must refuse Commander-shaped text
//! lists instead of silently flattening every `*:` header away.
//!
//! The bug: `DeckList::parse_practice` skipped *any* line ending in `:`, so a
//! file with `Commander:` followed by supported cards was accepted and played
//! as a normal practice list. A Commander deck is never a practice deck.

use aperture_core::DeckList;

const DECK_A: &str = include_str!("../../../decks/temur-beats.txt");
const DECK_B: &str = include_str!("../../../decks/izzet-control.txt");

#[test]
fn commander_section_is_refused_by_the_practice_parser() {
    let text = "Commander:\n1 Grizzly Bears\nDeck:\n39 Forest\n";
    let err = DeckList::parse_practice("launder", text).expect_err(
        "a Commander section must be refused; flattening it into a practice list is illegal",
    );
    assert!(err.to_lowercase().contains("commander"), "{err}");
    assert!(err.contains("practice"), "{err}");
}

#[test]
fn companion_and_sideboard_sections_are_refused_by_the_practice_parser() {
    for header in ["Companion:", "Sideboard:", "Command Zone:", "Maybeboard:"] {
        let text = format!("{header}\n1 Grizzly Bears\n39 Forest\n");
        let err = DeckList::parse_practice("launder", &text)
            .unwrap_err_or_else(|| panic!("{header} must not be flattened"));
        assert!(
            err.to_lowercase()
                .contains(&header.trim_end_matches(':').to_lowercase()),
            "{err}"
        );
    }
}

trait UnwrapErrOrElse<T, E> {
    fn unwrap_err_or_else(self, f: impl FnOnce() -> E) -> E;
}

impl<T, E> UnwrapErrOrElse<T, E> for Result<T, E> {
    fn unwrap_err_or_else(self, f: impl FnOnce() -> E) -> E {
        match self {
            Ok(_) => f(),
            Err(err) => err,
        }
    }
}

#[test]
fn the_bundled_practice_lists_still_parse_at_exactly_forty_cards() {
    for (name, text) in [("Temur Beats", DECK_A), ("Izzet Control", DECK_B)] {
        let deck = DeckList::parse_practice(name, text)
            .unwrap_or_else(|e| panic!("{name} must keep parsing: {e}"));
        assert_eq!(deck.size(), 40, "{name}");
        assert!(deck.render().contains("Grizzly Bears") || deck.render().contains("Hill Giant"));
    }
}

#[test]
fn parenthesized_commander_or_sideboard_headers_are_refused() {
    for header in [
        "Commander (1):",
        "Sideboard (15):",
        "Deck (40):",
        "Mainboard (40):",
    ] {
        let text = format!("{header}\n1 Grizzly Bears\n39 Forest\n");
        let err = DeckList::parse_practice("launder", &text)
            .expect_err("unknown section headers must not be skipped");
        assert!(
            err.contains("section") || err.contains("header"),
            "{header}: {err}"
        );
    }
}

#[test]
fn the_acknowledged_deck_header_still_works() {
    let deck = DeckList::parse_practice("ok", "Deck:\n1 Grizzly Bears\n39 Forest\n")
        .expect("the practice Deck header is acknowledged");
    assert_eq!(deck.size(), 40);
}

#[test]
fn a_valid_forty_card_mainboard_prefixed_list_still_parses() {
    let deck = DeckList::parse_practice(
        "mainboard practice",
        "Mainboard:\n1 Grizzly Bears\n39 Forest\n",
    )
    .expect("the acknowledged Mainboard header must remain accepted");
    assert_eq!(deck.size(), 40);
}
