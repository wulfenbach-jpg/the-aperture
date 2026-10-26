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
    /// CR 115.1 + 115.4: a player ("target player"). Every live seat is a legal
    /// target, including the ability's own controller (Hedron Crab, Blood
    /// Artist).
    Player,
    Creature,
    /// A creature controlled by an opponent of the effect's controller.
    OpponentCreature,
    Spell,
    /// CR 115.1 + 205.2a: a spell on the stack that is not a creature spell
    /// (Negate). A creature spell on the stack is not a legal target.
    NoncreatureSpell,
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
    /// The controller of the source gains N life (Brindle Boar).
    GainLife(i32),
    /// The source itself gets +P/+T until end of turn (Rootwalla: a pump with
    /// no target).
    PumpSource(i32, i32),
    /// CR 615: prevent the next N damage that would be dealt to the target this
    /// turn. A one-shot shield: once it has absorbed N damage it stops applying
    /// and is removed in cleanup (CR 514.2).
    PreventNextDamage(u32),
    /// Create N 1/1 red Goblin creature tokens controlled by the source's
    /// controller (CR 111.2, 608.2c). The token card is never importable.
    CreateGoblinTokens(u8),
    /// Create one 1/1 red Goblin creature token for each Goblin the source's
    /// controller controls as this resolves (Krenko, Mob Boss).
    CreateGoblinTokensForEachGoblin,
    /// Create one 2/2 blue Drake creature token with flying (Talrand).
    CreateDrakeToken,
    /// Deal N damage to each opponent of the ability's controller, excluding
    /// the controller and eliminated seats (Guttersnipe).
    DamageEachOpponent(u8),
    /// CR 701.13a: the effect's controller mills N cards — the top N cards of
    /// their library go to their graveyard (Stitcher's Supplier).
    MillSelf(u8),
    /// CR 701.13a: each opponent of the effect's controller mills N cards.
    /// Eliminated seats are skipped, and the controller never mills.
    MillEachOpponent(u8),
    /// CR 106.4: add N mana of one colour to the effect controller's mana pool
    /// (Pyretic Ritual, Seething Song). No permanent is tapped.
    AddMana {
        color: ManaColor,
        amount: u8,
    },
    /// CR 701.13a: the *target player* mills N cards (Hedron Crab).
    MillTargetPlayer(u8),
    /// CR 118.2 + 119.3: the *target player* loses N life. This is life loss,
    /// not damage: nothing is dealt, so damage prevention and damage triggers
    /// never apply (Blood Artist).
    LoseLife(i32),
    /// CR 118.2 + 119.3: each opponent of the effect's controller loses N life
    /// (Zulaport Cutthroat). Loss of life is not damage.
    LoseLifeEachOpponent(i32),
    /// Damage to the spell's target creature equal to the number of Goblins its
    /// controller controls (Volley Veteran).
    DamageTargetCreatureForEachGoblin,
    /// Damage to the spell's target player equal to the number of Goblins its
    /// controller controls (Goblin War Strike).
    DamageTargetPlayerForEachGoblin,
    /// Return every attacking creature to its owner's hand (Aetherize).
    ReturnAllAttackersToHand,
    /// Untap every nonland permanent the effect's controller controls
    /// (Dramatic Reversal).
    UntapAllNonlandsYouControl,
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
            Effect::GainLife(n) => format!("its controller gains {n} life"),
            Effect::PumpSource(p, t) => {
                format!("this permanent gets {p:+}/{t:+} until end of turn")
            }
            Effect::PreventNextDamage(n) => {
                format!("prevent the next {n} damage that would be dealt to the target this turn")
            }
            Effect::CreateGoblinTokens(n) => format!("create {n} 1/1 red Goblin tokens"),
            Effect::CreateGoblinTokensForEachGoblin => {
                "create a 1/1 red Goblin token for each Goblin you control".to_string()
            }
            Effect::CreateDrakeToken => "create a 2/2 blue Drake token with flying".to_string(),
            Effect::DamageEachOpponent(n) => format!("deal {n} damage to each opponent"),
            Effect::DamageTargetCreatureForEachGoblin => {
                "deal damage to the target creature equal to Goblins you control".to_string()
            }
            Effect::DamageTargetPlayerForEachGoblin => {
                "deal damage to the target player equal to Goblins you control".to_string()
            }
            Effect::ReturnAllAttackersToHand => {
                "return all attacking creatures to hand".to_string()
            }
            Effect::UntapAllNonlandsYouControl => {
                "untap all nonland permanents you control".to_string()
            }
            Effect::MillSelf(n) => format!("mill {n}"),
            Effect::MillEachOpponent(n) => format!("each opponent mills {n}"),
            Effect::AddMana { color, amount } => {
                format!("add {amount} {} mana", color.name())
            }
            Effect::MillTargetPlayer(n) => format!("target player mills {n}"),
            Effect::LoseLife(n) => format!("the target player loses {n} life"),
            Effect::LoseLifeEachOpponent(n) => format!("each opponent loses {n} life"),
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
    /// Printed colors. Tokens carry theirs here; catalog cards leave it empty
    /// because the engine only needs it for token characteristics in this slice.
    pub colors: &'static [ManaColor],
    pub flying: bool,
    pub defender: bool,
    /// CR 702.8a: "You may cast this spell any time you could cast an instant."
    /// Only printed Flash is modelled; nothing grants it to another card.
    pub flash: bool,
    /// CR 702.19b: excess combat damage may be assigned to the defending
    /// player once every blocker has been assigned lethal damage.
    pub trample: bool,
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

    pub fn has_flying(&self) -> bool {
        self.flying
    }

    pub fn has_defender(&self) -> bool {
        self.defender
    }

    pub fn has_flash(&self) -> bool {
        self.flash
    }

    pub fn has_trample(&self) -> bool {
        self.trample
    }

    /// CR 117.1a: an instant, or a card that prints Flash, may be cast while
    /// any player has priority.
    pub fn casts_at_instant_timing(&self) -> bool {
        self.is_instant() || self.flash
    }

    /// The Goblin creature subtype (CR 205.3m), read from the printed type
    /// line. Krenko counts Goblins by this predicate, tokens included.
    pub fn is_goblin(&self) -> bool {
        self.type_line
            .split_once('—')
            .is_some_and(|(_, subtypes)| subtypes.split_whitespace().any(|s| s == "Goblin"))
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
    ProdigalSorcerer,
    Rootwalla,
    BrindleBoar,
    ElvishVisionary,
    FesteringGoblin,
    Oculus,
    MendingHands,
    AngrathsMarauders,
    WightOfPrecinctSix,
    KrenkoMobBoss,
    TalrandSkySummoner,
    TatyovaBenthicDruid,
    JaddiOffshoot,
    Swamp,
    LightningBolt,
    ForestBear,
    StripedBears,
    Guttersnipe,
    SoulsAttendant,
    RazorclawBear,
    VolleyVeteran,
    Aetherize,
    DramaticReversal,
    GoblinWarStrike,
    DrakeToken,
    DragonFodder,
    KrenkosCommand,
    HordelingOutburst,
    GoblinInstigator,
    BeetlebackChief,
    GoblinToken,
    AlabornTrooper,
    ArmoredWarhorse,
    BastionEnforcer,
    BladeOfTheSixthPride,
    DevotedHero,
    EagerCadet,
    EliteVanguard,
    GlorySeeker,
    IndomitableAncients,
    KnightErrant,
    LoomingAltisaur,
    LoxodonLineBreaker,
    OreskosSwiftclaw,
    PearledUnicorn,
    PillarfieldOx,
    RaptorCompanion,
    SavannahLions,
    SiegeMastodon,
    SilentArtisan,
    SilvercoatLion,
    StaunchShieldmate,
    ValiantGuard,
    YokedOx,
    AlphaTyrranax,
    AlpineGrizzly,
    AxebaneBeast,
    BalduvianBears,
    BearCub,
    BlanchwoodTreefolk,
    CylianElf,
    DurkwoodBoars,
    EnormousBaloth,
    FeralKrushok,
    GoldenBear,
    HollowhengeBeast,
    KalonianTusker,
    NessianCourser,
    NorwoodRanger,
    SwordwiseCentaur,
    TerrainElemental,
    TrainedArmodon,
    TrainedJackal,
    VastwoodGorger,
    WillowElf,
    ArmoredWolfRider,
    GrizzledLeotau,
    Watchwolf,
    JasmineBoreal,
    OrdinaryBear,
    AshcoatBear,
    Owlbear,
    SoulWarden,
    StitchersSupplier,
    RuinCrab,
    Negate,
    Preordain,
    PyreticRitual,
    SeethingSong,
    ZulaportCutthroat,
    BloodArtist,
    HedronCrab,
    /// 1/1 vanilla Soldier token (CR 111). Never importable.
    SoldierToken,
    /// 1/1 flying Spirit token (CR 111, 702.9).
    SpiritToken,
}

/// Explicit command-zone practice allow-list. This is deliberately narrower
/// than CR 903.3: no general legendary/Vehicle/Spacecraft support exists in
/// this slice. Every entry is a genuine printed vanilla legendary creature; new
/// commanders are added here and to the catalog, never inferred from a type
/// line. Commander-only cards are refused in ordinary starting libraries
/// because the legend rule is not modelled.
pub const SUPPORTED_COMMANDERS: &[CardId] = &[
    CardId::IsamaruHoundOfKonda,
    CardId::JasmineBoreal,
    CardId::KrenkoMobBoss,
    CardId::TalrandSkySummoner,
    CardId::TatyovaBenthicDruid,
];

/// Token cards exist as engine objects but are never importable: `catalog::all`
/// and `catalog::by_name` deliberately exclude them, and the deck parser
/// refuses them like any other unknown name.
pub const TOKEN_CARDS: &[CardId] = &[CardId::GoblinToken, CardId::DrakeToken];

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

    pub fn is_supported_commander(self) -> bool {
        SUPPORTED_COMMANDERS.contains(&self)
    }

    /// True for a token's synthetic characteristic card (CR 111.1: a token is
    /// not a card, but it needs characteristics and an identity).
    pub fn is_token(self) -> bool {
        matches!(
            self,
            CardId::SoldierToken | CardId::SpiritToken | CardId::GoblinToken | CardId::DrakeToken
        )
    }
}

impl std::fmt::Display for CardId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
