//! `/proc/swaps`.
//!
//! ```text
//! Filename                                Type            Size            Used            Priority
//! /dev/sdc                                partition       2097152         3864            -2
//! ```
//!
//! The header is always there, even with no swap area, as `swaps_show` in
//! `mm/swapfile.c` prints it first. A row is the path, padded to forty
//! columns (one space after a path that is longer), the type, and the size,
//! the use and the priority, the sizes in KiB.
//!
//! The tabs are the host's, and Linux's: `swap_show` prints a second tab
//! after a number under ten million and none after a wider one, and it
//! prints `file` with a tab of its own, so the columns line up whatever the
//! numbers are.

use alloc::vec::Vec;

use crate::text::{pad_to, put};

/// The header line.
pub const HEADER: &[u8] = b"Filename\t\t\t\tType\t\tSize\t\tUsed\t\tPriority\n";

/// The column the type starts at.
const TYPE_COLUMN: usize = 40;

/// The number from which Linux prints no second tab.
const WIDE: u64 = 10_000_000;

/// What a swap area sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapKind {
    /// A block device.
    Partition,
    /// A regular file.
    File,
}

/// One swap area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Swap<'a> {
    /// The path it was turned on with.
    pub path: &'a [u8],
    /// Partition or file.
    pub kind: SwapKind,
    /// The size in KiB.
    pub size_kib: u64,
    /// The part in use, in KiB.
    pub used_kib: u64,
    /// The priority, which may be negative.
    pub priority: i32,
}

/// Append the header and a row for each of `swaps`.
pub fn render(out: &mut Vec<u8>, swaps: &[Swap<'_>]) {
    out.extend_from_slice(HEADER);
    for swap in swaps {
        let start = out.len();
        out.extend_from_slice(swap.path);
        if out.len().saturating_sub(start) >= TYPE_COLUMN {
            out.push(b' ');
        } else {
            pad_to(out, start, TYPE_COLUMN);
        }
        let kind: &[u8] = match swap.kind {
            SwapKind::Partition => b"partition",
            SwapKind::File => b"file\t",
        };
        out.extend_from_slice(kind);
        put(out, format_args!("\t{}", swap.size_kib));
        if swap.size_kib < WIDE {
            out.push(b'\t');
        }
        put(out, format_args!("\t{}", swap.used_kib));
        if swap.used_kib < WIDE {
            out.push(b'\t');
        }
        put(out, format_args!("\t{}\n", swap.priority));
    }
}
