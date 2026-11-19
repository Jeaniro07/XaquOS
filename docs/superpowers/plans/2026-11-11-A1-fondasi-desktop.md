# A1: Fondasi desktop gaya macOS - Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Boot Ferrix selesai secepat LordOS (perbaikan `lode-pstore`), tombol jendela bulat merah/kuning/hijau di kiri, tema terang kaca, wallpaper PNG ringan, dan skrip ukur yang sama untuk Ferrix dan LordOS.

**Architecture:** Empat perubahan kecil yang masing-masing lolos uji sendiri: satu baris unit init, opsi baru di `hyprix/src/titlebar.rs`, isi baru `themes/yoru.conf` beserta wallpaper dan penilai layar `test-yoru`, dan dua skrip Python di luar repo. Dok, menu bar, dan komponen lain tidak disentuh di rencana ini (A2 dan seterusnya).

**Tech Stack:** Rust 2024 (workspace Ferrix, lint ketat: tanpa `unwrap`, `expect`, `panic`, indexing langsung), hyprlang config, Python 3 stdlib, QEMU KVM di WSL Arch.

**Spec:** `C:\Users\jeani\Downloads\OS\docs\superpowers\specs\2026-11-11-desktop-gaya-macos-design.md` §3, §8 butir 7, §12.1, §12.2.

---

## Aturan eksekusi

- Semua kerja di clone WSL `~/ferrix` (Arch, `wsl.exe -d archlinux`), komponen apps di `~/ferrix/src/user/apps`. Folder Windows `Downloads\OS\FERRIX` tidak diubah.
- **Branch dan commit hanya setelah pengguna menyetujui** (aturan awal: jangan membuat branch atau commit tanpa izin). Langkah "Commit" di bawah dijalankan setelah izin itu ada. Nama branch usulan: `desktop/a1-fondasi` di `ferrix` dan di `apps`.
- Perintah `cargo xtask` x86_64 butuh `FERRIX_OVMF_CODE=/usr/share/edk2/x64/OVMF_CODE.4m.fd` dan `FERRIX_OVMF_VARS=/usr/share/edk2/x64/OVMF_VARS.4m.fd`.

## Peta berkas

| Berkas | Repo | Tanggung jawab |
|---|---|---|
| `src/user/system/linux/init/units/lode-pstore.service` | ferrix | tidak lagi menahan `multi-user.target` |
| `src/lib/init/svc/src/tests/manager.rs` | ferrix | uji: oneshot tanpa default dependency tidak menahan target |
| `src/user/system/linux/compositor/hyprix/src/titlebar.rs` | ferrix | `button_side = left`, `col.close/minimize/maximize` |
| `src/user/system/linux/compositor/hyprix/themes/yoru.conf` | ferrix | tema terang kaca |
| `docs/brand/yoru/make-light-glass.py`, `docs/brand/yoru/yoru-light-glass.png` | ferrix | wallpaper gradien PNG |
| `tools/common/xtask/src/compositor/theme.rs` | ferrix | membawa PNG ke image |
| `tools/common/xtask/src/compositor/yoru.rs` | ferrix | penilai layar `test-yoru` untuk tema terang |
| `src/user/apps/yoru-shell/src/walls.rs` | apps | wallpaper bawaan PNG |
| `Downloads\OS\tools\measure\ferrix_desktop.py`, `lordos.py` | di luar repo | metode ukur §12.1 |

---

### Task 1: `lode-pstore` tidak lagi menahan boot

Penyebab yang sudah terbukti dari kode: `multi-user.target` otomatis berada *setelah* semua unit yang di-`Wants` bila unit itu punya default dependency (`src/lib/init/svc/src/manager/graph.rs:143-178`, `order_target`). `lode-pstore.service` adalah `Type=oneshot` yang menunggu disk sampai 120 s (`src/user/system/native/lode/src/pstore.rs:201`, `WAIT_MS`). Pada image tanpa ESP yang terbaca, target menunggu 2 menit. `DefaultDependencies=no` memutus urutan implisit itu.

**Files:**
- Test: `src/lib/init/svc/src/tests/manager.rs` (tambah di akhir berkas)
- Modify: `src/user/system/linux/init/units/lode-pstore.service`

- [ ] **Step 1: Tulis uji (karakterisasi + kontrol negatif)**

Tambahkan di akhir `src/lib/init/svc/src/tests/manager.rs`:

```rust
/// A slow one-shot the boot wants holds `multi-user.target` until it ends,
/// unless it says `DefaultDependencies=no`: then the target is not after
/// it (`lode-pstore.service`, which waits up to 2 min for a disk).
#[test]
fn a_oneshot_without_default_dependencies_does_not_hold_the_target() {
    let unit = |defaults: &str| {
        alloc::format!(
            "[Unit]\n{defaults}After=sysinit.target\n[Service]\nType=oneshot\nExecStart=/bin/slow\n"
        )
    };
    for (defaults, held) in [("", true), ("DefaultDependencies=no\n", false)] {
        let text = unit(defaults);
        let mut rig = Rig::new(&[
            ("slow.service", text.as_str()),
            ("multi-user.target.wants/slow.service", "->slow.service"),
        ]);
        let _ = rig.boot();
        let _ = rig.spawned("slow.service");
        assert_eq!(
            rig.state("multi-user.target") == ActiveState::Active,
            !held,
            "DefaultDependencies {defaults:?}: the target is held while slow runs: {held}"
        );
    }
}
```

- [ ] **Step 2: Jalankan uji**

Run: `cd ~/ferrix && cargo test -p ferrix-svc a_oneshot_without_default_dependencies -- --nocapture`
Expected: PASS. Uji ini mengunci perilaku manager yang sudah ada; baris `("", true)` adalah kontrol negatifnya (tanpa `DefaultDependencies=no` target memang tertahan). Bila baris kontrol gagal, berhenti: anggapan penyebab salah, laporkan ke pengguna.

- [ ] **Step 3: Ubah unit**

Ganti seluruh isi `src/user/system/linux/init/units/lode-pstore.service` dengan:

```ini
# lode pstore-fetch: the previous boot's end, for lode to import
# (docs/LOGGING.md §5). Root's, as a raw block device opens only for root:
# it finds the ESP, reads the loader's FERRIX/LASTKERNEL.TXT off it read
# only, and copies it to /run/log/lastkernel.txt, lode's alone (0600),
# waiting up to 2 min for the disk's driver; lode, which is uid 91 by then,
# imports from the copy and never opens a disk. Not started unless an image
# enables it (tools/common/xtask/src/lode.rs).
#
# DefaultDependencies=no: the wait must not hold multi-user.target and the
# desktop's graphical.target behind it (a target is after what it wants
# only when both have default dependencies, svc's graph.rs order_target).
# It still runs after lode, which is after basic.target.
[Unit]
Description=Copy the loader's last kernel log for lode
DefaultDependencies=no
After=lode.service

[Service]
Type=oneshot
ExecStart=/sbin/lode pstore-fetch
TimeoutStartSec=180
```

- [ ] **Step 4: Uji boot dan log**

Run: `cd ~/ferrix && cargo xtask test-boot --arch x86_64` lalu `cargo xtask test-log-pstore --arch x86_64` (dengan env OVMF).
Expected: keduanya PASS (`FERRIX-BOOT-OK`, `test-log-pstore` tetap mengimpor `LASTKERNEL.TXT`).

- [ ] **Step 5: Ukur ulang** (setelah Task 5 ada)

Run: `python3 /mnt/c/Users/jeani/Downloads/OS/tools/measure/ferrix_desktop.py`
Expected: baris `graphical.target` jauh di bawah 121 s; target spec: sama atau lebih cepat dari LordOS 12,57 s.

- [ ] **Step 6: Commit** (setelah izin pengguna)

```bash
cd ~/ferrix
git add src/user/system/linux/init/units/lode-pstore.service src/lib/init/svc/src/tests/manager.rs
git commit -m "init: lode-pstore no longer holds multi-user.target for its disk wait"
```

---

### Task 2: Tombol bulat di kiri (`button_side = left`)

**Files:**
- Modify: `src/user/system/linux/compositor/hyprix/src/titlebar.rs` (Style 33-178, `hit_among` 354-388, `draw` 460-554, `button` 656-715, tests)

- [ ] **Step 1: Tulis uji yang gagal**

Tambahkan di modul `tests` `titlebar.rs`, setelah `a_press_finds_the_button_under_it`:

```rust
    /// With `button_side = left` the buttons stand from the bar's left edge
    /// as macOS has them: close, minimize, maximize; the right end is bar.
    #[test]
    fn buttons_on_the_left_are_close_minimize_maximize() {
        let mut bars = Bars::default();
        bars.style.buttons_left = true;
        bars.placed
            .push((WindowId(7), Rect::new(100, 100, 400, super::HEIGHT)));
        let middle = 100.0 + 19.0;
        assert_eq!(
            bars.hit((100.0 + 21.0, middle)),
            Some((WindowId(7), Hit::Close))
        );
        assert_eq!(
            bars.hit((100.0 + 54.0, middle)),
            Some((WindowId(7), Hit::Minimize))
        );
        assert_eq!(
            bars.hit((100.0 + 87.0, middle)),
            Some((WindowId(7), Hit::Maximize))
        );
        assert_eq!(
            bars.hit((500.0 - 21.0, middle)),
            Some((WindowId(7), Hit::Bar)),
            "nothing at the right end"
        );
    }
```

- [ ] **Step 2: Jalankan, pastikan gagal**

Run: `cd ~/ferrix/src/user/system/linux/compositor && cargo test -p hyprix buttons_on_the_left`
Expected: FAIL kompilasi, `no field buttons_left on type Style`.

- [ ] **Step 3: Tambah field di `Style`**

Di `pub struct Style`, setelah field `title_left`:

```rust
    /// `button_side`: `right` (the default), or `left`, where the buttons
    /// stand from the bar's left edge as close, minimize, maximize.
    pub buttons_left: bool,
    /// `col.close`, `col.minimize`, `col.maximize`: a button's own disc,
    /// drawn without a glyph; transparent (the default) for `col.button`
    /// with the glyph.
    pub close: Rgba,
    /// The same, for minimize.
    pub minimize: Rgba,
    /// The same, for maximize.
    pub maximize: Rgba,
```

Di `impl Default for Style`, setelah `title_left: false,`:

```rust
            buttons_left: false,
            close: CLEAR,
            minimize: CLEAR,
            maximize: CLEAR,
```

Di atas `pub struct Style`, tambahkan:

```rust
/// No colour: a theme that sets none keeps the default look.
const CLEAR: Rgba = Rgba {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};
```

Di `Style::from_config`, setelah `title_left: ...,`:

```rust
            buttons_left: config.str("hyprix:titlebar:button_side") == Some("left"),
            close: colour("col.close", CLEAR),
            minimize: colour("col.minimize", CLEAR),
            maximize: colour("col.maximize", CLEAR),
```

Di `impl Style`, ganti komentar dan tambahkan dua fungsi setelah `button_from_right`:

```rust
    /// The buttons in the order they stand from the edge they are on.
    const fn order(&self) -> [Hit; 3] {
        if self.buttons_left {
            [Hit::Close, Hit::Minimize, Hit::Maximize]
        } else {
            [Hit::Close, Hit::Maximize, Hit::Minimize]
        }
    }

    /// How far the buttons reach in from their edge, margin included.
    fn buttons_span(&self) -> f32 {
        self.button_margin + 3.0 * self.button_size + 2.0 * self.button_gap
    }

    /// A button's own disc, when the theme gives one.
    fn own_disc(&self, which: Hit) -> Option<Rgba> {
        let colour = match which {
            Hit::Close => self.close,
            Hit::Minimize => self.minimize,
            Hit::Maximize => self.maximize,
            Hit::Bar => return None,
        };
        (colour.a != 0).then_some(colour)
    }
```

Ubah doc `button_from_right` menjadi: `/// The centre of the \`index\`th button of [`Style::order`], counted from the edge the buttons are on.` (nama fungsi tetap.)

- [ ] **Step 4: Ubah `hit_among`**

Ganti blok dari `let from_right = right - x;` sampai akhir loop `for` dengan:

```rust
            let middle = (top + bottom) / 2.0;
            let half = f64::from(self.style.button_size) / 2.0;
            for (index, hit) in self.style.order().into_iter().enumerate() {
                let offset = f64::from(self.style.button_from_right(index));
                let centre = if self.style.buttons_left {
                    left + offset
                } else {
                    right - offset
                };
                if (x - centre).abs() <= half + 1.0 && (y - middle).abs() <= half + 2.0 {
                    return Some((*window, hit));
                }
            }
```

- [ ] **Step 5: Ubah `draw` dan `button`**

Di `draw`, ganti blok `// The buttons, right to left ...` sampai sebelum `// The title, ...` dengan:

```rust
        // The buttons, from their edge: close first.
        let ink = if focused {
            style.text
        } else {
            style.text_inactive
        };
        let span = style.buttons_span() * s;
        for (index, which) in style.order().into_iter().enumerate() {
            let offset = style.button_from_right(index) * s;
            let cx = if style.buttons_left { offset } else { full - offset };
            let (disc, glyph) = style
                .own_disc(which)
                .map_or((style.disc, Some(ink)), |own| (own, None));
            button(
                &mut pixmap,
                (cx, middle),
                style.button_size * s / 2.0,
                which,
                (disc, glyph),
                s,
            );
        }
```

Di blok judul, ganti perhitungan `room` dan `x`:

```rust
            let room = if style.title_left {
                full - span - 2.0 * margin
            } else {
                (full - span - 2.0 * margin).max(0.0) - span - margin
            };
```

```rust
            let x = if style.title_left {
                if style.buttons_left { span + margin } else { margin }
            } else {
                ((full - run.width) / 2.0).max(margin)
            };
```

Hapus variabel `right_edge` yang tidak terpakai lagi.

Di `fn button`, ubah parameter `(disc_colour, ink): (Rgba, Rgba)` menjadi `(disc_colour, ink): (Rgba, Option<Rgba>)`, perbarui doc menjadi `/// One button: its disc, and its glyph in \`ink\` when there is one.`, lalu tepat sebelum `let mut pen = Paint::default();` tambahkan:

```rust
    let Some(ink) = ink else {
        return;
    };
```

- [ ] **Step 6: Jalankan uji**

Run: `cd ~/ferrix/src/user/system/linux/compositor && cargo test -p hyprix titlebar && cargo clippy -p hyprix --all-targets -- -D warnings`
Expected: semua uji `titlebar` PASS, termasuk `a_press_finds_the_button_under_it` (sisi kanan tidak berubah) dan `a_bar_is_drawn_at_its_size`; clippy bersih.

- [ ] **Step 7: Commit** (setelah izin pengguna)

```bash
cd ~/ferrix
git add src/user/system/linux/compositor/hyprix/src/titlebar.rs
git commit -m "hyprix: title bar buttons on the left, each with its own disc"
```

---

### Task 3: Wallpaper gradien PNG

**Files:**
- Create: `docs/brand/yoru/make-light-glass.py`
- Create (hasil skrip): `docs/brand/yoru/yoru-light-glass.png`

- [ ] **Step 1: Tulis skrip**

```python
"""Yoru's light-glass wallpaper: a 1920x1080 diagonal gradient, blue to lilac
to sand, Ferrix's own. Python's standard library only, so the PNG can be made
again anywhere: python3 make-light-glass.py yoru-light-glass.png"""
import struct, sys, zlib

W, H = 1920, 1080
STOPS = [(0.0, (0xA9, 0xC6, 0xF5)), (0.55, (0xE8, 0xD5, 0xF2)), (1.0, (0xF6, 0xE3, 0xC9))]


def colour(t):
    for (t0, c0), (t1, c1) in zip(STOPS, STOPS[1:]):
        if t <= t1:
            f = (t - t0) / (t1 - t0)
            return tuple(round(a + (b - a) * f) for a, b in zip(c0, c1))
    return STOPS[-1][1]


def chunk(kind, data):
    body = kind + data
    return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))


def main(path):
    rows = bytearray()
    span = 0.94 * W + 0.34 * H  # the 160-degree axis, as the mock-up drew it
    for y in range(H):
        raw = bytearray()
        for x in range(W):
            raw += bytes(colour((0.94 * x + 0.34 * y) / span))
        rows.append(1)  # PNG filter "Sub": a gradient's neighbours differ by 0 or 1
        rows += bytes(raw[i] if i < 3 else (raw[i] - raw[i - 3]) & 0xFF for i in range(len(raw)))
    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(rows), 9))
    png += chunk(b"IEND", b"")
    with open(path, "wb") as out:
        out.write(png)


if __name__ == "__main__":
    main(sys.argv[1])
```

- [ ] **Step 2: Hasilkan dan periksa**

Run: `cd ~/ferrix/docs/brand/yoru && python3 make-light-glass.py yoru-light-glass.png && file yoru-light-glass.png && ls -l yoru-light-glass.png`
Expected: `PNG image data, 1920 x 1080, 8-bit/color RGB`; ukuran di bawah 2 MB. Bila lebih, laporkan angkanya sebelum lanjut.

- [ ] **Step 3: Commit** (setelah izin pengguna)

```bash
cd ~/ferrix
git add docs/brand/yoru/make-light-glass.py docs/brand/yoru/yoru-light-glass.png
git commit -m "brand: Yoru light-glass wallpaper, a PNG gradient made by a stdlib script"
```

---

### Task 4: Tema terang kaca dan penilai `test-yoru`

**Files:**
- Modify: `src/user/system/linux/compositor/hyprix/themes/yoru.conf` (seluruhnya)
- Modify: `tools/common/xtask/src/compositor/theme.rs:26`, `:186`, `:249`
- Modify: `tools/common/xtask/src/compositor/yoru.rs:200-320` (`judge` dan tests)
- Modify: `src/user/apps/yoru-shell/src/walls.rs:25` (repo apps)

Catatan: dok, sidebar, dan launcher yoru-shell masih berwarna Nocturne gelap sampai A2. Penilai dok di `judge` tidak diubah. Tema GTK dan portald juga masih gelap sampai A3 (Control Center).

- [ ] **Step 1: Ubah uji penilai dulu (gagal dengan tema lama)**

Di `tools/common/xtask/src/compositor/yoru.rs`, ganti modul `tests` dengan:

```rust
#[cfg(test)]
mod tests {
    use super::{SCREEN, judge};
    use crate::display::Image;

    /// The wallpaper's corner colour, sand.
    const CORNER: [u8; 3] = [0xf6, 0xe3, 0xc9];

    /// A screen of the wallpaper alone is not Yoru's desktop: no bars.
    #[test]
    fn the_wallpaper_alone_has_no_bars() {
        let screen = Image {
            width: SCREEN.0,
            height: SCREEN.1,
            pixels: CORNER.repeat(SCREEN.0 * SCREEN.1),
        };
        let said = judge(&screen).expect_err("no bars");
        assert!(said.contains("surface"), "{said}");
    }

    /// A light bar with a red close button, the wallpaper's blue near the
    /// top, and the dock are.
    #[test]
    fn a_light_bar_with_its_close_button_is() {
        let mut pixels = CORNER.repeat(SCREEN.0 * SCREEN.1);
        let mut paint = |xs: core::ops::Range<usize>, ys: core::ops::Range<usize>, colour: [u8; 3]| {
            for y in ys {
                for x in xs.clone() {
                    let at = (y * SCREEN.0 + x) * 3;
                    pixels[at..at + 3].copy_from_slice(&colour);
                }
            }
        };
        paint(100..700, 300..330, [0xf3, 0xf3, 0xf5]);
        paint(110..120, 310..320, [0xff, 0x5f, 0x57]);
        paint(900..901, 100..101, [0xc6, 0xcf, 0xf4]);
        paint(SCREEN.0 / 2..SCREEN.0 / 2 + 1, SCREEN.1 - 36..SCREEN.1 - 35, [0x20, 0x22, 0x30]);
        paint(80..110, SCREEN.1 - 50..SCREEN.1 - 48, [0x96, 0x8a, 0xe0]);
        let screen = Image {
            width: SCREEN.0,
            height: SCREEN.1,
            pixels,
        };
        assert_eq!(judge(&screen).map(|_| ()), Ok(()));
    }
}
```

Run: `cd ~/ferrix && cargo test -p xtask compositor::yoru`
Expected: FAIL (`judge` lama menolak sudut terang: "not Nocturne's ground").

- [ ] **Step 2: Ganti `judge`**

Ganti fungsi `judge` (doc dan badan) dengan:

```rust
/// Whether `screen` is Yoru's light-glass desktop: the wallpaper's sand in
/// the lower right corner, which no window reaches; title bars on the light
/// surface, `#f3f3f5`; a red close button; the wallpaper's blue near the
/// top; and the dock, still Nocturne's until the dock's own change.
fn judge(screen: &Image) -> core::result::Result<String, String> {
    if (screen.width, screen.height) != SCREEN {
        return Err(format!("a {}x{} screen", screen.width, screen.height));
    }
    let pixel = |x: usize, y: usize| -> [i32; 3] {
        let at = (y * screen.width + x) * 3;
        match screen.pixels.get(at..at + 3) {
            Some(&[r, g, b]) => [i32::from(r), i32::from(g), i32::from(b)],
            _ => [0, 0, 0],
        }
    };
    let [r, g, b] = pixel(screen.width - 3, screen.height - 3);
    if !(r >= 0xe8 && b < r && b >= 0xb8) {
        return Err(format!("the corner is {r},{g},{b}, not the wallpaper's sand"));
    }
    let near = |[r, g, b]: [i32; 3], [r0, g0, b0]: [i32; 3], by: i32| {
        (r - r0).abs() <= by && (g - g0).abs() <= by && (b - b0).abs() <= by
    };
    let mut surface = 0usize;
    let mut close = 0usize;
    for y in 0..screen.height {
        for x in 0..screen.width {
            let seen = pixel(x, y);
            if near(seen, [0xf3, 0xf3, 0xf5], 2) {
                surface += 1;
            }
            if near(seen, [0xff, 0x5f, 0x57], 24) {
                close += 1;
            }
        }
    }
    // A bar's worth at least 200 pixels wide and 30 high; a close button.
    if surface < 200 * 30 {
        return Err(format!("{surface} pixels of the bars' surface"));
    }
    if close < 40 {
        return Err(format!("{close} pixels of a close button"));
    }
    let [r, g, b] = pixel(900, 100);
    if !(b >= 0xe0 && b > r + 0x10) {
        return Err(format!("no wallpaper's blue at 900,100: {r},{g},{b}"));
    }
    let row = screen.height - 6 - 30;
    let [r, g, b] = pixel(screen.width / 2, row);
    if !(r < 0x30 && g < 0x30 && b < 0x40) {
        return Err(format!("the dock's middle is {r},{g},{b}, not its bar"));
    }
    let lit = (0..200).any(|x| {
        (screen.height - 66..screen.height - 6).any(|y| near(pixel(x, y), [0x96, 0x8a, 0xe0], 20))
    });
    if !lit {
        return Err("no workspace lit in the dock".to_owned());
    }
    Ok(format!(
        "the light-glass wallpaper, {surface} pixels of title bar, {close} of close buttons, the dock"
    ))
}
```

Run: `cd ~/ferrix && cargo test -p xtask compositor::yoru`
Expected: PASS kedua uji.

- [ ] **Step 3: Bawa PNG ke image**

Di `tools/common/xtask/src/compositor/theme.rs`:
- baris 26: `pub(super) const YORU_WALLPAPER: &str = "usr/share/backgrounds/yoru/yoru-light-glass.png";`
- baris 186: `root.join("docs/brand/yoru/yoru-light-glass.png"),`
- baris 249: `"usr/share/backgrounds/yoru/yoru-light-glass.png",`

Di repo apps, `src/user/apps/yoru-shell/src/walls.rs:25`:

```rust
pub const DEFAULT: &str = "/usr/share/backgrounds/yoru/yoru-light-glass.png";
```

Run: `cd ~/ferrix && cargo test -p xtask compositor::theme` dan `cd ~/ferrix/src/user/apps/yoru-shell && cargo test`
Expected: PASS. Bila uji walls menyebut berkas `.svg` secara harfiah, ganti nama berkas di uji itu saja dan catat di pesan commit.

- [ ] **Step 4: Tulis tema baru**

Ganti seluruh isi `src/user/system/linux/compositor/hyprix/themes/yoru.conf`:

```ini
# Yoru, light glass: hyprix drawn as the macOS-style desktop of
# docs/superpowers/specs/2026-11-11-desktop-gaya-macos-design.md, Ferrix's
# own colours (no Apple asset or name). `source = <this file>` from a
# hyprland.conf, before the lines that should win over it, or as the whole
# configuration.
#
# Tokens, as hex:
#   ground  #e8e8ed   surface #ffffff   bar #f3f3f5   text #1d1d1f
#   accent  #0a84ff   close #ff5f57   minimize #febc2e   maximize #28c840

general {
    layout = floating
    gaps_in = 0
    gaps_out = 0
    float_gaps = 0
    border_size = 1
    # A hairline around every window, a little darker on the focused one.
    col.active_border = rgba(00000026)
    col.inactive_border = rgba(00000014)
    resize_on_border = true
    extend_border_grab_area = 8
    snap {
        enabled = true
        window_gap = 14
        monitor_gap = 14
    }
}

hyprix {
    blur_in_software = 0
    # Where there is no blur, the shell's layers get the light ground at
    # 85% behind them.
    blur_scrim = rgba(f5f5f7d9)
    own_frames = true
    titlebars = 1
    # 30 px, the title centred in Inter 600, three 12 px discs at the left:
    # close, minimize, maximize, no glyphs.
    titlebar {
        height = 30
        rounding = 10
        col.active = rgba(f3f3f5ff)
        col.inactive = rgba(f6f6f8ff)
        col.text = rgba(1d1d1fff)
        col.text_inactive = rgba(9a9aa0ff)
        col.button = rgba(00000000)
        col.close = rgba(ff5f57ff)
        col.minimize = rgba(febc2eff)
        col.maximize = rgba(28c840ff)
        col.separator = rgba(0000001a)
        button_side = left
        button_size = 12
        button_gap = 8
        button_margin = 12
        title_align = center
        font_family = Inter Variable
        font_size = 13
        font_weight = 600
    }
}

decoration {
    rounding = 10
    active_opacity = 1.0
    inactive_opacity = 1.0
    dim_inactive = false
    shadow {
        enabled = true
        range = 30
        render_power = 2
        offset = 0 12
        color = rgba(00000038)
        color_inactive = rgba(0000001f)
    }
    glow {
        enabled = false
    }
    # The glass: what shows through the dock, the bar and the panels.
    blur {
        enabled = true
        size = 8
        passes = 2
        noise = 0.0117
        contrast = 1.0
        brightness = 1.05
        vibrancy = 0.2
        popups = true
    }
}

animations {
    enabled = true
    bezier = yoru, 0.2, 0.8, 0.2, 1.0
    animation = windows, 1, 2, yoru, popin 96%
    animation = windowsMove, 1, 2, yoru
    animation = workspaces, 1, 3, yoru, slidefade 4%
    animation = fade, 1, 2, yoru
    animation = layers, 1, 3, yoru, fade
}

misc {
    disable_hyprland_logo = true
    disable_splash_rendering = true
    background_color = rgb(e8e8ed)
}

layerrule = blur true, match:namespace ^(yoru-.*)$
layerrule = ignore_alpha 0.05, match:namespace ^(yoru-.*)$
layerrule = blur true, match:namespace ^(launcher)$

# The wallpaper: Ferrix's own gradient, a PNG (docs/brand/yoru/
# make-light-glass.py). A person's picture is a `wallpaper =` line after
# this file, or Settings' Wallpaper page.
wallpaper = , picture /usr/share/backgrounds/yoru/yoru-light-glass.png, fit stretch, colour rgb(e8e8ed)

exec-once = notifyd

env = GTK_USE_PORTAL,1
env = GDK_DEBUG,portals
exec-once = portald

# The keys are in yoru-keys.conf beside this file.
```

- [ ] **Step 5: Uji boot desktop**

Run: `cd ~/ferrix && cargo xtask test-yoru --arch x86_64` (env OVMF).
Expected: PASS dengan pesan `the light-glass wallpaper, N pixels of title bar, M of close buttons, the dock`. Bila nama subcommand berbeda, cari dengan `cargo xtask --help | grep -i yoru` (fungsi `test_yoru` di `compositor/yoru.rs:33`).

- [ ] **Step 6: Tangkap layar untuk pengguna**

Run: `cargo xtask run-compositor --arch x86_64 --vnc :0 --no-gl --smp 4 --memory 4096 --accel kvm`, sambungkan VNC, simpan tangkapan layar ke `C:\Users\jeani\Downloads\OS\docs\superpowers\shots\a1-light-glass.png`, kirim ke pengguna untuk dinilai.

- [ ] **Step 7: Commit** (setelah izin pengguna)

```bash
cd ~/ferrix
git add src/user/system/linux/compositor/hyprix/themes/yoru.conf tools/common/xtask/src/compositor/theme.rs tools/common/xtask/src/compositor/yoru.rs
git commit -m "yoru: light-glass theme, buttons on the left, the PNG wallpaper"
cd ~/ferrix/src/user/apps
git add yoru-shell/src/walls.rs
git commit -m "yoru-shell: the light-glass wallpaper by default"
```

---

### Task 5: Skrip ukur yang sama untuk Ferrix dan LordOS (§12.1)

Di luar repo: `C:\Users\jeani\Downloads\OS\tools\measure\`. Dasarnya skrip baseline 2026-11-11 di scratchpad, disamakan ke 4 GB dan 4 core, dan ditambah waktu `graphical.target`.

**Files:**
- Create: `C:\Users\jeani\Downloads\OS\tools\measure\ferrix_desktop.py`
- Create: `C:\Users\jeani\Downloads\OS\tools\measure\lordos.py` (salin `...\scratchpad\lordos-iso\measure.py`, lalu ubah `-m` ke `4096` dan `-smp` ke `4` bila berbeda)

- [ ] **Step 1: Tulis `ferrix_desktop.py`**

```python
"""Ferrix's desktop under KVM in WSL, measured as spec §12.1 says: QEMU start
-> dock drawn, -> graphical.target active; after 65 s idle, MemTotal -
MemAvailable and each process's RSS, read from the serial console.
Run inside WSL: python3 ferrix_desktop.py [~/ferrix]"""
import os, re, subprocess, sys, threading, time

root = os.path.expanduser(sys.argv[1] if len(sys.argv) > 1 else "~/ferrix")
env = dict(os.environ, FERRIX_OVMF_CODE="/usr/share/edk2/x64/OVMF_CODE.4m.fd",
           FERRIX_OVMF_VARS="/usr/share/edk2/x64/OVMF_VARS.4m.fd")
cmd = ["cargo", "xtask", "run-compositor", "--arch", "x86_64", "--vnc", ":0", "--no-gl",
       "--smp", "4", "--memory", "4096", "--accel", "kvm"]
log = open(os.path.expanduser("~/ferrix-measure.log"), "w")
p = subprocess.Popen(cmd, cwd=root, env=env, stdin=subprocess.PIPE,
                     stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
lines = []


def pump():
    for raw in iter(p.stdout.readline, b""):
        s = raw.decode("utf-8", "replace").rstrip("\n")
        lines.append((time.time(), s))
        log.write(f"{time.time():.2f} {s}\n")
        log.flush()


threading.Thread(target=pump, daemon=True).start()


def wait_for(pattern, timeout, after=0.0):
    end = time.time() + timeout
    while time.time() < end:
        for t, s in list(lines):
            if t >= after and re.search(pattern, s):
                return t
        if p.poll() is not None:
            return None
        time.sleep(0.2)
    return None


def send(line):
    p.stdin.write((line + "\n").encode())
    p.stdin.flush()


def since(t, t0):
    return "not seen" if t is None or t0 is None else f"{t - t0:.2f} s"


t0 = wait_for(r"booting \(quit with", 1800)
dock = wait_for(r"the dock is up", 300, t0 or 0)
graphical = wait_for(r"graphical\.target.*(active|reached)", 300, t0 or 0)
print(f"QEMU start -> dock drawn: {since(dock, t0)}", flush=True)
print(f"QEMU start -> graphical.target: {since(graphical, t0)}", flush=True)
time.sleep(65)
mark = len(lines)
send("echo ===MEASURE")
send("while read k v u; do case $k in MemTotal:|MemAvailable:) echo $k $v $u;; esac; done < /proc/meminfo")
send("for f in /proc/[0-9]*/status; do n=; while read k v r; do case $k in Name:) n=$v;; VmRSS:) echo RSS $v $n;; esac; done < $f; done")
send("echo ===END")
wait_for(r"^===END", 60, lines[mark - 1][0] if mark else 0)
time.sleep(3)
mem = {}
for _, s in lines[mark:]:
    m = re.match(r"(MemTotal|MemAvailable): (\d+)", s)
    if m:
        mem[m.group(1)] = int(m.group(2))
    elif s.startswith("RSS "):
        print(s)
if len(mem) == 2:
    print(f"used (MemTotal - MemAvailable): {(mem['MemTotal'] - mem['MemAvailable']) / 1024:.0f} MiB")
p.kill()
```

- [ ] **Step 2: Jalankan baseline baru (sebelum Task 1 dan 4 di-build)**

Run (WSL): `python3 /mnt/c/Users/jeani/Downloads/OS/tools/measure/ferrix_desktop.py`
Expected: tiga angka tercetak; `graphical.target` sekitar 121 s pada kode lama. Bila pola `graphical.target` tidak pernah cocok, cari baris init yang sebenarnya di `~/ferrix-measure.log` (`grep -n graphical ~/ferrix-measure.log`) dan sesuaikan regex; jangan mengarang angka.

- [ ] **Step 3: Jalankan LordOS dengan 4 GB / 4 core**

Run (PowerShell, QEMU WHPX seperti baseline): `python "C:\Users\jeani\Downloads\OS\tools\measure\lordos.py"`
Expected: `systemd-analyze` dan `free`. Catat bahwa LordOS diukur di WHPX dan Ferrix di KVM WSL; titik (a) "dok tergambar" untuk LordOS belum terukur (butuh deteksi layar) dan ditulis "belum diukur" di laporan.

- [ ] **Step 4: Ukur setelah Task 1-4**

Run: `ferrix_desktop.py` lagi.
Expected (batas spec §12.2): dok 8 s ke bawah; `graphical.target` sama atau lebih cepat dari LordOS; RAM 200 MiB ke bawah; proses `wallpaper` di bawah 20 MB. Laporkan tabel sebelum/sesudah ke pengguna. Bila wallpaper masih di atas 20 MB, laporkan; perbaikannya masuk A2.

---

## Self-review

- Spec §8 butir 7 (penyebab 121 s): Task 1, penyebab dibuktikan dari kode, bukan dugaan.
- Spec §3 titlebar dan tema: Task 2 dan 4. Wallpaper PNG: Task 3 dan 4.
- Spec §12.1 dan §12.2: Task 5.
- Tidak dicakup di A1 (masuk rencana berikutnya, lihat `2026-11-11-peta-rencana.md`): dok tengah, MenuBar, `ferrix_appmenu_v1`, Spotlight, Control, Launchpad, Notices, Mission Control, Hemat, tema GTK terang, uji `multiapp`.
