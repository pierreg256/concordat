use concordat::vv::{Dot, VersionVector};

// ─── Dot basics ─────────────────────────────────────────────

#[test]
fn test_dot_equality() {
    let d1 = Dot { replica: "a".into(), counter: 1 };
    let d2 = Dot { replica: "a".into(), counter: 1 };
    let d3 = Dot { replica: "a".into(), counter: 2 };
    assert_eq!(d1, d2);
    assert_ne!(d1, d3);
}

// ─── VersionVector::inc ─────────────────────────────────────

#[test]
fn test_inc_produces_sequential_dots() {
    let mut vv = VersionVector::new();
    let d1 = vv.inc("a");
    let d2 = vv.inc("a");
    let d3 = vv.inc("a");
    assert_eq!(d1.counter, 1);
    assert_eq!(d2.counter, 2);
    assert_eq!(d3.counter, 3);
    assert_eq!(d1.replica, "a");
}

#[test]
fn test_inc_multiple_replicas() {
    let mut vv = VersionVector::new();
    let da = vv.inc("a");
    let db = vv.inc("b");
    let da2 = vv.inc("a");
    assert_eq!(da.counter, 1);
    assert_eq!(db.counter, 1);
    assert_eq!(da2.counter, 2);
    assert_eq!(vv.get("a"), 2);
    assert_eq!(vv.get("b"), 1);
}

// ─── VersionVector::contains ────────────────────────────────

#[test]
fn test_contains_seen_dots() {
    let mut vv = VersionVector::new();
    vv.inc("a"); // counter = 1
    vv.inc("a"); // counter = 2

    assert!(vv.contains(&Dot { replica: "a".into(), counter: 1 }));
    assert!(vv.contains(&Dot { replica: "a".into(), counter: 2 }));
    assert!(!vv.contains(&Dot { replica: "a".into(), counter: 3 }));
}

#[test]
fn test_contains_unseen_replica() {
    let vv = VersionVector::new();
    assert!(!vv.contains(&Dot { replica: "x".into(), counter: 1 }));
}

// ─── VersionVector::merge — Commutativity ───────────────────

#[test]
fn test_merge_commutativity() {
    let mut a = VersionVector::new();
    a.inc("x");
    a.inc("x");
    a.inc("y");

    let mut b = VersionVector::new();
    b.inc("x");
    b.inc("y");
    b.inc("y");
    b.inc("z");

    // merge(A, B)
    let mut ab = a.clone();
    ab.merge(&b);

    // merge(B, A)
    let mut ba = b.clone();
    ba.merge(&a);

    assert_eq!(ab, ba);
}

#[test]
fn test_merge_commutativity_disjoint() {
    let mut a = VersionVector::new();
    a.inc("a");

    let mut b = VersionVector::new();
    b.inc("b");

    let mut ab = a.clone();
    ab.merge(&b);

    let mut ba = b.clone();
    ba.merge(&a);

    assert_eq!(ab, ba);
}

// ─── VersionVector::merge — Associativity ───────────────────

#[test]
fn test_merge_associativity() {
    let mut a = VersionVector::new();
    a.inc("x");
    a.inc("x");

    let mut b = VersionVector::new();
    b.inc("y");
    b.inc("y");

    let mut c = VersionVector::new();
    c.inc("x");
    c.inc("z");

    // merge(merge(A, B), C)
    let mut ab = a.clone();
    ab.merge(&b);
    let mut ab_c = ab.clone();
    ab_c.merge(&c);

    // merge(A, merge(B, C))
    let mut bc = b.clone();
    bc.merge(&c);
    let mut a_bc = a.clone();
    a_bc.merge(&bc);

    assert_eq!(ab_c, a_bc);
}

// ─── VersionVector::merge — Idempotence ─────────────────────

#[test]
fn test_merge_idempotence() {
    let mut a = VersionVector::new();
    a.inc("x");
    a.inc("y");
    a.inc("y");

    let original = a.clone();

    // merge(A, A) == A
    a.merge(&original);
    assert_eq!(a, original);
}

#[test]
fn test_merge_idempotence_repeated() {
    let mut a = VersionVector::new();
    a.inc("r1");
    a.inc("r2");

    let snapshot = a.clone();
    a.merge(&snapshot);
    a.merge(&snapshot);
    a.merge(&snapshot);
    assert_eq!(a, snapshot);
}

// ─── VersionVector::merge — Point-wise max ──────────────────

#[test]
fn test_merge_takes_max() {
    let mut a = VersionVector::new();
    a.inc("x"); // x=1
    a.inc("x"); // x=2
    a.inc("x"); // x=3

    let mut b = VersionVector::new();
    b.inc("x"); // x=1
    b.inc("y"); // y=1

    let mut merged = a.clone();
    merged.merge(&b);

    assert_eq!(merged.get("x"), 3); // max(3, 1)
    assert_eq!(merged.get("y"), 1); // max(0, 1)
}

// ─── VersionVector::delta_since ─────────────────────────────

#[test]
fn test_delta_since_returns_diff() {
    let mut a = VersionVector::new();
    a.inc("x"); // x=1
    a.inc("x"); // x=2
    a.inc("y"); // y=1

    let mut old = VersionVector::new();
    old.inc("x"); // x=1

    let delta = a.delta_since(&old);

    // x advanced from 1→2, y is new
    assert_eq!(delta.get("x"), 2);
    assert_eq!(delta.get("y"), 1);
}

#[test]
fn test_delta_since_empty_returns_full() {
    let mut a = VersionVector::new();
    a.inc("x");
    a.inc("y");

    let empty = VersionVector::new();
    let delta = a.delta_since(&empty);

    assert_eq!(delta, a);
}

#[test]
fn test_delta_since_same_returns_empty() {
    let mut a = VersionVector::new();
    a.inc("x");
    a.inc("y");

    let delta = a.delta_since(&a);
    assert!(delta.is_empty());
}

#[test]
fn test_delta_since_no_regression() {
    let mut old = VersionVector::new();
    old.inc("x"); // x=1
    old.inc("x"); // x=2
    old.inc("y"); // y=1

    let mut new = VersionVector::new();
    new.inc("x"); // x=1 (behind old on x)
    new.inc("y"); // y=1
    new.inc("z"); // z=1 (new)

    let delta = new.delta_since(&old);

    // x is behind, shouldn't appear; z is new
    assert_eq!(delta.get("x"), 0);
    assert_eq!(delta.get("y"), 0);
    assert_eq!(delta.get("z"), 1);
}

// ─── VersionVector — edge cases ─────────────────────────────

#[test]
fn test_new_is_empty() {
    let vv = VersionVector::new();
    assert!(vv.is_empty());
    assert_eq!(vv.get("anything"), 0);
}

#[test]
fn test_merge_with_empty() {
    let mut a = VersionVector::new();
    a.inc("x");

    let original = a.clone();
    let empty = VersionVector::new();

    a.merge(&empty);
    assert_eq!(a, original);

    let mut b = VersionVector::new();
    b.merge(&original);
    assert_eq!(b, original);
}
