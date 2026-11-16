use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::Entry;

/// One page of an anonymous VMO: resident in a frame `F`, or out at an entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Slot<F> {
    /// The page is in RAM.
    Ram(F),
    /// The page is out; a fault restores it from the entry.
    Swapped(Entry),
}

/// The sparse page list of one anonymous VMO, generic over the frame so the
/// transitions are tested without the frame allocator. An index with no slot
/// is "never touched, reads as zero"; a swapped page is a slot, so the fault
/// path can never confuse the two and hand back zeroes for data.
// upgrade path: the kernel's `Pages` already keeps a `BTreeMap<u64, Frame>`;
// wiring is making its value this `Slot<Frame>` and nothing else.
#[derive(Debug)]
pub struct Anon<F> {
    pages: BTreeMap<u64, Slot<F>>,
    resident: usize,
    swapped: usize,
}

impl<F> Anon<F> {
    /// An empty list.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pages: BTreeMap::new(),
            resident: 0,
            swapped: 0,
        }
    }

    /// The slot at `index`, if the page was ever committed.
    #[must_use]
    pub fn get(&self, index: u64) -> Option<&Slot<F>> {
        self.pages.get(&index)
    }

    /// Commit `frame` at `index`; the frame comes back if the index is taken
    /// (resident or swapped), so nothing is overwritten or leaked.
    pub fn insert(&mut self, index: u64, frame: F) -> Result<(), F> {
        if self.pages.contains_key(&index) {
            return Err(frame);
        }
        let _ = self.pages.insert(index, Slot::Ram(frame));
        self.resident += 1;
        Ok(())
    }

    /// Replace a resident page by `entry`; the frame to release. If the page
    /// is absent or already out, `entry` comes back for the caller to free.
    pub fn swap_out(&mut self, index: u64, entry: Entry) -> Result<F, Entry> {
        match self.pages.get_mut(&index) {
            Some(slot @ Slot::Ram(_)) => {
                let Slot::Ram(frame) = core::mem::replace(slot, Slot::Swapped(entry)) else {
                    return Err(entry);
                };
                self.resident -= 1;
                self.swapped += 1;
                Ok(frame)
            }
            _ => Err(entry),
        }
    }

    /// Restore a swapped page into `frame`; the entry to free. If the page is
    /// resident or absent, `frame` comes back.
    pub fn swap_in(&mut self, index: u64, frame: F) -> Result<Entry, F> {
        // `Entry` is `Copy`: read it first, so a refusal still owns `frame`.
        let Some(&Slot::Swapped(entry)) = self.pages.get(&index) else {
            return Err(frame);
        };
        let _ = self.pages.insert(index, Slot::Ram(frame));
        self.swapped -= 1;
        self.resident += 1;
        Ok(entry)
    }

    /// Pages in RAM.
    #[must_use]
    pub const fn resident(&self) -> usize {
        self.resident
    }

    /// Pages out.
    #[must_use]
    pub const fn swapped(&self) -> usize {
        self.swapped
    }

    /// End of life: every frame to release and every entry to free, so a
    /// dying VMO leaks neither RAM nor a pool or disk slot.
    #[must_use]
    pub fn retire(self) -> (Vec<F>, Vec<Entry>) {
        let mut frames = Vec::new();
        let mut entries = Vec::new();
        for slot in self.pages.into_values() {
            match slot {
                Slot::Ram(frame) => frames.push(frame),
                Slot::Swapped(entry) => entries.push(entry),
            }
        }
        (frames, entries)
    }
}

impl<F> Default for Anon<F> {
    fn default() -> Self {
        Self::new()
    }
}
