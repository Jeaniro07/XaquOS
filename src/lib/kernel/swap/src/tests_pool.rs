use crate::{Entry, LoadError, PAGE, Pool, Refused};

fn zeros() -> [u8; PAGE] {
    [0; PAGE]
}

fn text() -> [u8; PAGE] {
    let mut page = [0_u8; PAGE];
    for (i, byte) in page.iter_mut().enumerate() {
        *byte = b"xaqu swap pool "[i % 15];
    }
    page
}

/// xorshift noise: LZ4 cannot shrink it.
fn noise() -> [u8; PAGE] {
    let mut page = [0_u8; PAGE];
    let mut x = 0x9e37_79b9_7f4a_7c15_u64;
    for byte in &mut page {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *byte = x as u8;
    }
    page
}

fn pool(budget: usize) -> Pool {
    Pool::new(budget).expect("a pool of this size allocates")
}

#[test]
fn a_compressible_page_comes_back_byte_for_byte() {
    let mut pool = pool(1 << 20);
    for page in [zeros(), text()] {
        let entry = pool.store(&page).expect("compressible");
        let mut out = [0xAA_u8; PAGE];
        assert_eq!(pool.load(entry, &mut out), Ok(()));
        assert_eq!(out, page);
    }
}

#[test]
fn stored_bytes_are_far_below_a_page() {
    let mut pool = pool(1 << 20);
    let _ = pool.store(&zeros()).expect("compressible");
    assert!(pool.used() < PAGE / 8, "used {}", pool.used());
}

#[test]
fn a_poorly_compressing_page_stays_in_ram() {
    let mut pool = pool(1 << 20);
    assert_eq!(pool.store(&noise()), Err(Refused::Poor));
    assert_eq!(pool.used(), 0);
    assert_eq!(pool.stats().poor, 1);
}

#[test]
fn a_full_pool_refuses_and_keeps_what_it_has() {
    let one = {
        let mut probe = pool(1 << 20);
        let _ = probe.store(&zeros()).expect("compressible");
        probe.used()
    };
    let mut pool = pool(one); // room for exactly one zero page
    let first = pool.store(&zeros()).expect("fits the budget exactly");
    let bytes = pool.used();
    assert_eq!(bytes, one);
    assert_eq!(pool.store(&text()), Err(Refused::Full));
    assert_eq!(pool.used(), bytes);
    assert_eq!(pool.stats().full, 1);
    let mut out = [1_u8; PAGE];
    assert_eq!(pool.load(first, &mut out), Ok(()));
    assert_eq!(out, zeros());
}

#[test]
fn a_load_frees_the_slot_and_a_second_load_finds_nothing() {
    let mut pool = pool(1 << 20);
    let entry = pool.store(&text()).expect("compressible");
    let mut out = [0_u8; PAGE];
    assert_eq!(pool.load(entry, &mut out), Ok(()));
    assert_eq!(pool.used(), 0);
    assert_eq!(pool.load(entry, &mut out), Err(LoadError::Missing));
}

#[test]
fn free_returns_the_bytes_and_the_slot_is_reused() {
    let mut pool = pool(1 << 20);
    let a = pool.store(&text()).expect("compressible");
    pool.free(a);
    assert_eq!(pool.used(), 0);
    let b = pool.store(&zeros()).expect("compressible");
    assert_eq!(a, b);
}

#[test]
fn a_disk_entry_is_not_the_pools() {
    let mut pool = pool(1 << 20);
    let mut out = [0_u8; PAGE];
    assert_eq!(pool.load(Entry::Disk(0), &mut out), Err(LoadError::Missing));
}

#[test]
fn only_whole_pages_go_in_and_come_out() {
    let mut pool = pool(1 << 20);
    assert_eq!(pool.store(&[0_u8; 100]), Err(Refused::BadSize));
    let entry = pool.store(&text()).expect("compressible");
    let mut small = [0_u8; 100];
    assert_eq!(pool.load(entry, &mut small), Err(LoadError::BadSize));
    // A refused read keeps the page: it was the caller's buffer that was wrong.
    let mut out = [0_u8; PAGE];
    assert_eq!(pool.load(entry, &mut out), Ok(()));
}

#[test]
fn a_damaged_block_is_an_error_not_a_panic_and_frees_the_slot() {
    let mut pool = pool(1 << 20);
    let entry = pool.store(&text()).expect("compressible");
    pool.damage(entry);
    let mut out = [0_u8; PAGE];
    assert_eq!(pool.load(entry, &mut out), Err(LoadError::Corrupt));
    assert_eq!(pool.used(), 0);
}

#[test]
fn stats_count_what_happened() {
    let mut pool = pool(1 << 20);
    let a = pool.store(&text()).expect("compressible");
    let _ = pool.store(&noise());
    let mut out = [0_u8; PAGE];
    assert_eq!(pool.load(a, &mut out), Ok(()));
    let s = pool.stats();
    assert_eq!((s.stored, s.loaded, s.poor, s.pages), (1, 1, 1, 0));
}
