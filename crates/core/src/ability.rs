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
//! static abilities, replacement effects, intervening-"if" clauses, delayed or
//! reflexive triggers, and control changes.

use crate::card::{CardId, Effect, TargetSpec};

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
}

impl TriggerEvent {
    pub fn name(self) -> &'static str {
        match self {
            TriggerEvent::EntersBattlefield => "enters the battlefield",
            TriggerEvent::Dies => "dies",
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
}

impl AbilityCost {
    pub const TAP: AbilityCost = AbilityCost {
        tap: true,
        sacrifice_source: false,
        mana: None,
    };
    pub const SACRIFICE_SELF: AbilityCost = AbilityCost {
        tap: false,
        sacrifice_source: true,
        mana: None,
    };
    pub const fn mana(cost: &'static str) -> AbilityCost {
        AbilityCost {
            tap: false,
            sacrifice_source: false,
            mana: Some(cost),
        }
    }

    pub fn is_free(self) -> bool {
        !self.tap && !self.sacrifice_source && self.mana.is_none()
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
})];

const ROOTWALLA: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "{1}{G}: Rootwalla gets +2/+2 until end of turn. Activate only once each turn.",
    cost: AbilityCost::mana("{1}{G}"),
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::PumpSource(2, 2)],
    once_per_turn: true,
})];

const BRINDLE_BOAR: &[Ability] = &[Ability::Activated(ActivatedAbility {
    text: "Sacrifice Brindle Boar: You gain 4 life.",
    cost: AbilityCost::SACRIFICE_SELF,
    target: TargetSpec::None,
    modes: SINGLE_MODE,
    effects: &[Effect::GainLife(4)],
    once_per_turn: false,
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

/// Every ability the rules layer knows for a card, in printed order.
pub fn abilities(card: CardId) -> &'static [Ability] {
    match card {
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
