# Wi-Fi Intel AC 9560 (`8086:a0f0`) — Fase 7

Laptop ini: realme RMNBXXXX, Intel Core i3-1115G4 (Tiger Lake-UP3).
Wi-Fi: **Intel AC 9560**, CNVi, PCI `8086:a0f0` subsys `8086:0034`.

Dokumen ini adalah rujukan driver Wi-Fi di Xaqu: apa yang sudah ada, apa
yang belum, dan urutan pengerjaan berikutnya.

Status (2026-11-19): lapisan **firmware** sudah ada dan teruji di host —
crate `xaqu-iwl` (`src/lib/drivers/net/iwl/`) dengan 21 tes, clippy dan
fmt bersih. Yang belum ada: program native, transport PCIe, dan seluruh
lapisan 802.11/WPA2. **Tidak ada klaim bahwa Wi-Fi sudah terhubung.**

## 1. Perangkat

| apa | nilai | dari mana |
|---|---|---|
| host CNVi di bus | `8086:a0f0` | PnP Windows |
| subsistem | `8086:0034` | PnP Windows |
| radio | AC 9560 (keluarga 9000) | PnP Windows |
| Bluetooth | USB `8087:0aaa` | PnP Windows |

Perhatikan: pada CNVi yang tampil di bus adalah **host**-nya, bukan radio
nya. Tabel driver Linux (`drivers/net/wireless/intel/iwlwifi/pcie/drv.c`)
mempetakan `0xA0F0` ke beberapa konfigurasi (AX201 dan lain-lain);
subsistem `0034` tidak ada di tabel itu, dan radio sebenarnya (AC 9560)
ditemukan perangkatnya sendiri dari EEPROM/OTP — bukan dari id di bus.

## 2. Firmware

Radio Intel tidak punya kode sendiri di dalam chip: kodenya datang dari
luar sebagai berkas `.ucode`.

- Prefiks nama berkas: `iwlwifi-9000-pu-b0-jf-b0`
  (`IWL9000_FW_PRE`, `drivers/net/wireless/intel/iwlwifi/pcie/ctxt-info.c`
  di kernel v6.6).
- Nama berkas penuh: `<prefiks>-<api>.ucode`, misalnya
  `iwlwifi-9000-pu-b0-jf-b0-13.ucode` dan `...-14.ucode`.
- Isi berkas: header lalu daftar blok TLV. Header-nya
  `struct iwl_tlv_ucode_header`: empat byte nol, magic `0x0a4c5749`,
  64 byte nama yang bisa dibaca manusia, nomor versi, nomor build, dan
  delapan byte yang diabaikan driver. Setiap blok: tipe, panjang (data
  saja), data; blok dilapis supaya blok berikutnya mulai pada batas
  empat byte.

Berkas firmware harus ikut di dalam initramfs Xaqu. Repo ini belum punya
pemuat firmware di ruang pengguna; itu pekerjaan berikutnya (§5).

## 3. Yang sudah ada: crate `xaqu-iwl`

`src/lib/drivers/net/iwl/`, tanpa akses ke perangkat keras:

| modul | apa |
|---|---|
| `firmware` | header berkas firmware dan blok-blok TLV-nya: `Firmware::parse`, `Firmware::block`, `TlvType` |
| `device` | id perangkat di bus, nama berkas firmware yang diminta, `Devices::find` |
| `cmd` | header perintah dan jawabannya: `Header`, `encode`, `decode` |

21 tes host: berkas dengan header baik dibaca, berkas terlalu pendek dan
magic salah ditolak, blok yang melewati ujung berkas ditolak, blok sought
ditemukan, nama firmware untuk API 13 tepat
(`iwlwifi-9000-pu-b0-jf-b0-13.ucode`), perintah utuh kembali setelah
dienkode dan dibaca.

## 4. Yang belum ada

1. **Program native** yang memetakan BAR perangkat dan mengunggah
   firmware. Prasyarat: device PCI `8086:a0f0` diterbitkan kernel ke
   devmgr (device PCI sudah ada, tinggal baris tabelnya).
2. **Transport PCIe**: antrean TX/RX, header respons, management frames
   dari firmware.
3. **MAC 802.11**: pindai, associate, kecepatan, roam.
4. **WPA2**: supplicant, derivasi PMK/PTK, dan penyimpanan kunci.
5. **Radio**: baca EEPROM/OTP, kalibrasi, tabelCountry/regulatory.
6. **Bluetooth** (`8087:0aaa`): belum disentuh sama sekali.

Langkah 1–3 saja sudah memberi "terhubung ke jaringan terbuka"; WPA2
menambahkan 4.

## 5. Urutan pengerjaan

1. Program native `iwl`: device PCI → peta BAR → unduh firmware dari
   initramfs → parses dengan `xaqu-iwl` → unggah ke perangkat → hidupkan
   radio. Ini yang membuktikan rantai firmware bekerja.
2. Transport PCIe (TX/RX queue).
3. MAC: pindai dan associate (tanpa enkripsi dulu).
4. WPA2.

## 6. Sumber

- `drivers/net/wireless/intel/iwlwifi/fw/file.h` — header berkas, TLV.
- `drivers/net/wireless/intel/iwlwifi/fw/api/cmdhdr.h` — header perintah.
- `drivers/net/wireless/intel/iwlwifi/pcie/drv.c` — tabel perangkat.
- `drivers/net/wireless/intel/iwlwifi/pcie/ctxt-info.c` — nama firmware.
