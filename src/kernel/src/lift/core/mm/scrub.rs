use crate::sync::SpinLock;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicU8;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use xaqu_bootinfo::BootView;
use xaqu_bootinfo::PAGE_SIZE;
use xaqu_frame::Frame;
pub fn render(out: &mut impl core::fmt::Write) -> core::fmt::Result {
    Ok(())
}
