# A2: Dok di Tengah, MenuBar, dan Tema Terang `.tole` - Rencana Implementasi

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dok Yoru jadi bar kaca terang di tengah bawah (tile aplikasi, titik untuk aplikasi yang punya jendela, aplikasi lain yang terbuka, Stash), ada MenuBar kaca di atas dengan menu standar, token `.tole` bawaan jadi terang, switcher wallpaper punya kartu "Default", dan anggaran waktu/RAM §12 tidak mundur.

**Architecture:** Semua kode baru Rust. Kode berada di `yoru-shell` (repo apps) dan `xtask` serta tema hyprix (repo ferrix).
- MenuBar berjalan sebagai thread di proses dok. Ini mengikuti spec §2: satu proses, banyak surface. Thread itu punya `Ui` dan koneksi Wayland sendiri.
- Special workspace `minimized` sudah ada di hyprix: `compositor-layout` `minimize`/`is_minimized`, unminimize saat fokus, dan `focuswindow address:` yang memulihkan. A2 hanya memakainya dan memverifikasinya dengan tes yang sudah ada.
- Sidebar kiri/kanan dihapus oleh spec §3 (isinya pindah ke Control dan Notices di A4). Karena itu A2 berhenti menyalakannya di tema, dan kodenya dibiarkan untuk boot yang masih mengujinya.

**Tech Stack:** Rust (yoru-shell, xtask), markup tole (`ui/*.tole`, dikompilasi `tole-compiler` di `build.rs`), IPC hyprix kompatibel Hyprland (`.socket.sock`, `.socket2.sock`), QEMU/OVMF untuk boot uji.

**Spec:** `docs/superpowers/specs/2026-11-11-desktop-gaya-macos-design.md` §2, §3, §4, §5, §8.1, §12, §13.

---

## Aturan eksekusi

- Kode dibaca dan ditulis di klon WSL `/root/ferrix` (distro `archlinux`). Repo apps ada di `/root/ferrix/src/user/apps`, sebagai repo git terpisah yang dipin di `components.toml`.
- Perintah dari Windows dijalankan lewat skrip, dengan pola: `MSYS_NO_PATHCONV=1 timeout 600 wsl.exe -d archlinux -- bash -l /mnt/c/<skrip>.sh < /dev/null 2>&1`. Semua perintah di rencana ini ditulis sebagai isi skrip itu.
- Commit mengikuti `docs/CONVENTIONS.md`:
  - pesan berisi subjek, baris kosong, lalu badan
  - **tanpa trailer `Co-Authored-By` atau `Generated with`**
  - hook aktif (`git config core.hooksPath .githooks`), dan tidak pernah memakai `--no-verify`
  - identitas pemilik repo, bukan identitas alat
  - selalu cek `git diff --cached --stat` sebelum commit
  - tidak pernah memakai `git stash`
- Setiap langkah "Commit" hanya dijalankan **setelah pengguna memberi izin**, seperti di A1.
- `Cargo.lock` di kedua repo sudah berubah sebelum A2 dimulai. Jangan ikut men-stage perubahan itu kecuali task ini memang mengubah dependensi; A2 tidak menambah dependensi. Stage berkas satu per satu.
- Tidak ada aset atau nama Apple di berkas yang dikirim (ikon, string, komentar di konfigurasi terpasang). Ikon memakai Phosphor (MIT), yang sudah ada di `look.rs`.
- Anggaran di setiap task diukur dengan `cargo xtask measure-desktop --boots 2`. Boot yang dibaca adalah boot **kedua** (root yang disimpan). Tidak boleh ada regresi dari baseline Task 0.
- Bila sebuah ambang judge dari rencana ini ternyata gagal di layar asli, **cetak nilai yang terbaca**. Ambang hanya boleh dikencangkan ke nilai nyata, tidak boleh dilonggarkan tanpa izin pengguna.

## Keputusan pengguna yang dibutuhkan

Rencana ini sudah memilih default untuk setiap butir. Butir bertanda **[KEPUTUSAN]** perlu disetujui atau diganti sebelum task terkait dieksekusi.

1. **[KEPUTUSAN] MenuBar sebagai thread di proses dok** (default, sesuai spec §2). Alternatifnya proses terpisah: `exec-once = /bin/yoru-shell --menubar`. Flag `--menubar` tetap dibuat di Task 6 sebagai jalur cadangan.
2. **[KEPUTUSAN] Sidebar tidak dinyalakan lagi sejak A2.** Akibatnya volume, tema, dan todo di sidebar hilang dari desktop sampai Control Center di A4. Boot `quick`, `weather`, dan `sidebars` tetap menyalakannya sendiri.
3. **[KEPUTUSAN] Isi lama dok dihapus:** workspace, pomodoro, focus mode, jam, dan tombol alat. Penggantinya:
   - jam pindah ke MenuBar
   - Wallpaper, Settings, dan Power pindah ke menu tanda "FX"
   - Search pindah ke ikon kaca pembesar di MenuBar dan tetap tersedia lewat SUPER Space
4. **[KEPUTUSAN] Tanda "FX" berupa teks**, sebagai pengganti sementara "FX mark" (`docs/DESKTOP.md` §6.4) yang belum digambar.
5. **[KEPUTUSAN] String UI dalam bahasa Inggris** ("Hide", "Quit", "Move to Desktop 2"), sesuai string yoru-shell yang sudah ada.
6. **[KEPUTUSAN] SUPER W menjalankan `killactive`** di `yoru-keys.conf`, dan bind debug `SUPER, W, exec, /bin/hyprctl activewindow` dihapus dari `RUN_CONFIG` (`run.rs`). Bind debug yang sama di `layout.rs`, `monitors.rs`, dan `boot.rs` tidak disentuh, karena boot itu tidak memakai tema Yoru.
7. **[KEPUTUSAN] Arti "Hide" menyimpang dari spec §5.** Hide memfokuskan lalu me-minimize setiap jendela aplikasi itu di desktop yang tampil. Spec menulis `movetoworkspacesilent special:minimized`, tetapi jalur itu melewati pembukuan `minimize` milik compositor-layout. Akibatnya jendela tidak kembali saat difokus dan tidak terhitung minimized di overview. "Quit" berarti `killactive`, yaitu menutup jendela yang fokus saja. Menutup semua jendela aplikasi baru bisa dilakukan setelah ada `ferrix_appmenu_v1` (A3).
8. **[KEPUTUSAN] Warna tile:**
   - Terminal `#2b2b30`
   - Files memakai warna aksen
   - Browser `#5aa9f0`
   - Settings `#8e9198`
   - aplikasi lain memakai `n900` dengan huruf awalnya
   - "About <app>" ditampilkan nonaktif sampai A3
9. **[KEPUTUSAN] Tema default menjadi kaca terang.** Berkas tema yang sudah ditulis "Nocturne Dark" secara eksplisit tetap gelap, dengan token hasil mesin `Tokens::new`, bukan konstanta lama `NOCTURNE`.
10. **[KEPUTUSAN] Nama cabang** `desktop/a2-dok-menubar` di kedua repo, mengikuti A1. CONVENTIONS mencantumkan prefiks lain (`feature/` dan seterusnya), jadi bila pengguna mau, ganti ke `feature/a2-dok-menubar`.
11. **[KEPUTUSAN] Push repo apps lalu pin di `components.toml`** (Task 9). Keduanya baru dilakukan setelah izin.

## Peta berkas

| Berkas | Repo | Task | Perubahan |
|---|---|---|---|
| `src/user/apps/yoru-shell/src/theme.rs` | apps | 1 | `Tokens::GLASS`, default `Appearance::Light`, `frame_requests()`, tes |
| `src/user/apps/yoru-shell/ui/nocturne.tole` | apps | 1 | nilai bawaan jadi token kaca terang |
| `src/user/apps/yoru-shell/src/window.rs` | apps | 1, 4, 6 | `frames()` lewat `frame_requests`; dok di tengah; menyalakan MenuBar |
| `src/user/apps/yoru-shell/src/walls.rs` | apps | 2 | `GRADIENT`, `gradient_at`, `default_wall`, `with_default`, `Wall::is_default/request` |
| `src/user/apps/yoru-shell/src/switch.rs` | apps | 2 | kartu Default di daftar, `request()`, cache melewati Default |
| `src/user/apps/yoru-shell/src/picker.rs` | apps | 2 | `Card.own`, painter `gradient`, tes render |
| `src/user/apps/yoru-shell/ui/walls.tole` | apps | 2 | `own: bool`, `property painter gradient`, Canvas di kartu |
| `src/user/apps/yoru-shell/src/model.rs` | apps | 3 | `State` baru, `Launcher.name/tile/window`, `Other`, `clients()` lintas desktop |
| `src/user/apps/yoru-shell/src/ui.rs` | apps | 4 | `HEIGHT 68`, `width()`, `Action` baru, `View` dok baru |
| `src/user/apps/yoru-shell/ui/dock.tole` | apps | 4 | ditulis ulang: bar kaca, tile, titik, Stash |
| `src/user/apps/yoru-shell/src/cli.rs` | apps | 4, 6 | `sample()`/`render()` baru, `--menubar`, `--render-menubar`, tes |
| `src/user/apps/yoru-shell/src/menus.rs` | apps | 5 | **baru**: model menu murni |
| `src/user/apps/yoru-shell/ui/menubar.tole` | apps | 6 | **baru** |
| `src/user/apps/yoru-shell/src/menubar.rs` | apps | 6 | **baru**: View, loop, `start()` |
| `src/user/apps/yoru-shell/src/lib.rs` | apps | 5, 6 | `pub mod menus; pub mod menubar;` |
| `src/user/apps/yoru-shell/README.md` | apps | 9 | dok baru dan MenuBar |
| `src/user/system/linux/compositor/hyprix/themes/yoru-keys.conf` | ferrix | 7 | `SUPER, W, killactive` |
| `src/user/system/linux/compositor/hyprix/themes/yoru-fuzzel.ini` | ferrix | 7 | warna terang |
| `tools/common/xtask/src/compositor/theme.rs` | ferrix | 7 | sidebar tidak lagi `exec-once`, tes |
| `tools/common/xtask/src/compositor/run.rs` | ferrix | 7 | bind debug SUPER W dihapus |
| `tools/common/xtask/src/compositor/quick.rs`, `weather.rs`, `sidebars.rs` | ferrix | 7 | menyalakan sidebar sendiri |
| `tools/common/xtask/src/compositor/yoru.rs` | ferrix | 8 | `judge` dan `judge_launcher` baru, syarat siap MenuBar, tes |
| `components.toml` | ferrix | 9 | pin apps ke commit A2 (setelah izin push) |
| `docs/superpowers/shots/a2-dok-menubar.png` | ferrix (Windows) | 9 | tangkapan layar VNC |

---

### Task 0: Persiapan, baseline, dan bukti `minimized`

Task ini tidak ada commit.

**Files:** tidak ada yang diubah.

- [ ] **Step 1: Buat cabang (setelah izin pengguna)**

```bash
cd /root/ferrix && git status --short | head
git switch -c desktop/a2-dok-menubar desktop/a1-fondasi
cd /root/ferrix/src/user/apps && git status --short | head
git switch -c desktop/a2-dok-menubar desktop/a1-fondasi
git -C /root/ferrix config core.hooksPath; git -C /root/ferrix/src/user/apps config core.hooksPath
```

Expected:
- `git status` hanya menampilkan ` M Cargo.lock`, ditambah `?? yoru-shell/target/` di repo apps.
- Kedua `config core.hooksPath` mencetak `.githooks`.
- Bila perintah itu kosong, jalankan `git config core.hooksPath .githooks` di repo itu.

- [ ] **Step 2: Baseline ukur**

```bash
cd /root/ferrix && cargo xtask measure-desktop --boots 2 2>&1 | tail -25
```

Expected (boot kedua, mendekati angka A1):
- `QEMU start -> dock drawn` kira-kira 0,6-0,8 s setelah baris `graphical.target`
- `used (MemTotal - MemAvailable): 97 MiB` atau kurang
- baris RSS: `yoru-shell` sekitar 7,6 MB (dok) dan 2 x 5,2 MB (sidebar), `hyprix` sekitar 41 MB

Salin keluaran ke catatan eksekusi. Angka ini menjadi batas atas semua task berikutnya.

- [ ] **Step 3: Bukti special workspace `minimized` sudah bekerja (spec §8.1)**

```bash
cd /root/ferrix/src/user/system/linux/compositor
cargo test -p compositor-layout a_minimized_window_comes_back_when_it_is_focused 2>&1 | tail -3
cargo test -p hyprix --test two_clients a_minimized_window_is_found_again 2>&1 | tail -3
cargo test -p hyprix --test two_clients the_switcher_brings_a_minimized_window_back 2>&1 | tail -3
```

Expected: setiap perintah berakhir dengan `test result: ok. 1 passed`. Bila ada yang gagal, berhenti: A2 bergantung pada perilaku ini.

- [ ] **Step 4: Tes yoru-shell hijau sebelum mulai**

```bash
cd /root/ferrix/src/user/apps/yoru-shell && cargo test 2>&1 | grep -E "^test result|FAILED|panicked" | head
```

Expected: semua `test result: ok`.

---

### Task 1: Token kaca terang sebagai bawaan `.tole`

Anggaran: tidak ada perubahan runtime yang terukur (warna saja).

**Files:**
- Modify: `src/user/apps/yoru-shell/src/theme.rs`
- Modify: `src/user/apps/yoru-shell/ui/nocturne.tole`
- Modify: `src/user/apps/yoru-shell/src/window.rs` (`frames`)

- [ ] **Step 1: Tulis tes yang gagal** di `mod tests` pada `theme.rs`. Ganti tes `the_default_is_the_markups_nocturne_and_the_engine_near_it` dengan dua tes berikut. Tes mesin lama dipertahankan dengan nama baru di bawahnya.

```rust
    #[test]
    fn the_default_is_the_light_glass_and_the_markup_says_so() {
        assert_eq!(
            Tokens::of(&Theme::default(), &Wallpaper::default()),
            Tokens::GLASS
        );
        assert!(Tokens::GLASS.light);
        let markup = include_str!("../ui/nocturne.tole");
        let said = |name: &str| -> Option<[u8; 3]> {
            let start = format!("property color {name}:");
            let line = markup
                .lines()
                .find(|line| line.trim_start().starts_with(&start))?;
            let hex = line.split('#').nth(1)?.get(..6)?;
            let [_, r, g, b] = u32::from_str_radix(hex, 16).ok()?.to_be_bytes();
            Some([r, g, b])
        };
        let g = Tokens::GLASS;
        assert_eq!(said("bg"), Some(g.bg));
        assert_eq!(said("surface"), Some(g.surface));
        assert_eq!(said("text"), Some(g.text));
        assert_eq!(said("accent"), Some(g.accent));
        for i in 0..9 {
            assert_eq!(said(&format!("n{}00", i + 1)), Some(g.n[i]), "n{}00", i + 1);
            assert_eq!(said(&format!("a{}00", i + 1)), Some(g.a[i]), "a{}00", i + 1);
        }
        for i in 2..6 {
            assert_eq!(said(&format!("b{}00", i + 1)), Some(g.b[i]), "b{}00", i + 1);
            assert_eq!(said(&format!("c{}00", i + 1)), Some(g.c[i]), "c{}00", i + 1);
        }
        // The stash boot finds its badge by this colour.
        assert_eq!(g.b[5], [0x00, 0x91, 0x99]);
    }

    #[test]
    fn the_engine_stays_near_nocturne_for_its_hues() {
        let t = Tokens::new([289.0, 200.0, 340.0], 1.0, false);
        let near = |got: [u8; 3], want: [u8; 3]| {
            got.iter()
                .zip(want)
                .all(|(g, w)| (i16::from(*g) - i16::from(w)).abs() <= 16)
        };
        for (i, (got, want)) in t.a.iter().zip(Tokens::NOCTURNE.a).enumerate() {
            assert!(near(*got, want), "a{}00 {got:02x?} {want:02x?}", i + 1);
        }
        assert!(near(t.n[0], Tokens::NOCTURNE.n[0]));
        assert!(!t.light);
    }

    #[test]
    fn the_glass_hands_the_frames_back_to_the_theme_file() {
        let reset = Tokens::GLASS.frame_requests();
        assert!(!reset.is_empty());
        assert!(reset.iter().all(|line| line.starts_with("setting reset ")), "{reset:?}");
        assert!(reset.contains(&"setting reset general:col.active_border".to_owned()));
        let own = Tokens::new([289.0, 200.0, 340.0], 1.3, false).frame_requests();
        assert!(own.iter().all(|line| line.starts_with("setting ") && !line.starts_with("setting reset ")));
    }
```

Di tes `the_file_round_trips_and_ignores_what_it_does_not_know`, baris `assert_eq!(odd.appearance, Appearance::Dark);` diganti menjadi `assert_eq!(odd.appearance, Appearance::Light);`, karena yang tidak dikenal kini jatuh ke default terang.

- [ ] **Step 2: Jalankan, pastikan gagal**

```bash
cd /root/ferrix/src/user/apps/yoru-shell && cargo test --lib theme:: 2>&1 | tail -15
```

Expected: gagal kompilasi, dengan `no associated item named GLASS` dan `no method named frame_requests`.

- [ ] **Step 3: Implementasi di `theme.rs`**

Ubah `impl Default for Theme` dan doc comment-nya:

```rust
/// Until a person chooses, Yoru's light glass (`Tokens::GLASS`, the
/// markup's own values; spec §4).
impl Default for Theme {
    fn default() -> Self {
        Theme {
            mode: Mode::Named(0),
            appearance: Appearance::Light,
            vibrance: 1.3,
        }
    }
}
```

Tambahkan konstanta berikut tepat setelah `pub const NOCTURNE: Tokens = {...};`:

```rust
    /// Yoru's light glass, the default (spec §4): a white surface, ink near
    /// black, a clear blue accent. The ramps run from 100 the darkest to
    /// 900 the lightest, as the markup uses them on a light ground (hover
    /// on `n900`, quiet ink on `n300`); accent-2's 600 is the stash badge's
    /// teal, as before.
    pub const GLASS: Tokens = {
        const fn c(hex: u32) -> Rgb {
            [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8]
        }
        Tokens {
            light: true,
            bg: c(0xf2_f2f5),
            surface: c(0xff_ffff),
            text: c(0x1d_1d1f),
            accent: c(0x0a_84ff),
            n: [
                c(0x1f_2024),
                c(0x33_353b),
                c(0x4a_4d55),
                c(0x64_6872),
                c(0x86_8a94),
                c(0xa7_abb4),
                c(0xc6_c9d0),
                c(0xdc_dee3),
                c(0xec_edf0),
            ],
            a: [
                c(0x06_2a57),
                c(0x0a_3d7d),
                c(0x0d_53a8),
                c(0x0a_6fdc),
                c(0x0a_84ff),
                c(0x4a_a3ff),
                c(0x8c_c4ff),
                c(0xc4_e0ff),
                c(0xe6_f1ff),
            ],
            b: [
                c(0x05_3a3d),
                c(0x08_494d),
                c(0x0b_5d62),
                c(0x00_7980),
                c(0x00_868d),
                c(0x00_9199),
                c(0x55_c3c8),
                c(0xa6_e2e5),
                c(0xdc_f4f5),
            ],
            c: [
                c(0x4f_1037),
                c(0x6c_1a4c),
                c(0x8a_2763),
                c(0xa8_387b),
                c(0xc2_5294),
                c(0xd4_7fb0),
                c(0xe3_a9cb),
                c(0xf0_d0e3),
                c(0xf9_ebf3),
            ],
        }
    };
```

Di `Tokens::of`, ganti `return Tokens::NOCTURNE;` dengan `return Tokens::GLASS;`, lalu ubah doc comment-nya menjadi `/// The tokens of `theme` over `wall`: the default's are `GLASS`.`.

Tambahkan method berikut setelah `frame_settings`:

```rust
    /// What hyprix is asked so the window frames wear these tokens: for the
    /// glass, the theme file's own frames back (each kept setting reset);
    /// for any other, each setting said.
    #[must_use]
    pub fn frame_requests(&self) -> Vec<String> {
        let glass = *self == Tokens::GLASS;
        self.frame_settings()
            .into_iter()
            .map(|(key, value)| {
                if glass {
                    format!("setting reset {key}")
                } else {
                    format!("setting {key} {value}")
                }
            })
            .collect()
    }
```

- [ ] **Step 4: `nocturne.tole` memakai nilai kaca terang.** Nama singleton `Nocturne` tetap, supaya semua markup lain tidak berubah. Ganti blok properti warna dan komentar kepalanya:

```
// Nocturne, the design system Yoru is drawn in, as tokens: the ground, the
// surface, the text, and the 100-900 ramps of the neutrals and the three
// accents. Until the theme says otherwise (src/theme.rs, Tokens::GLASS),
// Yoru's light glass: 100 the darkest, 900 the lightest. hyprix's
// themes/yoru.conf carries the same look for the window frames.
singleton Nocturne {
    property color bg: "#f2f2f5"
    property color surface: "#ffffff"
    property color text: "#1d1d1f"
    property color accent: "#0a84ff"
    property color divider: #1d1d1f29

    property color n100: "#1f2024"
    property color n200: "#33353b"
    property color n300: "#4a4d55"
    property color n400: "#646872"
    property color n500: "#868a94"
    property color n600: "#a7abb4"
    property color n700: "#c6c9d0"
    property color n800: "#dcdee3"
    property color n900: "#ecedf0"

    property color a100: "#062a57"
    property color a200: "#0a3d7d"
    property color a300: "#0d53a8"
    property color a400: "#0a6fdc"
    property color a500: "#0a84ff"
    property color a600: "#4aa3ff"
    property color a700: "#8cc4ff"
    property color a800: "#c4e0ff"
    property color a900: "#e6f1ff"

    property color b300: "#0b5d62"
    property color b400: "#007980"
    property color b500: "#00868d"
    property color b600: "#009199"

    property color c300: "#8a2763"
    property color c400: "#a8387b"
    property color c500: "#c25294"
    property color c600: "#d47fb0"
```

Bagian font (`sansFamily`, `iconFamily`, `sans()`, `icon()`) tidak diubah.

- [ ] **Step 5: `frames()` di `window.rs` memakai `frame_requests`.** Ganti badan fungsinya:

```rust
fn frames(hypr: Option<&Hypr>, tokens: &crate::theme::Tokens) {
    let Some(hypr) = hypr else { return };
    for line in tokens.frame_requests() {
        let said = hypr.ask(&line).unwrap_or_default();
        if said.trim() != "ok" {
            say(&format!("theme: {line}: {}", said.trim()));
        }
    }
}
```

Catatan: `setting reset` memuat ulang konfigurasi. Sudah diperiksa di `hyprix/src/state.rs:1819` bahwa `exec-once` tidak dijalankan lagi saat reload. `frames()` hanya dipanggil saat tema berubah, bukan saat start.

**Belum diketahui:** jawaban `setting reset KEY` untuk kunci yang tidak ada di `settings.conf` (apakah `ok` atau pesan lain). Cara memeriksa: setelah boot, ganti tema ke default dua kali, lalu periksa log `theme: setting reset ...: <jawaban>`. Bila jawabannya bukan `ok` tetapi tidak berbahaya, biarkan baris log itu saja.

- [ ] **Step 6: Tes hijau dan tidak ada yang rusak**

```bash
cd /root/ferrix/src/user/apps/yoru-shell && cargo test 2>&1 | grep -E "^test result|FAILED|panicked" | head
```

Expected: semua `test result: ok`. Tes `power.rs` dan `shelf.rs` tidak bergantung warna. Bila ada tes render lain yang memeriksa warna gelap (misalnya `render_sidebar`), perbarui ambangnya ke latar terang. Jangan menghapus pemeriksaannya.

- [ ] **Step 7: Commit (setelah izin pengguna)**

```bash
cd /root/ferrix/src/user/apps
git add yoru-shell/src/theme.rs yoru-shell/ui/nocturne.tole yoru-shell/src/window.rs
git diff --cached --stat
git commit -m "yoru-shell: the light glass is the markup's default" -m "Tokens::GLASS is what Theme::default() draws and what nocturne.tole says; a test reads the markup back. Going back to the default resets the frames to the theme file's own rather than writing them."
```

---

### Task 2: Kartu "Default" di switcher wallpaper

Anggaran: tidak ada perubahan pada boot desktop. Switcher dibuka sesuai permintaan.

**Files:**
- Modify: `src/user/apps/yoru-shell/src/walls.rs`
- Modify: `src/user/apps/yoru-shell/src/switch.rs`
- Modify: `src/user/apps/yoru-shell/src/picker.rs`
- Modify: `src/user/apps/yoru-shell/ui/walls.tole`

- [ ] **Step 1: Tes yang gagal** di `mod tests` pada `walls.rs`:

```rust
    #[test]
    fn the_default_card_is_the_themes_gradient() {
        let own = default_wall();
        assert!(own.is_default());
        assert_eq!(own.name, "Default");
        assert_eq!(own.request(), "setting reset wallpaper");
        assert!(own.look.is_some(), "read without a file");
        let picture = wall("dune", "Ferrix", None);
        assert!(!picture.is_default());
        assert_eq!(
            picture.request(),
            "setting wallpaper , picture /w/dune.jpg, fit fill"
        );
        let (walls, folders) = with_default(vec![picture], vec!["Mine".to_owned()]);
        assert!(walls[0].is_default());
        assert_eq!(folders, vec![DEFAULT_FOLDER.to_owned(), "Mine".to_owned()]);
        // The gradient in force: in_force says nothing, so the default is held.
        let picker = Picker::new(walls, folders, DEFAULT);
        assert_eq!(picker.held(), Some(0));
    }

    #[test]
    fn the_gradient_is_the_one_yoru_conf_draws() {
        let conf = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../system/linux/compositor/hyprix/themes/yoru.conf"
        );
        let Ok(text) = std::fs::read_to_string(conf) else {
            eprintln!("{conf} is not here (apps built alone): skipped");
            return;
        };
        let line = text
            .lines()
            .find(|line| line.trim_start().starts_with("wallpaper") && line.contains("gradient"))
            .expect("yoru.conf's wallpaper line");
        let hex = |[r, g, b]: [u8; 3]| format!("{r:02x}{g:02x}{b:02x}");
        for (at, colour) in GRADIENT {
            assert!(line.contains(&hex(colour)), "{} in {line}", hex(colour));
            if at > 0.0 && at < 1.0 {
                let percent = format!("{}%", (at * 100.0).round());
                assert!(line.contains(&percent), "{percent} in {line}");
            }
        }
        assert_eq!(gradient_at(0.0), GRADIENT[0].1);
        assert_eq!(gradient_at(1.0), GRADIENT[2].1);
    }
```

Di `mod tests` pada `picker.rs`:

```rust
    #[test]
    fn the_default_card_draws_the_gradient() {
        let (walls, folders) = crate::walls::with_default(Vec::new(), Vec::new());
        let picker = crate::walls::Picker::new(walls, folders, crate::walls::DEFAULT);
        let frame = crate::cli::render_walls(&picker, (1600, 1000), 1.0).expect("drawn");
        let pixmap = frame.pixmap();
        let at = |x: u32, y: u32| {
            let pixel = pixmap.pixel(x, y).expect("inside").demultiply();
            (pixel.red(), pixel.green(), pixel.blue())
        };
        let middle_y = super::place(&picker, 1600.0, 600.0)
            .last()
            .map(|p| p.2 + p.4 / 2.0)
            .expect("placed");
        let stage = 162.0_f32.max((1000.0 - 112.0 + 162.0) / 2.0 - 170.0);
        let y = (stage + middle_y) as u32;
        let light: Vec<(u8, u8, u8)> = (600..1000)
            .map(|x| at(x, y))
            .filter(|&(r, g, b)| r > 0xa0 && g > 0xc0 && b > 0xc0)
            .collect();
        assert!(light.len() > 150, "the gradient is drawn: {}", light.len());
        let reds = light.iter().map(|p| p.0);
        let spread = reds.clone().max().unwrap_or(0) - reds.min().unwrap_or(0);
        assert!(spread > 8, "it runs from blue toward sand: {spread}");
    }
```

- [ ] **Step 2: Jalankan, pastikan gagal**

```bash
cd /root/ferrix/src/user/apps/yoru-shell && cargo test --lib walls:: picker:: 2>&1 | tail -8
```

Expected: gagal kompilasi, dengan `cannot find function default_wall` dan item lain yang belum ada.

- [ ] **Step 3: Implementasi di `walls.rs`.** Tambahkan setelah `pub const DEFAULT: &str = "";`:

```rust
/// Yoru's own wallpaper as `themes/yoru.conf` draws it
/// (`gradient 110deg a9c6f5 e8d5f2 55% f6e3c9`): (place 0 to 1, colour).
pub const GRADIENT: [(f32, [u8; 3]); 3] = [
    (0.0, [0xa9, 0xc6, 0xf5]),
    (0.55, [0xe8, 0xd5, 0xf2]),
    (1.0, [0xf6, 0xe3, 0xc9]),
];

/// The folder the default card is shown in.
pub const DEFAULT_FOLDER: &str = "Ferrix";

/// The gradient's colour `t` of the way along it.
#[must_use]
pub fn gradient_at(t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let (from, to) = if t <= GRADIENT[1].0 {
        (GRADIENT[0], GRADIENT[1])
    } else {
        (GRADIENT[1], GRADIENT[2])
    };
    let along = (t - from.0) / (to.0 - from.0);
    let mix = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * along).round() as u8;
    [
        mix(from.1[0], to.1[0]),
        mix(from.1[1], to.1[1]),
        mix(from.1[2], to.1[2]),
    ]
}

/// The theme's own wallpaper as a card: picked, the setting a person chose
/// is taken back and `yoru.conf`'s gradient shows again. Its colour is
/// read off the gradient itself, 48 by 27 as a picture's is.
#[must_use]
pub fn default_wall() -> Wall {
    let mut sample = Vec::with_capacity(48 * 27);
    for _row in 0..27 {
        for column in 0..48_u16 {
            let [r, g, b] = gradient_at((f32::from(column) + 0.5) / 48.0);
            sample.push([r, g, b, 0xff]);
        }
    }
    Wall {
        path: DEFAULT.to_owned(),
        name: "Default".to_owned(),
        folder: DEFAULT_FOLDER.to_owned(),
        video: false,
        look: Some(look(&sample)),
    }
}

/// `walls` and `folders` as found, the default card first.
#[must_use]
pub fn with_default(mut walls: Vec<Wall>, mut folders: Vec<String>) -> (Vec<Wall>, Vec<String>) {
    walls.insert(0, default_wall());
    if !folders.iter().any(|folder| folder == DEFAULT_FOLDER) {
        folders.insert(0, DEFAULT_FOLDER.to_owned());
    }
    (walls, folders)
}
```

Di `impl Wall`:
- Tambahkan di awal `meta()`: `if self.is_default() { return "Gradient · the theme's own".to_owned(); }`
- Tambahkan:

```rust
    /// The theme's own gradient rather than a file.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.path == DEFAULT
    }

    /// What hyprix is asked to show it everywhere.
    #[must_use]
    pub fn request(&self) -> String {
        if self.is_default() {
            "setting reset wallpaper".to_owned()
        } else {
            format!("setting wallpaper {}", self.setting())
        }
    }
```

Catatan: ini memakai `setting reset wallpaper`, bukan menulis baris gradien. Akibatnya bila `yoru.conf` berubah, Default ikut berubah. Foto pengguna (PNG/JPEG/SVG) tetap lewat `setting()` dan `compositor_image`, jadi jalurnya tidak berubah.

- [ ] **Step 4: `switch.rs`**
  - Di `found()`, setelah `for wall in &mut found { ... }` dan sebelum `let current`, tambahkan `let (found, folders) = walls::with_default(found, folders);`.
  - Di `apply()`, ganti `let line = format!("setting wallpaper {}", wall.setting());` dengan `let line = wall.request();`.
  - Di `take_read()`, filter cache `.filter(|wall| !wall.video)` menjadi `.filter(|wall| !wall.video && !wall.is_default())`.

- [ ] **Step 5: `picker.rs`.** Tambahkan setelah `fn shade()`:

```rust
/// The default card's picture: Yoru's gradient across it in columns 2
/// pixels wide. Upgrade path: the 110deg slant `yoru.conf` gives it, should
/// a card ever be big enough to show the difference.
fn gradient() -> tole::CanvasPainter {
    tole::CanvasPainter::new(|painter: &mut tole::Painter<'_>| {
        let (width, height) = painter.size();
        let mut x = 0.0;
        while x < width {
            let [r, g, b] = crate::walls::gradient_at((x + 1.0) / width);
            painter.fill_rect(
                tole::Rect {
                    x,
                    y: 0.0,
                    w: 2.0_f32.min(width - x),
                    h: height,
                },
                0.0,
                tole::Color::rgba(r, g, b, 0xff),
            );
            x += 2.0;
        }
    })
}
```

Di `View::new`, setelah `walls.shade.set(&mut ui, shade());`, tambahkan `walls.gradient.set(&mut ui, gradient());`.

Di pembuatan `Card { ... }`:

```rust
                    source: s(if wall.video || wall.is_default() { "" } else { wall.path.as_str() }),
```

Tambahkan juga field `own: wall.is_default(),` setelah `dim,`.

- [ ] **Step 6: `walls.tole`**
  - Di `struct Card` (sekitar baris 30), tambahkan baris `own: bool`.
  - Setelah `property painter shade` (baris 60), tambahkan `property painter gradient`.
  - Di delegate kartu, tepat setelah blok `Image { ... }` (sekitar baris 473-479), tambahkan:

```
                    Canvas {
                        visible: modelData.own
                        x: 1; y: 1
                        width: modelData.w - 2
                        height: modelData.h - 2
                        painter: walls.gradient
                    }
```

**Belum diketahui:** apakah tole menerima `Canvas` di dalam delegate `Repeater` yang mengikat painter milik root. Cara memeriksa: `cargo build` di Step 7. Bila compiler menolak, ganti Canvas dengan tiga `Rectangle` bertumpuk (lebar 55%, 45%, warna `#a9c6f5`, `#e8d5f2`, `#f6e3c9`) dan perbarui tes `spread`. Jangan melonggarkan `light.len() > 150`.

- [ ] **Step 7: Tes hijau**

```bash
cd /root/ferrix/src/user/apps/yoru-shell && cargo test 2>&1 | grep -E "^test result|FAILED|panicked" | head
cargo run -q -- --render-walls /tmp/a2-walls.png && ls -l /tmp/a2-walls.png
```

Expected:
- semua `test result: ok`
- PNG ada; kartu pertama menampilkan gradien biru-lavender-pasir dengan nama "Default"

- [ ] **Step 8: Commit (setelah izin pengguna)**

```bash
cd /root/ferrix/src/user/apps
git add yoru-shell/src/walls.rs yoru-shell/src/switch.rs yoru-shell/src/picker.rs yoru-shell/ui/walls.tole
git diff --cached --stat
git commit -m "yoru-shell: a Default card in the wallpaper switcher" -m "The theme's own gradient as the first card, drawn by code from the same stops yoru.conf uses (a test reads them back). Picking it resets the wallpaper setting, so the theme file's line shows again; pictures still go through the compositor's image loader."
```

---

### Task 3: Model dok: aplikasi berjalan di desktop mana pun

Anggaran: tidak ada perubahan pada boot (model murni).

**Files:**
- Modify: `src/user/apps/yoru-shell/src/model.rs`

- [ ] **Step 1: Tes yang gagal** di `mod tests` pada `model.rs`:

```rust
    #[test]
    fn running_wherever_the_windows_are() {
        let mut state = State::new(launchers(&[(
            "settings.desktop".to_owned(),
            "settings".to_owned(),
            None,
        )]));
        let list = Json::parse(
            r#"[
              {"address":"0xa","class":"foot","workspace":{"id":2,"name":"2"},"focusHistoryID":1},
              {"address":"0xb","class":"foot","workspace":{"id":-99,"name":"special:minimized"},"focusHistoryID":0},
              {"address":"0xc","class":"org.example.Notes","workspace":{"id":1,"name":"1"},"focusHistoryID":2},
              {"address":"0xd","class":"org.example.Notes","workspace":{"id":3,"name":"3"},"focusHistoryID":3}
            ]"#,
        )
        .expect("json");
        state.clients(&list);
        let term = &state.launchers[0];
        assert_eq!(term.name, "Terminal");
        assert!(term.running, "a window on desktop 2 counts");
        assert_eq!(term.window.as_deref(), Some("0xb"), "the one used last, minimized or not");
        assert!(!state.launchers[1].running, "Settings has no window");
        assert_eq!(
            state.others,
            vec![Other {
                class: "org.example.Notes".to_owned(),
                name: "Notes".to_owned(),
                window: "0xc".to_owned(),
            }]
        );
        assert_eq!(shown_class("rocks.magical.shelf"), "Shelf");
        assert_eq!(shown_class("foot"), "Foot");
    }
```

Bila `Json` belum di-`use` di `mod tests`, tambahkan `use crate::json::Json;`.

- [ ] **Step 2: Jalankan, pastikan gagal**

```bash
cd /root/ferrix/src/user/apps/yoru-shell && cargo test --lib model:: 2>&1 | tail -8
```

Expected: gagal kompilasi (`State::new` butuh 2 argumen, `Other` tidak ada).

- [ ] **Step 3: Implementasi.** Field di `Launcher` setelah `running`:

```rust
    /// Its name in the menu bar.
    pub name: String,
    /// Its tile's colour in the dock.
    pub tile: Tile,
    /// The window of it used last, minimized or not, as `0x..`; `None`
    /// when it has none.
    pub window: Option<String>,
```

Ubah doc `running` menjadi `/// Whether it has a window anywhere, minimized ones included.`

Tambahkan sebelum `pub struct Launcher`:

```rust
/// A tile's colour (the dock's markup draws each).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    /// Near black, white glyph.
    Dark = 0,
    /// The theme's accent.
    Accent = 1,
    /// A sky blue.
    Sky = 2,
    /// A cool grey.
    Grey = 3,
}

/// A program with windows that has no launcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Other {
    /// Its window class.
    pub class: String,
    /// Its name as shown.
    pub name: String,
    /// Its window used last.
    pub window: String,
}

/// A window class as a name: its last dotted part, the first letter up.
#[must_use]
pub fn shown_class(class: &str) -> String {
    let last = class.rsplit('.').next().unwrap_or(class);
    let mut chars = last.chars();
    chars
        .next()
        .map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
}

impl Launcher {
    /// Whether a window of `class` is this program's.
    #[must_use]
    pub fn is(&self, class: &str) -> bool {
        self.classes
            .iter()
            .any(|wanted| class.eq_ignore_ascii_case(wanted))
    }
}
```

`State` baru menggantikan struct lama:

```rust
/// Everything the dock shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    /// The launchers.
    pub launchers: Vec<Launcher>,
    /// The programs with windows that have no launcher, by class.
    pub others: Vec<Other>,
    /// How many files the stash holds.
    pub stashed: usize,
}

impl State {
    /// The dock with `launchers` and nothing yet known of the compositor.
    #[must_use]
    pub fn new(launchers: Vec<Launcher>) -> Self {
        State {
            launchers,
            others: Vec::new(),
            stashed: 0,
        }
    }

    /// Which programs have windows, on any desktop or minimized, from
    /// `j/clients`: each launcher's window used last, and the rest.
    pub fn clients(&mut self, list: &Json) {
        let mut windows: Vec<(i64, String, String)> = list
            .items()
            .iter()
            .filter_map(|client| {
                let class = client.get("class").and_then(Json::str)?;
                let address = client.get("address").and_then(Json::str)?;
                let order = client
                    .get("focusHistoryID")
                    .and_then(Json::int)
                    .unwrap_or(i64::MAX);
                (!class.is_empty()).then(|| (order, class.to_owned(), address.to_owned()))
            })
            .collect();
        windows.sort_by_key(|window| window.0);
        for launcher in &mut self.launchers {
            launcher.window = windows
                .iter()
                .find(|(_, class, _)| launcher.is(class))
                .map(|(_, _, address)| address.clone());
            launcher.running = launcher.window.is_some();
        }
        let mut others: Vec<Other> = Vec::new();
        for (_, class, address) in &windows {
            let known = self.launchers.iter().any(|launcher| launcher.is(class))
                || others.iter().any(|other| other.class == *class);
            if !known {
                others.push(Other {
                    class: class.clone(),
                    name: shown_class(class),
                    window: address.clone(),
                });
            }
        }
        others.sort_by(|a, b| a.class.cmp(&b.class));
        self.others = others;
    }
}
```

Di `launchers()`, setiap `Launcher { ... }` mendapat `window: None` dan `name`/`tile` berikut:

| launcher | `name` | `tile` |
|---|---|---|
| terminal | `"Terminal".to_owned()` | `Tile::Dark` |
| browser | `"Browser".to_owned()` | `Tile::Sky` |
| files | `"Files".to_owned()` | `Tile::Accent` |
| settings | `"Settings".to_owned()` | `Tile::Grey` |

- [ ] **Step 4: Bersihkan yang menjadi yatim karena perubahan ini**

```bash
cd /root/ferrix/src/user/apps/yoru-shell
grep -rn "Pomodoro\|Tool::\|model::Tool\|NAMED\|Space\b\|\.workspaces(\|\.spaces\|\.pomodoro\|\.focus\b\|state\.clock\|State::new" src/ | grep -v "^src/model.rs"
```

Expected: yang muncul hanya `src/ui.rs`, `src/window.rs`, dan `src/cli.rs`, yang ditulis ulang di Task 4.
- Bila `side.rs` atau modul lain memakai `Pomodoro`, `Tool`, atau `Space`, item itu **dipertahankan**.
- Bila tidak ada pemakai lain, hapus dari `model.rs`: `NAMED`, `Space`, `Tool` (dan `Tool::ALL`), `Pomodoro`, `State::workspaces`, beserta tes `workspaces_and_their_windows` dan `a_pomodoro`.
- `Clock`, `civil`, `offset`, dan tes `the_clock` dipertahankan, karena dipakai `side.rs` dan MenuBar.

- [ ] **Step 5: Tes model hijau.** Crate belum bisa dikompilasi utuh sampai Task 4, jadi Task 3 dan 4 di-commit bersama.

```bash
cd /root/ferrix/src/user/apps/yoru-shell && cargo test --lib model:: 2>&1 | tail -3
```

Expected: gagal kompilasi di `ui.rs`/`window.rs` saja. Lanjut langsung ke Task 4; commit di akhir Task 4.

---

### Task 4: Dok kaca di tengah

Anggaran (boot kedua, `measure-desktop --boots 2`):
- RSS dok `yoru-shell` paling banyak 7,6 MB
- `dock drawn` paling lambat 0,8 s setelah `graphical.target`
- `used` paling banyak 97 MiB

**Files:**
- Modify: `src/user/apps/yoru-shell/ui/dock.tole` (tulis ulang)
- Modify: `src/user/apps/yoru-shell/src/ui.rs`
- Modify: `src/user/apps/yoru-shell/src/window.rs`
- Modify: `src/user/apps/yoru-shell/src/cli.rs`

- [ ] **Step 1: Tes yang gagal.** Ganti `sample()` dan tes `the_dock_draws` di `cli.rs`:

```rust
/// The dock as drawn for a picture: the terminal running, the browser and
/// Settings installed.
#[must_use]
pub fn sample() -> State {
    let mut state = State::new(launchers(&[
        (
            "google-chrome.desktop".to_owned(),
            "/bin/chrome".to_owned(),
            None,
        ),
        ("settings.desktop".to_owned(), "settings".to_owned(), None),
    ]));
    let clients = crate::json::Json::parse(
        r#"[{"address":"0x1","class":"rocks.magical.term","workspace":{"id":1},"focusHistoryID":0}]"#,
    )
    .unwrap_or(crate::json::Json::Null);
    state.clients(&clients);
    state
}
```

```rust
    #[test]
    fn the_dock_draws() {
        let state = sample();
        let width = crate::ui::width(&state);
        let frame = render(&state, 1.0).expect("drawn");
        let pixmap = frame.pixmap();
        assert_eq!((pixmap.width(), pixmap.height()), (width, crate::ui::HEIGHT));
        let at = |x: u32, y: u32| {
            let pixel = pixmap.pixel(x, y).expect("inside").demultiply();
            (pixel.red(), pixel.green(), pixel.blue())
        };
        // The glass above the tiles: light and near grey.
        let (r, g, b) = at(width / 2, 4);
        let (lo, hi) = (r.min(g).min(b), r.max(g).max(b));
        assert!(lo >= 0xe0 && hi - lo <= 0x10, "the glass: {r} {g} {b}");
        // The terminal's dark tile comes first.
        let pad = crate::ui::PAD;
        let (r, g, b) = at(pad + 4, pad + 4);
        assert!(
            (0x25..=0x31).contains(&r) && (0x25..=0x31).contains(&g) && (0x2a..=0x36).contains(&b),
            "the terminal's tile: {r} {g} {b}"
        );
        // Its dot under it: it has a window.
        let dot = (pad..pad + crate::ui::TILE).any(|x| {
            (59..=65).any(|y| {
                let (r, g, b) = at(x, y);
                r < 0x70 && g < 0x70 && b < 0x80
            })
        });
        assert!(dot, "the terminal's running dot");
        // Settings has no window: nothing dark under its tile (the third).
        let third = pad + 2 * (crate::ui::TILE + crate::ui::GAP);
        let none = (third..third + crate::ui::TILE).all(|x| {
            let (r, _, _) = at(x, 62);
            r > 0xa0
        });
        assert!(none, "no dot under Settings");
    }
```

- [ ] **Step 2: Jalankan, pastikan gagal**

```bash
cd /root/ferrix/src/user/apps/yoru-shell && cargo test --lib cli::tests::the_dock_draws 2>&1 | tail -6
```

Expected: gagal kompilasi (`crate::ui::width` tidak ada, `render` butuh 3 argumen).

- [ ] **Step 3: `ui/dock.tole` baru (seluruh isi)**

```
// Yoru's dock (spec §4): a bar of glass in the middle of the screen's
// foot. The launchers as tiles with a dot under each that has a window
// anywhere, the other programs with windows after them, a line, and the
// stash. Rust hands it what to show and turns its signals into the
// compositor's dispatchers (`src/ui.rs`).
import Fx

struct AppRow {
    glyph: string
    letter: string
    tile: int
    title: string
    running: bool
}

export component Dock : Item {
    id: dock
    property list<AppRow> apps
    property string stashIcon
    property string stashBadge
    property bool overStash: stashArea.containsMouse

    signal app(int index)
    signal stash()
    // What the pointer is over: a tile's title, said for its tooltip
    // (empty when it leaves).
    signal hover(string title)

    Rectangle {
        anchors.fill: parent
        radius: 18
        color: alpha(Nocturne.surface, 0.8)
        border.width: 1
        border.color: alpha(Nocturne.text, 0.12)
    }

    Row {
        x: 10; y: 10
        height: 56
        spacing: 8
        Repeater {
            model: dock.apps
            Item {
                width: 48; height: 56
                Rectangle {
                    width: 48; height: 48
                    radius: 11
                    color: modelData.tile == 0 ? "#2b2b30" : modelData.tile == 1 ? Nocturne.accent : modelData.tile == 2 ? "#5aa9f0" : modelData.tile == 3 ? "#8e9198" : Nocturne.n900
                    Text {
                        anchors.centerIn: parent
                        visible: modelData.glyph != ""
                        text: modelData.glyph
                        font: Nocturne.icon(26)
                        color: "#ffffff"
                    }
                    Text {
                        anchors.centerIn: parent
                        visible: modelData.glyph == ""
                        text: modelData.letter
                        font: Nocturne.sans(20, 600)
                        color: Nocturne.text
                    }
                }
                Rectangle {
                    x: 22; y: 51
                    width: modelData.running ? 4 : 0
                    height: 4
                    radius: 2
                    color: Nocturne.n300
                }
                MouseArea {
                    x: 0; y: 0
                    width: 48; height: 48
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: dock.app(index)
                    onEntered: dock.hover(modelData.title)
                    onExited: dock.hover("")
                }
            }
        }
        Rectangle {
            y: 8
            width: 1; height: 32
            color: Nocturne.divider
        }
        // The stash: files dropped here are put aside; a press opens the
        // panel of them.
        Item {
            width: 48; height: 56
            Rectangle {
                width: 48; height: 48
                radius: 11
                color: stashArea.containsMouse ? Nocturne.n800 : Nocturne.n900
                Text {
                    anchors.centerIn: parent
                    text: dock.stashIcon
                    font: Nocturne.icon(24)
                    color: Nocturne.n300
                }
            }
            Rectangle {
                visible: dock.stashBadge != ""
                x: 48 - width + 2; y: -2
                width: max(16, badgeText.implicitWidth + 8)
                height: 16
                radius: 8
                color: Nocturne.b600
                border.width: 2
                border.color: Nocturne.surface
                Text {
                    id: badgeText
                    anchors.centerIn: parent
                    text: dock.stashBadge
                    font: Nocturne.sans(9.5, 600)
                    color: "#e6fbfc"
                }
            }
            MouseArea {
                id: stashArea
                x: 0; y: 0
                width: 48; height: 48
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: dock.stash()
                onEntered: dock.hover("Stash · drop files here")
                onExited: dock.hover("")
            }
        }
    }
}
```

**Belum diketahui:** apakah markup tole menerima `==` pada `int` dan ternary bertingkat lima. `!=` pada string dan ternary bertingkat sudah dipakai di `dock.tole` lama. Cara memeriksa: `cargo build`. Bila `==` ditolak, ganti `tile: int` dengan empat field `bool` (`dark`, `accent`, `sky`, `grey`) dan ternary `modelData.dark ? ... : modelData.accent ? ...`, lalu ubah `ui.rs` sesuai.

- [ ] **Step 4: `ui.rs` (ganti dari `pub const HEIGHT` sampai akhir berkas; `tole::include_ui!();` tetap)**

```rust
use std::rc::Rc;

use tole::{Element as _, Ui};

use crate::look::icon;
use crate::model::State;

tole::include_ui!();

/// The dock's height, its glass included, in logical pixels.
pub const HEIGHT: u32 = 68;
/// A tile's side.
pub const TILE: u32 = 48;
/// Between tiles.
pub const GAP: u32 = 8;
/// Round the tiles, inside the glass.
pub const PAD: u32 = 10;

/// How wide the dock is for `state`: its tiles and the stash's, the gaps,
/// and the line before the stash (1 wide, a gap each side).
#[must_use]
pub fn width(state: &State) -> u32 {
    let tiles = u32::try_from(state.launchers.len() + state.others.len() + 1).unwrap_or(1);
    2 * PAD + tiles * (TILE + GAP) + 1
}

/// What a press in the dock asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Launcher `n`: its window brought, or the program started.
    Launch(usize),
    /// Other program `n`: its window brought.
    Other(usize),
    /// The stash's panel.
    Stash,
    /// The pointer came over a tile (its title), or left one (empty).
    Hover(String),
}

/// A string as the tree holds it.
fn s(value: impl AsRef<str>) -> Rc<str> {
    Rc::from(value.as_ref())
}

/// The tree and what the program keeps of it.
pub struct View {
    /// The tree.
    pub ui: Ui,
    /// The dock.
    pub dock: Dock,
    /// The state last shown.
    shown: Option<State>,
}

impl std::fmt::Debug for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("View").finish_non_exhaustive()
    }
}

impl View {
    /// The tree, drawn with `fonts`; `None` when the markup's items could
    /// not be made, a fault of this program.
    #[must_use]
    pub fn new(fonts: tole::Fonts) -> Option<Self> {
        let mut ui = Ui::new(fonts);
        let dock = Dock::new(&mut ui, None)?;
        dock.stash_icon.set(&mut ui, s(icon("archive")));
        Some(View {
            ui,
            dock,
            shown: None,
        })
    }

    /// The dock's size in logical pixels.
    pub fn resize(&mut self, (width, height): (u32, u32)) {
        let common = self.dock.common();
        common.width.set(&mut self.ui, width as f32);
        common.height.set(&mut self.ui, height as f32);
    }

    /// Hand the tree `state`.
    pub fn show(&mut self, state: &State) {
        if self.shown.as_ref() == Some(state) {
            return;
        }
        let ui = &mut self.ui;
        let apps = state
            .launchers
            .iter()
            .map(|launcher| AppRow {
                glyph: s(icon(launcher.icon)),
                letter: s(""),
                tile: launcher.tile as i32,
                title: s(&launcher.title),
                running: launcher.running,
            })
            .chain(state.others.iter().map(|other| AppRow {
                glyph: s(""),
                letter: s(other
                    .name
                    .chars()
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>())
                    .unwrap_or_default()),
                tile: 4,
                title: s(&other.name),
                running: true,
            }))
            .collect::<Vec<_>>();
        self.dock.apps.set(ui, apps);
        self.dock
            .stash_badge
            .set(ui, s(crate::stash::badge(state.stashed)));
        self.shown = Some(state.clone());
    }

    /// What the tree asked since the last call.
    pub fn actions(&mut self) -> Vec<Action> {
        let emitted = self.ui.take_signals();
        let launchers = self.shown.as_ref().map_or(0, |state| state.launchers.len());
        let mut actions = Vec::new();
        for each in &emitted {
            let Some(signal) = self.dock.signal(each) else {
                continue;
            };
            let action = match signal {
                DockSignal::App(i) => usize::try_from(i).ok().map(|i| {
                    if i < launchers {
                        Action::Launch(i)
                    } else {
                        Action::Other(i - launchers)
                    }
                }),
                DockSignal::Stash => Some(Action::Stash),
                DockSignal::Hover(title) => Some(Action::Hover(title.to_string())),
            };
            actions.extend(action);
        }
        actions
    }
}
```

Painter `frame()` lama dan import `Color` dihapus, karena perubahan ini membuat keduanya yatim. Komentar kepala berkas (`//!`) tetap.

- [ ] **Step 5: `window.rs`**
  - Import: `use crate::model::{self, State};` dan `use crate::ui::{Action, HEIGHT, View};`.
  - Ubah visibilitas `say`, `now`, `installed`, `own_path`, dan `drain` menjadi `pub(crate)`, dan `PALETTE` menjadi `pub(crate) const`, supaya MenuBar (Task 6) bisa memakainya.

Ganti `refresh`:

```rust
/// Ask hyprix again for the windows.
fn refresh(hypr: Option<&Hypr>, state: &mut State) {
    let Some(hypr) = hypr else {
        return;
    };
    if let Some(list) = hypr.ask("j/clients").and_then(|text| Json::parse(&text)) {
        state.clients(&list);
    }
}
```

Ganti `act`:

```rust
/// Do what a press asked.
fn act(hypr: Option<&Hypr>, state: &State, action: Action) {
    let dispatch = |what: &str| {
        let done = hypr.is_some_and(|hypr| hypr.dispatch(what));
        say(&format!(
            "dispatch {what}: {}",
            if done { "ok" } else { "refused" }
        ));
    };
    match action {
        // A window of it anywhere, minimized too, is brought; focusing a
        // minimized window takes it out of special:minimized (§8.1).
        Action::Launch(at) => {
            if let Some(launcher) = state.launchers.get(at) {
                match &launcher.window {
                    Some(address) => dispatch(&format!("focuswindow address:{address}")),
                    None => dispatch(&format!("exec {}", launcher.exec)),
                }
            }
        }
        Action::Other(at) => {
            if let Some(other) = state.others.get(at) {
                dispatch(&format!("focuswindow address:{}", other.window));
            }
        }
        Action::Stash => dispatch(&format!("exec {} --stash-panel", own_path())),
        // Said for the tooltip's sake, and for a boot that finds a tile by
        // moving the pointer along the dock.
        Action::Hover(title) => {
            if !title.is_empty() {
                say(&format!("over {title}"));
            }
        }
    }
}
```

Di `run()`:

```rust
    let mut state = State::new(model::launchers(&installed()));
```

Ganti blok `let layer = LayerOptions {...}; let options = ...` dengan:

```rust
    // Anchored to the foot alone, so the compositor centres it; as wide as
    // its tiles, and set again when a program comes or goes.
    let layer = |width: u32| LayerOptions {
        output: None,
        layer: Layer::Top,
        namespace: NAMESPACE.to_owned(),
        size: (width, HEIGHT),
        anchor: Anchor::BOTTOM,
        exclusive_zone: i32::try_from(HEIGHT).unwrap_or(68) + MARGIN,
        margin: Margin {
            top: 0,
            right: 0,
            bottom: MARGIN,
            left: 0,
        },
        keyboard: KeyboardInteractivity::None,
    };
    let mut width = crate::ui::width(&state);
    let options = WindowOptions {
        title: "Yoru".to_owned(),
        app_id: NAMESPACE.to_owned(),
        size: (width, HEIGHT),
        background: Color::rgba(0, 0, 0, 0),
        clipboard: false,
        layer: Some(layer(width)),
        ..WindowOptions::default()
    };
```

Ganti juga beberapa bagian lain di `run()`:
- Pesan "up" menjadi `say(&format!("the dock is up, {w}x{h}, {} launchers", state.launchers.len()));`.
- Isi cabang `Event::Timer(timer) if timer == tick` menjadi:

```rust
                Event::Timer(timer) if timer == tick => {
                    let stashed =
                        crate::stash::folder().map_or(0, |dir| crate::stash::count(&dir));
                    if stashed != state.stashed {
                        state.stashed = stashed;
                        changed = true;
                    }
                    if let Some(tokens) = theme.poll() {
                        tokens.apply(&mut view.ui);
                        frames(hypr.as_ref(), &tokens);
                        say(&format!(
                            "dock: {}",
                            crate::theme::said(&theme.theme, &tokens)
                        ));
                        changed = true;
                    }
                }
```

  Dok tidak lagi menggambar ulang setiap detik, karena jam sudah pindah ke MenuBar.
- `act(hypr.as_ref(), &mut state, action);` menjadi `act(hypr.as_ref(), &state, action);`.
- Blok `if changed { ... }` menjadi:

```rust
        if changed {
            let wanted = crate::ui::width(&state);
            if wanted != width {
                width = wanted;
                window.set_layer(layer(width));
            }
            view.show(&state);
            if let Err(error) = window.refresh(&mut view.ui) {
                say(&format!("err: the compositor went away: {error}"));
                return 1;
            }
        }
```

**Belum diketahui:** apakah hyprix memusatkan layer yang hanya di-anchor ke `BOTTOM`, seperti aturan wlr-layer-shell. Cara memeriksa: judge Task 8 (bar harus di tengah dalam 8 px), atau log `the dock is up, <w>x68`. Bila tidak dipusatkan, kembalikan anchor `BOTTOM|LEFT|RIGHT` dan hitung `margin.left = margin.right = (lebar layar - width) / 2` dari `window.size()` pertama. Ini butuh satu `set_layer` setelah konfigurasi awal.

- [ ] **Step 6: `cli.rs` `render` dan `--render`**

```rust
/// The dock drawn at `scale`, as wide as its tiles, over the gradient's
/// middle colour, as RGBA rows.
#[must_use]
pub fn render(state: &State, scale: f32) -> Option<tole::Frame> {
    let width = crate::ui::width(state);
    let mut view = View::new(crate::look::fonts())?;
    view.ui.set_scale(scale);
    view.resize((width, HEIGHT));
    view.show(state);
    view.ui.update();
    let mut frame = tole::Frame::new((width, HEIGHT), scale, tole::Color::hex(0xe8_d5f2))?;
    let _ = view.ui.render_into(&mut frame);
    Some(frame)
}
```

Di cabang `--render`, `render(&sample(), 1600, 2.0)` menjadi `render(&sample(), 2.0)`.

- [ ] **Step 7: Tes, clippy, dan gambar**

```bash
cd /root/ferrix/src/user/apps/yoru-shell
cargo test 2>&1 | grep -E "^test result|FAILED|panicked" | head
cargo clippy --all-targets 2>&1 | grep -E "^(warning|error)" | sort | uniq -c | head
cargo run -q -- --render /tmp/a2-dock.png && ls -l /tmp/a2-dock.png
```

Expected:
- semua `test result: ok`
- clippy tanpa `error`; jumlah warning tidak lebih dari sebelum Task 1
- **belum diketahui:** set lint pedantic crate apps; `model.rs` memakai `#[expect]`, jadi ikuti pola itu bila clippy meminta
- PNG dok: tile gelap terminal dengan titik, tile biru langit browser, tile abu Settings, garis, lalu Stash

- [ ] **Step 8: Boot dan anggaran**

```bash
cd /root/ferrix
cargo xtask test-compositor --arch x86_64 --boot files 2>&1 | tail -5
cargo xtask test-compositor --arch x86_64 --boot stash 2>&1 | tail -5
cargo xtask measure-desktop --boots 2 2>&1 | tail -25
```

Expected:
- `files` dan `stash` lulus: sapuan `y = H-36` jatuh di tile (layer `H-74..H-6`, tile `H-64..H-16`), dan badge `#009199` tetap ada
- boot `yoru` **masih gagal** di judge lama (dok tidak gelap lagi); itu diperbaiki di Task 8
- measure:
  - RSS `yoru-shell` dok paling banyak 7,6 MB
  - `dock drawn` paling lambat 0,8 s setelah `graphical.target`
  - `used` paling banyak 97 MiB
- Variabel env OVMF diperlukan seperti di A1.

- [ ] **Step 9: Commit Task 3 dan 4 (setelah izin pengguna)**

```bash
cd /root/ferrix/src/user/apps
git add yoru-shell/src/model.rs yoru-shell/src/ui.rs yoru-shell/src/window.rs yoru-shell/src/cli.rs yoru-shell/ui/dock.tole
git diff --cached --stat
git commit -m "yoru-shell: the dock is a glass bar in the middle" -m "Tiles for the launchers with a dot under each that has a window on any desktop or minimized, the other programs with windows after them, a line, and the stash. A press brings the window used last or starts the program. The layer is as wide as its tiles and set again when that changes; it no longer redraws every second. Workspaces, the pomodoro, focus mode, the clock and the tools leave the dock (spec §3, §4)."
```

---

### Task 5: Model menu (murni, tanpa Wayland)

Anggaran: tidak ada perubahan pada boot.

**Files:**
- Create: `src/user/apps/yoru-shell/src/menus.rs`
- Modify: `src/user/apps/yoru-shell/src/lib.rs` (tambahkan `pub mod menus;` berurutan abjad setelah `pub mod look;`)

- [ ] **Step 1: Tulis `menus.rs` dengan tes lebih dulu.** Tes ada di bagian bawah berkas. Jalankan sekali sebelum fungsi diisi, dengan badan `todo!()`, untuk melihatnya gagal. Isi akhir berkas:

```rust
//! The menu bar's menus (spec §5): the mark's, the focused program's and
//! Window, as data. `menubar.rs` draws them and does what is picked.
//! Upgrade path: a program's own menus come with `ferrix_appmenu_v1` (A3).

use crate::json::Json;
use crate::model::Launcher;

/// The desktops Window's "Move to" offers.
pub const DESKTOPS: u32 = 4;
/// The mark at the bar's left: words until it is drawn (docs/DESKTOP.md §6.4).
pub const MARK: &str = "FX";
/// The program menu's title when no window has the focus.
pub const NOBODY: &str = "Desktop";

/// What an entry does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Does {
    /// A dispatcher, as `dispatch` takes it.
    Dispatch(String),
    /// The focused program's windows on this desktop minimized.
    Hide,
}

/// A line of a menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// Words, the keys that do the same, and what it does; `None` is shown
    /// but cannot be picked.
    Item {
        /// Its words.
        label: String,
        /// The keys, as shown.
        keys: String,
        /// What picking it does.
        does: Option<Does>,
    },
    /// A divider.
    Line,
}

/// A menu: its title in the bar and its lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    /// The title.
    pub label: String,
    /// Drawn bold, as the mark's and the program's are.
    pub bold: bool,
    /// Its lines.
    pub entries: Vec<Entry>,
}

/// The window with the focus.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Focused {
    /// Its address, `0x..`.
    pub address: String,
    /// Its class.
    pub class: String,
}

/// The focused window from `j/activewindow`; `None` when none has it.
#[must_use]
pub fn focused(json: &Json) -> Option<Focused> {
    let address = json.get("address").and_then(Json::str)?;
    if address.is_empty() {
        return None;
    }
    let class = json.get("class").and_then(Json::str).unwrap_or_default();
    Some(Focused {
        address: address.to_owned(),
        class: class.to_owned(),
    })
}

/// A program's name: its launcher's, or its class shown.
#[must_use]
pub fn app_name(class: &str, launchers: &[Launcher]) -> String {
    launchers
        .iter()
        .find(|launcher| launcher.is(class))
        .map_or_else(|| crate::model::shown_class(class), |launcher| launcher.name.clone())
}

fn item(label: &str, keys: &str, does: Option<Does>) -> Entry {
    Entry::Item {
        label: label.to_owned(),
        keys: keys.to_owned(),
        does,
    }
}

fn dispatch(what: &str) -> Option<Does> {
    Some(Does::Dispatch(what.to_owned()))
}

/// The bar's menus with `app` focused (`None`: no window has the focus);
/// `own` is this program's path, for its switcher and power menu.
#[must_use]
pub fn standard(app: Option<&str>, own: &str) -> Vec<Menu> {
    let wallpaper = item("Wallpaper…", "Super I", dispatch(&format!("exec {own} --walls")));
    let mut menus = vec![Menu {
        label: MARK.to_owned(),
        bold: true,
        entries: vec![
            item("About Ferrix", "", None),
            Entry::Line,
            item("Settings…", "", dispatch("exec settings")),
            wallpaper.clone(),
            Entry::Line,
            item("Restart or Shut Down…", "", dispatch(&format!("exec {own} --power"))),
        ],
    }];
    let Some(app) = app else {
        menus.push(Menu {
            label: NOBODY.to_owned(),
            bold: true,
            entries: vec![wallpaper],
        });
        return menus;
    };
    menus.push(Menu {
        label: app.to_owned(),
        bold: true,
        entries: vec![
            item(&format!("About {app}"), "", None),
            Entry::Line,
            item(&format!("Hide {app}"), "", Some(Does::Hide)),
            Entry::Line,
            item(&format!("Quit {app}"), "Super Q", dispatch("killactive")),
        ],
    });
    let mut window = vec![
        item("Minimize", "Super M", dispatch("minimize")),
        item("Full Screen", "Super F", dispatch("fullscreen")),
        Entry::Line,
    ];
    for n in 1..=DESKTOPS {
        window.push(item(
            &format!("Move to Desktop {n}"),
            "",
            dispatch(&format!("movetoworkspace {n}")),
        ));
    }
    window.push(Entry::Line);
    window.push(item("Close", "Super W", dispatch("killactive")));
    menus.push(Menu {
        label: "Window".to_owned(),
        bold: false,
        entries: window,
    });
    menus
}

/// What hides `class` on desktop `desktop`: each of its windows there
/// focused, then minimized. Spec §5 says `movetoworkspacesilent
/// special:minimized`, which would pass by compositor-layout's own record
/// of minimized windows, so a window hidden that way would not come back
/// when focused; `minimize` keeps it.
#[must_use]
pub fn hide(clients: &Json, class: &str, desktop: i64) -> Vec<String> {
    clients
        .items()
        .iter()
        .filter(|client| {
            client.get("class").and_then(Json::str) == Some(class)
                && client
                    .get("workspace")
                    .and_then(|workspace| workspace.get("id"))
                    .and_then(Json::int)
                    == Some(desktop)
        })
        .filter_map(|client| client.get("address").and_then(Json::str))
        .flat_map(|address| [format!("focuswindow address:{address}"), "minimize".to_owned()])
        .collect()
}

/// Where each title sits: from 10, each as wide as its words
/// (`width_of(label, bold)`) and 18 more; `(x, w)` per menu.
pub fn place(menus: &[Menu], mut width_of: impl FnMut(&str, bool) -> f32) -> Vec<(f32, f32)> {
    let mut x = 10.0;
    menus
        .iter()
        .map(|menu| {
            let w = width_of(&menu.label, menu.bold) + 18.0;
            let at = (x, w);
            x += w;
            at
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn does(menu: &Menu, label: &str) -> Option<Does> {
        menu.entries.iter().find_map(|entry| match entry {
            Entry::Item { label: l, does, .. } if l == label => does.clone(),
            _ => None,
        })
    }

    #[test]
    fn a_focused_program_has_its_menu_and_window() {
        let menus = standard(Some("Terminal"), "/bin/yoru-shell");
        let titles: Vec<&str> = menus.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(titles, ["FX", "Terminal", "Window"]);
        assert_eq!(does(&menus[1], "Quit Terminal"), Some(Does::Dispatch("killactive".to_owned())));
        assert_eq!(does(&menus[1], "Hide Terminal"), Some(Does::Hide));
        assert_eq!(does(&menus[1], "About Terminal"), None);
        assert_eq!(does(&menus[2], "Minimize"), Some(Does::Dispatch("minimize".to_owned())));
        assert_eq!(does(&menus[2], "Full Screen"), Some(Does::Dispatch("fullscreen".to_owned())));
        assert_eq!(
            does(&menus[2], "Move to Desktop 3"),
            Some(Does::Dispatch("movetoworkspace 3".to_owned()))
        );
        assert_eq!(does(&menus[2], "Close"), Some(Does::Dispatch("killactive".to_owned())));
        assert_eq!(
            does(&menus[0], "Wallpaper…"),
            Some(Does::Dispatch("exec /bin/yoru-shell --walls".to_owned()))
        );
    }

    #[test]
    fn with_nothing_focused_the_desktop_has_its_own() {
        let menus = standard(None, "/bin/yoru-shell");
        let titles: Vec<&str> = menus.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(titles, ["FX", "Desktop"]);
    }

    #[test]
    fn the_focus_and_the_names() {
        let json = Json::parse(r#"{"address":"0x5","class":"foot"}"#).expect("json");
        assert_eq!(
            focused(&json),
            Some(Focused { address: "0x5".to_owned(), class: "foot".to_owned() })
        );
        assert_eq!(focused(&Json::parse("{}").expect("json")), None);
        let launchers = crate::model::launchers(&[]);
        assert_eq!(app_name("foot", &launchers), "Terminal");
        assert_eq!(app_name("org.example.Notes", &launchers), "Notes");
    }

    #[test]
    fn hide_minimizes_the_programs_windows_on_this_desktop_only() {
        let clients = Json::parse(
            r#"[
              {"address":"0x1","class":"foot","workspace":{"id":2}},
              {"address":"0x2","class":"foot","workspace":{"id":3}},
              {"address":"0x3","class":"other","workspace":{"id":2}},
              {"address":"0x4","class":"foot","workspace":{"id":-99}}
            ]"#,
        )
        .expect("json");
        assert_eq!(hide(&clients, "foot", 2), ["focuswindow address:0x1", "minimize"]);
    }

    #[test]
    fn titles_sit_one_after_another() {
        let menus = standard(Some("Files"), "/x");
        let places = place(&menus, |label, _| label.len() as f32 * 10.0);
        assert_eq!(places, [(10.0, 38.0), (48.0, 68.0), (116.0, 78.0)]);
    }
}
```

- [ ] **Step 2: Jalankan**

```bash
cd /root/ferrix/src/user/apps/yoru-shell && cargo test --lib menus:: 2>&1 | tail -4
```

Expected: `test result: ok. 5 passed`. Bila sebelumnya diisi `todo!()`, tes akan panic dulu, baru lulus setelah badan diisi.

- [ ] **Step 3: Commit (setelah izin pengguna)**

```bash
cd /root/ferrix/src/user/apps
git add yoru-shell/src/menus.rs yoru-shell/src/lib.rs
git diff --cached --stat
git commit -m "yoru-shell: the menu bar's standard menus, as data" -m "The mark's menu, the focused program's (About, Hide, Quit) and Window (Minimize, Full Screen, Move to Desktop 1-4, Close). Hide minimizes the program's windows on the desktop shown, so they come back when focused, rather than moving them past the compositor's record."
```

---

### Task 6: MenuBar

Anggaran (boot kedua): RSS total proses `yoru-shell` yang memuat dok dan MenuBar paling banyak 12 MB (sebelumnya dok 7,6 MB ditambah dua sidebar 10,4 MB). `dock drawn` tetap paling lambat 0,8 s setelah `graphical.target`.

**Files:**
- Create: `src/user/apps/yoru-shell/ui/menubar.tole`
- Create: `src/user/apps/yoru-shell/src/menubar.rs`
- Modify: `src/user/apps/yoru-shell/src/lib.rs` (`pub mod menubar;` setelah `pub mod look;`, sebelum `pub mod menus;`)
- Modify: `src/user/apps/yoru-shell/src/window.rs` (menyalakan MenuBar)
- Modify: `src/user/apps/yoru-shell/src/cli.rs` (`--menubar`, `--render-menubar`)

- [ ] **Step 1: `ui/menubar.tole`**

```
// The menu bar (spec §5): a strip of glass across the top of the screen,
// the mark and the focused program's menus at the left, the search and the
// clock at the right; an open menu drops below its title. Rust measures
// the titles, says which is open and does what is picked
// (`src/menubar.rs`).
import Fx

struct TopMenu {
    label: string
    bold: bool
    on: bool
    x: real
    w: real
}

struct MenuLine {
    label: string
    keys: string
    line: bool
    enabled: bool
}

export component MenuBar : Item {
    id: bar
    property list<TopMenu> menus
    property list<MenuLine> lines
    property bool open
    property real dropX
    property string clock
    property string searchIcon

    signal pick(int index)
    signal choose(int index)
    signal dismiss()
    signal search()

    Rectangle {
        x: 0; y: 0
        width: bar.width
        height: 24
        color: alpha(Nocturne.surface, 0.8)
        Repeater {
            model: bar.menus
            Rectangle {
                x: modelData.x; y: 2
                width: modelData.w; height: 20
                radius: 5
                color: modelData.on ? alpha(Nocturne.text, 0.1) : "transparent"
                Text {
                    anchors.centerIn: parent
                    visible: modelData.bold
                    text: modelData.label
                    font: Nocturne.sans(13, 700)
                    color: Nocturne.text
                }
                Text {
                    anchors.centerIn: parent
                    visible: !modelData.bold
                    text: modelData.label
                    font: Nocturne.sans(13, 500)
                    color: Nocturne.text
                }
                MouseArea {
                    anchors.fill: parent
                    onClicked: bar.pick(index)
                }
            }
        }
        Text {
            id: clockText
            x: bar.width - clockText.implicitWidth - 12
            y: 4
            text: bar.clock
            font: Nocturne.sans(13, 500)
            color: Nocturne.text
        }
        Text {
            id: searchText
            x: clockText.x - 30
            y: 4
            text: bar.searchIcon
            font: Nocturne.icon(15)
            color: Nocturne.text
        }
        MouseArea {
            x: searchText.x - 6; y: 0
            width: 28; height: 24
            onClicked: bar.search()
        }
    }

    // Below the strip while a menu is open: a press there closes it.
    MouseArea {
        visible: bar.open
        x: 0; y: 24
        width: bar.width
        height: bar.height - 24
        onClicked: bar.dismiss()
    }

    Rectangle {
        visible: bar.open
        x: bar.dropX; y: 26
        width: 260
        height: bar.height - 30
        radius: 8
        color: Nocturne.surface
        border.width: 1
        border.color: alpha(Nocturne.text, 0.14)
        Column {
            x: 5; y: 5
            width: 250
            Repeater {
                model: bar.lines
                Item {
                    width: 250
                    height: modelData.line ? 9 : 24
                    Rectangle {
                        visible: modelData.line
                        x: 6; y: 4
                        width: 238; height: 1
                        color: Nocturne.divider
                    }
                    Rectangle {
                        visible: !modelData.line
                        anchors.fill: parent
                        radius: 4
                        color: lineArea.containsMouse && modelData.enabled ? Nocturne.accent : "transparent"
                        Text {
                            x: 10; y: 4
                            text: modelData.label
                            font: Nocturne.sans(13, 500)
                            color: !modelData.enabled ? Nocturne.n500 : lineArea.containsMouse ? "#ffffff" : Nocturne.text
                        }
                        Text {
                            id: keysText
                            x: 240 - keysText.implicitWidth; y: 4
                            text: modelData.keys
                            font: Nocturne.sans(12, 500)
                            color: lineArea.containsMouse && modelData.enabled ? "#ffffff" : Nocturne.n500
                        }
                        MouseArea {
                            id: lineArea
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: bar.choose(index)
                        }
                    }
                }
            }
        }
    }
}
```

**Belum diketahui:**
- Apakah `Text` boleh merujuk `implicitWidth`-nya sendiri lewat `id` di `x`, dan apakah `MouseArea` yang `visible: false` tidak menerima tekanan.
- Cara memeriksa: tes Step 4.
- Bila `x` yang merujuk dirinya sendiri ditolak, Rust menghitung `clockX` dengan `ui.text_width` dan mengirimnya sebagai `property real clockX`.

- [ ] **Step 2: `src/menubar.rs`**

```rust
//! The menu bar (spec §5): a strip of glass across the top of the screen,
//! drawn by `ui/menubar.tole`. It follows the focus from hyprix's events,
//! shows the standard menus for the focused program (`menus.rs`), the
//! search and the time, and does what is picked. It runs on a thread of
//! the dock's program (spec §2), or alone with `--menubar`.

use std::os::fd::AsRawFd as _;
use std::rc::Rc;
use std::time::Duration;

use compositor_toolkit::Event;
use tole::window::{
    Anchor, KeyboardInteractivity, Layer, LayerOptions, Margin, Window, WindowOptions,
};
use tole::{Color, Element as _, Ui};

use crate::hypr::{self, Hypr};
use crate::json::Json;
use crate::menus::{self, Does, Entry, Focused, Menu};
use crate::ui::{MenuBar, MenuBarSignal, MenuLine, TopMenu};
use crate::window::say;

/// The surface's namespace: Yoru's theme blurs behind every `yoru-` one.
pub const NAMESPACE: &str = "yoru-menubar";
/// The strip's height, which windows keep clear of.
pub const HEIGHT: u32 = 24;
/// The words' fonts, as `Nocturne.sans` has them.
const SANS: &str = "Inter Variable, Inter, sans-serif";

/// A string as the tree holds it.
fn s(value: impl AsRef<str>) -> Rc<str> {
    Rc::from(value.as_ref())
}

/// An open menu's height below the strip, its lines and its edges.
#[must_use]
pub fn drop_height(entries: &[Entry]) -> u32 {
    10 + entries
        .iter()
        .map(|entry| if matches!(entry, Entry::Line) { 9 } else { 24 })
        .sum::<u32>()
}

/// The surface's height with `entries` dropped.
#[must_use]
pub fn open_height(entries: &[Entry]) -> u32 {
    26 + drop_height(entries) + 4
}

/// The layer `height` tall; windows keep clear of the strip alone.
fn layer(height: u32) -> LayerOptions {
    LayerOptions {
        output: None,
        layer: Layer::Top,
        namespace: NAMESPACE.to_owned(),
        size: (0, height),
        anchor: Anchor::TOP.with(Anchor::LEFT).with(Anchor::RIGHT),
        exclusive_zone: i32::try_from(HEIGHT).unwrap_or(24),
        margin: Margin {
            top: 0,
            right: 0,
            bottom: 0,
            left: 0,
        },
        keyboard: KeyboardInteractivity::None,
    }
}

/// What a press in the bar asks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Menu `n` dropped.
    Open(usize),
    /// The open menu closed.
    Close,
    /// An entry's deed.
    Do(Does),
    /// The search.
    Search,
}

/// The tree and what the program keeps of it.
pub struct View {
    /// The tree.
    pub ui: Ui,
    /// The bar.
    pub bar: MenuBar,
    menus: Vec<Menu>,
    open: Option<usize>,
    places: Vec<(f32, f32)>,
}

impl std::fmt::Debug for View {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("View").finish_non_exhaustive()
    }
}

impl View {
    /// The tree, drawn with `fonts`; `None` when the markup's items could
    /// not be made.
    #[must_use]
    pub fn new(fonts: tole::Fonts) -> Option<Self> {
        let mut ui = Ui::new(fonts);
        let bar = MenuBar::new(&mut ui, None)?;
        bar.search_icon
            .set(&mut ui, s(crate::look::icon("magnifying-glass")));
        Some(View {
            ui,
            bar,
            menus: Vec::new(),
            open: None,
            places: Vec::new(),
        })
    }

    /// The bar's size in logical pixels.
    pub fn resize(&mut self, (width, height): (u32, u32)) {
        let common = self.bar.common();
        common.width.set(&mut self.ui, width as f32);
        common.height.set(&mut self.ui, height as f32);
    }

    /// Show `menus`, `open` the one dropped, and `clock`.
    pub fn show(&mut self, menus: Vec<Menu>, open: Option<usize>, clock: &str) {
        let ui = &mut self.ui;
        let regular = tole::Font::new(SANS, 13.0, 500);
        let bold = tole::Font::new(SANS, 13.0, 700);
        self.places = menus::place(&menus, |label, heavy| {
            ui.text_width(if heavy { &bold } else { &regular }, label)
        });
        let tops = menus
            .iter()
            .zip(&self.places)
            .enumerate()
            .map(|(i, (menu, &(x, w)))| TopMenu {
                label: s(&menu.label),
                bold: menu.bold,
                on: open == Some(i),
                x,
                w,
            })
            .collect::<Vec<_>>();
        self.bar.menus.set(ui, tops);
        let lines = open
            .and_then(|i| menus.get(i))
            .map(|menu| {
                menu.entries
                    .iter()
                    .map(|entry| match entry {
                        Entry::Line => MenuLine {
                            label: s(""),
                            keys: s(""),
                            line: true,
                            enabled: false,
                        },
                        Entry::Item { label, keys, does } => MenuLine {
                            label: s(label),
                            keys: s(keys),
                            line: false,
                            enabled: does.is_some(),
                        },
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.bar.lines.set(ui, lines);
        self.bar.open.set(ui, open.is_some());
        let drop_x = open.and_then(|i| self.places.get(i)).map_or(0.0, |place| place.0);
        self.bar.drop_x.set(ui, drop_x);
        self.bar.clock.set(ui, s(clock));
        self.menus = menus;
        self.open = open;
    }

    /// What the tree asked since the last call.
    pub fn asked(&mut self) -> Vec<Asked> {
        let emitted = self.ui.take_signals();
        let mut out = Vec::new();
        for each in &emitted {
            let Some(signal) = self.bar.signal(each) else {
                continue;
            };
            match signal {
                MenuBarSignal::Pick(i) => out.push(match usize::try_from(i) {
                    Ok(i) if Some(i) != self.open => Asked::Open(i),
                    _ => Asked::Close,
                }),
                MenuBarSignal::Choose(i) => {
                    let does = usize::try_from(i).ok().and_then(|i| {
                        let menu = self.open.and_then(|m| self.menus.get(m))?;
                        match menu.entries.get(i)? {
                            Entry::Item { does, .. } => does.clone(),
                            Entry::Line => None,
                        }
                    });
                    out.extend(does.map(Asked::Do));
                }
                MenuBarSignal::Dismiss => out.push(Asked::Close),
                MenuBarSignal::Search => out.push(Asked::Search),
            }
        }
        out
    }
}

/// The window with the focus, asked of hyprix.
fn ask_focused(hypr: Option<&Hypr>) -> Option<Focused> {
    hypr?
        .ask("j/activewindow")
        .and_then(|text| Json::parse(&text))
        .and_then(|json| menus::focused(&json))
}

/// The time as the bar shows it.
fn clock_now() -> String {
    let clock = crate::model::Clock::at(crate::window::now());
    format!("{}  {}", clock.date, clock.time)
}

/// Dispatch each of `lines`, said.
fn dispatch_all(hypr: &Hypr, lines: &[String]) {
    for what in lines {
        let done = hypr.dispatch(what);
        say(&format!(
            "menu: dispatch {what}: {}",
            if done { "ok" } else { "refused" }
        ));
    }
}

/// Do an entry's deed. A press on the bar may have moved the focus, so the
/// focused window is focused again first.
fn done(hypr: Option<&Hypr>, focused: Option<&Focused>, does: Does) {
    let Some(hypr) = hypr else { return };
    let again = focused.map(|f| format!("focuswindow address:{}", f.address));
    let lines: Vec<String> = match does {
        Does::Dispatch(what) => again.into_iter().chain(std::iter::once(what)).collect(),
        Does::Hide => {
            let desktop = hypr
                .ask("j/activeworkspace")
                .and_then(|text| Json::parse(&text))
                .and_then(|json| json.get("id").and_then(Json::int))
                .unwrap_or(1);
            let clients = hypr
                .ask("j/clients")
                .and_then(|text| Json::parse(&text))
                .unwrap_or(Json::Null);
            focused.map_or_else(Vec::new, |f| menus::hide(&clients, &f.class, desktop))
        }
    };
    dispatch_all(hypr, &lines);
}

/// The search: fuzzel in Yoru's look, as SUPER Space starts it.
fn search(hypr: Option<&Hypr>) {
    let Some(hypr) = hypr else { return };
    let what = if std::path::Path::new(crate::window::PALETTE).exists() {
        format!("exec /bin/fuzzel --config {}", crate::window::PALETTE)
    } else {
        "exec /bin/fuzzel".to_owned()
    };
    dispatch_all(hypr, &[what]);
}

/// Run the menu bar until the compositor goes away; the exit status.
#[must_use]
pub fn run() -> i32 {
    let hypr = Hypr::from_env();
    let launchers = crate::model::launchers(&crate::window::installed());
    let own = crate::window::own_path();
    let Some(mut view) = View::new(crate::look::fonts()) else {
        say("err: the menu bar's tree could not be made");
        return 1;
    };
    let mut theme = crate::theme::Live::new();
    theme.tokens.apply(&mut view.ui);
    let standard = |focused: Option<&Focused>| {
        let name = focused.map(|f| menus::app_name(&f.class, &launchers));
        menus::standard(name.as_deref(), &own)
    };
    let mut focused = ask_focused(hypr.as_ref());
    let mut clock = clock_now();
    let mut open: Option<usize> = None;
    let mut height = HEIGHT;
    view.show(standard(focused.as_ref()), open, &clock);
    let options = WindowOptions {
        title: "Yoru menu bar".to_owned(),
        app_id: NAMESPACE.to_owned(),
        size: (1280, HEIGHT),
        background: Color::rgba(0, 0, 0, 0),
        clipboard: false,
        layer: Some(layer(HEIGHT)),
        ..WindowOptions::default()
    };
    let mut window = match Window::open(view.bar.common(), options) {
        Ok(window) => window,
        Err(error) => {
            say(&format!("err: no compositor to put the menu bar on: {error}"));
            return 1;
        }
    };
    let mut events = hypr.as_ref().and_then(Hypr::events);
    let watch = events
        .as_ref()
        .map(|stream| window.client().watch_fd(stream.as_raw_fd()));
    let tick = window
        .client()
        .add_timer(Duration::from_secs(1), Some(Duration::from_secs(1)));
    let mut pending = String::new();
    let mut up = false;
    loop {
        let happened = match window.dispatch(&mut view.ui, None) {
            Ok(happened) => happened,
            Err(error) => {
                say(&format!("err: the compositor went away: {error}"));
                return 1;
            }
        };
        if !up && let Some((w, h)) = window.size() {
            up = true;
            say(&format!("the menu bar is up, {w}x{h}"));
        }
        let mut changed = false;
        for event in happened {
            match event {
                Event::Closed(_) | Event::CloseRequested(_) => return 0,
                Event::Timer(timer) if timer == tick => {
                    let now = clock_now();
                    if now != clock {
                        clock = now;
                        changed = true;
                    }
                    if let Some(tokens) = theme.poll() {
                        tokens.apply(&mut view.ui);
                        changed = true;
                    }
                }
                Event::Readable(id) if Some(id) == watch => {
                    if let Some(stream) = events.as_mut()
                        && !crate::window::drain(stream, &mut pending)
                    {
                        say("menu bar: hyprix's event socket closed");
                        watch
                            .into_iter()
                            .for_each(|w| window.client().unwatch_fd(w));
                        events = None;
                    }
                    let mut moved = false;
                    while let Some(end) = pending.find('\n') {
                        let line: String = pending.drain(..=end).collect();
                        moved |= hypr::wanted(line.trim_end());
                    }
                    if moved {
                        let now = ask_focused(hypr.as_ref());
                        if now != focused {
                            focused = now;
                            open = None;
                            changed = true;
                        }
                    }
                }
                _ => {}
            }
        }
        for asked in view.asked() {
            match asked {
                Asked::Open(i) => open = Some(i),
                Asked::Close => open = None,
                Asked::Search => {
                    open = None;
                    search(hypr.as_ref());
                }
                Asked::Do(does) => {
                    open = None;
                    done(hypr.as_ref(), focused.as_ref(), does);
                }
            }
            changed = true;
        }
        if changed {
            let menus = standard(focused.as_ref());
            let wanted = open
                .and_then(|i| menus.get(i))
                .map_or(HEIGHT, |menu| open_height(&menu.entries));
            if wanted != height {
                height = wanted;
                window.set_layer(layer(height));
            }
            view.show(menus, open, &clock);
            if let Err(error) = window.refresh(&mut view.ui) {
                say(&format!("err: the compositor went away: {error}"));
                return 1;
            }
        }
    }
}

/// The menu bar on a thread of the dock's program (spec §2: one program,
/// several surfaces), with its own tree and its own connection to the
/// compositor. Upgrade path: one Wayland connection for both surfaces,
/// should tole's `Window` learn to share a client.
pub fn start() {
    let spawned = std::thread::Builder::new()
        .name("menubar".to_owned())
        .spawn(|| {
            let code = run();
            say(&format!("the menu bar stopped: {code}"));
        });
    if let Err(error) = spawned {
        say(&format!("err: the menu bar's thread: {error}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_draws_glass_and_words() {
        let frame = crate::cli::render_menubar(1024, None, 1.0).expect("drawn");
        let pixmap = frame.pixmap();
        assert_eq!((pixmap.width(), pixmap.height()), (1024, HEIGHT));
        let at = |x: u32, y: u32| {
            let pixel = pixmap.pixel(x, y).expect("inside").demultiply();
            (pixel.red(), pixel.green(), pixel.blue())
        };
        let (r, g, b) = at(512, 3);
        let (lo, hi) = (r.min(g).min(b), r.max(g).max(b));
        assert!(lo >= 0xd8 && hi - lo <= 0x18, "the glass: {r} {g} {b}");
        let dark = |xs: std::ops::Range<u32>| {
            xs.flat_map(|x| (4..20).map(move |y| (x, y)))
                .filter(|&(x, y)| {
                    let (r, g, b) = at(x, y);
                    u32::from(r) + u32::from(g) + u32::from(b) < 0x150
                })
                .count()
        };
        assert!(dark(4..160) >= 30, "the menus' words at the left");
        assert!(dark(1024 - 160..1024 - 4) >= 30, "the clock at the right");
    }

    #[test]
    fn an_open_menu_drops_white_below_its_title() {
        let menus = menus::standard(Some("Terminal"), "/bin/yoru-shell");
        let frame = crate::cli::render_menubar(1024, Some(2), 1.0).expect("drawn");
        let pixmap = frame.pixmap();
        assert_eq!(pixmap.height(), open_height(&menus[2].entries));
        let white = (0..1024)
            .flat_map(|x| (40..120).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                let pixel = pixmap.pixel(x, y).expect("inside").demultiply();
                pixel.red() >= 0xf5 && pixel.green() >= 0xf5 && pixel.blue() >= 0xf5
            })
            .count();
        assert!(white > 5000, "the dropped menu: {white}");
    }

    #[test]
    fn a_press_opens_a_menu_and_a_pick_does_its_entry() {
        let menus = menus::standard(Some("Terminal"), "/bin/yoru-shell");
        let mut view = View::new(crate::look::fonts()).expect("tree");
        view.resize((1024, HEIGHT));
        view.show(menus.clone(), None, "Thu 12 Nov  21:40");
        view.ui.update();
        let (x, w) = view.places[2];
        view.ui.pointer_move(x + w / 2.0, 12.0);
        view.ui.pointer_press();
        view.ui.pointer_release();
        assert_eq!(view.asked(), vec![Asked::Open(2)]);
        view.resize((1024, open_height(&menus[2].entries)));
        view.show(menus.clone(), Some(2), "Thu 12 Nov  21:40");
        view.ui.update();
        // Minimize, the first line: 26 down, 5 in, half a line.
        view.ui.pointer_move(x + 40.0, 26.0 + 5.0 + 12.0);
        view.ui.pointer_press();
        view.ui.pointer_release();
        assert_eq!(
            view.asked(),
            vec![Asked::Do(Does::Dispatch("minimize".to_owned()))]
        );
    }
}
```

- [ ] **Step 3: `cli.rs`: renderer dan flag.** Tambahkan setelah `render()`:

```rust
/// The menu bar drawn `width` wide at `scale`, the Window menu's or
/// another's dropped (`open`), with the terminal focused, over the
/// gradient's middle colour.
#[must_use]
pub fn render_menubar(width: u32, open: Option<usize>, scale: f32) -> Option<tole::Frame> {
    let mut view = crate::menubar::View::new(crate::look::fonts())?;
    view.ui.set_scale(scale);
    let menus = crate::menus::standard(Some("Terminal"), "/bin/yoru-shell");
    let height = open
        .and_then(|i| menus.get(i))
        .map_or(crate::menubar::HEIGHT, |menu| crate::menubar::open_height(&menu.entries));
    view.resize((width, height));
    view.show(menus, open, "Thu 12 Nov  21:40");
    view.ui.update();
    let mut frame = tole::Frame::new((width, height), scale, tole::Color::hex(0xe8_d5f2))?;
    let _ = view.ui.render_into(&mut frame);
    Some(frame)
}
```

Di `main()`, sebelum cabang `--render`:

```rust
    if arguments.iter().any(|word| word == "--menubar") {
        return if crate::menubar::run() == 0 {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }
    if let Some(at) = arguments.iter().position(|word| word == "--render-menubar") {
        let Some(path) = arguments.get(at + 1) else {
            eprintln!("yoru-shell: --render-menubar needs a file");
            return ExitCode::FAILURE;
        };
        let open = arguments
            .iter()
            .position(|word| word == "--open")
            .and_then(|at| arguments.get(at + 1))
            .and_then(|n| n.parse().ok());
        let Some(frame) = render_menubar(1600, open, 2.0) else {
            eprintln!("yoru-shell: the menu bar could not be drawn");
            return ExitCode::FAILURE;
        };
        let pixmap = frame.pixmap();
        let png = tole::png::encode(pixmap.width(), pixmap.height(), pixmap.data());
        return match std::fs::write(path, png) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("yoru-shell: {path}: {error}");
                ExitCode::FAILURE
            }
        };
    }
```

- [ ] **Step 4: Jalankan tes MenuBar**

```bash
cd /root/ferrix/src/user/apps/yoru-shell && cargo test --lib menubar:: 2>&1 | tail -6
```

Expected: `test result: ok. 3 passed`. Bila koordinat klik di tes ketiga meleset satu baris, cetak `view.places` dan ukuran baris. Perbaiki markup (tinggi baris 24, Column `y: 5`), bukan tesnya.

- [ ] **Step 5: Nyalakan MenuBar dari dok.** Di `window.rs` `run()`, di dalam `if !up && let Some((w, h)) = window.size() { ... }`, setelah `say(... "the dock is up" ...)`, tambahkan:

```rust
            // The menu bar after the dock, so the dock is drawn first.
            crate::menubar::start();
```

- [ ] **Step 6: Tes penuh, gambar, boot, dan anggaran**

```bash
cd /root/ferrix/src/user/apps/yoru-shell
cargo test 2>&1 | grep -E "^test result|FAILED|panicked" | head
cargo run -q -- --render-menubar /tmp/a2-menubar.png --open 2 && ls -l /tmp/a2-menubar.png
cd /root/ferrix
cargo xtask test-compositor --arch x86_64 --boot files 2>&1 | tail -5
cargo xtask measure-desktop --boots 2 2>&1 | tail -25
```

Expected:
- tes hijau
- PNG menampilkan strip kaca dengan **FX**, **Terminal**, Window, kaca pembesar, dan jam; menu Window turun putih
- boot `files` masih lulus
- log memuat `yoru-shell: the menu bar is up, 1024x24`
- measure: RSS total `yoru-shell` (satu proses) paling banyak 12 MB, dan `dock drawn` paling lambat 0,8 s setelah `graphical.target`

Pada tahap ini sidebar masih menyala, jadi `used` boleh naik sedikit. Batas `used` paling banyak 97 MiB ditegakkan di Task 7.

**Belum diketahui:** apakah dua koneksi Wayland di satu proses (`Window::open` memanggil `Client::connect()`) berjalan aman dengan state global tole atau `compositor_toolkit`.
- Cara memeriksa: boot `files` dan log tanpa `err:`.
- Bila ada masalah, pakai **[KEPUTUSAN] 1** cadangan: hapus `crate::menubar::start();` lalu tambahkan `exec-once = /{DOCK} --menubar` di Task 7.

- [ ] **Step 7: Commit (setelah izin pengguna)**

```bash
cd /root/ferrix/src/user/apps
git add yoru-shell/ui/menubar.tole yoru-shell/src/menubar.rs yoru-shell/src/lib.rs yoru-shell/src/window.rs yoru-shell/src/cli.rs
git diff --cached --stat
git commit -m "yoru-shell: a menu bar across the top" -m "A strip of glass 24 pixels high that windows keep clear of: the FX mark, the focused program's menu and Window at the left, the search and the clock at the right. A menu drops below its title and the surface grows to hold it; a press below the strip closes it. It runs on a thread of the dock's program with its own connection, started once the dock is up; --menubar runs it alone."
```

---

### Task 7: Konfigurasi: sidebar berhenti, SUPER W, fuzzel terang

Anggaran (boot kedua):
- `used` paling banyak 97 MiB
- RSS total `yoru-shell` paling banyak 12 MB (sebelumnya 18,0 MB untuk tiga proses)
- `hyprix` paling banyak 41 MB
- `dock drawn` paling lambat 0,8 s setelah `graphical.target`

**Files:**
- Modify: `tools/common/xtask/src/compositor/theme.rs`
- Modify: `tools/common/xtask/src/compositor/quick.rs`, `weather.rs`, `sidebars.rs`
- Modify: `tools/common/xtask/src/compositor/run.rs`
- Modify: `src/user/system/linux/compositor/hyprix/themes/yoru-keys.conf`
- Modify: `src/user/system/linux/compositor/hyprix/themes/yoru-fuzzel.ini`

- [ ] **Step 1: Tes yang gagal** di `theme.rs` (xtask), di dalam `yoru_comes_first_with_its_keys`, setelah `assert!(config.contains("exec-once = /bin/yoru-shell"), "the dock");`:

```rust
        // The sidebars left the desktop with A2 (spec §3): their boots start
        // them themselves.
        assert!(!config.contains("--sidebar"), "no sidebars: {config}");
```

- [ ] **Step 2: Jalankan, pastikan gagal**

```bash
cd /root/ferrix && cargo test -p xtask yoru_comes_first_with_its_keys 2>&1 | tail -4
```

Expected: `FAILED`, `no sidebars`. **Belum diketahui:** nama paket xtask di `cargo test -p`. Cara memeriksa: `grep -m1 '^name' tools/common/xtask/Cargo.toml`.

- [ ] **Step 3: Implementasi theme.rs.** Ganti baris 169-175:

```rust
    // Yoru's dock, which starts its menu bar, unless the configuration
    // starts a bar of its own.
    if theme == Theme::Yoru && !starts_a_bar(&config) {
        head.push_str(&format!("exec-once = /{DOCK}\n"));
    }
```

- [ ] **Step 4: Boot yang masih menguji sidebar menyalakannya sendiri.**
  - `quick.rs:63`: `let (config, mut carried)` menjadi `let (mut config, mut carried)`, lalu tambahkan sesudahnya `config.push_str(&format!("exec-once = /{} --sidebar right\n", super::theme::DOCK));`.
  - `weather.rs`, setelah baris 64: `config.push_str(&format!("exec-once = /{} --sidebar left\n", super::theme::DOCK));`.
  - `sidebars.rs`, setelah baris 62: kedua baris (`left` lalu `right`).

**Belum diketahui:** sisi sidebar yang dibaca `quick` dan `weather`. Cara memeriksa: `grep -n "sidebar\|left\|right" tools/common/xtask/src/compositor/quick.rs weather.rs | head`. Pasang sisi yang dicari boot itu. Bila ragu, pasang keduanya.

- [ ] **Step 5: SUPER W.** Di `yoru-keys.conf`, tambahkan setelah `bind = SUPER, M, minimize` (baris 33):

```
bind = SUPER, W, killactive
```

Di `run.rs` baris 66, hapus `bind = SUPER, W, exec, /bin/hyprctl activewindow` (**[KEPUTUSAN] 6**). Sebelum menghapus, periksa:

```bash
cd /root/ferrix && grep -rn "hyprctl activewindow\|SUPER W\|\"w\"" tools/common/xtask/src/compositor/run.rs tools/common/xtask/src/compositor/yoru.rs tools/common/xtask/src/compositor/files.rs
```

Expected: hanya baris bind itu sendiri yang muncul. Bila ada boot bertema Yoru yang menekan SUPER W dan membaca keluaran `hyprctl`, berhenti dan tanyakan pengguna.

- [ ] **Step 6: `yoru-fuzzel.ini` terang.** Ganti blok `[colors]`, dan ubah komentar kepala `on the surface` menjadi `on the light glass surface`:

```
[colors]
background=ffffffff
text=3a3a40ff
prompt=0a84ffff
placeholder=8e8e96ff
input=1d1d1fff
match=0a84ffff
selection=0a84ff29
selection-text=1d1d1fff
selection-match=0a6fdcff
counter=8e8e96ff
border=00000024
```

- [ ] **Step 7: Tes dan boot**

```bash
cd /root/ferrix
cargo test -p xtask theme 2>&1 | grep -E "^test result|FAILED"
for boot in quick weather sidebars; do cargo xtask test-compositor --arch x86_64 --boot $boot 2>&1 | tail -3; done
cargo xtask measure-desktop --boots 2 2>&1 | tail -25
```

Expected:
- tes theme ok
- `quick`, `weather`, dan `sidebars` lulus
- measure (boot kedua):
  - tidak ada baris RSS sidebar
  - RSS total `yoru-shell` paling banyak 12 MB
  - `used` paling banyak 97 MiB (diharapkan turun sekitar 10 MiB)
  - `hyprix` paling banyak 41 MB
  - `dock drawn` paling lambat 0,8 s setelah `graphical.target`

Boot `yoru-launcher` masih gagal di judge lama (kartu gelap). Itu diperbaiki di Task 8.

- [ ] **Step 8: Commit (setelah izin pengguna)**

```bash
cd /root/ferrix
git add tools/common/xtask/src/compositor/theme.rs tools/common/xtask/src/compositor/quick.rs tools/common/xtask/src/compositor/weather.rs tools/common/xtask/src/compositor/sidebars.rs tools/common/xtask/src/compositor/run.rs src/user/system/linux/compositor/hyprix/themes/yoru-keys.conf src/user/system/linux/compositor/hyprix/themes/yoru-fuzzel.ini
git diff --cached --stat
git commit -m "yoru: no sidebars on the desktop, Super W closes, a light launcher" -m "The theme starts the dock alone; it brings its menu bar. The sidebars leave the desktop (spec §3) and the boots that test them start them themselves. Super W closes the focused window as the menu bar says, and the run config's debug bind for it goes. fuzzel wears the light glass."
```

---

### Task 8: Judge yang lebih ketat untuk dok, MenuBar, dan launcher

Anggaran: tidak berubah dari Task 7.

**Files:**
- Modify: `tools/common/xtask/src/compositor/yoru.rs`

- [ ] **Step 1: Tes sintetis yang gagal.** Ganti `mod tests` di `yoru.rs` dengan:

```rust
#[cfg(test)]
mod tests {
    use super::{SCREEN, judge, judge_launcher};
    use crate::display::Image;

    /// The wallpaper's corner colour, sand.
    const CORNER: [u8; 3] = [0xf6, 0xe3, 0xc9];

    fn paint(
        pixels: &mut [u8],
        xs: core::ops::Range<usize>,
        ys: core::ops::Range<usize>,
        colour: [u8; 3],
    ) {
        for y in ys {
            for x in xs.clone() {
                let at = (y * SCREEN.0 + x) * 3;
                pixels[at..at + 3].copy_from_slice(&colour);
            }
        }
    }

    /// Yoru's desktop as A2 draws it: a light bar with its close button,
    /// the wallpaper's blue near the top, the menu bar with words at both
    /// ends, and the dock, `dock_w` wide and centred, the terminal's tile
    /// first with its dot.
    fn desktop(dock_w: usize, menu_bar: bool) -> Vec<u8> {
        let (w, h) = SCREEN;
        let mut pixels = CORNER.repeat(w * h);
        paint(&mut pixels, 100..700, 300..330, [0xf3, 0xf3, 0xf5]);
        paint(&mut pixels, 110..120, 310..320, [0xff, 0x5f, 0x57]);
        paint(&mut pixels, 100..101, 100..101, [0xc6, 0xcf, 0xf4]);
        if menu_bar {
            paint(&mut pixels, 0..w, 0..24, [0xf0, 0xf0, 0xf2]);
            paint(&mut pixels, 10..40, 8..16, [0x1d, 0x1d, 0x1f]);
            paint(&mut pixels, w - 60..w - 20, 8..16, [0x1d, 0x1d, 0x1f]);
        }
        let left = (w - dock_w) / 2;
        paint(&mut pixels, left..left + dock_w, h - 74..h - 6, [0xf4, 0xf4, 0xf6]);
        paint(&mut pixels, left + 10..left + 58, h - 64..h - 16, [0x2b, 0x2b, 0x30]);
        paint(&mut pixels, left + 32..left + 36, h - 13..h - 9, [0x4a, 0x4d, 0x55]);
        pixels
    }

    fn screen(pixels: Vec<u8>) -> Image {
        Image {
            width: SCREEN.0,
            height: SCREEN.1,
            pixels,
        }
    }

    /// A screen of the wallpaper alone is not Yoru's desktop: no bars.
    #[test]
    fn the_wallpaper_alone_has_no_bars() {
        let said = judge(&screen(CORNER.repeat(SCREEN.0 * SCREEN.1))).expect_err("no bars");
        assert!(said.contains("surface"), "{said}");
    }

    #[test]
    fn the_a2_desktop_is() {
        assert_eq!(judge(&screen(desktop(301, true))).map(|_| ()), Ok(()));
    }

    #[test]
    fn no_menu_bar_is_not() {
        let said = judge(&screen(desktop(301, false))).expect_err("no menu bar");
        assert!(said.contains("menu bar"), "{said}");
    }

    #[test]
    fn a_dock_across_the_screen_is_not() {
        let said = judge(&screen(desktop(SCREEN.0 - 24, true))).expect_err("too wide");
        assert!(said.contains("dock"), "{said}");
    }

    #[test]
    fn the_old_dark_dock_is_not() {
        let mut pixels = desktop(301, true);
        let (w, h) = SCREEN;
        paint(&mut pixels, 12..w - 12, h - 66..h - 6, [0x3e, 0x45, 0x52]);
        let said = judge(&screen(pixels)).expect_err("dark dock");
        assert!(said.contains("dock"), "{said}");
    }

    #[test]
    fn the_launcher_is_a_white_card_in_the_middle() {
        let mut pixels = desktop(301, true);
        paint(&mut pixels, 212..812, 174..400, [0xff, 0xff, 0xff]);
        assert_eq!(judge_launcher(&screen(pixels)).map(|_| ()), Ok(()));
        let mut dark = desktop(301, true);
        paint(&mut dark, 212..812, 174..400, [0x23, 0x25, 0x32]);
        assert!(judge_launcher(&screen(dark)).is_err(), "the old dark card");
    }
}
```

- [ ] **Step 2: Jalankan, pastikan gagal**

```bash
cd /root/ferrix && cargo test -p xtask compositor::yoru 2>&1 | grep -E "test |result"
```

Expected:
- `the_a2_desktop_is` FAILED: judge lama mencari dok gelap
- `no_menu_bar_is_not`, `a_dock_across_the_screen_is_not`, dan `the_launcher_is_a_white_card_in_the_middle` juga FAILED

- [ ] **Step 3: Implementasi.** Ganti doc dan isi `judge` mulai dari `// The dock's bar is translucent` (baris 249) sampai `Ok(...)`:

```rust
    // The menu bar: glass at the top middle, where no menu reaches, and
    // words at both ends (the menus at the left, the clock at the right).
    let glass = |[r, g, b]: [i32; 3], floor: i32, spread: i32| {
        r.min(g).min(b) >= floor && r.max(g).max(b) - r.min(g).min(b) <= spread
    };
    let [r, g, b] = pixel(screen.width / 2, 3);
    if !glass([r, g, b], 0xd8, 0x18) {
        return Err(format!("the menu bar's glass at the top is {r},{g},{b}"));
    }
    let words = |xs: core::ops::Range<usize>| {
        xs.flat_map(|x| (4..20).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                let [r, g, b] = pixel(x, y);
                r + g + b < 0x150
            })
            .count()
    };
    let (left, right) = (words(4..160), words(screen.width - 160..screen.width - 4));
    if left < 30 || right < 30 {
        return Err(format!("the menu bar's words: {left} at the left, {right} at the right"));
    }
    // The dock: a bar of glass above its tiles, centred, far narrower than
    // the screen; the terminal's dark tile; a running dot below a tile.
    let row = screen.height - 70;
    let run: Vec<usize> = (0..screen.width)
        .filter(|&x| glass(pixel(x, row), 0xe0, 0x10))
        .collect();
    let (Some(&from), Some(&to)) = (run.first(), run.last()) else {
        return Err(format!("no dock's glass across row {row}"));
    };
    let wide = to - from + 1;
    if !(120..=screen.width - 200).contains(&wide) {
        return Err(format!("the dock's glass is {wide} wide, from {from}"));
    }
    let middle = (from + to) / 2;
    if middle.abs_diff(screen.width / 2) > 8 {
        return Err(format!("the dock's middle is at {middle}, not the screen's"));
    }
    let count = |ys: core::ops::Range<usize>, want: [i32; 3], by: i32| {
        (from..=to)
            .flat_map(|x| ys.clone().map(move |y| (x, y)))
            .filter(|&(x, y)| near(pixel(x, y), want, by))
            .count()
    };
    let tile = count(screen.height - 62..screen.height - 18, [0x2b, 0x2b, 0x30], 6);
    if tile < 900 {
        return Err(format!("{tile} pixels of the terminal's tile in the dock"));
    }
    let dots = count(screen.height - 14..screen.height - 8, [0x4a, 0x4d, 0x55], 24);
    if dots < 6 {
        return Err(format!("{dots} pixels of a running dot in the dock"));
    }
    Ok(format!(
        "the light-glass wallpaper, {surface} pixels of title bar, {close} of close buttons, \
         the menu bar ({left}/{right} of words), the dock {wide} wide at {middle}, \
         {tile} of the terminal's tile, {dots} of dots"
    ))
```

Doc comment `judge` menjadi:

```rust
/// Whether `screen` is Yoru's light-glass desktop: the wallpaper's sand in
/// the lower right corner, which no window reaches; title bars on the light
/// surface, `#f3f3f5`; a red close button; the wallpaper's blue at the
/// top left, where the gradient starts; the menu bar's glass with words at
/// both ends; and the dock, a centred bar of glass with the terminal's
/// dark tile and a running dot.
```

Ganti `judge_launcher` dan doc-nya:

```rust
/// Whether the launcher is open on `screen`: its white card, a run of
/// `#ffffff` 400 to 760 pixels long centred on the screen, somewhere in
/// the rows fuzzel is put at (its top margin below the menu bar).
fn judge_launcher(screen: &Image) -> core::result::Result<String, String> {
    let white = |x: usize, y: usize| {
        let at = (y * screen.width + x) * 3;
        matches!(screen.pixels.get(at..at + 3), Some(&[r, g, b]) if r >= 0xfc && g >= 0xfc && b >= 0xfc)
    };
    let mut best = (0usize, 0usize, 0usize);
    for y in 120..260 {
        let mut x = 0;
        while x < screen.width {
            if !white(x, y) {
                x += 1;
                continue;
            }
            let start = x;
            while x < screen.width && white(x, y) {
                x += 1;
            }
            if x - start > best.0 {
                best = (x - start, start, y);
            }
        }
    }
    let (long, start, y) = best;
    let middle = start + long / 2;
    if !(400..=760).contains(&long) || middle.abs_diff(screen.width / 2) > 16 {
        return Err(format!(
            "the launcher's card: the longest white run is {long} at {start},{y}"
        ));
    }
    Ok(format!("the launcher's card {long} pixels across at row {y}"))
}
```

Syarat siap boot `yoru` (baris 66-70):

```rust
            let docked = lines
                .iter()
                .any(|line| line.contains("yoru-shell: the dock is up"));
            let barred = lines
                .iter()
                .any(|line| line.contains("yoru-shell: the menu bar is up"));
            drawn && framed && docked && barred
```

Pesan galatnya menjadi `"{arch}: yoru: the wallpaper, the second window, the dock or the menu bar never said it was up"`.

- [ ] **Step 4: Tes sintetis hijau**

```bash
cd /root/ferrix && cargo test -p xtask compositor::yoru 2>&1 | grep -E "test |result"
```

Expected: 6 tes `ok`.

- [ ] **Step 5: Boot nyata, ambang dibaca dari layar asli**

```bash
cd /root/ferrix
for boot in yoru yoru-launcher files stash walls edge-drag sidebars quick weather; do
  echo "== $boot"; cargo xtask test-compositor --arch x86_64 --boot $boot 2>&1 | tail -3
done
for boot in chrome-yoru portal-desktop settings-live; do
  echo "== $boot"; cargo xtask test-compositor --arch x86_64 --boot $boot 2>&1 | tail -3
done
```

Expected: semua lulus.
- Baris `yoru` mencetak nilai nyata, misalnya `the dock 301 wide at 512, 2000 of the terminal's tile, 12 of dots`. Catat nilai itu.
- Bila angka nyata jauh di atas ambang (misalnya tile 2100 terhadap ambang 900), **kencangkan** ambang ke kira-kira 80% nilai nyata, lalu ulang Step 4 dan 5.
- Bila sebuah ambang gagal, cetak nilainya. Perbaiki penyebabnya: posisi, warna markup, atau pergeseran 24 px dari exclusive zone MenuBar. Jangan melonggarkan ambang.
- Boot yang mengklik koordinat jendela (`edge-drag`, `walls`, `stash`) bisa bergeser 24 px karena MenuBar. Perbaiki koordinatnya di berkas boot itu, dengan alasan di commit.
- `chrome-yoru`, `portal-desktop`, dan `settings-live` dijalankan bila prasyaratnya ada seperti di A1 (Chrome, portald). Bila tidak bisa dijalankan, tulis di catatan, jangan dianggap lulus.

- [ ] **Step 6: Commit (setelah izin pengguna)**

```bash
cd /root/ferrix
git add tools/common/xtask/src/compositor/yoru.rs
git diff --cached --stat
git commit -m "xtask: judge the light dock, the menu bar and the light launcher" -m "The desktop needs the menu bar's glass with words at both ends, and a dock that is a centred bar of glass narrower than the screen with the terminal's dark tile and a running dot. The launcher is a white card centred below the menu bar. Synthetic screens check each, the old dark dock and a dock across the screen among them; the yoru boot also waits for the menu bar."
```

---

### Task 9: Ukur akhir, tangkapan layar, README, dan pin

**Files:**
- Modify: `src/user/apps/yoru-shell/README.md`
- Create: `docs/superpowers/shots/a2-dok-menubar.png` (Windows: `C:\Users\jeani\Downloads\OS\docs\superpowers\shots\`)
- Modify: `components.toml` (setelah izin push)

- [ ] **Step 1: Ukur akhir**

```bash
cd /root/ferrix && cargo xtask measure-desktop --boots 2 2>&1 | tail -25
```

Tabel anggaran untuk catatan eksekusi (boot kedua):

| Ukuran | Baseline A1 | Batas A2 | Hasil |
|---|---|---|---|
| `dock drawn` setelah `graphical.target` | 0,6-0,8 s | paling lambat 0,8 s | |
| `used` (MemTotal - MemAvailable) | 97 MiB | paling banyak 97 MiB | |
| RSS `yoru-shell` (semua) | 18,0 MB (7,6 + 5,2 + 5,2) | paling banyak 12 MB | |
| RSS `hyprix` | 41 MB | paling banyak 41 MB | |

Bila ada batas yang dilampaui, berhenti dan laporkan. Jangan commit Task 9 dulu.

- [ ] **Step 2: Tangkapan layar VNC.** Jalankan `run-compositor` bertema Yoru seperti di A1, buka satu terminal, lalu buka menu Window. Ambil gambar lewat VNC dan simpan ke `C:\Users\jeani\Downloads\OS\docs\superpowers\shots\a2-dok-menubar.png`.

**Belum diketahui:** perintah persis `run-compositor` dengan VNC. Pakai yang tercatat di catatan eksekusi A1 (bagian tangkapan layar).

- [ ] **Step 3: Tidak ada nama Apple di berkas yang dikirim**

```bash
cd /root/ferrix
grep -rniE "apple|macos|mac os|finder|cupertino|app nap" src/user/apps/yoru-shell/src src/user/apps/yoru-shell/ui src/user/system/linux/compositor/hyprix/themes
```

Expected: tidak ada keluaran. Bila ada, ganti kata itu: "macOS-like" menjadi "the spec's", "Finder" menjadi "Files".

- [ ] **Step 4: README yoru-shell.** Ganti bagian yang menjelaskan dok lama (workspace, pomodoro, jam) dengan:

```markdown
## The dock and the menu bar

- **Dock** - a bar of glass in the middle of the screen's foot. The launchers
  (Terminal, Browser, Files, Settings) as tiles, a dot under each that has a
  window on any desktop or minimized; the other programs with windows after
  them; a line; the stash, where files can be dropped. A press brings the
  window used last (a minimized one comes back) or starts the program.
- **Menu bar** - a strip of glass across the top, on a thread of the same
  program. At the left the FX menu (Settings, Wallpaper, Restart or Shut
  Down), the focused program's (About, Hide, Quit) and Window (Minimize,
  Full Screen, Move to Desktop 1-4, Close); at the right the search and the
  clock. `--menubar` runs it alone; `--render-menubar FILE [--open N]` draws
  it to a PNG.
- **Wallpaper** - the switcher's first card, Default, is the theme's own
  gradient; pictures (PNG, JPEG, SVG) and videos as before.

The sidebars (`--sidebar left|right`) are no longer started by the theme;
their contents move to Control Center and Notices (A4).
```

- [ ] **Step 5: Commit README (setelah izin pengguna)**

```bash
cd /root/ferrix/src/user/apps
git add yoru-shell/README.md
git diff --cached --stat
git commit -m "yoru-shell: the README says what the dock and the menu bar are" -m "The glass dock, the menu bar's menus, the Default wallpaper card; the sidebars no longer start with the theme."
```

- [ ] **Step 6: Push apps dan pin (hanya setelah izin pengguna, [KEPUTUSAN] 11)**

```bash
cd /root/ferrix/src/user/apps && git push -u origin desktop/a2-dok-menubar
cd /root/ferrix && cargo xtask pin-components 2>&1 | tail -5
git diff --stat components.toml
git add components.toml
git diff --cached --stat
git commit -m "components: pin the apps at A2's dock and menu bar" -m "yoru-shell with the glass dock, the menu bar and the Default wallpaper card."
```

**Belum diketahui:** nama subperintah pin (`pin-components` disebut di catatan A1). Cara memeriksa: `cargo xtask --help | grep -i pin`.

---

## Risiko

1. **Fitur markup tole yang belum terbukti:**
   - `==` pada `int`
   - ternary bertingkat lima
   - `Canvas` di delegate `Repeater` yang mengikat painter root
   - `x` yang merujuk `implicitWidth` miliknya sendiri lewat `id`
   - nama field atau varian sinyal hasil generator (`drop_x`, `MenuBarSignal::Pick`, `DockSignal::App`)

   Setiap butir punya cadangan di step-nya, dan semuanya terbukti di `cargo build` dan tes, sebelum boot.
2. **Dua koneksi Wayland dalam satu proses.** Tiap `Window::open` membuka `Client::connect()`. Bila ada state global, pakai cadangan `--menubar` sebagai proses terpisah. RAM proses kedua sekitar 5 MB tetap di bawah anggaran 18 MB yang hilang bersama sidebar.
3. **Menu ditutup hanya oleh klik** (judul lagi, entri, area di bawah strip) atau oleh pergantian fokus. Tidak ada Escape, karena `KeyboardInteractivity::None`. Area transparan di bawah strip memakan satu klik saat menu terbuka.
4. **Exclusive zone 24 px menggeser jendela ke bawah.** Koordinat klik di boot lama bisa meleset (Task 8 Step 5). Posisi fuzzel (`y-margin=150`) juga bergeser 24 px. `judge_launcher` mencari di baris 120-260.
5. **Ambang judge adalah hasil penalaran,** bukan dibaca dari screendump nyata. Task 8 mewajibkan pencetakan nilai nyata dan hanya mengizinkan pengencangan.
6. **Layer dok hanya di-anchor ke BOTTOM.** Pemusatan oleh hyprix belum diketahui. Cadangan ada di Task 4 Step 5.
7. **`setting reset` memuat ulang konfigurasi.** `exec-once` tidak jalan ulang (diperiksa di `state.rs:1819`), tetapi perubahan `keyword` runtime ikut hilang. Contohnya `decoration:dim_inactive` dari quick settings, yang sekarang hanya ada di sidebar. Jawaban reset untuk kunci yang tidak tersimpan belum diketahui.
8. **Tindakan menu Window pada jendela yang salah.** Bila klik di layer sempat memindah fokus, `done()` memfokuskan ulang alamat yang tercatat sebelum dispatch. Apakah hyprix memindah fokus saat layer diklik belum diketahui; cek log `menu: dispatch focuswindow ...`.
9. **RAM dari `Ui` dan font kedua di thread MenuBar.** Font dimuat ulang oleh `look::fonts()` per `Ui`. Bila RSS melewati 12 MB, cadangan berikutnya adalah membagi `tole::Fonts` (`Rc`, jadi tidak bisa lintas thread). Itu berarti satu thread dan satu loop untuk dua surface, yang dicatat sebagai upgrade path.
10. **Aplikasi Settings belum punya pilihan wallpaper "Default".** Hanya switcher yoru-shell yang punya. Diselaraskan di A4.
11. **GTK dan portald tetap gelap sampai A4.** Dialog GTK di atas desktop terang akan tampak kontras.
12. **Set lint clippy crate apps belum diketahui.** Ikuti pola `#[expect]` yang ada di `model.rs`.
13. **`Cargo.lock` di kedua repo sudah dimodifikasi** sebelum A2. Stage berkas satu per satu dan jangan ikutkan `Cargo.lock`.
14. **Tema berkas "Nocturne Dark" eksplisit** kini memakai token hasil mesin (mendekati Nocturne dalam rentang 16 per kanal, menurut tes), bukan konstanta. Perbedaan kecil warna mungkin terlihat.

## Self-review

- Cakupan spec:
  - §2 satu proses, banyak surface: Task 6
  - §3 sidebar, pomodoro, dan cuaca hilang dari desktop: Task 4 dan 7
  - §4 dok tengah dan tema terang: Task 1, 3, 4
  - §5 MenuBar dan shortcut SUPER Q/W/M: Task 5, 6, 7 (Hide menyimpang, ditandai **[KEPUTUSAN] 7**)
  - §8.1 `minimized`: Task 0 Step 3 dan Task 4 `focuswindow`
  - §12 anggaran: tiap task dan Task 9
  - §13 tanpa Apple: Task 9 Step 3
- Permintaan tambahan:
  - kartu wallpaper Default dan foto pengguna tetap jalan: Task 2
  - sidebar: spec menggantinya, jadi berhenti dinyalakan (Task 7), bukan digabung
  - judge diperketat dengan kontrol negatif: Task 8
- Tidak ada placeholder kode. Butir yang tidak bisa dipastikan dari kode ditulis "Belum diketahui" beserta cara memeriksanya.
- Nama konsisten: `Tokens::GLASS`, `frame_requests`, `default_wall`/`with_default`/`request`, `State::new(launchers)`, `ui::width`, `menus::standard`, `menubar::start`, `render_menubar`.
