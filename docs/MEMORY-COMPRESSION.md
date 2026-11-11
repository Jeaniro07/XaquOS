# Compressed memory and swap

**Design, 2026-11-11. Nothing in it is built but the LZ4 codec
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

The compressor's ratio, measured on the host and not yet on a real
application's pages (`cargo test -p ferrix-lz4 --release -- --ignored
--nocapture ratio`): a page of zeros 4096 -> 26 bytes; a page of one
repeated 28-byte line of source text 4096 -> 54; a page of xorshift noise
4096 -> 4114, 18 bytes over the page and within `bound`. The ratio on real
application memory is B3's to measure.
