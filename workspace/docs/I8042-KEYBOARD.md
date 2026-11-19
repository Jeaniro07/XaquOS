# Keyboard bawaan PS/2 (i8042) — Fase 7

Laptop ini: realme RMNBXXXX, Intel Core i3-1115G4 (Tiger Lake-UP3).
Keyboard bawaan: **ACPI `MSFT0001`** — kontroler AT/i8042, port I/O
`0x60`/`0x64`, IRQ 1, **scancode set 2**.

Dokumen ini adalah rujukan driver keyboard di Xaqu.

Status (2026-11-19): logika protokolnya sudah ada dan teruji di host —
crate `xaqu-i8042` (`src/lib/drivers/input/i8042/`) dengan 21 tes, clippy
dan fmt bersih. **Program native-nya belum bisa jalan**, karena Xaqu belum
memberi ruang pengguna akses ke port I/O. Lihat §4.

## 1. Yang sudah ada: crate `xaqu-i8042`

Protokolnya saja, tanpa menyentuh port:

- `scancode`: tabel scancode set 2 → kode tombol Linux, diambil dari
  `atkbd_set2_keycode` di `drivers/input/keyboard/atkbd.c`.
- `decoder`: state machine byte → event (make/break, awalan `0xF0` dan
  `0xE0`, urutan lock `0xE1`).
- `modifiers`: status Shift/Ctrl/Alt/Meta/AltGr dan kunci Caps/Num/Scroll.
- `keyboard`: `Keyboard::feed(byte)` yang mengembalikan event tombol.

### F1 sampai F12

Nomor scancode set 2 untuk baris fungsi **bukan** urutan labelnya:

| tombol | scancode | tombol | scancode |
|---|---|---|---|
| F1 | `0x05` | F7 | `0x02` (atau `0x83`) |
| F2 | `0x06` | F8 | `0x0A` |
| F3 | `0x04` | F9 | `0x01` |
| F4 | `0x0C` | F10 | `0x09` |
| F5 | `0x03` | F11 | `0x78` |
| F6 | `0x0B` | F12 | `0x07` |

Bentuk perluasan (`0xE0` di depan) juga dikenali. Kode tombol yang
dilaporkan adalah kode Linux (`KEY_F1` = 59 … `KEY_F12` = 70), sama
seperti yang dilaporkan driver HID, jadi tidak ada terjemahan kedua.

21 tes host, antara lain: setiap tombol fungsi ditekan dan dilepas
terbaca, bentuk perluasan, panah, tombol keypad, modifier yang
mengikuti, kunci Caps yang menyala dan mati, laporan kunci (`0xE1`),
PrintScreen, Pause, dan scancode yang tidak dikenal yang tidak
menghasilkan apa pun.

## 2. Jalur tanpa kernel (sudah jalan)

Keyboard USB lewat `usbhid` sudah dipetakan: halaman keyboard HID sudah
punya kode F1–F12 (`KEYBOARD_CODES` di
`src/lib/drivers/usb/usb-host/src/hid.rs`, usage `0x3A`–`0x45` → 59–70).
Jadi **F1–F12 keyboard USB sudah berfungsi hari ini**; yang belum adalah
keyboard *bawaan* laptop.

## 3. Program native `i8042` (sudah ada)

`src/user/system/native/drivers/input/i8042/` — bin `i8042`, paket
`xaqu-i8042-driver`. Alurnya:

1. Minta port `0x60` dan `0x64` dengan syscall Linux `ioperm` (G9, §4).
2. Deteksi kontroler: port status yang mengambang (semua bit 1, `0xFF`)
   berarti tidak ada kontroler — program berhenti bersih.
3. Atur kontroler: baca *command byte*, **matikan translasi** (bit 6) dan
   nyalakan jam keyboard (bit 4 = 0), lalu kirim `0xF4` (*enable scanning*).
   Matinya translasi penting: `xaqu-i8042` membaca scancode set 2,
   sedangkan kontroler biasanya menerjemahkannya ke set 1.
4. Poll port status tiap 1 ms, baca byte yang ada, dekode dengan
   `xaqu-i8042`, lalu kirim ke inti input sebagai `EV_KEY` + `EV_SYN`
   (peta kode Linux, `BUS_I8042`).
5. Kirim `HELLO` ke inti input lebih dulu, dengan deklarasi tepat semua
   kode yang bisa dilaporkan (`xaqu_i8042::codes::ALL`, 126 kode).

Program ini **polling**, bukan interrupt: IRQ 1 belum diteruskan ke
program mana pun. Poll adalah jalur sementara yang sah; jalur interrupt
membaca port yang sama saat platform bilang ada yang masuk.

## 4. G9 (bitmap I/O port) di kernel — sudah dikerjakan

Pemilik proyek menyetujui G9, dan sekarang sudah ada:

- `src/lib/proto/ioport/` — crate `xaqu-ioport`: aturan port yang **tidak
  pernah** diberikan (DMA1/DMA2/DMA page, PIC1/PIC2, PIT, PCI config
  `0xCF8`–`0xCFF`) dan bitmap 8192 port; grant yang menyentuh port
  terlarang ditolak **sepenuhnya** (EPERM).
- `src/lib/proto/linux-abi/` — nomor syscall `ioperm`: x86-64 `296`,
  ARMv7-A `356` (aarch64 tidak punya).
- Kernel: bitmap I/O port di TSS x86-64 (`arch/x86_64/gdt.rs`), syscall
  `syscall/ioport.rs`, grant **per-proses** (`Process::io_ports`), dan
  bitmap dipasang ulang pada tiap context switch (`sched::switch_user_state`).
- Node perangkat: kontroler PS/2 tidak ada di bus mana pun, jadi kernel
  mempublikasikan node `TREE_X86_I8042` bila port statusnya tidak
  mengambang (`discovery/i8042.rs`), dan `devmgr` menjalankan program
  `i8042` di node itu (`TREE_DRIVERS`).

Port yang diberikan program keyboard hanya `0x60` dan `0x64`.

## 5. Yang belum ada

1. Interrupt (IRQ 1) sebagai gantinya polling — §3.
2. Uji di QEMU: mesin `q35` punya kontroler i8042, jadi program native bisa
   diuji di QEMU tanpa perangkat keras, dengan `-device i8042` (atau
   `-M q35` bawaan) dan keyboard ps2.

## 6. Verifikasi

--Host: 21 tes crate `xaqu-i8042`.
- QEMU: mesin `q35` punya kontroler i8042 (ps2), jadi program native bisa
  diuji di QEMU tanpa perangkat keras, dengan `-device i8042` (atau
  `-M q35` bawaan) dan `-usbdevice`/keyboard ps2.

## 7. Sumber

- `drivers/input/keyboard/atkbd.c` — `atkbd_set2_keycode`.
- `include/uapi/linux/input-event-codes.h` — nomor `KEY_*`.
