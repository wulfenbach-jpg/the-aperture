//! Creature tokens (CR 111) and the keyword set the practice slice enforces.
//!
//! A token is not a card: it is never importable or cast from a deck. It is
//! owned by its creator (CR 111.2) and controlled by that player (CR 111.3).
//! A departing token reaches its destination and applicable abilities trigger;
//! it then ceases to exist at the next state-based-action check (CR 111.7).
//!
//! Soldier/Spirit characteristics are compiled in [`TOKEN_CARDS`]; corpus
//! Goblin/Drake characteristics live in the catalog's separate token table.
//! Neither table is part of the importable [`crate::catalog::CARDS`] pool.
//!
//! Typed flying/reach/defender keywords are enforced by attacking and blocking
//! rules (CR 508.1, 509.1b, 702.9b/c, 702.17b). The keyword lookup is explicit;
//! it never infers mechanics by interpreting card text at runtime.

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
    /// CR 702.14a: "This creature can't be blocked as long as defending player
    /// controls an Island." Modelled as a blocking restriction only; nothing
    /// grants or removes it, and no other landwalk variant is supported.
    Islandwalk,
    /// CR 702.14a: can't be blocked while defending player controls a Mountain.
    Mountainwalk,
    /// CR 702.10a: a creature with haste ignores the summoning-sickness
    /// attack/tap restriction.
    Haste,
    /// CR 702.20a: "Attacking doesn't cause this creature to tap." The combat
    /// engine reads it at the only place an attacker is tapped (CR 508.1f); it
    /// changes nothing else, so a vigilant creature can block in the following
    /// turn's combat just as it can attack again.
    Vigilance,
}

impl Keyword {
    pub fn name(self) -> &'static str {
        match self {
            Keyword::Flying => "flying",
            Keyword::Reach => "reach",
            Keyword::Defender => "defender",
            Keyword::Islandwalk => "islandwalk",
            Keyword::Vigilance => "vigilance",
            Keyword::Haste => "haste",
            Keyword::Mountainwalk => "mountainwalk",
        }
    }
}

/// CR 509.1b: a blocking creature must be able to block the attacker.
/// Evasion is the only restriction this slice models: flying (CR 702.9b) and
/// islandwalk (CR 702.14c).
///
/// `defender_controls_island` is read from the battlefield of the player being
/// attacked, never from the attacker's own lands, because landwalk is a
/// property of the defending player's board (CR 702.14c).
pub fn can_block(
    attacker: &[Keyword],
    blocker: &[Keyword],
    defender_controls_island: bool,
    defender_controls_mountain: bool,
    attacker_has_mountainwalk: bool,
) -> bool {
    // Landwalk is checked first: it forbids every blocker, whatever keywords
    // that blocker has, so no finite keyword list can answer it.
    if attacker.contains(&Keyword::Islandwalk) && defender_controls_island {
        return false;
    }
    if (attacker_has_mountainwalk || attacker.contains(&Keyword::Mountainwalk))
        && defender_controls_mountain
    {
        return false;
    }
    if !attacker.contains(&Keyword::Flying) {
        return true;
    }
    // CR 702.9b: flying can be blocked only by a creature with flying.
    // CR 702.17b: reach lets a creature block as though it had flying.
    blocker.contains(&Keyword::Flying) || blocker.contains(&Keyword::Reach)
}

/// The keyword list for a card. Token cards carry theirs here, and so do pool
/// cards whose printed keyword changes blocking legality: this table is the one
/// source the blocking path reads, so an islandwalk card is declared here rather
/// than in a `CardDef` flag that no pair check consults.
pub fn keywords(card: CardId) -> &'static [Keyword] {
    match card {
        CardId::SpiritToken | CardId::DrakeToken => &[Keyword::Flying],
        CardId::JaddiOffshoot => &[Keyword::Defender],
        CardId::PaleBears | CardId::RiverBear => &[Keyword::Islandwalk],
        // Lumra, Bellow of the Woods prints "Reach, vigilance": reach lets it
        // block a flyer, vigilance keeps it untapped when it attacks.
        CardId::LumraBellowOfTheWoods => &[Keyword::Reach, Keyword::Vigilance],
        // Rampaging Yao Guai prints "Vigilance, trample": trample is the
        // `CardDef` flag the damage assignment reads, and vigilance is here
        // because the attack path asks this table before it taps an attacker
        // (CR 702.20a).
        CardId::RampagingYaoGuai => &[Keyword::Vigilance],
        // Goblin Chieftain prints Haste; its other-Goblin haste grant is a
        // separate static ability.
        CardId::GoblinChieftain => &[Keyword::Haste],
        CardId::VolcanicDragon => &[Keyword::Flying, Keyword::Haste],
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
        artifact_mana: None,
        type_line: "Token Creature — Soldier",
        colors: &[],
        flying: false,
        defender: false,
        flash: false,
        trample: false,
        islandwalk: false,
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
        artifact_mana: None,
        type_line: "Token Creature — Spirit",
        colors: &[crate::card::ManaColor::White],
        flying: true,
        defender: false,
        flash: false,
        trample: false,
        islandwalk: false,
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
                paired_with: None,
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
        assert!(can_block(&[], &[], false, false, false));
        assert!(can_block(&[Keyword::Defender], &[], false, false, false));
        assert!(!can_block(&[Keyword::Flying], &[], false, false, false));
        assert!(!can_block(
            &[Keyword::Flying],
            &[Keyword::Defender],
            false,
            false,
            false
        ));
        assert!(can_block(
            &[Keyword::Flying],
            &[Keyword::Flying],
            false,
            false,
            false
        ));
        assert!(can_block(
            &[Keyword::Flying],
            &[Keyword::Reach],
            false,
            false,
            false
        ));
    }

    #[test]
    fn islandwalk_forbids_every_blocker_while_the_defender_controls_an_island() {
        // CR 702.14c: no blocker is legal, whatever its keywords.
        assert!(!can_block(&[Keyword::Islandwalk], &[], true, false, false));
        assert!(!can_block(
            &[Keyword::Islandwalk],
            &[Keyword::Reach],
            true,
            false,
            false
        ));
        // Without an Island on the defending board the restriction is inert.
        assert!(can_block(&[Keyword::Islandwalk], &[], false, false, false));
        assert!(can_block(
            &[Keyword::Islandwalk],
            &[Keyword::Defender],
            false,
            false,
            false
        ));
        // It is not a grant: a plain creature is unaffected either way.
        assert!(can_block(&[], &[Keyword::Islandwalk], true, false, false));
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
