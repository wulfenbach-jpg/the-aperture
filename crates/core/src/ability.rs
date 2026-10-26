//! Typed activated and triggered abilities (CR 602, 603).
//!
//! The ability model is a rules-layer record: costs, targets, modes and the
//! card the ability belongs to are declared here, independently of any UI or
//! AI decision policy. Decision policies only ever see the engine's
//! [`crate::action::Action`] values; they never supply rules code.
//!
//! Supported in this slice (only abilities printed on cards in the catalog):
//! * activated abilities with a `{T}` cost, a mana cost, or "Sacrifice this
//!   creature" as the cost, each with an optional once-each-turn restriction;
//! * enters-the-battlefield and "dies" triggered abilities, mandatory or
//!   optional ("you may").
//!
//! Not implemented, and not claimed: mana abilities on nonland permanents,
//! intervening-"if" clauses, delayed or reflexive triggers, and control
//! changes. Static abilities are declared here only for the two printed forms
//! the supported pool actually uses: a characteristic-defining power/toughness
//! ability (CR 604.3, layer 7a) and a damage-doubling replacement effect
//! (CR 614.1a).

use crate::card::{CardId, Effect, TargetSpec};
use crate::layer::Layer;

/// The event that makes a triggered ability trigger.
///
/// Deliberately small: every variant here corresponds to a real trigger on a
/// card in the supported pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerEvent {
    /// CR 603.6a: the source permanent entered the battlefield.
    EntersBattlefield,
    /// CR 700.4 with CR 603.6c: the source permanent was put into a graveyard
    /// from the battlefield ("dies").
    Dies,
    /// CR 603.6a + 601.2: the controller cast an instant or sorcery spell.
    SpellCast,
    /// CR 603.6a: a land entered under this source's controller.
    LandEnters,
    /// CR 509.3c: this creature became blocked.
    BecomesBlocked,
    /// CR 603.6a with the printed word "another" (CR 603.6a + 109.5): a
    /// creature other than the source entered the battlefield. The entering
    /// permanent's controller is irrelevant; the trigger's controller is the
    /// source permanent's controller.
    AnotherCreatureEnters,
    /// CR 603.6c + 700.4 with the printed wording "creature dies" (no
    /// controller restriction): any creature anywhere died, including the
    /// source itself (Blood Artist).
    CreatureDies,
    /// CR 603.6c + 700.4 with the printed wording "this creature or another
    /// creature you control dies": a creature the watcher's controller
    /// controlled died. The dying creature may be the watcher itself
    /// (Zulaport Cutthroat).
    CreatureYouControlDies,
    /// Same wording but strictly "another creature you control dies": the
    /// dying creature must not be the watcher itself (Elas il-Kor).
    AnotherCreatureYouControlDies,
}

impl TriggerEvent {
    pub fn name(self) -> &'static str {
        match self {
            TriggerEvent::EntersBattlefield => "enters the battlefield",
            TriggerEvent::Dies => "dies",
            TriggerEvent::SpellCast => "you cast an instant or sorcery spell",
            TriggerEvent::LandEnters => "a land you control enters",
            TriggerEvent::BecomesBlocked => "this creature becomes blocked",
            TriggerEvent::AnotherCreatureEnters => "another creature enters",
            TriggerEvent::CreatureDies => "a creature dies",
            TriggerEvent::CreatureYouControlDies => {
                "this creature or another creature you control dies"
            }
            TriggerEvent::AnotherCreatureYouControlDies => "another creature you control dies",
        }
    }
}

/// What activating an ability costs (CR 602.2b, 118.4).
///
/// The three components are independent so a future card can combine them; the
/// current pool uses exactly one per ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityCost {
    /// `{T}`: tap the source (CR 107.5). A creature with this cost obeys the
    /// CR 302.6 summoning-sickness restriction.
    pub tap: bool,
    /// "Sacrifice this permanent": the source is put into its owner's graveyard
    /// as part of paying the cost.
    pub sacrifice_source: bool,
    /// Mana cost as printed, paid from the pool and untapped basic lands.
    pub mana: Option<&'static str>,
    /// "Discard a land card": the activating player discards one land card from
    /// their hand as part of paying the cost (CR 118.3, 602.2b). The card is
    /// chosen by the activator and named in the action, never by the engine.
    pub discard_land: bool,
    /// "Exile N cards from your graveyard": the activating player exiles that
    /// many cards from their own graveyard as part of the cost.
    pub exile_graveyard_cards: u8,
    /// "Exile this card from your graveyard": the ability is activated from the
    /// graveyard and the source card itself is exiled to pay the cost, so the
    /// source is already in exile when the ability is put on the stack.
    pub exile_source_from_graveyard: bool,
}

impl AbilityCost {
    /// No cost at all: every component off.
    pub const NONE: AbilityCost = AbilityCost {
        tap: false,
        sacrifice_source: false,
        mana: None,
        discard_land: false,
        exile_graveyard_cards: 0,
        exile_source_from_graveyard: false,
    };
    pub const TAP: AbilityCost = AbilityCost {
        tap: true,
        ..AbilityCost::NONE
    };
    pub const SACRIFICE_SELF: AbilityCost = AbilityCost {
        sacrifice_source: true,
        ..AbilityCost::NONE
    };
    pub const fn mana(cost: &'static str) -> AbilityCost {
        AbilityCost {
            mana: Some(cost),
            ..AbilityCost::NONE
        }
    }

    /// How many cards the activator must name to pay this cost. The engine
    /// validates the named cards against the source's zone rules; it never
    /// picks them itself.
    pub fn card_cost_count(self) -> usize {
        usize::from(self.discard_land) + self.exile_graveyard_cards as usize
    }

    pub fn is_free(self) -> bool {
        !self.tap
            && !self.sacrifice_source
            && self.mana.is_none()
            && self.card_cost_count() == 0
            && !self.exile_source_from_graveyard
    }

    /// Printed cost text, e.g. `"{T}"`, `"{1}{G}"`, `"Sacrifice this permanent"`.
    pub fn describe(self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.tap {
            parts.push("{T}".to_string());
        }
        if let Some(mana) = self.mana {
            parts.push(mana.to_string());
        }
        if self.sacrifice_source {
            parts.push("Sacrifice this permanent".to_string());
        }
        if self.discard_land {
            parts.push("Discard a land card".to_string());
        }
        if self.exile_graveyard_cards > 0 {
            parts.push(format!(
                "Exile {} card(s) from your graveyard",
                self.exile_graveyard_cards
            ));
        }
        if self.exile_source_from_graveyard {
            parts.push("Exile this card from your graveyard".to_string());
        }
        if parts.is_empty() {
            "no cost".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// One mode of an ability. A card with several modes ("Choose one —") would
/// carry several entries; the current pool has single-mode abilities only, and
/// the mode index is still part of the action so a caller can never smuggle a
/// mode the card does not print.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbilityMode {
    pub text: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivatedAbility {
    /// Printed ability text, the human-readable source of truth.
    pub text: &'static str,
    pub cost: AbilityCost,
    pub target: TargetSpec,
    pub modes: &'static [AbilityMode],
    /// Applied in order, exactly like a spell's instructions (CR 608.2c).
    pub effects: &'static [Effect],
    /// "Activate only once each turn" (CR 602.5b restriction).
    pub once_per_turn: bool,
    /// "Activate only as a sorcery" (CR 307.5): the ability may be activated
    /// only during its controller's main phase, while the stack is empty.
    pub sorcery_timing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggeredAbility {
    pub text: &'static str,
    pub event: TriggerEvent,
    /// `true` for "you may" abilities (CR 603.5): the ability still goes on
    /// the stack, and the choice is made when it resolves.
    pub optional: bool,
    pub target: TargetSpec,
    pub effects: &'static [Effect],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ability {
    Activated(ActivatedAbility),
    Triggered(TriggeredAbility),
}

impl Ability {
    pub fn text(&self) -> &'static str {
        match self {
            Ability::Activated(a) => a.text,
            Ability::Triggered(t) => t.text,
        }
    }

    pub fn activated(&self) -> Option<&ActivatedAbility> {
        match self {
            Ability::Activated(a) => Some(a),
            Ability::Triggered(_) => None,
        }
    }

    pub fn triggered(&self) -> Option<&TriggeredAbility> {
        match self {
            Ability::Triggered(t) => Some(t),
            Ability::Activated(_) => None,
        }
    }

    pub fn target_spec(&self) -> TargetSpec {
        match self {
            Ability::Activated(a) => a.target,
            Ability::Triggered(t) => t.target,
        }
    }

    pub fn effects(&self) -> &'static [Effect] {
        match self {
            Ability::Activated(a) => a.effects,
            Ability::Triggered(t) => t.effects,
        }
    }
}

const NO_ABILITIES: &[Ability] = &[];

const SINGLE_MODE: &[AbilityMode] = &[AbilityMode {
    text: "the ability's only mode",
}];

const TORCHER: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{T}: Prodigal Sorcerer deals 1 damage to any target.",
    cost: AbilityCost::TAP,
    target: TargetSpec::AnyTarget,
    modes: SINGLE_MODE,
    effects: &[Effect::Damage(1)],
    once_per_turn: false,
    sorcery_timing: false,
})];

const ROOTWALLA: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{1}{G}: Rootwalla gets +2/+2 until end of turn. Activate only once each turn.",
    cost: AbilityCost::mana("{1}{G}"),
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::PumpSource(2, 2)],
    once_per_turn: true,
    sorcery_timing: false,
})];

const BRINDLE_BOAR: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "Sacrifice Brindle Boar: You gain 4 life.",
    cost: AbilityCost::SACRIFICE_SELF,
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::GainLife(4)],
    once_per_turn: false,
    sorcery_timing: false,
})];

const ELVISH_VISIONARY: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When Elvish Visionary enters the battlefield, draw a card.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::Draw(1)],
})];

const FESTERING_GOBLIN: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When Festering Goblin dies, target creature gets -1/-1 until end of turn.",
    event: TriggerEvent::Dies,
    optional: false,
    target: TargetSpec::Creature,
    effects: &[Effect::Pump(-1, -1)],
})];

const OCULUS: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When Oculus dies, you may draw a card.",
    event: TriggerEvent::Dies,
    optional: true,
    target: TargetSpec::None,
    effects: &[Effect::Draw(1)],
})];

const STRIPED_BEARS: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When this creature enters, draw a card.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::Draw(1)],
})];

const GUTTERSNIPE: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever you cast an instant or sorcery spell, this creature deals 2 damage to each opponent.",
    event: TriggerEvent::SpellCast,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::DamageEachOpponent(2)],
})];

const SOULS_ATTENDANT: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever another creature enters, you may gain 1 life.",
    event: TriggerEvent::AnotherCreatureEnters,
    optional: true,
    target: TargetSpec::None,
    effects: &[Effect::GainLife(1)],
})];

const RAZORCLAW_BEAR: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever this creature becomes blocked, it gets +2/+2 until end of turn.",
    event: TriggerEvent::BecomesBlocked,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::PumpSource(2, 2)],
})];

const VOLLEY_VETERAN: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When this creature enters, it deals damage to target creature an opponent controls equal to the number of Goblins you control.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::OpponentCreature,
    effects: &[Effect::DamageTargetCreatureForEachGoblin],
})];

const SAZH: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Landfall — Whenever a land you control enters, put a +1/+1 counter on this creature.",
    event: TriggerEvent::LandEnters,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::PutPlusOneCounterOnSource],
})];

const RAMPAGING_BALOTHS: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Landfall — Whenever a land you control enters, create a 4/4 green Beast creature token.",
    event: TriggerEvent::LandEnters,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::CreateBeastToken],
})];

const PROFESSOR: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When this creature enters, create a 1/1 black and green Pest creature token with \"When this token dies, you gain 1 life.\"",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::CreatePestToken],
})];

const PEST_DIES: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When this token dies, you gain 1 life.",
    event: TriggerEvent::Dies,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::GainLife(1)],
})];

const COMMANDERS_SPHERE: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "Sacrifice this artifact: Draw a card.",
    cost: AbilityCost::SACRIFICE_SELF,
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::Draw(1)],
    once_per_turn: false,
    sorcery_timing: false,
})];

const MIND_STONE: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{1}, {T}, Sacrifice this artifact: Draw a card.",
    cost: AbilityCost {
        tap: true,
        sacrifice_source: true,
        mana: Some("{1}"),
        ..AbilityCost::NONE
    },
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::Draw(1)],
    once_per_turn: false,
    sorcery_timing: false,
})];

const HEDRON_ARCHIVE: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{2}, {T}, Sacrifice this artifact: Draw two cards.",
    cost: AbilityCost {
        tap: true,
        sacrifice_source: true,
        mana: Some("{2}"),
        ..AbilityCost::NONE
    },
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::Draw(2)],
    once_per_turn: false,
    sorcery_timing: false,
})];

const TATYOVA: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Landfall — Whenever a land you control enters, you gain 1 life and draw a card.",
    event: TriggerEvent::LandEnters,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::GainLife(1), Effect::Draw(1)],
})];

const JADDI: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Landfall — Whenever a land you control enters, you gain 1 life.",
    event: TriggerEvent::LandEnters,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::GainLife(1)],
})];

const TALRAND: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever you cast an instant or sorcery spell, create a 2/2 blue Drake creature token with flying.",
    event: TriggerEvent::SpellCast,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::CreateDrakeToken],
})];

const KRENKO: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{T}: Create X 1/1 red Goblin creature tokens, where X is the number of Goblins you control.",
    cost: AbilityCost::TAP,
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::CreateGoblinTokensForEachGoblin],
    once_per_turn: false,
    sorcery_timing: false,
})];

const GOBLIN_INSTIGATOR: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When this creature enters, create a 1/1 red Goblin creature token.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::CreateGoblinTokens(1)],
})];

const BEETLEBACK_CHIEF: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When this creature enters, create two 1/1 red Goblin creature tokens.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::CreateGoblinTokens(2)],
})];

const OWLBEAR: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Keen Senses — When this creature enters, draw a card.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::Draw(1)],
})];

const SOUL_WARDEN: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever another creature enters, you gain 1 life.",
    event: TriggerEvent::AnotherCreatureEnters,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::GainLife(1)],
})];

/// Ayula's Influence: the first pool card whose cost is paid with a card from
/// hand. The land is named by the activator in `cost_cards`; the engine only
/// checks that it is a land card in that player's hand.
const AYULAS_INFLUENCE: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "Discard a land card: Create a 2/2 green Bear creature token.",
    cost: AbilityCost {
        discard_land: true,
        ..AbilityCost::NONE
    },
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::CreateBearTokens(1)],
    once_per_turn: false,
    sorcery_timing: false,
})];

/// Bearscape: a mana cost plus two named cards from the activator's own
/// graveyard, exiled as the cost (CR 118.3). Nothing about the cost reads the
/// cards' types, so any two cards qualify.
const BEARSCAPE: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{1}{G}, Exile two cards from your graveyard: Create a 2/2 green Bear creature token.",
    cost: AbilityCost {
        mana: Some("{1}{G}"),
        exile_graveyard_cards: 2,
        ..AbilityCost::NONE
    },
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::CreateBearTokens(1)],
    once_per_turn: false,
    sorcery_timing: false,
})];

/// Mother Bear: activated from the graveyard, exiling the card itself as the
/// cost. "Activate only as a sorcery" is modelled as the CR 307.5 timing
/// restriction on the activated ability.
const MOTHER_BEAR: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{3}{G}{G}, Exile this card from your graveyard: Create two 2/2 green Bear creature tokens. Activate only as a sorcery.",
    cost: AbilityCost {
        mana: Some("{3}{G}{G}"),
        exile_source_from_graveyard: true,
        ..AbilityCost::NONE
    },
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::CreateBearTokens(2)],
    once_per_turn: false,
    sorcery_timing: true,
})];

/// "When this creature enters or dies, mill three cards." One printed ability
/// with two trigger events is modelled as two `TriggeredAbility` entries on the
/// same card (CR 603.6a for enters, CR 700.4/603.6c for dies); the printed text
/// is carried on both so neither entry claims wording the card does not print.
const STITCHERS_SUPPLIER: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "When this creature enters or dies, mill three cards.",
        event: TriggerEvent::EntersBattlefield,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::MillSelf(3)],
    }),
    Ability::Triggered(TriggeredAbility {
        text: "When this creature enters or dies, mill three cards.",
        event: TriggerEvent::Dies,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::MillSelf(3)],
    }),
];

const RUIN_CRAB: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Landfall — Whenever a land you control enters, each opponent mills three cards.",
    event: TriggerEvent::LandEnters,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::MillEachOpponent(3)],
})];

const BLOOD_ARTIST: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever this creature or another creature dies, target player loses 1 life and you gain 1 life.",
    event: TriggerEvent::CreatureDies,
    optional: false,
    target: TargetSpec::Player,
    effects: &[Effect::LoseLife(1), Effect::GainLife(1)],
})];

const ZULAPORT_CUTTHROAT: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever this creature or another creature you control dies, each opponent loses 1 life and you gain 1 life.",
    event: TriggerEvent::CreatureYouControlDies,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::LoseLifeEachOpponent(1), Effect::GainLife(1)],
})];

const HEDRON_CRAB: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Landfall — Whenever a land you control enters, target player mills three cards.",
    event: TriggerEvent::LandEnters,
    optional: false,
    target: TargetSpec::Player,
    effects: &[Effect::MillTargetPlayer(3)],
})];

/// Every ability the rules layer knows for a card, in printed order.
pub fn abilities(card: CardId) -> &'static [Ability] {
    match card {
        CardId::StitchersSupplier => STITCHERS_SUPPLIER,
        CardId::RuinCrab => RUIN_CRAB,
        CardId::BloodArtist => BLOOD_ARTIST,
        CardId::ZulaportCutthroat => ZULAPORT_CUTTHROAT,
        CardId::HedronCrab => HEDRON_CRAB,
        CardId::Owlbear => OWLBEAR,
        CardId::SoulWarden => SOUL_WARDEN,
        CardId::SazhsChocobo => SAZH,
        CardId::RampagingBaloths => RAMPAGING_BALOTHS,
        CardId::ProfessorOfZoomancy => PROFESSOR,
        CardId::PestToken => PEST_DIES,
        CardId::CommandersSphere => COMMANDERS_SPHERE,
        CardId::AyulasInfluence => AYULAS_INFLUENCE,
        CardId::Bearscape => BEARSCAPE,
        CardId::MotherBear => MOTHER_BEAR,
        CardId::MindStone => MIND_STONE,
        CardId::HedronArchive => HEDRON_ARCHIVE,
        CardId::SoulsAttendant => SOULS_ATTENDANT,
        CardId::RazorclawBear => RAZORCLAW_BEAR,
        CardId::VolleyVeteran => VOLLEY_VETERAN,
        CardId::StripedBears => STRIPED_BEARS,
        CardId::Guttersnipe => GUTTERSNIPE,
        CardId::TatyovaBenthicDruid => TATYOVA,
        CardId::JaddiOffshoot => JADDI,
        CardId::TalrandSkySummoner => TALRAND,
        CardId::KrenkoMobBoss => KRENKO,
        CardId::GoblinInstigator => GOBLIN_INSTIGATOR,
        CardId::BeetlebackChief => BEETLEBACK_CHIEF,
        CardId::ProdigalSorcerer => TORCHER,
        CardId::Rootwalla => ROOTWALLA,
        CardId::BrindleBoar => BRINDLE_BOAR,
        CardId::ElvishVisionary => ELVISH_VISIONARY,
        CardId::FesteringGoblin => FESTERING_GOBLIN,
        CardId::Oculus => OCULUS,
        _ => NO_ABILITIES,
    }
}

/// Triggered abilities on `card` that fire for `event`.
pub fn triggers_on(
    card: CardId,
    event: TriggerEvent,
) -> impl Iterator<Item = (u8, &'static TriggeredAbility)> {
    abilities(card)
        .iter()
        .enumerate()
        .filter_map(move |(index, ability)| match ability {
            Ability::Triggered(t) if t.event == event => Some((index as u8, t)),
            _ => None,
        })
}

// ------------------------------------------------------- static abilities

/// What a layered static power/toughness ability counts. Each variant exists
/// because a supported card prints it; adding a card with a different count
/// means adding a variant here, not approximating it in the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CountSource {
    /// "for each creature card in your opponents' graveyards"
    /// (Wight of Precinct Six — a layer 7c static, *not* characteristic-defining).
    CreatureCardsInOpponentsGraveyards,
}

/// A printed static ability that produces a continuous or replacement effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaticAbility {
    /// CR 613.4c: "gets +P/+T for each ..." — an additive modifier that applies
    /// for as long as the source is on the battlefield. It is a layer 7c
    /// effect, so it stacks with until-end-of-turn modifiers instead of
    /// setting a base value.
    PowerToughnessPerCount {
        count: CountSource,
        power: i32,
        toughness: i32,
        text: &'static str,
    },
    /// CR 614.1a: "If a source you control would deal damage to a permanent or
    /// player, it deals double that damage instead." The doubled source may be
    /// a permanent (a creature's combat or ability damage) or a spell on the
    /// stack (Shock), because the printed word is "source", not "creature".
    DamageFromControlledSources { factor: u32, text: &'static str },
    /// Threshold +P/+T while the controller has at least this many cards in
    /// their graveyard (Werebear).
    ThresholdPowerToughness {
        minimum: usize,
        power: i32,
        toughness: i32,
        text: &'static str,
    },
}

impl StaticAbility {
    pub fn text(self) -> &'static str {
        match self {
            StaticAbility::PowerToughnessPerCount { text, .. } => text,
            StaticAbility::DamageFromControlledSources { text, .. }
            | StaticAbility::ThresholdPowerToughness { text, .. } => text,
        }
    }

    /// Layer this ability's continuous effect belongs to. A replacement effect
    /// is layerless: it never applies in a layer, so `None`.
    pub fn layer(self) -> Option<Layer> {
        match self {
            StaticAbility::PowerToughnessPerCount { .. } => Some(Layer::Modifying),
            StaticAbility::DamageFromControlledSources { .. }
            | StaticAbility::ThresholdPowerToughness { .. } => None,
        }
    }

    pub fn count_source(self) -> Option<CountSource> {
        match self {
            StaticAbility::PowerToughnessPerCount { count, .. } => Some(count),
            StaticAbility::DamageFromControlledSources { .. }
            | StaticAbility::ThresholdPowerToughness { .. } => None,
        }
    }
}

/// Printed text of every static ability in the pool is recorded with its
/// authoritative source in `crates/core/tests/data/card-printings.json`; the
/// catalog strings below are copied from those printings verbatim.
const WIGHT: &[StaticAbility] = &[StaticAbility::PowerToughnessPerCount {
    count: CountSource::CreatureCardsInOpponentsGraveyards,
    power: 1,
    toughness: 1,
    text: "This creature gets +1/+1 for each creature card in your opponents' graveyards.",
}];

const ANGRATH: &[StaticAbility] = &[StaticAbility::DamageFromControlledSources {
    factor: 2,
    text: "If a source you control would deal damage to a permanent or player, it deals double that damage to that permanent or player instead.",
}];

const WEREBEAR: &[StaticAbility] = &[StaticAbility::ThresholdPowerToughness {
    minimum: 7,
    power: 3,
    toughness: 3,
    text: "Threshold — This creature gets +3/+3 as long as there are seven or more cards in your graveyard.",
}];

const NO_STATICS: &[StaticAbility] = &[];

/// Every static ability the rules layer knows for a card, in printed order.
pub fn static_abilities(card: CardId) -> &'static [StaticAbility] {
    match card {
        CardId::WightOfPrecinctSix => WIGHT,
        CardId::AngrathsMarauders => ANGRATH,
        CardId::Werebear => WEREBEAR,
        _ => NO_STATICS,
    }
}
