//! VersionVector and Dot — causal context tracking.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Unique identifier for a replica.
pub type ReplicaId = String;

/// A logical dot: a (replica, counter) pair representing a single event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dot {
    pub replica: ReplicaId,
    pub counter: u64,
}

/// A version vector tracking the latest known counter for each replica.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VersionVector {
    map: BTreeMap<ReplicaId, u64>,
}

impl VersionVector {
    /// Create an empty version vector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment the counter for `replica` and return the new dot.
    pub fn inc(&mut self, replica: &str) -> Dot {
        let counter = self.map.entry(replica.to_string()).or_insert(0);
        *counter += 1;
        Dot {
            replica: replica.to_string(),
            counter: *counter,
        }
    }

    /// Get the current counter for a replica (0 if unseen).
    pub fn get(&self, replica: &str) -> u64 {
        self.map.get(replica).copied().unwrap_or(0)
    }

    /// Check whether this version vector has seen the given dot.
    pub fn contains(&self, dot: &Dot) -> bool {
        self.get(&dot.replica) >= dot.counter
    }

    /// Merge another version vector into this one (point-wise max).
    pub fn merge(&mut self, other: &VersionVector) {
        for (replica, &counter) in &other.map {
            let entry = self.map.entry(replica.clone()).or_insert(0);
            *entry = (*entry).max(counter);
        }
    }

    /// Return a new version vector containing only the entries
    /// where `self` is ahead of `other`.
    pub fn delta_since(&self, other: &VersionVector) -> VersionVector {
        let mut delta = VersionVector::new();
        for (replica, &counter) in &self.map {
            let other_counter = other.get(replica);
            if counter > other_counter {
                delta.map.insert(replica.clone(), counter);
            }
        }
        delta
    }

    /// Return an iterator over (replica, counter) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, u64)> {
        self.map.iter().map(|(r, &c)| (r.as_str(), c))
    }

    /// Check if this version vector is empty (no events seen).
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}
