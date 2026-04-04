use concordat::ormap::OrMap;
use concordat::vv::VersionVector;

// ─── Helper ─────────────────────────────────────────────────

fn make_map_with(
    replica: &str,
    key: &str,
    value: i32,
    vv: &mut VersionVector,
) -> OrMap<String, i32> {
    let mut map = OrMap::new();
    let dot = vv.inc(replica);
    map.put(key.to_string(), value, dot);
    map
}

// ─── Commutativity ──────────────────────────────────────────

#[test]
fn test_ormap_merge_commutativity() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();

    let map_a = make_map_with("a", "x", 10, &mut vv_a);
    let map_b = make_map_with("b", "y", 20, &mut vv_b);

    let mut ab = map_a.clone();
    ab.merge(&map_b);

    let mut ba = map_b.clone();
    ba.merge(&map_a);

    assert_eq!(ab, ba);
}

#[test]
fn test_ormap_merge_commutativity_same_key() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();

    let map_a = make_map_with("a", "x", 10, &mut vv_a);
    let map_b = make_map_with("b", "x", 20, &mut vv_b);

    let mut ab = map_a.clone();
    ab.merge(&map_b);

    let mut ba = map_b.clone();
    ba.merge(&map_a);

    assert_eq!(ab, ba);
}

// ─── Associativity ──────────────────────────────────────────

#[test]
fn test_ormap_merge_associativity() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();
    let mut vv_c = VersionVector::new();

    let map_a = make_map_with("a", "x", 1, &mut vv_a);
    let map_b = make_map_with("b", "y", 2, &mut vv_b);
    let map_c = make_map_with("c", "z", 3, &mut vv_c);

    // merge(merge(A, B), C)
    let mut ab = map_a.clone();
    ab.merge(&map_b);
    let mut ab_c = ab.clone();
    ab_c.merge(&map_c);

    // merge(A, merge(B, C))
    let mut bc = map_b.clone();
    bc.merge(&map_c);
    let mut a_bc = map_a.clone();
    a_bc.merge(&bc);

    assert_eq!(ab_c, a_bc);
}

// ─── Idempotence ────────────────────────────────────────────

#[test]
fn test_ormap_merge_idempotence() {
    let mut vv_a = VersionVector::new();
    let map_a = make_map_with("a", "x", 42, &mut vv_a);

    let original = map_a.clone();
    let mut merged = map_a.clone();
    merged.merge(&original);
    assert_eq!(merged, original);
}

#[test]
fn test_ormap_merge_idempotence_repeated() {
    let mut vv_a = VersionVector::new();
    let map_a = make_map_with("a", "x", 42, &mut vv_a);

    let snapshot = map_a.clone();
    let mut merged = map_a.clone();
    merged.merge(&snapshot);
    merged.merge(&snapshot);
    merged.merge(&snapshot);
    assert_eq!(merged, snapshot);
}

// ─── Add then remove ────────────────────────────────────────

#[test]
fn test_ormap_add_then_remove() {
    let mut vv = VersionVector::new();
    let mut map = OrMap::new();

    let dot = vv.inc("a");
    map.put("x".to_string(), 10, dot);
    assert!(map.contains_key(&"x".to_string()));

    map.remove(&"x".to_string(), &vv);
    assert!(!map.contains_key(&"x".to_string()));
}

// ─── Concurrent add/remove: add wins ────────────────────────

#[test]
fn test_ormap_concurrent_add_remove_add_wins() {
    let mut vv_a = VersionVector::new();

    // Replica A adds key "x"
    let mut map_a = OrMap::new();
    let dot_a = vv_a.inc("a");
    map_a.put("x".to_string(), 10, dot_a);

    // Replica B also knows about "x" and removes it
    let mut map_b = map_a.clone();
    let vv_b_full = vv_a.clone();
    map_b.remove(&"x".to_string(), &vv_b_full);

    // Meanwhile A does a concurrent add with a new dot
    let dot_a2 = vv_a.inc("a");
    map_a.put("x".to_string(), 20, dot_a2);

    // Merge: B removed the old dot, but A's new dot survives → add wins
    let mut merged = map_b.clone();
    merged.merge(&map_a);
    assert!(merged.contains_key(&"x".to_string()));
}

// ─── Concurrent puts on same key ────────────────────────────

#[test]
fn test_ormap_concurrent_puts_same_key() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();

    let map_a = make_map_with("a", "x", 10, &mut vv_a);
    let map_b = make_map_with("b", "x", 20, &mut vv_b);

    let mut merged = map_a.clone();
    merged.merge(&map_b);

    // Key "x" should exist (both added concurrently)
    assert!(merged.contains_key(&"x".to_string()));
}

// ─── Remove then re-add ─────────────────────────────────────

#[test]
fn test_ormap_remove_then_readd() {
    let mut vv = VersionVector::new();
    let mut map = OrMap::new();

    // Add
    let dot1 = vv.inc("a");
    map.put("x".to_string(), 10, dot1);

    // Remove
    map.remove(&"x".to_string(), &vv);
    assert!(!map.contains_key(&"x".to_string()));

    // Re-add
    let dot2 = vv.inc("a");
    map.put("x".to_string(), 20, dot2);
    assert!(map.contains_key(&"x".to_string()));
    assert_eq!(map.get(&"x".to_string()), Some(&20));
}

// ─── Edge cases ─────────────────────────────────────────────

#[test]
fn test_ormap_new_is_empty() {
    let map: OrMap<String, i32> = OrMap::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
}

#[test]
fn test_ormap_merge_with_empty() {
    let mut vv_a = VersionVector::new();
    let map_a = make_map_with("a", "x", 42, &mut vv_a);
    let empty: OrMap<String, i32> = OrMap::new();

    let mut merged = map_a.clone();
    merged.merge(&empty);
    assert_eq!(merged.get(&"x".to_string()), Some(&42));

    let mut merged2 = empty.clone();
    merged2.merge(&map_a);
    assert_eq!(merged2.get(&"x".to_string()), Some(&42));
}

#[test]
fn test_ormap_multiple_keys() {
    let mut vv = VersionVector::new();
    let mut map = OrMap::new();

    let d1 = vv.inc("a");
    map.put("x".to_string(), 1, d1);
    let d2 = vv.inc("a");
    map.put("y".to_string(), 2, d2);
    let d3 = vv.inc("a");
    map.put("z".to_string(), 3, d3);

    assert_eq!(map.len(), 3);
    assert_eq!(map.get(&"x".to_string()), Some(&1));
    assert_eq!(map.get(&"y".to_string()), Some(&2));
    assert_eq!(map.get(&"z".to_string()), Some(&3));
}
