use super::{Error, Table, bound, compress, decompress};

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

/// A deterministic stream of bytes no compressor shrinks much.
fn noise(len: usize) -> [u8; 4096] {
    let mut out = [0u8; 4096];
    let mut state = 0x2545_f491_u32;
    for byte in out.iter_mut().take(len) {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        *byte = state.to_le_bytes()[0];
    }
    out
}

/// What goes in comes out, for a page's worth of anything.
fn round_trip(input: &[u8]) -> usize {
    let mut table = Table::new();
    let mut packed = [0u8; bound(4096)];
    let size = compress(input, &mut packed, &mut table).expect("bound is enough");
    let mut out = [0u8; 4096];
    let back = decompress(packed.get(..size).expect("within"), &mut out);
    assert_eq!(back, Ok(input.len()));
    assert_eq!(out.get(..input.len()), Some(input));
    size
}

#[test]
fn a_page_of_zeros_shrinks_to_almost_nothing() {
    assert!(round_trip(&[0u8; 4096]) < 64);
}

#[test]
fn text_round_trips_smaller() {
    let mut page = [0u8; 4096];
    for (byte, from) in page.iter_mut().zip(b"the quick brown fox, ".iter().cycle()) {
        *byte = *from;
    }
    assert!(round_trip(&page) < 400);
}

#[test]
fn noise_round_trips_within_bound() {
    let page = noise(4096);
    assert!(round_trip(&page) <= bound(4096));
}

#[test]
fn every_short_length_round_trips() {
    let page = noise(4096);
    for len in 0..300 {
        let mut mixed = [0u8; 300];
        for (at, byte) in mixed.iter_mut().enumerate().take(len) {
            *byte = if at % 7 < 3 {
                b'z'
            } else {
                *page.get(at).expect("within")
            };
        }
        let _ = round_trip(mixed.get(..len).expect("within"));
    }
}

#[test]
fn a_full_output_is_none_not_a_panic() {
    let mut table = Table::new();
    let mut small = [0u8; 100];
    assert_eq!(compress(&noise(4096), &mut small, &mut table), None);
}

#[test]
fn input_past_64_kib_is_refused() {
    let mut table = Table::new();
    let big = [0u8; 65_536];
    let mut out = [0u8; 1024];
    assert_eq!(compress(&big, &mut out, &mut table), None);
}

/// Measured, not asserted: the ratio on a page of each kind. Run with
/// `cargo test -p xaqu-lz4 --release -- --ignored --nocapture ratio`.
#[test]
#[ignore = "a measurement for docs/MEMORY-COMPRESSION.md"]
#[expect(clippy::print_stdout, reason = "the measurement is its output")]
fn ratio() {
    extern crate std;
    let mut text = [0u8; 4096];
    for (byte, from) in text
        .iter_mut()
        .zip(b"fn main() { let x = 1; } // ".iter().cycle())
    {
        *byte = *from;
    }
    for (name, page) in [
        ("zeros", [0u8; 4096]),
        ("text", text),
        ("noise", noise(4096)),
    ] {
        std::println!("{name}: 4096 -> {}", round_trip(&page));
    }
}
