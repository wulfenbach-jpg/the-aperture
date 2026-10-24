use serde::{Deserialize, Serialize};

const NO_EFFECTS: &[Effect] = &[];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ManaColor {
    White,
    Blue,
    Black,
    Red,
    Green,
    Colorless,
}

impl ManaColor {
    pub fn name(self) -> &'static str {
        match self {
            ManaColor::White => "white",
            ManaColor::Blue => "blue",
            ManaColor::Black => "black",
            ManaColor::Red => "red",
            ManaColor::Green => "green",
            ManaColor::Colorless => "colorless",
        }
    }

    /// Mana symbol as printed.
    pub fn symbol(self) -> &'static str {
        match self {
            ManaColor::White => "{W}",
            ManaColor::Blue => "{U}",
            ManaColor::Black => "{B}",
            ManaColor::Red => "{R}",
            ManaColor::Green => "{G}",
            ManaColor::Colorless => "{C}",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    Land,
    Creature,
    Instant,
    Sorcery,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Land => "Land",
            Kind::Creature => "Creature",
            Kind::Instant => "Instant",
            Kind::Sorcery => "Sorcery",
        }
    }
}

/// What a spell requires as a target. The slice supports at most one target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TargetSpec {
    None,
    /// "any target": a player or a creature (CR 115.4 covers the wider set).
    AnyTarget,
    Creature,
    Spell,
}

/// The complete set of effects this slice implements. Anything not here is not
/// playable, and cards using it are refused by the deck importer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect {
    Draw(u8),
    Scry(u8),
    Surveil(u8),
    /// Deal N damage to the spell's target.
    Damage(u8),
    CounterTargetSpell,
    /// Target creature gets +P/+T until end of turn.
    Pump(i32, i32),
    /// Each player simultaneously sacrifices a permanent of their choice.
    EachPlayerSacrifice,
    /// Return the target creature to its owner's hand.
    ReturnToHand,
    /// Put the target creature on top of its owner's library.
    PutOnTopOfLibrary,
    /// Exile the target creature, then its controller gains life equal to its
    /// power as last known (Swords to Plowshares).
    ExileAndGainLife,
    /// Full combat damage prevention for the rest of this turn (Fog). Partial
    /// prevention and redirection are not modelled.
    PreventAllCombatDamageThisTurn,
}

impl Effect {
    pub fn describe(self) -> String {
        match self {
            Effect::Draw(n) => format!("draw {n}"),
            Effect::Scry(n) => format!("scry {n}"),
            Effect::Surveil(n) => format!("surveil {n}"),
            Effect::Damage(n) => format!("deal {n} damage to the target"),
            Effect::CounterTargetSpell => "counter target spell".to_string(),
            Effect::Pump(p, t) => format!("target creature gets {p:+}/{t:+} until end of turn"),
            Effect::EachPlayerSacrifice => "each player sacrifices a permanent".to_string(),
            Effect::ReturnToHand => "return the target creature to its owner's hand".to_string(),
            Effect::PutOnTopOfLibrary => {
                "put the target creature on top of its owner's library".to_string()
            }
            Effect::ExileAndGainLife => {
                "exile the target creature; its controller gains life equal to its power"
                    .to_string()
            }
            Effect::PreventAllCombatDamageThisTurn => {
                "prevent all combat damage that would be dealt this turn".to_string()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SpellSpec {
    pub target: TargetSpec,
    pub instant: bool,
    /// Applied in order (CR 608.2c).
    pub effects: &'static [Effect],
}

/// Static catalog entry. Not `Deserialize`: the catalog is compiled in and
/// referenced by `CardId`.
#[derive(Debug, Clone, Copy)]
pub struct CardDef {
    pub id: CardId,
    pub name: &'static str,
    /// `None` for lands (which have no mana cost).
    pub cost: Option<&'static str>,
    pub kinds: &'static [Kind],
    /// Set for basic lands: the single mana the land's mana ability produces.
    pub basic_mana: Option<ManaColor>,
    pub type_line: &'static str,
    pub power: Option<i32>,
    pub toughness: Option<i32>,
    pub oracle: &'static str,
    pub spell: Option<SpellSpec>,
}

impl CardDef {
    pub fn is_land(&self) -> bool {
        self.kinds.contains(&Kind::Land)
    }

    pub fn is_creature(&self) -> bool {
        self.kinds.contains(&Kind::Creature)
    }

    pub fn is_instant(&self) -> bool {
        self.kinds.contains(&Kind::Instant)
    }

    pub fn is_sorcery(&self) -> bool {
        self.kinds.contains(&Kind::Sorcery)
    }

    /// Non-instant spells are cast at sorcery timing (CR 117.1a).
    pub fn needs_sorcery_timing(&self) -> bool {
        !self.is_instant() && !self.is_land()
    }

    pub fn mana_cost(&self) -> Option<crate::mana::ManaCost> {
        self.cost.map(crate::mana::parse_cost)
    }

    /// Spell characteristics used for casting and resolution. A vanilla
    /// creature is a permanent spell with no targets and no effects (CR 608.3).
    pub fn spell_spec(&self) -> Option<SpellSpec> {
        match self.spell {
            Some(spec) => Some(spec),
            None if self.is_creature() => Some(SpellSpec {
                target: TargetSpec::None,
                instant: false,
                effects: NO_EFFECTS,
            }),
            None => None,
        }
    }

    /// A creature spell becomes a permanent on resolution rather than going to
    /// the graveyard (CR 608.3).
    pub fn is_permanent_spell(&self) -> bool {
        self.is_creature()
    }
}

/// Card identity inside the supported pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CardId {
    Island,
    Mountain,
    Forest,
    GrizzlyBears,
    RuneclawBear,
    CentaurCourser,
    CrawWurm,
    HillGiant,
    GiantGrowth,
    Shock,
    LightningStrike,
    Counterspell,
    Divination,
    Opt,
    Consider,
    CrackTheEarth,
    Plains,
    IsamaruHoundOfKonda,
    Unsummon,
    TimeEbb,
    SwordsToPlowshares,
    Fog,
}

impl CardId {
    pub fn def(self) -> &'static CardDef {
        crate::catalog::def(self)
    }

    pub fn name(self) -> &'static str {
        self.def().name
    }

    pub fn mana_cost(self) -> Option<crate::mana::ManaCost> {
        self.def().mana_cost()
    }

    /// Explicit command-zone practice allow-list. This is deliberately narrower
    /// than CR 903.3: no general legendary/Vehicle/Spacecraft support exists in
    /// this slice.
    pub fn is_supported_commander(self) -> bool {
        matches!(self, CardId::IsamaruHoundOfKonda)
    }
}

impl std::fmt::Display for CardId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
