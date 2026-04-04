//! Codec — serialization and deserialization of deltas.
//!
//! Uses `serde_json` for serialization. A binary format (e.g. CBOR)
//! can be substituted later without changing the API.
//! Deltas are opaque byte buffers — the transport layer must not interpret them.

use crate::delta::Delta;

/// Errors that can occur during decoding.
#[derive(Debug)]
pub enum CodecError {
    /// The byte buffer could not be deserialized.
    DecodeFailed(String),
}

impl std::fmt::Display for CodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CodecError::DecodeFailed(msg) => write!(f, "decode failed: {msg}"),
        }
    }
}

impl std::error::Error for CodecError {}

/// Encode a delta into a byte buffer.
pub fn encode(delta: &Delta) -> Vec<u8> {
    serde_json::to_vec(delta).expect("delta serialization should not fail")
}

/// Decode a delta from a byte buffer.
pub fn decode(bytes: &[u8]) -> Result<Delta, CodecError> {
    serde_json::from_slice(bytes).map_err(|e| CodecError::DecodeFailed(e.to_string()))
}
