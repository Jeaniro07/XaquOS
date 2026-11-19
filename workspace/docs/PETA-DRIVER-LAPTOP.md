# Peta driver Xaqu untuk laptop realme (2026-11-18)

Tujuan: semua perangkat laptop ini berfungsi di Xaqu. Dokumen ini hanya peta;
belum ada kode driver baru yang ditulis. Pengerjaan dilanjutkan di sesi lain.

Repo: WSL archlinux `/root/ferrix`, cabang `rename/xaqu` (commit terakhir
`45e4c42e6`). App ada di repo terpisah `src/user/apps`.

## 1. Laptop

- realme RMNBXXXX, Intel Core i3-1115G4 (Tiger Lake-UP3), RAM 8 GB
- Dibaca dari Windows (Win32_PnPEntity), ID PCI/USB di bawah adalah ID asli.

## 2. Tabel semua perangkat

| # | Perangkat | ID | Status di Xaqu | Besar kerja | Uji di QEMU |
|---|---|---|---|---|---|
| 1 | CPU, RAM | - | Jalan | - | Ya |
| 2 | SSD NVMe Samsung MZVLQ256 | PCI 144d:a809 | Ada (`nvme`) | - | Ya |
| 3 | USB xHCI (2 kontroler) | PCI 8086:9a13, 8086:a0ed | Ada (`usbhid`, host xHCI) | - | Ya |
| 4 | Keyboard/mouse USB | kelas HID | Ada (`usbhid`) | - | Ya |
| 5 | Keyboard bawaan | ACPI MSFT0001 (i8042, port 0x60/0x64, IRQ1) | Belum | Kecil, tapi butuh keputusan kernel (bagian 4.3) | Ya (q35 punya i8042) |
| 6 | Audio speaker/headphone | PCI 8086:a0c8 (HDA, mode DSP), codec Realtek 10ec:0269 subsys 088c:2166 | Belum | Sedang | Ya (`intel-hda` + `hda-duplex`) |
| 7 | Internet lewat USB tethering HP Android | USB kelas RNDIS (E0/01/03 atau EF/04/01) atau CDC-NCM (02/0D) | Belum | Sedang | Ya (`usb-net`) |
| 8 | GPU Intel UHD (Tiger Lake Gen12.0) | PCI 8086:9a78 | Driver `intel` hanya mengikat ADL-N (N150). Layar jalan lewat framebuffer firmware (`efifb`), tanpa akselerasi | Sedang, risiko hang | Tidak |
| 9 | Kamera 720p | USB 2b7e:b514 (UVC) | Belum | Sedang | Sebagian (`usb-host` webcam dari host) |
| 10 | Touchpad Goodix | ACPI GXTP786, I2C HID di PCI 8086:a0e8 (Serial IO I2C) | Belum, fondasi I2C/GPIO/ACPI juga belum | Besar | Tidak |
| 11 | Wi-Fi Intel AC 9560 | PCI 8086:a0f0 subsys 8086:0034 (CNVi) | Belum | Sangat besar | Tidak |
| 12 | Bluetooth Intel | USB 8087:0aaa | Belum | Besar | Tidak |
| 13 | Mikrofon digital | DSP Smart Sound 8086:a0c8 (SOF) | Belum | Sangat besar | Tidak |
| 14 | Sidik jari Goodix | USB 27c6:5e0a | Belum | Protokol tertutup, tidak realistis | Tidak |
| 15 | Lain-lain (tidak perlu driver untuk dipakai) | ME 8086:a0e0, SMBus a0a3, SPI flash a0a4, UART a0a8, SPI a0ab, termal 9a03, PMT 9a0d, RAM ctrl a0ef | Tidak perlu | - | - |

Tidak ada port LAN di laptop ini, jadi internet hanya lewat Wi-Fi atau USB
(tethering HP, atau adaptor USB Ethernet).

## 3. Cara membuat driver di Xaqu (pola yang sudah ada)

Setiap driver terdiri atas:

1. **Crate logika** `src/lib/drivers/<kelas>/<nama>`: tanpa syscall, di atas
   trait register dan memori DMA, diuji di host (contoh `xaqu-rtl8169`,
   `xaqu-virtio-snd`).
2. **Program driver** `src/user/system/native/drivers/<kelas>/<nama>`:
   `#![no_std] #![no_main]`, `xaqu_rt::entry!(main)`, membaca START dari
   bootstrap channel, memetakan BAR (`device.aperture(i)` lalu
   `device.io_mapping`), interrupt (`device.interrupt(i)`, boleh polling bila
   gagal, seperti `rtl8169`), memori DMA (`vmo::create` + `device.pin`).
   Contoh paling dekat: `drivers/sound/virtio-snd/src/main.rs` (715 baris)
   dan `drivers/net/rtl8169/src/main.rs`.
3. **devmgr** `src/user/system/native/devmgr/src/main.rs`: tambah baris di
   `PCI_DRIVERS` (cocok vendor + kelas/ID), dan pilih nama program di
   `start_sound` / `start_net` per vendor (seperti `start_net` memilih
   `yt6801` / `rtl8169`). Driver USB di `devmgr/src/usb.rs`, tabel
   `USB_DRIVERS` (sekarang hanya cocok vendor/produk).
4. **Image**: tambah `Program { package, binary, directory: DRIVERS }` di
   `tools/common/xtask/src/native.rs`. Batas kernel 32 image driver (G7 di
   `docs/DRIVER-MODEL.md`).
5. **Workspace**: tambah ke `members` dan `[workspace.dependencies]` di
   `Cargo.toml` root.
6. **Lint ketat** (workspace): `missing_docs`, `unwrap_used`, `expect_used`,
   `panic`, `indexing_slicing`, `too_many_lines`, `excessive_nesting` semua
   deny; setiap blok `unsafe` wajib komentar `SAFETY:` dan satu operasi saja.
   Pakai `.get()` bukan `[i]`.
7. Lisensi: berkas di `lib/drivers/*` sudah tercakup aturan `[components.xaqu]`
   di `tools/common/data/release-licences.toml`.

Aturan arsitektur penting (`docs/DRIVER-MODEL.md`): **menulis driver tidak
boleh mengubah kernel**. Driver PCI memenuhi aturan ini. Yang melanggar:
port I/O (G9) dan perangkat ACPI tanpa AML (B3).

## 4. Rancangan per driver

### 4.1 Audio Intel HDA (#6), urutan pertama

Sesuai aturan (driver PCI biasa). Rancangan yang sudah dipelajari:

- **Protokol ke kernel** sudah ada: `src/lib/proto/sndctl` (HELLO, READY,
  SUBMIT, ELAPSED, HALT, HALTED, STOP). Konfigurasi tetap versi 1: S16_LE
  (format ALSA 2), 2 kanal, 48 kHz, period 3840 byte, buffer 61440 byte
  (15 halaman), buffer milik core dan di-pin **read-only**. SUBMIT boleh
  sebagian period (saat tidak ada yang berjalan), tidak pernah melewati batas
  period, selalu bersambung. ELAPSED harus urut dan hanya untuk SUBMIT yang
  sedang berjalan; HALTED membawa jumlah SUBMIT yang belum dimainkan.
- **devmgr**: tambah `(ANY_VENDOR, PciMatch::Interface(0x04, 0x03, 0x00), b"hda", Kind::Sound)`
  dan `(INTEL_VENDOR, PciMatch::Devices(&[0xA0C8]), b"hda", Kind::Sound)`
  (0xA0C8 pada mode DSP berkelas 04:01, bukan 04:03). `start_sound` memilih
  program `snd` untuk virtio, `hda` untuk lainnya; START tetap bentuk blk
  (blok virtio bernilai nol, driver memakai `device.info()` dan aperture).
- **Kontroler** (BAR0, MMIO): reset `GCTL.CRST` (0x08) 0 lalu 1, tunggu
  codec di `STATESTS` (0x0E). `GCAP` (0x00): OSS bit 15:12, ISS bit 11:8.
  CORB/RIRB dengan polling (tanpa interrupt): satu halaman DMA, CORB di 0
  (256 x 4 byte), RIRB di 1024 (256 x 8 byte), BDL di 3072. Register
  CORBLBASE 0x40, CORBWP 0x48, CORBRP 0x4A, CORBCTL 0x4C, CORBSIZE 0x4E,
  RIRBLBASE 0x50, RIRBWP 0x58, RINTCNT 0x5A, RIRBCTL 0x5C, RIRBSIZE 0x5E.
- **Codec**: verb 12-bit `(cad<<28)|(nid<<20)|(verb<<8)|payload`, verb 4-bit
  `(cad<<28)|(nid<<20)|(verb<<16)|payload16`. GET_PARAMETER 0xF00 (0x04 node
  count, 0x05 function type, 0x09 widget cap, 0x0C pin cap, 0x0E conn list
  len, 0x12 amp out cap), GET_CONN_LIST 0xF02, GET_CONFIG_DEFAULT 0xF1C,
  SET_CONN_SELECT 0x701, SET_POWER_STATE 0x705 (D0), SET_CHANNEL_STREAMID
  0x706, SET_PIN_WIDGET_CONTROL 0x707 (0x40 out, 0x80 HP), SET_EAPD 0x70C
  (0x02), SET_AMP_GAIN_MUTE 0x3 (4-bit), SET_CONVERTER_FORMAT 0x2 (4-bit).
- **Jalur**: pin dengan default device Speaker (1), HP Out (2) atau Line Out
  (0) dan konektivitas bukan "none"; DFS lewat daftar koneksi (mixer,
  selector) ke widget output (tipe 0); set conn select, unmute amp ke 0 dB,
  EAPD, power D0; semua DAC terpilih memakai stream tag yang sama (speaker
  dan headphone berbunyi bersamaan; deteksi jack = upgrade path).
- **Stream**: output stream pertama di `0x80 + ISS*0x20`; CTL (SRST bit0,
  RUN bit1, tag bit 23:20), LPIB +4, CBL +8, LVI +0xC, FMT +0x12 (48k S16 2ch
  = 0x0011), BDPL +0x18. BDL dipecah per halaman (alamat harus kelipatan
  128). Mainkan siklik dari offset SUBMIT pertama (dibulatkan turun ke 128,
  BDL diputar), hitung posisi dari LPIB dengan timer (tanpa interrupt),
  kirim ELAPSED saat posisi melewati akhir SUBMIT, hentikan stream saat
  antrean kosong. Penyederhanaan ini ditandai komentar upgrade path
  (interrupt IOC, buffer posisi DMA, CORB/RIRB dengan interrupt).
- **Risiko di laptop**: mode DSP (SOF) aktif dari BIOS; mode HDA legacy
  biasanya tetap bisa memutar speaker/headphone (mikrofon digital tidak).
  ALC269 varian realme mungkin butuh fixup pin/COEF; log `vendor id`,
  `subsystem`, dan semua pin config ke log supaya bisa diperbaiki.
  Cek juga snoop (config 0x78 bit 11) dan catat ke log.
- QEMU: `-device intel-hda -device hda-duplex`. Cari tempat xtask menambah
  perangkat QEMU di `tools/common/xtask/src/qemu.rs`.

#### Hasil Fase 2 — 2026-11-18 (implementasi + koreksi rancangan)

Sudah dikerjakan di repo `/root/ferrix`:

- **Crate logika `xaqu-hda`** (`src/lib/drivers/sound/hda/`): `controller.rs`
  (register + reset), `corb.rs` (verb + ring CORB/RIRB), `codec.rs`
  (parameter, widget, pin, amp, connection list), `path.rs` (pencarian jalur
  konverter→pin), `bdl.rs` (descriptor), `control.rs` (pesan START).
  **22 tes host hijau**.
- **Program native `hda`** (`src/user/system/native/drivers/sound/hda/`):
  START sendiri, peta register block lewat `device.aperture` + `io_mapping`,
  reset, CORB/RIRB di memori yang di-pin, pembacaan node codec, pemilihan
  pin (speaker → headphone → line out), pemrograman jalur, HELLO/READY ke
  `xaqu_sndctl`, pemrograman stream + BDL, dan jawaban SUBMIT/ELAPSED.
  Lolos build untuk target `x86_64-unknown-none`.
- **Integrasi**: `devmgr` punya entri `HDA_INTERFACE` (04:03:00) → program
  `hda` dengan `Kind::Sound`, dan `start_hda_sound` mengirim START bentuk HDA
  (bukan bentuk blk); `xtask` membawa program `hda` di direktori driver;
  `Cargo.toml` workspace memuat dua paket baru.
- **Dokumen**: `docs/HDA.md` di repo (register, verb, parameter, bentuk
  program, yang belum diverifikasi).

**Koreksi terhadap rancangan di atas** (sumber: definisi QEMU
`hw/audio/intel-hda-defs.h` + `hw/audio/intel-hda.c`, yang menyalin
`linux/sound/pci/hda/hda_intel.c` — bukan ingatan):

1. **CORB/RIRB bukan di register block.** Keduanya ada di memori DMA yang
   dibuat driver; register block hanya memuat alamat dasarnya
   (CORBLBASE/CORBUBASE 0x40/0x44, RIRBLBASE/RIRBUBASE 0x50/0x54) dan
   pointer-nya. "CORB di 0, RIRB di 1024, BDL di 3072" tidak benar.
2. **BDL juga di memori DMA**, alamatnya di BDLPL/BDLPU (0x18/0x1C) per
   stream, dan **satu descriptor 16 byte**: alamat 8 byte, panjang 4 byte,
   flag 4 byte (bit 0 = interupt). Bukan 8 byte.
3. **Stream output mulai di 0x100**, bukan 0x80: index 0..3 = input
   (0x80..0xE0), index 4..7 = output (0x100..0x160). "0x80 + ISS*0x20" salah.
4. **Format 48 kHz S16 2 kanal = 0x0012**, bukan 0x0011: kanal di bit 3:0
   (2), bits-per-sample di bit 6:4 (`AC_FMT_BITS_16` = 1 → 0x10).
5. **CORBCTL.RUN = bit 1** (bit 0 = CMEIE); **RIRBCTL**: IRQ_EN bit 0,
   DMA_EN bit 1, OVERRUN_EN bit 2.
6. **INTCTL 0x20 dan INTSTS 0x24** (bukan 0x60/0x62); bit 31 global,
   bit 30 controller, bit 0..7 per stream.
7. **Tipe widget di bit 23:20**, bukan bit 3:0: 0=out, 1=in, 2=mixer,
   3=selector, 4=pin, 5=power, 6=knob, 7=beep, 0xF=vendor.
8. **Pin capabilities**: output = bit 4, input = bit 5, presence = bit 2,
   headphone drive = bit 3, EAPD = bit 16.
9. **STATESTS** adalah "bit per codec sudah di luar reset", bukan bit reset
   kontroler; reset kontroler dibaca dari GCTL.
10. **Device 0xA0C8** memakai kelas 04:03:00 untuk HDA legacy; pada mode
    DSP/SOF (0xA0C8) kontroler muncul sebagai 04:01:xx (audio DSP), jadi
    driver perlu mengenali dua kelas itu.
11. **Konvensi indeks CORB**: kontroler membaca entri setelah entri
    terakhir yang dibaca, jadi verb pertama ditulis di entri kedua (indeks
    1) dan CORBWP diisi indeks entri yang baru ditulis. RIRB: jawaban
    ditulis di `(RIRBWP+1)&0xFF` lalu RIRBWP diisi indeks itu — jawaban
    terakhir ada tepat di RIRBWP.

Belum dilakukan: boot QEMU dengan `-device intel-hda`, verifikasi pada
laptop nyata, jack detect, capture/mikrofon, dan streaming period bertahap.


### 4.2 USB tethering HP Android (#7), urutan kedua

- Pengganti cepat untuk Wi-Fi. Android memakai RNDIS (kelas E0/01/03 atau
  EF/04/01); Android baru juga bisa NCM (02/0D).
- Program baru `usbnet` dengan pola `usbserial` (driver kelas USB yang
  dimulai devmgr, R2 di `docs/USB-SERIAL.md` §6.1) dan sisi net ring seperti
  `virtio-net` / `rtl8169` (`xaqu-netserve`, `docs/NET-RING.md`).
- `devmgr/src/usb.rs`: `USB_DRIVERS` sekarang hanya cocok vendor/produk;
  perlu cocok kelas interface.
- **Pertanyaan terbuka yang harus dicek dulu**: apakah net ring
  (`NET_RING_CREATE`) bisa dibuat untuk anak USB, karena sekarang dibuat
  dari node perangkat kernel. Kalau tidak bisa, ini butuh rancangan (G3/G4
  di `docs/DRIVER-MODEL.md`), bukan sekadar driver.
- RNDIS: pesan kontrol lewat SEND_ENCAPSULATED_COMMAND /
  GET_ENCAPSULATED_RESPONSE (INITIALIZE, QUERY MAC, SET packet filter), data
  bulk dengan header 44 byte per frame.
- QEMU: `-device usb-net` (RNDIS + CDC-ECM).

#### Hasil Fase 3 (2026-11-18/19) — criptek USB-net + sisi net

- **Crate logika `xaqu-usbnet`** (`src/lib/drivers/net/usbnet/`): `cdc.rs`
  (permintaan kelas, notifikasi 8 byte, line coding, filter paket),
  `ecm.rs` (urutan setup ECM, MAC dari descriptor Ethernet, bingkai),
  `ncm.rs` (header NTB16/NTB32, NDP16/NDP32, cari datagram, setup NTB),
  `rndis.rs` (header pesan, INIT/QUERY/SET, OID, status, state machine).
  **48 tes host hijau**, clippy bersih.
- **Integrasi `devmgr`**: tabel `USB_DRIVERS` (`devmgr/src/usb.rs`) tidak lagi
  vendor/produk saja: barisnya bisa mencocokkan vendor, produk, kelas
  interface, dan subclass. Ditambah dua baris untuk CDC-ECM (kelas `0x02`,
  subclass `0x06`) dan CDC-NCM (`0x02`/`0x0D`), keduanya ke program `usbnet`,
  dengan kelas nomor sendiri (`Class::Net`).

**Temuan yang mengubah rancangan (bukan ingatan, dibaca dari sumber):**

1. **Sisi net tidak bisa pakai `NET_RING_CREATE` milik anak USB.** Panggilan
   itu terikat pada handle device kernel (`Device::net_ring()` →
   `self.ring(nr::NET_RING_CREATE)`), dan anak USB tidak punya node device
   di kernel: kernel tidak menerbitkan perangkat untuk USB. Jadi
   `usbnet` tidak bisa membuka net ring seperti `rtl8169`.
2. **AF_PACKET + veth tidak bisa menjembatani unicast.** Riset kode kernel:
   `AF_PACKET` hanya melihat frame **masuk** dan hanya yang alamatnya cocok
   dengan interface itu (`src/lib/network/net/src/input.rs:59-67`); frame
   egress dari stack ke gateway dibuang sebelum bisa dibaca socket. `IFF_PROMISC`
   ada di ioctl tapi tidak consulted di mana pun. Jadi lewat
   `AF_PACKET` + pasangan veth, frames dari stack ke internet tidak akan
   sampai ke driver.
3. **Yang tersedia tanpa perubahan kernel**: netlink `RTM_NEWLINK` +
   `IFLA_LINKINFO`/"veth" (kernel membuat pasangan veth; templat byte-nya ada
   di `src/kernel/src/fs/netns_check.rs:905-924`), socket `AF_NETLINK`/`AF_PACKET`
   dari program native lewat `xaqu_rt::linux::call` (contoh nyata:
   `drivers/console/vport` memakai `AF_UNIX`), dan netring `xaqu-netserve`
   untuk sisi driver.
4. **Konsekuensi**: sisi net `usbnet` butuh **perubahan kernel kecil** --
  (net ring tanpa device, atau device virtual untuk antarmuka USB jaringan).
   Pertanyaan sudah diajukan ke pemilik proyek; jawabannya menentukan
   bentuk program.

**Sisi USB sudah jelas dan tidak butuh kernel:** `usbhid` mengekspor antarmuka,
`devmgr` memulainya seperti `usbserial` (BIND + channel antarmuka + VMO
bersama), lalu driver memakai `xaqu-usbif`: control transfer untuk
permintaan kelas CDC, bulk OUT untuk frame, bulk IN untuk frame masuk.

### 4.3 Keyboard bawaan PS/2 (#5): butuh keputusan

- Logikanya kecil (i8042 + scancode set 1/2 ke `inputctl`), dan q35 punya
  i8042 jadi bisa diuji.
- Masalahnya: driver ring-3 butuh akses port 0x60/0x64 dan IRQ1 (GSI 1 lewat
  IOAPIC). Xaqu sengaja **belum punya** pemberian port I/O (G9), dan
  `docs/DRIVER-MODEL.md` §4.4 meminta mekanisme itu dirancang tersendiri
  (bitmap I/O di TSS x86-64, menolak port kernel, 0xCF8-0xCFF, port DMA ISA).
- Pilihan: (a) rancang dan bangun G9 sekali (perubahan kernel, setelah itu
  driver port lain tidak perlu ubah kernel lagi), atau (b) pakai keyboard
  USB. Ini keputusan pemilik proyek.
- Perangkat ini ditemukan lewat ACPI (MSFT0001), sementara Xaqu belum punya
  AML. Untuk laptop ini cukup menganggap i8042 ada di port standar.

### 4.4 GPU Tiger Lake (#8)

- `src/lib/drivers/gpu/intel-xelp/src/device_info.rs`: `lookup` hanya
  menerima `ADLN_IDS`. Tiger Lake juga Gen12.0 (Xe-LP), jadi menambah 0x9A78
  (dan saudara TGL dari `i915/include/pciids.h`) mungkin cukup untuk render.
- Perlu dicek: perbedaan TGL dengan ADL-N (jumlah EU, fuse media, GuC tidak
  wajib di TGL), dan aturan kernel `intel_gpu()` / `xaqu.intelgpu` (fungsi
  dipinjamkan hanya dengan opsi boot itu, dan butuh VT-d).
- Driver `intel` adalah render saja; layar tetap lewat framebuffer firmware.
  Tanpa ini desktop tetap jalan, hanya lebih lambat.

### 4.5 Kamera UVC (#9)

- Kelas USB video (0E). Butuh driver kelas USB baru dan jalur ke program
  (V4L2 `/dev/video0`); kernel belum punya core video, jadi termasuk G3
  (perlu rancangan kelas, atau forwarding K5). Prioritas rendah.
- **Temuan 2026-11-19** (crate `xaqu-uvc` sudah ada, 26 tes hijau):
  1. **Kamera punya dua antarmuka** (VideoControl subclass 1 dan
     VideoStreaming subclass 2), sedangkan devmgr memulai satu driver per
     antarmuka dan `Bind` hanya membawa satu channel. Jadi driver kamera
     butuh dua channel: devmgr harus mengelompokkan antarmuka satu
     perangkat (kunci `Attach` yang sama) dan mengirim satu `Bind` per
     antarmuka dengan nomor yang sama.
  2. **Aturan `xaqu_usbif::allowed` menolak `SET_INTERFACE`**: permintaan
     standar ke device hanya boleh `GET_STATUS`/`GET_DESCRIPTOR`. Tanpa
     `SET_INTERFACE` (alternate setting 1 milik antarmuka streaming) kamera
     tidak dapat streaming. Perbaikannya di protocol `xaqu-usbif` (ruang
     pengguna), bukan kernel.
  3. Streaming bisa ditulis ke berkas (`/tmp/uvc<N>.<ext>`) sampai ada core
     video di kernel.

### 4.6 Touchpad I2C HID (#10)

Rantai yang dibutuhkan, semuanya belum ada:
1. Driver kontroler I2C Intel Serial IO (DesignWare, PCI 8086:a0e8), sesuai
   aturan (driver PCI).
2. Alamat I2C touchpad, register HID descriptor, dan pin GPIO interrupt.
   Normalnya dari ACPI `_CRS`/`_DSM` (butuh AML, B3). Jalan pintas khusus
   laptop ini: baca DSDT dari registry Windows
   (`HKLM\HARDWARE\ACPI\DSDT`), decompile dengan `iasl`, tulis nilainya
   sebagai konstanta (ditandai upgrade path).
3. Interrupt GPIO Tiger Lake (pinctrl INT34C5): termasuk G5 (interrupt di
   belakang kontroler sekunder), atau polling sebagai jalan pintas.
4. HID-over-I2C, lalu laporan HID ke `inputctl` (parser HID sudah ada di
   `usb-host/src/hid.rs` / `report.rs`, bisa dipakai ulang).

**Hasil Fase 0b (2026-11-18):** tabel ACPI laptop diambil dari registry
Windows dan di-decompile (`docs/acpi/`, DSDT 351.297 byte + 13 SSDT +
FADT/FACS/RSDT; `iasl` 20251212). Data touchpad yang terbaca:

| yang dicari | nilainya | dari mana |
|---|---|---|
| perangkat | `\_SB.PC00.I2C0.TPD0` | DSDT (Scope `_SB.PC00.I2C0`) |
| kontroler | Intel Serial IO I2C #0 (PCI 8086:a0e8) | `I2C0`, `I2CX = 0` |
| bus | I2C0 (bukan I2C1/2/3) | `\_SB.PC00.I2C0` |
| model | Goodix **GXTP7863** | `_HID` 方法: `TPDT = 0x08` → `"GXTP7863"` |
| alamat slave | **0x5D** (7-bit) | `BADR = 0x5D` |
| kecepatan | **400 000 Hz (400 kHz)** | `SPED = 0x00061A80` |
| status | `0x0F` (ada, aktif) | `_STA` |
| descriptor HID | lewat `_DSM` UUID `3cdff6f7-4267-4555-ad05-b30a3d8938de` | `_DSM` → `HIDD()`; `HID2 = 1` berarti alamatnya dari `_DSM`, bukan 0x0000 |
| interrupt | dua bentuk: `SBFI` (IRQ tetap) atau `SBFG` (GpioInt **Level, ActiveLow** di `\_SB.GPI0`) | `_CRS` memilihnya lewat `TPDM` |

Yang **tidak** bisa dibaca tanpa evaluasi AML: `TPDM` (0 = IRQ tetap,
1 = GPIO), `TPDT` (0x08 di laptop ini), IRQ `INT2 = INUM(0x090B0008)`, dan
nomor pin GPIO `INT1 = GNUM(0x090B0008) = GINF(grup 0x0B, 0x06) + 8` —
`GPCH[]` (dasar tiap grup GPIO) diisi BIOS saat boot. Jadi **pin GPIO
touchpad tidak bisa dijadikan konstanta** dari DSDT statis; jalannya:
polling tiap 8 ms (upgrade path ke interrupt GPIO) atau eval AML (B3).
Catatan: `TPDT`/`TPDM` adalah field di *Platform Configuration Table*
DSDT (offset 0x3FA), nilainya diisi firmware saat boot.

Berkas ACPI (bukan rahasia, tapi tetap lokal): `docs/acpi/*.aml` dan
`docs/acpi/dsl/*.dsl`.

**Hasil Fase 6 (2026-11-19) — paruh protokol:**

- **Crate logika `xaqu-i2c-hid`** (`src/lib/drivers/input/i2c-hid/`):
  `descriptor.rs` (descriptor HID 30 byte: register report descriptor, input,
  output, command, data -- dibaca dari perangkat, bukan ditebak) dan
  `command.rs` (opcode: reset 0x01, get/set report 0x02/0x03, idle 0x04/0x05,
  protocol 0x06/0x07, power 0x08; report ID ditulis di data, bukan di header).
  **15 tes host hijau**. Sumber: `drivers/hid/i2c-hid.c` Linux.
- **Temuan kernel**: Xaqu **belum punya I2C sama sekali** (cari di
  `src/kernel/src`: tidak ada file maupun sebutan `i2c`). rantai touchpad
  karena itu butuh: (1) enumerasi controller I2C ACPI (`PNP0C02` +
  `I2cSerialBusV2`, BAR0/BAR1 masing-masing 4 KiB, dari `I2CH()` di DSDT),
  (2) driver host Intel Serial IO I2C (PCI `8086:a0e8`, DesignWare), (3)
  HID-over-I2C di user space memakai crate di atas, (4) laporan HID ke
  `inputctl` (parser HID sudah ada di `usb-host/src/hid.rs`), (5) interrupt:
  polling tiap 8 ms (pin GPIO hanya diketahui saat boot, lihat tabel Fase 0b).
- **Perbaikan rancangan (2026-11-19)**: **kernel tidak perlu disentuh**.
  Controller LPSS I2C (`8086:a0e8`) adalah fungsi PCI yang sudah diterbitkan
  kernel, dan driver ring-3 boleh memetakan BAR-nya sendiri
  (`device.aperture()` + `device.io_mapping`, pola HDA/NVMe). Jadi cukup:
  baris `PciMatch::Devices(&[0xa0e8])` di devmgr → program `i2c-hid`
  (`Kind::Input`), crate `xaqu-lpss-i2c` untuk mesin keadaan transaksinya,
  lalu HID-over-I2C di user space. Rancangan lengkap: `docs/I2C-TOUCHPAD.md`.

### 4.7 Wi-Fi AC 9560 (#11)

- Setara iwlwifi (mvm) + mac80211 + WPA2 di Linux: ratusan ribu baris, butuh
  firmware Intel `iwlwifi-9000-pu-b0-jf-b0-*.ucode` (boleh didistribusikan,
  tapi harus dicatat di audit lisensi). Kerja berbulan-bulan.
- Alternatif: USB tethering (4.2) atau adaptor USB Ethernet (RTL8153 /
  AX88179, driver kelas USB lebih kecil).

### 4.8 Bluetooth (#12), mikrofon digital (#13), sidik jari (#14)

- Bluetooth: btusb + firmware Intel + stack HCI/L2CAP. Besar, prioritas
  rendah.
- Mikrofon digital lewat DSP (SOF): butuh firmware DSP dan topologi. Tidak
  disarankan.
- Sidik jari Goodix 27c6:5e0a: protokol tertutup. Tidak disarankan.

## 5. Urutan yang disarankan

1. Audio HDA (4.1)
2. USB tethering (4.2), setelah pertanyaan net ring untuk anak USB terjawab
3. Keputusan PS/2 (4.3)
4. GPU Tiger Lake (4.4)
5. Touchpad (4.6)
6. Kamera, lalu Wi-Fi, lalu Bluetooth

Setiap driver selesai dengan: tes host lolos, uji QEMU (bila bisa),
`cargo xtask check` hijau, ISO baru. Karena laptop tidak punya port serial,
log driver sebaiknya bisa disalin ke flashdisk untuk uji di perangkat asli.

## 6. Prompt untuk sesi berikutnya

> Lanjutkan driver Xaqu untuk laptop saya mengikuti
> `C:\Users\jeani\Downloads\OS\docs\PETA-DRIVER-LAPTOP.md`. Repo di WSL
> archlinux `/root/ferrix`, cabang `rename/xaqu`. Kerjakan bagian 4.1 (audio
> Intel HDA) dulu sesuai pola di bagian 3: crate logika dengan tes host,
> program driver `hda`, baris devmgr, `native.rs`, lalu `cargo xtask check`
> (pakai `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`, VM WSL hanya 6 GB).
> Jangan ubah kernel tanpa tanya. Jangan push.

### 4.9 Fase 7 (2026-11-19): Wi-Fi, keyboard, sidik jari

Prioritas pemilik proyek: Wi-Fi, keyboard (F1–F12), trackpad, sidik jari.
Tiga crate logika baru dibuat dan **sudah hijau**; program native belum.

| crate | isi | tes | clippy/fmt |
|---|---|---|---|
| `xaqu-i8042` (`src/lib/drivers/input/i8042/`) | protokol PS/2 set 2: scancode → kode tombol Linux, state machine byte (make/break, `0xF0`, `0xE0`, lock `0xE1`), modifier & kunci | 21 | bersih |
| `xaqu-iwl` (`src/lib/drivers/net/iwl/`) | lapisan firmware Intel: header berkas `.ucode` (magic `0x0a4c5749`), blok TLV, tabel device, penamaan `iwlwifi-9000-pu-b0-jf-b0-<api>.ucode`, header perintah | 21 | bersih |
| `xaqu-goodix-fp` (`src/lib/drivers/usb/goodix-fp/`) | sensor sidik jari Goodix `27c6:5e0a`: descriptor HID, framing paket,penyusunan ulang gambar; **tabel opcode vendor sengaja kosong** | 20 | bersih |

Dokumen baru: `docs/I8042-KEYBOARD.md`, `docs/IWLIFI.md`, `docs/FINGERPRINT.md`.

**Yang sudah berfungsi hari ini tanpa perubahan kernel:**
- Trackpad (Fase 6): HID-over-I2C lewat LPSS I2C, lengkap.
- Keyboard USB: F1–F12 sudah terpetakan di halaman keyboard HID.

**Yang belum, dan alasannya:**
1. **Keyboard bawaan (PS/2)** butuh akses port I/O `0x60` untuk ruang
   pengguna → butuh **G9 (bitmap port I/O di TSS)**, perubahan kernel.
   Keputusan pemilik proyek (`PETA` §4.3).lihat `docs/I8042-KEYBOARD.md`.
2. **Wi-Fi**: crate baru hanya lapisan firmware. Yang belum: program native,
   transport PCIe (TX/RX), MAC 802.11, WPA2, dan pemuat firmware di
   initramfs. Butuh berkas `iwlwifi-9000-pu-b0-jf-b0-*.ucode` di image.
3. **Sidik jari**: opcode vendor (presence/capture/enroll/match/store) tidak
   dipublikasikan, jadi **tidak dikarang** — tabelnya kosong dan setiap
   permintaan ditolak. Perlu tangkapan USB dari sensor nyata.
4. **Program native** untuk ketiganya belum ditulis; `devmgr` belum
   diberi baris untuk `iwl`/`goodix-fp` karena image-nya belum ada.

### 4.10 Fase 7b: program native `goodix-fp` + crate `xaqu-usb-link`

- **Program native `goodix-fp`** (`src/user/system/native/drivers/usb/goodix-fp/`,
  package `xaqu-goodix-fp-driver`): class driver satu antarmuka USB. Membaca
  HID report descriptor lewat control request, memparse dengan
  `xaqu-goodix-fp`, membuka stream IN interrupt, lalu membaca tiap laporan
  sebagai paket dan menyatakannya. **Tidak mengirim perintah vendor
  apa pun** (opcode tidak dipublikasikan). Build `x86_64-unknown-none`
  bersih, clippy bersih, fmt bersih.
- **Wiring devmgr**: baris baru di `USB_DRIVERS`
  (`src/user/system/native/devmgr/src/usb.rs`): vendor `0x27c6`, produk
  `0x5e0a` → image `goodix-fp`, kelas baru `Class::Fp`. Tabel `USB_DRIVERS`
  4 → 5 baris.
- **Wiring xtask**: `PROGRAMS` memuat `xaqu-goodix-fp-driver` → bin `goodix-fp`.
- **Refactor**: `xaqu-usb-link` (`src/user/system/native/usb-link/`) —
  kode `link.rs` yang sebelumnya milik program `usbserial` (klien
  `xaqu-usbif` di atas channel antarmuka: control request, stream OUT,
  chunk IN). Dipisah menjadi crate agar semua class driver USB bicara
  dengan host dengan cara yang sama, dan agar program `uvc` yang
  menyusul memakainya. Program `usbserial` tetap build bersih.

### 4.11 Fase 8: G9 (bitmap I/O port) + program native `i8042`

- **G9 disetujui pemilik proyek dan sudah dikerjakan.** Crate
  `xaqu-ioport` (`src/lib/proto/ioport/`): bitmap 8192 port, aturan port
  yang tidak pernah diberikan (DMA1/DMA2/DMA page, PIC1/PIC2, PIT, PCI
  config `0xCF8`–`0xCFF`); grant yang menyentuh port terlarang ditolak
  **sepenuhnya** (EPERM). Nomor syscall Linux `ioperm`: x86-64 `296`,
  ARMv7-A `356` (`src/lib/proto/linux-abi/src/nr.rs`).
- **Kernel**: bitmap I/O port di TSS x86-64 (`arch/x86_64/gdt.rs`),
  syscall `syscall/ioport.rs`, grant **per-proses** (`Process::io_ports`,
  `SpinLock`), dan bitmap dipasang ke TSS pada saat grant diberikan
  **dan** pada tiap context switch (`sched::switch_user_state`). Build
  kernel lewat `cargo xtask build --arch x86_64` **bersih** (pertama
  kalinya patch G9 ini dikompilasi — sebelumnya belum pernah).
- **Program native `i8042`** (`src/user/system/native/drivers/input/i8042/`,
  package `xaqu-i8042-driver`, bin `i8042`): minta port `0x60`/`0x64`
  dengan `ioperm`, deteksi kontroler (port status `0xFF` = tidak ada),
  matikan translasi set 2 → set 1 dan nyalakan jam keyboard, kirim
  `0xF4` (*enable scanning*), poll port status tiap 1 ms, decode dengan
  `xaqu-i8042`, lalu kirim `EV_KEY` + `EV_SYN` ke inti input. `HELLO`
  mendeklarasikan 126 kode (`xaqu_i8042::codes::ALL`).
- **Node perangkat**: kontroler PS/2 tidak ada di bus mana pun, jadi
  kernel mempublikasikan node `TREE_X86_I8042` (`discovery/i8042.rs`)
  hanya bila port statusnya tidak mengambang, dan `devmgr` menjalankan
  program `i8042` di node itu (baris `TREE_DRIVERS`).
- **Verifikasi**: 21 tes `xaqu-i8042`, 14 tes `xaqu-ioport`, 158 tes
  `xaqu-linux-abi`, 22 tes `xaqu-hda` — hijau; build `x86_64-unknown-none`
  program `i8042` bersih, clippy bersih, fmt bersih; image `xaqu.img`
  terbentuk (initramfs 44814 → 45804 KiB, program `i8042` masuk).
- **Sisa**: IRQ 1 belum diteruskan ke program, jadi polling yang dipakai;
  belum diuji di QEMU (`q35` punya kontroler i8042 bawaan).

### 4.12 Fase 8b: program native `uvc` (kamera)

- **Program native `uvc`** (`src/user/system/native/drivers/video/uvc/`,
  package `xaqu-uvc-driver`, bin `uvc`): baca configuration descriptor lewat
  control request ke device (`GET_DESCRIPTOR` `wValue=0x0200`), parse dengan
  `xaqu_uvc::descriptor::walk_configuration`, **says** interface kontrol,
  interface streaming, alternate setting yang streaming, dan tiap
  format/frame (geometri, laju, layout piksel); lalu `SET_INTERFACE` ke
  interface sendiri, buka endpoint IN (isokron, atau bulk), baca tiap
  transfer, parse header payload, dan **hitung frame** (jumlah & byte).
- **Frame tidak dirakit dan tidak dikirim ke mana pun**: Xaqu belum punya
  pipeline kamera (inti yang menerima frame lalu menampilkannya). Yang
  dapat dilakukan tanpa itu:-what yang kamera tawarkan + hitung frame.
- **Perubahan kebijakan `xaqu_usbif::allowed`**: `SET_INTERFACE` kini
  boleh ke interface yang diklaim driver itu sendiri (`wIndex` = nomor
  interface-nya). Alternate setting 0 = diam, alternate setting 1 yang
  membawa frame; tanpa ini kamera tidak bisa streaming. Interface lain
  tetap ditolak. 4 tes `xaqu-usbif` hijau.
- **Wiring devmgr**: baris baru di `USB_DRIVERS` — vendor `0x2b7e`, produk
  `0xb514`, class `0x0E`, subclass `0x02` (interface streaming), image
  `uvc`, kelas baru `Class::Video`; tabel 5 → 6 baris. `xtask PROGRAMS`
  memuat `xaqu-uvc-driver` → bin `uvc`.
- **Dokumen**: `docs/UVC.md`.
- **Verifikasi**: 26 tes `xaqu-uvc` + 4 tes `xaqu-usbif` hijau; build
  `x86_64-unknown-none` program `uvc` bersih, clippy bersih, fmt bersih.
