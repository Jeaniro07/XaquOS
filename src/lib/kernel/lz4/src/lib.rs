//! LZ4's block format (the LZ4 project's `lz4_Block_format.md`), written
//! from the format: what compressed memory (docs/MEMORY-COMPRESSION.md)
//! keeps a page in. `no_std`, no `unsafe`, no allocation: the caller owns
//! the output and the compressor's [`Table`].

#![no_std]
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

/// Why a block did not decode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The block ended inside a sequence.
    Truncated,
    /// A match points before the output's start, at offset 0, or a length
    /// overflows.
    Corrupt,
    /// The output is too small for what the block holds.
    OutputFull,
}

/// Decode `input` into `out`; the number of bytes written.
pub fn decompress(input: &[u8], out: &mut [u8]) -> Result<usize, Error> {
    let mut at = 0usize;
    let mut written = 0usize;
    loop {
        let token = *input.get(at).ok_or(Error::Truncated)?;
        at += 1;
        let mut literals = usize::from(token >> 4);
        if literals == 15 {
            literals = literals
                .checked_add(length(input, &mut at)?)
                .ok_or(Error::Corrupt)?;
        }
        let from = input
            .get(at..at.checked_add(literals).ok_or(Error::Corrupt)?)
            .ok_or(Error::Truncated)?;
        let end = written.checked_add(literals).ok_or(Error::Corrupt)?;
        out.get_mut(written..end)
            .ok_or(Error::OutputFull)?
            .copy_from_slice(from);
        at += literals;
        written = end;
        if at == input.len() {
            return Ok(written);
        }
        let offset = match input.get(at..at + 2) {
            Some(&[low, high]) => usize::from(u16::from_le_bytes([low, high])),
            _ => return Err(Error::Truncated),
        };
        at += 2;
        if offset == 0 || offset > written {
            return Err(Error::Corrupt);
        }
        let mut matched = usize::from(token & 0x0f);
        if matched == 15 {
            matched = matched
                .checked_add(length(input, &mut at)?)
                .ok_or(Error::Corrupt)?;
        }
        let end = written
            .checked_add(matched)
            .and_then(|n| n.checked_add(MIN_MATCH))
            .ok_or(Error::Corrupt)?;
        if end > out.len() {
            return Err(Error::OutputFull);
        }
        // Byte by byte: a match may overlap what it is copying.
        for to in written..end {
            let byte = *out.get(to - offset).ok_or(Error::Corrupt)?;
            *out.get_mut(to).ok_or(Error::OutputFull)? = byte;
        }
        written = end;
    }
}

/// A length's extra bytes: 255 each until one is less.
fn length(input: &[u8], at: &mut usize) -> Result<usize, Error> {
    let mut total = 0usize;
    loop {
        let byte = *input.get(*at).ok_or(Error::Truncated)?;
        *at += 1;
        total = total.checked_add(usize::from(byte)).ok_or(Error::Corrupt)?;
        if byte != 255 {
            return Ok(total);
        }
    }
}

/// The shortest match the format has.
const MIN_MATCH: usize = 4;

/// Bits of a position's hash.
const HASH_BITS: u32 = 12;

/// The input `compress` takes at most: positions fit a `u16`.
pub const MAX_INPUT: usize = 65_535;

/// The format's rule: the last match starts at least 12 bytes before the
/// end, and the last 5 bytes are literals.
const MATCH_LIMIT: usize = 12;
const LAST_LITERALS: usize = 5;

/// The compressor's hash table, which the caller owns (8 KiB), so that
/// `compress` puts nothing large on a kernel stack.
#[derive(Debug)]
pub struct Table {
    slots: [u16; 1 << HASH_BITS],
}

impl Table {
    /// An empty table.
    #[must_use]
    pub const fn new() -> Self {
        Table {
            slots: [0; 1 << HASH_BITS],
        }
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

/// The largest output `compress` can write for `len` input bytes.
#[must_use]
pub const fn bound(len: usize) -> usize {
    len + len / 255 + 16
}

/// Compress `input` into `out`; the bytes written, or `None` when `out` is
/// too small or `input` is longer than [`MAX_INPUT`]. A caller keeping a
/// page keeps it uncompressed on `None`.
///
/// One hash entry per slot, no chains: enough for a 4 KiB page. Upgrade
/// path: hash chains, or zstd for cold pages bound for disk (B4).
pub fn compress(input: &[u8], out: &mut [u8], table: &mut Table) -> Option<usize> {
    if input.len() > MAX_INPUT {
        return None;
    }
    table.slots.fill(0);
    let mut writer = Writer { out, at: 0 };
    let mut anchor = 0usize;
    let mut at = 0usize;
    let limit = input.len().saturating_sub(MATCH_LIMIT);
    let match_end_limit = input.len().saturating_sub(LAST_LITERALS);
    while at < limit {
        let seen = read4(input, at)?;
        let slot = table.slots.get_mut(hash(seen))?;
        let candidate = usize::from(*slot);
        *slot = u16::try_from(at).ok()?;
        if candidate < at && read4(input, candidate) == Some(seen) {
            let mut end = at + MIN_MATCH;
            let mut from = candidate + MIN_MATCH;
            while end < match_end_limit && input.get(end) == input.get(from) {
                end += 1;
                from += 1;
            }
            writer.sequence(input.get(anchor..at)?, at - candidate, end - at)?;
            at = end;
            anchor = end;
        } else {
            at += 1;
        }
    }
    writer.last(input.get(anchor..)?)?;
    Some(writer.at)
}

/// Four bytes at `at`, as one number.
fn read4(input: &[u8], at: usize) -> Option<u32> {
    match input.get(at..at.checked_add(4)?) {
        Some(&[a, b, c, d]) => Some(u32::from_le_bytes([a, b, c, d])),
        _ => None,
    }
}

/// A position's slot in the table.
fn hash(seen: u32) -> usize {
    (seen.wrapping_mul(2_654_435_761) >> (32 - HASH_BITS)) as usize
}

/// The output, written forward.
struct Writer<'a> {
    out: &'a mut [u8],
    at: usize,
}

impl Writer<'_> {
    fn byte(&mut self, byte: u8) -> Option<()> {
        *self.out.get_mut(self.at)? = byte;
        self.at += 1;
        Some(())
    }

    fn bytes(&mut self, bytes: &[u8]) -> Option<()> {
        let end = self.at.checked_add(bytes.len())?;
        self.out.get_mut(self.at..end)?.copy_from_slice(bytes);
        self.at = end;
        Some(())
    }

    /// A length of 15 or more: its extra bytes after the token's 15.
    fn length(&mut self, mut rest: usize) -> Option<()> {
        while rest >= 255 {
            self.byte(255)?;
            rest -= 255;
        }
        self.byte(u8::try_from(rest).ok()?)
    }

    /// One sequence: literals, then a match `matched` long `offset` back.
    fn sequence(&mut self, literals: &[u8], offset: usize, matched: usize) -> Option<()> {
        let extra = matched.checked_sub(MIN_MATCH)?;
        self.byte((nibble(literals.len()) << 4) | nibble(extra))?;
        if literals.len() >= 15 {
            self.length(literals.len() - 15)?;
        }
        self.bytes(literals)?;
        self.bytes(&u16::try_from(offset).ok()?.to_le_bytes())?;
        if extra >= 15 {
            self.length(extra - 15)?;
        }
        Some(())
    }

    /// The last sequence: literals only.
    fn last(&mut self, literals: &[u8]) -> Option<()> {
        self.byte(nibble(literals.len()) << 4)?;
        if literals.len() >= 15 {
            self.length(literals.len() - 15)?;
        }
        self.bytes(literals)
    }
}

/// A length as a token's half: 15 means "more follows".
fn nibble(len: usize) -> u8 {
    u8::try_from(len.min(15)).unwrap_or(15)
}
