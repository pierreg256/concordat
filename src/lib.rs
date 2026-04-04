//! # Concordat
//!
//! Delta-state CRDT JSON library.
//!
//! Provides Strong Eventual Consistency for distributed JSON documents
//! with no network dependency.

pub mod vv;
pub mod register;
pub mod ormap;
pub mod rga;
pub mod value;
pub mod doc;
pub mod delta;
pub mod codec;
