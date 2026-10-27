use serde::{Deserialize, Serialize};

use crate::layer::SubtypeLine;

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
pub enum ManaChoice {
    CommanderIdentity,
    OpponentLandColors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    Land,
    Creature,
    Instant,
    Sorcery,
    Artifact,
    /// CR 303: an enchantment permanent. Slice 15 adds the first two pool
    /// cards of this kind (Ayula's Influence, Bearscape); they behave as
    /// ordinary noncreature permanents with activated abilities and are not
    /// creatures, so nothing in combat or the layer system treats them
    /// specially.
    Enchantment,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Land => "Land",
            Kind::Creature => "Creature",
            Kind::Instant => "Instant",
            Kind::Sorcery => "Sorcery",
            Kind::Artifact => "Artifact",
            Kind::Enchantment => "Enchantment",
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
    /// A creature controlled by the effect's controller ("target creature you
    /// control"), excluding the ability's own source when the printed wording
    /// says "another" (Little Bear).
    CreatureYouControl,
    Spell,
    /// CR 115.1 + 205.2a: a spell on the stack that is not a creature spell
    /// (Negate). A creature spell on the stack is not a legal target.
    NoncreatureSpell,
    /// CR 115.1: a Bear on the battlefield, either controller.
    Bear,
    /// A Bear controlled by the effect's controller, including the source.
    BearYouControl,
    /// A Human controlled by the effect's controller.
    HumanYouControl,
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
    /// Untap every nonland permanent the effect's controller controls
    /// (Dramatic Reversal).
    UntapAllNonlandsYouControl,
    /// Add {R} for each Goblin on the battlefield (Brightstone Ritual).
    AddManaForEachGoblin,
    /// Add {R} for each tapped land an opponent controls (Mana Geyser).
    AddManaForEachTappedOpponentLand,
    /// Add {R}{R}, then {R} for each Rite of Flame card in all graveyards.
    AddManaForRiteOfFlame,
    /// Put one +1/+1 counter on the ability's source (landfall Sazh's Chocobo).
    PutPlusOneCounterOnSource,
    /// Create one 4/4 green Beast token (Rampaging Baloths).
    CreateBeastToken,
    /// Create one 1/1 black and green Pest token with its dies trigger.
    CreatePestToken,
    /// Create N 2/2 green Bear creature tokens (Ayula's Influence, Bearscape,
    /// Mother Bear).
    CreateBearTokens(u8),
    /// "Each player who controls an artifact or enchantment creates a 2/2 green
    /// Bear creature token" (Fade from History). The token's controller is the
    /// seat that controlled the permanent, not the spell's controller.
    BearTokenForEachPlayerWithPermanent,
    /// "Then destroy all artifacts and enchantments" (Fade from History). One
    /// simultaneous event (CR 704.3): every death is recorded before the
    /// dies-watcher triggers are dispatched for the whole batch.
    DestroyAllArtifactsAndEnchantments,
    /// "Untap another target creature you control. If that creature is a Bear,
    /// put a +1/+1 counter on it" (Little Bear). The counter is conditional on
    /// the target's own *effective* subtype — `Game::has_subtype`, so a layer 4
    /// type change counts — never on the source's subtype.
    UntapTargetAndCounterIfBear,
    /// Each live player draws N cards in APNAP order from the active player.
    EachPlayerDraws(u8),
    /// Put N +1/+1 counters on the target creature.
    PutPlusOneCountersOnTarget(u32),
    /// Create one 2/2 green Bear token per nontoken creature its controller
    /// owned that was put into their graveyard from the battlefield this turn.
    CreateBearTokensForEachDeathThisTurn,
    /// CR 701.14a: two target creatures deal damage equal to their power to
    /// each other simultaneously. Both must still satisfy the printed
    /// Bear/opponent relation at resolution or no damage is dealt.
    FightTargets,
    /// "It doesn't untap during your next untap step" (Spectral Bears). The
    /// source object is recorded in `GameState::skip_next_untap` and its
    /// controller's next untap step leaves it tapped exactly once.
    SkipNextUntapStep,
    /// Look at the top card without moving it. A creature/planeswalker card
    /// gets a take-or-bottom choice; any other card goes to the bottom without
    /// being revealed. Empty library is not a draw.
    LookTopTakeOrBottom,
    /// CR 701.19a: create a regeneration shield on the ability's source.
    RegenerateSource,
    /// "Put X +1/+1 counters on target creature you control, where X is that
    /// spell's mana value" (Dancing from Dark to Dawn). The triggering creature
    /// spell's mana value travels with the trigger (`TriggerRef` →
    /// `StackItem::spell_mana_value`) so resolution reads the value the spell
    /// had when it was cast, never the board's current state.
    PutPlusOneCountersOnTargetForTriggerSpellManaValue,
    /// "Each creature you control with power N or greater gets +P/+T and gains
    /// trample until end of turn" (Goreclaw, Terror of Qal Sisma). Power is the
    /// permanent's *current* power at resolution (CR 613.4c results included),
    /// and the trample grant is a layer 6 continuous effect with the same
    /// until-end-of-turn duration as the pump.
    BoostAndTrampleEachCreatureYouControlWithPowerAtLeast {
        minimum_power: i32,
        power: i32,
        toughness: i32,
    },
    /// Put a +1/+1 counter on and grant haste to the entered object captured
    /// by the trigger (`it`, not a target).
    PutCounterAndHasteOnEntered,
    /// Set the target Human's base P/T to the ability source's effective P/T
    /// captured at resolution (or its LKI if the source departed).
    SetBasePowerToughnessFromSource,
    /// CR 613.1d: the ability's own source becomes this subtype line until end
    /// of turn, replacing its printed subtypes (Ursine Champion). A layer 4
    /// continuous effect, so `Game::has_subtype` answers for it everywhere the
    /// printed type line used to be read directly.
    SetSourceCreatureSubtypes(SubtypeLine),
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
            Effect::UntapAllNonlandsYouControl => {
                "untap all nonland permanents you control".to_string()
            }
            Effect::AddManaForEachGoblin => {
                "add {R} for each Goblin on the battlefield".to_string()
            }
            Effect::AddManaForEachTappedOpponentLand => {
                "add {R} for each tapped land an opponent controls".to_string()
            }
            Effect::AddManaForRiteOfFlame => {
                "add {R}{R}, then {R} per graveyard Rite of Flame".to_string()
            }
            Effect::MillSelf(n) => format!("mill {n}"),
            Effect::MillEachOpponent(n) => format!("each opponent mills {n}"),
            Effect::AddMana { color, amount } => {
                format!("add {amount} {} mana", color.name())
            }
            Effect::MillTargetPlayer(n) => format!("target player mills {n}"),
            Effect::LoseLife(n) => format!("the target player loses {n} life"),
            Effect::LoseLifeEachOpponent(n) => format!("each opponent loses {n} life"),
            Effect::PutPlusOneCounterOnSource => "put a +1/+1 counter on this creature".to_string(),
            Effect::CreateBeastToken => "create a 4/4 green Beast token".to_string(),
            Effect::CreatePestToken => "create a 1/1 black and green Pest token".to_string(),
            Effect::CreateBearTokens(n) => format!("create {n} 2/2 green Bear token(s)"),
            Effect::BearTokenForEachPlayerWithPermanent => {
                "each player who controls an artifact or enchantment creates a 2/2 green Bear token"
                    .to_string()
            }
            Effect::DestroyAllArtifactsAndEnchantments => {
                "destroy all artifacts and enchantments".to_string()
            }
            Effect::EachPlayerDraws(n) => format!("each player draws {n}"),
            Effect::PutPlusOneCountersOnTarget(n) => {
                format!("put {n} +1/+1 counter(s) on the target creature")
            }
            Effect::FightTargets => {
                "target Bear you control fights target creature you don't control".to_string()
            }
            Effect::RegenerateSource => "regenerate this creature".to_string(),
            Effect::SetBasePowerToughnessFromSource => {
                "set target Human's base P/T to this creature's P/T".to_string()
            }
            Effect::PutCounterAndHasteOnEntered => {
                "put a +1/+1 counter on it and it gains haste".to_string()
            }
            Effect::LookTopTakeOrBottom => {
                "look at the top card; take a creature/planeswalker or put it on the bottom"
                    .to_string()
            }
            Effect::SkipNextUntapStep => "it doesn't untap during your next untap step".to_string(),
            Effect::CreateBearTokensForEachDeathThisTurn => {
                "create a 2/2 green Bear token per nontoken creature death this turn".to_string()
            }
            Effect::UntapTargetAndCounterIfBear => {
                "untap the target creature; if it is a Bear, put a +1/+1 counter on it".to_string()
            }
            Effect::PutPlusOneCountersOnTargetForTriggerSpellManaValue => {
                "put X +1/+1 counters on the target creature, X = that spell's mana value"
                    .to_string()
            }
            Effect::BoostAndTrampleEachCreatureYouControlWithPowerAtLeast {
                minimum_power,
                power,
                toughness,
            } => format!(
                "each creature you control with power {minimum_power} or greater gets \
                 {power:+}/{toughness:+} and gains trample until end of turn"
            ),
            Effect::SetSourceCreatureSubtypes(line) => {
                format!("this creature becomes a {} until end of turn", line.text())
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
    /// CR 605.1a mana ability on a nonland artifact: (color, amount), tap cost.
    pub artifact_mana: Option<(ManaColor, u8)>,
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
    /// CR 702.14a: printed islandwalk. A blocking restriction, executed by
    /// `tokens::can_block` through the keyword table; nothing grants it.
    pub islandwalk: bool,
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

    pub fn is_artifact(&self) -> bool {
        self.kinds.contains(&Kind::Artifact)
    }

    /// CR 303.1: an enchantment card. Enchantments are permanents but never
    /// creatures, so combat, blocking and the layer system ignore them.
    pub fn is_enchantment(&self) -> bool {
        self.kinds.contains(&Kind::Enchantment)
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

    /// CR 702.14a: "This creature can't be blocked as long as defending
    /// player controls an Island."
    pub fn has_islandwalk(&self) -> bool {
        self.islandwalk
    }

    /// CR 202.2: the card is black. Tokens carry their colors in
    /// [`CardDef::colors`]; for catalog cards the printed color is derived from
    /// the mana cost, which is what CR 202.2a makes authoritative. Nothing in
    /// the pool sets, adds or changes a permanent's color, so this predicate is
    /// the entire color model Spectral Bears reads.
    pub fn is_black(&self) -> bool {
        if self.colors.contains(&ManaColor::Black) {
            return true;
        }
        self.cost.is_some_and(|cost| {
            crate::mana::parse_cost(cost)
                .pips
                .contains(&ManaColor::Black)
        })
    }

    /// CR 117.1a: an instant, or a card that prints Flash, may be cast while
    /// any player has priority.
    pub fn casts_at_instant_timing(&self) -> bool {
        self.is_instant() || self.flash
    }

    /// The Goblin creature subtype (CR 205.3m), read from the printed type
    /// line. Krenko counts Goblins by this predicate, tokens included.
    pub fn is_goblin(&self) -> bool {
        self.has_subtype("Goblin")
    }

    /// A creature/land subtype (CR 205.3) from the printed type line, which is
    /// written `Types — Subtype Subtype`. Landwalk (CR 702.14b) and tribal
    /// counts both read subtypes here rather than special-casing a card name.
    pub fn has_subtype(&self, subtype: &str) -> bool {
        self.type_line
            .split_once('—')
            .is_some_and(|(_, subtypes)| subtypes.split_whitespace().any(|s| s == subtype))
    }

    /// An Island land (CR 305.6), read from the subtype so a nonbasic land with
    /// the Island subtype counts and a card merely named "Island" elsewhere does
    /// not. Islandwalk asks the defending player's battlefield for this.
    pub fn is_island(&self) -> bool {
        self.is_land() && self.has_subtype("Island")
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

    /// CR 202.3: a card's mana value is the total amount of mana in its mana
    /// cost. Read from the printed cost string: a numeric generic symbol adds
    /// that number, `{X}` adds 0 (its value is fixed only while the spell is on
    /// the stack, and no X-cost spell in this pool is a creature), a hybrid or
    /// Phyrexian symbol adds its numeric half when it prints one and otherwise
    /// 1, and any other single symbol adds 1.
    pub fn mana_value(&self) -> u8 {
        self.cost.map_or(0, cost_mana_value)
    }

    /// Spell characteristics used for casting and resolution. A vanilla
    /// creature is a permanent spell with no targets and no effects (CR 608.3).
    pub fn spell_spec(&self) -> Option<SpellSpec> {
        match self.spell {
            Some(spec) => Some(spec),
            None if self.is_creature() || self.is_artifact() || self.is_enchantment() => {
                Some(SpellSpec {
                    target: TargetSpec::None,
                    instant: false,
                    effects: NO_EFFECTS,
                })
            }
            None => None,
        }
    }

    /// A creature spell becomes a permanent on resolution rather than going to
    /// the graveyard (CR 608.3).
    pub fn is_permanent_spell(&self) -> bool {
        self.is_creature() || self.is_artifact() || self.is_enchantment()
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
    DramaticReversal,
    GoblinWarStrike,
    SolRing,
    MindStone,
    HedronArchive,
    ArcaneSignet,
    FellwarStone,
    Werebear,
    CommandersSphere,
    FriendlyTeddy,
    UlvenwaldBear,
    CallerOfTheClaw,
    AyulaQueenAmongBears,
    ViviensGrizzly,
    DragonScarredBear,
    SurrakAndGoreclaw,
    ExuberantWolfbear,
    AyulasInfluence,
    Bearscape,
    MotherBear,
    FadeFromHistory,
    LittleBear,
    DancingFromDarkToDawn,
    GoreclawTerrorOfQalSisma,
    UrsineChampion,
    SazhsChocobo,
    RampagingBaloths,
    ProfessorOfZoomancy,
    BeastToken,
    PestToken,
    BearToken,
    BrightstoneRitual,
    ManaGeyser,
    RiteOfFlame,
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
    PaleBears,
    RiverBear,
    SpectralBears,
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
    CardId::AyulaQueenAmongBears,
    CardId::KrenkoMobBoss,
    CardId::TalrandSkySummoner,
    CardId::TatyovaBenthicDruid,
];

/// Token cards exist as engine objects but are never importable: `catalog::all`
/// and `catalog::by_name` deliberately exclude them, and the deck parser
/// refuses them like any other unknown name.
pub const TOKEN_CARDS: &[CardId] = &[
    CardId::GoblinToken,
    CardId::DrakeToken,
    CardId::BeastToken,
    CardId::PestToken,
    CardId::BearToken,
];

impl CardId {
    /// Printed color identity of the supported commanders, in canonical WUBRG
    /// order. Known only for the explicit registry; unknown cards return empty.
    pub fn commander_color_identity(self) -> &'static [ManaColor] {
        match self {
            CardId::IsamaruHoundOfKonda => &[ManaColor::White],
            CardId::JasmineBoreal => &[ManaColor::White, ManaColor::Green],
            CardId::KrenkoMobBoss => &[ManaColor::Red],
            CardId::TalrandSkySummoner => &[ManaColor::Blue],
            CardId::TatyovaBenthicDruid => &[ManaColor::Blue, ManaColor::Green],
            _ => &[],
        }
    }

    pub fn dynamic_mana(self) -> Option<ManaChoice> {
        match self {
            CardId::ArcaneSignet => Some(ManaChoice::CommanderIdentity),
            CardId::FellwarStone => Some(ManaChoice::OpponentLandColors),
            CardId::CommandersSphere => Some(ManaChoice::CommanderIdentity),
            _ => None,
        }
    }

    pub fn creature_mana(self) -> Option<ManaColor> {
        match self {
            CardId::Werebear => Some(ManaColor::Green),
            _ => None,
        }
    }

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
            CardId::SoldierToken
                | CardId::SpiritToken
                | CardId::GoblinToken
                | CardId::DrakeToken
                | CardId::BeastToken
                | CardId::PestToken
                | CardId::BearToken
        )
    }
}

impl std::fmt::Display for CardId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// CR 202.3: mana value of a printed mana cost string, without needing a
/// `CardDef`. Exposed separately from `CardDef::mana_value` so the rule can be
/// tested on symbol shapes the current pool does not print (hybrid, Phyrexian,
/// snow), instead of adding fixture cards only a test would ever use.
///
/// Each symbol counts as:
/// * a pure number — that number (`{3}` = 3);
/// * `{X}` — 0, because an X outside the stack is 0 (and no X-cost spell in the
///   pool is a creature, so no supported card reads this while X is chosen);
/// * a hybrid/Phyrexian symbol with a numeric half — that number (`{2/W}` = 2);
/// * any other hybrid or Phyrexian symbol — 1 (`{G/W}`, `{W/P}`);
/// * any other single symbol — 1 (`{G}`, `{S}`, `{C}`).
pub fn cost_mana_value(cost: &str) -> u8 {
    cost.split('{')
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(symbol, _)| symbol))
        .map(|symbol| {
            if let Ok(generic) = symbol.parse::<u8>() {
                generic
            } else if symbol.eq_ignore_ascii_case("X") {
                0
            } else if let Some((half, _)) = symbol.split_once('/') {
                half.parse::<u8>().unwrap_or(1)
            } else {
                1
            }
        })
        .sum()
}
