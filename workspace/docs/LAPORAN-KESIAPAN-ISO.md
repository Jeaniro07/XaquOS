# Laporan kesiapan ISO Xaqu (2026-11-18)

Kesimpulan: **ISO jadi dan lolos uji boot.**

- Berkas: `D:\Xaqu-ISO\xaqu-live.iso` (208 MiB, x86_64, UEFI)
- SHA-256: `619cce1a98add900a6133660ad8d08a991f3b5327b0b0b54349e9ec391b89e20`
- Burning ke USB: Rufus mode **DD image**, atau `dd if=xaqu-live.iso of=/dev/sdX bs=4M`. Boot UEFI, Secure Boot dimatikan. Tidak ada boot BIOS lama.

## Terbukti
- `cargo xtask check` hijau penuh (EXIT=0), termasuk unit test dan check-side.
- `cargo xtask iso`: semua bagian image tercatat lisensinya (674 crate, 22 app, 2936 berkas).
- `cargo xtask test-iso` di OVMF: boot sebagai CD dan sebagai USB stick, desktop live muncul, klik "Install Xaqu" menjalankan installer, `reboot` kembali ke desktop.

## Diperbaiki di putaran ini
- Tes xtask: `docs/APPS.md` menyebut path folder app (aturan APPS §4).
- `term`: indeks kolom tes prompt agnoster sisa rename (`root@ferrix` ke `root@xaqu`).
- `xaqu-shell`: clippy `cmp_owned` di `shelf.rs`.
- `lode-net`: format.
- `bridgeplay`: peluncur Steam dikeluarkan dari paket (ISO rilis tidak membawa Steam; `.exe` dibuka lewat `xaqu-open`).
- `release-licences.toml`: `bin/xaqu-eventlog` tercatat.
- `xtask paths.rs`: firmware OVMF dicari juga di `<prefix>/share/qemu` milik QEMU yang dibangun xtask.
- `xaqu-open`: runtime boleh membawa argumen; image desktop membuka Windows lewat `bridgeplay run`.

## Batas yang jujur
- Aplikasi Windows: perlu volume Wine (`fetch-wine.sh`); tanpa itu `bridgeplay` menolak dengan pesan jelas.
- Android dan macOS: `xaqu-open` sudah mengenali `.apk`/`.app`/`.dmg`, tetapi runtime-nya (Android Translation Layer, Darling) belum diporting ke Xaqu.
- Belum diuji di perangkat keras nyata; uji boot hanya di QEMU/OVMF.
- Semua perubahan belum dicommit.
- Lingkungan build: symlink `~/.local/share/ferrix -> xaqu` untuk cache port lama; berkas liar `/Cargo.toml` dan `/src` dipindah ke `/root/stray-root-backup`.
