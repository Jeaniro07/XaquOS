# HDA — driver Intel HD Audio

Dokumen ini adalah rujukan driver audio Intel HD Audio (HDA) di Xaqu: apa
yang dikendalikan, di mana logikanya, dan bagaimana program native-nya
berbicara dengan inti audio.

Status (2026-11-18): logika driver (`xaqu-hda`) ada dan teruji di host;
program native (`hda`) ada dan lulus build untuk target Xaqu; integrasi
`devmgr` sudah terpasang. **Belum** diverifikasi di perangkat nyata (belum
ada boot QEMU dengan `-device intel-hda` pun yang diuji), dan belum ada
pengujian suara nyata.

## 1. Untuk laptop realme RMNBXXXX (Intel Core i3-1115G4, Tiger Lake-UP3)

| apa | nilai | sumber |
| --- | --- | --- |
| Kontroler | Intel HD Audio, Sunrise Point-LP, PCI 8086:a0c8 | enumerasi Windows |
| Kelas PCI | 04:03:00 (multimedia, audio, HDA) | QEMU `intel-hda`, `PCI_CLASS_MULTIMEDIA_HD_AUDIO` |
| Register block | BAR 0, 16 KiB (jendela MMIO 8 KiB + alias) | QEMU `hw/audio/intel-hda.c` |
| Codec |Codec HDA di recites address 0 | HDA 1.0a |
| Alamat codec | `0` (yang pertama) | QEMU `hda_codec_bus_init` |
| Stream | 4 input (0x80..0xE0), 4 output (0x100..0x160) | QEMU `intel-hda-defs.h` |

Kontroler HDA pada Tiger Lake-LP adalah fungsi PCI terpisah dari xHCI; jadi
Fase 1 (keyboard/mouse) dan Fase 2 (audio) tidak saling mengganggu.

## 2. Register dan bit

Semua offset dan bit diambil dari definisi QEMU yang disalin dari Linux
(`hw/audio/intel-hda-defs.h`, "from linux/sound/pci/hda/hda_intel.c") dan
dari `hw/audio/intel-hda.c` — bukan dari ingatan. Ringkasnya:

| register | offset | catatan |
| --- | --- | --- |
| GCAP | 0x00 | bit 11:8 jumlah stream input, bit 15:12 jumlah stream output |
| GCTL | 0x08 | bit 0 CRST (reset), bit 1 FCNTRL, bit 8 UNSOL |
| STATESTS | 0x0E | satu bit per codec: codec siap (di luar reset) |
| INTCTL / INTSTS | 0x20 / 0x24 | bit 31 global, bit 30 controller, bit 0..7 stream |
| CORBLBASE / CORBUBASE | 0x40 / 0x44 | alamat 64-bit ring CORB di memori DMA |
| CORBWP / CORBRP | 0x48 / 0x4A | pointer tulis/baca, 8 bit |
| CORBCTL | 0x4C | bit 1 RUN, bit 0 CMEIE |
| CORBSIZE | 0x4E | bit 7 nol → ring 256 entri |
| RIRBLBASE / RIRBUBASE | 0x50 / 0x54 | alamat 64-bit ring RIRB |
| RIRBWP | 0x58 | pointer tulis, bit 15 = reset pointer |
| RINTCNT | 0x5A | berapa jawaban sebelum interupt |
| RIRBCTL | 0x5C | bit 0 IRQ_EN, bit 1 DMA_EN, bit 2 OVERRUN_EN |
| RIRBSIZE | 0x5E | bit 7 nol → ring 256 entri |
| Stream block | 0x80 + n·0x20 | CTL 0x00, LPIB 0x04, CBL 0x08, LVI 0x0C, FORMAT 0x12, BDLPL 0x18, BDLPU 0x1C |

Penting: CORB dan RIRB **tidak** berada di register block. Keduanya berada
di memori yang driver buat dan pin-kan untuk perangkat; register block hanya
memuat alamat dasarnya dan pointer-nya.

Konvensi indeks CORB: kontroler membaca entri **setelah** entri terakhir yang
ia baca, jadi verb pertama ditulis pada entri kedua (indeks 1) dan CORBWP
diisi indeks entri yang baru ditulis. RIRB: jawaban ditulis pada indeks
`(RIRBWP + 1) & 0xFF`, lalu RIRBWP diisi indeks tersebut — jadi jawaban
terakhir ada tepat di RIRBWP.

### Verb

Format 12-bit (bawaan): CAD bit 31:28, NID bit 27:20, parameter bit 19:8,
nilai bit 7:0. Format 4-bit: CAD bit 31:28, NID bit 27:20, parameter bit
19:16, nilai bit 15:0.

| verb | nilai | catatan |
| --- | --- | --- |
| GET_PARAMETER | 0xF00 | id parameter di byte rendah |
| GET_CONFIG_DEFAULT | 0xF1C | id parameter 0x1C |
| SET_POWER_STATE | 0x705 | status di byte rendah |
| SET_PIN_WIDGET_CONTROL | 0x707 | kendali di byte rendah (bit 5 in, 6 out, 7 headphone) |
| SET_EAPD | 0x70C | bit 1 = EAPD menyala |
| SET_CONNECT_SEL | 0x701 | indeks di byte rendah |
| SET_AMP_GAIN_MUTE | 0x300 | gain bit 6:0, mute bit 7, kiri 13, kanan 12, in 14, out 15 |
| SET_STREAM_FORMAT | 0x200 | format 16 bit |

Parameter: 0x00 vendor, 0x01 subsystem, 0x02 revisi, 0x04 jumlah node,
0x05 tipe function group, 0x09 widget capabilities, 0x0C pin capabilities,
0x0D amp-in capabilities, 0x0E panjang connection list, 0x0F connection list,
0x12 amp-out capabilities.

Widget capabilities: tipe di bit 23:20 (0 out, 1 in, 2 mixer, 3 selector,
4 pin, 5 power, 6 knob, 7 beep, 0xF vendor). Pin capabilities: bit 4 output,
bit 5 input, bit 2 presence detect, bit 3 headphone drive, bit 16 EAPD.
Amp capabilities: offset bit 6:0, jumlah step bit 14:8, besar step bit 22:16,
mute bit 31.

### BDL

Satu descriptor 16 byte: alamat 8 byte, panjang 4 byte, flag 4 byte (bit 0 =
interupt pada selesai). Jumlah descriptor = LVI + 1. Alamat list ada di
BDLPL/BDLPU.

## 3. Bentuk program

Program native `hda` (paket `xaqu-hda-driver`) adalah program ring-3 seperti
`snd` (virtio-snd) dan `input`:

1. **START** — `devmgr` mengirim pesan START milik HDA (`xaqu_hda::control`):
   perangkat, kanal kontrol suara, dan BAR mana yang berisi register block.
   Driver memetakan register block lewat `device.aperture` + `io_mapping`,
   persis seperti driver NVMe dan AHCI.
2. **Reset dan ring** — kontroler di-reset, CORB dan RIRB diberi memori yang
   di-pin, `CORBLBASE`/`RIRBLBASE` diisi, CORB dan RIRB dijalankan.
3. **Codec** — node-node codec dibaca (jenis, pin capabilities, config
   default, connection list), pin untuk suara keluar dipilih (speaker dulu,
   lalu headphone, lalu line out), dan jalur dari konverter ke pin dicari.
4. **Pemrograman jalur** — tiap widget dipath diberi power D0, mixer
   diberi انتخاب koneksi, amplifier diberi gain, pin diberi kendali
   (output + headphone) dan EAPD.
5. **Publikasi** — HELLO ke inti audio (`xaqu_sndctl`), READY membawa VMO
   buffer; buffer itu di-pin untuk perangkat, BDL ditulis, stream
   diprogram (LVI, CBL, FORMAT, BDLPL/BDLPU) dan dijalankan.
6. **Pelayanan** — setiap SUBMIT ditunggu lewat interupt kontroler lalu
   dijawab ELAPSED; HALT dijawab HALTED; STOP dijawab STOPPED dan stream
   dihentikan.

Logika yang bisa diuji host ada di crate `xaqu-hda`
(`src/lib/drivers/sound/hda/`): `controller.rs` (register & reset), `corb.rs`
(verb & ring), `codec.rs` (parameter & widget), `path.rs` (jalur), `bdl.rs`
(descriptor), `control.rs` (pesan START). 22 tes host menutup bagian itu.

## 4. Yang belum diverifikasi

- Jalur di perangkat nyata (belum ada boot dengan `-device intel-hda`).
- Codec yang sebenarnya di laptop ini: node mana yang pin speaker-nya,
  amplifier-nya, dan mixer-nya.
- Ukuran buffer dan period yang inti audio pilih; program memakai satu
  descriptor untuk seluruh buffer.
- Pel-ban (jack detect) dan unsolicited response belum ditangani; headphone
  tidak akan terdeteksi.
- Capture (mikrofon) belum diimplementasikan; hanya playback.

## 5. Jalur pembaruan

- **Pintasan**: status RIRB dibaca berulang (polling) alih-alih menunggu
  interupt RIRB.
  **Jalur pembaruan**: RIRBCTL.IRQ_EN dan INTCTL sudah dipasang dan port
  interupt sudah ter-bind — tinggal menunggu di port itu.
- **Pintasan**: satu descriptor untuk satu buffer.
  **Jalur pembaruan**: daftar descriptor bertahap (LVI > 0) dan `LPIB`
  untuk posisi, supaya period kecil tidak menambah banyak DMA.
