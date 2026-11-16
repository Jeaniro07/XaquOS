use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use xaqu_sched::MAX_CPUS;
pub const HARDENED: bool = !cfg!(xaqu_mitigations = "off");
pub fn switch_barriers() -> u64 {
    0
}
