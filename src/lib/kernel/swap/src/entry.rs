/// Where a page that left RAM went: what a VMO keeps in its list in place of
/// the frame (docs/MEMORY-COMPRESSION.md, "swap entries").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    /// A slot of the compressed pool, in RAM.
    Pool(u32),
    /// A slot of the swap area on disk.
    // upgrade path: more than one area means `Disk { area, slot }`.
    Disk(u32),
}
