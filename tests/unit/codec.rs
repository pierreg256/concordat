use concordat::codec;
use concordat::doc::CrdtDoc;
use concordat::vv::VersionVector;
use serde_json::json;

// ─── Round-trip ─────────────────────────────────────────────

#[test]
fn test_codec_roundtrip_simple() {
    let mut doc = CrdtDoc::new("a");
    doc.set("/x", json!(42));
    doc.set("/name", json!("Alice"));

    let delta = doc.delta_since(&VersionVector::new());
    let bytes = codec::encode(&delta);
    let decoded = codec::decode(&bytes).unwrap();

    // Apply decoded delta to a fresh replica — should produce same state
    let mut replica = CrdtDoc::new("b");
    replica.merge_delta(&decoded);
    assert_eq!(doc.materialize(), replica.materialize());
}

#[test]
fn test_codec_roundtrip_with_array() {
    let mut doc = CrdtDoc::new("a");
    doc.set_array("/items");
    doc.array_insert("/items", 0, json!("first"));
    doc.array_insert("/items", 1, json!("second"));

    let delta = doc.delta_since(&VersionVector::new());
    let bytes = codec::encode(&delta);
    let decoded = codec::decode(&bytes).unwrap();

    let mut replica = CrdtDoc::new("b");
    replica.merge_delta(&decoded);
    assert_eq!(doc.materialize(), replica.materialize());
}

#[test]
fn test_codec_roundtrip_nested() {
    let mut doc = CrdtDoc::new("a");
    doc.set("/user/name", json!("Bob"));
    doc.set("/user/age", json!(30));

    let delta = doc.delta_since(&VersionVector::new());
    let bytes = codec::encode(&delta);
    let decoded = codec::decode(&bytes).unwrap();

    let mut replica = CrdtDoc::new("b");
    replica.merge_delta(&decoded);
    assert_eq!(doc.materialize(), replica.materialize());
}

// ─── Corrupt bytes return error ─────────────────────────────

#[test]
fn test_codec_corrupt_bytes_error() {
    let result = codec::decode(&[0xFF, 0xFE, 0xFD, 0x00]);
    assert!(result.is_err());
}

#[test]
fn test_codec_empty_bytes_error() {
    let result = codec::decode(&[]);
    assert!(result.is_err());
}

// ─── Integration: serialize → deserialize → merge → convergence

#[test]
fn test_codec_integration_convergence() {
    let mut doc_a = CrdtDoc::new("a");
    let mut doc_b = CrdtDoc::new("b");

    doc_a.set("/x", json!("from_a"));
    doc_b.set("/y", json!("from_b"));

    // Serialize deltas
    let bytes_a = codec::encode(&doc_a.delta_since(&VersionVector::new()));
    let bytes_b = codec::encode(&doc_b.delta_since(&VersionVector::new()));

    // Deserialize and merge
    let delta_a = codec::decode(&bytes_a).unwrap();
    let delta_b = codec::decode(&bytes_b).unwrap();

    doc_a.merge_delta(&delta_b);
    doc_b.merge_delta(&delta_a);

    assert_eq!(doc_a.materialize(), doc_b.materialize());
}

#[test]
fn test_codec_roundtrip_idempotent() {
    let mut doc = CrdtDoc::new("a");
    doc.set("/key", json!("value"));

    let delta = doc.delta_since(&VersionVector::new());
    let bytes = codec::encode(&delta);

    // Decode twice, merge twice — still idempotent
    let d1 = codec::decode(&bytes).unwrap();
    let d2 = codec::decode(&bytes).unwrap();

    let mut replica = CrdtDoc::new("b");
    replica.merge_delta(&d1);
    let after_first = replica.materialize();
    replica.merge_delta(&d2);
    assert_eq!(replica.materialize(), after_first);
}
