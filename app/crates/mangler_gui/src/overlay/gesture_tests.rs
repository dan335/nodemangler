//! Unit tests for the shared overlay gesture flags.

use super::*;

#[test]
fn default_and_idle_agree() {
    assert_eq!(Gesture::default(), Gesture::IDLE);
}

#[test]
fn constructors_set_the_expected_flags() {
    assert_eq!(Gesture::dragging(), Gesture { changed: true, commit: false });
    assert_eq!(Gesture::edited(), Gesture { changed: true, commit: true });
    assert_eq!(Gesture::released(), Gesture { changed: false, commit: true });
}

#[test]
fn merge_is_a_field_wise_or() {
    let mut g = Gesture::IDLE;
    g.merge(Gesture::dragging());
    assert_eq!(g, Gesture { changed: true, commit: false });
    g.merge(Gesture::released());
    assert_eq!(g, Gesture { changed: true, commit: true });
}

#[test]
fn merge_is_order_independent_and_sticky() {
    // The whole point of OR-folding: an editor accumulates across handles in
    // whatever order they happen to be registered, and a later idle frame from
    // one handle never clears an earlier handle's result.
    let all = [Gesture::IDLE, Gesture::dragging(), Gesture::released(), Gesture::edited()];
    for a in all {
        for b in all {
            let mut ab = a;
            ab.merge(b);
            let mut ba = b;
            ba.merge(a);
            assert_eq!(ab, ba, "merge({a:?}, {b:?}) should be order-independent");
            assert!(ab.changed >= a.changed && ab.changed >= b.changed);
            assert!(ab.commit >= a.commit && ab.commit >= b.commit);
        }
    }
}

#[test]
fn merging_idle_never_clears_anything() {
    let mut g = Gesture::edited();
    g.merge(Gesture::IDLE);
    assert_eq!(g, Gesture::edited());
}
