use alloc::boxed::Box;
use alloc::vec::Vec;

use xaqu_crng::block;
use xaqu_sha256::digest;

use crate::{Entry, PAGE};

/// `ChaCha20` yields 64 bytes per block.
const BLOCK: usize = 64;

/// The block device behind a swap area: whole pages, slot by slot.
pub trait Device {
    /// Write one page into `slot`.
    fn write(&mut self, slot: u32, data: &[u8; PAGE]) -> Result<(), DeviceError>;
    /// Read the page in `slot` into `out`.
    fn read(&mut self, slot: u32, out: &mut [u8; PAGE]) -> Result<(), DeviceError>;
}

/// The device failed; it says nothing more because nothing more can be done.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceError;

/// Why swap-on did not happen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwapOnError {
    /// An area with no slots.
    Empty,
    /// A device is already attached.
    Already,
    /// The slot table or the page buffer did not allocate.
    NoMemory,
}

/// Why a page did not go to disk; in every case it stays in RAM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteError {
    /// Disk swap is off (never turned on, or turned off after a failure).
    Off,
    /// Not exactly one page.
    BadSize,
    /// Every slot is taken.
    Full,
    /// The device refused the write. The area is now marked bad and off.
    Failed,
}

/// Why a page did not come back from disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// No such slot (a pool entry, a freed slot, an index out of range).
    Missing,
    /// The output is not exactly one page; the slot is kept.
    BadSize,
    /// The device failed the read. The slot is freed; the caller kills the
    /// owning job, never the kernel.
    Io,
    /// What came back is not what was written (the digest differs). The
    /// slot is freed, nothing is handed out, the caller kills the owner.
    Corrupt,
}

/// What the RAM side remembers about a written slot. It never reaches the
/// disk, so whoever can rewrite the disk cannot also rewrite the digest.
#[derive(Clone, Copy, Debug)]
struct Meta {
    nonce: u64,
    digest: [u8; 32],
}

/// Disk swap: pages encrypted with `ChaCha20` (a fresh nonce per write) and
/// checked with SHA-256 on the way back. Off until `swapon`; a failed write
/// turns it off for good. Everything the write path needs is allocated at
/// `swapon`, because it runs when memory is already short.
// upgrade path: several areas with priorities, `Entry::Disk{area, slot}`, and
// a key that comes from the CRNG at boot instead of from the caller.
#[derive(Debug)]
pub struct Area<D> {
    device: Option<D>,
    key: [u8; 32],
    slots: Vec<Option<Meta>>,
    free: Vec<u32>,
    scratch: Option<Box<[u8; PAGE]>>,
    next_nonce: u64,
    bad: bool,
}

impl<D> Default for Area<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D> Area<D> {
    /// An area that is off.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            device: None,
            key: [0; 32],
            slots: Vec::new(),
            free: Vec::new(),
            scratch: None,
            next_nonce: 0,
            bad: false,
        }
    }

    /// Whether a page may be written now.
    #[must_use]
    pub const fn is_on(&self) -> bool {
        self.device.is_some() && !self.bad
    }

    /// Whether a write has failed (the area is off for good).
    #[must_use]
    pub const fn is_bad(&self) -> bool {
        self.bad
    }

    /// Slots in use.
    #[must_use]
    pub fn used(&self) -> usize {
        self.slots.len() - self.free.len()
    }

    /// The device, if one is attached.
    #[must_use]
    pub const fn device(&self) -> Option<&D> {
        self.device.as_ref()
    }

    /// The device, if one is attached.
    pub const fn device_mut(&mut self) -> Option<&mut D> {
        self.device.as_mut()
    }

    /// Attach `device` with `slots` page slots and turn swap on.
    pub fn swapon(&mut self, device: D, slots: u32, key: [u8; 32]) -> Result<(), SwapOnError> {
        if self.device.is_some() {
            return Err(SwapOnError::Already);
        }
        if slots == 0 {
            return Err(SwapOnError::Empty);
        }
        let count = usize::try_from(slots).map_err(|_| SwapOnError::NoMemory)?;
        let mut table = Vec::new();
        table
            .try_reserve_exact(count)
            .map_err(|_| SwapOnError::NoMemory)?;
        table.resize(count, None);
        let mut free = Vec::new();
        free.try_reserve_exact(count)
            .map_err(|_| SwapOnError::NoMemory)?;
        for slot in (0..slots).rev() {
            free.push(slot);
        }
        let mut buffer = Vec::new();
        buffer
            .try_reserve_exact(PAGE)
            .map_err(|_| SwapOnError::NoMemory)?;
        buffer.resize(PAGE, 0_u8);
        let scratch: Box<[u8; PAGE]> = buffer
            .into_boxed_slice()
            .try_into()
            .map_err(|_| SwapOnError::NoMemory)?;
        self.slots = table;
        self.free = free;
        self.scratch = Some(scratch);
        self.key = key;
        self.device = Some(device);
        Ok(())
    }

    /// Drop a page without reading it (its VMO died). Freeing what is not
    /// there does nothing.
    pub fn free(&mut self, entry: Entry) {
        if let Entry::Disk(slot) = entry {
            let _ = self.take(slot);
        }
    }

    /// Remove slot `slot` from use and return what was remembered of it.
    fn take(&mut self, slot: u32) -> Option<Meta> {
        let meta = self.slots.get_mut(usize::try_from(slot).ok()?)?.take()?;
        // Capacity for every slot was reserved at swapon: this never allocates.
        self.free.push(slot);
        Some(meta)
    }
}

impl<D: Device> Area<D> {
    /// Encrypt `page` and put it on disk. On any refusal the caller keeps
    /// the page.
    pub fn write(&mut self, page: &[u8]) -> Result<Entry, WriteError> {
        let (Some(device), Some(scratch)) = (self.device.as_mut(), self.scratch.as_mut()) else {
            return Err(WriteError::Off);
        };
        if self.bad {
            return Err(WriteError::Off);
        }
        if page.len() != PAGE {
            return Err(WriteError::BadSize);
        }
        let slot = self.free.pop().ok_or(WriteError::Full)?;
        let nonce = self.next_nonce;
        let Some(after) = nonce.checked_add(1) else {
            self.free.push(slot);
            self.bad = true;
            return Err(WriteError::Failed);
        };
        scratch.copy_from_slice(page);
        xor_stream(&self.key, nonce, scratch.as_mut_slice());
        let sum = digest(scratch.as_slice());
        if device.write(slot, scratch).is_err() {
            self.free.push(slot);
            self.bad = true;
            return Err(WriteError::Failed);
        }
        self.next_nonce = after;
        if let Some(at) = usize::try_from(slot)
            .ok()
            .and_then(|at| self.slots.get_mut(at))
        {
            *at = Some(Meta { nonce, digest: sum });
        }
        Ok(Entry::Disk(slot))
    }

    /// Give a page back into `out` and free its slot. `out` is written only
    /// when the page is whole and checked.
    pub fn read(&mut self, entry: Entry, out: &mut [u8]) -> Result<(), ReadError> {
        let Entry::Disk(slot) = entry else {
            return Err(ReadError::Missing);
        };
        if out.len() != PAGE {
            return Err(ReadError::BadSize);
        }
        let meta = self.take(slot).ok_or(ReadError::Missing)?;
        let (Some(device), Some(scratch)) = (self.device.as_mut(), self.scratch.as_mut()) else {
            return Err(ReadError::Io);
        };
        if device.read(slot, scratch).is_err() {
            return Err(ReadError::Io);
        }
        if digest(scratch.as_slice()) != meta.digest {
            return Err(ReadError::Corrupt);
        }
        out.copy_from_slice(scratch.as_slice());
        xor_stream(&self.key, meta.nonce, out);
        Ok(())
    }
}

/// XOR `buf` with the `ChaCha20` stream of (`key`, `nonce`); applying it
/// twice gives the input back.
fn xor_stream(key: &[u8; 32], nonce: u64, buf: &mut [u8]) {
    for (counter, chunk) in (0_u64..).zip(buf.chunks_mut(BLOCK)) {
        let stream = block(key, counter, nonce);
        for (byte, pad) in chunk.iter_mut().zip(stream.iter()) {
            *byte ^= *pad;
        }
    }
}
