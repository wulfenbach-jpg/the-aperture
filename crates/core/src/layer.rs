//! Typed continuous, replacement and prevention effects.
//!
//! * CR 613 orders continuous effects into layers and sublayers. This starter
//!   set implements five of them, each because a supported card prints it:
//!   layer 4 type-changing effects (`Type`, CR 613.1d, Ursine Champion's
//!   subtype replacement and Stuffed Bear's creature-type addition), layer 5
//!   colour-changing effects (`Color`, CR 613.1e, Stuffed Bear's green
//!   animation), layer 6 ability grants (`Ability`, CR 613.4d, Goreclaw),
//!   layer 7b base power/toughness setting (`Setting`, CR 613.4b, Exuberant
//!   Wolfbear, Circle of the Moon Druid and Stuffed Bear) and layer 7c
//!   power/toughness modifiers (`Modifying`, CR 613.4c, including printed
//!   count-based statics such as Wight of Precinct Six). Layer 7a
//!   (characteristic-defining power/toughness) and 7d (+1/+1 counters) are not
//!   implemented because no supported card needs them; they are refused at
//!   import rather than approximated. Layer 7d is deliberately *not* applied
//!   here: counters remain display-only until the counters lane makes them
//!   characteristic-changing, so this lane never claims 7d coverage.
//! * CR 614 replacement effects use "instead". Every effect applies at most
//!   once to a single event (CR 614.5).
//! * CR 615 prevention effects are one-shot shields with a finite amount or an
//!   unlimited amount for the rest of the turn.
//! * CR 616.1: when more than one replacement/prevention effect applies to the
//!   same event, the affected player (or the controller of the affected object)
//!   chooses the order. The engine acts for that chooser with a documented,
//!   deterministic policy (see `Game::damage_order`) and records the chooser in
//!   the `ReplacementApplied` / `DamagePrevented` events, so no external agent
//!   can pick a different order after the fact.

use serde::{Deserialize, Serialize};

use crate::card::{CardId, ManaColor};
use crate::ids::{ObjectId, PlayerId};

/// CR 613.4 layer/sublayer of a continuous effect. Ordered: an effect in an
/// earlier layer is applied before an effect in a later one.
///
/// Five layers are implemented, each arrived at by a card that prints it:
/// `Type` (4, Ursine Champion and Stuffed Bear), `Color` (5, Stuffed Bear),
/// `Ability` (6, Goreclaw), `Setting` (7b, Exuberant Wolfbear, Circle of the
/// Moon Druid and Stuffed Bear) and `Modifying` (7c, the printed count-based
/// statics and pumps). Layer 7a (characteristic-defining power/toughness) was
/// dropped with issue 15's review: its only candidate card, Wight of Precinct
/// Six, prints an additive "+1/+1 for each ..." static (layer 7c), not a
/// characteristic-defining ability, so keeping a 7a code path would have meant
/// shipping untested production code for no card. Layer 7d (+1/+1 counters)
/// is likewise not implemented here; 7d belongs to the counters lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Layer {
    /// CR 613.1d: type-changing effects. Slice 26 adds the first one the pool
    /// needs — "becomes a Bear Berserker until end of turn" (Ursine Champion).
    /// A layer 4 effect replaces the creature's subtype line for as long as it
    /// applies, so every subtype query must read `Game::has_subtype` rather
    /// than the printed type line.
    Type,
    /// CR 613.1e: colour-changing effects. Slice 29 adds the first one the pool
    /// needs — Stuffed Bear's animation makes the artifact green. A layer 5
    /// effect *sets* the object's colour, so every colour question must read
    /// `Game::has_color` rather than the printed colour.
    Color,
    /// CR 613.4d: effects that grant or remove abilities. Slice 23 adds the
    /// first one the pool needs — a until-end-of-turn trample grant (Goreclaw,
    /// Terror of Qal Sisma). It is ordered before `Modifying`, so a pump granted
    /// in the same resolution still applies after the ability grant.
    Ability,
    /// CR 613.4b: effects that set base power/toughness, before modifiers.
    Setting,
    /// CR 613.4c: effects that modify power and/or toughness without setting
    /// them, applied in timestamp order (CR 613.7).
    Modifying,
}

impl Layer {
    pub fn label(self) -> &'static str {
        match self {
            Layer::Type => "4",
            Layer::Color => "5",
            Layer::Ability => "6",
            Layer::Setting => "7b",
            Layer::Modifying => "7c",
        }
    }

    pub fn rules(self) -> &'static str {
        match self {
            Layer::Type => "CR 613.1d type-changing effects",
            Layer::Color => "CR 613.1e colour-changing effects",
            Layer::Ability => "CR 613.4d ability-granting effects",
            Layer::Setting => "CR 613.4b base power/toughness setting",
            Layer::Modifying => "CR 613.4c power/toughness modifiers",
        }
    }
}

/// The creature subtype line a layer 4 effect installs. Only the lines the pool
/// actually prints are represented; a new "becomes a <type>" card adds a
/// variant here, never a free-form string, so the effect stays typed and
/// replay-serializable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubtypeLine {
    /// Ursine Champion: "becomes a Bear Berserker until end of turn".
    BearBerserker,
    /// Circle of the Moon Druid: "is a Bear ... (It loses all other creature
    /// types.)" — a printed static line, not an activated one.
    Bear,
}

impl SubtypeLine {
    /// Printed subtype line, exactly as the card writes it.
    pub fn text(self) -> &'static str {
        match self {
            SubtypeLine::BearBerserker => "Bear Berserker",
            SubtypeLine::Bear => "Bear",
        }
    }

    /// Whether the effective subtype line carries `subtype` (CR 205.3).
    pub fn has(self, subtype: &str) -> bool {
        self.text().split_whitespace().any(|part| part == subtype)
    }
}

/// How long a continuous effect lasts. A `UntilEndOfTurn` effect keeps applying
/// even if its source object leaves the battlefield (CR 611.2a); only the
/// cleanup step removes it (CR 514.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectDuration {
    UntilEndOfTurn,
    WhileSourceOnBattlefield,
}

/// What a continuous effect does once its layer is being applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContinuousKind {
    /// CR 613.4c: add to power and toughness. Both values are stored so a
    /// -1/-1 effect is never confused with a +1/+1 counter.
    ModifyPowerToughness { power: i32, toughness: i32 },
    /// CR 613.4d + 702.19a: the target permanent has trample for as long as this
    /// effect applies. Combat damage assignment asks the engine, not the printed
    /// card, so a granted trample is indistinguishable from printed trample at
    /// the point the rule is executed.
    GrantsTrample,
    /// CR 613.4d + 702.10a: the object has haste until end of turn.
    GrantsHaste,
    /// CR 613.4b: set base power/toughness to the captured values.
    SetBasePowerToughness { power: i32, toughness: i32 },
    /// CR 613.1d: the creature's subtype line becomes this one, replacing the
    /// printed subtypes for as long as the effect applies. Type-changing
    /// effects apply before every power/toughness layer, so a pump in the same
    /// resolution is unaffected by the new line.
    SetCreatureSubtypes(SubtypeLine),
    /// CR 613.1d: the object *gains* the creature card type and this subtype
    /// line in addition to its printed types (Stuffed Bear's "becomes a 4/4
    /// green Bear artifact creature"). It stays an artifact, so it is an
    /// artifact creature; nothing about its printed card changes.
    AddCreatureSubtypes(SubtypeLine),
    /// CR 613.1e: the object's colour becomes this one for as long as the
    /// effect applies (Stuffed Bear's "green"). It is a *set*, not an addition,
    /// so the object is exactly this colour and no other.
    SetColor(ManaColor),
}

/// One continuous effect, carrying the timestamp that fixes its order inside
/// its layer (CR 613.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuousEffect {
    pub id: u32,
    /// Monotonic rule-visible timestamp; the engine never uses wall-clock time.
    pub timestamp: u64,
    pub source: Option<ObjectId>,
    pub card: CardId,
    pub controller: PlayerId,
    pub layer: Layer,
    pub target: ObjectId,
    pub kind: ContinuousKind,
    pub duration: EffectDuration,
}

impl ContinuousEffect {
    pub fn describe(&self) -> String {
        match self.kind {
            ContinuousKind::ModifyPowerToughness { power, toughness } => format!(
                "{:+}/{:+} until end of turn ({})",
                power,
                toughness,
                self.layer.rules()
            ),
            ContinuousKind::GrantsTrample => {
                format!("trample until end of turn ({})", self.layer.rules())
            }
            ContinuousKind::GrantsHaste => {
                format!("haste until end of turn ({})", self.layer.rules())
            }
            ContinuousKind::SetBasePowerToughness { power, toughness } => {
                format!(
                    "base power/toughness {power}/{toughness} until end of turn ({})",
                    self.layer.rules()
                )
            }
            ContinuousKind::SetCreatureSubtypes(line) => format!(
                "creature type becomes {} until end of turn ({})",
                line.text(),
                self.layer.rules()
            ),
            ContinuousKind::AddCreatureSubtypes(line) => format!(
                "becomes a creature with type {} until end of turn ({})",
                line.text(),
                self.layer.rules()
            ),
            ContinuousKind::SetColor(color) => format!(
                "becomes {} until end of turn ({})",
                color.name(),
                self.layer.rules()
            ),
        }
    }
}

/// The object or player a damage event is dealt to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageTarget {
    Player(PlayerId),
    Permanent(ObjectId),
}

/// Which damage instances a prevention shield covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShieldScope {
    /// Any damage: combat, spell and ability damage alike.
    AnyDamage,
    /// Combat damage only (Fog-style full prevention keeps its own flag; this
    /// scope exists for a bounded partial shield on combat damage).
    CombatDamageOnly,
}

/// What a prevention shield protects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShieldTarget {
    Any,
    Player(PlayerId),
    Permanent(ObjectId),
}

impl ShieldTarget {
    pub fn matches(self, target: DamageTarget) -> bool {
        match (self, target) {
            (ShieldTarget::Any, _) => true,
            (ShieldTarget::Player(a), DamageTarget::Player(b)) => a == b,
            (ShieldTarget::Permanent(a), DamageTarget::Permanent(b)) => a == b,
            _ => false,
        }
    }

    pub fn label(self, game: &crate::state::GameState) -> String {
        match self {
            ShieldTarget::Any => "any target".to_string(),
            ShieldTarget::Player(p) => format!("P{p}"),
            ShieldTarget::Permanent(id) => game
                .try_object(id)
                .map(|object| format!("{} ({})", object.card.name(), id))
                .unwrap_or_else(|| "a permanent that left the battlefield".to_string()),
        }
    }
}

/// A one-shot prevention shield. `remaining: None` prevents all applicable
/// damage for the rest of the turn (Fog-style); `Some(n)` prevents `n` more
/// points and then no longer applies (CR 615.1, 615.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageShield {
    pub id: u32,
    pub timestamp: u64,
    pub source: Option<ObjectId>,
    pub card: CardId,
    /// Controller of the effect that created the shield.
    pub controller: PlayerId,
    pub scope: ShieldScope,
    pub target: ShieldTarget,
    pub remaining: Option<u32>,
    /// Turn the shield was created on; it expires in that turn's cleanup step.
    pub created_turn: u32,
}

impl DamageShield {
    pub fn applies_to(&self, target: DamageTarget, combat: bool) -> bool {
        if self.scope == ShieldScope::CombatDamageOnly && !combat {
            return false;
        }
        if self.remaining == Some(0) {
            return false;
        }
        self.target.matches(target)
    }

    /// Consume as much of `amount` as the shield can prevent and return the
    /// prevented part.
    pub fn absorb(&mut self, amount: u32) -> u32 {
        match self.remaining {
            None => amount,
            Some(remaining) => {
                let prevented = remaining.min(amount);
                self.remaining = Some(remaining - prevented);
                prevented
            }
        }
    }

    pub fn exhausted(&self) -> bool {
        self.remaining == Some(0)
    }

    pub fn amount_text(&self) -> String {
        match self.remaining {
            None => "all damage".to_string(),
            Some(0) => "0 damage".to_string(),
            Some(n) => format!("the next {n} damage"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_shield_absorbs_only_its_remaining_amount() {
        let mut shield = DamageShield {
            id: 1,
            timestamp: 1,
            source: None,
            card: CardId::MendingHands,
            controller: 0,
            scope: ShieldScope::AnyDamage,
            target: ShieldTarget::Player(1),
            remaining: Some(4),
            created_turn: 2,
        };
        assert_eq!(shield.absorb(2), 2);
        assert_eq!(shield.remaining, Some(2));
        assert!(!shield.exhausted());
        assert_eq!(shield.absorb(3), 2);
        assert!(shield.exhausted());
        assert!(!shield.applies_to(DamageTarget::Player(1), false));
        // Scope and target never widen.
        assert!(!shield.applies_to(DamageTarget::Player(0), false));
    }

    #[test]
    fn unlimited_shield_prevents_every_applicable_instance() {
        let mut shield = DamageShield {
            id: 2,
            timestamp: 1,
            source: None,
            card: CardId::Fog,
            controller: 0,
            scope: ShieldScope::AnyDamage,
            target: ShieldTarget::Any,
            remaining: None,
            created_turn: 2,
        };
        assert_eq!(shield.absorb(7), 7);
        assert_eq!(shield.absorb(1), 1);
        assert!(!shield.exhausted());

        shield.scope = ShieldScope::CombatDamageOnly;
        assert!(shield.applies_to(DamageTarget::Player(3), true));
        assert!(!shield.applies_to(DamageTarget::Player(3), false));
    }

    #[test]
    fn modifier_effects_describe_themselves_in_layer_7c() {
        let effect = ContinuousEffect {
            id: 7,
            timestamp: 12,
            source: None,
            card: CardId::GiantGrowth,
            controller: 0,
            layer: Layer::Modifying,
            target: ObjectId(3),
            kind: ContinuousKind::ModifyPowerToughness {
                power: 3,
                toughness: -1,
            },
            duration: EffectDuration::UntilEndOfTurn,
        };
        assert_eq!(effect.layer.label(), "7c");
        assert_eq!(
            effect.describe(),
            "+3/-1 until end of turn (CR 613.4c power/toughness modifiers)"
        );
    }
}
