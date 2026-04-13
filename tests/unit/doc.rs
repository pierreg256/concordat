use concordat::doc::CrdtDoc;
use concordat::vv::VersionVector;
use serde_json::json;

// ─── set + materialize round-trip ───────────────────────────

#[test]
fn test_doc_set_materialize_simple() {
    let mut doc = CrdtDoc::new("a");
    doc.set("/name", json!("Alice"));
    doc.set("/age", json!(30));

    let mat = doc.materialize();
    assert_eq!(mat["name"], json!("Alice"));
    assert_eq!(mat["age"], json!(30));
}

#[test]
fn test_doc_set_overwrite() {
    let mut doc = CrdtDoc::new("a");
    doc.set("/x", json!(1));
    doc.set("/x", json!(2));

    let mat = doc.materialize();
    assert_eq!(mat["x"], json!(2));
}

// ─── set nested path creates intermediate objects ───────────

#[test]
fn test_doc_set_nested_creates_intermediates() {
    let mut doc = CrdtDoc::new("a");
    doc.set("/user/name", json!("Bob"));

    let mat = doc.materialize();
    assert_eq!(mat["user"]["name"], json!("Bob"));
}

#[test]
fn test_doc_set_deeply_nested() {
    let mut doc = CrdtDoc::new("a");
    doc.set("/a/b/c", json!(42));

    let mat = doc.materialize();
    assert_eq!(mat["a"]["b"]["c"], json!(42));
}

// ─── remove ─────────────────────────────────────────────────

#[test]
fn test_doc_remove_top_level() {
    let mut doc = CrdtDoc::new("a");
    doc.set("/x", json!(1));
    doc.set("/y", json!(2));
    doc.remove("/x");

    let mat = doc.materialize();
    assert!(mat.get("x").is_none());
    assert_eq!(mat["y"], json!(2));
}

#[test]
fn test_doc_remove_nested() {
    let mut doc = CrdtDoc::new("a");
    doc.set("/user/name", json!("Alice"));
    doc.set("/user/age", json!(30));
    doc.remove("/user/age");

    let mat = doc.materialize();
    assert_eq!(mat["user"]["name"], json!("Alice"));
    assert!(mat["user"].get("age").is_none());
}

// ─── array_insert / array_delete ────────────────────────────

#[test]
fn test_doc_array_operations() {
    let mut doc = CrdtDoc::new("a");

    // First create the array
    doc.set("/items", json!(null)); // placeholder
    // Remove and re-create as array type
    doc.remove("/items");

    // Create an array by inserting into a path that we set up
    // For now, let's set an array path directly via internal mechanism
    // Use a different approach: set up using the API
    let mut doc2 = CrdtDoc::new("a");
    doc2.set_array("/scores");
    doc2.array_insert("/scores", 0, json!(100));
    doc2.array_insert("/scores", 1, json!(200));
    doc2.array_insert("/scores", 2, json!(300));

    let mat = doc2.materialize();
    assert_eq!(mat["scores"], json!([100, 200, 300]));
}

#[test]
fn test_doc_array_delete() {
    let mut doc = CrdtDoc::new("a");
    doc.set_array("/items");
    doc.array_insert("/items", 0, json!("a"));
    doc.array_insert("/items", 1, json!("b"));
    doc.array_insert("/items", 2, json!("c"));

    doc.array_delete("/items", 1); // delete "b"

    let mat = doc.materialize();
    assert_eq!(mat["items"], json!(["a", "c"]));
}

// ─── Multiple operations ────────────────────────────────────

#[test]
fn test_doc_mixed_operations() {
    let mut doc = CrdtDoc::new("a");

    doc.set("/title", json!("Shopping List"));
    doc.set_array("/items");
    doc.array_insert("/items", 0, json!("milk"));
    doc.array_insert("/items", 1, json!("bread"));
    doc.set("/count", json!(2));

    let mat = doc.materialize();
    assert_eq!(mat["title"], json!("Shopping List"));
    assert_eq!(mat["items"], json!(["milk", "bread"]));
    assert_eq!(mat["count"], json!(2));
}

#[test]
fn test_doc_empty_materialized() {
    let doc = CrdtDoc::new("a");
    assert_eq!(doc.materialize(), json!({}));
}

#[test]
fn test_doc_version_vector_advances() {
    let mut doc = CrdtDoc::new("a");
    assert!(doc.version_vector().is_empty());

    doc.set("/x", json!(1));
    assert!(!doc.version_vector().is_empty());
    let v1 = doc.version_vector().get("a");

    doc.set("/y", json!(2));
    let v2 = doc.version_vector().get("a");
    assert!(v2 > v1);
}

// ─── Cross-replica remove ───────────────────────────────────

/// Removing a key added by a different replica must actually remove it
/// from the materialized view. This is a basic CRDT requirement.
#[test]
fn test_doc_remove_cross_replica_top_level() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");

    // B adds a key
    b.set("/x", json!(42));

    // A merges B's state — A now sees /x
    let delta_b = b.delta_since(a.version_vector());
    a.merge_delta(&delta_b);
    assert_eq!(a.materialize()["x"], json!(42));

    // A removes B's key
    a.remove("/x");

    // The key must be gone
    let mat = a.materialize();
    assert!(
        mat.get("x").is_none(),
        "cross-replica remove failed: /x still present after remove"
    );
}

/// Removing a nested key added by a different replica.
#[test]
fn test_doc_remove_cross_replica_nested() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");

    // B adds a nested key
    b.set("/nodes/b", json!({"addr": "127.0.0.1:4370"}));

    // Sync B → A
    let delta = b.delta_since(&a.version_vector().clone());
    a.merge_delta(&delta);
    assert!(a.materialize()["nodes"]["b"]["addr"] == "127.0.0.1:4370");

    // A removes B's nested key
    a.remove("/nodes/b");

    // Must be gone
    let nodes = &a.materialize()["nodes"];
    let has_b = nodes
        .as_object()
        .map(|m| m.contains_key("b"))
        .unwrap_or(false);
    assert!(
        !has_b,
        "cross-replica nested remove failed: /nodes/b still present"
    );
}

/// After A removes a key from B, gossiping B's stale state to A must NOT
/// re-introduce the removed key (the removal must win over stale data).
#[test]
fn test_doc_remove_survives_stale_gossip() {
    let mut a = CrdtDoc::new("a");
    let mut b = CrdtDoc::new("b");
    let mut c = CrdtDoc::new("c");

    // All three add themselves
    a.set("/nodes/a", json!("a"));
    b.set("/nodes/b", json!("b"));
    c.set("/nodes/c", json!("c"));

    // Full sync
    let da = a.delta_since(&VersionVector::new());
    let db = b.delta_since(&VersionVector::new());
    let dc = c.delta_since(&VersionVector::new());
    a.merge_delta(&db);
    a.merge_delta(&dc);
    b.merge_delta(&da);
    b.merge_delta(&dc);
    c.merge_delta(&da);
    c.merge_delta(&db);

    // A removes node C
    a.remove("/nodes/c");
    let mat = a.materialize();
    let a_nodes = mat["nodes"].as_object().unwrap();
    assert_eq!(a_nodes.len(), 2, "A should have 2 nodes after removing C");

    // B (stale) gossips to A — B still has C
    let delta_b = b.delta_since(&a.version_vector().clone());
    a.merge_delta(&delta_b);

    // C must NOT reappear on A
    let mat = a.materialize();
    let a_nodes = mat["nodes"].as_object().unwrap();
    assert_eq!(
        a_nodes.len(),
        2,
        "node C reappeared on A after stale gossip from B"
    );

    // A gossips removal to B
    let delta_a = a.delta_since(&b.version_vector().clone());
    b.merge_delta(&delta_a);
    let mat = b.materialize();
    let b_nodes = mat["nodes"].as_object().unwrap();
    assert_eq!(b_nodes.len(), 2, "removal did not propagate from A to B");
}

/// Exact PMD scenario: star topology, seed removes a disconnected node,
/// then receives gossip from another leaf that still has the dead node.
/// The remove from the seed must "stick" through the merge.
#[test]
fn test_doc_remove_not_undone_by_merge_from_stale_full_delta() {
    let mut seed = CrdtDoc::new("seed");
    let mut leaf1 = CrdtDoc::new("leaf1");
    let mut leaf2 = CrdtDoc::new("leaf2");

    // Each node adds itself
    seed.set("/nodes/seed", json!({"addr": "127.0.0.1:4369"}));
    leaf1.set("/nodes/leaf1", json!({"addr": "127.0.0.1:4370"}));
    leaf2.set("/nodes/leaf2", json!({"addr": "127.0.0.1:4371"}));

    // All sync with seed (star topology: seed ↔ leaf1, seed ↔ leaf2)
    let ds = seed.delta_since(&VersionVector::new());
    let d1 = leaf1.delta_since(&VersionVector::new());
    let d2 = leaf2.delta_since(&VersionVector::new());
    seed.merge_delta(&d1);
    seed.merge_delta(&d2);
    leaf1.merge_delta(&ds);
    leaf1.merge_delta(&d2);
    leaf2.merge_delta(&ds);
    leaf2.merge_delta(&d1);

    // Verify all see 3 nodes
    assert_eq!(seed.materialize()["nodes"].as_object().unwrap().len(), 3);
    assert_eq!(leaf1.materialize()["nodes"].as_object().unwrap().len(), 3);
    assert_eq!(leaf2.materialize()["nodes"].as_object().unwrap().len(), 3);

    // leaf2 disconnects. Seed removes it.
    seed.remove("/nodes/leaf2");
    let mat = seed.materialize();
    let s_nodes = mat["nodes"].as_object().unwrap();
    assert_eq!(
        s_nodes.len(),
        2,
        "seed should have 2 nodes after removing leaf2"
    );
    assert!(
        !s_nodes.contains_key("leaf2"),
        "leaf2 should be gone from seed"
    );

    // leaf1 gossips its full state to seed (leaf1 still has leaf2).
    // This is what delta_since returns (full state).
    let delta_leaf1 = leaf1.delta_since(&seed.version_vector().clone());
    seed.merge_delta(&delta_leaf1);

    // leaf2 must NOT reappear on seed
    let mat = seed.materialize();
    let s_nodes = mat["nodes"].as_object().unwrap();
    assert!(
        !s_nodes.contains_key("leaf2"),
        "leaf2 reappeared on seed after merge with stale leaf1 delta! \
         Found {} nodes: {:?}",
        s_nodes.len(),
        s_nodes.keys().collect::<Vec<_>>()
    );
}
