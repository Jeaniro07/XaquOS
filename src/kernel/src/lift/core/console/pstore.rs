use crate::arch;
use crate::console::log;
use crate::console::println;
use crate::mm;
use core::sync::atomic::AtomicU8;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use xaqu_bootinfo::BootView;
use xaqu_bootinfo::MemKind;
use xaqu_bootinfo::PAGE_SIZE;
use xaqu_bootinfo::pstore;
use xaqu_bootinfo::pstore::HEADER_BYTES;
use xaqu_bootinfo::pstore::STOP_OFFSET;
use xaqu_bootinfo::pstore::Stop;
use xaqu_bootinfo::pstore::TABLE_COUNT_OFFSET;
use xaqu_bootinfo::pstore::TABLE_ENTRIES;
use xaqu_bootinfo::pstore::TABLE_OFFSET;
use xaqu_bootinfo::pstore::WRITTEN_OFFSET;
use xaqu_sync::Once;
pub fn stop(stop: Stop) -> bool {
    false
}

pub fn record(bytes: &[u8]) {}
