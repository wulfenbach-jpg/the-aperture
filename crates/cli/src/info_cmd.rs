//! Informational commands: the pool, the bundled decks, and coaching output.

use std::path::Path;

use aperture_core::{Game, RULES_REVISION, catalog, coach};

use crate::decks;

pub fn pool() -> anyhow::Result<()> {
    println!(
        "THE APERTURE supported pool — {} cards (rules revision {RULES_REVISION})",
        catalog::pool_size()
    );
    println!(
        "practice profile: 1v1, 20 life, 40-card decks, London mulligan. Commander: NOT implemented.\n"
    );
    for card in catalog::all() {
        let kind = card
            .kinds
            .iter()
            .map(|k| k.name())
            .collect::<Vec<_>>()
            .join(" ");
        let stats = match (card.power, card.toughness) {
            (Some(p), Some(t)) => format!("{p}/{t}"),
            _ => String::new(),
        };
        println!(
            "{:<16} {:<6} {:<14} {:<4} {}",
            card.name,
            card.cost.unwrap_or("—"),
            kind,
            stats,
            card.oracle
        );
    }
    println!(
        "\nnot implemented (refused at import): triggers, other activated abilities, static abilities, replacement effects, tokens, counters in the rules layer, milling, tutoring, multi-face cards, any card outside this pool."
    );
    Ok(())
}

pub fn decks() -> anyhow::Result<()> {
    for (name, text) in [
        ("Temur Beats (bundled A)", decks::BUNDLED_TEMUR),
        ("Izzet Control (bundled B)", decks::BUNDLED_IZZET),
    ] {
        let list = aperture_core::DeckList::parse_practice(name, text)
            .map_err(|e| anyhow::anyhow!("bundled {name}: {e}"))?;
        println!("{name}: {} cards", list.size());
        for (card, count) in &list.cards {
            println!("  {count} {}", card.name());
        }
    }
    Ok(())
}

pub fn coach(seed: u64, deck_a: &Path, deck_b: &Path) -> anyhow::Result<()> {
    let (a, _) = decks::load(deck_a, decks::BUNDLED_TEMUR, "Temur Beats")?;
    let (b, _) = decks::load(deck_b, decks::BUNDLED_IZZET, "Izzet Control")?;
    let (game, log) = Game::new_logged(seed, &a, &b);
    let legal = game.legal_actions();
    println!(
        "coaching for seed {seed} — decision point {} (revision {}, pending: none yet)",
        game.decision(),
        game.revision()
    );
    for line in game.view_for(aperture_core::P0, &log).log {
        println!("  log: {line}");
    }
    for explanation in coach::explain(&game, aperture_core::P0, &legal) {
        println!("\n[{}] {}", explanation.topic, explanation.text);
        for rule in &explanation.rules {
            match coach::rule_note(rule) {
                Some(note) => println!("    CR {rule}: {note}"),
                None => println!("    CR {rule}"),
            }
        }
    }
    Ok(())
}
