//! MV-Register — Multi-Value Register for JSON scalars.
//!
//! A multi-value register keeps all concurrently written values.
//! When a write causally supersedes all previous writes, only the new value remains.

use serde::{Deserialize, Serialize};

use crate::vv::{Dot, VersionVector};

/// An entry in the register: a value tagged with the dot that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Entry<V> {
    dot: Dot,
    value: V,
}

/// Multi-Value Register.
///
/// Stores all concurrently written values. A causal write removes all
/// entries the writer has already seen and adds the new value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MvRegister<V> {
    entries: Vec<Entry<V>>,
}

impl<V: Clone + PartialEq + Eq> MvRegister<V> {
    /// Create an empty register.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Set a new value. `dot` is the fresh dot for this write, `vv` is the
    /// writer's version vector *before* the write (used to remove causally
    /// dominated entries).
    pub fn set(&mut self, value: V, dot: Dot, vv: &VersionVector) {
        // Remove all entries whose dot is dominated by the writer's VV
        self.entries.retain(|e| !vv.contains(&e.dot));
        self.entries.push(Entry { dot, value });
    }

    /// Merge another register into this one.
    ///
    /// Keep entries that are NOT dominated by the other side, plus all
    /// entries from the other side that are NOT dominated by this side.
    pub fn merge(&mut self, other: &MvRegister<V>) {
        // Build a combined VV from our entries
        let self_vv = self.entries_vv();
        let other_vv = other.entries_vv();

        // Keep our entries not dominated by other
        let mut merged: Vec<Entry<V>> = self
            .entries
            .iter()
            .filter(|e| !other_vv.contains(&e.dot))
            .cloned()
            .collect();

        // Add other's entries not dominated by us
        for e in &other.entries {
            if !self_vv.contains(&e.dot) {
                merged.push(e.clone());
            }
        }

        // Keep entries from both sides that exist in both (seen by both VVs)
        for e in &self.entries {
            if other_vv.contains(&e.dot) && !merged.iter().any(|m| m.dot == e.dot) {
                merged.push(e.clone());
            }
        }

        // Sort for determinism
        merged.sort_by(|a, b| {
            a.dot
                .replica
                .cmp(&b.dot.replica)
                .then(a.dot.counter.cmp(&b.dot.counter))
        });
        merged.dedup_by(|a, b| a.dot == b.dot);

        self.entries = merged;
    }

    /// Return all current values (one per concurrent write).
    pub fn values(&self) -> Vec<&V> {
        self.entries.iter().map(|e| &e.value).collect()
    }

    /// Return the single value if there's exactly one, `None` if empty or conflicting.
    pub fn value(&self) -> Option<&V> {
        if self.entries.len() == 1 {
            Some(&self.entries[0].value)
        } else {
            None
        }
    }

    /// Check if the register is empty (no values set).
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Build a version vector from the dots of the current entries.
    fn entries_vv(&self) -> VersionVector {
        let mut vv = VersionVector::new();
        for e in &self.entries {
            // Ensure VV tracks at least this dot
            while vv.get(&e.dot.replica) < e.dot.counter {
                vv.inc(&e.dot.replica);
            }
        }
        vv
    }
}

impl<V: Clone + PartialEq + Eq> Default for MvRegister<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + PartialEq + Eq> PartialEq for MvRegister<V> {
    fn eq(&self, other: &Self) -> bool {
        // Canonical comparison: sorted entries by dot
        let mut a = self.entries.clone();
        let mut b = other.entries.clone();
        a.sort_by(|x, y| {
            x.dot
                .replica
                .cmp(&y.dot.replica)
                .then(x.dot.counter.cmp(&y.dot.counter))
        });
        b.sort_by(|x, y| {
            x.dot
                .replica
                .cmp(&y.dot.replica)
                .then(x.dot.counter.cmp(&y.dot.counter))
        });
        a == b
    }
}

impl<V: Clone + PartialEq + Eq> Eq for MvRegister<V> {}
