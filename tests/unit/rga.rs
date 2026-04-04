use concordat::rga::Rga;
use concordat::vv::VersionVector;

// ─── Helper ─────────────────────────────────────────────────

fn make_rga(replica: &str, values: &[i32], vv: &mut VersionVector) -> Rga<i32> {
    let mut rga = Rga::new();
    for (i, &v) in values.iter().enumerate() {
        let dot = vv.inc(replica);
        rga.insert(i, v, dot);
    }
    rga
}

// ─── Commutativity ──────────────────────────────────────────

#[test]
fn test_rga_merge_commutativity() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();

    let rga_a = make_rga("a", &[1, 2], &mut vv_a);
    let rga_b = make_rga("b", &[3, 4], &mut vv_b);

    let mut ab = rga_a.clone();
    ab.merge(&rga_b);

    let mut ba = rga_b.clone();
    ba.merge(&rga_a);

    assert_eq!(ab, ba);
}

// ─── Associativity ──────────────────────────────────────────

#[test]
fn test_rga_merge_associativity() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();
    let mut vv_c = VersionVector::new();

    let rga_a = make_rga("a", &[1], &mut vv_a);
    let rga_b = make_rga("b", &[2], &mut vv_b);
    let rga_c = make_rga("c", &[3], &mut vv_c);

    // merge(merge(A, B), C)
    let mut ab = rga_a.clone();
    ab.merge(&rga_b);
    let mut ab_c = ab.clone();
    ab_c.merge(&rga_c);

    // merge(A, merge(B, C))
    let mut bc = rga_b.clone();
    bc.merge(&rga_c);
    let mut a_bc = rga_a.clone();
    a_bc.merge(&bc);

    assert_eq!(ab_c, a_bc);
}

// ─── Idempotence ────────────────────────────────────────────

#[test]
fn test_rga_merge_idempotence() {
    let mut vv_a = VersionVector::new();
    let rga_a = make_rga("a", &[1, 2, 3], &mut vv_a);

    let original = rga_a.clone();
    let mut merged = rga_a.clone();
    merged.merge(&original);
    assert_eq!(merged, original);
}

#[test]
fn test_rga_merge_idempotence_repeated() {
    let mut vv_a = VersionVector::new();
    let rga_a = make_rga("a", &[10, 20], &mut vv_a);

    let snapshot = rga_a.clone();
    let mut merged = rga_a.clone();
    merged.merge(&snapshot);
    merged.merge(&snapshot);
    merged.merge(&snapshot);
    assert_eq!(merged, snapshot);
}

// ─── Sequential insert/delete ───────────────────────────────

#[test]
fn test_rga_sequential_insert_delete() {
    let mut vv = VersionVector::new();
    let mut rga = Rga::new();

    let d1 = vv.inc("a");
    rga.insert(0, 10, d1);
    let d2 = vv.inc("a");
    rga.insert(1, 20, d2);
    let d3 = vv.inc("a");
    rga.insert(2, 30, d3);

    assert_eq!(rga.to_vec(), vec![&10, &20, &30]);

    // Delete middle
    rga.delete(1);
    assert_eq!(rga.to_vec(), vec![&10, &30]);

    // Delete head
    rga.delete(0);
    assert_eq!(rga.to_vec(), vec![&30]);
}

// ─── Concurrent insert at same index ────────────────────────

#[test]
fn test_rga_concurrent_insert_same_index() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();

    // Both replicas insert at position 0
    let mut rga_a = Rga::new();
    let dot_a = vv_a.inc("a");
    rga_a.insert(0, 10, dot_a);

    let mut rga_b = Rga::new();
    let dot_b = vv_b.inc("b");
    rga_b.insert(0, 20, dot_b);

    // Merge both ways — must produce same order
    let mut ab = rga_a.clone();
    ab.merge(&rga_b);

    let mut ba = rga_b.clone();
    ba.merge(&rga_a);

    assert_eq!(ab, ba);
    assert_eq!(ab.len(), 2);
}

// ─── Concurrent delete of same element ──────────────────────

#[test]
fn test_rga_concurrent_delete_same_element() {
    let mut vv = VersionVector::new();
    let mut rga = Rga::new();
    let d1 = vv.inc("a");
    rga.insert(0, 42, d1);

    // Both replicas delete index 0
    let mut rga_a = rga.clone();
    rga_a.delete(0);

    let mut rga_b = rga.clone();
    rga_b.delete(0);

    // Merge — should still be empty
    let mut merged = rga_a.clone();
    merged.merge(&rga_b);
    assert_eq!(merged.len(), 0);

    let mut merged2 = rga_b.clone();
    merged2.merge(&rga_a);
    assert_eq!(merged2.len(), 0);
}

// ─── Insert after tombstone ─────────────────────────────────

#[test]
fn test_rga_insert_after_tombstone() {
    let mut vv = VersionVector::new();
    let mut rga = Rga::new();

    // Insert [10, 20, 30]
    let d1 = vv.inc("a");
    rga.insert(0, 10, d1);
    let d2 = vv.inc("a");
    rga.insert(1, 20, d2);
    let d3 = vv.inc("a");
    rga.insert(2, 30, d3);

    // Delete middle (20 becomes tombstone)
    rga.delete(1);
    assert_eq!(rga.to_vec(), vec![&10, &30]);

    // Insert at visible index 1 (after 10, where 20 used to be)
    let d4 = vv.inc("a");
    rga.insert(1, 25, d4);
    assert_eq!(rga.to_vec(), vec![&10, &25, &30]);
}

// ─── Multiple inserts at different positions ────────────────

#[test]
fn test_rga_inserts_at_different_positions() {
    let mut vv = VersionVector::new();
    let mut rga = Rga::new();

    let d1 = vv.inc("a");
    rga.insert(0, 1, d1); // [1]

    let d2 = vv.inc("a");
    rga.insert(0, 0, d2); // insert at head: [0, 1]  (d2 > d1, so 0 comes first)

    let d3 = vv.inc("a");
    rga.insert(2, 2, d3); // insert at end: [0, 1, 2]

    let d4 = vv.inc("a");
    rga.insert(1, 99, d4); // insert after 0: [0, 99, 1, 2]

    assert_eq!(rga.to_vec(), vec![&0, &99, &1, &2]);
}

// ─── Delete at head, middle, tail ───────────────────────────

#[test]
fn test_rga_delete_head_middle_tail() {
    let mut vv = VersionVector::new();
    let mut rga = make_rga("a", &[1, 2, 3, 4, 5], &mut vv);

    rga.delete(4); // tail → [1, 2, 3, 4]
    assert_eq!(rga.to_vec(), vec![&1, &2, &3, &4]);

    rga.delete(0); // head → [2, 3, 4]
    assert_eq!(rga.to_vec(), vec![&2, &3, &4]);

    rga.delete(1); // middle → [2, 4]
    assert_eq!(rga.to_vec(), vec![&2, &4]);
}

// ─── Edge cases ─────────────────────────────────────────────

#[test]
fn test_rga_new_is_empty() {
    let rga: Rga<i32> = Rga::new();
    assert!(rga.is_empty());
    assert_eq!(rga.len(), 0);
    assert_eq!(rga.get(0), None);
}

#[test]
fn test_rga_merge_with_empty() {
    let mut vv_a = VersionVector::new();
    let rga_a = make_rga("a", &[1, 2, 3], &mut vv_a);
    let empty: Rga<i32> = Rga::new();

    let mut merged = rga_a.clone();
    merged.merge(&empty);
    assert_eq!(merged, rga_a);

    let mut merged2 = empty.clone();
    merged2.merge(&rga_a);
    assert_eq!(merged2, rga_a);
}

#[test]
fn test_rga_get_by_index() {
    let mut vv = VersionVector::new();
    let rga = make_rga("a", &[10, 20, 30], &mut vv);

    assert_eq!(rga.get(0), Some(&10));
    assert_eq!(rga.get(1), Some(&20));
    assert_eq!(rga.get(2), Some(&30));
    assert_eq!(rga.get(3), None);
}
