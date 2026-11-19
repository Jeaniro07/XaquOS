//! The verbs, and the two rings they travel on.
//!
//! A verb is four bytes that name a codec, a node in it and a parameter,
//! and carry a value. The codec's answer comes back in the RIRB, not to the
//! verb that asked it, so a driver that wants an answer has to remember
//! which question it asked and wait for the RIRB to fill.
//!
//! Both rings live in memory the controller reaches by address, not in its
//! register block: the CORB holds the verbs, four bytes each, and the RIRB
//! holds the answers, eight bytes each. The register block holds their
//! base addresses and their pointers.

/// Why a verb could not be sent, or its answer could not be read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CorbError {
    /// The CORB is full: the controller has not taken a verb yet.
    Full,
    /// The codec did not answer.
    NoAnswer,
    /// The codec answered that it will not.
    Refused,
}

/// The controller's memory, as the driver reads and writes it.
///
/// The rings, the buffer descriptor lists and the sample buffers live in
/// memory the driver makes and pins for the device; the driver reads and
/// writes them by their offset in that memory.
pub trait Memory {
    /// The little-endian `u32` at `offset`.
    fn read32(&self, offset: u32) -> u32;
    /// Write the little-endian `u32` at `offset`.
    fn write32(&mut self, offset: u32, value: u32);
}

/// A verb with a twelve-bit parameter: the codec's usual form.
///
/// The codec address takes bits 31:28, the node address bits 27:20, the
/// parameter identifier bits 19:8 and the value bits 7:0.
#[must_use]
pub fn verb12(cad: u8, nid: u8, parameter: u8, value: u8) -> u32 {
    (u32::from(cad) << 28)
        | (u32::from(nid) << 20)
        | (u32::from(parameter) << 8)
        | u32::from(value)
}

/// A verb with a four-bit parameter and a sixteen-bit value.
///
/// The codec address takes bits 31:28, the node address bits 27:20, the
/// parameter identifier bits 19:16 and the value bits 15:0.
#[must_use]
pub fn verb4(cad: u8, nid: u8, parameter: u8, value: u16) -> u32 {
    (u32::from(cad) << 28)
        | (u32::from(nid) << 20)
        | (u32::from(parameter) << 16)
        | u32::from(value)
}

/// The CORB: the ring the verbs go out on.
///
/// The controller reads the entry after the one it read last, so the first
/// verb goes into the second entry and the read pointer starts at the first.
pub struct Corb<'a, M: Memory> {
    memory: &'a mut M,
    entries: u16,
    write: u16,
    read: u16,
}

impl<M: Memory> core::fmt::Debug for Corb<'_, M> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Corb")
    }
}

impl<'a, M: Memory> Corb<'a, M> {
    /// The CORB in the memory `memory`, with `entries` entries.
    pub fn new(memory: &'a mut M, entries: u16) -> Self {
        let first = if entries > 1 { 1 } else { 0 };
        Self {
            memory,
            entries,
            write: first,
            read: 0,
        }
    }

    /// The entry the next verb goes into.
    pub fn write_pointer(&self) -> u16 {
        self.write
    }

    /// The entry the controller has taken and is working on.
    pub fn read_pointer(&self) -> u16 {
        self.read
    }

    /// Say which entry the controller has taken.
    pub fn set_read_pointer(&mut self, read: u16) {
        self.read = read;
    }

    /// Whether the CORB has room for one more verb.
    pub fn has_room(&self) -> bool {
        self.write != self.read
    }

    /// Put `verb` into the CORB, and say which entry it went into.
    ///
    /// The entry the verb went into is the one the controller reads next:
    /// the write pointer the driver gives it is that entry, and the
    /// controller reads the entry after the one it read last.
    ///
    /// # Errors
    ///
    /// [`CorbError::Full`] if the CORB has no room for it.
    pub fn push(&mut self, verb: u32) -> Result<u16, CorbError> {
        if !self.has_room() {
            return Err(CorbError::Full);
        }
        let at = self.write;
        self.memory.write32(u32::from(at) * 4, verb);
        self.write = (at + 1) % self.entries;
        Ok(at)
    }
}

/// The RIRB: the ring the answers come back on.
pub struct Rirb<'a, M: Memory> {
    memory: &'a M,
    entries: u16,
}

impl<M: Memory> core::fmt::Debug for Rirb<'_, M> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Rirb")
    }
}

impl<'a, M: Memory> Rirb<'a, M> {
    /// The RIRB in the memory `memory`, with `entries` entries.
    pub fn new(memory: &'a M, entries: u16) -> Self {
        Self {
            memory,
            entries,
        }
    }

    /// The answer in the entry `index`, if it has one.
    ///
    /// An entry is eight bytes: the answer in the lower half, and the
    /// controller's marks of it in the upper half.
    pub fn answer(&self, index: u16) -> Option<u32> {
        if index >= self.entries {
            return None;
        }
        Some(self.memory.read32(u32::from(index) * 8))
    }

    /// The controller's marks of the answer in the entry `index`.
    ///
    /// Bit 4 says the answer was unsolicited, and bits 3:0 name the codec
    /// that gave it.
    pub fn extra(&self, index: u16) -> Option<u32> {
        if index >= self.entries {
            return None;
        }
        Some(self.memory.read32(u32::from(index) * 8 + 4))
    }

    /// Whether the answer in the entry `index` was unsolicited.
    pub fn unsolicited(&self, index: u16) -> bool {
        self.extra(index).is_some_and(|extra| extra & (1 << 4) != 0)
    }

    /// The codec that gave the answer in the entry `index`.
    pub fn codec(&self, index: u16) -> Option<u8> {
        self.extra(index).map(|extra| (extra & 0x0F) as u8)
    }
}
