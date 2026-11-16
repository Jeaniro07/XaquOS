use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// What the ager knows of one page.
#[derive(Clone, Copy, Debug)]
struct Meta {
    /// Scans since the hardware last saw the page used; saturates.
    age: u8,
    /// Never a victim: GPU and DMA pages, mlock, kernel memory, ring-3
    /// drivers and devmgr, pid 1, writable shared mappings. The caller that
    /// knows says so; the ager only honours it.
    pinned: bool,
}

/// Page aging for the swap choice (docs/MEMORY-COMPRESSION.md): a periodic
/// scan reads and clears each page's accessed bit and calls [`Ager::scan`]
/// with it; a page that stays unused grows old and becomes a victim. `K`
/// names a page (a VMO and an index, in the kernel).
// upgrade path: a generation list per cgroup instead of a map scanned for
// victims, once `victims` shows up in a profile.
#[derive(Debug)]
pub struct Ager<K> {
    pages: BTreeMap<K, Meta>,
}

impl<K: Ord + Copy> Ager<K> {
    /// No page tracked.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pages: BTreeMap::new(),
        }
    }

    /// Start tracking a resident page, young and not pinned.
    pub fn track(&mut self, key: K) {
        let _ = self.pages.insert(
            key,
            Meta {
                age: 0,
                pinned: false,
            },
        );
    }

    /// Stop tracking: the page left RAM or its VMO died.
    pub fn forget(&mut self, key: K) {
        let _ = self.pages.remove(&key);
    }

    /// Mark a page never-swap (or lift the mark). No-op for an untracked page.
    pub fn pin(&mut self, key: K, pinned: bool) {
        if let Some(meta) = self.pages.get_mut(&key) {
            meta.pinned = pinned;
        }
    }

    /// One scan step: the accessed bit read for `key`. The new age, or `None`
    /// for a page that is not tracked.
    pub fn scan(&mut self, key: K, accessed: bool) -> Option<u8> {
        let meta = self.pages.get_mut(&key)?;
        meta.age = if accessed {
            0
        } else {
            meta.age.saturating_add(1)
        };
        Some(meta.age)
    }

    /// Up to `limit` unpinned pages at least `min_age` old, oldest first and
    /// by key among equals.
    #[must_use]
    pub fn victims(&self, min_age: u8, limit: usize) -> Vec<K> {
        let mut old: Vec<(u8, K)> = self
            .pages
            .iter()
            .filter(|(_, meta)| !meta.pinned && meta.age >= min_age)
            .map(|(key, meta)| (meta.age, *key))
            .collect();
        old.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        old.into_iter().take(limit).map(|(_, key)| key).collect()
    }
}

impl<K: Ord + Copy> Default for Ager<K> {
    fn default() -> Self {
        Self::new()
    }
}
