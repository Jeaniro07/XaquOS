use super::{Error, decompress};

/// A hand-made block: one literal `a`, a match of 7 at offset 1 (the
/// overlapping copy), then the last literal `b`.
#[test]
fn a_hand_made_block_decodes_with_an_overlapping_match() {
    let block = [0x13, b'a', 0x01, 0x00, 0x10, b'b'];
    let mut out = [0u8; 16];
    assert_eq!(decompress(&block, &mut out), Ok(9));
    assert_eq!(out.get(..9), Some(&b"aaaaaaaab"[..]));
}

/// Literals alone, and the empty block (one token of nothing).
#[test]
fn literals_alone_decode() {
    let mut out = [0u8; 8];
    assert_eq!(decompress(&[0x30, b'x', b'y', b'z'], &mut out), Ok(3));
    assert_eq!(out.get(..3), Some(&b"xyz"[..]));
    assert_eq!(decompress(&[0x00], &mut out), Ok(0));
}

/// Every bad block is an error, never a panic or a read outside.
#[test]
fn bad_blocks_are_refused() {
    let mut out = [0u8; 16];
    assert_eq!(decompress(&[], &mut out), Err(Error::Truncated));
    assert_eq!(decompress(&[0x20, b'a'], &mut out), Err(Error::Truncated));
    assert_eq!(
        decompress(&[0x10, b'a', 0x00, 0x00, 0x00], &mut out),
        Err(Error::Corrupt),
        "offset 0"
    );
    assert_eq!(
        decompress(&[0x10, b'a', 0x02, 0x00, 0x00], &mut out),
        Err(Error::Corrupt),
        "an offset before the start"
    );
    assert_eq!(
        decompress(&[0x1f, b'a', 0x01, 0x00, 0xff, 0xff, 0x10, 0x00], &mut out),
        Err(Error::OutputFull)
    );
    assert_eq!(
        decompress(&[0x10, b'a', 0x01, 0x00], &mut out),
        Err(Error::Truncated),
        "a block must end with literals"
    );
}
