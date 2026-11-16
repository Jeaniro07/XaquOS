//! Which of its two descriptions a machine is read by.
//!
//! A machine describes itself with ACPI tables, with a device tree, or --
//! on AArch64 under EDK2 -- with both, and where both are there the ACPI
//! tables are authoritative. Everything that finds devices, the IOMMUs and
//! the PCI hosts asks this first, and [`of`] is the one place it is answered.
//! The decision itself is `xaqu_description::choose`, tested on the host
//! for every combination; this opens the two for it.
//!
//! Code that only ever reads the tree -- the console, the timers, the other
//! processors, a board's own peripherals -- opens it with [`fdt::open`]
//! directly, as before: on those machines it is the only description there
//! is.

use xaqu_bootinfo::BootView;
use xaqu_fdt::Fdt;

use crate::discovery::{acpi, fdt};

/// How a machine describes itself: [`Description::Acpi`] with its opened
/// tables, [`Description::Tree`] with its parsed tree, or
/// [`Description::Neither`].
pub type Description = xaqu_description::Description<acpi::Firmware, Fdt<'static>>;

/// The description `view`'s machine is read by: its ACPI tables if they
/// open, else its device tree if it parses, else neither. The tree is not
/// parsed when the tables open.
pub fn of(view: &BootView<'_>) -> Description {
    xaqu_description::choose(acpi::Firmware::open(view), || fdt::open(view))
}
