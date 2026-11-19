# Prompt sesi untuk semua driver laptop (2026-11-18)

Setiap prompt di bawah berdiri sendiri: salin satu blok ke satu sesi baru.
Peta lengkapnya ada di `docs/PETA-DRIVER-LAPTOP.md`.

Urutan dan ketergantungan:

| No | Driver | Bergantung pada | Boleh jalan bersamaan dengan |
|---|---|---|---|
| 1 | Audio Intel HDA | - | 2, 3, 4, 5 |
| 2 | USB tethering (RNDIS/NCM) | - | 1, 3, 4, 5 |
| 3 | Data ACPI touchpad (tanpa kode) | - | semua |
| 4 | Keyboard bawaan PS/2 | keputusan Anda soal kernel | 1, 2, 3, 5 |
| 5 | GPU Tiger Lake | - | 1, 2, 3, 4 |
| 6 | Touchpad I2C HID | 3 selesai | 7 |
| 7 | Kamera UVC | - | 6 |
| 8 | Adaptor USB Ethernet (cadangan Wi-Fi) | 2 selesai | 6, 7 |
| 9 | Wi-Fi Intel AC 9560 | 2 selesai | - |
| 10 | Bluetooth Intel | 9 tahap firmware | - |
| 11 | Mikrofon digital (SOF) | 1 selesai | - |
| 12 | Sidik jari Goodix | - | - |
| 13 | Integrasi: ISO, log ke flashdisk, uji di laptop | driver yang sudah selesai | - |

Dua sesi yang menulis kode jangan menyentuh berkas yang sama secara
bersamaan. `devmgr/src/main.rs`, `devmgr/src/usb.rs`, `Cargo.toml` root, dan
`xtask/src/native.rs` dipakai hampir semua driver. Sesi yang jalan bersamaan
sebaiknya memakai worktree terpisah, lalu digabung satu per satu.

Aturan bersama (sudah disalin ke setiap prompt):

- Kode minimal ("ponytail"); penyederhanaan yang disengaja diberi komentar
  upgrade path.
- Lint workspace ketat: tanpa unwrap, expect, panic, atau indexing (pakai
  `.get()`); setiap `unsafe` wajib berkomentar `SAFETY:` dan hanya berisi
  satu operasi; `missing_docs` deny.
- Jangan ubah kernel (`src/kernel/`) tanpa tanya.
- `cargo xtask check` dijalankan terlepas (`setsid nohup`) dengan
  `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`, karena VM WSL hanya 6 GB.
- Jangan push atau buat PR. Commit hanya setelah disetujui. Repo ferrix
  menolak trailer Co-Authored-By.
- Bahasa Indonesia.

---

## Prompt 1: Audio Intel HDA

```text
Lanjutkan proyek Xaqu (OS berkernel Rust). Repo di WSL archlinux /root/ferrix, cabang rename/xaqu. Edit berkas lewat path \\wsl.localhost\archlinux\root\ferrix\...

Baca dulu C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md bagian 3 (pola driver Xaqu) dan bagian 4.1 (rancangan audio Intel HDA).

Tugas: driver audio Intel HDA untuk laptop saya (PCI 8086:a0c8, codec Realtek ALC269 10ec:0269, subsystem 088c:2166).
1. Crate logika src/lib/drivers/sound/hda (xaqu-hda) dengan tes host.
   - Controller: reset, CORB/RIRB dengan polling.
   - Codec: pencarian jalur pin ke DAC, unmute, EAPD, power D0.
   - Stream: BDL per halaman, posisi dari LPIB, ELAPSED yang urut, HALT.
   - Tesnya memakai codec palsu bergaya ALC269.
2. Program driver src/user/system/native/drivers/sound/hda (binary "hda"). Polanya ikuti drivers/sound/virtio-snd/src/main.rs (protokol sndctl) dan drivers/net/rtl8169/src/main.rs (BAR, aperture, interrupt atau polling).
3. Devmgr: tambah baris PCI_DRIVERS untuk kelas 04:03:00 dan untuk 8086:a0c8. Di start_sound, pilih program "snd" untuk virtio dan "hda" untuk lainnya.
4. Tambah Program di tools/common/xtask/src/native.rs, lalu members dan workspace.dependencies di Cargo.toml.
5. Tulis ke log: vendor id codec, subsystem, semua pin config default, dan bit snoop (PCI config 0x78), supaya bisa diperbaiki saat diuji di laptop.
6. Uji di QEMU dengan -device intel-hda -device hda-duplex. Cari tempat perangkat QEMU ditambahkan di tools/common/xtask/src/qemu.rs.
7. Jalankan cargo xtask check sampai hijau.

Aturan:
- Kode minimal; penyederhanaan yang disengaja diberi komentar upgrade path.
- Lint workspace ketat: tanpa unwrap/expect/panic/indexing (pakai .get()); setiap unsafe wajib berkomentar SAFETY dan berisi satu operasi.
- Jangan ubah kernel tanpa tanya saya.
- cargo xtask check dijalankan terlepas (setsid nohup) dengan CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2.
- Jangan push atau buat PR. Commit hanya setelah saya setujui. Repo ferrix menolak trailer Co-Authored-By.
- Bahasa Indonesia.
```

## Prompt 2: USB tethering HP Android

```text
Lanjutkan proyek Xaqu. Repo di WSL archlinux /root/ferrix, cabang rename/xaqu. Baca C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md bagian 3 dan 4.2.

Langkah 1, jawab dulu sebelum menulis kode: apakah net ring kernel (NET_RING_CREATE, docs/NET-RING.md) bisa dibuat untuk driver kelas USB yang dimulai devmgr (pola usbserial, R2 di docs/USB-SERIAL.md §6.1), atau hanya untuk node perangkat kernel? Baca devmgr/src/main.rs (start_net), devmgr/src/usb.rs, src/user/system/native/drivers/usb/usbserial, dan src/kernel/src/interfaces. Laporkan hasilnya ke saya.

Langkah 2, hanya bila bisa tanpa mengubah kernel:
- Buat driver kelas USB "usbnet" untuk RNDIS (kelas interface E0/01/03 dan EF/04/01), lalu CDC-NCM (02/0D) bila waktunya cukup.
- Logikanya di crate src/lib/drivers/net/usbnet dengan tes host. RNDIS: INITIALIZE, QUERY alamat MAC, SET packet filter, frame data dengan header 44 byte.
- Sisi jaringan memakai xaqu-netserve, seperti virtio-net dan rtl8169.
- Di devmgr/src/usb.rs, USB_DRIVERS ditambah pencocokan berdasarkan kelas interface.
- Uji di QEMU dengan -device usb-net.
- Jalankan cargo xtask check sampai hijau.

Kalau butuh perubahan kernel: berhenti, jelaskan pilihan dan risikonya, lalu tanya saya.

Aturan:
- Kode minimal dengan komentar upgrade path.
- Lint workspace ketat: tanpa unwrap/expect/panic/indexing; setiap unsafe wajib berkomentar SAFETY.
- cargo xtask check terlepas dengan CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2.
- Jangan push. Commit hanya dengan persetujuan saya, tanpa trailer Co-Authored-By di repo ferrix.
- Bahasa Indonesia.
```

## Prompt 3: Data ACPI touchpad (tanpa kode)

```text
Di laptop Windows saya, ambil tabel ACPI DSDT (dan SSDT bila ada) dari registry HKLM\HARDWARE\ACPI. Simpan ke C:\Users\jeani\Downloads\OS\docs\acpi\, lalu decompile dengan iasl. Kalau iasl belum ada, pakai paket acpica di WSL archlinux.

Dari hasil decompile, cari perangkat touchpad Goodix (HID GXTP786, CID PNP0C50) dan catat:
- kontroler I2C yang dipakai: nama ACPI, alamat PCI (kemungkinan 8086:a0e8, Serial IO I2C), dan nomor bus;
- alamat slave I2C dan kecepatan bus (I2cSerialBusV2 di _CRS);
- alamat register HID descriptor (_DSM dengan GUID 3cdff6f7-4267-4555-ad05-b30a3d8938de);
- interrupt: GpioInt (kontroler pinctrl INT34C5, nomor pin, mode trigger, polaritas) atau Interrupt biasa;
- method _PS0/_PS3/_STA yang menyalakan touchpad, beserta isinya.

Tulis temuannya ke bagian 4.6 di C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md. Cari juga di DSDT perangkat MSFT0001/PNP0303 (keyboard PS/2) dan catat port serta IRQ-nya di bagian 4.3.

Jangan menulis kode driver dan jangan mengubah repo /root/ferrix. Data ini hanya dipakai di komputer saya; jangan dikirim ke layanan luar. Bahasa Indonesia.
```

## Prompt 4: Keyboard bawaan PS/2

```text
Lanjutkan proyek Xaqu. Repo di WSL archlinux /root/ferrix, cabang rename/xaqu. Baca C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md bagian 3 dan 4.3, lalu docs/DRIVER-MODEL.md (G9 di §2, B3 di §4.4, aturan "driver tidak boleh mengubah kernel").

Keyboard bawaan laptop saya adalah i8042 (port 0x60/0x64, IRQ1). Driver ring-3 butuh akses port I/O, dan Xaqu belum punya mekanismenya (G9).

Langkah 1, jangan menulis kode dulu. Susun rancangan G9 dan tunjukkan ke saya:
- pemberian port I/O lewat bitmap I/O di TSS x86-64, sebagai objek yang diberikan devmgr;
- penolakan port kernel, 0xCF8-0xCFF, dan port DMA ISA;
- cara IRQ1 (GSI 1 di IOAPIC) diberikan ke driver ring-3;
- node perangkat untuk i8042 tanpa AML: port standar, ditandai upgrade path ke acpid;
- perkiraan berkas kernel yang berubah dan tes negatifnya.
Ikuti gaya dokumen rancangan di docs/ dan tunggu persetujuan saya.

Langkah 2, setelah saya setujui:
- Bangun G9 sesuai rancangan.
- Buat crate logika src/lib/drivers/input/i8042 dengan tes host: inisialisasi kontroler, scancode set 2 dengan translasi ke set 1, kode tombol ke event evdev.
- Buat program driver "i8042" yang melayani inputctl (pola drivers/input/virtio-input, docs/INPUT.md §3.2).
- Tambahkan di devmgr dan native.rs.
- Uji di QEMU q35 (punya i8042).
- Jalankan cargo xtask check sampai hijau.

Aturan:
- Kode minimal dengan komentar upgrade path.
- Lint ketat; setiap unsafe wajib berkomentar SAFETY.
- cargo xtask check terlepas dengan CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2.
- Jangan push. Commit hanya dengan persetujuan saya, tanpa trailer Co-Authored-By di repo ferrix.
- Bahasa Indonesia.
```

## Prompt 5: GPU Intel Tiger Lake

```text
Lanjutkan proyek Xaqu. Repo di WSL archlinux /root/ferrix, cabang rename/xaqu. Baca C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md bagian 4.4, docs/INTEL-GPU.md, dan docs/N150.md.

GPU laptop saya Intel UHD Tiger Lake (PCI 8086:9a78, Gen12.0 Xe-LP). Driver "intel" (src/lib/drivers/gpu/intel-xelp, src/user/system/native/drivers/gpu/intel) hanya menerima ADL-N: device_info::lookup memakai ADLN_IDS, dan devmgr memakai ADLN_IDS.

Langkah 1, analisis dulu, jangan menulis kode:
- Bandingkan TGL dan ADL-N untuk semua yang dipakai driver: jumlah EU dan subslice, fuse media, workaround (Wa_*), GuC/HuC (TGL tidak wajib GuC), tabel PAT, MOCS, frekuensi timestamp, dan langkah power/forcewake. Sumbernya i915 di src/lib/drivers/gpu/intel-xelp/i915 dan pciids.h (ID TGL ada di sana).
- Cek syarat kernel: intel_gpu(), xaqu.intelgpu, xaqu.efifb, VT-d/translasi domain, dan isolated-interrupts.
Laporkan apa yang perlu berubah dan risikonya, termasuk risiko GPU hang dan cara mematikannya lewat opsi boot.

Langkah 2, setelah saya setujui:
- Tambah ID TGL dan DeviceInfo TGL di crate logika, dengan tes host.
- Tambah ID TGL di devmgr.
- Jalankan cargo xtask check sampai hijau.
- Driver harus tetap mati secara default kecuali ada opsi boot, supaya ISO tetap aman di laptop.

Aturan:
- Kode minimal dengan komentar upgrade path.
- Lint ketat; setiap unsafe wajib berkomentar SAFETY.
- Jangan ubah kernel tanpa tanya saya.
- cargo xtask check terlepas dengan CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2.
- Jangan push. Commit hanya dengan persetujuan saya, tanpa trailer Co-Authored-By di repo ferrix.
- Bahasa Indonesia.
```

## Prompt 6: Touchpad I2C HID

```text
Lanjutkan proyek Xaqu. Repo di WSL archlinux /root/ferrix, cabang rename/xaqu. Baca C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md bagian 3 dan 4.6. Bagian 4.6 harus sudah berisi data ACPI touchpad dari prompt 3; kalau belum ada, berhenti dan beri tahu saya.

Touchpad laptop saya: Goodix GXTP786, HID-over-I2C di kontroler Intel Serial IO I2C (PCI 8086:a0e8, DesignWare).

Langkah 1, analisis dulu: tentukan apakah interrupt GPIO (pinctrl Tiger Lake INT34C5) bisa sampai ke driver ring-3 tanpa mengubah kernel (G5 di docs/DRIVER-MODEL.md). Kalau tidak bisa, usulkan polling sebagai jalan pintas (misalnya tiap 8 ms, diberi komentar upgrade path). Laporkan ke saya sebelum menulis kode.

Langkah 2, setelah saya setujui:
- Buat crate src/lib/drivers/i2c/designware dengan tes host: inisialisasi kontroler, transfer tulis-baca, timing SCL dari data ACPI.
- Buat crate src/lib/drivers/input/i2c-hid dengan tes host: ambil HID descriptor, report descriptor, dan input report, lalu ubah ke event inputctl. Pakai ulang parser HID di src/lib/drivers/usb/usb-host/src/hid.rs dan report.rs bila memungkinkan, untuk sentuhan multi-jari dan klik.
- Buat program driver "i2chid" yang memegang PCI 8086:a0e8, dengan alamat touchpad dari data ACPI ditulis sebagai konstanta untuk laptop ini (komentar upgrade path ke acpid).
- Tambahkan di devmgr dan native.rs.
- Jalankan cargo xtask check sampai hijau. Uji perangkat asli menyusul di laptop.

Aturan:
- Kode minimal dengan komentar upgrade path.
- Lint ketat; setiap unsafe wajib berkomentar SAFETY.
- Jangan ubah kernel tanpa tanya saya.
- cargo xtask check terlepas dengan CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2.
- Jangan push. Commit hanya dengan persetujuan saya, tanpa trailer Co-Authored-By di repo ferrix.
- Bahasa Indonesia.
```

## Prompt 7: Kamera UVC

```text
Lanjutkan proyek Xaqu. Repo di WSL archlinux /root/ferrix, cabang rename/xaqu. Baca C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md bagian 3 dan 4.5, docs/USB-SERIAL.md §6, dan docs/DRIVER-MODEL.md §5-§6 (G3, G4, K5).

Kamera laptop saya: USB 2b7e:b514, kelas video UVC (0E).

Langkah 1, analisis dulu: Xaqu belum punya core kernel untuk video (/dev/video0, V4L2). Bandingkan dua jalan:
(a) forwarding gaya CUSE (K5), sehingga driver ring-3 menyajikan /dev/video0 tanpa core kernel baru;
(b) program ruang pengguna yang membaca kamera lewat klaim antarmuka USB (R3) tanpa /dev/video0.
Sebutkan aplikasi apa di Xaqu yang akan memakai kamera (Chrome?) dan jalan mana yang dibutuhkannya. Laporkan ke saya sebelum menulis kode.

Langkah 2, setelah saya setujui: buat driver kelas USB "uvc" dengan tes host (descriptor UVC, probe/commit, aliran isochronous atau bulk dalam format MJPEG atau YUYV), sesuai jalan yang dipilih. Tambahkan di devmgr/src/usb.rs dan native.rs, lalu jalankan cargo xtask check sampai hijau.

Aturan:
- Kode minimal dengan komentar upgrade path.
- Lint ketat; setiap unsafe wajib berkomentar SAFETY.
- Jangan ubah kernel tanpa tanya saya.
- cargo xtask check terlepas dengan CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2.
- Jangan push. Commit hanya dengan persetujuan saya, tanpa trailer Co-Authored-By di repo ferrix.
- Bahasa Indonesia.
```

## Prompt 8: Adaptor USB Ethernet (cadangan sebelum Wi-Fi)

```text
Lanjutkan proyek Xaqu. Repo di WSL archlinux /root/ferrix, cabang rename/xaqu. Baca C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md bagian 4.2 dan 4.7. Driver "usbnet" (USB tethering) harus sudah ada; kalau belum, berhenti dan beri tahu saya.

Tugas: tambah dukungan adaptor USB Ethernet murah ke driver usbnet yang sudah ada, sebagai jalan internet selain tethering:
- CDC-ECM (kelas 02/06), cara paling generik;
- Realtek RTL8153 (0bda:8153), adaptor paling umum. Kalau dukungan ECM bawaannya tidak cukup, buat mode vendornya mengikuti r8152 Linux secara minimal.
Sebutkan dulu adaptor mana yang sebaiknya saya beli, berdasarkan mana yang paling sedikit kodenya.

Buat tes host. Uji di QEMU dengan -device usb-net bila bisa. Jalankan cargo xtask check sampai hijau.

Aturan:
- Kode minimal dengan komentar upgrade path.
- Lint ketat; setiap unsafe wajib berkomentar SAFETY.
- Jangan ubah kernel tanpa tanya saya.
- cargo xtask check terlepas dengan CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2.
- Jangan push. Commit hanya dengan persetujuan saya, tanpa trailer Co-Authored-By di repo ferrix.
- Bahasa Indonesia.
```

## Prompt 9: Wi-Fi Intel AC 9560

```text
Lanjutkan proyek Xaqu. Repo di WSL archlinux /root/ferrix, cabang rename/xaqu. Baca C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md bagian 3 dan 4.7, dan docs/NET-RING.md.

Wi-Fi laptop saya: Intel Wireless-AC 9560 (PCI 8086:a0f0, subsystem 8086:0034, CNVi, keluarga iwlwifi 9000 "jf"). Ini pekerjaan sangat besar. Pecah menjadi tahap yang masing-masing bisa diverifikasi, dan jangan menulis semuanya sekaligus.

Langkah 1, rencana saja, tanpa kode:
- Peta komponen yang dibutuhkan, setara dengan iwlwifi pcie trans, iwlmvm, mac80211, dan supplicant WPA2-PSK. Untuk tiap komponen sebutkan pilihan: tulis sendiri di Rust, port, atau pakai crate yang ada. Sebutkan lisensinya (audit di tools/common/data/release-licences.toml).
- Firmware: nama berkas iwlwifi-9000-pu-b0-jf-b0-*.ucode yang cocok, lisensinya, dan cara memasukkannya ke ISO.
- Tahapan dengan titik uji: (1) firmware termuat dan alive, (2) pindai jaringan dan tampilkan daftar SSID di log, (3) asosiasi ke jaringan terbuka, (4) WPA2-PSK, (5) jalur data lewat net ring, (6) UI pemilihan jaringan di desktop.
- Perkiraan ukuran kode per tahap dan risikonya.
- Jalur uji: QEMU tidak bisa meniru kartu ini. Usulkan cara uji di laptop, misalnya log ke flashdisk.
Tunggu persetujuan saya sebelum mulai tahap 1.

Aturan:
- Kode minimal dengan komentar upgrade path.
- Lint ketat; setiap unsafe wajib berkomentar SAFETY.
- Jangan ubah kernel tanpa tanya saya.
- cargo xtask check terlepas dengan CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2.
- Jangan push. Commit hanya dengan persetujuan saya, tanpa trailer Co-Authored-By di repo ferrix.
- Bahasa Indonesia.
```

## Prompt 10: Bluetooth Intel

```text
Lanjutkan proyek Xaqu. Repo di WSL archlinux /root/ferrix, cabang rename/xaqu. Baca C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md bagian 4.8.

Bluetooth laptop saya: Intel 8087:0aaa (USB, pasangan Wi-Fi AC 9560). Pakai utama yang saya inginkan: mouse/keyboard Bluetooth (HID) dan headset audio.

Langkah 1, rencana saja, tanpa kode:
- Komponen: driver kelas USB btusb (HCI lewat USB), pemuatan firmware Intel (ibt-17-16-1.sfi atau yang cocok, beserta lisensinya), HCI, L2CAP, SMP (pairing), HID over L2CAP/HOGP, lalu A2DP bila audio diperlukan.
- Bagaimana perangkat HID Bluetooth sampai ke inputctl, dan bagaimana audio sampai ke sndctl.
- Tahapan dengan titik uji: (1) firmware termuat, (2) pindai perangkat, (3) pairing mouse, (4) input mouse jalan, (5) headset.
- Perkiraan ukuran dan risiko.
Tunggu persetujuan saya.

Aturan:
- Kode minimal dengan komentar upgrade path.
- Lint ketat; setiap unsafe wajib berkomentar SAFETY.
- Jangan ubah kernel tanpa tanya saya.
- cargo xtask check terlepas dengan CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2.
- Jangan push. Commit hanya dengan persetujuan saya, tanpa trailer Co-Authored-By di repo ferrix.
- Bahasa Indonesia.
```

## Prompt 11: Mikrofon digital (DSP Smart Sound / SOF)

```text
Lanjutkan proyek Xaqu. Repo di WSL archlinux /root/ferrix, cabang rename/xaqu. Baca C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md bagian 4.1 dan 4.8. Driver audio "hda" harus sudah ada.

Mikrofon laptop saya adalah mikrofon digital (DMIC) di belakang DSP Intel Smart Sound (PCI 8086:a0c8, Tiger Lake). Di Linux jalurnya lewat Sound Open Firmware (SOF).

Langkah 1, kajian saja, tanpa kode:
- Apa yang dibutuhkan: firmware SOF (sof-tgl.ri) dan topologi (sof-hda-generic-2ch.tplg atau yang cocok dengan NHLT laptop ini), IPC ke DSP, jalur capture DMIC.
- Apakah ada jalan lebih pendek, misalnya mikrofon headset lewat codec HDA biasa (ALC269 punya input analog), yang bisa ditambahkan ke driver hda sebagai stream capture.
- Lisensi firmware SOF untuk ISO.
- Ukuran kerja dan rekomendasi.
Tunggu persetujuan saya.

Aturan:
- Kode minimal dengan komentar upgrade path.
- Lint ketat; setiap unsafe wajib berkomentar SAFETY.
- Jangan ubah kernel tanpa tanya saya.
- cargo xtask check terlepas dengan CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2.
- Jangan push. Commit hanya dengan persetujuan saya, tanpa trailer Co-Authored-By di repo ferrix.
- Bahasa Indonesia.
```

## Prompt 12: Sidik jari Goodix

```text
Kajian saja, tanpa kode dan tanpa mengubah repo.

Sensor sidik jari laptop saya: Goodix USB 27c6:5e0a. Cari tahu dengan sumber yang bisa diverifikasi (libfprint, proyek goodix-fp-linux-dev, dan sejenisnya):
- apakah protokol perangkat ini sudah dipetakan atau didukung di Linux;
- apakah butuh firmware atau kunci enkripsi khusus;
- berapa besar kerja untuk membuat drivernya di Xaqu (driver kelas USB di repo /root/ferrix, pola usbserial), dan apa gunanya di Xaqu (login?).
Jangan berhalusinasi: setiap klaim harus disertai sumbernya. Tulis kesimpulan singkat ke bagian 4.8 di C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md. Bahasa Indonesia.
```

## Prompt 13: Integrasi, ISO, dan uji di laptop

```text
Lanjutkan proyek Xaqu. Repo di WSL archlinux /root/ferrix, cabang rename/xaqu. Baca C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md dan C:\Users\jeani\Downloads\OS\docs\LAPORAN-KESIAPAN-ISO.md.

Tugas:
1. Daftar driver baru yang sudah ada di cabang ini (hda, usbnet, i8042, intel TGL, i2chid, uvc, dan lainnya). Gabungkan cabang worktree yang belum masuk, hanya setelah saya setujui.
2. Log untuk uji di laptop asli: laptop tidak punya port serial. Buat cara menyalin log boot dan log driver (lode/journal) ke flashdisk FAT32, otomatis saat boot live atau dengan satu perintah, beserta petunjuk singkatnya untuk saya.
3. Jalankan cargo xtask check sampai hijau, lalu cargo xtask iso dan cargo xtask test-iso. Salin ISO ke D:\Xaqu-ISO\ dan catat SHA-256-nya. C: sering hampir penuh, jadi cek ruang disk dulu.
4. Tulis checklist uji di laptop ke LAPORAN-KESIAPAN-ISO.md:
   - boot live, jangan klik Install;
   - per perangkat: apa yang dicoba, hasil yang diharapkan, dan nama log yang harus saya kirim balik.
5. Perbarui tabel status di PETA-DRIVER-LAPTOP.md.

Aturan:
- cargo xtask check terlepas dengan CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2.
- Jangan ubah kernel tanpa tanya saya.
- Jangan push. Commit hanya dengan persetujuan saya, tanpa trailer Co-Authored-By di repo ferrix.
- Bahasa Indonesia.
```
