# A4b: Control Center dan tema terang-gelap sampai GTK dan portald - Rencana Implementasi

> **Untuk pelaksana:** kerjakan task demi task, berurutan. Tiap task dimulai dengan tes yang gagal, lalu kode, lalu tes lulus, lalu commit. Izin commit dan eksekusi dilimpahkan koordinator untuk rencana ini.

**Goal:** ikon di menu bar membuka Control Center: kartu kaca di kanan atas berisi volume keluar dan masuk, tile cepat, pilihan tampilan (Light, Dark, Auto) dan warna tema. Pilihan terang atau gelap sampai ke aplikasi GTK dan ke portald, sehingga Chrome dan libadwaita ikut berubah.

**Arsitektur:**
1. portald (workspace compositor, paket `compositor-portald`) membaca berkas `~/.config/xaqu-shell/appearance` (`dark` atau `light`) dua kali per detik dan mengirim `SettingChanged` bila isinya berubah.
2. Menu bar xaqu-shell (proses yang selalu hidup) menerbitkan tampilan yang berlaku: berkas `appearance` dan `~/.config/gtk-{3,4}.0/settings.ini`, sekali per perubahan (`theme::publish`).
3. Baris cepat (slider, tile, tampilan, aksen) dipakai bersama oleh tab cepat di sidebar dan kartu baru lewat `ui::quick_rows` dan `quick::slide_change/mute_change/tile_change`; tidak ada daftar kedua.
4. `xaqu-shell --control` adalah permukaan Overlay baru (pola Spotlight): proses pendek, socket tunggal, dibuka lagi berarti tutup. Kartu 320x506 di kanan atas, Escape atau klik di luar menutup.
5. Ikon `sliders-horizontal` di kiri ikon search pada menu bar menjalankan `--control`.
6. Boot xtask `control`: kartu muncul, dihitung di layar, Escape menutupnya.

**Tech stack:** Rust 2024; repo apps (xaqu-shell, tole `.tole`), repo ferrix (portald, xtask, docs). Kode baru hanya Rust.

**Spec:** `docs/superpowers/specs/2026-11-11-desktop-gaya-macos-design.md` (Control Center; tema terang-gelap sampai GTK dan portald).

**Gaya kode:** "ponytail". Penyederhanaan ditandai `// Ponytail: ... Upgrade path: ...`.

---

## Aturan eksekusi

- Semua perintah di WSL Arch, worktree `/root/ferrix-a`; `export CARGO_TARGET_DIR=$HOME/ferrix/target`; tidak menyentuh `~/ferrix` dan dua `Cargo.lock` yang berubah di sana.
- Branch `desktop/a4b-control-center`: ferrix dari `c00f3637b`, apps dari `10d4a76`.
- Commit apps diakhiri trailer Co-Authored-By; hook commit-msg ferrix menolak trailer itu, maka commit ferrix tanpa trailer (hook tidak dilewati).
- Hanya uji crate yang disentuh dan boot milik rencana ini (TCG). Tidak ada `cargo xtask check` penuh, tidak ada ISO, tidak ada push.
- Cargo panjang dijalankan terlepas (`setsid nohup`), log dan berkas `done`, dipantau tanpa sleep di depan.

## Keputusan

1. [KEPUTUSAN] **Satu sumber kebenaran untuk baris cepat.** Kartu dan tab sidebar memakai `quick_rows`; kartu tidak menyalin logika slider. Tab cepat di sidebar tetap (tidak dihapus).
2. [KEPUTUSAN] **Wi-Fi, Bluetooth, Night light tetap redup** seperti di tab cepat: Ferrix belum punya backend-nya.
3. [KEPUTUSAN] **Tampilan diterbitkan oleh menu bar**, karena ia sudah memegang `Live` (pengamat tema) dan selalu hidup; portald hanya membaca satu berkas kecil.
4. [KEPUTUSAN] **Kartu mengikuti tema lewat `Live::poll` tiap 500 ms** selama terbuka. Ponytail: upgrade path, satu pengamat bersama dengan menu bar.
5. [KEPUTUSAN] **GTK 3 hanya untuk program yang baru dijalankan.** GTK 3 membaca `settings.ini` saat start; pembaruan langsung GTK 3 tidak diklaim dan tidak diverifikasi. GTK 4 dan Chrome mengikuti lewat `SettingChanged`.

## Peta berkas dan status

| Task | Repo | Berkas | Status |
|---|---|---|---|
| 1 | ferrix | `compositor/portald/src/{lib.rs,main.rs}`, `tests/bus.rs`: `appearance_file`, `parse_appearance`, `changed`, pengamat berkas | selesai, ferrix `9e2afe0fe`: fmt, clippy -D warnings bersih; 9 tes lib + tes bus lulus |
| 2 | apps | `xaqu-shell/src/{theme.rs,menubar.rs}`: `gtk_settings`, `publish_in`, `publish`, dipanggil setelah `Live::new` dan di cabang `poll` | selesai, apps `060ea8e`: clippy bersih; `cargo test theme` 10 lulus |
| 3 | apps | `xaqu-shell/src/{quick.rs,ui.rs,side.rs}`: `slide_change`, `mute_change`, `tile_change`, `QuickRows`, `quick_rows` | selesai, apps `4424526`: clippy bersih; 107 tes lulus |
| 4 | apps | `xaqu-shell/ui/control.tole`, `src/control.rs`, `src/lib.rs`, `src/cli.rs` (`--control`, `--render-control`) | selesai, apps `5ef21bb`: clippy bersih; 111 tes lulus (4 tes kartu baru) |
| 5 | apps | `xaqu-shell/ui/menubar.tole`, `src/menubar.rs`: ikon, sinyal `control`, `Asked::Control` | selesai, apps `6643e5b`: clippy bersih; 112 tes lulus |
| 6 | ferrix | `tools/common/xtask/src/compositor/control.rs`, `compositor.rs` (mod dan tabel BOOTS) | lihat laporan akhir untuk hasil boot |
| 7 | ferrix | `docs/DESKTOP.md` (§6.5 portald, §6.9 Control Center), rencana ini, peta rencana | selesai |

## Task 1: portald mengikuti berkas tampilan

- [x] Tes gagal: `the_appearance_file_says_dark_or_light`, `a_change_of_the_file_is_told_once`, dan tes bus yang menunggu `SettingChanged`.
- [x] `appearance_file()` (`$XDG_CONFIG_HOME/xaqu-shell/appearance`, atau `~/.config`), `parse_appearance()` (`dark`/`light`, lainnya `None`), pengamat dua kali per detik di `main.rs` yang mengirim `SettingChanged`.
- [x] fmt, clippy -D warnings, uji dari direktori workspace compositor (`cargo test -p compositor-portald` dari akar ferrix gagal: jalankan dari workspace-nya).

## Task 2: tampilan sampai GTK

- [x] Tes `the_look_is_published_once_for_portald_and_gtk`: berkas `appearance` dan `settings.ini` GTK 3 dan 4 tertulis, panggilan kedua dengan isi sama tidak menulis lagi.
- [x] `theme::publish(light)` dipanggil dari menu bar sesudah `Live::new` dan di cabang `poll`.

## Task 3: baris cepat bersama

- [x] Tes `a_press_in_the_rows_is_the_change_it_names`, `the_quick_rows_stand_with_nothing_found`, `the_chosen_look_is_the_one_marked`.
- [x] `side.rs` memakai `quick_rows`; impor yang jadi tidak terpakai dibuang.

## Task 4: kartu Control Center

- [x] Tes host: tekan di kartu menanyakan perubahan yang dinamainya; tekan di luar kartu dan Escape menutup; trek slider menetapkan volume (`Change::Volume(50)`, `Change::Mute`); kartu tergambar.
- [x] `control.tole` (kartu 320x506 di x = lebar - 336, y = 34) dan `control.rs` (`switch::once`, loop overlay mengikuti Spotlight).

## Task 5: ikon di menu bar

- [x] Tes `the_control_icon_asks_for_the_center_beside_the_search`: menyapu pointer di x 700..1020 menghasilkan `Asked::Control` dan `Asked::Search`.
- [x] Ikon `sliders-horizontal` 30 px di kiri ikon search, `control()` menjalankan `exec /bin/xaqu-shell --control`.

## Task 6: boot xtask `control`

- [x] Tes sintetis `Image`: kartu terang berisi tinta dinilai kartu, wallpaper saja bukan, kartu di kiri bukan.
- [x] Boot: skrip tamu menjalankan `--control`, tuan rumah mengambil layar, menghitung 80 baris pertama kartu, menekan Escape, menunggu `the control center closed`.

## Task 7: dokumen

- [x] `docs/DESKTOP.md`: portald (terang atau gelap, berkas `appearance`, GTK 3 hanya program baru) dan bagian Control Center.
- [x] Baris A4b di `2026-11-11-peta-rencana.md`.

## Yang belum atau tidak diverifikasi

- Pembaruan langsung GTK 3 pada program yang sudah berjalan (keputusan 5).
- Boot menilai tampilan kartu dan Escape; klik pada slider dan tile diuji di tes host, tidak di boot.
- Anggaran RSS shell (<= 12 MB) tidak diukur ulang.
