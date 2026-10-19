//! Deck lists: a name plus card quantities. The library *order* is always
//! derived from a seed, never stored, so a replay only needs the lists.

use serde::{Deserialize, Serialize};

use crate::card::CardId;
use crate::catalog;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckList {
    pub name: String,
    /// Card id plus quantity, in the order written.
    pub cards: Vec<(CardId, u8)>,
}

impl DeckList {
    pub fn new(name: impl Into<String>, cards: Vec<(CardId, u8)>) -> Self {
        DeckList {
            name: name.into(),
            cards: cards.into_iter().filter(|(_, n)| *n > 0).collect(),
        }
    }

    pub fn size(&self) -> usize {
        self.cards.iter().map(|(_, n)| *n as usize).sum()
    }

    /// Expanded list of card ids (one entry per physical card).
    pub fn expand(&self) -> Vec<CardId> {
        let mut out = Vec::with_capacity(self.size());
        for (card, n) in &self.cards {
            for _ in 0..*n {
                out.push(*card);
            }
        }
        out
    }

    pub fn count_of(&self, card: CardId) -> u8 {
        self.cards
            .iter()
            .filter(|(c, _)| *c == card)
            .map(|(_, n)| *n)
            .sum()
    }

    pub fn lands(&self) -> u8 {
        self.cards
            .iter()
            .filter(|(c, _)| c.def().is_land())
            .map(|(_, n)| *n)
            .sum()
    }

    /// Parse the practice list format: `1 Card Name`, `// comment`, blank
    /// lines, and section headers (`Deck`, `Sideboard`, `Commander`) which the
    /// practice profile does not use.
    pub fn parse_practice(name: &str, text: &str) -> Result<DeckList, String> {
        let mut cards: Vec<(CardId, u8)> = Vec::new();
        for (lineno, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                continue;
            }
            if line.ends_with(':') || line.eq_ignore_ascii_case("deck") {
                continue;
            }
            let mut parts = line.split_whitespace();
            let qty = parts
                .next()
                .ok_or_else(|| format!("line {}: empty", lineno + 1))?
                .parse::<u8>()
                .map_err(|_| format!("line {}: expected a quantity, got '{line}'", lineno + 1))?;
            let card_name = parts.collect::<Vec<_>>().join(" ");
            let card = catalog::by_name(&card_name).ok_or_else(|| {
                format!(
                    "line {}: '{card_name}' is not in the supported pool",
                    lineno + 1
                )
            })?;
            match cards.iter_mut().find(|(c, _)| *c == card) {
                Some((_, n)) => *n = n.saturating_add(qty),
                None => cards.push((card, qty)),
            }
        }
        if cards.is_empty() {
            return Err("deck list contains no supported cards".to_string());
        }
        Ok(DeckList::new(name, cards))
    }

    pub fn render(&self) -> String {
        let mut out = format!("// {}\n", self.name);
        for (card, n) in &self.cards {
            out.push_str(&format!("{n} {}\n", card.name()));
        }
        out
    }
}
