//! Delta types — delta production and representation.
//!
//! A delta is a self-contained fragment of a document's state that can be
//! merged into another replica to bring it up to date. In delta-state CRDTs,
//! the delta is itself a valid state (a subset of the full state).

use serde::{Deserialize, Serialize};

use crate::ormap::OrMap;
use crate::value::CrdtValue;
use crate::vv::VersionVector;

/// A delta: a fragment of document state that can be merged into a remote replica.
///
/// Contains the subset of the OR-Map root and the version vector.
/// Merging a delta into a document is identical to merging two documents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    /// The root OR-Map state captured in this delta.
    pub(crate) root: OrMap<String, CrdtValue>,
    /// The version vector at the time the delta was produced.
    pub(crate) vv: VersionVector,
}

impl Delta {
    /// Check if this delta is empty (no state to merge).
    pub fn is_empty(&self) -> bool {
        self.root.is_empty()
    }

    /// Get the version vector of this delta.
    pub fn version_vector(&self) -> &VersionVector {
        &self.vv
    }
}
