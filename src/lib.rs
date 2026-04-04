//! # Concordat
//!
//! Delta-state CRDT JSON library.
//!
//! Provides Strong Eventual Consistency for distributed JSON documents
//! with no network dependency.

pub mod codec;
pub mod delta;
pub mod doc;
pub mod ormap;
pub mod register;
pub mod rga;
pub mod value;
pub mod vv;
