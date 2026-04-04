//! OR-Map — Observed-Remove Map for JSON objects.
//!
//! An OR-Map associates keys with values and tracks causal context for
//! add/remove operations. Add-wins semantics: a concurrent add and remove
//! of the same key results in the key being present.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::vv::{Dot, VersionVector};

/// An entry in the OR-Map: a value tagged with the set of dots that justify its presence.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Entry<V> {
    /// Each dot paired with the value it introduced.
    items: Vec<(Dot, V)>,
}

impl<V: Clone + PartialEq> Entry<V> {
    fn dots(&self) -> impl Iterator<Item = &Dot> {
        self.items.iter().map(|(d, _)| d)
    }

    /// The deterministic "current" value: from the highest dot (by replica, then counter).
    fn value(&self) -> Option<&V> {
        self.items
            .iter()
            .max_by(|(a, _), (b, _)| a.replica.cmp(&b.replica).then(a.counter.cmp(&b.counter)))
            .map(|(_, v)| v)
    }
}

/// Observed-Remove Map.
///
/// Keys are associated with values and tracked by dots. When a key is removed,
/// only the dots the remover has seen are removed. If a concurrent add
/// introduces a new dot, the key survives (add-wins).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrMap<K: Ord + Clone, V: Clone> {
    entries: BTreeMap<K, Entry<V>>,
    /// Tracks all dots that have ever been seen (adds + removes).
    clock: VersionVector,
}

impl<K: Ord + Clone + Serialize, V: Clone + PartialEq> OrMap<K, V> {
    /// Create an empty OR-Map.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            clock: VersionVector::new(),
        }
    }

    /// Put a key-value pair. The `dot` is the fresh dot for this operation.
    pub fn put(&mut self, key: K, value: V, dot: Dot) {
        self.clock.inc_to(&dot);
        let entry = self
            .entries
            .entry(key)
            .or_insert_with(|| Entry { items: Vec::new() });
        entry.items.push((dot, value));
    }

    /// Remove a key. Only removes dots the caller has seen (via `vv`).
    /// Returns true if the key was present and removed.
    pub fn remove(&mut self, key: &K, vv: &VersionVector) -> bool {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.items.retain(|(d, _)| !vv.contains(d));
            if entry.items.is_empty() {
                self.entries.remove(key);
                return true;
            }
        }
        false
    }

    /// Get a reference to the value for a key (deterministic: highest dot wins).
    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key).and_then(|e| e.value())
    }

    /// Get a mutable reference to the entry's items for a key.
    pub fn get_entry_mut(&mut self, key: &K) -> Option<&mut Vec<(Dot, V)>> {
        self.entries.get_mut(key).map(|e| &mut e.items)
    }

    /// Iterate over keys.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.keys()
    }

    /// Check if the map contains a key.
    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Merge another OR-Map into this one.
    ///
    /// For each key:
    /// - Dots present in both: keep
    /// - Dots only in self but known to other's clock: remove (other removed it)
    /// - Dots only in self and NOT known to other's clock: keep (concurrent add)
    /// - Dots only in other but known to self's clock: remove (self removed it)
    /// - Dots only in other and NOT known to self's clock: keep (concurrent add)
    pub fn merge(&mut self, other: &OrMap<K, V>) {
        let self_clock = self.clock.clone();

        // Process keys in other
        for (key, other_entry) in &other.entries {
            if let Some(self_entry) = self.entries.get_mut(key) {
                // Key exists in both — merge item sets
                let mut merged_items: Vec<(Dot, V)> = Vec::new();

                // Keep self items not removed by other
                for (d, v) in &self_entry.items {
                    if other_entry.items.iter().any(|(od, _)| od == d) || !other.clock.contains(d) {
                        merged_items.push((d.clone(), v.clone()));
                    }
                }

                // Add other items not removed by self
                for (d, v) in &other_entry.items {
                    if !self_clock.contains(d) && !merged_items.iter().any(|(md, _)| md == d) {
                        merged_items.push((d.clone(), v.clone()));
                    }
                }

                if merged_items.is_empty() {
                    self.entries.remove(key);
                } else {
                    // Sort for determinism
                    merged_items.sort_by(|(a, _), (b, _)| {
                        a.replica.cmp(&b.replica).then(a.counter.cmp(&b.counter))
                    });
                    self_entry.items = merged_items;
                }
            } else {
                // Key only in other — keep items not seen by self's clock
                let surviving: Vec<(Dot, V)> = other_entry
                    .items
                    .iter()
                    .filter(|(d, _)| !self_clock.contains(d))
                    .cloned()
                    .collect();

                if !surviving.is_empty() {
                    self.entries.insert(key.clone(), Entry { items: surviving });
                }
            }
        }

        // Remove self-only keys whose dots are all known to other
        let keys_to_check: Vec<K> = self
            .entries
            .keys()
            .filter(|k| !other.entries.contains_key(k))
            .cloned()
            .collect();

        for key in keys_to_check {
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.items.retain(|(d, _)| !other.clock.contains(d));
                if entry.items.is_empty() {
                    self.entries.remove(&key);
                }
            }
        }

        self.clock.merge(&other.clock);
    }

    /// Return a reference to the internal clock.
    pub fn clock(&self) -> &VersionVector {
        &self.clock
    }
}

impl<K: Ord + Clone + Serialize, V: Clone + PartialEq> Default for OrMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord + Clone + Serialize, V: Clone + PartialEq> PartialEq for OrMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        if self.entries.len() != other.entries.len() {
            return false;
        }
        for (key, self_entry) in &self.entries {
            if let Some(other_entry) = other.entries.get(key) {
                // Compare sorted item sets
                let mut si: Vec<&Dot> = self_entry.dots().collect();
                let mut oi: Vec<&Dot> = other_entry.dots().collect();
                si.sort_by(|a, b| a.replica.cmp(&b.replica).then(a.counter.cmp(&b.counter)));
                oi.sort_by(|a, b| a.replica.cmp(&b.replica).then(a.counter.cmp(&b.counter)));
                if si != oi {
                    return false;
                }
                // Compare deterministic value
                if self_entry.value() != other_entry.value() {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
}

impl<K: Ord + Clone + Serialize, V: Clone + PartialEq> Eq for OrMap<K, V> {}
