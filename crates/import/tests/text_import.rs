//! Text-list fallback import: real counts, sections, and strict refusal.

use aperture_import::*;

const DECK_A: &str = include_str!("../../../decks/temur-beats.txt");

#[test]
fn a_supported_text_list_launches() {
    let (list, report) = import_text("Temur Beats", DECK_A).expect("bundled list imports");
    assert_eq!(list.size(), 40);
    assert!(report.launchable);
    assert_eq!(report.supported_cards, 40);
    assert_eq!(report.unsupported_cards, 0);
    assert_eq!(report.provider, "text");
    assert_eq!(report.total_cards, 40);
}

#[test]
fn sideboard_and_commander_sections_are_kept_out_of_the_deck() {
    let text = "\
Deck
20 Forest
20 Grizzly Bears
Sideboard
2 Shock
Commander
1 Centaur Courser
Maybeboard
1 Llanowar Elves
";
    let report = text_report("Sections", text).expect("sections analyze");
    assert_eq!(report.total_cards, 40);
    assert_eq!(report.sideboard.len(), 1);
    assert_eq!(report.sideboard[0].name, "Shock");
    assert_eq!(report.commander, vec!["Centaur Courser".to_string()]);
    assert!(!report.launchable, "Commander is not available");
    assert!(
        report
            .refusal
            .clone()
            .unwrap_or_default()
            .contains("Commander")
    );
    assert_eq!(report.excluded.len(), 1);
    assert_eq!(report.excluded[0].name, "Llanowar Elves");
    assert!(
        plan_deck(&report).is_err(),
        "a Commander deck never launches"
    );
}

#[test]
fn an_unsupported_card_in_a_text_list_is_refused_with_its_name() {
    let text = "Deck\n20 Forest\n20 Llanowar Elves\n";
    let err = import_text("Unsupported", text).expect_err("must refuse");
    let rendered = err.to_string();
    assert!(rendered.contains("Llanowar Elves"), "{rendered}");
    assert!(rendered.contains("not in the supported pool"), "{rendered}");
}

#[test]
fn an_empty_text_list_is_an_error() {
    assert!(import_text("Empty", "\n// nothing here\n").is_err());
}
