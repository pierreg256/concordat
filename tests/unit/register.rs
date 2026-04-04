use concordat::register::MvRegister;
use concordat::vv::VersionVector;

// ─── Helper: build a register with a single set ────────────

fn make_register(replica: &str, value: i32, vv: &mut VersionVector) -> MvRegister<i32> {
    let mut reg = MvRegister::new();
    let dot = vv.inc(replica);
    reg.set(value, dot, vv);
    reg
}

// ─── Commutativity ──────────────────────────────────────────

#[test]
fn test_register_merge_commutativity() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();

    let reg_a = make_register("a", 10, &mut vv_a);
    let reg_b = make_register("b", 20, &mut vv_b);

    // merge(A, B)
    let mut ab = reg_a.clone();
    ab.merge(&reg_b);

    // merge(B, A)
    let mut ba = reg_b.clone();
    ba.merge(&reg_a);

    assert_eq!(ab, ba);
}

#[test]
fn test_register_merge_commutativity_same_key() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();

    let reg_a = make_register("a", 42, &mut vv_a);
    let reg_b = make_register("b", 99, &mut vv_b);

    let mut ab = reg_a.clone();
    ab.merge(&reg_b);

    let mut ba = reg_b.clone();
    ba.merge(&reg_a);

    assert_eq!(ab, ba);
}

// ─── Associativity ──────────────────────────────────────────

#[test]
fn test_register_merge_associativity() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();
    let mut vv_c = VersionVector::new();

    let reg_a = make_register("a", 1, &mut vv_a);
    let reg_b = make_register("b", 2, &mut vv_b);
    let reg_c = make_register("c", 3, &mut vv_c);

    // merge(merge(A, B), C)
    let mut ab = reg_a.clone();
    ab.merge(&reg_b);
    let mut ab_c = ab.clone();
    ab_c.merge(&reg_c);

    // merge(A, merge(B, C))
    let mut bc = reg_b.clone();
    bc.merge(&reg_c);
    let mut a_bc = reg_a.clone();
    a_bc.merge(&bc);

    assert_eq!(ab_c, a_bc);
}

// ─── Idempotence ────────────────────────────────────────────

#[test]
fn test_register_merge_idempotence() {
    let mut vv_a = VersionVector::new();
    let reg_a = make_register("a", 42, &mut vv_a);

    let original = reg_a.clone();
    let mut merged = reg_a.clone();
    merged.merge(&original);
    assert_eq!(merged, original);
}

#[test]
fn test_register_merge_idempotence_repeated() {
    let mut vv_a = VersionVector::new();
    let reg_a = make_register("a", 42, &mut vv_a);

    let snapshot = reg_a.clone();
    let mut merged = reg_a.clone();
    merged.merge(&snapshot);
    merged.merge(&snapshot);
    merged.merge(&snapshot);
    assert_eq!(merged, snapshot);
}

// ─── Sequential writes ─────────────────────────────────────

#[test]
fn test_register_sequential_write_wins() {
    let mut vv = VersionVector::new();
    let mut reg = MvRegister::new();

    // First write
    let dot1 = vv.inc("a");
    reg.set(10, dot1, &vv);
    assert_eq!(reg.value(), Some(&10));

    // Second write (causal successor)
    let dot2 = vv.inc("a");
    reg.set(20, dot2, &vv);
    assert_eq!(reg.value(), Some(&20));
    assert_eq!(reg.values().len(), 1);
}

// ─── Concurrent writes ─────────────────────────────────────

#[test]
fn test_register_concurrent_writes_both_preserved() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();

    let reg_a = make_register("a", 10, &mut vv_a);
    let reg_b = make_register("b", 20, &mut vv_b);

    let mut merged = reg_a.clone();
    merged.merge(&reg_b);

    // Both values should be preserved (concurrent, neither dominates)
    let values = merged.values();
    assert_eq!(values.len(), 2);
    assert!(values.contains(&&10));
    assert!(values.contains(&&20));
    assert_eq!(merged.value(), None); // conflict
}

// ─── Merge after concurrent writes resolves correctly ───────

#[test]
fn test_register_resolve_after_concurrent() {
    let mut vv_a = VersionVector::new();
    let mut vv_b = VersionVector::new();

    // Concurrent writes
    let reg_a = make_register("a", 10, &mut vv_a);
    let reg_b = make_register("b", 20, &mut vv_b);

    // Replica A merges B
    let mut merged = reg_a.clone();
    merged.merge(&reg_b);
    assert_eq!(merged.values().len(), 2);

    // Now A does a causal write that sees both
    let mut resolve_vv = vv_a.clone();
    resolve_vv.merge(&vv_b);
    let dot = resolve_vv.inc("a");
    merged.set(30, dot, &resolve_vv);

    assert_eq!(merged.value(), Some(&30));
    assert_eq!(merged.values().len(), 1);
}

// ─── Edge cases ─────────────────────────────────────────────

#[test]
fn test_register_new_is_empty() {
    let reg: MvRegister<i32> = MvRegister::new();
    assert!(reg.is_empty());
    assert_eq!(reg.value(), None);
    assert!(reg.values().is_empty());
}

#[test]
fn test_register_merge_with_empty() {
    let mut vv_a = VersionVector::new();
    let reg_a = make_register("a", 10, &mut vv_a);
    let empty = MvRegister::new();

    let mut merged = reg_a.clone();
    merged.merge(&empty);
    assert_eq!(merged, reg_a);

    let mut merged2 = empty.clone();
    merged2.merge(&reg_a);
    assert_eq!(merged2, reg_a);
}
