use crate::{Anon, Entry, Slot};

#[test]
fn a_swapped_page_is_not_an_absent_page() {
    let mut anon: Anon<u32> = Anon::new();
    assert!(anon.insert(3, 77).is_ok());
    assert_eq!(anon.swap_out(3, Entry::Pool(9)), Ok(77));
    // Absent would mean "zero fill" to the fault path; Swapped must not.
    assert_eq!(anon.get(3), Some(&Slot::Swapped(Entry::Pool(9))));
    assert_eq!(anon.get(4), None);
}

#[test]
fn swap_in_returns_the_entry_to_free() {
    let mut anon: Anon<u32> = Anon::new();
    assert!(anon.insert(0, 1).is_ok());
    assert_eq!(anon.swap_out(0, Entry::Disk(5)), Ok(1));
    assert_eq!(anon.swap_in(0, 2), Ok(Entry::Disk(5)));
    assert_eq!(anon.get(0), Some(&Slot::Ram(2)));
}

#[test]
fn a_page_already_out_or_absent_cannot_go_out_again() {
    let mut anon: Anon<u32> = Anon::new();
    assert_eq!(anon.swap_out(0, Entry::Pool(1)), Err(Entry::Pool(1)));
    assert!(anon.insert(0, 1).is_ok());
    assert_eq!(anon.swap_out(0, Entry::Pool(1)), Ok(1));
    assert_eq!(anon.swap_out(0, Entry::Pool(2)), Err(Entry::Pool(2)));
}

#[test]
fn swap_in_of_a_resident_page_hands_the_frame_back() {
    let mut anon: Anon<u32> = Anon::new();
    assert!(anon.insert(0, 1).is_ok());
    assert_eq!(anon.swap_in(0, 9), Err(9));
    assert_eq!(anon.swap_in(5, 9), Err(9));
}

#[test]
fn insert_never_overwrites_a_swapped_page() {
    let mut anon: Anon<u32> = Anon::new();
    assert!(anon.insert(0, 1).is_ok());
    assert_eq!(anon.swap_out(0, Entry::Pool(1)), Ok(1));
    assert_eq!(anon.insert(0, 2), Err(2));
    assert_eq!(anon.get(0), Some(&Slot::Swapped(Entry::Pool(1))));
}

#[test]
fn counts_follow_the_transitions() {
    let mut anon: Anon<u32> = Anon::new();
    for i in 0..4 {
        assert!(anon.insert(i, 10 + u32::try_from(i).unwrap_or(0)).is_ok());
    }
    assert_eq!((anon.resident(), anon.swapped()), (4, 0));
    assert!(anon.swap_out(1, Entry::Pool(0)).is_ok());
    assert!(anon.swap_out(2, Entry::Disk(0)).is_ok());
    assert_eq!((anon.resident(), anon.swapped()), (2, 2));
    assert!(anon.swap_in(1, 99).is_ok());
    assert_eq!((anon.resident(), anon.swapped()), (3, 1));
}

#[test]
fn retire_gives_back_every_frame_and_every_entry() {
    let mut anon: Anon<u32> = Anon::new();
    for i in 0..3 {
        assert!(anon.insert(i, 100 + u32::try_from(i).unwrap_or(0)).is_ok());
    }
    assert!(anon.swap_out(1, Entry::Pool(7)).is_ok());
    let (mut frames, entries) = anon.retire();
    frames.sort_unstable();
    assert_eq!(frames, [100, 102]);
    assert_eq!(entries, [Entry::Pool(7)]);
}
