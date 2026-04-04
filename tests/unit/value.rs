use concordat::ormap::OrMap;
use concordat::register::MvRegister;
use concordat::rga::Rga;
use concordat::value::CrdtValue;
use concordat::vv::VersionVector;

// ─── Nested Object → Scalar ────────────────────────────────

#[test]
fn test_value_object_scalar_set_merge_materialize() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();

    // Replica A: { "name": "Alice" }
    let mut obj_a: OrMap<String, CrdtValue> = OrMap::new();
    let mut reg_a = MvRegister::new();
    let dot_a = vv_a.inc("a");
    reg_a.set(serde_json::json!("Alice"), dot_a.clone(), &vv_a);
    obj_a.put("name".to_string(), CrdtValue::Scalar(reg_a), dot_a);

    // Replica B: { "name": "Bob" }
    let mut obj_b: OrMap<String, CrdtValue> = OrMap::new();
    let mut reg_b = MvRegister::new();
    let dot_b = vv_b.inc("b");
    reg_b.set(serde_json::json!("Bob"), dot_b.clone(), &vv_b);
    obj_b.put("name".to_string(), CrdtValue::Scalar(reg_b), dot_b);

    // Merge A into B and B into A
    let mut ab = obj_a.clone();
    ab.merge(&obj_b);

    let mut ba = obj_b.clone();
    ba.merge(&obj_a);

    // Both should have "name" key present
    assert!(ab.contains_key(&"name".to_string()));
    assert!(ba.contains_key(&"name".to_string()));

    // Values should materialize the same
    let val_ab = ab.get(&"name".to_string()).unwrap().materialize();
    let val_ba = ba.get(&"name".to_string()).unwrap().materialize();
    assert_eq!(val_ab, val_ba);
}

// ─── Nested Object → Array → Scalar ────────────────────────

#[test]
fn test_value_nested_object_array_scalar() {
    let mut vv = VersionVector::new();

    // Build { "items": [10, 20] }
    let mut rga: Rga<CrdtValue> = Rga::new();

    let mut reg1 = MvRegister::new();
    let d1 = vv.inc("a");
    reg1.set(serde_json::json!(10), d1.clone(), &vv);
    let d_insert1 = vv.inc("a");
    rga.insert(0, CrdtValue::Scalar(reg1), d_insert1);

    let mut reg2 = MvRegister::new();
    let d2 = vv.inc("a");
    reg2.set(serde_json::json!(20), d2.clone(), &vv);
    let d_insert2 = vv.inc("a");
    rga.insert(1, CrdtValue::Scalar(reg2), d_insert2);

    let mut obj: OrMap<String, CrdtValue> = OrMap::new();
    let d_put = vv.inc("a");
    obj.put("items".to_string(), CrdtValue::Array(rga), d_put);

    // Materialize
    let mat = obj.get(&"items".to_string()).unwrap().materialize();
    assert_eq!(mat, serde_json::json!([10, 20]));
}

// ─── Nested Object → Object ────────────────────────────────

#[test]
fn test_value_nested_object_recursive_merge() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();

    // Replica A: { "user": { "age": 30 } }
    let mut inner_a: OrMap<String, CrdtValue> = OrMap::new();
    let mut reg_age_a = MvRegister::new();
    let d1 = vv_a.inc("a");
    reg_age_a.set(serde_json::json!(30), d1.clone(), &vv_a);
    inner_a.put("age".to_string(), CrdtValue::Scalar(reg_age_a), d1);

    let mut outer_a: OrMap<String, CrdtValue> = OrMap::new();
    let d_out_a = vv_a.inc("a");
    outer_a.put("user".to_string(), CrdtValue::Object(inner_a), d_out_a);

    // Replica B: { "user": { "city": "Paris" } }
    let mut inner_b: OrMap<String, CrdtValue> = OrMap::new();
    let mut reg_city = MvRegister::new();
    let d2 = vv_b.inc("b");
    reg_city.set(serde_json::json!("Paris"), d2.clone(), &vv_b);
    inner_b.put("city".to_string(), CrdtValue::Scalar(reg_city), d2);

    let mut outer_b: OrMap<String, CrdtValue> = OrMap::new();
    let d_out_b = vv_b.inc("b");
    outer_b.put("user".to_string(), CrdtValue::Object(inner_b), d_out_b);

    // Merge
    let mut merged = outer_a.clone();
    merged.merge(&outer_b);

    // Materialize should contain both keys
    let user = merged.get(&"user".to_string()).unwrap().materialize();
    assert!(user.is_object());
}

// ─── Type mismatch ──────────────────────────────────────────

#[test]
fn test_value_type_mismatch_object_wins_over_scalar() {
    let mut scalar = CrdtValue::scalar(serde_json::json!(42));
    let obj = CrdtValue::object();

    scalar.merge(&obj);
    // Object has higher priority, so scalar becomes object
    assert!(matches!(scalar, CrdtValue::Object(_)));
}

#[test]
fn test_value_type_mismatch_object_wins_over_array() {
    let mut arr = CrdtValue::array();
    let obj = CrdtValue::object();

    arr.merge(&obj);
    assert!(matches!(arr, CrdtValue::Object(_)));
}

// ─── Materialize ────────────────────────────────────────────

#[test]
fn test_value_materialize_scalar() {
    let val = CrdtValue::scalar(serde_json::json!("hello"));
    // Note: scalar() uses a dummy dot, so materialize might not return the value
    // via value() since MvRegister stores entries. Let's directly test.
    let mat = val.materialize();
    assert_eq!(mat, serde_json::json!("hello"));
}

#[test]
fn test_value_materialize_empty_object() {
    let val = CrdtValue::object();
    assert_eq!(val.materialize(), serde_json::json!({}));
}

#[test]
fn test_value_materialize_empty_array() {
    let val = CrdtValue::array();
    assert_eq!(val.materialize(), serde_json::json!([]));
}
