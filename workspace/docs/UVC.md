# Kamera USB (UVC) — Fase 8

Laptop ini punya kamera USB `2b7e:b514` (Realtek, USB Video Class).
Dokumen ini menulis apa yang sudah ada, apa yang belum, dan mengapa.

## 1. Crate `xaqu-uvc` (sudah ada, 26 tes)

`src/lib/drivers/video/uvc/` — logika murni atas byte yang kamera kirim:

| Modul | Isi |
|---|---|
| `descriptor.rs` | descriptor VideoControl & VideoStreaming, dan penelusuran configuration descriptor (`walk_configuration`) → `VideoLayout` |
| `format.rs` | format piksel (YUY2, NV12, MJPEG) dan laju frame dari interval 100 ns |
| `streaming.rs` | header payload transfer: panjang header, bit "akhir frame" (UVC 1.0 bit 1 **atau** UVC 1.1 bit 6), bit "rusak", panjang payload |
| `request.rs` | control request kelas UVC (`SET_CUR`, `GET_CUR`, `GET_MIN/MAX/RES`) |

Spesifikasi yang diikuti: USB Video Class 1.5.

## 2. Program native `uvc` (baru)

`src/user/system/native/drivers/video/uvc/` — paket `xaqu-uvc-driver`,
bin `uvc`. Alurnya:

1. BIND → interface channel; HELLO → identitas + endpoint + VMO.
2. **Baca configuration descriptor** lewat control request ke *device*
   (`GET_DESCRIPTOR`, `wValue = 0x0200`), lalu parse dengan
   `xaqu_uvc::descriptor::walk_configuration`.
3. **Says** apa yang kamera tawarkan: interface kontrol, interface
   streaming, alternate setting yang streaming, dan tiap format/frame
   (geometri, laju, layout piksel).
4. **Pilih alternate setting** yang streaming: `SET_INTERFACE` ke interface
   sendiri dengan `wValue` = alternate setting streaming.
5. Buka endpoint IN (isokron, atau bulk bila itu yang ada), baca tiap
   transfer, parse header payload, dan **hitung frame**: berapa frame yang
   selesai dan berapa byte-nya.

### Yang tidak dikerjakan, dan kenapa

- **Frame tidak dirakit dan tidak dikirim ke mana pun.** Xaqu belum punya
  pipeline kamera — tidak ada inti yang menerima frame lalu menampilkannya.
  Yang dapat dilakukan tanpa itu: يقولkan apa yang kamera tawarkan, dan
  menghitung frame yang datang.
- **Kontrol kamera** (exposure, gain, white balance) tidak dikirim:
  `SET_CUR` perlu nilai per kontrol, dan nilainya belum diuji di sensor
  nyata.

## 3. Perubahan kebijakan: `SET_INTERFACE`

`xaqu_usbif::allowed` (`src/lib/proto/usbif/src/lib.rs`) kini mengizinkan
`SET_INTERFACE` **ke interface yang diklaim driver itu sendiri**
(`wIndex` = nomor interface-nya). Sebelumnya alternate setting tidak
boleh diakses, dan kamera tidak bisa mulai streaming tanpa itu: alternate
setting 0 adalah yang diam, alternate setting 1 yang membawa frame.
Standar USB mengizinkan hal yang sama pada interface yang sudah diklaim
(`usbfs` juga mengizinkan), jadi ini penyempitan, bukan pelonggaran.

Permintaan ke interface **lain** tetap ditolak.

## 4. Wiring

- `devmgr`: baris baru di `USB_DRIVERS` (`src/user/system/native/devmgr/src/usb.rs`)
  — vendor `0x2b7e`, produk `0xb514`, class `0x0E`, subclass `0x02` (interface
  streaming), image `uvc`, kelas baru `Class::Video`. Tabel 5 → 6 baris.
- `xtask`: `PROGRAMS` memuat `xaqu-uvc-driver` → bin `uvc`, ke `lib/drivers`.
- `Cargo.toml`: program menjadi anggota workspace dan dependensi
  `xaqu-uvc-driver`.

## 5. Verifikasi

- 26 tes crate `xaqu-uvc` hijau; 4 tes `xaqu-usbif` hijau (termasuk
  `SET_INTERFACE` ke interface sendiri boleh, ke interface lain tidak).
- Build `x86_64-unknown-none` program `uvc` bersih, clippy bersih, fmt bersih.

## 6. Sumber

- USB Video Class specification 1.5 (UVC 1.5 §3): descriptor format dan
  streaming interface.
- `drivers/media/usbvideo/uvcvideo.c` — driver Linux yang jadi acuan
  struktur descriptor.
