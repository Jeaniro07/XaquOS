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
