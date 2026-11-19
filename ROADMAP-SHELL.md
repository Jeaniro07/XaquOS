# Roadmap: Desktop Rust bergaya macOS di atas Linux

Arsitektur:

    Linux kernel (driver GPU, input, audio)
      -> Compositor Wayland (Rust, Smithay)      : jendela, animasi, blur, efek
      -> Shell (Rust, Iced / Slint)              : dock, menu bar, Control Center, launcher
      -> Aplikasi (Rust + Wayland, atau GTK/Qt apa saja)
      -> Distro (Arch/Debian minimal + ISO installer)

Lingkungan dev: WSL2 + WSLg (compositor jalan sebagai jendela di Windows).
Uji di hardware: VM Proxmox, lalu PC asli.

## Arah desain: ringan dan jelas

Acuan interaksi: [kanal Nava OS](https://www.youtube.com/@NavaOS-h6c), terutama [Command Bar & Terminal](https://www.youtube.com/watch?v=HyYjPYj4nKA). Ambil prinsipnya, bukan identitas visual atau asetnya. untuk desain gunakan 

- Utamakan permukaan desktop yang tenang; tampilkan kontrol saat dibutuhkan, bukan sebagai panel yang terus memenuhi layar.
- Buat launcher keyboard sebagai jalan cepat untuk membuka aplikasi dan berpindah jendela.
- Pakai hierarki teks, jarak, dan status yang jelas; hindari dekorasi yang tidak membantu tugas.
- Batasi blur dan animasi: gunakan hanya jika memperjelas konteks, dan ukur dampak performanya sebelum diaktifkan.
- Jaga setiap tahap tetap kecil; tambah efek hanya setelah fungsi dasar terasa cepat dan mudah dipahami.

## Tahap 1 - Compositor jalan (1-2 minggu)
- [x] Fondasi `smallvil` dari repo Smithay.
- [x] Jalankan backend winit di WSLg; `run.sh` membuka `foot` dan argumennya diteruskan.
- [x] `weston-terminal` dan `foot` tersambung; handshake `xdg_toplevel` teramati.
- [ ] Verifikasi klik pointer langsung pada jendela WSLg; uji injeksi klik otomatis tidak andal pada jalur X11/Wayland bertingkat.
- Selesai bila: aplikasi Wayland tampil dan bisa diklik di dalam compositor.

## Tahap 2 - Manajemen jendela (2-4 minggu)
- [x] Dekorasi server-side dengan tombol merah-kuning-hijau, fokus, geser, maximize, dan bayangan.
- [x] Fullscreen xdg-toplevel; dekorasi disembunyikan dan posisi/ukuran dipulihkan saat keluar.
- [x] Minimize via tombol kuning; pulihkan jendela terakhir dengan `Ctrl+Alt+R`.
- [ ] Tambahkan resize tepi yang konsisten dan bulatkan isi jendela, bukan hanya bingkainya.
- [ ] Verifikasi tiga jendela ditumpuk, digeser, diubah ukurannya, diminimalkan/dipulihkan, dan ditutup.

## Tahap 3 - Rendering cantik (3-6 minggu)
- Renderer OpenGL (GlesRenderer Smithay) dengan shader: sudut membulat, blur latar (Kawase blur).
- Animasi berbasis spring: buka/tutup jendela, minimize ke dock.
- Wallpaper.
- Selesai bila: animasi 60 fps di VM dengan GPU.

## Tahap 4 - Shell (4-8 minggu)
Setiap komponen = klien layer-shell (protokol wlr-layer-shell):
- Menu bar atas: menu aplikasi aktif, jam, status jaringan/baterai.
- Dock bawah: ikon aplikasi, indikator jalan, efek magnifikasi.
- Launcher (gaya Spotlight / Command Bar Nava) dengan pencarian.
- Control Center: WiFi (NetworkManager via D-Bus), volume (PipeWire), kecerahan.
- Notifikasi (org.freedesktop.Notifications).

## Tahap 5 - Sesi penuh (2-4 minggu)
- Backend DRM/KMS + libinput: compositor jalan langsung di TTY, tanpa WSL.
- Login screen (greetd + greeter Rust).
- Multi-monitor, HiDPI scaling.

## Tahap 6 - Distro dan installer
- Base: Arch Linux atau Debian minimal.
- Paket compositor + shell, tema ikon dan font (Inter / SF-like).
- Bangun ISO (archiso atau live-build) dan installer grafis.
- Selesai bila: ISO boot di PC asli sampai desktop.

## Referensi
- Smithay - https://github.com/Smithay/smithay (contoh: smallvil, anvil)
- Niri (compositor Rust, animasi bagus) - https://github.com/YaLTeR/niri
- COSMIC (desktop Rust System76) - https://github.com/pop-os/cosmic-epoch
- cosmic-comp (compositor COSMIC) - https://github.com/pop-os/cosmic-comp
- Iced - https://github.com/iced-rs/iced
- Slint - https://slint.dev
- Wayland Book - https://wayland-book.com
- wlr-protocols (layer-shell) - https://gitlab.freedesktop.org/wlroots/wlr-protocols
- SwayFX (contoh blur/sudut membulat) - https://github.com/WillPower3309/swayfx
