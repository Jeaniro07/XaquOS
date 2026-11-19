# B0: Desain kompresi memori dan swap, plus kompresor LZ4 - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Menyiapkan dokumen desain Proyek B untuk review konsultan sertifikasi Ferrix, dan membangun crate `ferrix-lz4` (`no_std`, tanpa `unsafe`, tanpa alokasi) yang teruji host, fuzz, dan Miri.

**Architecture:** Kernel tidak disentuh di rencana ini. Jalur fault, entri swap, dan swap disk (B1, B2, B4, B5) hanya boleh dikodekan setelah konsultan menyetujui desain, sesuai aturan Ferrix untuk core ring (`docs/APP-SUSPEND.md` §3.2, "The certified item"). Kompresor adalah fungsi murni atas byte, jadi ia tinggal di `src/lib/kernel/lz4` tempat `cargo test`, Miri, dan fuzzer bisa menjangkaunya.

**Tech Stack:** Rust 2024 `no_std`, `#![forbid(unsafe_code)]`, workspace lints Ferrix (tanpa `unwrap`, `expect`, `panic`, indexing langsung), libFuzzer (`src/tests/fuzz`), Miri.

**Spec:** `C:\Users\jeani\Downloads\OS\docs\superpowers\specs\2026-11-11-desktop-gaya-macos-design.md` §9.

---

## Aturan eksekusi

- Kerja di clone WSL `~/ferrix`. Branch dan commit hanya setelah izin pengguna; nama usulan `mm/b0-lz4`.
- Kompresor ditulis dari spesifikasi format blok LZ4 (`lz4_Block_format.md` milik proyek LZ4), tanpa menyalin kode implementasi lain (spec §13).

## Peta berkas

| Berkas | Tanggung jawab |
|---|---|
| `docs/MEMORY-COMPRESSION.md` | desain B1-B5 untuk konsultan |
| `src/lib/kernel/lz4/Cargo.toml` | crate `ferrix-lz4` |
| `src/lib/kernel/lz4/src/lib.rs` | `compress`, `decompress`, `Table`, `Error`, `bound` |
| `src/lib/kernel/lz4/src/tests.rs` | uji host |
| `Cargo.toml` (root) | anggota workspace dan dependency |
| `src/tests/fuzz/fuzz_targets/lz4_roundtrip.rs`, `src/tests/fuzz/Cargo.toml` | fuzz |

---

### Task 1: Dokumen desain untuk konsultan

**Files:**
- Create: `docs/MEMORY-COMPRESSION.md`

- [ ] **Step 1: Tulis dokumen**

```markdown
# Compressed memory and swap

**Design, 2026-11-11. Nothing in it is built but the compressor
(`src/lib/kernel/lz4`).** For the certification consultant's review before
any change to `mm`. The customer wants many applications open at once with
their memory held as macOS holds it: compressed in RAM first, the coldest
on disk behind that.

## 1. Where it stands

* No swap (APP-SUSPEND.md §5). Reclaim takes clean page cache only
  (CGROUPS.md §10); anonymous memory never leaves RAM.
* Reclaim's hand is a clock with no active and inactive lists.
* The kernel has decompressors (btrfs: zlib, LZO, zstd) and no compressor.

## 2. Design

A page leaving RAM is replaced, in its VMO's page list and in every
mapping, by a swap entry naming where it went: the compressed pool or a
slot of the swap area. A fault on the entry brings it back.

1. **B1 entries.** Anonymous pages out and in through the fault path, kept
   in RAM uncompressed at first (the proof of concept of APP-SUSPEND §3.2).
2. **B2 ageing.** Accessed bits and active/inactive lists, so reclaim takes
   cold pages.
3. **B3 pool.** `ferrix-lz4` compresses a page into a pool capped at 25% of
   RAM; a page compressing worse than 50% goes on to B4 or stays. Prefetch
   of a job's pages when its application comes back to the front.
4. **B4 disk.** A preallocated, no-COW swap file on `/data`, encrypted with
   a per-boot key that is never stored; the write path allocates nothing,
   its buffers made when swap is switched on, so a page-out never waits on
   memory the ring-3 disk driver needs.
5. **B5 interfaces.** `memory.reclaim` for anonymous memory,
   `memory.swap.current` and `memory.swap.max`, `memory.stat`'s `zswap` and
   `swapcached`, `/proc/meminfo`'s `SwapTotal` and `SwapFree`, `/proc/swaps`.

Never leaving RAM: pinned pages (GPU objects, DMA, `mlock`), the kernel's
memory, ring-3 drivers' and devmgr's memory, pid 1's, and pages of shared
mappings written through a shared VMO (first version).

## 3. Fail closed

* Compression fails or the pool is full: the page stays.
* A disk write fails: the page stays, the area is marked bad, disk swap off.
* A disk read or decryption fails: the owning job is killed, never the kernel.
* Disk swap is off by default until B4 passes on the N150; the pool is on.

## 4. What the consultant is asked

1. Requirement ids to reserve (`requirement-reservations.json`) for B1-B5.
2. The residual-information argument for the per-boot key.
3. The `unsafe` of the I/O path, and where the fault path may sleep on I/O.
4. Negative controls: a corrupted entry stops the boot on its check's own
   message; a pinned page never leaves; an injected read error kills only
   the job.
5. Whether B1's in-RAM store may land before B2-B4, behind a build flag.

## 5. Points (backlog units, a guess)

B0 3, B1 13, B2 5, B3 8, B4 13, B5 3: about 45, sized again after review.
```

- [ ] **Step 2: Serahkan ke pengguna**

Kirim berkas ke pengguna dan minta pengguna meneruskannya ke konsultan sertifikasi Ferrix sesuai proses proyek. Rencana B1 dan seterusnya ditulis setelah jawaban konsultan ada. Jangan mengklaim review sudah terjadi.

- [ ] **Step 3: Commit** (setelah izin pengguna)

```bash
cd ~/ferrix
git add docs/MEMORY-COMPRESSION.md
git commit -m "docs: compressed memory and swap, a design for review"
```

---

### Task 2: Crate `ferrix-lz4`, dekompresor dulu

**Files:**
- Create: `src/lib/kernel/lz4/Cargo.toml`, `src/lib/kernel/lz4/src/lib.rs`, `src/lib/kernel/lz4/src/tests.rs`
- Modify: `Cargo.toml` (root): `members` (di dekat `"src/lib/kernel/kmem"`) dan `[workspace.dependencies]` (di dekat `ferrix-sched`)

- [ ] **Step 1: Manifest dan registrasi**

`src/lib/kernel/lz4/Cargo.toml`:

```toml
[package]
name = "ferrix-lz4"
description = "LZ4 block compression and decompression for compressed memory (docs/MEMORY-COMPRESSION.md): no_std, no unsafe, no allocation"
version = "0.1.0"
edition = "2024"
rust-version = "1.99"
license.workspace = true
publish = false

[dependencies]

[lints]
workspace = true
```

Root `Cargo.toml`: tambahkan `"src/lib/kernel/lz4",` ke `members`, dan `ferrix-lz4 = { path = "src/lib/kernel/lz4" }` ke `[workspace.dependencies]`.

- [ ] **Step 2: Tulis uji dekompresor yang gagal**

`src/lib/kernel/lz4/src/tests.rs`:

```rust
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
```

`src/lib/kernel/lz4/src/lib.rs` (kerangka agar uji terkompilasi dan gagal):

```rust
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
pub fn decompress(_input: &[u8], _out: &mut [u8]) -> Result<usize, Error> {
    Err(Error::Corrupt)
}
```

Run: `cd ~/ferrix && cargo test -p ferrix-lz4`
Expected: FAIL pada tiga uji (dekompresor kerangka selalu `Err(Corrupt)`).

- [ ] **Step 3: Dekompresor**

Ganti fungsi `decompress` di `lib.rs` dengan:

```rust
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
```

Run: `cd ~/ferrix && cargo test -p ferrix-lz4`
Expected: tiga uji dekompresor PASS.

---

### Task 3: Kompresor

**Files:**
- Modify: `src/lib/kernel/lz4/src/lib.rs`, `src/lib/kernel/lz4/src/tests.rs`

- [ ] **Step 1: Tulis uji kompresor yang gagal**

Ganti baris import di atas `tests.rs` menjadi `use super::{Error, Table, bound, compress, decompress};`, lalu tambahkan:

```rust
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
            *byte = if at % 7 < 3 { b'z' } else { *page.get(at).expect("within") };
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
```

Catatan lint: `expect` dilarang di kode produksi oleh workspace; bila clippy menolaknya juga di uji, tambahkan di atas `tests.rs` baris `#![expect(clippy::expect_used, reason = "a test's failure is its message")]`, sebagaimana crate lain Ferrix menanganinya (cek dulu dengan `grep -rn "expect_used" src/lib/crypto/sha256/src/tests.rs src/lib/fs/common/cpio/src`).

Run: `cd ~/ferrix && cargo test -p ferrix-lz4`
Expected: FAIL kompilasi (`Table`, `bound`, `compress` belum ada).

- [ ] **Step 2: Implementasi kompresor**

Tambahkan ke `lib.rs`, setelah `MIN_MATCH`:

```rust
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
```

Alasan desain singkat (ditulis sebagai komentar di atas `compress` bila konsultan memintanya): hash satu entri per slot, tanpa rantai; cukup untuk halaman 4 KiB. Upgrade path: rantai hash atau zstd untuk halaman dingin yang menuju disk (B4).

- [ ] **Step 3: Jalankan uji dan lint**

Run: `cd ~/ferrix && cargo test -p ferrix-lz4 && cargo clippy -p ferrix-lz4 --all-targets -- -D warnings`
Expected: sembilan uji PASS; clippy bersih. Bila clippy menolak `as usize` di `hash`, ganti dengan `usize::try_from(...).unwrap_or(0)` dan jalankan ulang.

- [ ] **Step 4: Miri**

Run: `cd ~/ferrix && cargo miri test -p ferrix-lz4`
Expected: PASS tanpa laporan UB. Bila Miri belum terpasang, `rustup component add miri` (toolchain dari `rust-toolchain.toml`).

- [ ] **Step 5: Commit** (setelah izin pengguna)

```bash
cd ~/ferrix
git add Cargo.toml Cargo.lock src/lib/kernel/lz4
git commit -m "lz4: a no_std block compressor and decompressor for compressed memory"
```

---

### Task 4: Fuzz

**Files:**
- Create: `src/tests/fuzz/fuzz_targets/lz4_roundtrip.rs`
- Modify: `src/tests/fuzz/Cargo.toml` (dependency dan `[[bin]]`)

- [ ] **Step 1: Target fuzz**

```rust
//! Fuzz the LZ4 block codec compressed memory keeps pages in.
//!
//! # The properties
//!
//! 1. **Any block decodes or is refused**, never a panic, into any output.
//! 2. **A round trip**: every input up to 64 KiB, compressed with `bound`'s
//!    room, decompresses to itself.
//! 3. **A short output is `None`**, never a panic.

#![no_main]

use ferrix_lz4::{Table, bound, compress, decompress};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    let mut out = vec![0u8; 65_536];
    let _ = decompress(bytes, &mut out);

    let input = &bytes[..bytes.len().min(ferrix_lz4::MAX_INPUT)];
    let mut table = Table::new();
    let mut packed = vec![0u8; bound(input.len())];
    let size = compress(input, &mut packed, &mut table).expect("bound is enough");
    let mut back = vec![0u8; input.len()];
    assert_eq!(decompress(&packed[..size], &mut back), Ok(input.len()));
    assert_eq!(back, input);

    let mut small = vec![0u8; input.len() / 4];
    let _ = compress(input, &mut small, &mut table);
});
```

Di `src/tests/fuzz/Cargo.toml`: tambahkan `ferrix-lz4 = { path = "../../lib/kernel/lz4" }` ke `[dependencies]`, dan:

```toml
[[bin]]
name = "lz4_roundtrip"
path = "fuzz_targets/lz4_roundtrip.rs"
test = false
doc = false
bench = false
```

- [ ] **Step 2: Jalankan 10 menit**

Run: `cd ~/ferrix/src/tests/fuzz && cargo +nightly fuzz run lz4_roundtrip -- -max_total_time=600`
Expected: selesai tanpa crash. Bila crash, simpan input di `artifacts/` dan kembali ke Task 3 (systematic-debugging), jangan melonggarkan properti.

- [ ] **Step 3: Commit** (setelah izin pengguna)

```bash
cd ~/ferrix
git add src/tests/fuzz/Cargo.toml src/tests/fuzz/Cargo.lock src/tests/fuzz/fuzz_targets/lz4_roundtrip.rs
git commit -m "fuzz: lz4 round trip and any block refused without a panic"
```

---

### Task 5: Ukur rasio dan kecepatan (bahan B3 dan target §12.3)

- [ ] **Step 1: Uji ukur yang diabaikan secara bawaan**

Tambahkan ke `tests.rs`:

```rust
/// Measured, not asserted: the ratio on a page of each kind. Run with
/// `cargo test -p ferrix-lz4 --release -- --ignored --nocapture ratio`.
#[test]
#[ignore = "a measurement for docs/MEMORY-COMPRESSION.md"]
fn ratio() {
    extern crate std;
    let mut text = [0u8; 4096];
    for (byte, from) in text.iter_mut().zip(b"fn main() { let x = 1; } // ".iter().cycle()) {
        *byte = *from;
    }
    for (name, page) in [("zeros", [0u8; 4096]), ("text", text), ("noise", noise(4096))] {
        std::println!("{name}: 4096 -> {}", round_trip(&page));
    }
}
```

Run: `cd ~/ferrix && cargo test -p ferrix-lz4 --release -- --ignored --nocapture ratio`
Expected: tiga baris angka. Tulis angkanya ke §5 `docs/MEMORY-COMPRESSION.md` sebagai "diukur di host, belum di halaman app nyata". Rasio pada memori app nyata diukur di B3, bukan di sini.

---

## Self-review

- Spec §9.2-§9.5 tercakup sebagai desain (Task 1). §9.3 B3 kompresor: Task 2-5. §11 "Host: LZ4 bolak-balik, fuzz, Miri": Task 3-4.
- B1, B2, B4, B5 sengaja tidak ada kodenya: menunggu review konsultan (spec §9.1, §9.5). Rencananya ditulis setelah itu.
