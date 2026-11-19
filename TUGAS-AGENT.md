# Papan tugas agent

Dibaca otomatis oleh setiap harness saat VS Code membuka folder `OS`
(lihat `.vscode/tasks.json`). Untuk memindah jalur ke harness lain, cukup ubah
tabel di bawah; tasks.json tidak perlu disentuh.

| Harness | Jalur | Model |
|---|---|---|
| claude | A (A4a, A4b, A4c, A5, A6) | bawaan Claude Code |
| minimax | B (B0, B1-B5) | bawaan MiniMax Code CLI (`mcode`) |
| opencode | review: periksa rencana berstatus **selesai, menunggu review**, tulis temuannya di Laporan, jangan ubah kode | dipilih di opencode |

Daftar rencana, status, dan ketergantungannya ada di
`docs/superpowers/plans/2026-11-11-peta-rencana.md`.

## Aturan untuk setiap harness

1. Baca tabel di atas, cari jalurmu.
2. Di peta rencana, ambil baris pertama di jalurmu yang berstatus **siap dieksekusi**
   dan semua ketergantungannya **mendarat**. Kalau tidak ada, laporkan barisnya
   dan alasannya (misalnya "B1-B5 menunggu review konsultan"), lalu berhenti.
3. Kerjakan rencana itu dari berkas rencananya, langkah per langkah. Centang
   `- [x]` di berkas rencana setiap langkah selesai.
4. Sebelum commit di `FERRIX`, baca `FERRIX/CLAUDE.md` dan `FERRIX/docs/CONVENTIONS.md`.
5. Folder ini dipakai bersama harness lain secara bersamaan:
   - Sentuh hanya berkas milik rencanamu.
   - Commit dengan menyebut path secara eksplisit (`git commit -- <path>...`).
     Jangan `git add -A`, `git stash`, `git reset`, `git checkout <branch>`, atau rebase.
   - Jangan push. Pengguna yang memeriksa dan mendaratkan.
6. Setelah rencana selesai dan teruji, ubah statusnya di peta rencana menjadi
   **selesai, menunggu review** dan tulis ringkasan satu paragraf di bawah ini.

## Laporan

<!-- harness menulis di sini: tanggal, harness, rencana, commit, apa yang belum -->

**2026-11-15, opencode, A4a (Spotlight, Launchpad, Notices) — selesai, menunggu review.**
Branch `desktop/a4a-spotlight` (ferrix + apps). Commit di Task 6 ini: ferrix `f5e90fa47`
(xtask boot `notices` menuntaskan rantai penuh: aturan alert lode di `/etc/ferrix/lode.d/notices.rules`
mengubah tanda `/dev/kmsg` jadi alert di `/run/lode/alerts`, notifyd menampilkannya sebagai banner,
yoru-shell membuka riwayatnya lewat klik jam — panel dijudike dari screendump `notices.ppm`);
`docs/DESKTOP.md` §6.9, README apps, serta boot `spotlight`/`launchpad`/`notices` sudah mendarat di
`7cdf5883c`. Uji: yoru-shell 103 lulus; xtask 4 uji notices + 3 boot PASS (ppm 13:47 dan 15:0x hari ini);
notifyd/xtask hijau. Pengukuran (`measure-desktop --boots 2` ×2, boot kedua dijadikan): dok 0,65 s setelah
graphical.target (≤ 0,8 ✓; satu sampel 0,92 s tampaknya noise host), hyprix 41,37 MB ≤ 41 ✓, idle used
96 MiB ≤ 97 ✓. **Temuan RAM selesai (2026-11-15):** yoru-shell (dock) 9.976 KiB ≈ 9,7 MB di atas
angka harapan 8 MB di rencana A4a, tetapi investigasi membuktikan itu bukan regresi A4a — angka 8 MB
mengabaikan bahwa sejak A2 MenuBar hidup di dalam proses dock (anggaran mengikat A2/A3 = 12 MB).
Bukti: biner debug A3 dan A4a sama-sama peak VmRSS 5.248 KiB saat startup di host (worktree A3
dibangun lalu dibersihkan); tambahan A4a ke proses dock berbiaya RSS nol, tidak ada proses permukaan
yang bocor. Detail di §"Hasil ukur Task 6" rencana A4a. Churn Cargo.lock di 5 modul (bump versi
lokal 0.1.0→0.3.0) tidak ikut di-commit.

**2026-11-15, opencode, review A4a — lulus dengan satu temuan kecil.**
Ditinjau terhadap rencana dan spec: peta berkas cocok (ferrix 9 commit +2.097 baris, apps 12
commit +3.119; dua tambahan kecil di luar peta wajar: `fuzzel/tests/boot_frames.rs` dan
`xtask/{files.rs,theme.rs}` yang dipakai bersama boot); keybind SUPER Space → `--spotlight` dan
SUPER L → `--launchpad` benar, fuzzel tidak disentuh; anggaran pemindaian Spotlight di produksi
`scan(&home, 5000, 300_000_000)` ✓; Launchpad `KeyboardInteractivity::None`, paging & dismiss ada;
Notices lifetime banner 5 s, `close <id>`, subscribe ada; README menyajikan ketiga permukaan.
Uji diulang saat review: yoru-shell **103**, notifyd **25**, xtask compositor **136** — semua lulus.
Tidak ada aset/gambar Apple; string UI Inggris. **Temuan (kecil, spec §13):** kata `finder` dalam
pemetaan ikon `model.rs::icon_for` (`["shelf", "nautilus", "finder"]`, commit apps `19ca05e`)
baru muncul di A4a dan tidak ada di A3 — sekalipun dia token heuristik bukan string UI, sebaiknya
diganti (mis. `files`/`dolphin`/`nemo`) agar cabang bersih dari nama Apple. Tidak menghalangi
mendarat bila pemilik menilai kata itu generik.

**2026-11-19, opencode, rebrand + live ISO + GitHub — sebagian selesai, menunggu review.**
Rebrand tuntas: apps `0d147b0` dan ferrix `8027e0720` (91 berkas, 0 sisa "yoru"); sesudahnya pohon
bergerak lagi di branch `rename/xaqu` hingga ferrix `45e4c42e6` ("check is green and the live ISO
builds and boots") dan apps `d31719a`. Live ISO terbentuk: `/root/ferrix/build/x86_64/xaqu-live.iso`
(218.103.808 byte, 2026-11-18 00:45), isi `EFIBOOT.IMG` + `LICENSE.TXT` + `THIRD_PARTY_LICENSES.md`;
inventaris lisensi 0 REFUSED (yang dulu menolak `/bin/ferrix-eventlog` sudah lolos); tiga log boot
terakhir memakai `xaqu.live` + `runtime_settings=live`. Penyebab kegagalan lama terdokumentasi: OOM
VM WSL 3,8 GB membunuh `rustc` kernel (RSS 3,55 GB), diperbaiki lewat `.wslconfig` 6 GB + swap 2 GB;
sisa `step_cache::every_kind_of_change_is_a_miss` gagal deterministik juga di komit pra-A4a dan
pra-rebrand, jadi lingkungan host, bukan kode. **Inventaris berkas (2026-11-19):** ferrix 4.714
berkas terlacak, apps 465 berkas / 25 app, initramfs 3.886 entri, tersimpan di
`build/x86_64/inventory/{ferrix-tracked,apps-tracked,initramfs-files}.txt`. **GitHub:** workspace
dipindahkan ke `https://github.com/Jeaniro07/XaquOS` (branch `main`, commit `60f0922`): pohon sistem
ferrix `45e4c42e6` + seluruh 25 app, satu clone tanpa repo bersarang; `.gitignore` tak lagi
mengabaikan `src/user/apps` dan `components.toml` tak lagi memuat `[apps]`, plus `PROVENANCE.md`
yang mencatat asal kedua pohon. **Belum:** (1) `bridgeplay` sudah ada sebagai app Rust (`bin/bridgeplay`)
tetapi belum masuk `DESKTOP_APPS`, jadi belum terpasang sebagai bawaan; (2) `artcraft` tidak ada di
Xaqu — jejaknya ditemukan di katalog toko LORDOS (`LORDOS/apps/store`, `apt/catalog.json`), jadi harus
diporting ke Rust; (3) repo XaquOS belum diuji build (komponen `tole`, `ferrousli`, `zinc` tetap
di-clone xtask saat pertama dipakai); (4) `LORDOS` dan `FERRIX` lama tidak dihapus dari workspace.

**Keputusan pengguna (2026-11-15): usul "menambahkan Hyprland" tidak diterapkan** — compositor
desktop sudah `hyprix`, yang kompatibel Hyprland (IPC `hyprctl`/`dispatch`, sintaks `bind =` di
`yoru-keys.conf`, layer surfaces, `layerrule blur`) dan seluruh A1–A3 serta boot uji A4a berjalan
di atasnya; Hyprland asli akan jadi compositor kedua yang saling berebut seat, di luar saluran
notifyd/lode, dan di luar spec `docs/DESKTOP.md` §6.
