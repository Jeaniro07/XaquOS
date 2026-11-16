use super::amdvi;
use super::vtd;
use crate::iommu::Unit;
use crate::panic::catalog;
use crate::panic::fatal;
use crate::println;
use alloc::vec::Vec;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use xaqu_acpi::dmar;
use xaqu_acpi::dmar::Structure;
use xaqu_paging::vtd::remap;
use xaqu_paging::vtd::remap::Remap;
use xaqu_paging::vtd::remap::UnitState;
use xaqu_pci::Address;
use xaqu_sync::Once;
#[derive(Clone, Copy, Debug)]
pub enum Route {
    Compatibility,
}
