use alloc::vec::Vec;

use ferrix_lz4::{Table, bound, compress, decompress};

use crate::Entry;

/// The page size the pool takes and returns.
pub const PAGE: usize = 4096;

/// A page that compresses to more than half stays in RAM: the pool would
/// hold nearly a page for the work of a copy (docs/MEMORY-COMPRESSION.md).
const MOST_STORED: usize = PAGE / 2;

/// Why a page was not taken; in every case it stays where it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// Not exactly one page.
    BadSize,
    /// It compresses too poorly (or not at all).
    Poor,
    /// The pool is at its byte budget.
    Full,
    /// The allocator had no room for the block or the slot.
    NoMemory,
}

/// Why a page did not come back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadError {
    /// No such slot (a disk entry, a freed slot, an index out of range).
    Missing,
    /// The output is not exactly one page; the slot is kept.
    BadSize,
    /// The block did not decode to one page. The slot is freed; the caller
    /// kills the owning job, never the kernel.
    Corrupt,
}

/// What the pool has done, for `memory.stat` and `/proc/meminfo`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Pages held now.
    pub pages: usize,
    /// Compressed bytes held now.
    pub bytes: usize,
    /// Pages ever taken.
    pub stored: u64,
    /// Pages ever returned.
    pub loaded: u64,
    /// Pages refused for compressing poorly.
    pub poor: u64,
    /// Pages refused because the pool was full.
    pub full: u64,
}

/// The compressed pool: pages kept in RAM as LZ4 blocks, up to a byte
/// budget (25% of RAM by default, set by the caller).
// upgrade path: one preallocated arena instead of a `Vec` per block, so the
// store path allocates nothing at all; today a failed allocation is
// `Refused::NoMemory` and the page simply stays.
#[derive(Debug)]
pub struct Pool {
    slots: Vec<Option<Vec<u8>>>,
    free: Vec<u32>,
    budget: usize,
    used: usize,
    held: usize,
    table: Table,
    scratch: Vec<u8>,
    stats: Stats,
}

impl Pool {
    /// A pool of at most `budget` compressed bytes. The compressor's
    /// scratch is allocated here, once.
    pub fn new(budget: usize) -> Result<Self, Refused> {
        let mut scratch = Vec::new();
        scratch
            .try_reserve_exact(bound(PAGE))
            .map_err(|_| Refused::NoMemory)?;
        scratch.resize(bound(PAGE), 0);
        Ok(Self {
            slots: Vec::new(),
            free: Vec::new(),
            budget,
            used: 0,
            held: 0,
            table: Table::new(),
            scratch,
            stats: Stats::default(),
        })
    }

    /// Compressed bytes held.
    #[must_use]
    pub const fn used(&self) -> usize {
        self.used
    }

    /// The counters.
    #[must_use]
    pub fn stats(&self) -> Stats {
        Stats {
            pages: self.held,
            bytes: self.used,
            ..self.stats
        }
    }

    /// Take a page. Everything that can fail is reserved before anything is
    /// changed, so a refusal leaves the pool exactly as it was.
    pub fn store(&mut self, page: &[u8]) -> Result<Entry, Refused> {
        if page.len() != PAGE {
            return Err(Refused::BadSize);
        }
        let Some(len) = compress(page, &mut self.scratch, &mut self.table) else {
            self.stats.poor += 1;
            return Err(Refused::Poor);
        };
        if len > MOST_STORED {
            self.stats.poor += 1;
            return Err(Refused::Poor);
        }
        if self
            .used
            .checked_add(len)
            .is_none_or(|sum| sum > self.budget)
        {
            self.stats.full += 1;
            return Err(Refused::Full);
        }
        let Some(packed) = self.scratch.get(..len) else {
            return Err(Refused::Poor);
        };
        let mut block = Vec::new();
        block
            .try_reserve_exact(len)
            .map_err(|_| Refused::NoMemory)?;
        block.extend_from_slice(packed);
        let id = match self.free.pop() {
            Some(id) => id,
            None => {
                self.slots.try_reserve(1).map_err(|_| Refused::NoMemory)?;
                let id = u32::try_from(self.slots.len()).map_err(|_| Refused::NoMemory)?;
                self.slots.push(None);
                id
            }
        };
        let Some(slot) = usize::try_from(id)
            .ok()
            .and_then(|at| self.slots.get_mut(at))
        else {
            return Err(Refused::NoMemory);
        };
        *slot = Some(block);
        self.used += len;
        self.held += 1;
        self.stats.stored += 1;
        Ok(Entry::Pool(id))
    }

    /// Give a page back into `out` and free its slot.
    pub fn load(&mut self, entry: Entry, out: &mut [u8]) -> Result<(), LoadError> {
        let Entry::Pool(id) = entry else {
            return Err(LoadError::Missing);
        };
        if out.len() != PAGE {
            return Err(LoadError::BadSize);
        }
        let block = self.take(id).ok_or(LoadError::Missing)?;
        match decompress(&block, out) {
            Ok(PAGE) => {
                self.stats.loaded += 1;
                Ok(())
            }
            _ => Err(LoadError::Corrupt),
        }
    }

    /// Drop a page without reading it (its VMO died). Freeing what is not
    /// there does nothing.
    pub fn free(&mut self, entry: Entry) {
        if let Entry::Pool(id) = entry {
            let _ = self.take(id);
        }
    }

    /// Remove the block of slot `id` and return its accounting.
    fn take(&mut self, id: u32) -> Option<Vec<u8>> {
        let slot = self.slots.get_mut(usize::try_from(id).ok()?)?;
        let block = slot.take()?;
        self.used -= block.len();
        self.held -= 1;
        // If the free list cannot grow the id is simply not reused: the slot
        // stays empty, a few bytes lost, and nothing else is affected.
        if self.free.try_reserve(1).is_ok() {
            self.free.push(id);
        }
        Some(block)
    }

    /// Test only: make a stored block undecodable.
    #[cfg(test)]
    pub(crate) fn damage(&mut self, entry: Entry) {
        if let Entry::Pool(id) = entry
            && let Some(Some(block)) = self
                .slots
                .get_mut(usize::try_from(id).unwrap_or(usize::MAX))
        {
            block.fill(0xFF);
        }
    }
}
