# Prompt lengkap: semua driver laptop dalam satu sesi (2026-11-18)

Satu prompt untuk satu sesi panjang yang mengerjakan semua driver secara
bertahap. Keputusan keyboard sudah diambil: **keyboard memakai keyboard USB**.
Keyboard bawaan PS/2 tidak dikerjakan, dan kernel tidak diubah untuk itu.

Salin seluruh blok di bawah ke sesi baru.

```text
Lanjutkan proyek Xaqu (OS berkernel Rust dengan desktop gaya macOS). Repo ada di WSL archlinux /root/ferrix, cabang rename/xaqu. Edit berkas lewat path \\wsl.localhost\archlinux\root\ferrix\... App ada di repo terpisah src/user/apps.

Tujuan sesi ini: semua perangkat laptop saya berfungsi di Xaqu, dikerjakan bertahap dalam satu sesi sampai ada ISO baru yang siap saya uji di laptop.

== Bacaan wajib sebelum mulai ==
1. C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md: peta semua perangkat, pola driver Xaqu (bagian 3), dan rancangan tiap driver (bagian 4).
2. C:\Users\jeani\Downloads\OS\docs\LAPORAN-KESIAPAN-ISO.md: kondisi ISO terakhir.
3. Di repo: docs/DRIVER-MODEL.md (aturan "driver tidak boleh mengubah kernel", G3/G4/G5/G9), docs/AUDIO.md §3, docs/INPUT.md §3 dan §7, docs/NET-RING.md, docs/USB-SERIAL.md §6, docs/XHCI.md, docs/INTEL-GPU.md.

== Laptop saya ==
realme, Intel Core i3-1115G4 (Tiger Lake), RAM 8 GB, SSD NVMe Samsung.
- Audio: PCI 8086:a0c8 (HDA, mode DSP Smart Sound), codec Realtek ALC269 10ec:0269 subsystem 088c:2166, mikrofon digital lewat DSP.
- GPU: Intel UHD PCI 8086:9a78 (Gen12.0 Xe-LP).
- Wi-Fi: Intel AC 9560 PCI 8086:a0f0 subsystem 8086:0034 (CNVi). Bluetooth: USB 8087:0aaa.
- Touchpad: Goodix GXTP786, HID-over-I2C di Serial IO I2C PCI 8086:a0e8.
- Kamera: USB 2b7e:b514 (UVC). Sidik jari: Goodix USB 27c6:5e0a.
- USB xHCI: PCI 8086:9a13 dan 8086:a0ed.
- Keyboard bawaan: PS/2 (i8042). Keputusan saya: TIDAK dikerjakan. Saya memakai keyboard USB.
- Tidak ada port LAN dan tidak ada port serial.

== Cara kerja ==
- Kerjakan fase demi fase sesuai urutan di bawah. Setiap fase selesai dengan:
  - tes host lolos;
  - uji QEMU, bila perangkatnya bisa ditiru;
  - cargo xtask check hijau;
  - ringkasan singkat untuk saya.
  Setelah itu lanjut ke fase berikutnya tanpa menunggu, kecuali di titik yang ditandai [TANYA].
- Pekerjaan yang saling lepas boleh dikerjakan paralel dengan subagent, masing-masing di worktree terpisah. Hasilnya digabung satu per satu, karena devmgr/src/main.rs, devmgr/src/usb.rs, Cargo.toml root, dan tools/common/xtask/src/native.rs dipakai hampir semua driver.
- Ikuti pola driver di PETA bagian 3: crate logika di src/lib/drivers/<kelas>/<nama> dengan tes host, program driver di src/user/system/native/drivers/<kelas>/<nama>, baris di devmgr, Program di native.rs, lalu members dan workspace.dependencies di Cargo.toml.
- Laptop tidak punya port serial, jadi setiap driver harus menulis ke log semua yang dibutuhkan untuk memperbaikinya dari jauh: ID, versi, register penting, dan alasan gagal.
- Catat kemajuan di tabel status PETA-DRIVER-LAPTOP.md setiap kali satu fase selesai, supaya sesi lain bisa melanjutkan bila sesi ini terputus.

== Fase 0: Persiapan ==
a. Cek kondisi: git status di ketiga repo, ruang disk C: dan D:, dan apakah check terakhir hijau.
b. Ambil tabel ACPI DSDT/SSDT dari registry Windows (HKLM\HARDWARE\ACPI), simpan di C:\Users\jeani\Downloads\OS\docs\acpi\, dan decompile dengan iasl (paket acpica di WSL). Catat data touchpad ke PETA bagian 4.6: kontroler dan bus I2C, alamat slave, kecepatan, alamat HID descriptor (_DSM), GpioInt (pinctrl INT34C5, pin, trigger, polaritas), serta _PS0/_PS3/_STA. Data ini tidak boleh dikirim ke layanan luar.

== Fase 1: Keyboard dan mouse USB ==
Ini pengganti keyboard bawaan. Pastikan keyboard dan mouse USB benar-benar siap di laptop ini:
a. Telusuri jalur usbhid (src/user/system/native/drivers/usb/usbhid, src/lib/drivers/usb/usb-host: xhci.rs, hid.rs, report.rs) untuk keyboard dan mouse USB di kontroler xHCI Intel Tiger Lake (8086:9a13 dan 8086:a0ed). Periksa: apakah kedua kontroler diikat (devmgr cocok kelas 0C:03:30), port USB 3 dan USB-C, hub internal, colok-cabut saat jalan, keyboard boot protocol dan report protocol, tombol multimedia, mouse dengan roda, receiver wireless 2.4 GHz (composite dengan beberapa interface), dan LED Caps Lock.
b. Cek docs/XHCI.md dan BACKLOG untuk catatan "real hardware" atau quirk Intel xHCI: BIOS handoff (USBLEGSUP), port power, dan IOMMU/DMA gate di perangkat asli. Perbaiki yang kurang, dengan tes host.
c. Pastikan layout keyboard (docs/INPUT.md §4a) bisa dipilih di Settings dan default-nya US.
d. Uji di QEMU dengan usb-kbd dan usb-mouse/usb-tablet di qemu-xhci.
e. Tambahkan catatan di LAPORAN-KESIAPAN-ISO.md bahwa keyboard bawaan dan touchpad tidak berfungsi sampai fase 6, jadi pakai keyboard dan mouse USB.

== Fase 2: Audio Intel HDA ==
Ikuti rancangan PETA bagian 4.1 sepenuhnya:
- crate xaqu-hda: controller, CORB/RIRB polling, codec, jalur pin ke DAC, stream BDL per halaman, posisi dari LPIB, ELAPSED urut, HALT;
- program "hda";
- devmgr: kelas 04:03:00 dan 8086:a0c8; start_sound memilih "snd" untuk virtio dan "hda" untuk lainnya;
- uji QEMU dengan -device intel-hda -device hda-duplex;
- log: vendor id codec, subsystem, semua pin config, dan bit snoop (PCI config 0x78).
Setelah playback jalan, tambahkan capture dari mikrofon analog headset lewat ALC269, bila protokol sndctl mendukung stream capture.

== Fase 3: Internet lewat USB ==
a. Jawab dulu: apakah net ring kernel (NET_RING_CREATE) bisa dibuat untuk driver kelas USB yang dimulai devmgr (pola usbserial, R2), tanpa mengubah kernel? [TANYA] bila butuh perubahan kernel; jelaskan pilihan dan risikonya.
b. Buat driver "usbnet":
   - RNDIS (E0/01/03 dan EF/04/01) untuk USB tethering HP Android;
   - CDC-NCM (02/0D);
   - CDC-ECM (02/06) untuk adaptor USB Ethernet;
   - devmgr/src/usb.rs dicocokkan berdasarkan kelas interface.
   Uji di QEMU dengan -device usb-net.
c. Pastikan DHCP (dhcpc) jalan di antarmuka itu dan desktop menampilkan status jaringan.
d. Sebutkan adaptor USB Ethernet mana yang sebaiknya saya beli, yaitu yang langsung jalan lewat CDC-ECM/NCM tanpa kode vendor.

== Fase 4: GPU Tiger Lake ==
a. Analisis beda TGL dan ADL-N untuk semua yang dipakai driver intel-xelp:
   - EU dan subslice, fuse media, workaround Wa_*;
   - GuC/HuC, PAT, MOCS, timestamp, forcewake;
   - syarat kernel: intel_gpu(), xaqu.intelgpu, xaqu.efifb, VT-d.
b. Tambahkan ID TGL dari i915/include/pciids.h dan DeviceInfo TGL, dengan tes host. Driver harus tetap mati secara default dan baru nyala dengan opsi boot, supaya ISO aman bila GPU hang. Tulis cara menyalakannya di laporan.
c. [TANYA] bila ternyata butuh perubahan kernel.

== Fase 5: Kamera UVC ==
a. Tentukan jalur yang paling minimal tanpa core kernel baru: forwarding gaya CUSE (K5), atau klaim antarmuka USB (R3), sesuai apa yang dipakai Chrome di Xaqu. [TANYA] bila semua jalur butuh perubahan kernel.
b. Buat driver kelas USB "uvc" (descriptor UVC, probe/commit, MJPEG atau YUYV) dengan tes host.

== Fase 6: Touchpad I2C HID ==
Butuh data dari fase 0b.
a. Cek apakah interrupt GPIO bisa sampai ke ring-3 tanpa perubahan kernel (G5). Kalau tidak bisa, pakai polling tiap 8 ms dengan komentar upgrade path.
b. Buat crate src/lib/drivers/i2c/designware dan src/lib/drivers/input/i2c-hid. Pakai ulang parser HID dari usb-host. Dukung multi-jari, klik, scroll dua jari, dan tap-to-click bila laporan HID-nya mendukung.
c. Buat program "i2chid" yang memegang PCI 8086:a0e8. Alamat touchpad dari DSDT ditulis sebagai konstanta untuk laptop ini (komentar upgrade path ke acpid).

== Fase 7: Wi-Fi Intel AC 9560 ==
Pekerjaan terbesar. [TANYA] sebelum menulis kode, dengan membawa rencana bertahap:
- komponen setara iwlwifi pcie trans, iwlmvm, mac80211, dan supplicant WPA2-PSK, dengan pilihan untuk tiap komponen (tulis sendiri di Rust, port, atau crate yang ada) beserta lisensinya;
- firmware iwlwifi-9000-pu-b0-jf-b0-*.ucode: nama berkas, lisensi, cara masuk ISO, dan baris di tools/common/data/release-licences.toml;
- tahap dengan titik uji: firmware alive, pindai SSID di log, asosiasi jaringan terbuka, WPA2-PSK, jalur data lewat net ring, UI pemilihan jaringan di desktop;
- perkiraan ukuran kode per tahap.
Setelah saya setujui, kerjakan tahap per tahap.

== Fase 8: Bluetooth Intel ==
[TANYA] sebelum menulis kode, dengan rencana:
- btusb, firmware Intel (ibt-*.sfi) beserta lisensinya, HCI, L2CAP, SMP;
- HID (mouse/keyboard Bluetooth) ke inputctl, lalu A2DP headset ke sndctl;
- tahap: firmware, pindai, pairing, input, audio.

== Fase 9: Mikrofon digital dan sidik jari (kajian saja) ==
a. Mikrofon digital lewat DSP SOF: kebutuhan (sof-tgl.ri, topologi dari NHLT laptop ini, IPC DSP), lisensi, ukuran kerja, dan rekomendasi. Tanpa kode.
b. Sidik jari Goodix 27c6:5e0a: apakah protokolnya sudah dipetakan (libfprint, goodix-fp-linux-dev), apakah butuh kunci enkripsi, dan ukuran kerjanya. Setiap klaim harus ada sumbernya. Tanpa kode.
Tulis kesimpulan keduanya di PETA bagian 4.8.

== Fase 10: Integrasi dan ISO ==
a. Gabungkan semua worktree ke rename/xaqu.
b. Log ke flashdisk: buat cara menyalin log boot dan log driver (lode/journal) ke flashdisk FAT32, otomatis saat boot live atau dengan satu perintah, beserta petunjuk singkat untuk saya.
c. Jalankan cargo xtask check, cargo xtask iso, dan cargo xtask test-iso sampai semuanya lolos. Cek ruang C: dulu, karena sering hampir penuh. Salin ISO ke D:\Xaqu-ISO\ dan catat SHA-256-nya.
d. Tulis checklist uji di laptop ke LAPORAN-KESIAPAN-ISO.md:
   - boot live, jangan klik Install;
   - per perangkat: apa yang dicoba, hasil yang diharapkan, dan log yang harus saya kirim balik.
e. Perbarui tabel status di PETA-DRIVER-LAPTOP.md: apa yang selesai, apa yang menunggu uji di laptop, dan apa yang menunggu keputusan saya.

== Aturan ==
- Kode minimal ("ponytail"). Penyederhanaan yang disengaja ditandai komentar upgrade path. Tanpa fitur spekulatif.
- Lint workspace ketat:
  - deny: missing_docs, unwrap_used, expect_used, panic, indexing_slicing (pakai .get()), too_many_lines, excessive_nesting;
  - setiap unsafe wajib berkomentar SAFETY dan berisi satu operasi.
- Jangan ubah kernel (src/kernel/) tanpa tanya saya.
- cargo xtask check dijalankan terlepas (setsid nohup ... &) dengan CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2, karena VM WSL hanya 6 GB dan check penuh bisa lebih dari 30 menit. Pantau lewat berkas penanda, bukan menunggu di foreground.
- Jangan berhalusinasi: register, ID, dan nama firmware diambil dari sumber (i915/iwlwifi di repo, spesifikasi, Linux), bukan dari ingatan.
- Jangan push atau buat PR. Commit hanya setelah saya setujui. Repo ferrix menolak trailer Co-Authored-By. Identitas git: samakan dengan commit sebelumnya.
- Data pribadi dan tabel ACPI laptop saya tidak boleh dikirim ke layanan luar.
- Pakai Bahasa Indonesia. Laporan singkat dan jujur: sebutkan apa yang terbukti dan apa yang belum diuji di perangkat asli.
```
