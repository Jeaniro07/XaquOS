# Touchpad I2C-HID (Fase 6): rancangan

Laptop ini: realme RMNBXXXX, Intel Core i3-1115G4 (Tiger Lake-UP3).
Touchpad: **Goodix GXTP7863**, terpasang pada bus I2C0 (Intel Serial IO,
PCI `8086:a0e8`), alamat slave **0x5D**, 400 kHz.

Dokumen ini adalah rancangan, bukan implementasi. **Kernel tidak perlu
disentuh**: controller LPSS I2C adalah fungsi PCI yang sudah diterbitkan
kernel, dan driver ring-3 boleh memetakan BAR-nya sendiri (bagian 4).

## 1. Yang sudah ada

| bagian | keadaan |
|---|---|
| crate `xaqu-i2c-hid` | ada (`src/lib/drivers/input/i2c-hid/`), 15 tes host: descriptor HID 30 byte + opcode perintah |
| parser HID (laporan → event) | ada di `src/lib/drivers/usb/usb-host/src/hid.rs` dan `report.rs`; bisa dipakai ulang |
| kernel I2C | **tidak ada sama sekali** (cari di `src/kernel/src`: tidak ada berkas maupun sebutan `i2c`) |
| GPIO/AML | tidak ada (interrupt touchpad diketahui hanya saat boot) |

## 2. Bukti dari ACPI (DSDT laptop ini, `docs/acpi/dsl/dsdt-00000000.dsl`)

| yang dicari | nilainya | dari mana |
|---|---|---|
| perangkat | `\_SB.PC00.I2C0.TPD0` | DSDT (Scope `_SB.PC00.I2C0`) |
| model | Goodix **GXTP7863** | `_HID` metode: `TPDT = 0x08` → `"GXTP7863"` |
| alamat slave | **0x5D** (7-bit) | field `BADR` |
| kecepatan | **400 000 Hz** | `SPED = 0x00061A80` |
| bus | I2C0 | `\_SB.PC00.I2C0` |
| controller | Intel Serial IO I2C #0, PCI `8086:a0e8` | `I2C0`, `I2CX = 0` |
| descriptor HID | lewat `_DSM` UUID `3cdff6f7-4267-4555-ad05-b30a3d8938de` | `HID2 = 1` berarti alamat descriptor dari `_DSM`, bukan 0x0000 |
| interrupt | `SBFI` (IRQ tetap) atau `SBFG` (GpioInt **Level, ActiveLow** di `\_SB.GPI0`) | `_CRS` memilih lewat `TPDM` |

Yang tidak bisa dibaca tanpa evaluasi AML: `TPDM` (0 = IRQ tetap,
1 = GPIO), IRQ `INT2 = INUM(0x090B0008)`, dan nomor pin GPIO
`INT1 = GNUM(0x090B0008) = GINF(grup 0x0B, 0x06) + 8`. Jadi **pin GPIO tidak
bisa dijadikan konstanta**: jalannya polling tiap 8 ms (upgrade path ke
interrupt GPIO) atau eval AML (B3).

Berkas ACPI (bukan rahasia, tapi tetap lokal): `docs/acpi/*.aml`,
`docs/acpi/dsl/*.dsl`.

## 3. Protokol HID-over-I2C (dasar)

Sumber: `drivers/hid/i2c-hid.c` Linux.

- Descriptor HID 30 byte, little-endian:
  `wHIDDescLength, bcdVersion, wReportDescLength, wReportDescRegister,
  wInputRegister, wMaxInputLength, wOutputRegister, wMaxOutputLength,
  wCommandRegister, wDataRegister, wVendorID, wProductID, wVersionID,
  reserved(u32)`. Register-register ini **dibaca dari perangkat**, bukan
  ditebak (crate `xaqu-i2c-hid` mem-pars-nya).
- Perintah: 4 byte — register perintah (u16 LE), tipe laporan (u8),
  opcode (u8); lalu datanya. Opcode: reset `0x01`, get report `0x02`,
  set report `0x03`, get idle `0x04`, set idle `0x05`, get protocol `0x06`,
  set protocol `0x07`, set power `0x08`.
- Laporan input dibaca dari `wInputRegister`, paling banyak
  `wMaxInputLength` byte; laporan keluar dari `wOutputRegister`.

## 4. Yang harus ada (tanpa mengubah kernel)

Temuan penting: kernel Xaqu sudah menerbitkan **fungsi PCI** sebagai device
(devmgr mencocokkan vendor + device id), dan driver ring-3 boleh memetakan
BAR-nya sendiri (`device.aperture()` + `device.io_mapping`, pola driver HDA
dan NVMe). Jadi **tidak perlu subsystem I2C di kernel**:

1. **devmgr**ermanyakan baris `PciMatch::Devices(&[0xa0e8])` untuk vendor
   `0x8086` → program `i2c-hid`, dengan `Kind::Input` (driver menerbitkan
   event ke input core, seperti `virtio-input`).
2. **Driver** memetakan BAR0 controller LPSS I2C dan memegang registernya
   sendiri. Register LPSS I2C (layout Synopsys DesignWare, dari datasheet
   Intel "Serial I/O (LPSS)"): `IC_CON 0x000`, `IC_TAR 0x004`, `IC_DATA_CMD
   0x010`, `IC_DATA 0x014`, `IC_INTR_STAT 0x02C`, `IC_RAW_INTR_STAT 0x034`,
   `IC_RX_TL 0x038`, `IC_STATUS 0x070`. Timeout diatur lewat `IC_CON`/`IC_TAR`.
3. **Crate logika `xaqu-lpss-i2c`**: mesin keadaan transaksi di atas register
   itu (aktif/nonaktif, alamat target, tulis, baca, Southampton, status
   TFNF/RFNE, batas waktu), diuji di host.

Bila nanti perlu I2C juga untuk perangkat lain, barulah subsystem I2C
kernel perlu dirancang (itu pekerjaan tersendiri).

## 5. Bentuk program driver (user space)

Paket `xaqu-i2c-hid-driver`, program `i2c-hid`:

1. START dari devmgr: device kontroler I2C (PCI `8086:a0e8`) + register
   block-nya, seperti driver lain.
2. Inisialisasi LPSS: clock, `IC_CON` (master mode, speed 400 kHz →
   divisor sesuai clock LPSS), clear interrupts.
3. Baca descriptor HID dari perangkat (register `wHIDDescRegister`; kalau
   perangkat tidak menjawab, pakai descriptor dari ACPI yang sudah
   didecompile di `docs/acpi/` — ditandai upgrade path).
4. Parse descriptor laporan HID (parser HID yang sudah ada), lalu baca
   laporan input tiap 8 ms (polling) atau lewat interrupt GPIO bila
   pin-nya diketahui.
5. Ubah laporan menjadi event input lewat `inputctl` (pola `virtio-input`).

## 6. Yang belum dan belum bisa dipastikan

- Interrupt GPIO: butuh AML (B3) atau penentuan pin saat boot.
- Kecepatan aktual bus setelah inisialisasi (LPSS clock dari ACPI/SSC).
- Quirk Goodix: Linux memakai quirk untuk beberapa touchpad Goodix
  (`hid-multitouch`/elan); belum diselidiki untuk GXTP7863.
