//! The little-endian field readers and writers the protocol modules share.
//!
//! Every structure in this crate is packed on the wire the way the USB
//! specification packs it: little-endian, no padding. Rust's own layouts
//! cannot be used for that, so each structure is built from bytes and read
//! back from bytes, and these four helpers are the whole of the machinery.

/// The two bytes at `at`, little-endian.
pub(crate) fn get16(bytes: &[u8], at: usize) -> Option<u16> {
    let field = bytes.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(field.try_into().ok()?))
}

/// The four bytes at `at`, little-endian.
pub(crate) fn get32(bytes: &[u8], at: usize) -> Option<u32> {
    let field = bytes.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(field.try_into().ok()?))
}

/// The two bytes at `at`, big-endian.
///
/// The Ethernet frame header's type field is in network order, which is
/// the one place in these protocols that is not little-endian.
pub(crate) fn get16_be(bytes: &[u8], at: usize) -> Option<u16> {
    let field = bytes.get(at..at.checked_add(2)?)?;
    Some(u16::from_be_bytes(field.try_into().ok()?))
}

/// Write `value` at `at`, little-endian, if there is room for it.
pub(crate) fn put16(out: &mut [u8], at: usize, value: u16) {
    if let Some(slot) = out.get_mut(at..at.saturating_add(2)) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

/// Write `value` at `at`, little-endian, if there is room for it.
pub(crate) fn put32(out: &mut [u8], at: usize, value: u32) {
    if let Some(slot) = out.get_mut(at..at.saturating_add(4)) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}
