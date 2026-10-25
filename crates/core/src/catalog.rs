//! The supported card pool. Twenty-eight real cards; every printed ability is
//! implemented. Card text is the short functional Oracle wording.
//!
//! Activated and triggered abilities are declared in [`crate::ability`] and
//! executed by the engine; a card with an ability this crate cannot execute is
//! refused at import instead of being approximated.
//!
//! Deliberate exclusions (documented in HANDOFF.md): no static abilities, no
//! replacement effects, and no keyword printed on a pool card. Typed counters
//! (CR 122) and creature tokens (CR 111) are implemented in [`crate::counters`]
//! and [`crate::tokens`]; token characteristics live outside this pool, so a
//! token can never be imported or cast as a card.

use crate::card::{CardDef, CardId, Effect, Kind, ManaColor, SpellSpec, TargetSpec};

const DRAW_TWO: &[Effect] = &[Effect::Draw(2)];
const SCRY_ONE_DRAW: &[Effect] = &[Effect::Scry(1), Effect::Draw(1)];
const SURVEIL_ONE_DRAW: &[Effect] = &[Effect::Surveil(1), Effect::Draw(1)];
const SHOCK: &[Effect] = &[Effect::Damage(2)];
const STRIKE: &[Effect] = &[Effect::Damage(3)];
const COUNTER: &[Effect] = &[Effect::CounterTargetSpell];
const GROWTH: &[Effect] = &[Effect::Pump(3, 3)];
const SACRIFICE_ALL: &[Effect] = &[Effect::EachPlayerSacrifice];
const RETURN_HAND: &[Effect] = &[Effect::ReturnToHand];
const TOP_LIBRARY: &[Effect] = &[Effect::PutOnTopOfLibrary];
const SWORDS: &[Effect] = &[Effect::ExileAndGainLife];
const PREVENT_COMBAT: &[Effect] = &[Effect::PreventAllCombatDamageThisTurn];

const LAND: &[Kind] = &[Kind::Land];
const CREATURE: &[Kind] = &[Kind::Creature];
const INSTANT: &[Kind] = &[Kind::Instant];
const SORCERY: &[Kind] = &[Kind::Sorcery];

macro_rules! card {
    (
        $id:ident, $name:literal, cost: $cost:expr, kinds: $kinds:expr, basic: $basic:expr,
        type_line: $tl:literal, power: $power:expr, toughness: $toughness:expr,
        oracle: $oracle:literal, spell: $spell:expr
    ) => {
        CardDef {
            id: CardId::$id,
            name: $name,
            cost: $cost,
            kinds: $kinds,
            basic_mana: $basic,
            type_line: $tl,
            power: $power,
            toughness: $toughness,
            oracle: $oracle,
            spell: $spell,
        }
    };
}

/// Compiled-in pool; the order matches the catalog documentation.
pub static CARDS: [CardDef; 28] = [
    card!(
        Island, "Island", cost: None, kinds: LAND, basic: Some(ManaColor::Blue),
        type_line: "Basic Land — Island", power: None, toughness: None,
        oracle: "({T}: Add {U}.)", spell: None
    ),
    card!(
        Mountain, "Mountain", cost: None, kinds: LAND, basic: Some(ManaColor::Red),
        type_line: "Basic Land — Mountain", power: None, toughness: None,
        oracle: "({T}: Add {R}.)", spell: None
    ),
    card!(
        Forest, "Forest", cost: None, kinds: LAND, basic: Some(ManaColor::Green),
        type_line: "Basic Land — Forest", power: None, toughness: None,
        oracle: "({T}: Add {G}.)", spell: None
    ),
    card!(
        GrizzlyBears, "Grizzly Bears", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(2), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        RuneclawBear, "Runeclaw Bear", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(2), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        CentaurCourser, "Centaur Courser", cost: Some("{2}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Centaur Warrior", power: Some(3), toughness: Some(3),
        oracle: "", spell: None
    ),
    card!(
        CrawWurm, "Craw Wurm", cost: Some("{4}{G}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Wurm", power: Some(6), toughness: Some(4),
        oracle: "", spell: None
    ),
    card!(
        HillGiant, "Hill Giant", cost: Some("{3}{R}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Giant", power: Some(3), toughness: Some(3),
        oracle: "", spell: None
    ),
    card!(
        GiantGrowth, "Giant Growth", cost: Some("{G}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Target creature gets +3/+3 until end of turn.",
        spell: Some(SpellSpec { target: TargetSpec::Creature, instant: true, effects: GROWTH })
    ),
    card!(
        Shock, "Shock", cost: Some("{R}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Shock deals 2 damage to any target.",
        spell: Some(SpellSpec { target: TargetSpec::AnyTarget, instant: true, effects: SHOCK })
    ),
    card!(
        LightningStrike, "Lightning Strike", cost: Some("{1}{R}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Lightning Strike deals 3 damage to any target.",
        spell: Some(SpellSpec { target: TargetSpec::AnyTarget, instant: true, effects: STRIKE })
    ),
    card!(
        Counterspell, "Counterspell", cost: Some("{U}{U}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Counter target spell.",
        spell: Some(SpellSpec { target: TargetSpec::Spell, instant: true, effects: COUNTER })
    ),
    card!(
        Divination, "Divination", cost: Some("{2}{U}"), kinds: SORCERY, basic: None,
        type_line: "Sorcery", power: None, toughness: None,
        oracle: "Draw two cards.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: false, effects: DRAW_TWO })
    ),
    card!(
        Opt, "Opt", cost: Some("{U}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Scry 1. Draw a card.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: true, effects: SCRY_ONE_DRAW })
    ),
    card!(
        Consider, "Consider", cost: Some("{U}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Surveil 1. Draw a card.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: true, effects: SURVEIL_ONE_DRAW })
    ),
    card!(
        CrackTheEarth, "Crack the Earth", cost: Some("{R}"), kinds: SORCERY, basic: None,
        type_line: "Sorcery — Arcane", power: None, toughness: None,
        oracle: "Each player sacrifices a permanent of their choice.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: false, effects: SACRIFICE_ALL })
    ),
    card!(
        Plains, "Plains", cost: None, kinds: LAND, basic: Some(ManaColor::White),
        type_line: "Basic Land — Plains", power: None, toughness: None,
        oracle: "({T}: Add {W}.)", spell: None
    ),
    card!(
        IsamaruHoundOfKonda, "Isamaru, Hound of Konda", cost: Some("{W}"), kinds: CREATURE, basic: None,
        type_line: "Legendary Creature — Dog", power: Some(2), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        Unsummon, "Unsummon", cost: Some("{U}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Return target creature to its owner's hand.",
        spell: Some(SpellSpec { target: TargetSpec::Creature, instant: true, effects: RETURN_HAND })
    ),
    card!(
        TimeEbb, "Time Ebb", cost: Some("{2}{U}"), kinds: SORCERY, basic: None,
        type_line: "Sorcery", power: None, toughness: None,
        oracle: "Put target creature on top of its owner's library.",
        spell: Some(SpellSpec { target: TargetSpec::Creature, instant: false, effects: TOP_LIBRARY })
    ),
    card!(
        SwordsToPlowshares, "Swords to Plowshares", cost: Some("{W}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Exile target creature. Its controller gains life equal to its power.",
        spell: Some(SpellSpec { target: TargetSpec::Creature, instant: true, effects: SWORDS })
    ),
    card!(
        Fog, "Fog", cost: Some("{G}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Prevent all combat damage that would be dealt this turn.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: true, effects: PREVENT_COMBAT })
    ),
    card!(
        ProdigalSorcerer, "Prodigal Sorcerer", cost: Some("{2}{U}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Human Wizard", power: Some(1), toughness: Some(1),
        oracle: "{T}: Prodigal Sorcerer deals 1 damage to any target.", spell: None
    ),
    card!(
        Rootwalla, "Rootwalla", cost: Some("{2}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Lizard", power: Some(2), toughness: Some(2),
        oracle: "{1}{G}: Rootwalla gets +2/+2 until end of turn. Activate only once each turn.",
        spell: None
    ),
    card!(
        BrindleBoar, "Brindle Boar", cost: Some("{2}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Boar", power: Some(2), toughness: Some(2),
        oracle: "Sacrifice Brindle Boar: You gain 4 life.", spell: None
    ),
    card!(
        ElvishVisionary, "Elvish Visionary", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Elf Shaman", power: Some(1), toughness: Some(1),
        oracle: "When Elvish Visionary enters the battlefield, draw a card.", spell: None
    ),
    card!(
        FesteringGoblin, "Festering Goblin", cost: Some("{B}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Zombie Goblin", power: Some(1), toughness: Some(1),
        oracle: "When Festering Goblin dies, target creature gets -1/-1 until end of turn.",
        spell: None
    ),
    card!(
        Oculus, "Oculus", cost: Some("{1}{U}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Phyrexian Homunculus", power: Some(1), toughness: Some(1),
        oracle: "When Oculus dies, you may draw a card.", spell: None
    ),
];

pub fn all() -> &'static [CardDef] {
    &CARDS
}

pub fn def(id: CardId) -> &'static CardDef {
    CARDS
        .iter()
        .find(|c| c.id == id)
        .unwrap_or_else(|| crate::tokens::def(id))
}

pub fn by_name(name: &str) -> Option<CardId> {
    let needle = name.trim();
    CARDS
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case(needle))
        .map(|c| c.id)
}

pub fn pool_size() -> usize {
    CARDS.len()
}
