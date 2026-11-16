//! Fuzz the LZ4 block codec compressed memory keeps pages in.
//!
//! # The properties
//!
//! 1. **Any block decodes or is refused**, never a panic, into any output.
//! 2. **A round trip**: every input up to 64 KiB, compressed with `bound`'s
//!    room, decompresses to itself.
//! 3. **A short output is `None`**, never a panic.

#![no_main]

use xaqu_lz4::{Table, bound, compress, decompress};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let mut out = vec![0u8; 65_536];
    let _ = decompress(bytes, &mut out);

    let input = &bytes[..bytes.len().min(xaqu_lz4::MAX_INPUT)];
    let mut table = Table::new();
    let mut packed = vec![0u8; bound(input.len())];
    let size = compress(input, &mut packed, &mut table).expect("bound is enough");
    let mut back = vec![0u8; input.len()];
    assert_eq!(decompress(&packed[..size], &mut back), Ok(input.len()));
    assert_eq!(back, input);

    let mut small = vec![0u8; input.len() / 4];
    let _ = compress(input, &mut small, &mut table);
});
