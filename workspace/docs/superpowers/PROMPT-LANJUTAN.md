# Prompt lanjutan Xaqu (diperbarui 2026-11-18)

Menggantikan versi 2026-11-15, yang sudah usang. Salin blok di bawah ke sesi
baru. Untuk pekerjaan driver saja, pakai `docs/PROMPT-DRIVER-LENGKAP.md`.

---

```text
Lanjutkan proyek Xaqu (dulu bernama Ferrix): OS berkernel mikro Rust dengan desktop gaya macOS, yang ditujukan menjalankan aplikasi Linux, Windows (lewat Wine), Android, dan macOS seperti FlowOS. Pakai Bahasa Indonesia.

Prinsip kerja:
- Kode seminimal mungkin ("ponytail"). Penyederhanaan yang disengaja ditandai komentar upgrade path.
- Kode baru ditulis dalam Rust.
- Pakai plugin superpowers: writing-plans, test-driven-development, subagent-driven-development, verification-before-completion.
- Jangan berhalusinasi: register, ID, dan API diambil dari sumber, bukan dari ingatan.

== Lokasi ==
- Repo utama: WSL archlinux /root/ferrix, cabang integrasi rename/xaqu (HEAD 45e4c42e6). Dari Windows, edit lewat \\wsl.localhost\archlinux\root\ferrix\...
- Repo bersarang (di-gitignore oleh ferrix, commit terpisah per repo):
  - src/user/apps: cabang desktop/a4a-spotlight, HEAD d31719a.
  - src/user/system/linux/tole: cabang desktop/a3-appmenu, HEAD 63c1bcb.
  - docs/website: cabang rename/xaqu, HEAD f25cfb1.
- Worktree lain: /root/ferrix-d (desktop/a5-hemat, sudah digabung) dan /root/ferrix-pre (detached di d80f32b15, cadangan sebelum rename). Keduanya boleh dibersihkan setelah saya setujui.
- Dokumen di Windows, C:\Users\jeani\Downloads\OS\docs\:
  - LAPORAN-KESIAPAN-ISO.md: kondisi ISO;
  - PETA-DRIVER-LAPTOP.md: peta driver laptop saya;
  - PROMPT-DRIVER-LENGKAP.md dan PROMPT-DRIVER.md: prompt driver;
  - superpowers\plans\2026-11-11-peta-rencana.md: peta rencana desktop dan memori;
  - superpowers\specs\2026-11-11-desktop-gaya-macos-design.md: spek desktop.
- ISO terakhir: D:\Xaqu-ISO\xaqu-live.iso, 218103808 byte, SHA-256 619cce1a98add900a6133660ad8d08a991f3b5327b0b0b54349e9ec391b89e20. Lolos test-iso sebagai CD dan USB di QEMU/OVMF. Belum pernah diuji di perangkat asli.

== Status ==
Selesai dan masuk rename/xaqu:
- Desktop A1 (fondasi), A2 (dok dan menu bar), A3 (menu aplikasi), A4a (Spotlight, Launchpad, Notices), A4b (Control Center), A4c (Mission Control), A5 (Hemat tahap 1).
- Memori B1-B5 (cabang mm/b1-b5).
- Rename Ferrix menjadi Xaqu, commit aba043145.
- Perbaikan sampai cargo xtask check hijau penuh dan ISO jadi, commit 45e4c42e6:
  - docs/APPS.md;
  - format lode-net;
  - lisensi bin/xaqu-eventlog;
  - pencarian OVMF di xtask paths.rs;
  - indeks tes term agnoster;
  - clippy xaqu-shell;
  - peluncur Steam dikeluarkan dari paket bridgeplay.
- App xaqu-open (repo apps): membuka berkas sesuai dunianya. .exe lewat "bridgeplay run" (Wine); .apk, .app, dan .dmg sudah dikenali, tapi runtime-nya belum ada.

Masih terbuka (cek dan kerjakan berurutan):
1. Repo tole punya 11 berkas belum dicommit: .github/workflows/ci.yml, Cargo.toml, LICENSE, README.md, clippy.toml, examples/src/lib.rs, Theme.tole, tole-compiler/src/lib.rs, tole/Cargo.toml, tole/src/lib.rs, tole/src/window.rs. Kemungkinan sisa rename Ferrix menjadi Xaqu. Tinjau diff-nya, laporkan ke saya, dan commit hanya setelah saya setujui. Hati-hati dengan LICENSE: perubahan isi lisensi harus saya putuskan.
2. Cabang mm/b0-lz4 punya 6 commit yang tidak ada di HEAD. Periksa apakah isinya (dokumen MEMORY-COMPRESSION.md, crate lz4) sudah masuk lewat mm/b1-b5 dengan commit lain. Kalau belum, laporkan dan tanya sebelum digabung.
3. A4a belum direview ulang oleh reviewer independen setelah perbaikannya.
4. A6 (Hemat tahap 2, cgroup.freeze) belum dimulai. Butuh A5 (sudah) dan APP-SUSPEND S1 (cek docs/APP-SUSPEND.md).
5. Peta rencana sudah usang: baris B0 masih "siap dieksekusi" dan B1-B5 masih "ditulis setelah review konsultan". Perbarui sesuai kenyataan.
6. Aplikasi lintas OS:
   - Windows butuh volume Wine (tools/common/fetch/fetch-wine.sh) yang tidak ada di ISO. Rancang cara memasang volume Wine dari desktop, misalnya unduhan sekali klik yang tercatat di audit lisensi.
   - Android: porting Android Translation Layer.
   - macOS: porting Darling.
   Ketiganya proyek besar. Buat rencana dulu dan tanya saya sebelum mengerjakan.
7. Driver laptop saya (realme, i3-1115G4 Tiger Lake): hanya CPU, NVMe, dan USB yang berfungsi. Keyboard memakai keyboard USB (keputusan saya); keyboard bawaan PS/2 tidak dikerjakan. Semua pekerjaan driver mengikuti PROMPT-DRIVER-LENGKAP.md. Kerjakan di sesi ini hanya bila saya minta.
8. Masalah lama yang belum tuntas:
   - self-check jam FX-0308 panik di KVM-WSL;
   - boot idle hypridle tidak stabil di TCG;
   - ArtCraft ditunda.
9. Lingkungan build (bersihkan hanya setelah saya setujui):
   - symlink /root/.local/share/ferrix -> xaqu, untuk cache port lama; hapus setelah cache port dibangun ulang;
   - /root/stray-root-backup berisi /Cargo.toml dan /src liar dari agen B1;
   - cadangan WSL D:\wsl-backup\archlinux.tar.gz;
   - skrip /root/run-final-check.sh, /root/probe.sh, /root/commit-all.sh, /root/repos-status.sh.

== Langkah ==
1. Cek keadaan: git status dan git log di ferrix, apps, tole, dan website; git worktree list; ruang disk C: dan D: (C: pernah 0 MB dan WSL gagal start); pastikan tidak ada proses cargo atau qemu yang menggantung.
2. Kerjakan butir terbuka 1, 2, lalu 5. Untuk setiap keputusan, tanya saya dulu.
3. Review ulang A4a dengan reviewer independen, lalu laporkan temuannya.
4. Tulis rencana A6 (plans\2026-11-18-A6-hemat-tahap-2.md), tunjukkan ke saya, lalu kerjakan dengan TDD dalam worktree sendiri.
5. Jalankan cargo xtask check penuh sampai hijau. Bila diminta, lanjutkan dengan cargo xtask iso dan cargo xtask test-iso, salin ISO ke D:\Xaqu-ISO\, dan catat SHA-256-nya.
6. Perbarui peta-rencana, LAPORAN-KESIAPAN-ISO.md, dan prompt ini sesuai kenyataan. Laporkan dengan jujur apa yang terbukti dan apa yang belum (terutama: belum diuji di perangkat asli).

== Cara menjalankan build ==
- cargo xtask check, iso, dan test-iso dijalankan terlepas: setsid nohup bash skrip.sh, dengan log di /tmp dan berkas penanda .done. Jangan sleep di foreground; pantau berkas penandanya.
- Pakai CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2. VM WSL punya 6 GB (.wslconfig: memory 6GB, swap 2GB, processors 4); tes btrfs memakan sekitar 3,5 GB, dan check paralel pernah kena OOM.
- Satu target build bersama: CARGO_TARGET_DIR=/root/ferrix/target. Jangan buat target kedua, karena C: sempit.
- test-iso butuh QEMU berlabel (xaqu-cfi), dibangun lewat tools/common/fetch/fetch-qemu-linux.sh.
- Identitas git di WSL belum diatur. Untuk commit pakai GIT_AUTHOR_NAME, GIT_AUTHOR_EMAIL, GIT_COMMITTER_NAME, dan GIT_COMMITTER_EMAIL yang sama dengan commit sebelumnya (lihat git log --format="%an <%ae>").

== Aturan ==
- Jangan push dan jangan buat PR.
- Commit hanya setelah saya setujui.
- Jangan bypass hook git. Hook commit-msg repo ferrix menolak trailer Co-Authored-By; repo apps menerimanya.
- Sebelum menggabung cabang worktree ke rename/xaqu, tanya saya dulu.
- Jangan ubah kernel (src/kernel/) tanpa tanya. Aturan proyek: menulis driver tidak boleh mengubah kernel (docs/DRIVER-MODEL.md).
- Lint workspace ketat:
  - deny: missing_docs, unwrap_used, expect_used, panic, indexing_slicing, too_many_lines, excessive_nesting;
  - setiap unsafe wajib berkomentar SAFETY dan berisi satu operasi.
- Menghapus atau memindahkan WSL, vhdx, atau cadangan butuh konfirmasi baru dari saya.
- Data pribadi saya tidak boleh dikirim ke layanan luar.
```

---

## Catatan untuk saya

- `docs/MEMORY-COMPRESSION.md` perlu diserahkan ke konsultan (B0 Tugas 1
  Langkah 2), bila masih relevan.
- Vhdx WSL sekitar 67 GB lebih. C: kosong 7,7 GB dan D: 18,5 GB (per
  2026-11-18). Cadangan `D:\wsl-backup\archlinux.tar.gz` bisa dihapus bila
  sudah yakin WSL sehat.
- Keyboard laptop: pakai keyboard USB. Touchpad, audio, Wi-Fi, dan lainnya
  ada di `PROMPT-DRIVER-LENGKAP.md`.
