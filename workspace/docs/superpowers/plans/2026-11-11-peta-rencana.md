# Peta rencana: desktop gaya macOS dan memori (spec 2026-11-11)

Spec: `docs/superpowers/specs/2026-11-11-desktop-gaya-macos-design.md`.

Spec ini terlalu besar untuk satu rencana yang semua kodenya ditulis lengkap sekarang.
Komponen berikutnya bergantung pada kode yang belum dibaca rinci (bahasa markup `.tole`,
crate `compositor/protocol`, IPC hyprix) atau pada keputusan konsultan sertifikasi.
Karena itu rencana dipecah. Tiap rencana menghasilkan sesuatu yang berjalan dan teruji
sendiri, dan rencana berikutnya ditulis setelah rencana sebelumnya mendarat.

| Rencana | Isi | Status | Bergantung pada |
|---|---|---|---|
| **A1** `2026-11-11-A1-fondasi-desktop.md` | boot `lode-pstore`, tombol kiri, tema terang kaca, wallpaper PNG, skrip ukur | **mendarat** | - |
| **B0** `2026-11-11-B0-memori-desain-dan-lz4.md` | dokumen desain untuk konsultan, crate `ferrix-lz4` | **siap dieksekusi** | - |
| **A2** `2026-11-11-A2-dok-menubar.md` | dok di tengah (ikon, titik berjalan), tema `.tole` terang untuk dok/panel, MenuBar dengan menu standar, special workspace `minimized` | **mendarat** | A1 |
| **A3** `2026-11-11-A3-menu-aplikasi.md` | protokol `ferrix_appmenu_v1`, `appmenu` di hyprix, `set_menu` di tole, menu asli di Settings dan Files | **mendarat** (2026-11-14, commit `1c302a08a` di ferrix, `2f8cf00` di apps) | A2 |
| A4a | `2026-11-14-A4a-spotlight-launchpad-notices.md` | Spotlight (ganti fuzzel di SUPER Space), Launchpad, Notices (banner kanan atas + riwayat 50 lewat soket notifyd), boot `spotlight`/`launchpad`/`notices` | **selesai, menunggu review** (dipecah 2026-11-14, keputusan pengguna; dieksekusi opencode 2026-11-15) | A2, A3 |
| A4b | `2026-11-15-A4b-control-center.md` | Control Center (termasuk tema GTK/portald terang-gelap) | **mendarat** (2026-11-15, commit `29b4a3f5c`/`552cfe45f` di ferrix, `6643e5b`/`4150d80` di apps) | A4a |
| A4c | `2026-11-15-A4c-mission-control.md` | Mission Control (overview hyprix, gaya baru): ruang = workspace, kartu = permukaan jendela yang dikecilkan, SUPER Tab | **mendarat** (2026-11-16, branch `desktop/a4c-mission-control`) | A4b |
| A5 | Hemat tahap 1: satu cgroup per app di pohon `hemat`, keadaan Aktif/Latar/Tidur, `cpu.weight` 100/10, `memory.reclaim` saat Tidur, daftar pengecualian, tekanan memori dan banner Notices; uji boot `multiapp` | **mendarat** (2026-11-16, branch `desktop/a5-hemat`, worktree `/root/ferrix-d`; Task 1–7 ter-commit) | A2, A4a (permukaan Notices); **bukan** delegasi cgroup sesi: Hemat memakai cgroupnya sendiri (`/sys/fs/cgroup/hemat/`) karena aturan delegasi dan aturan tanpa proses internal menolaknya di bawah cgroup sesi |
| B1-B5 | entri swap, penuaan halaman, kolam terkompres, swap disk, antarmuka Linux | ditulis setelah review konsultan | B0 dan jawaban konsultan |
| A6 | Hemat tahap 2 (`cgroup.freeze`) | setelah A5 dan APP-SUSPEND S1 | A5 |

A1 dan B0 tidak saling bergantung dan bisa jalan paralel.

## Keputusan bahasa (2026-11-11, diserahkan pengguna)

- Semua kode baru A2-A6 dan B1-B5 ditulis dalam Rust, termasuk alat bantu.
- Alat Python buatan kita dipindah ke Rust:
  - `tools/measure/ferrix_desktop.py` dan `lordos.py` menjadi `cargo xtask measure-desktop` dan `cargo xtask measure-lordos`. LordOS ikut diukur lewat KVM di WSL, supaya akseleratornya sama dengan Ferrix.
  - `docs/brand/yoru/make-light-glass.py` dan PNG-nya diganti gradien yang digambar kode Rust di klien wallpaper. Dengan begitu dekode PNG hilang dan RAM turun.
- Alat Python dan shell upstream (gen, check, release, fetch) tidak di-port sekaligus. Port ke Rust hanya dilakukan bila berkas itu memang kita sentuh.
- Probe C ABI Linux, oracle seccomp, dan header referensi vendor tetap C. Fungsinya merekam perilaku Linux asli, dan Rust tidak bisa menggantikan itu.
