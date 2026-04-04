//! RGA — Replicated Growable Array for JSON arrays.
//!
//! Elements are identified by unique dots and ordered by their insertion anchor.
//! Deleted elements become tombstones (value removed but position preserved).

use serde::{Deserialize, Serialize};

use crate::vv::{Dot, VersionVector};

/// A single element in the RGA.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RgaNode<V> {
    /// Unique identifier for this element.
    id: Dot,
    /// The anchor: the id of the element after which this was inserted.
    /// `None` means inserted at the head.
    anchor: Option<Dot>,
    /// The value, `None` if tombstoned.
    value: Option<V>,
}

/// Replicated Growable Array.
///
/// Maintains a totally ordered sequence of elements using dot-based IDs.
/// Deleted elements are tombstoned in place to preserve ordering anchors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rga<V: Clone> {
    nodes: Vec<RgaNode<V>>,
    clock: VersionVector,
}

impl<V: Clone + PartialEq> Rga<V> {
    /// Create an empty RGA.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            clock: VersionVector::new(),
        }
    }

    /// Insert a value at `visible_index` (0-based among visible elements).
    /// `dot` is the fresh unique dot for this insert.
    pub fn insert(&mut self, visible_index: usize, value: V, dot: Dot) {
        self.clock.inc_to(&dot);

        // Find the anchor: the node at visible_index - 1 (or None for head)
        let anchor = if visible_index == 0 {
            None
        } else {
            let mut seen = 0;
            let mut anchor_id = None;
            for node in &self.nodes {
                if node.value.is_some() {
                    seen += 1;
                    if seen == visible_index {
                        anchor_id = Some(node.id.clone());
                        break;
                    }
                }
            }
            anchor_id
        };

        let new_node = RgaNode {
            id: dot,
            anchor,
            value: Some(value),
        };

        self.nodes.push(new_node);
        self.reorder();
    }

    /// Delete the element at `visible_index` (tombstone it).
    pub fn delete(&mut self, visible_index: usize) -> bool {
        let mut seen = 0;
        for node in &mut self.nodes {
            if node.value.is_some() {
                if seen == visible_index {
                    node.value = None;
                    return true;
                }
                seen += 1;
            }
        }
        false
    }

    /// Get a reference to the value at `visible_index`.
    pub fn get(&self, visible_index: usize) -> Option<&V> {
        self.visible_iter().nth(visible_index)
    }

    /// Number of visible (non-tombstoned) elements.
    pub fn len(&self) -> usize {
        self.nodes.iter().filter(|n| n.value.is_some()).count()
    }

    /// Check if empty (no visible elements).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Iterate over visible values (skipping tombstones).
    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.visible_iter()
    }

    /// Collect visible values into a Vec.
    pub fn to_vec(&self) -> Vec<&V> {
        self.visible_iter().collect()
    }

    /// Merge another RGA into this one.
    pub fn merge(&mut self, other: &Rga<V>) {
        for other_node in &other.nodes {
            if let Some(self_node) = self.nodes.iter_mut().find(|n| n.id == other_node.id) {
                // Node exists in both — if either tombstoned, tombstone it
                if other_node.value.is_none() {
                    self_node.value = None;
                }
            } else if !self.clock.contains(&other_node.id) {
                // New node from other
                self.nodes.push(other_node.clone());
            }
        }

        self.clock.merge(&other.clock);
        self.reorder();
    }

    /// Return a reference to the internal clock.
    pub fn clock(&self) -> &VersionVector {
        &self.clock
    }

    // ─── Internal helpers ───────────────────────────────────

    fn visible_iter(&self) -> impl Iterator<Item = &V> {
        self.nodes.iter().filter_map(|n| n.value.as_ref())
    }

    /// Reorder nodes into canonical RGA order via depth-first traversal.
    ///
    /// The tree structure: each node's anchor points to its parent.
    /// Among siblings (same anchor), higher dots come first (descending order).
    /// A node's subtree appears immediately after it, before the next sibling.
    fn reorder(&mut self) {
        let old = std::mem::take(&mut self.nodes);
        let mut ordered = Vec::with_capacity(old.len());

        // Collect root-level nodes (anchor = None)
        let mut roots: Vec<usize> = old
            .iter()
            .enumerate()
            .filter(|(_, n)| n.anchor.is_none())
            .map(|(i, _)| i)
            .collect();

        // Sort roots: higher dot first (descending)
        roots.sort_by(|&a, &b| Self::dot_cmp_desc(&old[a].id, &old[b].id));

        for &root_idx in &roots {
            Self::dfs(&old, root_idx, &mut ordered);
        }

        self.nodes = ordered;
    }

    /// Depth-first traversal: emit node, then children (sorted descending by dot).
    fn dfs(all: &[RgaNode<V>], idx: usize, result: &mut Vec<RgaNode<V>>) {
        result.push(all[idx].clone());
        let id = &all[idx].id;

        // Find children anchored to this node
        let mut children: Vec<usize> = all
            .iter()
            .enumerate()
            .filter(|(_, n)| n.anchor.as_ref() == Some(id))
            .map(|(i, _)| i)
            .collect();

        // Sort children: higher dot first (descending)
        children.sort_by(|&a, &b| Self::dot_cmp_desc(&all[a].id, &all[b].id));

        for &child_idx in &children {
            Self::dfs(all, child_idx, result);
        }
    }

    /// Compare dots in descending order (higher replica, then higher counter, comes first).
    fn dot_cmp_desc(a: &Dot, b: &Dot) -> std::cmp::Ordering {
        b.replica.cmp(&a.replica).then(b.counter.cmp(&a.counter))
    }
}

impl<V: Clone + PartialEq> Default for Rga<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: Clone + PartialEq> PartialEq for Rga<V> {
    fn eq(&self, other: &Self) -> bool {
        // Compare visible sequences
        let a: Vec<&V> = self.visible_iter().collect();
        let b: Vec<&V> = other.visible_iter().collect();
        a == b
    }
}

impl<V: Clone + PartialEq> Eq for Rga<V> {}
