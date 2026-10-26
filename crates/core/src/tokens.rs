//! Creature tokens (CR 111) and the keyword set the practice slice enforces.
//!
//! A token is not a card: it has no mana cost, never exists in a library or
//! hand, is owned by the player who created it (CR 111.2), is controlled by that
//! player (CR 111.3) and ceases to exist the instant it leaves the battlefield
//! (CR 111.7) instead of moving to the graveyard/hand/library.
//!
//! Token characteristics are compiled in [`TOKEN_CARDS`], which is deliberately
//! *not* part of [`crate::catalog::CARDS`]: the deck importer can name pool
//! cards only, so a token can never be imported, put in a deck, or cast. The
//! 28-card playable pool is unchanged.
//!
//! Keywords are typed and enforced at the two places the engine can observe
//! them: attacking (CR 508.1) and blocking (CR 509.1b, 702.9b/c, 702.17b). The
//! certified 28-card pool contains no keyword creatures, so today the only
//! source of a keyword is a token; a card keyword lands by adding a `card` arm
//! to [`keywords`], never by trusting card text at runtime.

use crate::card::{CardDef, CardId, Kind};
use crate::counters::Counters;
use crate::ids::{ObjectId, PlayerId, Zone};
use crate::state::{GameState, Object};

/// A keyword the combat rules engine actually enforces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Keyword {
    /// CR 702.9: can't be blocked except by flying or reach; can block flying.
    Flying,
    /// CR 702.17: can block a creature with flying.
    Reach,
    /// CR 702.3: can't attack.
    Defender,
}

impl Keyword {
    pub fn name(self) -> &'static str {
        match self {
            Keyword::Flying => "flying",
            Keyword::Reach => "reach",
            Keyword::Defender => "defender",
        }
    }
}

/// CR 509.1b: a blocking creature must be able to block the attacker.
/// Evasion is the only restriction this slice models.
pub fn can_block(attacker: &[Keyword], blocker: &[Keyword]) -> bool {
    if !attacker.contains(&Keyword::Flying) {
        return true;
    }
    // CR 702.9b: flying can be blocked only by a creature with flying.
    // CR 702.17b: reach lets a creature block as though it had flying.
    blocker.contains(&Keyword::Flying) || blocker.contains(&Keyword::Reach)
}

/// The keyword list for a card. Pool cards have none; token cards carry theirs.
pub fn keywords(card: CardId) -> &'static [Keyword] {
    match card {
        CardId::SpiritToken | CardId::DrakeToken => &[Keyword::Flying],
        CardId::JaddiOffshoot => &[Keyword::Defender],
        _ => &[],
    }
}

const CREATURE: &[Kind] = &[Kind::Creature];

/// Compiled-in token characteristics. Never part of the importable pool.
pub static TOKEN_CARDS: [CardDef; 2] = [
    CardDef {
        id: CardId::SoldierToken,
        name: "Soldier",
        cost: None,
        kinds: CREATURE,
        basic_mana: None,
        type_line: "Token Creature — Soldier",
        colors: &[],
        flying: false,
        defender: false,
        flash: false,
        trample: false,
        power: Some(1),
        toughness: Some(1),
        oracle: "",
        spell: None,
    },
    CardDef {
        id: CardId::SpiritToken,
        name: "Spirit",
        cost: None,
        kinds: CREATURE,
        basic_mana: None,
        type_line: "Token Creature — Spirit",
        colors: &[crate::card::ManaColor::White],
        flying: true,
        defender: false,
        flash: false,
        trample: false,
        power: Some(1),
        toughness: Some(1),
        oracle: "Flying",
        spell: None,
    },
];

/// One creatable token.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum TokenKind {
    /// 1/1 vanilla Soldier.
    Soldier,
    /// 1/1 flying Spirit (CR 702.9).
    Spirit,
}

impl TokenKind {
    pub const ALL: [TokenKind; 2] = [TokenKind::Soldier, TokenKind::Spirit];

    pub fn card(self) -> CardId {
        match self {
            TokenKind::Soldier => CardId::SoldierToken,
            TokenKind::Spirit => CardId::SpiritToken,
        }
    }

    pub fn spec(self) -> &'static CardDef {
        def(self.card())
    }

    pub fn name(self) -> &'static str {
        self.spec().name
    }
}

/// The token card definition for a token card id. Panics for a non-token id,
/// which is exactly what [`crate::catalog::def`] relies on when it falls back
/// here: a pool card is always found in the pool first.
pub fn def(card: CardId) -> &'static CardDef {
    TOKEN_CARDS
        .iter()
        .find(|def| def.id == card)
        .unwrap_or_else(|| panic!("{card:?} is not a token card"))
}

impl GameState {
    /// CR 111.1/111.2/111.3: create one token permanent for `controller`.
    ///
    /// The token is its creator's permanent from the instant it exists: owner
    /// and controller are both `controller`, it enters untapped and (as a
    /// creature) summoning-sick. It is never a spell, never on the stack and
    /// never placed in a zone list.
    pub fn create_token(&mut self, kind: TokenKind, controller: PlayerId) -> ObjectId {
        assert!(
            (controller as usize) < self.players.len(),
            "token controller P{controller} is not a seat"
        );
        let card = kind.card();
        let id = ObjectId(self.next_object);
        self.next_object += 1;
        self.objects.insert(
            id,
            Object {
                id,
                card,
                owner: controller,
                controller,
                zone: Zone::Battlefield,
                commander: None,
                tapped: false,
                damage: 0,
                // CR 302.6: a creature that was not under its controller's
                // control since their most recent turn began is summoning sick.
                summoning_sick: card.def().is_creature(),
                pump_power: 0,
                pump_toughness: 0,
                plus_one_counters: 0,
                counters: Counters::default(),
            },
        );
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flying_is_blocked_only_by_flying_or_reach() {
        assert!(can_block(&[], &[]));
        assert!(can_block(&[Keyword::Defender], &[]));
        assert!(!can_block(&[Keyword::Flying], &[]));
        assert!(!can_block(&[Keyword::Flying], &[Keyword::Defender]));
        assert!(can_block(&[Keyword::Flying], &[Keyword::Flying]));
        assert!(can_block(&[Keyword::Flying], &[Keyword::Reach]));
    }

    #[test]
    fn token_cards_are_creatures_outside_the_importable_pool() {
        for kind in TokenKind::ALL {
            let def = kind.spec();
            assert!(def.is_creature(), "{} is a creature token", def.name);
            assert!(
                def.cost.is_none(),
                "{} is not a card with a mana cost",
                def.name
            );
            assert!(
                crate::catalog::all().iter().all(|pool| pool.id != def.id),
                "{} must not be importable",
                def.name
            );
        }
        assert_eq!(keywords(CardId::SpiritToken), &[Keyword::Flying]);
        assert!(keywords(CardId::GrizzlyBears).is_empty());
    }
}
