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
    /// CR 603.6a: any land entered, regardless of who controls the land or the
    /// triggering source (Zo-Zu the Punisher).
    AnyLandEnters,
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
    /// CR 508.1 + 603.6a: this creature was declared as an attacker in the
    /// declare attackers step. The triggering object is the attacker itself and
    /// the step's defending seat is the one an intervening-if can read.
    Attacks,
    /// CR 603.6a + 601.2: the controller cast a creature spell. Distinct from
    /// `SpellCast` (instant or sorcery) because the printed wording is
    /// narrower: a noncreature permanent spell and an instant/sorcery do not
    /// trigger it, and a creature spell does not trigger `SpellCast`.
    CreatureSpellCast,
    /// CR 507.1 + 603.6a: "at the beginning of combat on your turn" — the
    /// beginning-of-combat step of this source's controller's turn. The step
    /// exists once per turn, so the trigger fires at most once per turn.
    BeginningOfCombat,
    /// CR 603.6c + 700.4 with the printed wording "whenever another creature
    /// dies": a creature other than the source died, regardless of controller.
    /// The printed intervening-if "if it was attacking or blocking alone" is
    /// evaluated by the engine where the death is dispatched, because it is a
    /// property of the dying creature's combat state at that moment (CR 506.4,
    /// 508.1, 509.1) and not of this source.
    AnotherCreatureDies,
    /// CR 603.6a: a Ring-bearer dealt combat damage to a player.
    RingCombatDamage,
    /// CR 603.7: the Ring's delayed sacrifice trigger at end of combat.
    EndOfCombat,
    /// CR 513.1 + 603.6a: "At the beginning of your end step".
    EndStep,
    /// CR 603.6c + 509.3 + 510.2: this creature dealt combat damage to a player.
    /// The trigger's controller is the creature's controller, and the damage
    /// must have been actually dealt (a prevented instance does not trigger).
    ThisDealsCombatDamageToPlayer,
    /// CR 603.6c + 700.4 with the printed wording "Pashalik Mons or another
    /// Goblin you control dies": a *Goblin* creature controlled by this source's
    /// controller died. Narrower than `CreatureYouControlDies` on purpose — a
    /// dying Bear never triggers it — and, like that event, it includes the
    /// source itself. "Goblin" is the dying permanent's effective subtype at the
    /// moment it left (CR 613.1d), captured on the death record.
    GoblinYouControlDies,
}

impl TriggerEvent {
    pub fn name(self) -> &'static str {
        match self {
            TriggerEvent::EntersBattlefield => "enters the battlefield",
            TriggerEvent::Dies => "dies",
            TriggerEvent::SpellCast => "you cast an instant or sorcery spell",
            TriggerEvent::LandEnters => "a land you control enters",
            TriggerEvent::AnyLandEnters => "a land enters",
            TriggerEvent::BecomesBlocked => "this creature becomes blocked",
            TriggerEvent::AnotherCreatureEnters => "another creature enters",
            TriggerEvent::CreatureDies => "a creature dies",
            TriggerEvent::CreatureYouControlDies => {
                "this creature or another creature you control dies"
            }
            TriggerEvent::AnotherCreatureYouControlDies => "another creature you control dies",
            TriggerEvent::Attacks => "this creature attacks",
            TriggerEvent::CreatureSpellCast => "you cast a creature spell",
            TriggerEvent::BeginningOfCombat => "at the beginning of combat on your turn",
            TriggerEvent::AnotherCreatureDies => "another creature dies",
            TriggerEvent::RingCombatDamage => "your Ring-bearer deals combat damage to a player",
            TriggerEvent::EndOfCombat => "at end of combat",
            TriggerEvent::EndStep => "at the beginning of your end step",
            TriggerEvent::ThisDealsCombatDamageToPlayer => {
                "this creature deals combat damage to a player"
            }
            TriggerEvent::GoblinYouControlDies => {
                "this creature or another Goblin you control dies"
            }
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
    /// Saddle N: tap any number of other untapped creatures you control with
    /// total effective power N or greater. 0 means not a saddle cost.
    pub saddle_power: Option<u8>,
    /// "Sacrifice a Goblin" (CR 701.21a, 118.3): the activator sacrifices one
    /// creature they control with this subtype as part of the cost. The
    /// sacrificed creature is chosen by the activator and named in the action,
    /// never by the engine, and it may be the ability's own source when the
    /// source prints the subtype (Siege-Gang Commander is itself a Goblin).
    pub sacrifice_creature_subtype: Option<&'static str>,
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
        saddle_power: None,
        sacrifice_creature_subtype: None,
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
        usize::from(self.discard_land)
            + usize::from(self.sacrifice_creature_subtype.is_some())
            + self.exile_graveyard_cards as usize
    }

    pub fn is_free(self) -> bool {
        !self.tap
            && !self.sacrifice_source
            && self.mana.is_none()
            && self.card_cost_count() == 0
            && !self.exile_source_from_graveyard
            && self.saddle_power.is_none()
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
        if let Some(subtype) = self.sacrifice_creature_subtype {
            parts.push(format!("Sacrifice a {subtype}"));
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

/// The Ring emblem's four level-gated triggered abilities, in level order.
static THE_RING: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "Whenever your Ring-bearer attacks, draw a card, then discard a card.",
        event: TriggerEvent::Attacks,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::Draw(1), Effect::DiscardOne],
    }),
    Ability::Triggered(TriggeredAbility {
        text: "Whenever your Ring-bearer becomes blocked by a creature, that creature's controller sacrifices it at end of combat.",
        event: TriggerEvent::BecomesBlocked,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::RingScheduleDelayedSacrifice],
    }),
    Ability::Triggered(TriggeredAbility {
        text: "Whenever your Ring-bearer deals combat damage to a player, each opponent loses 3 life.",
        event: TriggerEvent::RingCombatDamage,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::LoseLifeEachOpponent(3)],
    }),
    Ability::Triggered(TriggeredAbility {
        text: "At end of combat, sacrifice the creature that blocked your Ring-bearer.",
        event: TriggerEvent::EndOfCombat,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::RingDelayedSacrifice],
    }),
];

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

/// Issue 17 slice 19: the pool's first attack trigger. The printed
/// intervening-if is not part of the literal — it is the card's
/// `trigger_condition`, which the declare attackers step evaluates before the
/// trigger is queued and which resolution re-checks (CR 603.4).
const SPECTRAL_BEARS: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever this creature attacks, if defending player controls no black nontoken permanents, it doesn't untap during your next untap step.",
    event: TriggerEvent::Attacks,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::SkipNextUntapStep],
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

const FRIENDLY_TEDDY: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When this creature dies, each player draws a card.",
    event: TriggerEvent::Dies,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::EachPlayerDraws(1)],
})];

const ULVENWALD_BEAR: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Morbid — When this creature enters, if a creature died this turn, put two +1/+1 counters on target creature.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::Creature,
    effects: &[Effect::PutPlusOneCountersOnTarget(2)],
})];

const CALLER_OF_THE_CLAW: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When this creature enters, create a 2/2 green Bear creature token for each nontoken creature put into your graveyard from the battlefield this turn.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::CreateBearTokensForEachDeathThisTurn],
})];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipmentGrants {
    pub haste: bool,
    pub shroud: bool,
    pub hexproof: bool,
}

pub fn equipment_grants(card: CardId) -> Option<EquipmentGrants> {
    match card {
        CardId::LightningGreaves => Some(EquipmentGrants {
            haste: true,
            shroud: true,
            hexproof: false,
        }),
        CardId::SwiftfootBoots => Some(EquipmentGrants {
            haste: true,
            shroud: false,
            hexproof: true,
        }),
        _ => None,
    }
}

const DUNEDAIN_RANGERS: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Landfall — Whenever a land you control enters, if you don't control a Ring-bearer, the Ring tempts you.",
    event: TriggerEvent::LandEnters,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::RingTempts],
})];

const URSINE_MONSTROSITY: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "At the beginning of combat on your turn, mill a card and choose an opponent at random. This creature attacks that player this combat if able. Until end of turn, this creature gains indestructible and gets +1/+1 for each card type among cards in your graveyard.",
    event: TriggerEvent::BeginningOfCombat,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::UrsineCombatTrigger],
})];

/// Issue 17 slice 60: Evercoat Ursine. The printed "Hideaway 3, hideaway 3" is
/// two instances of the hideaway keyword, so the card carries two separate
/// enters-the-battlefield triggered abilities (abilities 0 and 1) and entering
/// resolves two independent looks. The combat-damage ability (index 2) is
/// optional ("you may") and prints an intervening-if, which is
/// `ability::trigger_condition(EvercoatUrsine)`: cards exiled with this
/// permanent must exist when it triggers and again when it resolves (CR 603.4).
const EVERCOAT_URSINE: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "Hideaway 3",
        event: TriggerEvent::EntersBattlefield,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::Hideaway(3)],
    }),
    Ability::Triggered(TriggeredAbility {
        text: "Hideaway 3",
        event: TriggerEvent::EntersBattlefield,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::Hideaway(3)],
    }),
    Ability::Triggered(TriggeredAbility {
        text: "Whenever this creature deals combat damage to a player, if there are cards exiled with it, you may play one of them without paying its mana cost.",
        event: TriggerEvent::ThisDealsCombatDamageToPlayer,
        optional: true,
        target: TargetSpec::None,
        effects: &[Effect::PlayCardExiledWithSource],
    }),
];

const SPIRIT_OF_THE_ALDERGARD: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When this creature enters, search your library for a snow land card, reveal it, put it into your hand, then shuffle.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::SearchSnowLandToHand],
})];

const SPIRIT_STATIC: &[StaticAbility] = &[StaticAbility::PowerToughnessPerCount {
    count: CountSource::OtherSnowPermanentsYouControl,
    power: 1,
    toughness: 0,
    text: "This creature gets +1/+0 for each other snow permanent you control.",
}];

const LIGHTNING_GREAVES: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "Equip {0}",
    cost: AbilityCost {
        mana: Some("{0}"),
        ..AbilityCost::NONE
    },
    target: TargetSpec::CreatureYouControl,
    modes: SINGLE_MODE,
    effects: &[Effect::AttachSourceToTarget],
    once_per_turn: false,
    sorcery_timing: true,
})];

const SWIFTFOOT_BOOTS: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "Equip {1}",
    cost: AbilityCost {
        mana: Some("{1}"),
        ..AbilityCost::NONE
    },
    target: TargetSpec::CreatureYouControl,
    modes: SINGLE_MODE,
    effects: &[Effect::AttachSourceToTarget],
    once_per_turn: false,
    sorcery_timing: true,
})];

const THE_EARTH_KING: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "When The Earth King enters, create a 4/4 green Bear creature token.",
        event: TriggerEvent::EntersBattlefield,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::CreateEarthKingBearToken],
    }),
    Ability::Triggered(TriggeredAbility {
        text: "Whenever one or more creatures you control with power 4 or greater attack, search your library for up to that many basic land cards, put them onto the battlefield tapped, then shuffle.",
        event: TriggerEvent::Attacks,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::SearchBasicLandsUpToTriggerAmount],
    }),
];

/// Lumra, Bellow of the Woods (BLB 183): `{4}{G}{G}` Legendary Creature —
/// Elemental Bear with reach and vigilance, a characteristic-defining
/// power/toughness equal to the number of lands its controller controls, and an
/// enters trigger that mills four cards and then returns every land card from
/// its controller's graveyard to the battlefield tapped.
///
/// The CDA is a printed static (`LUMRA_STATIC`), so it is read from the board on
/// every characteristic pass rather than captured when something else happens:
/// a land play, a land entering from this very trigger, or a land destroyed all
/// move the value (CR 604.3). The card prints `*` for both numbers, which is
/// represented as no printed power/toughness plus that static; the addend is
/// therefore exactly the land count.
const LUMRA: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When Lumra enters, mill four cards. Then return all land cards from your graveyard to the battlefield tapped.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::None,
    effects: &[
        Effect::MillSelf(4),
        Effect::ReturnLandCardsFromGraveyardTapped,
    ],
})];

const DROVER_GRIZZLY: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "Whenever this creature attacks while saddled, creatures you control gain trample until end of turn.",
        event: TriggerEvent::Attacks,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::GrantTrampleToControlledCreatures],
    }),
    Ability::Activated(ActivatedAbility {
        text: "Saddle 1 (Tap any number of other creatures you control with total power 1 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)",
        cost: AbilityCost {
            saddle_power: Some(1),
            ..AbilityCost::NONE
        },
        target: TargetSpec::None,
        modes: SINGLE_MODE,
        effects: &[Effect::SaddleSource],
        once_per_turn: false,
        sorcery_timing: true,
    }),
];

const EXUBERANT_WOLFBEAR: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever this creature attacks, you may have the base power and toughness of target Human you control become equal to this creature's power and toughness until end of turn.",
    event: TriggerEvent::Attacks,
    optional: true,
    target: TargetSpec::HumanYouControl,
    effects: &[Effect::SetBasePowerToughnessFromSource],
})];

const SURRAK_AND_GORECLAW: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever another nontoken creature you control enters, put a +1/+1 counter on it. It gains haste until end of turn.",
    event: TriggerEvent::AnotherCreatureEnters,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::PutCounterAndHasteOnEntered],
})];

const SURRAK_STATIC: &[StaticAbility] = &[StaticAbility::OtherControlledCreaturesHaveTrample {
    text: "Other creatures you control have trample.",
}];

const DRAGON_SCARRED_BEAR: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "Formidable — {1}{G}: Regenerate this creature. Activate only if creatures you control have total power 8 or greater.",
    cost: AbilityCost {
        mana: Some("{1}{G}"),
        ..AbilityCost::NONE
    },
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::RegenerateSource],
    once_per_turn: false,
    sorcery_timing: false,
})];

const VIVIENS_GRIZZLY: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "Look at the top card of your library. If it's a creature or planeswalker card, you may reveal it and put it into your hand. If you don't put the card into your hand, put it on the bottom of your library.",
    cost: AbilityCost {
        mana: Some("{3}{G}"),
        ..AbilityCost::NONE
    },
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::LookTopTakeOrBottom],
    once_per_turn: false,
    sorcery_timing: false,
})];

const AYULA_COUNTERS: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever another Bear you control enters, choose one —\n• Put two +1/+1 counters on target Bear.\n• Target Bear you control fights target creature you don't control.",
    event: TriggerEvent::AnotherCreatureEnters,
    optional: false,
    target: TargetSpec::Bear,
    effects: &[Effect::PutPlusOneCountersOnTarget(2)],
})];

const AYULA_FIGHT: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever another Bear you control enters, choose one —\n• Put two +1/+1 counters on target Bear.\n• Target Bear you control fights target creature you don't control.",
    event: TriggerEvent::AnotherCreatureEnters,
    optional: false,
    target: TargetSpec::BearYouControl,
    effects: &[Effect::FightTargets],
})];

const AYULA_QUEEN_AMONG_BEARS: &[Ability] = &[AYULA_COUNTERS[0], AYULA_FIGHT[0]];

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

/// Little Bear: one enters-the-battlefield trigger whose target is restricted
/// to the controller's own creatures *other than the Bear itself*. The printed
/// word "another" is `TargetSpec::AnotherCreatureYouControl`, which excludes the
/// trigger's own source; plain "target creature you control" would include it.
const LITTLE_BEAR: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When this creature enters, untap another target creature you control. If that creature is a Bear, put a +1/+1 counter on it.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::AnotherCreatureYouControl,
    effects: &[Effect::UntapTargetAndCounterIfBear],
})];

/// Dancing from Dark to Dawn prints two separate triggered abilities, so it is
/// two entries here: a cast trigger whose size is the cast spell's mana value,
/// and an ordinary landfall Bear token. The landfall entry re-reads the source's
/// current zone, so it stops firing once the enchantment has left the
/// battlefield, exactly like the other landfall watchers in the pool.
const DANCING_FROM_DARK_TO_DAWN: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "Whenever you cast a creature spell, put X +1/+1 counters on target creature you control, where X is that spell's mana value.",
        event: TriggerEvent::CreatureSpellCast,
        optional: false,
        target: TargetSpec::CreatureYouControl,
        effects: &[Effect::PutPlusOneCountersOnTargetForTriggerSpellManaValue],
    }),
    Ability::Triggered(TriggeredAbility {
        text: "Landfall — Whenever a land you control enters, create a 2/2 green Bear creature token.",
        event: TriggerEvent::LandEnters,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::CreateBearTokens(1)],
    }),
];

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

/// Goreclaw, Terror of Qal Sisma: an attack trigger that boosts and grants
/// trample to every creature its controller controls whose *current* power is 4
/// or greater. The cost-reduction half is a static ability (see
/// `GORECLAW_STATIC`), not a trigger.
const GORECLAW: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever Goreclaw attacks, each creature you control with power 4 or greater gets +1/+1 and gains trample until end of turn.",
    event: TriggerEvent::Attacks,
    optional: false,
    target: TargetSpec::None,
    effects: &[
        Effect::BoostAndTrampleEachCreatureYouControlWithPowerAtLeast {
            minimum_power: 4,
            power: 1,
            toughness: 1,
        },
    ],
})];

/// Ursine Champion (M19 304): "{5}{G}: This creature gets +3/+3 and becomes a
/// Bear Berserker until end of turn. Activate only once each turn."
///
/// Two effects resolve in printed order (CR 608.2c): the layer 7c pump, then the
/// layer 4 subtype-line change. Layer order — not resolution order — is what
/// decides the outcome, so the pump applies after the new type line as CR 613.1
/// requires. `once_per_turn` is the printed once-each-turn limit (the same
/// restriction Rootwalla prints) and is enforced by `Game::activation_used`.
const URSINE_CHAMPION: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{5}{G}: This creature gets +3/+3 and becomes a Bear Berserker until end of turn. Activate only once each turn.",
    cost: AbilityCost::mana("{5}{G}"),
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[
        Effect::PumpSource(3, 3),
        Effect::SetSourceCreatureSubtypes(crate::layer::SubtypeLine::BearBerserker),
    ],
    once_per_turn: true,
    sorcery_timing: false,
})];

/// Stuffed Bear (MID 259): "{2}: This artifact becomes a 4/4 green Bear artifact
/// creature until end of turn."
///
/// One printed ability, three layers, all until end of turn: layer 4 adds the
/// creature card type and the Bear subtype in addition to the printed Artifact
/// type (CR 613.1d), layer 5 makes it green (CR 613.1e) and layer 7b sets base
/// power/toughness 4/4 (CR 613.4b). Resolution order is irrelevant because the
/// layers decide the outcome (CR 613.1); the effects are listed in the order
/// the printed sentence reads. The object stays an artifact, so it is an
/// artifact creature, and it never stops being the same card.
const STUFFED_BEAR: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{2}: This artifact becomes a 4/4 green Bear artifact creature until end of turn.",
    cost: AbilityCost::mana("{2}"),
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[
        Effect::AddSourceCreatureType(crate::layer::SubtypeLine::Bear),
        Effect::SetSourceColor(crate::card::ManaColor::Green),
        Effect::SetSourceBasePowerToughness(4, 4),
    ],
    once_per_turn: false,
    sorcery_timing: false,
})];

/// Beorn's Hospitality (HOB 120): "Landfall — Whenever a land you control
/// enters, put a +1/+1 counter on target creature you control.
/// {5}{G}{G}: This enchantment becomes a Bear creature in addition to its other
/// types and gains "This creature's power and toughness are each equal to the
/// number of lands you control." (This effect doesn't end.)"
///
/// Two printed abilities. The landfall trigger is the pool's first *targeted*
/// landfall watcher ("target creature you control"), so it asks its controller
/// for a target on the same path as every other targeted trigger. The activated
/// ability is the pool's first layer 7a effect: it adds the creature card type
/// and the Bear subtype without removing Enchantment (CR 613.1d) and grants a
/// characteristic-defining power/toughness (CR 613.4a) that is re-read from the
/// board, and the printed "(This effect doesn't end.)" means the effects are
/// not swept up by cleanup — they end with the object they apply to.
const BEORNS_HOSPITALITY: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "Landfall — Whenever a land you control enters, put a +1/+1 counter on target creature you control.",
        event: TriggerEvent::LandEnters,
        optional: false,
        target: TargetSpec::CreatureYouControl,
        effects: &[Effect::PutPlusOneCountersOnTarget(1)],
    }),
    Ability::Activated(ActivatedAbility {
        text: "{5}{G}{G}: This enchantment becomes a Bear creature in addition to its other types and gains \"This creature's power and toughness are each equal to the number of lands you control.\" (This effect doesn't end.)",
        cost: AbilityCost::mana("{5}{G}{G}"),
        target: TargetSpec::None,
        modes: SINGLE_MODE,
        effects: &[
            Effect::AddSourceCreatureTypeWhileOnBattlefield(crate::layer::SubtypeLine::Bear),
            Effect::SetSourcePowerToughnessEqualToLands,
        ],
        once_per_turn: false,
        sorcery_timing: false,
    }),
];

/// Words of Wilding (ONS 305): "{1}: The next time you would draw a card this
/// turn, create a 2/2 green Bear creature token instead."
///
/// The pool's first *draw* replacement (CR 614.1a). The activated ability prints
/// no timing restriction and no once-per-turn limit, so its controller may arm
/// several replacements in one turn; each one consumes exactly one of that
/// seat's draws, and an unspent one ends in cleanup (CR 514.2). The armed
/// replacement is not tied to this enchantment's existence, so a replacement
/// that has already resolved still applies after the source is destroyed.
const WORDS_OF_WILDING: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{1}: The next time you would draw a card this turn, create a 2/2 green Bear creature token instead.",
    cost: AbilityCost::mana("{1}"),
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::ReplaceNextDrawWithBearToken],
    once_per_turn: false,
    sorcery_timing: false,
})];

/// Beorn the Fierce (HOB 119): "Trample
/// Other Bears you control get +2/+2.
/// At the beginning of combat on your turn, put a trample counter on up to one
/// target creature you control. It becomes a Bear in addition to its other
/// types. Then if you control three or more Bears, draw two cards."
///
/// The triggered ability is one ability with three instructions that resolve in
/// printed order (CR 608.2c): the counter (CR 122.1b, a keyword counter that
/// grants trample for as long as it stays), the layer 4 addition of the Bear
/// subtype (CR 613.1d, until end of turn because the ability states no
/// duration, CR 611.2a), and the conditional draw, whose Bear count is taken
/// after both earlier instructions have applied — so a creature that just
/// became a Bear counts toward the three. The target is "up to one", so the
/// controller may put the ability on the stack with no target at all
/// (CR 115.6); the draw still happens.
const BEORN_THE_FIERCE: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "At the beginning of combat on your turn, put a trample counter on up to one target creature you control. It becomes a Bear in addition to its other types. Then if you control three or more Bears, draw two cards.",
    event: TriggerEvent::BeginningOfCombat,
    optional: false,
    target: TargetSpec::UpToOneCreatureYouControl,
    effects: &[
        Effect::PutTrampleCounterOnTarget,
        Effect::AddTargetCreatureType(crate::layer::SubtypeLine::Bear),
        Effect::DrawIfControlsBears {
            minimum: 3,
            cards: 2,
        },
    ],
})];

/// Druid's Familiar (AVR 175): "Soulbond (You may pair this creature with
/// another unpaired creature when either enters. They remain paired for as long
/// as you control both of them.) / As long as this creature is paired with
/// another creature, each of those creatures gets +2/+2."
///
/// Two triggered abilities implement the printed soulbond reminder: one for this
/// creature's own entry (CR 603.6a) and one for another creature entering. Both
/// name the same set — another unpaired creature its controller controls — and
/// both resolve into the same pairing effect. The "you may" half is the target
/// choice itself: the pairing choice spec is the second spec the engine treats
/// as declinable (`trigger_target_is_optional`), so declining the choice is how
/// the controller chooses not to pair (CR 702.94a). The pairing is a state of
/// the two permanents, not a continuous effect: it is created once and ends
/// when one of them leaves the battlefield.
const DRUIDS_FAMILIAR: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "You may pair this creature with another unpaired creature when either enters.",
        event: TriggerEvent::EntersBattlefield,
        optional: false,
        target: TargetSpec::UnpairedAnotherCreatureYouControl,
        effects: &[Effect::PairSourceWithChosenCreature],
    }),
    Ability::Triggered(TriggeredAbility {
        text: "You may pair this creature with another unpaired creature when either enters.",
        event: TriggerEvent::AnotherCreatureEnters,
        optional: false,
        target: TargetSpec::UnpairedAnotherCreatureYouControl,
        effects: &[Effect::PairSourceWithChosenCreature],
    }),
];

/// Every ability the rules layer knows for a card, in printed order.
pub fn abilities(card: CardId) -> &'static [Ability] {
    match card {
        CardId::TheRing => THE_RING,
        CardId::SpiderHamPeterPorker => SPIDER_HAM,
        CardId::FoodToken => FOOD_TOKEN,
        CardId::ClueToken => CLUE_TOKEN,
        CardId::ThijarianWitness => THIJARIAN_WITNESS,
        CardId::OwlbearCub => OWLBEAR_CUB,
        CardId::RuxaPatientProfessor => RUXA_PATIENT_PROFESSOR,
        CardId::RampagingYaoGuai => RAMPAGING_YAO_GUAI,
        CardId::DruidSFamiliar => DRUIDS_FAMILIAR,
        CardId::BeornTheFierce => BEORN_THE_FIERCE,
        CardId::BeornsHospitality => BEORNS_HOSPITALITY,
        CardId::WordsOfWilding => WORDS_OF_WILDING,
        CardId::StuffedBear => STUFFED_BEAR,
        CardId::UrsineChampion => URSINE_CHAMPION,
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
        CardId::FriendlyTeddy => FRIENDLY_TEDDY,
        CardId::UlvenwaldBear => ULVENWALD_BEAR,
        CardId::CallerOfTheClaw => CALLER_OF_THE_CLAW,
        CardId::ViviensGrizzly => VIVIENS_GRIZZLY,
        CardId::DragonScarredBear => DRAGON_SCARRED_BEAR,
        CardId::SurrakAndGoreclaw => SURRAK_AND_GORECLAW,
        CardId::ExuberantWolfbear => EXUBERANT_WOLFBEAR,
        CardId::DroverGrizzly => DROVER_GRIZZLY,
        CardId::TheEarthKing => THE_EARTH_KING,
        CardId::LumraBellowOfTheWoods => LUMRA,
        CardId::SpiritOfTheAldergard => SPIRIT_OF_THE_ALDERGARD,
        CardId::UrsineMonstrosity => URSINE_MONSTROSITY,
        CardId::EvercoatUrsine => EVERCOAT_URSINE,
        CardId::DunedainRangers => DUNEDAIN_RANGERS,
        CardId::LightningGreaves => LIGHTNING_GREAVES,
        CardId::SwiftfootBoots => SWIFTFOOT_BOOTS,
        CardId::AyulaQueenAmongBears => AYULA_QUEEN_AMONG_BEARS,
        CardId::CommandersSphere => COMMANDERS_SPHERE,
        CardId::AyulasInfluence => AYULAS_INFLUENCE,
        CardId::Bearscape => BEARSCAPE,
        CardId::MotherBear => MOTHER_BEAR,
        CardId::LittleBear => LITTLE_BEAR,
        CardId::DancingFromDarkToDawn => DANCING_FROM_DARK_TO_DAWN,
        CardId::GoreclawTerrorOfQalSisma => GORECLAW,
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
        CardId::SearslicerGoblin => SEARSLICER_GOBLIN,
        CardId::SiegeGangLieutenant => SIEGE_GANG_LIEUTENANT,
        CardId::ZoZuThePunisher => ZO_ZU,
        CardId::GoblinSharpshooter => GOBLIN_SHARPSHOOTER,
        CardId::KrenkoTinStreetKingpin => KRENKO_TIN_STREET_KINGPIN,
        CardId::HobgoblinBanditLord => HOBGOBLIN_BANDIT_LORD,
        CardId::GoblinMatron => GOBLIN_MATRON,
        CardId::SiegeGangCommander => SIEGE_GANG_COMMANDER,
        CardId::PashalikMons => PASHALIK_MONS,
        CardId::GoblinTrashmaster => GOBLIN_TRASHMASTER,
        CardId::Moggcatcher => MOGGCATCHER,
        CardId::BeetlebackChief => BEETLEBACK_CHIEF,
        CardId::ProdigalSorcerer => TORCHER,
        CardId::Rootwalla => ROOTWALLA,
        CardId::SpectralBears => SPECTRAL_BEARS,
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

/// Whether the card prints an ability that can turn the permanent itself into a
/// creature. CR 302.6 asks whether a creature has been under its controller's
/// control since their most recent turn began, so a permanent that enters as a
/// noncreature but can animate itself has to record the same restriction when
/// it arrives; `Game::enter_battlefield` asks this instead of the printed card
/// type alone.
pub fn can_become_creature(card: CardId) -> bool {
    abilities(card)
        .iter()
        .filter_map(Ability::activated)
        .flat_map(|activated| activated.effects.iter())
        .any(|effect| matches!(effect, Effect::AddSourceCreatureType(_)))
}

/// CR 603.4: an intervening-if clause. It is held per card rather than in the
/// `TriggeredAbility` literal so that every pre-slice-19 trigger keeps its
/// exact literal, and it is evaluated twice by the rules — once when the
/// trigger would fire and once when it resolves.
///
/// Only the single clause the pool prints is modelled. A general condition
/// expression (`TriggerCondition::All(Vec<..>)`, comparisons, counters) is
/// deliberately not introduced until a second card needs it; adding one here
/// without a second consumer would be speculative structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerCondition {
    /// CR 701.54a: controller does not control their Ring-bearer.
    NoRingBearer,
    /// Source permanent is currently saddled (CR 702.171a).
    SourceIsSaddled,
    /// No printed clause: the trigger fires whenever its event happens.
    Always,
    /// "if defending player controls no black nontoken permanents". Black is
    /// read from printed color (CR 202.2a, `CardDef::is_black`), and a token is
    /// excluded by CR 111.6 even though it can have a color.
    DefendingPlayerControlsNoBlackNontokenPermanent,
    /// CR 508.1 + 603.4: "Raid — if you attacked this turn". The declaration
    /// history is read from the captured controller, not current attackers.
    RaidAttackedThisTurn,
    /// Lieutenant: "if you control your commander" (Siege-Gang Lieutenant).
    ControlsYourCommander,
    /// "if there are cards exiled with it" (Evercoat Ursine). Read from the
    /// live battlefield source: the cards are the face-down exile cards whose
    /// `hideaway_source` is that exact permanent object, so the clause is false
    /// once the permanent that exiled them has left the battlefield (CR 400.7:
    /// it is a new object in its new zone).
    CardsExiledWithSource,
}

/// The intervening-if clause of `card`'s trigger, if it prints one.
pub fn trigger_condition(card: CardId) -> TriggerCondition {
    match card {
        CardId::SpectralBears => TriggerCondition::DefendingPlayerControlsNoBlackNontokenPermanent,
        CardId::DunedainRangers => TriggerCondition::NoRingBearer,
        CardId::DroverGrizzly => TriggerCondition::SourceIsSaddled,
        CardId::EvercoatUrsine => TriggerCondition::CardsExiledWithSource,
        CardId::SearslicerGoblin => TriggerCondition::RaidAttackedThisTurn,
        CardId::SiegeGangLieutenant => TriggerCondition::ControlsYourCommander,
        _ => TriggerCondition::Always,
    }
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
    /// "the number of lands you control" (Lumra, Bellow of the Woods). Read
    /// from the board on every characteristic pass (CR 604.3), so a land that
    /// enters or leaves changes the value immediately.
    LandsYouControl,
    /// Other snow permanents controlled by the source's controller.
    OtherSnowPermanentsYouControl,
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
    /// CR 601.2f + 118.7: "Creature spells you cast with power N or greater cost
    /// {G} less to cast" (Goreclaw, Terror of Qal Sisma). It is a static ability
    /// of a permanent, so it applies only while that permanent is on the
    /// battlefield, and it reduces only the generic part of the cost — colored
    /// pips are never reduced (CR 118.7a).
    CreatureSpellsYouCastWithPowerCostLess {
        minimum_power: i32,
        generic: u8,
        text: &'static str,
    },
    /// CR 613.4d + 702.19a: other creatures controlled by this source's
    /// controller have trample while it is on the battlefield.
    OtherControlledCreaturesHaveTrample { text: &'static str },
    /// CR 613.1d + 611.3a: "During your turn, this creature is a Bear ... (It
    /// loses all other creature types.)" — a *printed static* layer 4 effect
    /// (Circle of the Moon Druid's Bear Form). Unlike a `SetCreatureSubtypes`
    /// continuous effect it is not created by a resolution: it applies for as
    /// long as its source is on the battlefield and its controller is the
    /// active player, so it never carries a timestamp of its own (its
    /// timestamp is when the permanent entered, which precedes any effect
    /// created later and therefore loses to one in the same layer).
    SelfBecomesSubtypeDuringYourTurn {
        line: crate::layer::SubtypeLine,
        text: &'static str,
    },
    /// CR 613.4b: "... with base power and toughness 4/2" — the layer 7b half
    /// of the same printed ability. It applies under the same
    /// during-your-turn condition as the subtype line above.
    SelfBasePowerToughnessDuringYourTurn {
        power: i32,
        toughness: i32,
        text: &'static str,
    },
    /// CR 613.4c: "Other Bears you control get +P/+T" (Beorn the Fierce). A
    /// layer 7c additive static that applies to every *other* creature its
    /// controller controls whose effective subtype is Bear — so a creature
    /// animated or made a Bear by a layer 4 effect counts (CR 613.1d), and the
    /// source never pumps itself.
    OtherBearsYouControlGet {
        power: i32,
        toughness: i32,
        text: &'static str,
    },
    /// CR 601.2f + 118.7a: "Goblin spells you cast cost {1} less to cast"
    /// (Goblin Warchief) or "Dragon spells ..." (Dragonlord's Servant). The
    /// static reduces only the generic part of a matching spell cost.
    SubtypeSpellsYouCastCostLess {
        subtype: &'static str,
        generic: u8,
        text: &'static str,
    },
    /// CR 613.4d + 205.3: "Goblins you control have haste" (Goblin Warchief).
    /// Unlike the "Other" variants, the printed wording includes the source.
    ControlledCreaturesWithAnySubtypeHaveHaste {
        subtypes: &'static [&'static str],
        text: &'static str,
    },
    /// CR 613.4c: "Other Goblins get +P/+T" with no controller restriction
    /// (Goblin King). Every other matching creature in every seat is affected;
    /// the source itself is not.
    AllOtherCreaturesWithAnySubtypeGet {
        subtypes: &'static [&'static str],
        power: i32,
        toughness: i32,
        text: &'static str,
    },
    /// CR 702.14c: "Other Goblins ... have mountainwalk" with no controller
    /// restriction (Goblin King). The source itself is excluded.
    AllOtherCreaturesWithAnySubtypeHaveMountainwalk {
        subtypes: &'static [&'static str],
        text: &'static str,
    },
    /// CR 613.4d + 205.3: "Other Goblin creatures you control have haste"
    /// (Goblin Chieftain). The source never grants itself haste; the recipient
    /// must be another creature controlled by the same seat and have any listed
    /// effective subtype (CR 613.1d).
    OtherControlledCreaturesWithAnySubtypeHaveHaste {
        subtypes: &'static [&'static str],
        text: &'static str,
    },
    /// CR 613.4c + 205.3: "Other Spiders, Boars, ... you control get +P/+T"
    /// (Spider-Ham, Peter Porker). The same layer 7c additive static as the Bear
    /// anthem above, but the recipient set is the printed *list* of creature
    /// subtypes: a creature qualifies if its effective subtype line carries any
    /// one of them, so a Bear made by a layer 4 animation counts and a creature
    /// with none of them never does. The printed word is "Other", so the source
    /// never pumps itself.
    OtherControlledCreaturesWithAnySubtypeGet {
        subtypes: &'static [&'static str],
        power: i32,
        toughness: i32,
        text: &'static str,
    },
    /// CR 702.94a + 613.4c: "As long as this creature is paired with another
    /// creature, each of those creatures gets +P/+T" (Druid's Familiar). A
    /// layer 7c additive static that applies to *both* members of the pair, read
    /// through the validating `Game::paired_with`, so a pairing that ended
    /// (a member left the battlefield, CR 400.7) pumps nothing. Two paired
    /// copies of the card each grant their own bonus, exactly as printed.
    PairedEachGet {
        power: i32,
        toughness: i32,
        text: &'static str,
    },
    /// CR 613.4c: "Creatures you control with no abilities get +P/+T" (Ruxa,
    /// Patient Professor). A layer 7c additive static whose recipient set is
    /// filtered by the shared no-abilities predicate rather than by subtype. The
    /// printed text has **no** "Other", so the source is a legal recipient if it
    /// ever stops having abilities — which cannot happen while this static is
    /// one of its abilities, but the code does not special-case the source.
    ControlledCreaturesWithNoAbilitiesGet {
        power: i32,
        toughness: i32,
        text: &'static str,
    },
    /// CR 510.1 + 509.1h: "For each creature you control with no abilities, you
    /// may have that creature assign its combat damage as though it weren't
    /// blocked" (Ruxa, Patient Professor). Not a layer effect: it is a
    /// permission that changes *how* one creature assigns its combat damage, so
    /// the combat-damage step reads it and asks the controller instead of
    /// assigning automatically.
    ControlledCreaturesWithNoAbilitiesAssignAsUnblocked { text: &'static str },
    /// CR 502.3: "This creature doesn't untap during your untap step" (Goblin
    /// Sharpshooter). A static restriction on the untap step, not a one-shot
    /// marker: it applies to every untap step of the source's controller for as
    /// long as that exact object is on the battlefield, and an effect that
    /// untaps the permanent directly ("whenever a creature dies, untap this
    /// creature") is unaffected by it.
    SelfDoesNotUntapDuringUntapStep { text: &'static str },
}

impl StaticAbility {
    pub fn text(self) -> &'static str {
        match self {
            StaticAbility::PowerToughnessPerCount { text, .. } => text,
            StaticAbility::DamageFromControlledSources { text, .. }
            | StaticAbility::ThresholdPowerToughness { text, .. }
            | StaticAbility::CreatureSpellsYouCastWithPowerCostLess { text, .. }
            | StaticAbility::OtherControlledCreaturesHaveTrample { text }
            | StaticAbility::SelfBecomesSubtypeDuringYourTurn { text, .. }
            | StaticAbility::SelfBasePowerToughnessDuringYourTurn { text, .. }
            | StaticAbility::OtherBearsYouControlGet { text, .. }
            | StaticAbility::OtherControlledCreaturesWithAnySubtypeGet { text, .. }
            | StaticAbility::OtherControlledCreaturesWithAnySubtypeHaveHaste { text, .. }
            | StaticAbility::SubtypeSpellsYouCastCostLess { text, .. }
            | StaticAbility::ControlledCreaturesWithAnySubtypeHaveHaste { text, .. }
            | StaticAbility::AllOtherCreaturesWithAnySubtypeGet { text, .. }
            | StaticAbility::AllOtherCreaturesWithAnySubtypeHaveMountainwalk { text, .. }
            | StaticAbility::PairedEachGet { text, .. }
            | StaticAbility::ControlledCreaturesWithNoAbilitiesGet { text, .. }
            | StaticAbility::ControlledCreaturesWithNoAbilitiesAssignAsUnblocked { text }
            | StaticAbility::SelfDoesNotUntapDuringUntapStep { text } => text,
        }
    }

    /// Layer this ability's continuous effect belongs to. A replacement effect
    /// is layerless: it never applies in a layer, so `None`. Neither is a
    /// cost-reduction ability: it modifies how a spell is cast (CR 601.2f) and
    /// is not a continuous effect on any object.
    pub fn layer(self) -> Option<Layer> {
        match self {
            StaticAbility::PowerToughnessPerCount { .. } => Some(Layer::Modifying),
            StaticAbility::SelfBecomesSubtypeDuringYourTurn { .. } => Some(Layer::Type),
            StaticAbility::SelfBasePowerToughnessDuringYourTurn { .. } => Some(Layer::Setting),
            StaticAbility::OtherBearsYouControlGet { .. } => Some(Layer::Modifying),
            StaticAbility::OtherControlledCreaturesWithAnySubtypeGet { .. } => {
                Some(Layer::Modifying)
            }
            StaticAbility::PairedEachGet { .. } => Some(Layer::Modifying),
            StaticAbility::ControlledCreaturesWithNoAbilitiesGet { .. } => Some(Layer::Modifying),
            StaticAbility::DamageFromControlledSources { .. }
            | StaticAbility::ThresholdPowerToughness { .. }
            | StaticAbility::CreatureSpellsYouCastWithPowerCostLess { .. }
            | StaticAbility::OtherControlledCreaturesHaveTrample { .. }
            | StaticAbility::OtherControlledCreaturesWithAnySubtypeHaveHaste { .. }
            | StaticAbility::SubtypeSpellsYouCastCostLess { .. }
            | StaticAbility::ControlledCreaturesWithAnySubtypeHaveHaste { .. }
            | StaticAbility::AllOtherCreaturesWithAnySubtypeHaveMountainwalk { .. }
            | StaticAbility::ControlledCreaturesWithNoAbilitiesAssignAsUnblocked { .. }
            | StaticAbility::SelfDoesNotUntapDuringUntapStep { .. } => None,
            StaticAbility::AllOtherCreaturesWithAnySubtypeGet { .. } => Some(Layer::Modifying),
        }
    }

    pub fn count_source(self) -> Option<CountSource> {
        match self {
            StaticAbility::PowerToughnessPerCount { count, .. } => Some(count),
            StaticAbility::DamageFromControlledSources { .. }
            | StaticAbility::ThresholdPowerToughness { .. }
            | StaticAbility::CreatureSpellsYouCastWithPowerCostLess { .. }
            | StaticAbility::OtherControlledCreaturesHaveTrample { .. }
            | StaticAbility::SelfBecomesSubtypeDuringYourTurn { .. }
            | StaticAbility::SelfBasePowerToughnessDuringYourTurn { .. }
            | StaticAbility::OtherBearsYouControlGet { .. }
            | StaticAbility::OtherControlledCreaturesWithAnySubtypeGet { .. }
            | StaticAbility::OtherControlledCreaturesWithAnySubtypeHaveHaste { .. }
            | StaticAbility::SubtypeSpellsYouCastCostLess { .. }
            | StaticAbility::ControlledCreaturesWithAnySubtypeHaveHaste { .. }
            | StaticAbility::AllOtherCreaturesWithAnySubtypeGet { .. }
            | StaticAbility::AllOtherCreaturesWithAnySubtypeHaveMountainwalk { .. }
            | StaticAbility::PairedEachGet { .. }
            | StaticAbility::ControlledCreaturesWithNoAbilitiesGet { .. }
            | StaticAbility::ControlledCreaturesWithNoAbilitiesAssignAsUnblocked { .. }
            | StaticAbility::SelfDoesNotUntapDuringUntapStep { .. } => None,
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

/// Lumra's characteristic-defining power/toughness (CR 604.3, 613.4a). The card
/// prints `*`/`*`, so the catalog records no printed numbers and this static is
/// the whole value: one point of each per land its controller controls, read from
/// the board on every characteristic pass. Counters and pumps still add on top
/// (layer 7d/7c), which is what "equal to the number of lands" plus a +1/+1
/// counter means.
const LUMRA_STATIC: &[StaticAbility] = &[StaticAbility::PowerToughnessPerCount {
    count: CountSource::LandsYouControl,
    power: 1,
    toughness: 1,
    text: "Lumra's power and toughness are each equal to the number of lands you control.",
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

/// Goreclaw, Terror of Qal Sisma's cost reduction. Printed power is what the
/// spell has while it is a card in a hand, library or command zone; nothing in
/// this pool changes a card's power outside the battlefield, so no continuous
/// recalculation is needed (disclosed in docs/issue17-starter-corpus.md).
const GORECLAW_STATIC: &[StaticAbility] =
    &[StaticAbility::CreatureSpellsYouCastWithPowerCostLess {
        minimum_power: 4,
        generic: 2,
        text: "Creature spells you cast with power 4 or greater cost {2} less to cast.",
    }];

/// Circle of the Moon Druid's Bear Form (AFR 177). One printed ability, two
/// layers: the subtype line is layer 4 (CR 613.1d) and the base power/toughness
/// is layer 7b (CR 613.4b). Both halves carry the same "during your turn"
/// condition, which the engine evaluates against the active player at the
/// moment it asks the question, so no effect object is created and nothing has
/// to expire in cleanup: the ability simply stops applying at end of turn.
const CIRCLE_OF_THE_MOON_DRUID: &[StaticAbility] = &[
    StaticAbility::SelfBecomesSubtypeDuringYourTurn {
        line: crate::layer::SubtypeLine::Bear,
        text: "Bear Form — During your turn, this creature is a Bear with base power and toughness 4/2. (It loses all other creature types.)",
    },
    StaticAbility::SelfBasePowerToughnessDuringYourTurn {
        power: 4,
        toughness: 2,
        text: "Bear Form — During your turn, this creature is a Bear with base power and toughness 4/2. (It loses all other creature types.)",
    },
];

/// Beorn the Fierce's printed anthem. It is a layer 7c additive static, so it
/// stacks with pumps and counters instead of setting a base value, and it reads
/// the *effective* subtype of each other creature (a Bear made by any layer 4
/// effect counts; Beorn itself never pumps itself).
const BEORN_STATIC: &[StaticAbility] = &[StaticAbility::OtherBearsYouControlGet {
    power: 2,
    toughness: 2,
    text: "Other Bears you control get +2/+2.",
}];

/// Druid's Familiar (AVR 175): "As long as this creature is paired with another
/// creature, each of those creatures gets +2/+2." The pair itself is engine
/// state (`Object::paired_with`), so this static only declares the bonus the
/// pair produces; the engine applies it to both members through
/// `Game::paired_with`, which re-validates that the pairing still exists.
const DRUIDS_FAMILIAR_STATIC: &[StaticAbility] = &[StaticAbility::PairedEachGet {
    power: 2,
    toughness: 2,
    text: "As long as this creature is paired with another creature, each of those creatures gets +2/+2.",
}];

/// The Food token's printed ability (CR 111.10, 205.3): "{2}, {T}, Sacrifice
/// this token: You gain 3 life." The cost shape itself is not new — Mind Stone
/// ("{1}, {T}, Sacrifice this artifact: Draw a card.") and Hedron Archive
/// ("{2}, {T}, Sacrifice this artifact: Draw two cards.") already combine a mana
/// cost, `{T}` and sacrificing the source. What is new is that the source is a
/// *token*, so paying the cost puts a token into the graveyard and CR 111.7 then
/// makes it cease to exist, and that the ability is offered only while the
/// untapped token is on the battlefield and the mana is payable.
const FOOD_TOKEN: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{2}, {T}, Sacrifice this token: You gain 3 life.",
    cost: AbilityCost {
        tap: true,
        sacrifice_source: true,
        mana: Some("{2}"),
        ..AbilityCost::NONE
    },
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::GainLife(3)],
    once_per_turn: false,
    sorcery_timing: false,
})];

/// The Clue token's printed ability (CR 111.10, 701.16a): "{2}, Sacrifice this
/// token: Draw a card." Unlike the Food token there is no `{T}`: the sacrifice is
/// the whole nonmana cost, so a Clue that entered this turn can be cracked at
/// once. Paying it puts a *token* into the graveyard, which then ceases to exist
/// as a state-based action (CR 111.7), exactly like the Food token's cost.
const CLUE_TOKEN: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{2}, Sacrifice this token: Draw a card.",
    cost: AbilityCost {
        tap: false,
        sacrifice_source: true,
        mana: Some("{2}"),
        ..AbilityCost::NONE
    },
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::Draw(1)],
    once_per_turn: false,
    sorcery_timing: false,
})];

/// Thijarian Witness (WHO 111). One printed trigger: "Bear Witness — Whenever
/// another creature dies, if it was attacking or blocking alone, exile it and
/// investigate." The intervening-if is a combat-state property of the dying
/// creature, so the death dispatch evaluates it before the trigger is queued and
/// the engine records the dead object on the trigger so the resolution can exile
/// that exact card (never "a" card the controller picks). Flash is printed
/// separately in the catalog definition and needs no ability literal (CR 702.8a).
const THIJARIAN_WITNESS: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Bear Witness — Whenever another creature dies, if it was attacking or blocking alone, exile it and investigate.",
    event: TriggerEvent::AnotherCreatureDies,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::ExileDeadCreatureAndInvestigate],
})];

/// Owlbear Cub (CLB 246). The attack event must be against a player controlling
/// eight or more lands; this is not an intervening-if (CR 603.4) and is not
/// rechecked at resolution. It is followed by a private look and optional put onto
/// the battlefield tapped and attacking. The trigger itself is not optional — the
/// "may" belongs to the put, which is the pending choice's decline path — so the
/// typed ability is not `optional`.
const OWLBEAR_CUB: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Mama's Coming — Whenever this creature attacks a player who controls eight or more lands, look at the top eight cards of your library. You may put a creature card from among them onto the battlefield tapped and attacking that player. Put the rest on the bottom of your library in a random order.",
    event: TriggerEvent::Attacks,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::LookAtTopPutCreatureAttacking],
})];

/// Ruxa, Patient Professor (C21 66). One printed sentence with **two** trigger
/// events ("Whenever Ruxa enters or attacks"), so the ability table carries two
/// entries with the same printed text and the same effect — the engine indexes
/// abilities per event and `TriggerRef::ability` is an index into this table, so
/// one shared entry could not fire on both. The printed target is a *card* in a
/// graveyard, which is `Target::Card`, not `Target::Permanent` (CR 110.1); the
/// "no abilities" filter is the shared predicate in the engine, so the trigger
/// has no legal target when the graveyard holds only creatures with abilities
/// and is then not put on the stack at all (CR 603.3d).
const RUXA_PATIENT_PROFESSOR: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "Whenever Ruxa enters or attacks, return target creature card with no abilities from your graveyard to your hand.",
        event: TriggerEvent::EntersBattlefield,
        optional: false,
        target: TargetSpec::NoAbilityCreatureCardInYourGraveyard,
        effects: &[Effect::ReturnGraveyardCardToHand],
    }),
    Ability::Triggered(TriggeredAbility {
        text: "Whenever Ruxa enters or attacks, return target creature card with no abilities from your graveyard to your hand.",
        event: TriggerEvent::Attacks,
        optional: false,
        target: TargetSpec::NoAbilityCreatureCardInYourGraveyard,
        effects: &[Effect::ReturnGraveyardCardToHand],
    }),
];

/// Rampaging Yao Guai's single printed triggered ability (PIP 82). "Any number
/// of target artifacts and/or enchantments with total mana value X or less" is
/// a *set* of targets under one aggregate restriction, so the target spec is the
/// aggregate one and the budget is the X the entering spell announced.
const RAMPAGING_YAO_GUAI: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When this creature enters, destroy any number of target artifacts and/or enchantments with total mana value X or less.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::ArtifactOrEnchantment,
    effects: &[Effect::DestroyTargetArtifactsAndEnchantments],
})];

/// Ruxa's two statics, in printed order: the layer 7c anthem and the
/// damage-assignment permission.
const RUXA_STATIC: &[StaticAbility] = &[
    StaticAbility::ControlledCreaturesWithNoAbilitiesGet {
        power: 1,
        toughness: 1,
        text: "Creatures you control with no abilities get +1/+1.",
    },
    StaticAbility::ControlledCreaturesWithNoAbilitiesAssignAsUnblocked {
        text: "For each creature you control with no abilities, you may have that creature assign its combat damage as though it weren't blocked.",
    },
];

/// Spider-Ham, Peter Porker (SPM 114). Two printed abilities: an enters trigger
/// that creates a Food token, and "Animal May-Ham — Other Spiders, Boars, ... you
/// control get +1/+1". The anthem is the first *list*-filtered subtype static in
/// the pool (see `SPIDER_HAM_STATIC`).
const SPIDER_HAM: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When Spider-Ham enters, create a Food token.",
    event: TriggerEvent::EntersBattlefield,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::CreateFoodToken],
})];

/// The printed subtype list of Spider-Ham's anthem, in singular form because
/// CR 205.3 writes subtypes in the singular and a type line prints "Bear", not
/// "Bears" ("Mice" and "Wolves" are the two plurals that are not just a suffix).
const SPIDER_HAM_SUBTYPES: &[&str] = &[
    "Spider", "Boar", "Bat", "Bear", "Bird", "Cat", "Dog", "Frog", "Jackal", "Lizard", "Mouse",
    "Otter", "Rabbit", "Raccoon", "Rat", "Squirrel", "Turtle", "Wolf",
];

const GOBLIN_SUBTYPES: &[&str] = &["Goblin"];

const ZO_ZU: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever a land enters, Zo-Zu deals 2 damage to that land's controller.",
    event: TriggerEvent::AnyLandEnters,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::DamageAffectedPlayer(2)],
})];

const SIEGE_GANG_LIEUTENANT: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "Lieutenant — At the beginning of combat on your turn, if you control your commander, create two 1/1 red Goblin creature tokens. Those tokens gain haste until end of turn.",
        event: TriggerEvent::BeginningOfCombat,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::CreateHasteGoblinTokens(2)],
    }),
    Ability::Activated(ActivatedAbility {
        text: "{2}, Sacrifice a Goblin: This creature deals 1 damage to any target.",
        cost: AbilityCost {
            mana: Some("{2}"),
            sacrifice_creature_subtype: Some("Goblin"),
            ..AbilityCost::NONE
        },
        target: TargetSpec::AnyTarget,
        modes: SINGLE_MODE,
        effects: &[Effect::Damage(1)],
        once_per_turn: false,
        sorcery_timing: false,
    }),
];

const SEARSLICER_GOBLIN: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Raid — At the beginning of your end step, if you attacked this turn, create a 1/1 red Goblin creature token.",
    event: TriggerEvent::EndStep,
    optional: false,
    target: TargetSpec::None,
    effects: &[Effect::CreateGoblinTokens(1)],
})];

const GOBLIN_CHIEFTAIN_STATIC: &[StaticAbility] = &[
    StaticAbility::OtherControlledCreaturesWithAnySubtypeGet {
        subtypes: GOBLIN_SUBTYPES,
        power: 1,
        toughness: 1,
        text: "Other Goblin creatures you control get +1/+1 and have haste.",
    },
    StaticAbility::OtherControlledCreaturesWithAnySubtypeHaveHaste {
        subtypes: GOBLIN_SUBTYPES,
        text: "Other Goblin creatures you control get +1/+1 and have haste.",
    },
];

const GOBLIN_WARCHIEF_STATIC: &[StaticAbility] = &[
    StaticAbility::SubtypeSpellsYouCastCostLess {
        subtype: "Goblin",
        generic: 1,
        text: "Goblin spells you cast cost {1} less to cast.",
    },
    StaticAbility::ControlledCreaturesWithAnySubtypeHaveHaste {
        subtypes: GOBLIN_SUBTYPES,
        text: "Goblins you control have haste.",
    },
];

const GOBLIN_KING_STATIC: &[StaticAbility] = &[
    StaticAbility::AllOtherCreaturesWithAnySubtypeGet {
        subtypes: GOBLIN_SUBTYPES,
        power: 1,
        toughness: 1,
        text: "Other Goblins get +1/+1 and have mountainwalk.",
    },
    StaticAbility::AllOtherCreaturesWithAnySubtypeHaveMountainwalk {
        subtypes: GOBLIN_SUBTYPES,
        text: "Other Goblins get +1/+1 and have mountainwalk.",
    },
];

const DRAGONLORDS_SERVANT_STATIC: &[StaticAbility] =
    &[StaticAbility::SubtypeSpellsYouCastCostLess {
        subtype: "Dragon",
        generic: 1,
        text: "Dragon spells you cast cost {1} less to cast.",
    }];

/// Goblin Sharpshooter (C13 111). Its first printed line is a *standing*
/// untap-step restriction (not a one-shot "doesn't untap during your next untap
/// step"), so it is a static ability rather than an effect; the second line
/// untaps it directly and is therefore not stopped by the first; the third is
/// the pool's second "{T}: deal 1 damage to any target" activated ability.
const GOBLIN_SHARPSHOOTER: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "Whenever a creature dies, untap this creature.",
        event: TriggerEvent::CreatureDies,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::UntapSource],
    }),
    Ability::Activated(ActivatedAbility {
        text: "{T}: This creature deals 1 damage to any target.",
        cost: AbilityCost::TAP,
        target: TargetSpec::AnyTarget,
        modes: SINGLE_MODE,
        effects: &[Effect::Damage(1)],
        once_per_turn: false,
        sorcery_timing: false,
    }),
];

const GOBLIN_SHARPSHOOTER_STATIC: &[StaticAbility] =
    &[StaticAbility::SelfDoesNotUntapDuringUntapStep {
        text: "This creature doesn't untap during your untap step.",
    }];

/// Pashalik Mons (DMR 133). Two printed abilities: a subtype-narrowed dies
/// watcher ("Pashalik Mons or another Goblin you control dies") that deals 1
/// damage to any target, and a `{3}{R}, Sacrifice a Goblin` token maker. The
/// sacrifice cost is the same chosen-cost primitive Siege-Gang Commander uses.
const PASHALIK_MONS: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "Whenever Pashalik Mons or another Goblin you control dies, Pashalik Mons deals 1 damage to any target.",
        event: TriggerEvent::GoblinYouControlDies,
        optional: false,
        target: TargetSpec::AnyTarget,
        effects: &[Effect::Damage(1)],
    }),
    Ability::Activated(ActivatedAbility {
        text: "{3}{R}, Sacrifice a Goblin: Create two 1/1 red Goblin creature tokens.",
        cost: AbilityCost {
            mana: Some("{3}{R}"),
            sacrifice_creature_subtype: Some("Goblin"),
            ..AbilityCost::NONE
        },
        target: TargetSpec::None,
        modes: SINGLE_MODE,
        effects: &[Effect::CreateGoblinTokens(2)],
        once_per_turn: false,
        sorcery_timing: false,
    }),
];

/// Goblin Trashmaster (M19 144). The anthem is the Chieftain's layer 7c
/// subtype-filtered static with this card's own printed text; the activated
/// ability is a *chosen* Goblin sacrifice with a `target artifact` destroy, so
/// it can never name an enchantment and never picks the sacrifice itself.
const GOBLIN_TRASHMASTER: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "Sacrifice a Goblin: Destroy target artifact.",
    cost: AbilityCost {
        sacrifice_creature_subtype: Some("Goblin"),
        ..AbilityCost::NONE
    },
    target: TargetSpec::Artifact,
    modes: SINGLE_MODE,
    effects: &[Effect::DestroyTargetArtifact],
    once_per_turn: false,
    sorcery_timing: false,
})];

const GOBLIN_TRASHMASTER_STATIC: &[StaticAbility] =
    &[StaticAbility::OtherControlledCreaturesWithAnySubtypeGet {
        subtypes: GOBLIN_SUBTYPES,
        power: 1,
        toughness: 1,
        text: "Other Goblins you control get +1/+1.",
    }];

/// Krenko, Tin Street Kingpin (MOC 287). One attack trigger, two instructions in
/// printed order: the counter lands first and the token count is Krenko's power
/// *after* it, which is why the two cannot be merged into one effect.
const KRENKO_TIN_STREET_KINGPIN: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "Whenever Krenko attacks, put a +1/+1 counter on it, then create a number of 1/1 red Goblin creature tokens equal to Krenko's power.",
    event: TriggerEvent::Attacks,
    optional: false,
    target: TargetSpec::None,
    effects: &[
        Effect::PutPlusOneCounterOnSource,
        Effect::CreateGoblinTokensEqualToSourcePower,
    ],
})];

/// Hobgoblin Bandit Lord (AFR 147). The anthem is the same layer 7c
/// subtype-filtered static the Chieftain uses; the activated ability's damage is
/// the turn's Goblin-entry history for its controller, not the current board, so
/// a Goblin that entered and then died still counts and a Goblin that was
/// already on the battlefield before the turn does not.
const HOBGOBLIN_BANDIT_LORD: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{R}, {T}: This creature deals damage equal to the number of Goblins that entered the battlefield under your control this turn to any target.",
    cost: AbilityCost {
        tap: true,
        mana: Some("{R}"),
        ..AbilityCost::NONE
    },
    target: TargetSpec::AnyTarget,
    modes: SINGLE_MODE,
    effects: &[Effect::DamageTargetForGoblinsEnteredThisTurn],
    once_per_turn: false,
    sorcery_timing: false,
})];

const HOBGOBLIN_BANDIT_LORD_STATIC: &[StaticAbility] =
    &[StaticAbility::OtherControlledCreaturesWithAnySubtypeGet {
        subtypes: GOBLIN_SUBTYPES,
        power: 1,
        toughness: 1,
        text: "Other Goblins you control get +1/+1.",
    }];

/// Goblin Matron (PLST DMR-123). "You may" makes the enters trigger optional
/// (CR 603.5), so declining it happens on the stack before anything is searched,
/// revealed or shuffled.
const GOBLIN_MATRON: &[Ability] = &[Ability::Triggered(TriggeredAbility {
    text: "When this creature enters, you may search your library for a Goblin card, reveal that card, put it into your hand, then shuffle.",
    event: TriggerEvent::EntersBattlefield,
    optional: true,
    target: TargetSpec::None,
    effects: &[Effect::SearchGoblinCardToHand],
})];

/// Siege-Gang Commander (SCG 103 and seventeen later printings). The current
/// printed text costs the throw at `{1}{R}`, not the `{2}{R}` of the original
/// Scourge printing, so the catalog carries the current wording fetched from
/// Scryfall on 2026-10-29. "Sacrifice a Goblin" is a real cost: the activator
/// names the Goblin, the creature is sacrificed as the ability is activated
/// (so its dies triggers fire before the damage), and the Commander may
/// sacrifice itself because it prints the subtype.
const SIEGE_GANG_COMMANDER: &[Ability] = &[
    Ability::Triggered(TriggeredAbility {
        text: "When this creature enters, create three 1/1 red Goblin creature tokens.",
        event: TriggerEvent::EntersBattlefield,
        optional: false,
        target: TargetSpec::None,
        effects: &[Effect::CreateGoblinTokens(3)],
    }),
    Ability::Activated(ActivatedAbility {
        text: "{1}{R}, Sacrifice a Goblin: This creature deals 2 damage to any target.",
        cost: AbilityCost {
            mana: Some("{1}{R}"),
            sacrifice_creature_subtype: Some("Goblin"),
            ..AbilityCost::NONE
        },
        target: TargetSpec::AnyTarget,
        modes: SINGLE_MODE,
        effects: &[Effect::Damage(2)],
        once_per_turn: false,
        sorcery_timing: false,
    }),
];

/// Moggcatcher (NEM 96): "{3}, {T}: Search your library for a Goblin
/// permanent card, put it onto the battlefield, then shuffle."
///
/// CR 701.23: the search is a real search of the whole library, answered as a
/// resumable pending choice (one offer per Goblin permanent card plus a
/// "finish searching" offer that finds nothing). The found card is put onto the
/// battlefield rather than revealed into hand, so it enters through the ordinary
/// entry path and its entry triggers fire (CR 603.6a). CR 110.4a is why the
/// filter is "permanent card": a Goblin instant or sorcery is not eligible.
const MOGGCATCHER: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{3}, {T}: Search your library for a Goblin permanent card, put it onto the battlefield, then shuffle.",
    cost: AbilityCost {
        tap: true,
        mana: Some("{3}"),
        ..AbilityCost::NONE
    },
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::SearchGoblinPermanentToBattlefield],
    once_per_turn: false,
    sorcery_timing: false,
})];

const SPIDER_HAM_STATIC: &[StaticAbility] = &[
    StaticAbility::OtherControlledCreaturesWithAnySubtypeGet {
        subtypes: SPIDER_HAM_SUBTYPES,
        power: 1,
        toughness: 1,
        text: "Animal May-Ham — Other Spiders, Boars, Bats, Bears, Birds, Cats, Dogs, Frogs, Jackals, Lizards, Mice, Otters, Rabbits, Raccoons, Rats, Squirrels, Turtles, and Wolves you control get +1/+1.",
    },
];

/// Every static ability the rules layer knows for a card, in printed order.
pub fn static_abilities(card: CardId) -> &'static [StaticAbility] {
    match card {
        CardId::GoblinChieftain => GOBLIN_CHIEFTAIN_STATIC,
        CardId::GoblinSharpshooter => GOBLIN_SHARPSHOOTER_STATIC,
        CardId::GoblinTrashmaster => GOBLIN_TRASHMASTER_STATIC,
        CardId::HobgoblinBanditLord => HOBGOBLIN_BANDIT_LORD_STATIC,
        CardId::GoblinWarchief => GOBLIN_WARCHIEF_STATIC,
        CardId::GoblinKing => GOBLIN_KING_STATIC,
        CardId::DragonlordsServant => DRAGONLORDS_SERVANT_STATIC,
        CardId::SpiderHamPeterPorker => SPIDER_HAM_STATIC,
        CardId::DruidSFamiliar => DRUIDS_FAMILIAR_STATIC,
        CardId::BeornTheFierce => BEORN_STATIC,
        CardId::WightOfPrecinctSix => WIGHT,
        CardId::LumraBellowOfTheWoods => LUMRA_STATIC,
        CardId::SurrakAndGoreclaw => SURRAK_STATIC,
        CardId::SpiritOfTheAldergard => SPIRIT_STATIC,
        CardId::AngrathsMarauders => ANGRATH,
        CardId::Werebear => WEREBEAR,
        CardId::GoreclawTerrorOfQalSisma => GORECLAW_STATIC,
        CardId::CircleOfTheMoonDruid => CIRCLE_OF_THE_MOON_DRUID,
        CardId::RuxaPatientProfessor => RUXA_STATIC,
        _ => NO_STATICS,
    }
}
