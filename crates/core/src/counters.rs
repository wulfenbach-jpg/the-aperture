//! Typed counters (CR 122).
//!
//! Counters are a permanent's attribute, not a card's: they live on the
//! per-zone object, are removed the moment that object leaves the battlefield
//! (CR 122.2), and never follow a card into another zone. Power/toughness
//! counters change the permanent's actual characteristics (CR 613.4c), so
//! combat, state-based actions and every projection read them through
//! [`crate::state::Object::power`] / [`crate::state::Object::toughness`].
//!
//! The supported counter kinds are exactly the two power/toughness counters this
//! slice models. The 28-card practice pool prints no card that creates a counter
//! (creation from card text is corpus work), so these kinds are driven through
//! [`Counters`] itself. A counter kind the rules layer does not model is
//! deliberately absent: this crate refuses unmodelled rules rather than
//! approximating them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One supported counter kind. Ordinal order is the canonical serialized order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CounterKind {
    /// +1/+1 (CR 122.1a): +1 power and +1 toughness per counter.
    PlusOnePlusOne,
    /// -1/-1 (CR 122.1a): -1 power and -1 toughness per counter.
    MinusOneMinusOne,
}

impl CounterKind {
    pub const ALL: [CounterKind; 2] = [CounterKind::PlusOnePlusOne, CounterKind::MinusOneMinusOne];

    pub fn name(self) -> &'static str {
        match self {
            CounterKind::PlusOnePlusOne => "+1/+1",
            CounterKind::MinusOneMinusOne => "-1/-1",
        }
    }

    pub fn power_delta(self) -> i32 {
        match self {
            CounterKind::PlusOnePlusOne => 1,
            CounterKind::MinusOneMinusOne => -1,
        }
    }

    pub fn toughness_delta(self) -> i32 {
        self.power_delta()
    }
}

impl std::fmt::Display for CounterKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Counters of every kind on one permanent, canonical by [`CounterKind`].
///
/// A kind with zero counters is never stored, so two permanents with the same
/// counters serialize identically. `Default` is "no counters".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Counters {
    counts: BTreeMap<CounterKind, u32>,
}

/// Deserialization enforces the same canonical form the engine writes. A save
/// that stores a zero-count kind, an unknown kind or a negative count is
/// refused instead of being silently coerced into an unequal state: a save file
/// never gets to describe counters the rules cannot produce.
impl<'de> Deserialize<'de> for Counters {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let counts = BTreeMap::<CounterKind, u32>::deserialize(deserializer)?;
        if let Some((kind, _)) = counts.iter().find(|(_, count)| **count == 0) {
            return Err(serde::de::Error::custom(format!(
                "counter kind {kind} stores a zero count; the canonical form omits zero kinds"
            )));
        }
        Ok(Self { counts })
    }
}

impl Counters {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, kind: CounterKind) -> u32 {
        self.counts.get(&kind).copied().unwrap_or(0)
    }

    /// CR 122.1: put `amount` counters of `kind` on this permanent. Returns the
    /// new total for that kind. Adding zero counters is not an event.
    pub fn add(&mut self, kind: CounterKind, amount: u32) -> u32 {
        if amount == 0 {
            return self.get(kind);
        }
        let total = self.get(kind).saturating_add(amount);
        self.counts.insert(kind, total);
        total
    }

    /// CR 122.3: remove up to `amount` counters of `kind`. Removing a counter
    /// that is not there does nothing (CR 122.6); the return value is the number
    /// actually removed.
    pub fn remove(&mut self, kind: CounterKind, amount: u32) -> u32 {
        let held = self.get(kind);
        let removed = held.min(amount);
        if removed == 0 {
            return 0;
        }
        let left = held - removed;
        if left == 0 {
            self.counts.remove(&kind);
        } else {
            self.counts.insert(kind, left);
        }
        removed
    }

    /// CR 122.2: every counter ceases to exist with the object that carried it.
    pub fn clear(&mut self) {
        self.counts.clear();
    }

    /// CR 704.5q: if a permanent has both +1/+1 and -1/-1 counters on it, N of
    /// each are removed, where N is the smaller of the two counts. Returns the
    /// number removed of each kind, or `None` when the permanent does not hold
    /// both kinds, so callers log only real state-based actions.
    pub fn annihilate(&mut self) -> Option<u32> {
        let n = self
            .get(CounterKind::PlusOnePlusOne)
            .min(self.get(CounterKind::MinusOneMinusOne));
        if n == 0 {
            return None;
        }
        self.remove(CounterKind::PlusOnePlusOne, n);
        self.remove(CounterKind::MinusOneMinusOne, n);
        Some(n)
    }

    pub fn is_empty(&self) -> bool {
        self.counts.is_empty()
    }

    pub fn total(&self) -> u32 {
        self.counts.values().copied().sum()
    }

    /// Canonical iteration: `+1/+1` before `-1/-1`.
    pub fn iter(&self) -> impl Iterator<Item = (CounterKind, u32)> + '_ {
        self.counts.iter().map(|(kind, count)| (*kind, *count))
    }

    /// Total power modification from counters (CR 613.4c).
    pub fn power_delta(&self) -> i32 {
        self.counts
            .iter()
            .map(|(kind, count)| kind.power_delta() * *count as i32)
            .sum()
    }

    /// Total toughness modification from counters (CR 613.4c).
    pub fn toughness_delta(&self) -> i32 {
        self.counts
            .iter()
            .map(|(kind, count)| kind.toughness_delta() * *count as i32)
            .sum()
    }

    /// Compact printable form, e.g. `2 +1/+1, 1 -1/-1`. Empty when none.
    pub fn describe(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        for (kind, count) in self.iter() {
            parts.push(format!("{count} {}", kind.name()));
        }
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_store_only_nonzero_kinds_and_sum_canonically() {
        let mut counters = Counters::new();
        assert!(counters.is_empty());
        assert_eq!(counters.add(CounterKind::PlusOnePlusOne, 0), 0);
        assert!(counters.is_empty(), "adding zero counters stores nothing");
        assert_eq!(counters.add(CounterKind::PlusOnePlusOne, 2), 2);
        assert_eq!(counters.add(CounterKind::MinusOneMinusOne, 1), 1);
        assert_eq!(counters.get(CounterKind::PlusOnePlusOne), 2);
        assert_eq!(counters.total(), 3);
        assert_eq!(counters.power_delta(), 1);
        assert_eq!(counters.toughness_delta(), 1);
        assert_eq!(counters.describe(), "2 +1/+1, 1 -1/-1");
        assert_eq!(
            counters.iter().map(|(k, _)| k).collect::<Vec<_>>(),
            vec![CounterKind::PlusOnePlusOne, CounterKind::MinusOneMinusOne]
        );
    }

    #[test]
    fn removing_more_counters_than_held_clamps_and_never_goes_negative() {
        let mut counters = Counters::new();
        assert_eq!(counters.remove(CounterKind::MinusOneMinusOne, 3), 0);
        counters.add(CounterKind::MinusOneMinusOne, 2);
        assert_eq!(counters.remove(CounterKind::MinusOneMinusOne, 5), 2);
        assert_eq!(counters.get(CounterKind::MinusOneMinusOne), 0);
        assert!(
            counters.is_empty(),
            "a kind reduced to zero is not stored at all"
        );
        assert_eq!(counters.power_delta(), 0);
    }

    #[test]
    fn paired_power_counters_annihilate_equally() {
        // CR 704.5q
        let mut counters = Counters::new();
        counters.add(CounterKind::PlusOnePlusOne, 2);
        counters.add(CounterKind::MinusOneMinusOne, 1);
        assert_eq!(counters.annihilate(), Some(1));
        assert_eq!(counters.get(CounterKind::PlusOnePlusOne), 1);
        assert_eq!(counters.get(CounterKind::MinusOneMinusOne), 0);
        assert_eq!(counters.annihilate(), None, "only one kind is left");
        assert_eq!(counters.describe(), "1 +1/+1");

        let mut balanced = Counters::new();
        balanced.add(CounterKind::PlusOnePlusOne, 1);
        balanced.add(CounterKind::MinusOneMinusOne, 1);
        assert_eq!(balanced.annihilate(), Some(1));
        assert!(balanced.is_empty(), "equal counts leave nothing behind");
    }

    #[test]
    fn deserialization_refuses_non_canonical_counters() {
        // A save may not describe counters the engine would never store.
        let zero: Result<Counters, _> = serde_json::from_str(r#"{"PlusOnePlusOne":0}"#);
        let zero = zero.expect_err("a zero-count kind is not canonical");
        assert!(
            zero.to_string().contains("zero count"),
            "the refusal names the reason: {zero}"
        );
        let unknown: Result<Counters, _> = serde_json::from_str(r#"{"Poison":1}"#);
        assert!(
            unknown.is_err(),
            "an unmodelled counter kind is refused, never coerced"
        );
        let negative: Result<Counters, _> = serde_json::from_str(r#"{"MinusOneMinusOne":-1}"#);
        assert!(negative.is_err(), "a negative count is refused");
        let empty: Counters = serde_json::from_str("{}").expect("no counters is canonical");
        assert!(empty.is_empty());
    }

    #[test]
    fn serde_round_trip_is_canonical() {
        let mut counters = Counters::new();
        counters.add(CounterKind::MinusOneMinusOne, 1);
        counters.add(CounterKind::PlusOnePlusOne, 3);
        let json = serde_json::to_string(&counters).expect("counters serialize");
        let back: Counters = serde_json::from_str(&json).expect("counters deserialize");
        assert_eq!(counters, back);
        assert_eq!(serde_json::to_string(&back).unwrap(), json);
    }
}
