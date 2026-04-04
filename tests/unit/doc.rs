use concordat::doc::CrdtDoc;
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
