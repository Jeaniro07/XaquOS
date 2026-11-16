use crate::mmio::Mmio;
use core::sync::atomic::AtomicU8;
use core::sync::atomic::AtomicU32;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;
use xaqu_acpi::Acpi;
use xaqu_acpi::GenericAddress;
use xaqu_acpi::hpet;
use xaqu_acpi::hpet::Capabilities;
pub fn counter_hz() -> u64 {
    1_000_000_000
}
/// Whether ring 3 reads the kernel's counter: the vDSO's question.
pub fn counter_is_tsc() -> bool {
    super::tsc_hz() != 0
}
