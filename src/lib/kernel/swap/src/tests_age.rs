use alloc::vec;

use crate::Ager;

#[test]
fn a_page_the_hardware_saw_used_starts_young_again() {
    let mut ager = Ager::new();
    ager.track(1_u64);
    assert_eq!(ager.scan(1, false), Some(1));
    assert_eq!(ager.scan(1, false), Some(2));
    assert_eq!(ager.scan(1, true), Some(0));
}

#[test]
fn age_saturates_and_never_wraps_to_young() {
    let mut ager = Ager::new();
    ager.track(1_u64);
    for _ in 0..300 {
        let _ = ager.scan(1, false);
    }
    assert_eq!(ager.scan(1, false), Some(u8::MAX));
}

#[test]
fn an_untracked_page_has_no_age() {
    let mut ager: Ager<u64> = Ager::new();
    assert_eq!(ager.scan(1, true), None);
}

#[test]
fn victims_are_the_oldest_first_and_only_past_the_threshold() {
    let mut ager = Ager::new();
    for k in 1..=4_u64 {
        ager.track(k);
    }
    for _ in 0..3 {
        let _ = ager.scan(1, false);
        let _ = ager.scan(2, false);
    }
    let _ = ager.scan(2, true); // 2 is young again
    let _ = ager.scan(3, false); // 3 is only 1 old
    assert_eq!(ager.victims(2, 10), vec![1]);
    assert_eq!(ager.victims(1, 10), vec![1, 3]);
}

#[test]
fn victims_respect_the_batch_limit() {
    let mut ager = Ager::new();
    for k in 0..10_u64 {
        ager.track(k);
        let _ = ager.scan(k, false);
    }
    assert_eq!(ager.victims(1, 3).len(), 3);
}

#[test]
fn a_pinned_page_is_never_a_victim() {
    // GPU, DMA, mlock, kernel memory, ring-3 drivers, pid 1 and written
    // shared mappings are all this one flag, set by whoever knows.
    let mut ager = Ager::new();
    ager.track(1_u64);
    ager.pin(1, true);
    for _ in 0..50 {
        let _ = ager.scan(1, false);
    }
    assert!(ager.victims(1, 10).is_empty());
    ager.pin(1, false);
    assert_eq!(ager.victims(1, 10), vec![1]);
}

#[test]
fn a_forgotten_page_is_gone() {
    let mut ager = Ager::new();
    ager.track(1_u64);
    let _ = ager.scan(1, false);
    ager.forget(1);
    assert!(ager.victims(0, 10).is_empty());
    assert_eq!(ager.scan(1, false), None);
}
