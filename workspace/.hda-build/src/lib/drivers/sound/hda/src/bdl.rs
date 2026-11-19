//! The buffer descriptor list a stream plays from.
//!
//! A stream does not read the buffer itself: it reads a list of
//! descriptors, each naming a piece of the buffer, how much of it there is,
//! and whether the stream interrupts when it has played that piece.
//!
//! A descriptor is sixteen bytes: the address of the piece in the first
//! eight, its length in the next four, and the flags in the last four. The
//! first flag is the interrupt on completion.

/// The bytes one descriptor takes.
pub const ENTRY_BYTES: usize = 16;

/// One descriptor of a buffer descriptor list.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Entry {
    /// Where the piece of the buffer starts.
    pub address: u64,
    /// How many bytes it has.
    pub bytes: u32,
    /// Whether the stream interrupts when it has played this piece.
    pub interrupt: bool,
}

impl Entry {
    /// The descriptor's bytes, as the controller's memory holds them.
    #[must_use]
    pub fn encode(&self) -> [u8; ENTRY_BYTES] {
        let mut out = [0_u8; ENTRY_BYTES];
        let address = self.address.to_le_bytes();
        if let Some(slot) = out.get_mut(..address.len()) {
            slot.copy_from_slice(&address);
        }
        let length = self.bytes.to_le_bytes();
        if let Some(slot) = out.get_mut(8..12) {
            slot.copy_from_slice(&length);
        }
        let flags = u32::from(self.interrupt).to_le_bytes();
        if let Some(slot) = out.get_mut(12..16) {
            slot.copy_from_slice(&flags);
        }
        out
    }

    /// The descriptor the bytes carry.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let head = bytes.get(..ENTRY_BYTES)?;
        let address = u64::from_le_bytes(head.get(..8)?.try_into().ok()?);
        let length = u32::from_le_bytes(head.get(8..12)?.try_into().ok()?);
        let flags = u32::from_le_bytes(head.get(12..16)?.try_into().ok()?);
        Some(Self {
            address,
            bytes: length,
            interrupt: flags & 1 != 0,
        })
    }
}

/// The bytes a list of descriptors takes.
#[must_use]
pub const fn list_bytes(entries: usize) -> usize {
    entries * ENTRY_BYTES
}
