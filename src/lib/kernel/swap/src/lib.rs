//! Swap, written as a library the kernel will wire in: see
//! docs/MEMORY-COMPRESSION.md. Nothing here touches a frame or a page table;
//! it holds the state machine so the rules (a page that cannot leave stays, a
//! bad read kills the owner and never the kernel) are tested on the host.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod age;
mod anon;
mod entry;

pub use age::Ager;
pub use anon::{Anon, Slot};
pub use entry::Entry;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_age;
