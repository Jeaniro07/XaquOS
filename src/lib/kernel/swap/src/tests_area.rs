use alloc::vec;
use alloc::vec::Vec;

use crate::{Area, Device, DeviceError, Entry, PAGE, ReadError, SwapOnError, WriteError};

/// A disk in RAM that a test can break.
struct Ram {
    sectors: Vec<[u8; PAGE]>,
    fail_write: bool,
    fail_read: bool,
}

impl Ram {
    fn new(slots: usize) -> Self {
        Self {
            sectors: vec![[0; PAGE]; slots],
            fail_write: false,
            fail_read: false,
        }
    }
}

impl Device for Ram {
    fn write(&mut self, slot: u32, data: &[u8; PAGE]) -> Result<(), DeviceError> {
        if self.fail_write {
            return Err(DeviceError);
        }
        let sector = self.sectors.get_mut(slot as usize).ok_or(DeviceError)?;
        *sector = *data;
        Ok(())
    }

    fn read(&mut self, slot: u32, out: &mut [u8; PAGE]) -> Result<(), DeviceError> {
        if self.fail_read {
            return Err(DeviceError);
        }
        *out = *self.sectors.get(slot as usize).ok_or(DeviceError)?;
        Ok(())
    }
}

const KEY: [u8; 32] = [7; 32];

fn secret() -> [u8; PAGE] {
    let mut page = [0_u8; PAGE];
    page.get_mut(..16)
        .expect("a page has 16 bytes")
        .copy_from_slice(b"SECRET-PASSWORD!");
    page
}

fn on(slots: u32) -> Area<Ram> {
    let mut area = Area::new();
    area.swapon(Ram::new(slots as usize), slots, KEY)
        .expect("swapon");
    area
}

fn disk(entry: Entry) -> u32 {
    match entry {
        Entry::Disk(slot) => slot,
        Entry::Pool(_) => panic!("a disk entry was expected"),
    }
}

#[test]
fn swap_is_off_until_it_is_turned_on() {
    let mut area: Area<Ram> = Area::new();
    assert!(!area.is_on());
    assert_eq!(area.write(&secret()), Err(WriteError::Off));
}

#[test]
fn an_area_with_no_slots_is_refused() {
    let mut area = Area::new();
    assert_eq!(area.swapon(Ram::new(0), 0, KEY), Err(SwapOnError::Empty));
    assert!(!area.is_on());
}

#[test]
fn a_page_comes_back_byte_for_byte() {
    let mut area = on(4);
    let page = secret();
    let entry = area.write(&page).expect("written");
    let mut out = [0_u8; PAGE];
    assert_eq!(area.read(entry, &mut out), Ok(()));
    assert_eq!(out, page);
}

#[test]
fn the_disk_never_sees_the_plaintext() {
    let mut area = on(4);
    let entry = area.write(&secret()).expect("written");
    let sector = area
        .device()
        .expect("device")
        .sectors
        .get(disk(entry) as usize)
        .expect("slot");
    assert_ne!(*sector, secret());
    assert!(!sector.windows(6).any(|w| w == b"SECRET"));
}

#[test]
fn the_same_page_written_twice_is_two_different_ciphertexts() {
    let mut area = on(4);
    let a = area.write(&secret()).expect("written");
    let b = area.write(&secret()).expect("written");
    let dev = area.device().expect("device");
    assert_ne!(
        dev.sectors.get(disk(a) as usize),
        dev.sectors.get(disk(b) as usize)
    );
}

#[test]
fn a_flipped_bit_on_disk_is_corrupt_and_frees_the_slot() {
    let mut area = on(4);
    let entry = area.write(&secret()).expect("written");
    let slot = disk(entry) as usize;
    if let Some(byte) = area
        .device_mut()
        .expect("device")
        .sectors
        .get_mut(slot)
        .and_then(|s| s.get_mut(100))
    {
        *byte ^= 1;
    }
    let mut out = [0_u8; PAGE];
    assert_eq!(area.read(entry, &mut out), Err(ReadError::Corrupt));
    assert_eq!(area.used(), 0);
    // Whatever was decoded must not be handed out as the page.
    assert_eq!(out, [0_u8; PAGE]);
}

#[test]
fn a_read_error_frees_the_slot_and_leaves_swap_on() {
    let mut area = on(4);
    let entry = area.write(&secret()).expect("written");
    area.device_mut().expect("device").fail_read = true;
    let mut out = [0_u8; PAGE];
    assert_eq!(area.read(entry, &mut out), Err(ReadError::Io));
    assert_eq!(area.used(), 0);
    assert!(area.is_on());
}

#[test]
fn a_failed_write_turns_disk_swap_off_but_old_pages_still_read() {
    let mut area = on(4);
    let kept = area.write(&secret()).expect("written");
    area.device_mut().expect("device").fail_write = true;
    assert_eq!(area.write(&secret()), Err(WriteError::Failed));
    assert!(area.is_bad());
    assert!(!area.is_on());
    area.device_mut().expect("device").fail_write = false;
    assert_eq!(area.write(&secret()), Err(WriteError::Off));
    let mut out = [0_u8; PAGE];
    assert_eq!(area.read(kept, &mut out), Ok(()));
    assert_eq!(out, secret());
}

#[test]
fn a_full_area_refuses_and_a_freed_slot_is_reused() {
    let mut area = on(2);
    let a = area.write(&secret()).expect("written");
    let _ = area.write(&secret()).expect("written");
    assert_eq!(area.write(&secret()), Err(WriteError::Full));
    area.free(a);
    assert!(area.write(&secret()).is_ok());
}

#[test]
fn a_read_frees_the_slot_and_a_second_read_finds_nothing() {
    let mut area = on(2);
    let entry = area.write(&secret()).expect("written");
    let mut out = [0_u8; PAGE];
    assert_eq!(area.read(entry, &mut out), Ok(()));
    assert_eq!(area.read(entry, &mut out), Err(ReadError::Missing));
    assert_eq!(area.read(Entry::Pool(0), &mut out), Err(ReadError::Missing));
}

#[test]
fn only_whole_pages_go_out_and_come_in() {
    let mut area = on(2);
    assert_eq!(area.write(&[0_u8; 10]), Err(WriteError::BadSize));
    let entry = area.write(&secret()).expect("written");
    let mut small = [0_u8; 10];
    assert_eq!(area.read(entry, &mut small), Err(ReadError::BadSize));
    let mut out = [0_u8; PAGE];
    assert_eq!(area.read(entry, &mut out), Ok(()));
}
