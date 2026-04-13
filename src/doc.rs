//! CrdtDoc — top-level JSON document API.
//!
//! Provides an ergonomic interface for manipulating a CRDT JSON document.
//! All operations are local; synchronization happens via deltas.

use crate::delta::Delta;
use crate::ormap::OrMap;
use crate::register::MvRegister;
use crate::value::CrdtValue;
use crate::vv::{ReplicaId, VersionVector};

/// A CRDT JSON document.
///
/// Each document belongs to a single replica. Mutations produce deltas
/// that can be sent to other replicas for convergence.
pub struct CrdtDoc {
    replica_id: ReplicaId,
    vv: VersionVector,
    root: OrMap<String, CrdtValue>,
}

impl CrdtDoc {
    /// Create a new empty document for the given replica.
    pub fn new(replica_id: &str) -> Self {
        Self {
            replica_id: replica_id.to_string(),
            vv: VersionVector::new(),
            root: OrMap::new(),
        }
    }

    /// Set a value at a JSON path.
    ///
    /// Path format: `"/key"` or `"/key1/key2/key3"` for nested objects.
    /// Intermediate objects are created automatically.
    pub fn set(&mut self, path: &str, value: serde_json::Value) {
        let segments = parse_path(path);
        if segments.is_empty() {
            return;
        }

        if segments.len() == 1 {
            let dot = self.vv.inc(&self.replica_id);
            let mut reg = MvRegister::new();
            reg.set(value, dot.clone(), &self.vv);
            self.root
                .put(segments[0].to_string(), CrdtValue::Scalar(reg), dot);
        } else {
            self.set_nested(&segments, value);
        }
    }

    /// Set an empty array at the given path.
    pub fn set_array(&mut self, path: &str) {
        let segments = parse_path(path);
        if segments.is_empty() {
            return;
        }

        let dot = self.vv.inc(&self.replica_id);
        if segments.len() == 1 {
            self.root
                .put(segments[0].to_string(), CrdtValue::array(), dot);
        } else {
            // Ensure intermediates, then put array at leaf
            self.set_nested_value(&segments, CrdtValue::array(), dot);
        }
    }

    /// Remove a key at the given path.
    pub fn remove(&mut self, path: &str) {
        let segments = parse_path(path);
        if segments.is_empty() {
            return;
        }

        if segments.len() == 1 {
            let vv = self.vv.clone();
            self.root.remove(&segments[0].to_string(), &vv);
        } else {
            let vv = self.vv.clone();
            let last = segments.last().unwrap().to_string();
            // For nested removes, we must remove from ALL concurrent entries
            // of the parent, not just the "last" one. Multiple replicas may
            // have created the same intermediate key independently, resulting
            // in multiple concurrent OrMap entries for the parent path.
            remove_nested_all(&mut self.root, &segments[..segments.len() - 1], &last, &vv);
        }
    }

    /// Insert a value into an array at the given path and index.
    pub fn array_insert(&mut self, path: &str, index: usize, value: serde_json::Value) {
        let segments = parse_path(path);
        if segments.is_empty() {
            return;
        }

        let replica = self.replica_id.clone();
        let dot = self.vv.inc(&replica);
        let vv = self.vv.clone();

        if let Some(CrdtValue::Array(rga)) = navigate_mut(&mut self.root, &segments) {
            let mut reg = MvRegister::new();
            reg.set(value, dot.clone(), &vv);
            rga.insert(index, CrdtValue::Scalar(reg), dot);
        }
    }

    /// Delete an element from an array at the given path and index.
    pub fn array_delete(&mut self, path: &str, index: usize) {
        let segments = parse_path(path);
        if segments.is_empty() {
            return;
        }

        if let Some(CrdtValue::Array(rga)) = navigate_mut(&mut self.root, &segments) {
            rga.delete(index);
        }
    }

    /// Materialize the document as a plain JSON value.
    pub fn materialize(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        for key in self.root.keys() {
            if let Some(value) = self.root.get_merged(key) {
                obj.insert(key.clone(), value.materialize());
            }
        }
        serde_json::Value::Object(obj)
    }

    /// Get the current version vector.
    pub fn version_vector(&self) -> &VersionVector {
        &self.vv
    }

    /// Get the replica ID.
    pub fn replica_id(&self) -> &str {
        &self.replica_id
    }

    /// Produce a delta containing all state since the given version vector.
    ///
    /// If `since` is empty, returns the full document state.
    /// If `since` matches the current VV, returns an empty delta.
    pub fn delta_since(&self, _since: &VersionVector) -> Delta {
        // For delta-state CRDTs, the delta is the full state.
        // A smarter implementation could filter, but correctness requires
        // that the delta, when merged, brings the remote up to date.
        // The simplest correct approach: send the full root + VV.
        // The merge operation is idempotent, so sending more than needed is safe.
        Delta {
            root: self.root.clone(),
            vv: self.vv.clone(),
        }
    }

    /// Merge a delta from a remote replica into this document.
    pub fn merge_delta(&mut self, delta: &Delta) {
        self.root.merge(&delta.root);
        self.vv.merge(&delta.vv);
    }

    // ─── Internal helpers ───────────────────────────────────

    fn set_nested(&mut self, segments: &[&str], value: serde_json::Value) {
        let dot = self.vv.inc(&self.replica_id);
        let vv = self.vv.clone();
        let mut reg = MvRegister::new();
        reg.set(value, dot.clone(), &vv);
        self.set_nested_value(segments, CrdtValue::Scalar(reg), dot);
    }

    fn set_nested_value(&mut self, segments: &[&str], value: CrdtValue, leaf_dot: crate::vv::Dot) {
        // Ensure all intermediate segments are objects
        for i in 0..segments.len() - 1 {
            let key = segments[i].to_string();
            let needs_create = if i == 0 {
                !self.root.contains_key(&key)
            } else {
                match navigate(&self.root, &segments[..i]) {
                    Some(CrdtValue::Object(map)) => !map.contains_key(&key),
                    _ => false,
                }
            };

            if needs_create {
                let dot = self.vv.inc(&self.replica_id);
                if i == 0 {
                    self.root.put(key, CrdtValue::object(), dot);
                } else if let Some(CrdtValue::Object(map)) =
                    navigate_mut(&mut self.root, &segments[..i])
                {
                    map.put(key, CrdtValue::object(), dot);
                }
            }
        }

        let last_key = segments.last().unwrap().to_string();
        if let Some(CrdtValue::Object(map)) =
            navigate_mut(&mut self.root, &segments[..segments.len() - 1])
        {
            map.put(last_key, value, leaf_dot);
        }
    }
}

/// Navigate to a CrdtValue at the given path (immutable).
fn navigate<'a>(root: &'a OrMap<String, CrdtValue>, segments: &[&str]) -> Option<&'a CrdtValue> {
    if segments.is_empty() {
        return None;
    }

    let first = segments[0].to_string();
    let mut current = root.get(&first)?;

    for &seg in &segments[1..] {
        match current {
            CrdtValue::Object(map) => {
                current = map.get(&seg.to_string())?;
            }
            _ => return None,
        }
    }
    Some(current)
}

/// Navigate to a mutable CrdtValue at the given path.
fn navigate_mut<'a>(
    root: &'a mut OrMap<String, CrdtValue>,
    segments: &[&str],
) -> Option<&'a mut CrdtValue> {
    if segments.is_empty() {
        return None;
    }

    let first_key = segments[0].to_string();
    let items = root.get_entry_mut(&first_key)?;
    let (_, cv) = items.last_mut()?;

    if segments.len() == 1 {
        return Some(cv);
    }

    let mut current = cv;
    for &seg in &segments[1..] {
        match current {
            CrdtValue::Object(map) => {
                let items = map.get_entry_mut(&seg.to_string())?;
                let (_, next) = items.last_mut()?;
                current = next;
            }
            _ => return None,
        }
    }
    Some(current)
}

/// Parse a path like "/a/b/c" into ["a", "b", "c"].
fn parse_path(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

/// Remove a key from ALL concurrent entries at a nested path.
///
/// When multiple replicas create the same intermediate key (e.g. `/nodes`)
/// independently, the OrMap entry for that key has multiple concurrent items.
/// A single `navigate_mut` picks only one, missing the others. This function
/// walks through ALL concurrent items at each level so the remove hits every
/// copy of the target key.
fn remove_nested_all(
    root: &mut OrMap<String, CrdtValue>,
    parent_segments: &[&str],
    leaf_key: &str,
    vv: &VersionVector,
) {
    if parent_segments.is_empty() {
        return;
    }

    let first_key = parent_segments[0].to_string();
    let items = match root.get_entry_mut(&first_key) {
        Some(items) => items,
        None => return,
    };
    let leaf_key = leaf_key.to_string();

    if parent_segments.len() == 1 {
        // We've reached the parent — remove the leaf from all concurrent
        // Object values at this level.
        for (_, cv) in items.iter_mut() {
            if let CrdtValue::Object(map) = cv {
                map.remove(&leaf_key, vv);
            }
        }
    } else {
        // Recurse into all concurrent Object values for the next segment.
        for (_, cv) in items.iter_mut() {
            if let CrdtValue::Object(map) = cv {
                remove_nested_all(map, &parent_segments[1..], &leaf_key, vv);
            }
        }
    }
}
