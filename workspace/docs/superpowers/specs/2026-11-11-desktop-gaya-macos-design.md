# Desktop gaya macOS di atas Ferrix (pengganti Yoru) - desain

Tanggal: 2026-11-11
Revisi 1: target boot dan RAM terhadap LordOS, efisiensi banyak aplikasi.
Revisi 2: kompresi memori dan swap di kernel (Proyek B).
Status: menunggu review spec
Basis: Ferrix `9b339e738e52ad6235ab65f412284848eb83ebd6`, komponen di pin `components.toml`
(apps `3598f84`, tole `a8b356c`)

## 0. Dua proyek dalam satu spec

| | Proyek A: desktop | Proyek B: memori |
|---|---|---|
| Isi | UI gaya macOS dan pengatur Hemat (§3-§8) | kompresi memori dan swap di kernel (§9) |
| Ring | ring 3 (hyprix, yoru-shell, tole, init) | kernel inti `mm` (core ring), item tersertifikasi |
| Proses Ferrix | review biasa | desain ke konsultan sertifikasi dulu, id requirement dipesan, kontrol negatif |
| Ketergantungan | tidak menunggu B | tidak menunggu A |

Keduanya bisa jalan paralel. Hemat (§6) memakai kompresi begitu B mendarat; sebelum itu Hemat
hanya melepas page cache. Tiap proyek mendapat rencana implementasi sendiri.

## 1. Tujuan

Desktop Ferrix tampil dan berperilaku seperti macOS: menu bar global di atas, dok ikon di tengah
bawah, tombol jendela bulat di kiri, Spotlight, Control Center, Mission Control, Launchpad,
notifikasi, tampilan terang berkaca. Boot dan RAM idle minimal setara LordOS. Banyak aplikasi
bisa terbuka sekaligus dengan RAM dan CPU tetap hemat, seperti macOS: aplikasi yang tak
terlihat ditidurkan, memorinya dikompres di RAM, dan yang dingin dipindah ke swap di disk.
Driver dan jalur aplikasi Linux, Windows dan Steam (Chrome, Wine/Proton, yserver) tidak diubah.
Kernel hanya diubah untuk Proyek B: `mm`, jalur tulis ring blok, cgroupfs, dan procfs (§9.3).

Kriteria berhasil:
- Delapan komponen di §3 berjalan di QEMU dan lolos boot `test-compositor` di §11.
- Boot dan RAM idle lolos batas di §12, diukur dengan metode yang sama untuk Ferrix dan LordOS.
- Uji banyak aplikasi dan uji tekanan memori di §12 lolos.
- Semua gate Ferrix yang ada tetap lulus, termasuk gate sertifikasi untuk perubahan `mm`.

## 2. Keputusan yang sudah diambil

| Keputusan | Pilihan |
|---|---|
| Tata letak | macOS penuh, termasuk menu bar global |
| Cakupan menu global | bertahap: menu standar untuk semua app; menu asli hanya dari app tole |
| Komponen versi pertama | kedelapan komponen di §3, plus pengatur Hemat (§6) |
| Hubungan dengan Yoru | menggantikan Yoru; nama internal `yoru`, `yoru-shell`, `yoru.conf` dipertahankan |
| Tampilan bawaan | terang, berkaca, aksen biru; gelap tersedia lewat Control Center |
| Arsitektur | perluas yoru-shell (satu proses, banyak permukaan) dan hyprix |
| Saluran menu | protokol Wayland `ferrix_appmenu_v1` yang terikat ke `xdg_toplevel` |
| Pembanding kinerja | LordOS 0.8.8 hasil ukur 2026-11-11, bukan angka klaim di README-nya |
| Memori | kompresi di RAM dulu (B1-B3), swap disk terenkripsi di belakangnya (B4) |

## 3. Komponen desktop

```
 app tole (Terminal, Settings, Files)  -- ferrix_appmenu_v1 --+
 app GTK/Qt/Chrome/Steam (tanpa menu asli)                     |
                                                               v
 hyprix
   - titlebar.rs: opsi tombol di kiri, urutan tutup/minimalkan/layar penuh (baru)
   - appmenu: menyimpan pohon menu per jendela, event ke IPC (baru)
   - IPC Hyprland (.socket2, j/clients, dispatch): ada
                                                               |
 yoru-shell (satu proses, permukaan layer-shell)               v
   - MenuBar   : logo, nama app, menu, status, jam             (baru)
   - Dock      : ikon tengah, titik berjalan, pemisah          (ubah dari dok lama)
   - Spotlight : app, jendela, berkas, kalkulator              (ubah dari palet)
   - Control   : Wi-Fi, Bluetooth, volume, kecerahan, tema     (pindah dari quick settings)
   - Launchpad : grid layar penuh dari .desktop                (baru)
   - Notices   : banner kanan atas + riwayat                   (baru, membaca notifyd)
   - Hemat     : satu cgroup per app, tidurkan app tak terlihat (baru, §6)
 Mission Control : overview hyprix, gaya baru
 Tema            : yoru.conf dan token tole diganti set "terang kaca", varian gelap,
                   wallpaper gradien buatan sendiri (PNG, bukan SVG)
```

| Unit | Tugas | Bergantung pada |
|---|---|---|
| hyprix `appmenu` | simpan pohon menu milik tiap jendela; tolak klien yang bukan pemilik | `ferrix_appmenu_v1` |
| hyprix `titlebar.rs` | opsi `hyprix:titlebar:button_side = left`, warna tombol merah/kuning/hijau | kode titlebar yang ada |
| tole `set_menu` | API untuk app tole mengirim menu dan menerima `activated(id)` | `ferrix_appmenu_v1` |
| MenuBar | tampilkan menu jendela fokus atau menu standar | event IPC, dispatcher |
| Dock, Spotlight, Control, Launchpad, Notices | satu modul masing-masing di yoru-shell; tidak saling membaca internal | model app yoru-shell |
| Hemat | atur `cpu.weight`, `memory.high`, `memory.reclaim`, dan `cgroup.freeze` per app menurut keadaan jendela | cgroup v2 Ferrix, event IPC, Proyek B |

Dihapus dari Yoru: sidebar kiri dan kanan (isinya pindah ke Control dan Notices), pomodoro,
dok lebar, cuaca. Tidak disentuh: driver, yserver, ferrousli, Chrome/Steam/Proton.

## 4. Protokol `ferrix_appmenu_v1`

- Didefinisikan di `src/user/system/linux/compositor/protocol`.
- `get_menu(xdg_toplevel)` hanya untuk toplevel milik klien itu sendiri; selain itu error protokol.
- `set(tree)`: seluruh pohon dikirim ulang setiap kali berubah. Item: `id` (u32), label,
  pintasan, aktif, centang; pemisah; submenu satu tingkat.
- Batas: 16 menu atas, 256 item total, label 128 byte. Pelanggaran = error protokol.
- `activated(id)` dikirim ke pemilik saat item diklik.
- Objek menu dihancurkan bersama toplevel.

## 5. Alur data desktop

Fokus pindah: hyprix mengirim `activewindow` (ada) dan `appmenu>>` (baru, pohon atau kosong)
lewat `.socket2`. MenuBar menggambar menu asli, atau menu standar bila kosong.

Klik item asli: yoru-shell memanggil `dispatch appmenu <address> <id>`; hyprix mengirim
`activated(id)` ke app pemilik.

Menu standar (semua app):
- Nama app: Tentang, Sembunyikan (`movetoworkspacesilent special:minimized`), Keluar (`killactive`).
- Jendela: Minimalkan, Layar Penuh (`fullscreen`), Pindah ke Desktop N (`movetoworkspace`), Tutup.

Dock: daftar dari `.desktop`, titik berjalan dari `j/clients`, klik meluncurkan app lewat Hemat
(§6) atau `focuswindow`. Minimalkan memakai special workspace `minimized`.

Spotlight: app (`.desktop`), jendela (`j/clients`), nama berkas di `~` (maksimal 5000 entri,
300 ms, dipindai saat dibuka, tanpa indeks latar), kalkulator empat operasi dan kurung.

Control: fungsi quick settings yang ada (pulsed, jalur Settings untuk Wi-Fi dan Bluetooth,
backlight, portald untuk terang/gelap).

Notices: banner dari notifyd, riwayat 50 terakhir di memori, hilang saat logout.

Pintasan (SUPER adalah tombol sistem, sesuai `docs/DESKTOP.md` §6.4): SUPER Space Spotlight,
SUPER L Launchpad, SUPER Tab atau SUPER Up Mission Control, SUPER Q keluar, SUPER W tutup,
SUPER M minimalkan. Pintasan di item menu ditampilkan dan dipetakan ke SUPER.

## 6. Hemat: efisiensi banyak aplikasi (meniru App Nap)

Bahan yang sudah ada di cgroup v2 Ferrix (`docs/CGROUPS.md`): `cpu.weight` dan `cpu.max`
(S1, S2), `memory.high` dengan reclaim page cache bersih (M2), `cgroup.freeze` (F1).
Proyek B menambah reclaim memori anonim ke kompresi dan swap.

1. **Satu cgroup per app.** Dock, Spotlight, dan Launchpad meluncurkan app ke
   `<cgroup sesi>/app-<nama>-<n>` sehingga memori dan CPU tiap app terhitung terpisah.
   App yang diluncurkan dari terminal tetap di cgroup terminal.
2. **Keadaan app**, dari event IPC hyprix:
   - Aktif: jendelanya fokus atau terlihat. `cpu.weight` 100, `memory.high` max.
   - Latar: semua jendelanya diminimalkan, tersembunyi, atau di desktop lain. `cpu.weight` 10.
   - Tidur: Latar lebih dari 5 menit, tidak memutar suara (dicek ke pulsed), bukan app pada
     daftar pengecualian. Hemat menulis `memory.reclaim` ke cgroup app: page cache bersih
     dilepas, dan setelah Proyek B halaman anonim dingin dikompres. `cgroup.freeze` hanya di
     tahap 2.
   - Kembali Aktif: batas dicabut sebelum jendela ditampilkan; halaman terkompres kembali saat
     disentuh (B3 menambah baca-awal).
3. **Tekanan memori.** Bila `MemAvailable` di bawah 10% selama 10 s, app Latar dan Tidur
   di-reclaim lebih dulu, dari yang paling lama tidak dilihat. Bila masih di bawah 5% dan
   kompresi serta swap penuh, Notices menampilkan dialog "memori aplikasi habis" berisi daftar
   app dan pemakaian memorinya, dan pengguna memilih app yang ditutup. Hemat tidak pernah
   menutup app sendiri.
4. **Daftar pengecualian** di `yoru.conf` (`yoru:hemat:never = steam, ...`): app itu selalu
   Aktif. Game layar penuh di jalur GAME-MODE tidak disentuh.

Tahap 2 (setelah tahap 1 lolos, butuh pekerjaan APP-SUSPEND S1 sekitar 2 minggu ring-3):
app Tidur dibekukan dengan `cgroup.freeze`, dan hyprix, yserver, pulsed, serta driver GPU
menganggap klien beku sebagai tidur, bukan rusak (`docs/APP-SUSPEND.md`).

## 7. Penanganan error desktop

| Kejadian | Perilaku |
|---|---|
| yoru-shell crash | dijalankan ulang; jendela app tidak terpengaruh; menu dimuat ulang dari hyprix; semua cgroup app dikembalikan ke Aktif |
| pohon menu tidak valid atau melebihi batas | error protokol untuk app itu; MenuBar memakai menu standar |
| jendela ditutup | objek menu ikut hilang; cgroup app dihapus saat kosong |
| perangkat Control tidak ada | item disembunyikan |
| pemindaian Spotlight melewati batas | hasil sebagian dengan tanda "lebih banyak" |
| tulis ke cgroup gagal | app dibiarkan Aktif, dicatat ke log sekali |
| `hyprland.conf` milik pengguna | tetap menang, seperti aturan Yoru sekarang |

## 8. Hal desktop yang harus dicek sebelum implementasi

1. Apakah hyprix sudah mendukung special workspace untuk minimalkan; bila belum, tambahkan.
2. Apakah hyprix sudah menjalankan ulang `exec-once` yang mati; bila belum, tambahkan restart.
3. Apakah tole sudah punya menu popup yang bisa dipakai MenuBar; bila belum, menu popup
   ditambahkan di tole sebagai tugas tersendiri sebelum MenuBar.
4. Apakah SUPER L dan SUPER Up sudah terpakai di `yoru-keys.conf`; bila ya, ikatan lama diganti.
5. Di cgroup mana sesi desktop berjalan dan apakah sudah didelegasikan ke pengguna
   (`chown` delegasi di CGROUPS.md); bila belum, init memberi delegasi untuk sesi.
6. Apakah hyprix berhenti mengirim frame callback ke jendela yang diminimalkan atau di
   desktop lain (DISPLAY.md 2026-09-18 baru menyebut permukaan layer yang tertutup).
7. Penyebab `graphical.target` baru aktif sekitar 121 s.

## 9. Proyek B: kompresi memori dan swap

### 9.1 Keadaan sekarang

- Ferrix tidak punya swap sama sekali (`APP-SUSPEND.md` §3.2 dan §5). Reclaim hanya mengambil
  page cache bersih (`CGROUPS.md` §10); memori anonim selalu tinggal di RAM.
- Reclaim memakai satu jarum jam tanpa daftar aktif/tidak aktif; halaman panas juga terambil
  sekali per putaran (`CGROUPS.md` §10).
- Kernel hanya punya dekompresor (zlib, LZO, zstd untuk btrfs); belum ada kompresor.
- Swap mengubah jalur fault dan daftar halaman VMO yang menjadi dasar klaim memori item
  tersertifikasi (`docs/certification/MEMORY-AND-TIMING.md`). Desain ke konsultan dulu.
- Driver disk berjalan di ring 3, jadi penulisan halaman ke disk tidak boleh menunggu memori
  yang dibutuhkan driver disk itu sendiri.

### 9.2 Cara kerja (seperti macOS: kompres dulu, disk belakangan)

```
 halaman anonim dingin
        |  reclaim (Hemat lewat memory.reclaim, atau tekanan global)
        v
 kompresor LZ4 ---> kolam terkompres di RAM (maks 25% RAM)  <-- fault: dekompres, kembali
        |  kolam penuh, atau halaman tidak terkompres di bawah 50%
        v
 swap disk terenkripsi (kunci per boot, tidak disimpan)      <-- fault: baca, dekripsi
```

Setiap halaman yang keluar dari RAM diganti satu entri swap di tabel halaman dan VMO-nya.
Entri menunjuk ke kolam terkompres atau ke slot di area swap. Fault pada entri itu
mengembalikan halaman ke RAM.

Yang tidak pernah keluar dari RAM: halaman yang di-pin (objek GPU, DMA, `mlock`), memori
kernel, memori proses driver ring 3 dan devmgr, memori pid 1, dan halaman pemetaan bersama
yang ditulis lewat VMO bersama (tahap pertama).

### 9.3 Unit

| Tahap | Unit | Tempat | Ring | Poin (perkiraan) |
|---|---|---|---|---|
| B0 | Desain dan id requirement ke konsultan sertifikasi; bukti konsep tmpfs dari `APP-SUSPEND.md` §3.2 | docs | - | 3 |
| B1 | Entri swap: halaman anonim keluar dan masuk lewat fault, dengan penyimpanan di RAM tanpa kompresi | `mm`, `user/space.rs`, `user/vmo.rs` | core | 13 |
| B2 | Penuaan halaman: bit accessed, daftar aktif dan tidak aktif, sehingga yang keluar adalah halaman dingin | `user/cache.rs`, `mm` | core | 5 |
| B3 | Kompresor LZ4 sendiri (crate `no_std` di `src/lib`, uji host, fuzz bolak-balik, Miri) dan kolam terkompres; baca-awal saat app kembali Aktif | `src/lib`, `mm` | core | 8 |
| B4 | Swap disk: berkas swap praalokasi tanpa COW di volume `/data`, enkripsi per boot, penulisan tanpa alokasi | `mm`, ring blok | core | 13 |
| B5 | Antarmuka Linux: `memory.reclaim` untuk anonim, `memory.swap.current` dan `memory.swap.max`, `memory.stat` (`zswap`, `swapcached`), `/proc/meminfo` `SwapTotal` dan `SwapFree`, `/proc/swaps` | `fs/cgroupfs`, procfs | load | 3 |
| | **Total** | | | **sekitar 45**, perkiraan kasar |

Poin mengikuti satuan backlog Ferrix; angkanya tebakan dan diukur ulang setelah B0.

### 9.4 Aturan gagal-tertutup

- Kompresi gagal atau kolam penuh: halaman tetap di RAM.
- Tulis ke disk gagal: halaman tetap di RAM, area swap ditandai rusak, swap disk dimatikan.
- Baca dari disk atau dekripsi gagal: job pemilik halaman dibunuh, bukan kernel.
- Tidak ada alokasi di jalur tulis swap; buffer disiapkan saat swap diaktifkan.
- Swap disk mati secara bawaan sampai B4 lolos uji di N150; kompresi RAM hidup secara bawaan.

### 9.5 Risiko

- Perubahan di jalur fault adalah wilayah paling sensitif kernel; tiap langkah butuh kontrol
  negatif dan review konsultan, sehingga waktunya bisa lebih lama dari poin di atas.
- Data app (termasuk token sesi) tertulis ke disk; kunci per boot yang tidak disimpan membuat
  isinya tidak terbaca setelah reboot.
- Keausan SSD dan waktu: kolam terkompres di depan disk mengurangi tulis ke disk.
- Nilai rasio kompresi dan kecepatan LZ4 di Ferrix belum diketahui; diukur di B3.

## 10. Di luar cakupan

Menu asli GTK dan Qt (dbusmenu), indeks berkas latar, widget, suara, cuaca, pembekuan app
(tahap 2 di §6), swap di partisi khusus (mengubah tabel partisi), hibernasi, swap untuk app
yang dibekukan lintas reboot (APP-SUSPEND S3).

## 11. Pengujian (QEMU dulu)

Desktop:
- Unit (host): validasi pohon menu, pemetaan menu standar, pencocokan Spotlight, kalkulator,
  mesin keadaan Hemat (Aktif, Latar, Tidur, kembali Aktif; app bersuara tidak Tidur).
- Protokol: klien yang memasang menu di jendela lain ditolak (kontrol negatif); pohon melebihi
  batas menghasilkan error.
- Boot `test-compositor` baru: `macos-desktop` (bar, dok tengah, tombol kiri, banding piksel),
  `appmenu` (app tole mengirim menu, klik QEMU memicu `activated`), `spotlight`, `launchpad`,
  `control`, `notices`, `multiapp` (§12.3).
- Regresi: `test-boot`, boot `test-compositor` lain, Chrome dan GTK. Boot `yoru`, `yoru-dock`,
  `sidebars` diganti boot di atas.

Memori:
- Host: LZ4 bolak-balik pada data acak, nol, dan halaman nyata; fuzz; Miri.
- Self-check boot: program menulis 256 MB anonim, di-reclaim, dibaca kembali identik; kontrol
  negatif (entri swap yang sengaja dirusak) menghentikan boot dengan pesan cek itu sendiri.
- Gagal-tertutup: tulis disk disuntik gagal, halaman tetap di RAM; baca disuntik gagal, job
  dibunuh dan kernel hidup.
- Halaman yang di-pin tidak pernah keluar (kontrol negatif).
- Regresi: semua gate sertifikasi `mm`, `test-boot` di tiga arsitektur, Chrome, Steam/Proton.

## 12. Anggaran kinerja

### 12.1 Metode yang sama untuk kedua sistem

Satu skrip ukur menjalankan Ferrix dan ISO LordOS di QEMU yang sama: KVM, 4 core, 4 GB,
1920x1080, tanpa self-check Ferrix (`ferrix.checks=skip`). Dua titik waktu diambil dari awal
QEMU: (a) dok desktop pertama kali tergambar, (b) sistem selesai boot (`graphical.target`
aktif di keduanya). RAM terpakai = MemTotal - MemAvailable setelah 65 s idle di desktop.
Hasil 2026-11-11 diukur dengan metode berbeda sehingga hanya menjadi acuan awal.

### 12.2 Boot dan RAM idle

| Ukuran | Ferrix (Yoru) 2026-11-11 | LordOS 0.8.8 2026-11-11 | Batas desktop baru |
|---|---|---|---|
| Sampai dok tergambar | 6,61 s | belum diukur | lebih cepat dari LordOS dan 8 s ke bawah |
| Sampai boot selesai | sekitar 121 s (menunggu `lode-pstore`) | 12,57 s (systemd-analyze) | sama atau lebih cepat dari LordOS |
| RAM terpakai idle | sekitar 153 MiB | 428 MB | 200 MiB ke bawah, dan selalu di bawah LordOS |
| Proses wallpaper | 51 MB (SVG) | - | di bawah 20 MB |

Pekerjaan boot: `lode-pstore.service` tidak boleh menahan `graphical.target`. Penyebab
tunggu sekitar 121 s diselidiki dulu dari log boot, baru diperbaiki (ubah urutan unit atau
batas waktu fetch). Bila penyebabnya di kernel atau driver, dicatat sebagai temuan dan tidak
dikerjakan di proyek ini.

### 12.3 Uji banyak aplikasi (`multiapp`)

Buka Terminal, Files, Settings, Chrome dengan 3 tab lokal, dan satu app GTK. Minimalkan
semua kecuali Terminal. Setelah 6 menit:

- CPU idle di atas 95% (app Latar tidak memakan CPU).
- Keempat app tersembunyi berstatus Tidur.
- Total `memory.current` app Tidur lebih kecil dari saat baru diminimalkan; setelah Proyek B,
  minimal 40% lebih kecil (target awal, disesuaikan dengan rasio LZ4 yang terukur di B3).
- Membuka kembali app Tidur dari dok sampai jendela tergambar: 1 s ke bawah.
- Meluncurkan app tole dari dok sampai jendela tergambar: 1 s ke bawah.

### 12.4 Uji tekanan memori

QEMU 1 GB. Buka app sampai total memori app melebihi RAM fisik.

- Sebelum Proyek B: dialog memori muncul saat `MemAvailable` di bawah 5%, tidak ada app yang
  ditutup otomatis, sistem tidak hang.
- Setelah B3 (kompresi): total memori app yang terbuka mencapai minimal 1,4 kali RAM fisik
  sebelum dialog muncul; app yang dibuka kembali tetap utuh.
- Setelah B4 (swap disk 2 GB): minimal 2 kali RAM fisik; desktop tetap responsif (dok
  bereaksi dalam 200 ms).

## 13. Merek dan lisensi

Tidak memakai logo, ikon, font, wallpaper, atau nama Apple. "App Nap" hanya disebut di
dokumen ini sebagai pembanding; nama fitur di UI adalah "Hemat". Ikon dari Phosphor (MIT,
sudah vendored). LZ4 ditulis sendiri dari spesifikasi formatnya, tanpa menyalin kode
implementasi lain. Tagline LordOS "Everything you need. Nothing you don't." tidak dipakai;
konten LordOS tidak tercakup lisensi MIT. Tagline sendiri diputuskan pemilik proyek.

## 14. Repositori yang tersentuh

- `ferrix-os/ferrix`: `src/user/system/linux/compositor/{protocol,hyprix}`, `hyprix/themes/`,
  urutan unit `lode-pstore` dan delegasi cgroup sesi di init bila perlu, xtask untuk boot uji
  baru dan skrip ukur §12.1; Proyek B di `src/kernel` (`mm`, `user/`), `src/lib` (LZ4),
  `src/lib/fs/common/cgroupfs`, procfs, dan dokumen sertifikasi.
- `ferrix-os/apps`: `yoru-shell` (termasuk Hemat), dan app tole yang mengirim menu (Settings, Files).
- `ferrix-os/tole`: API `set_menu` dan menu popup.

Perubahan di tiga repositori; `components.toml` dipin ulang setelah tiap komponen lolos.
