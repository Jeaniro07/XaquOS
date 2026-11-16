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

pub use crate::text::Full;
use crate::text::{push_within, put_within};

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

/// Append the header and a row for each of `swaps`, within `out`'s
/// capacity: nothing here allocates. Reserve [`longest_row`] for every
/// row, and the header's own bytes, before calling.
///
/// # Errors
///
/// [`Full`] when a row would not fit in what `out` has room for; part of
/// it may have been written, and the caller abandons the file.
pub fn render(out: &mut Vec<u8>, swaps: &[Swap<'_>]) -> Result<(), Full> {
    push_within(out, HEADER)?;
    for swap in swaps {
        let start = out.len();
        push_within(out, swap.path)?;
        if out.len().saturating_sub(start) >= TYPE_COLUMN {
            push_within(out, b" ")?;
        } else {
            // The type starts at the fortieth column, so a short path is
            // followed by spaces until then.
            while out.len().saturating_sub(start) < TYPE_COLUMN {
                push_within(out, b" ")?;
            }
        }
        let kind: &[u8] = match swap.kind {
            SwapKind::Partition => b"partition",
            SwapKind::File => b"file\t",
        };
        push_within(out, kind)?;
        put_within(out, format_args!("\t{}", swap.size_kib))?;
        if swap.size_kib < WIDE {
            push_within(out, b"\t")?;
        }
        put_within(out, format_args!("\t{}", swap.used_kib))?;
        if swap.used_kib < WIDE {
            push_within(out, b"\t")?;
        }
        put_within(out, format_args!("\t{}\n", swap.priority))?;
    }
    Ok(())
}

/// The most bytes a row for a path of `path_len` bytes takes.
///
/// The name's field is the path and a space, or the path and the padding
/// to the fortieth column; then the type, the two numbers with their
/// tabs, and the priority and the newline.
pub const fn longest_row(path_len: usize) -> usize {
    let name = if path_len + 1 > TYPE_COLUMN {
        path_len + 1
    } else {
        TYPE_COLUMN
    };
    // "partition" is the longer type, a size and a use are twenty digits,
    // a priority has a sign, and each number has a tab before it and one
    // more while it is narrow.
    name + 9 + 21 + 1 + 21 + 1 + 13
}

/// The bytes the header takes.
pub const fn header_len() -> usize {
    HEADER.len()
}
