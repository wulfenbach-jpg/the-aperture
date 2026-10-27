//! The supported card pool. One hundred and twenty-six real cards; every printed ability is
//! implemented. Card text is the short functional Oracle wording (vanilla
//! creatures print no text at all).
//!
//! The issue-17 starter package adds 47 sourced vanilla green/white creatures
//! and the vanilla commander Jasmine Boreal. Slice 2 adds the Krenko goblin
//! package: six sourced cards plus an engine-only 1/1 red Goblin token card.
//! Token creation is typed; tokens are never importable catalog cards.
//!
//! Slice 5 adds the sourced mill / counter / ritual package: Stitcher's Supplier
//! (mill on enters and on dies), Ruin Crab (landfall mill for each opponent),
//! Negate ("counter target noncreature spell"), Preordain (scry 2 then draw),
//! Pyretic Ritual and Seething Song (add red mana to the pool).
//!
//! Slice 6 adds the sourced death-watcher package: Blood Artist (any creature
//! dies → target player loses 1 life, you gain 1), Zulaport Cutthroat (a
//! creature you control dies → each opponent loses 1 life, you gain 1) and
//! Hedron Crab (landfall → target player mills three). It introduces
//! `TargetSpec::Player`, life loss as distinct from damage, and the "creature
//! dies" watcher family. `Effect::LoseLifeEachOpponent` and
//! `Effect::MillTargetPlayer` are ability-only for now; no printed spell text in
//! the pool uses them.
//!
//! Activated and triggered abilities are declared in [`crate::ability`] and
//! executed by the engine; a card with an ability this crate cannot execute is
//! refused at import instead of being approximated.
//!
//! Static abilities are declared in [`crate::ability::static_abilities`]; damage
//! doubling lives there and characteristic changes are applied through
//! [`crate::layer`]. Replacement effects (CR 614) and prevention shields (CR 615)
//! are resolved by the engine; static forms outside the declared kinds are
//! refused at import.
//!
//! Deliberate exclusions (documented in HANDOFF.md): no keyword printed on a pool
//! card and no layer 4–6/7b/7d continuous effects. Typed counters (CR 122) and
//! creature tokens (CR 111) are implemented in [`crate::counters`] and
//! [`crate::tokens`]; token characteristics live outside this pool, so a token can
//! never be imported or cast as a card.

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
const PREVENT_FOUR: &[Effect] = &[Effect::PreventNextDamage(4)];
const PREORDAIN_SCRY_TWO_DRAW: &[Effect] = &[Effect::Scry(2), Effect::Draw(1)];
const ADD_THREE_RED: &[Effect] = &[Effect::AddMana {
    color: ManaColor::Red,
    amount: 3,
}];
const ADD_FIVE_RED: &[Effect] = &[Effect::AddMana {
    color: ManaColor::Red,
    amount: 5,
}];

const LAND: &[Kind] = &[Kind::Land];
const CREATURE: &[Kind] = &[Kind::Creature];
const INSTANT: &[Kind] = &[Kind::Instant];
const SORCERY: &[Kind] = &[Kind::Sorcery];
const ARTIFACT_CREATURE: &[Kind] = &[Kind::Artifact, Kind::Creature];
const ENCHANTMENT: &[Kind] = &[Kind::Enchantment];

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
            artifact_mana: None,
            type_line: $tl,
            colors: &[],
            flying: false,
            defender: false,
            flash: false,
            trample: false,
            islandwalk: false,
            power: $power,
            toughness: $toughness,
            oracle: $oracle,
            spell: $spell,
        }
    };
}

macro_rules! card_artifact {
    (
        $id:ident, $name:literal, cost: $cost:expr, type_line: $tl:literal,
        oracle: $oracle:literal, mana: $mana:expr, spell: $spell:expr
    ) => {
        CardDef {
            id: CardId::$id,
            name: $name,
            cost: $cost,
            kinds: &[Kind::Artifact],
            basic_mana: None,
            artifact_mana: $mana,
            type_line: $tl,
            colors: &[],
            flying: false,
            defender: false,
            flash: false,
            trample: false,
            islandwalk: false,
            power: None,
            toughness: None,
            oracle: $oracle,
            spell: $spell,
        }
    };
}

macro_rules! card_defender {
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
            artifact_mana: None,
            type_line: $tl,
            colors: &[],
            flying: false,
            defender: true,
            flash: false,
            trample: false,
            islandwalk: false,
            power: $power,
            toughness: $toughness,
            oracle: $oracle,
            spell: $spell,
        }
    };
}

/// A creature whose only printed rules text is the Flash keyword (CR 702.8a).
macro_rules! card_flash {
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
            artifact_mana: None,
            type_line: $tl,
            colors: &[],
            flying: false,
            defender: false,
            flash: true,
            trample: false,
            islandwalk: false,
            power: $power,
            toughness: $toughness,
            oracle: $oracle,
            spell: $spell,
        }
    };
}

/// A creature whose only printed rules text is Islandwalk (CR 702.14a).
macro_rules! card_islandwalk {
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
            artifact_mana: None,
            type_line: $tl,
            colors: &[],
            flying: false,
            defender: false,
            flash: false,
            trample: false,
            islandwalk: true,
            power: $power,
            toughness: $toughness,
            oracle: $oracle,
            spell: $spell,
        }
    };
}

macro_rules! card_trample {
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
            artifact_mana: None,
            type_line: $tl,
            colors: &[],
            flying: false,
            defender: false,
            flash: false,
            trample: true,
            islandwalk: false,
            power: $power,
            toughness: $toughness,
            oracle: $oracle,
            spell: $spell,
        }
    };
}

/// Compiled-in pool; the order matches the catalog documentation.
pub static CARDS: [CardDef; 136] = [
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
    card!(
        MendingHands, "Mending Hands", cost: Some("{W}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Prevent the next 4 damage that would be dealt to any target this turn.",
        spell: Some(SpellSpec { target: TargetSpec::AnyTarget, instant: true, effects: PREVENT_FOUR })
    ),
    card!(
        AngrathsMarauders, "Angrath's Marauders", cost: Some("{5}{R}{R}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Human Pirate", power: Some(4), toughness: Some(4),
        oracle: "If a source you control would deal damage to a permanent or player, it deals double that damage to that permanent or player instead.",
        spell: None
    ),
    card!(
        WightOfPrecinctSix, "Wight of Precinct Six", cost: Some("{1}{B}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Zombie", power: Some(1), toughness: Some(1),
        oracle: "This creature gets +1/+1 for each creature card in your opponents' graveyards.",
        spell: None
    ),
    card!(
        SoulsAttendant, "Soul's Attendant", cost: Some("{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Human Cleric", power: Some(1), toughness: Some(1),
        oracle: "Whenever another creature enters, you may gain 1 life.", spell: None
    ),
    card!(
        RazorclawBear, "Razorclaw Bear", cost: Some("{2}{G}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(3), toughness: Some(3),
        oracle: "Whenever this creature becomes blocked, it gets +2/+2 until end of turn.", spell: None
    ),
    card!(
        VolleyVeteran, "Volley Veteran", cost: Some("{3}{R}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Goblin Warrior", power: Some(4), toughness: Some(2),
        oracle: "When this creature enters, it deals damage to target creature an opponent controls equal to the number of Goblins you control.", spell: None
    ),
    card!(
        DramaticReversal, "Dramatic Reversal", cost: Some("{1}{U}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Untap all nonland permanents you control.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: true, effects: &[Effect::UntapAllNonlandsYouControl] })
    ),
    card!(
        GoblinWarStrike, "Goblin War Strike", cost: Some("{R}"), kinds: SORCERY, basic: None,
        type_line: "Sorcery", power: None, toughness: None,
        oracle: "Goblin War Strike deals damage to target player or planeswalker equal to the number of Goblins you control.",
        spell: Some(SpellSpec { target: TargetSpec::Player, instant: false, effects: &[Effect::DamageTargetPlayerForEachGoblin] })
    ),
    card!(
        SazhsChocobo, "Sazh's Chocobo", cost: Some("{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bird", power: Some(0), toughness: Some(1),
        oracle: "Landfall — Whenever a land you control enters, put a +1/+1 counter on this creature.",
        spell: None
    ),
    card_trample!(
        RampagingBaloths, "Rampaging Baloths", cost: Some("{4}{G}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Beast", power: Some(6), toughness: Some(6),
        oracle: "Trample\nLandfall — Whenever a land you control enters, create a 4/4 green Beast creature token.",
        spell: None
    ),
    card!(
        ProfessorOfZoomancy, "Professor of Zoomancy", cost: Some("{3}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear Druid", power: Some(4), toughness: Some(3),
        oracle: "When this creature enters, create a 1/1 black and green Pest creature token with \"When this token dies, you gain 1 life.\"",
        spell: None
    ),
    card_artifact!(
        ArcaneSignet, "Arcane Signet", cost: Some("{2}"), type_line: "Artifact",
        oracle: "{T}: Add one mana of any color in your commander's color identity.",
        mana: None, spell: None
    ),
    card_artifact!(
        FellwarStone, "Fellwar Stone", cost: Some("{2}"), type_line: "Artifact",
        oracle: "{T}: Add one mana of any color that a land an opponent controls could produce.",
        mana: None, spell: None
    ),
    card!(
        FriendlyTeddy, "Friendly Teddy", cost: Some("{2}"), kinds: ARTIFACT_CREATURE, basic: None,
        type_line: "Artifact Creature — Bear Toy", power: Some(2), toughness: Some(2),
        oracle: "When this creature dies, each player draws a card.", spell: None
    ),
    card!(
        UlvenwaldBear, "Ulvenwald Bear", cost: Some("{2}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(2), toughness: Some(2),
        oracle: "Morbid — When this creature enters, if a creature died this turn, put two +1/+1 counters on target creature.",
        spell: None
    ),
    card_flash!(
        CallerOfTheClaw, "Caller of the Claw", cost: Some("{2}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Elf", power: Some(2), toughness: Some(2),
        oracle: "Flash\nWhen this creature enters, create a 2/2 green Bear creature token for each nontoken creature put into your graveyard from the battlefield this turn.",
        spell: None
    ),
    card_artifact!(
        CommandersSphere, "Commander's Sphere", cost: Some("{3}"), type_line: "Artifact",
        oracle: "{T}: Add one mana of any color in your commander's color identity.\nSacrifice this artifact: Draw a card.",
        mana: None, spell: None
    ),
    card!(
        Werebear, "Werebear", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Human Bear Druid", power: Some(1), toughness: Some(1),
        oracle: "{T}: Add {G}.\nThreshold — This creature gets +3/+3 as long as there are seven or more cards in your graveyard.",
        spell: None
    ),
    card_artifact!(
        SolRing, "Sol Ring", cost: Some("{1}"), type_line: "Artifact",
        oracle: "{T}: Add {C}{C}.", mana: Some((ManaColor::Colorless, 2)), spell: None
    ),
    card_artifact!(
        MindStone, "Mind Stone", cost: Some("{2}"), type_line: "Artifact",
        oracle: "{T}: Add {C}.\n{1}, {T}, Sacrifice this artifact: Draw a card.",
        mana: Some((ManaColor::Colorless, 1)), spell: None
    ),
    card_artifact!(
        HedronArchive, "Hedron Archive", cost: Some("{4}"), type_line: "Artifact",
        oracle: "{T}: Add {C}{C}.\n{2}, {T}, Sacrifice this artifact: Draw two cards.",
        mana: Some((ManaColor::Colorless, 2)), spell: None
    ),
    card!(
        BrightstoneRitual, "Brightstone Ritual", cost: Some("{R}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Add {R} for each Goblin on the battlefield.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: true, effects: &[Effect::AddManaForEachGoblin] })
    ),
    card!(
        ManaGeyser, "Mana Geyser", cost: Some("{3}{R}{R}"), kinds: SORCERY, basic: None,
        type_line: "Sorcery", power: None, toughness: None,
        oracle: "Add {R} for each tapped land your opponents control.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: false, effects: &[Effect::AddManaForEachTappedOpponentLand] })
    ),
    card!(
        RiteOfFlame, "Rite of Flame", cost: Some("{R}"), kinds: SORCERY, basic: None,
        type_line: "Sorcery", power: None, toughness: None,
        oracle: "Add {R}{R}, then add {R} for each card named Rite of Flame in each graveyard.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: false, effects: &[Effect::AddManaForRiteOfFlame] })
    ),
    card!(
        Swamp, "Swamp", cost: None, kinds: LAND, basic: Some(ManaColor::Black),
        type_line: "Basic Land — Swamp", power: None, toughness: None,
        oracle: "({T}: Add {B}.)", spell: None
    ),
    card!(
        LightningBolt, "Lightning Bolt", cost: Some("{R}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Lightning Bolt deals 3 damage to any target.",
        spell: Some(SpellSpec { target: TargetSpec::AnyTarget, instant: true, effects: STRIKE })
    ),
    card!(
        ForestBear, "Forest Bear", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(2), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        StripedBears, "Striped Bears", cost: Some("{3}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(2), toughness: Some(2),
        oracle: "When this creature enters, draw a card.", spell: None
    ),
    card!(
        Guttersnipe, "Guttersnipe", cost: Some("{2}{R}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Goblin Shaman", power: Some(2), toughness: Some(2),
        oracle: "Whenever you cast an instant or sorcery spell, this creature deals 2 damage to each opponent.",
        spell: None
    ),
    card!(
        TatyovaBenthicDruid, "Tatyova, Benthic Druid", cost: Some("{3}{G}{U}"), kinds: CREATURE, basic: None,
        type_line: "Legendary Creature — Merfolk Druid", power: Some(3), toughness: Some(3),
        oracle: "Landfall — Whenever a land you control enters, you gain 1 life and draw a card.",
        spell: None
    ),
    card_defender!(
        JaddiOffshoot, "Jaddi Offshoot", cost: Some("{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Plant", power: Some(0), toughness: Some(3),
        oracle: "Defender\nLandfall — Whenever a land you control enters, you gain 1 life.",
        spell: None
    ),
    card!(
        TalrandSkySummoner, "Talrand, Sky Summoner", cost: Some("{2}{U}{U}"), kinds: CREATURE, basic: None,
        type_line: "Legendary Creature — Merfolk Wizard", power: Some(2), toughness: Some(2),
        oracle: "Whenever you cast an instant or sorcery spell, create a 2/2 blue Drake creature token with flying.",
        spell: None
    ),
    // Issue 17 slice 2: Krenko goblin-token package. All text is recorded from
    // the stored Scryfall printings in fixtures/starter-corpus/planned/.
    card!(
        KrenkoMobBoss, "Krenko, Mob Boss", cost: Some("{2}{R}{R}"), kinds: CREATURE, basic: None,
        type_line: "Legendary Creature — Goblin Warrior", power: Some(3), toughness: Some(3),
        oracle: "{T}: Create X 1/1 red Goblin creature tokens, where X is the number of Goblins you control.",
        spell: None
    ),
    card!(
        DragonFodder, "Dragon Fodder", cost: Some("{1}{R}"), kinds: SORCERY, basic: None,
        type_line: "Sorcery", power: None, toughness: None,
        oracle: "Create two 1/1 red Goblin creature tokens.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: false, effects: &[Effect::CreateGoblinTokens(2)] })
    ),
    card!(
        KrenkosCommand, "Krenko's Command", cost: Some("{1}{R}"), kinds: SORCERY, basic: None,
        type_line: "Sorcery", power: None, toughness: None,
        oracle: "Create two 1/1 red Goblin creature tokens.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: false, effects: &[Effect::CreateGoblinTokens(2)] })
    ),
    card!(
        HordelingOutburst, "Hordeling Outburst", cost: Some("{1}{R}{R}"), kinds: SORCERY, basic: None,
        type_line: "Sorcery", power: None, toughness: None,
        oracle: "Create three 1/1 red Goblin creature tokens.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: false, effects: &[Effect::CreateGoblinTokens(3)] })
    ),
    card!(
        GoblinInstigator, "Goblin Instigator", cost: Some("{1}{R}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Goblin Rogue", power: Some(1), toughness: Some(1),
        oracle: "When this creature enters, create a 1/1 red Goblin creature token.",
        spell: None
    ),
    card!(
        BeetlebackChief, "Beetleback Chief", cost: Some("{2}{R}{R}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Goblin Warrior", power: Some(2), toughness: Some(2),
        oracle: "When this creature enters, create two 1/1 red Goblin creature tokens.",
        spell: None
    ),
    // Issue 17 starter corpus: sourced vanilla green/white creatures. Every
    // entry below prints no rules text, so the plain permanent-spell path is the
    // whole implementation.
    card!(
        AlabornTrooper, "Alaborn Trooper", cost: Some("{2}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Human Soldier", power: Some(2), toughness: Some(3),
        oracle: "", spell: None
    ),
    card!(
        ArmoredWarhorse, "Armored Warhorse", cost: Some("{W}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Horse", power: Some(2), toughness: Some(3),
        oracle: "", spell: None
    ),
    card!(
        BastionEnforcer, "Bastion Enforcer", cost: Some("{2}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Dwarf Soldier", power: Some(3), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        BladeOfTheSixthPride, "Blade of the Sixth Pride", cost: Some("{1}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Cat Rebel", power: Some(3), toughness: Some(1),
        oracle: "", spell: None
    ),
    card!(
        DevotedHero, "Devoted Hero", cost: Some("{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Elf Soldier", power: Some(1), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        EagerCadet, "Eager Cadet", cost: Some("{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Human Soldier", power: Some(1), toughness: Some(1),
        oracle: "", spell: None
    ),
    card!(
        EliteVanguard, "Elite Vanguard", cost: Some("{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Human Soldier", power: Some(2), toughness: Some(1),
        oracle: "", spell: None
    ),
    card!(
        GlorySeeker, "Glory Seeker", cost: Some("{1}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Human Soldier", power: Some(2), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        IndomitableAncients, "Indomitable Ancients", cost: Some("{2}{W}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Treefolk Warrior", power: Some(2), toughness: Some(10),
        oracle: "", spell: None
    ),
    card!(
        KnightErrant, "Knight Errant", cost: Some("{1}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Human Knight", power: Some(2), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        LoomingAltisaur, "Looming Altisaur", cost: Some("{3}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Dinosaur", power: Some(1), toughness: Some(7),
        oracle: "", spell: None
    ),
    card!(
        LoxodonLineBreaker, "Loxodon Line Breaker", cost: Some("{2}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Elephant Soldier", power: Some(3), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        OreskosSwiftclaw, "Oreskos Swiftclaw", cost: Some("{1}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Cat Warrior", power: Some(3), toughness: Some(1),
        oracle: "", spell: None
    ),
    card!(
        PearledUnicorn, "Pearled Unicorn", cost: Some("{2}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Unicorn", power: Some(2), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        PillarfieldOx, "Pillarfield Ox", cost: Some("{3}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Ox", power: Some(2), toughness: Some(4),
        oracle: "", spell: None
    ),
    card!(
        RaptorCompanion, "Raptor Companion", cost: Some("{1}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Dinosaur", power: Some(3), toughness: Some(1),
        oracle: "", spell: None
    ),
    card!(
        SavannahLions, "Savannah Lions", cost: Some("{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Cat", power: Some(2), toughness: Some(1),
        oracle: "", spell: None
    ),
    card!(
        SiegeMastodon, "Siege Mastodon", cost: Some("{4}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Elephant", power: Some(3), toughness: Some(5),
        oracle: "", spell: None
    ),
    card!(
        SilentArtisan, "Silent Artisan", cost: Some("{3}{W}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Giant", power: Some(3), toughness: Some(5),
        oracle: "", spell: None
    ),
    card!(
        SilvercoatLion, "Silvercoat Lion", cost: Some("{1}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Cat", power: Some(2), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        StaunchShieldmate, "Staunch Shieldmate", cost: Some("{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Dwarf Soldier", power: Some(1), toughness: Some(3),
        oracle: "", spell: None
    ),
    card!(
        ValiantGuard, "Valiant Guard", cost: Some("{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Human Soldier", power: Some(0), toughness: Some(3),
        oracle: "", spell: None
    ),
    card!(
        YokedOx, "Yoked Ox", cost: Some("{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Ox", power: Some(0), toughness: Some(4),
        oracle: "", spell: None
    ),
    card!(
        AlphaTyrranax, "Alpha Tyrranax", cost: Some("{4}{G}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Dinosaur Beast", power: Some(6), toughness: Some(5),
        oracle: "", spell: None
    ),
    card!(
        AlpineGrizzly, "Alpine Grizzly", cost: Some("{2}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(4), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        AxebaneBeast, "Axebane Beast", cost: Some("{3}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Beast", power: Some(3), toughness: Some(4),
        oracle: "", spell: None
    ),
    card!(
        BalduvianBears, "Balduvian Bears", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(2), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        BearCub, "Bear Cub", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(2), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        BlanchwoodTreefolk, "Blanchwood Treefolk", cost: Some("{4}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Treefolk", power: Some(4), toughness: Some(5),
        oracle: "", spell: None
    ),
    card!(
        CylianElf, "Cylian Elf", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Elf Scout", power: Some(2), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        DurkwoodBoars, "Durkwood Boars", cost: Some("{4}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Boar", power: Some(4), toughness: Some(4),
        oracle: "", spell: None
    ),
    card!(
        EnormousBaloth, "Enormous Baloth", cost: Some("{6}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Beast", power: Some(7), toughness: Some(7),
        oracle: "", spell: None
    ),
    card!(
        FeralKrushok, "Feral Krushok", cost: Some("{4}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Beast", power: Some(5), toughness: Some(4),
        oracle: "", spell: None
    ),
    card!(
        GoldenBear, "Golden Bear", cost: Some("{3}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(4), toughness: Some(3),
        oracle: "", spell: None
    ),
    card!(
        HollowhengeBeast, "Hollowhenge Beast", cost: Some("{3}{G}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Beast", power: Some(5), toughness: Some(5),
        oracle: "", spell: None
    ),
    card!(
        KalonianTusker, "Kalonian Tusker", cost: Some("{G}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Beast", power: Some(3), toughness: Some(3),
        oracle: "", spell: None
    ),
    card!(
        NessianCourser, "Nessian Courser", cost: Some("{2}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Centaur Warrior", power: Some(3), toughness: Some(3),
        oracle: "", spell: None
    ),
    card!(
        NorwoodRanger, "Norwood Ranger", cost: Some("{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Elf Scout Ranger", power: Some(1), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        SwordwiseCentaur, "Swordwise Centaur", cost: Some("{G}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Centaur Warrior", power: Some(3), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        TerrainElemental, "Terrain Elemental", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Elemental", power: Some(3), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        TrainedArmodon, "Trained Armodon", cost: Some("{1}{G}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Elephant", power: Some(3), toughness: Some(3),
        oracle: "", spell: None
    ),
    card!(
        TrainedJackal, "Trained Jackal", cost: Some("{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Jackal", power: Some(1), toughness: Some(2),
        oracle: "", spell: None
    ),
    card!(
        VastwoodGorger, "Vastwood Gorger", cost: Some("{5}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Wurm", power: Some(5), toughness: Some(6),
        oracle: "", spell: None
    ),
    card!(
        WillowElf, "Willow Elf", cost: Some("{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Elf", power: Some(1), toughness: Some(1),
        oracle: "", spell: None
    ),
    card!(
        ArmoredWolfRider, "Armored Wolf-Rider", cost: Some("{3}{G}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Elf Knight", power: Some(4), toughness: Some(6),
        oracle: "", spell: None
    ),
    card!(
        GrizzledLeotau, "Grizzled Leotau", cost: Some("{G}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Cat", power: Some(1), toughness: Some(5),
        oracle: "", spell: None
    ),
    card!(
        Watchwolf, "Watchwolf", cost: Some("{G}{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Wolf", power: Some(3), toughness: Some(3),
        oracle: "", spell: None
    ),
    // Issue 17 first supported commander: a genuine printed vanilla legendary
    // creature. It is registered in `CardId::SUPPORTED_COMMANDERS`, never as an
    // ordinary library card.
    card!(
        JasmineBoreal, "Jasmine Boreal", cost: Some("{3}{G}{W}"), kinds: CREATURE, basic: None,
        type_line: "Legendary Creature — Human", power: Some(4), toughness: Some(5),
        oracle: "", spell: None
    ),
    card!(
        AyulaQueenAmongBears, "Ayula, Queen Among Bears", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Legendary Creature — Bear", power: Some(2), toughness: Some(2),
        oracle: "Whenever another Bear you control enters, choose one —\n• Put two +1/+1 counters on target Bear.\n• Target Bear you control fights target creature you don't control.",
        spell: None
    ),
    // Issue 17 slice 4: sourced Ayula/Isamaru package. Ordinary Bear is the
    // last printed vanilla creature left in the eight desired lists; Ashcoat
    // Bear and Owlbear are the only two cards those lists print with nothing
    // but Flash or Trample plus an already-modelled effect; Soul Warden is the
    // first card in the pool with an "another creature enters" trigger.
    card!(
        OrdinaryBear, "Ordinary Bear", cost: Some("{3}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(4), toughness: Some(5),
        oracle: "", spell: None
    ),
    card_flash!(
        AshcoatBear, "Ashcoat Bear", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(2), toughness: Some(2),
        oracle: "Flash (You may cast this spell any time you could cast an instant.)",
        spell: None
    ),
    card_trample!(
        Owlbear, "Owlbear", cost: Some("{3}{G}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bird Bear", power: Some(4), toughness: Some(4),
        oracle: "Trample\nKeen Senses — When this creature enters, draw a card.",
        spell: None
    ),
    card!(
        SoulWarden, "Soul Warden", cost: Some("{W}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Human Cleric", power: Some(1), toughness: Some(1),
        oracle: "Whenever another creature enters, you gain 1 life.", spell: None
    ),
    card!(
        StitchersSupplier, "Stitcher's Supplier", cost: Some("{B}"), kinds: CREATURE,
        basic: None,
        type_line: "Creature — Zombie", power: Some(1), toughness: Some(1),
        oracle: "When this creature enters or dies, mill three cards. (Put the top three cards of your library into your graveyard.)",
        spell: None
    ),
    card!(
        RuinCrab, "Ruin Crab", cost: Some("{U}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Crab", power: Some(0), toughness: Some(3),
        oracle: "Landfall — Whenever a land you control enters, each opponent mills three cards. (To mill a card, a player puts the top card of their library into their graveyard.)",
        spell: None
    ),
    card!(
        Negate, "Negate", cost: Some("{1}{U}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Counter target noncreature spell.",
        spell: Some(SpellSpec {
            target: TargetSpec::NoncreatureSpell, instant: true, effects: COUNTER
        })
    ),
    card!(
        Preordain, "Preordain", cost: Some("{U}"), kinds: SORCERY, basic: None,
        type_line: "Sorcery", power: None, toughness: None,
        oracle: "Scry 2, then draw a card. (To scry 2, look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)",
        spell: Some(SpellSpec {
            target: TargetSpec::None, instant: false, effects: PREORDAIN_SCRY_TWO_DRAW
        })
    ),
    card!(
        PyreticRitual, "Pyretic Ritual", cost: Some("{1}{R}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Add {R}{R}{R}.",
        spell: Some(SpellSpec {
            target: TargetSpec::None, instant: true, effects: ADD_THREE_RED
        })
    ),
    card!(
        SeethingSong, "Seething Song", cost: Some("{2}{R}"), kinds: INSTANT, basic: None,
        type_line: "Instant", power: None, toughness: None,
        oracle: "Add {R}{R}{R}{R}{R}.",
        spell: Some(SpellSpec {
            target: TargetSpec::None, instant: true, effects: ADD_FIVE_RED
        })
    ),
    // Issue 17 slice 6: sourced death-trigger and target-player package. All
    // three creatures print nothing but already-modelled behaviour, so no new
    // keyword is claimed: Blood Artist and Zulaport Cutthroat use the new
    // "creature dies" watchers plus life loss (which is not damage), and
    // Hedron Crab is the first "target player" mill.
    card!(
        BloodArtist, "Blood Artist", cost: Some("{1}{B}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Vampire", power: Some(0), toughness: Some(1),
        oracle: "Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.",
        spell: None
    ),
    card!(
        ZulaportCutthroat, "Zulaport Cutthroat", cost: Some("{1}{B}"), kinds: CREATURE,
        basic: None,
        type_line: "Creature — Human Rogue Ally", power: Some(1), toughness: Some(1),
        oracle: "Whenever this creature or another creature you control dies, each opponent loses 1 life and you gain 1 life.",
        spell: None
    ),
    card!(
        HedronCrab, "Hedron Crab", cost: Some("{U}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Crab", power: Some(0), toughness: Some(2),
        oracle: "Landfall — Whenever a land you control enters, target player mills three cards. (They put the top three cards of their library into their graveyard.)",
        spell: None
    ),
    card_islandwalk!(
        PaleBears, "Pale Bears", cost: Some("{2}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(2), toughness: Some(2),
        oracle: "Islandwalk (This creature can't be blocked as long as defending player controls an Island.)",
        spell: None
    ),
    card_islandwalk!(
        RiverBear, "River Bear", cost: Some("{3}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(3), toughness: Some(3),
        oracle: "Islandwalk (This creature can't be blocked as long as defending player controls an Island.)",
        spell: None
    ),
    // Issue 17 slice 19: the pool's first attack trigger. Spectral Bears prints
    // an intervening-if ("if defending player controls no black nontoken
    // permanents") whose payoff is "it doesn't untap during your next untap
    // step". No keyword, layer, targeting or cost change is involved.
    card!(
        SpectralBears, "Spectral Bears", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear Spirit", power: Some(3), toughness: Some(3),
        oracle: "Whenever this creature attacks, if defending player controls no black nontoken permanents, it doesn't untap during your next untap step.",
        spell: None
    ),
    // Issue 17 slice 15: sourced Bear-token package. Each card prints one
    // activated ability whose cost is paid with a card (a land from hand, two
    // cards from the graveyard, or the card itself from the graveyard) and
    // which creates 2/2 green Bear tokens. Ayula's Influence and Bearscape are
    // the pool's first enchantment permanents (CR 303); they are noncreature
    // permanents with no continuous effect, so the layer system never applies
    // to them.
    card!(
        AyulasInfluence, "Ayula's Influence", cost: Some("{G}{G}{G}"), kinds: ENCHANTMENT,
        basic: None,
        type_line: "Enchantment", power: None, toughness: None,
        oracle: "Discard a land card: Create a 2/2 green Bear creature token.",
        spell: None
    ),
    card!(
        Bearscape, "Bearscape", cost: Some("{1}{G}{G}"), kinds: ENCHANTMENT, basic: None,
        type_line: "Enchantment", power: None, toughness: None,
        oracle: "{1}{G}, Exile two cards from your graveyard: Create a 2/2 green Bear creature token.",
        spell: None
    ),
    card!(
        MotherBear, "Mother Bear", cost: Some("{1}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(2), toughness: Some(2),
        oracle: "{3}{G}{G}, Exile this card from your graveyard: Create two 2/2 green Bear creature tokens. Activate only as a sorcery.",
        spell: None
    ),
    // Issue 17 slice 16: two more sourced Ayula-list cards. Fade from History
    // needs one mass destruction and a per-controller token; Little Bear needs
    // a targeted untap whose counter is conditional on the target's subtype.
    // Both printed texts are copied verbatim from the printings recorded in
    // fixtures/starter-corpus/slice16-sourced.json.
    card!(
        FadeFromHistory, "Fade from History", cost: Some("{2}{G}{G}"), kinds: SORCERY, basic: None,
        type_line: "Sorcery", power: None, toughness: None,
        oracle: "Each player who controls an artifact or enchantment creates a 2/2 green Bear creature token. Then destroy all artifacts and enchantments.",
        spell: Some(SpellSpec { target: TargetSpec::None, instant: false, effects: &[Effect::BearTokenForEachPlayerWithPermanent, Effect::DestroyAllArtifactsAndEnchantments] })
    ),
    card_flash!(
        LittleBear, "Little Bear", cost: Some("{2}{G}"), kinds: CREATURE, basic: None,
        type_line: "Creature — Bear", power: Some(3), toughness: Some(2),
        oracle: "Flash\nWhen this creature enters, untap another target creature you control. If that creature is a Bear, put a +1/+1 counter on it.",
        spell: None
    ),
];

pub fn all() -> &'static [CardDef] {
    &CARDS
}

/// Token definitions. Deliberately separate from `CARDS`: tokens are engine
/// objects, never importable or analyzer-supported cards.
pub static TOKENS: [CardDef; 5] = [
    CardDef {
        id: CardId::GoblinToken,
        name: "Goblin Token",
        cost: None,
        kinds: CREATURE,
        basic_mana: None,
        artifact_mana: None,
        type_line: "Token Creature — Goblin",
        colors: &[ManaColor::Red],
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
        id: CardId::DrakeToken,
        name: "Drake Token",
        cost: None,
        kinds: CREATURE,
        basic_mana: None,
        artifact_mana: None,
        type_line: "Token Creature — Drake",
        colors: &[ManaColor::Blue],
        flying: true,
        defender: false,
        flash: false,
        trample: false,
        islandwalk: false,
        power: Some(2),
        toughness: Some(2),
        oracle: "Flying",
        spell: None,
    },
    CardDef {
        id: CardId::BeastToken,
        name: "Beast Token",
        cost: None,
        kinds: CREATURE,
        basic_mana: None,
        artifact_mana: None,
        type_line: "Token Creature — Beast",
        colors: &[ManaColor::Green],
        flying: false,
        defender: false,
        flash: false,
        trample: false,
        islandwalk: false,
        power: Some(4),
        toughness: Some(4),
        oracle: "",
        spell: None,
    },
    CardDef {
        id: CardId::PestToken,
        name: "Pest Token",
        cost: None,
        kinds: CREATURE,
        basic_mana: None,
        artifact_mana: None,
        type_line: "Token Creature — Pest",
        colors: &[ManaColor::Black, ManaColor::Green],
        flying: false,
        defender: false,
        flash: false,
        trample: false,
        islandwalk: false,
        power: Some(1),
        toughness: Some(1),
        oracle: "When this token dies, you gain 1 life.",
        spell: None,
    },
    CardDef {
        id: CardId::BearToken,
        name: "Bear Token",
        cost: None,
        kinds: CREATURE,
        basic_mana: None,
        artifact_mana: None,
        type_line: "Token Creature — Bear",
        colors: &[ManaColor::Green],
        flying: false,
        defender: false,
        flash: false,
        trample: false,
        islandwalk: false,
        power: Some(2),
        toughness: Some(2),
        oracle: "",
        spell: None,
    },
];

pub fn def(id: CardId) -> &'static CardDef {
    CARDS
        .iter()
        .chain(TOKENS.iter())
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
