use concordat::codec;
use concordat::doc::CrdtDoc;
use concordat::vv::VersionVector;
use serde_json::json;

/// Helper: sync two docs by exchanging full deltas.
fn full_sync(a: &mut CrdtDoc, b: &mut CrdtDoc) {
    let da = a.delta_since(&VersionVector::new());
    let db = b.delta_since(&VersionVector::new());
    a.merge_delta(&db);
    b.merge_delta(&da);
}

/// Helper: sync via serialized bytes (simulates real transport).
fn full_sync_via_bytes(a: &mut CrdtDoc, b: &mut CrdtDoc) {
    let bytes_a = codec::encode(&a.delta_since(&VersionVector::new()));
    let bytes_b = codec::encode(&b.delta_since(&VersionVector::new()));
    a.merge_delta(&codec::decode(&bytes_b).unwrap());
    b.merge_delta(&codec::decode(&bytes_a).unwrap());
}

/// Assert all docs have identical materialized output.
fn assert_converged(docs: &[&CrdtDoc]) {
    let first = docs[0].materialize();
    for (i, doc) in docs.iter().enumerate().skip(1) {
        assert_eq!(first, doc.materialize(), "doc[0] and doc[{i}] diverged");
    }
}

// ═══════════════════════════════════════════════════════════
// 2-replica scenarios
// ═══════════════════════════════════════════════════════════

#[test]
fn test_2_replicas_disjoint_keys() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");

    a.set("/x", json!(1));
    a.set("/y", json!(2));
    b.set("/z", json!(3));
    b.set("/w", json!(4));

    full_sync(&mut a, &mut b);
    assert_converged(&[&a, &b]);

    let mat = a.materialize();
    assert_eq!(mat["x"], json!(1));
    assert_eq!(mat["y"], json!(2));
    assert_eq!(mat["z"], json!(3));
    assert_eq!(mat["w"], json!(4));
}

#[test]
fn test_2_replicas_same_key_concurrent() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");

    a.set("/title", json!("A's title"));
    b.set("/title", json!("B's title"));

    full_sync(&mut a, &mut b);
    assert_converged(&[&a, &b]);
}

#[test]
fn test_2_replicas_concurrent_array_inserts() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");

    // Both create arrays, sync
    a.set_array("/items");
    b.set_array("/items");
    full_sync(&mut a, &mut b);

    // Concurrent inserts
    a.array_insert("/items", 0, json!("apple"));
    b.array_insert("/items", 0, json!("banana"));

    full_sync(&mut a, &mut b);
    assert_converged(&[&a, &b]);

    // Both items should be present
    let items = a.materialize()["items"].as_array().unwrap().clone();
    assert_eq!(items.len(), 2);
}

// ═══════════════════════════════════════════════════════════
// 3-replica scenarios
// ═══════════════════════════════════════════════════════════

#[test]
fn test_3_replicas_chain_sync() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");
    let mut c = CrdtDoc::new("c");

    a.set("/from_a", json!(1));
    b.set("/from_b", json!(2));
    c.set("/from_c", json!(3));

    // Chain: A → B → C → A → B
    let da = a.delta_since(&VersionVector::new());
    b.merge_delta(&da);

    let db = b.delta_since(&VersionVector::new());
    c.merge_delta(&db);

    let dc = c.delta_since(&VersionVector::new());
    a.merge_delta(&dc);

    let da2 = a.delta_since(&VersionVector::new());
    b.merge_delta(&da2);

    assert_converged(&[&a, &b, &c]);
}

#[test]
fn test_3_replicas_star_sync() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");
    let mut c = CrdtDoc::new("c");

    a.set("/x", json!("a"));
    b.set("/y", json!("b"));
    c.set("/z", json!("c"));

    // Star: A↔B, A↔C, B↔C
    full_sync(&mut a, &mut b);
    full_sync(&mut a, &mut c);
    full_sync(&mut b, &mut c);

    assert_converged(&[&a, &b, &c]);
}

// ═══════════════════════════════════════════════════════════
// 5-replica scenario
// ═══════════════════════════════════════════════════════════

#[test]
fn test_5_replicas_complex_partition() {
    let mut r1 = CrdtDoc::new("r1");
    let mut r2 = CrdtDoc::new("r2");
    let mut r3 = CrdtDoc::new("r3");
    let mut r4 = CrdtDoc::new("r4");
    let mut r5 = CrdtDoc::new("r5");

    // Phase 1: all replicas mutate independently (full partition)
    r1.set("/from_r1", json!(1));
    r2.set("/from_r2", json!(2));
    r3.set("/from_r3", json!(3));
    r4.set("/from_r4", json!(4));
    r5.set("/from_r5", json!(5));

    // Phase 2: partial sync (r1↔r2, r3↔r4)
    full_sync(&mut r1, &mut r2);
    full_sync(&mut r3, &mut r4);

    // Phase 3: more mutations while partially synced
    r1.set("/phase2_r1", json!("hello"));
    r5.set("/phase2_r5", json!("world"));

    // Phase 4: full reconnection — multiple rounds to propagate everything
    // Round 1: cross-partition links
    full_sync(&mut r2, &mut r3);
    full_sync(&mut r4, &mut r5);
    // Round 2: propagate r1's phase2 mutation
    full_sync(&mut r1, &mut r2);
    full_sync(&mut r1, &mut r3);
    // Round 3: ensure all have everything
    full_sync(&mut r2, &mut r4);
    full_sync(&mut r3, &mut r5);
    full_sync(&mut r4, &mut r1);
    full_sync(&mut r5, &mut r2);

    assert_converged(&[&r1, &r2, &r3, &r4, &r5]);

    let mat = r1.materialize();
    assert_eq!(mat["from_r1"], json!(1));
    assert_eq!(mat["from_r5"], json!(5));
    assert_eq!(mat["phase2_r1"], json!("hello"));
    assert_eq!(mat["phase2_r5"], json!("world"));
}

// ═══════════════════════════════════════════════════════════
// Partition / Reconnect
// ═══════════════════════════════════════════════════════════

#[test]
fn test_partition_reconnect() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");

    // Initial sync
    a.set("/shared", json!("initial"));
    full_sync(&mut a, &mut b);

    // Partition: both mutate independently
    a.set("/shared", json!("updated_by_a"));
    a.set("/only_a", json!(true));

    b.set("/shared", json!("updated_by_b"));
    b.set("/only_b", json!(false));

    // Reconnect
    full_sync(&mut a, &mut b);
    assert_converged(&[&a, &b]);

    // Both exclusive keys should be present
    let mat = a.materialize();
    assert_eq!(mat["only_a"], json!(true));
    assert_eq!(mat["only_b"], json!(false));
}

// ═══════════════════════════════════════════════════════════
// Duplicate delta delivery
// ═══════════════════════════════════════════════════════════

#[test]
fn test_duplicate_delta_delivery() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");

    a.set("/x", json!(42));
    a.set("/y", json!("hello"));

    let delta = a.delta_since(&VersionVector::new());

    b.merge_delta(&delta);
    let after_first = b.materialize();

    // Send same delta 5 more times
    for _ in 0..5 {
        b.merge_delta(&delta);
    }

    assert_eq!(b.materialize(), after_first);
}

// ═══════════════════════════════════════════════════════════
// Out-of-order delta delivery
// ═══════════════════════════════════════════════════════════

#[test]
fn test_out_of_order_delivery() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");
    let mut c = CrdtDoc::new("c");

    a.set("/x", json!("from_a"));
    b.set("/y", json!("from_b"));
    c.set("/z", json!("from_c"));

    let da = a.delta_since(&VersionVector::new());
    let db = b.delta_since(&VersionVector::new());
    let dc = c.delta_since(&VersionVector::new());

    // Apply in order: C, A, B
    let mut r1 = CrdtDoc::new("r1");
    r1.merge_delta(&dc);
    r1.merge_delta(&da);
    r1.merge_delta(&db);

    // Apply in order: B, C, A
    let mut r2 = CrdtDoc::new("r2");
    r2.merge_delta(&db);
    r2.merge_delta(&dc);
    r2.merge_delta(&da);

    // Apply in order: A, B, C
    let mut r3 = CrdtDoc::new("r3");
    r3.merge_delta(&da);
    r3.merge_delta(&db);
    r3.merge_delta(&dc);

    assert_converged(&[&r1, &r2, &r3]);
}

// ═══════════════════════════════════════════════════════════
// Deeply nested JSON
// ═══════════════════════════════════════════════════════════

#[test]
fn test_deeply_nested_convergence() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");

    // A: { users: { alice: { scores: [] } } }
    a.set("/users/alice/name", json!("Alice"));
    a.set_array("/users/alice/scores");
    a.array_insert("/users/alice/scores", 0, json!(100));

    // B: { users: { bob: { scores: [] } } }
    b.set("/users/bob/name", json!("Bob"));
    b.set_array("/users/bob/scores");
    b.array_insert("/users/bob/scores", 0, json!(200));

    full_sync(&mut a, &mut b);
    assert_converged(&[&a, &b]);

    let mat = a.materialize();
    assert_eq!(mat["users"]["alice"]["name"], json!("Alice"));
    assert_eq!(mat["users"]["bob"]["name"], json!("Bob"));
}

// ═══════════════════════════════════════════════════════════
// Full integration via serialized bytes
// ═══════════════════════════════════════════════════════════

#[test]
fn test_convergence_via_serialized_bytes() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");

    a.set("/msg", json!("hello from a"));
    b.set("/msg", json!("hello from b"));
    b.set("/extra", json!(true));

    full_sync_via_bytes(&mut a, &mut b);
    assert_converged(&[&a, &b]);
}

// ═══════════════════════════════════════════════════════════
// Concurrent set + remove
// ═══════════════════════════════════════════════════════════

#[test]
fn test_concurrent_set_and_remove() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");

    // Both know about /x
    a.set("/x", json!(1));
    full_sync(&mut a, &mut b);

    // A removes /x, B updates /x concurrently
    a.remove("/x");
    b.set("/x", json!(2));

    full_sync(&mut a, &mut b);
    assert_converged(&[&a, &b]);

    // Add-wins: B's concurrent set should survive
    assert!(a.materialize().get("x").is_some());
}
