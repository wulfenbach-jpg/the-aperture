//! Typed counters (CR 122).
//!
//! Counters are a permanent's attribute, not a card's: they live on the
//! per-zone object, are removed the moment that object leaves the battlefield
//! (CR 122.2), and never follow a card into another zone. Power/toughness
//! counters change the permanent's actual characteristics (CR 613.4c), so
//! combat, state-based actions and every projection read them through
//! [`crate::state::Object::power`] / [`crate::state::Object::toughness`].
//!
//! The supported counter kinds are exactly the two power/toughness counters the
//! practice pool can produce. A counter kind that no supported card can create
//! is deliberately absent: this crate refuses unmodelled rules rather than
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Counters {
    counts: BTreeMap<CounterKind, u32>,
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
