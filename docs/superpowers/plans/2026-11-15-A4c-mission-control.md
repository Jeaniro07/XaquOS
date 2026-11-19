# A4c: Mission Control — overview hyprix digambar ulang (gaya baru) — Rencana Implementasi

> **Untuk pelaksana:** kerjakan task demi task, berurutan. Tiap task dimulai dengan tes yang gagal, lalu kode, lalu tes lulus, lalu commit. Izin commit dan eksekusi dilimpahkan koordinator untuk rencana ini. Repo yang disentuh: `ferrix` saja (hyprix, xtask, docs). Repo `apps` (`xaqu-shell`) **tidak** disentuh. Tidak ada push, tidak ada PR, tidak ada ISO.

**Goal:** Mission Control adalah overview hyprix yang digambar ulang: satu layar penuh berisi ruang-ruang — satu per workspace — yang ditata berderet seperti macOS, dan di dalam tiap ruang jendela-jendelanya sebagai kartu, yaitu permukaan jendela asli yang dikecilkan, bukan gambar. Satu dispatcher baru, `mission`, yang menggambarnya; tidak ada jendela baru, tidak ada proses baru, tidak ada API IPC baru. Klik sebuah kartu memindahkan layar ke workspace jendela itu dan memfokusnya; Escape atau klik di luar menutup tanpa mengubah apa pun.

**Arsitektur:**

1. Mission Control **bukan permukaan baru di `xaqu-shell`** dan bukan jendela Wayland baru: ia adalah gaya baru dari overview hyprix. Spec §3 menulis baris "Mission Control : overview hyprix, gaya baru" di luar daftar "Dock, Spotlight, Control, Launchpad, Notices: satu modul masing-masing di yoru-shell", jadi tidak ada modul shell yang dibuat. Konsekuensinya: tidak ada proses baru, tidak ada socket, tidak ada markup `.tole` baru — hyprix tidak memakai tole sama sekali, overview digambar compositor.
2. Bentuk kodenya: `hyprix/src/overview.rs` mendapat `pub enum Style { Classic, Mission }` dan field `style` pada `Overview`. `Classic` (perilaku sekarang) dan `Mission` memakai input yang sama (`take_input`) tetapi tata letak berbeda (`tiles` → `mission_tiles`). Dispatcher baru `mission` (nama baru, hanya di `hyprix/src/act.rs`; sama seperti `overview` dan `switcher`, ia tidak masuk ke `compositor_layout::Dispatcher`).
3. Sumber data: `compositor_layout::State` langsung, bukan IPC — `state.workspaces()` (ruang yang ada), `state.listing()` (jendela dan tempatnya), `state.minimized_windows()`, `state.active_workspace(monitor)`, `state.workspace_name(ws)`. `j/clients` dan `j/workspaces` tidak dipakai Mission Control; tidak ada API baru sama sekali.
4. Tata letak: satu **ruang** per workspace, ruang-ruang berderet mendatar seperti macOS (baris kedua bila ruang-ruang tidak muat pada lebar yang bisa dipakai, ruang paling sempit 240 px). Di dalam ruang, jendela workspace itu menjadi **kartu**: permukaan jendela asli yang diskalakan ke dalam sel, memakai `overview::grid` dan `overview::fit` yang sudah ada, jadi isinya hidup, bukan potret. Jendela yang diminimalkan ikut sebagai kartu workspace asalnya dan digambar redup (`MINIMIZED_OPACITY`). Workspace tanpa jendela tetap jadi ruang: berlabel, tanpa kartu.
5. Label ruang ("Desktop 1") digambar compositor sendiri sebagai permukaan milik compositor, seperti title bar: modul baru `hyprix/src/spaces.rs` yang menghasilkan `titlebar::Rendered` (bytes/width/height sudah `pub`) dan satu `Placed` dengan WindowId palsu, disambung ke `state/draw.rs` lewat peta `decorations` (pola `Bars::decorate`). Alasannya dua: shell tidak boleh menggambar di atas overview (compositor mengambil pointer dan keyboard selama overview hidup — `overview::take_input`), dan tata letak tidak boleh ada di dua tempat.
6. Klik kartu: `overview::take_input` menangkap pointer seperti sekarang; pada gaya Mission, klik kartu berarti `state.dispatch_str("workspace", "<id>")` lalu `state.focus_window(window)` (jendela yang diminimalkan kembali sendiri: `focus_window` → `unminimize_now`), lalu Mission Control menutup. `Escape` atau klik di luar menutup tanpa mengubah apa pun. `Tab`/panah memilih kartu, `Return` memilih — sama seperti overview.
7. Lapisan: kartu digambar pada frame yang sama seperti overview klasik (`state/draw.rs`: `crate::overview::outputs(&self.state, &self.overview)`), wallpaper tetap di belakang; tidak ada layer-shell baru. Warna kartu dan label datang dari tema compositor (`hyprix/themes/xaqu.conf` → `compositor_render::Style`); terang/gelap shell (A4b, berkas `appearance`) tidak mengubah compositor, jadi kartu mengikuti tema compositor apa adanya.
8. Hemat (A5): Mission Control membaca `state.minimized_windows()` seperti overview sekarang. Jendela yang *tidur* (A5) masih punya kartu karena permukaannya ada, tetapi isinya basi — belum diuji (lihat "Yang belum").

**Tech stack:** Rust 2024; repo `ferrix` saja: `hyprix` (overview, spaces, act, draw), `compositor-layout` (dibaca, tidak diubah), `xtask` (boot), `docs`. Kode baru hanya Rust; tidak ada crate baru, tidak ada dependency baru.

**Spec:** `docs/superpowers/specs/2026-11-11-desktop-gaya-macos-design.md` §3 (baris "Mission Control : overview hyprix, gaya baru", dan tabel "satu modul masing-masing di yoru-shell"), §5 (pintasan: "SUPER Tab atau SUPER Up Mission Control"), §6 (Hemat), §8 butir 4 (ikatan lama yang sudah dipakai dipindah, bukan dihapus), §11 (uji: boot QEMU), §12 (anggaran). Sidebar: `docs/DESKTOP.md` §6.2 ("Switcher, overview | hyprix's own | restyled"), §6.4 (SUPER tanpa Alt/Ctrl), §6.9 (pola boot yang sudah jadi).

**Gaya kode:** "ponytail". Penyederhanaan ditandai `// Ponytail: ... Upgrade path: ...`.

---

## Aturan eksekusi

- Semua perintah di WSL Arch; repo ferrix `/root/ferrix` (hanya dibaca).
- Worktree baru `/root/ferrix-c` dari `9e2afe0fe` (tip `desktop/a4b-control-center`): `git worktree add -b desktop/a4c-mission-control /root/ferrix-c 9e2afe0fe`. Worktree `/root/ferrix-a` milik A4b tidak disentuh.
- `export CARGO_TARGET_DIR=$HOME/ferrix/target` — **satu target bersama**; jangan target kedua (disk C: tinggal ~2 GB).
- Branch `desktop/a4c-mission-control`. Tidak ada push, tidak ada PR, tidak ada ISO, tidak ada `cargo xtask check` penuh.
- Commit ferrix **tanpa** trailer `Co-Authored-By`: hook `.githooks/commit-msg` menolak trailer itu (dan menolak penulis bermuatan "claude"/"anthropic"); hook tidak dilewati.
- Build dan boot panjang dijalankan terlepas (`setsid nohup ... &`, log + berkas `done`), tanpa `sleep` di depan; satu QEMU pada satu waktu.
- Repo `apps` tidak disentuh; tidak ada worktree apps, tidak ada `.tole` baru, tidak ada perubahan di `xaqu-shell`.

## Keputusan

1. [KEPUTUSAN] **Mission Control = gaya baru dari overview hyprix, bukan modul shell baru.** Alasannya spec §3: baris "Mission Control : overview hyprix, gaya baru" berdiri sendiri, di luar baris "Dock, Spotlight, Control, Launchpad, Notices: satu modul masing-masing di yoru-shell". Konsekuensi: nol proses baru, nol socket, nol IPC baru, dan `xaqu-shell` tidak tersentuh sama sekali.
2. [KEPUTUSAN] **Ruang = workspace, ruang satu baris; kartu = permukaan jendela asli yang diskalakan (thumbnail hidup), bukan gambar potret.** Alasannya: compositor sudah punya `overview::grid` + `overview::fit`, dan memakai permukaan jendela sendiri menghindari screencopy yang belum ada. Kalau kartu jadi gambar, layar tidak bisa interaktif dan isinya basi.
3. [KEPUTUSAN] **Label ruang digambar compositor sendiri** (modul baru `hyprix/src/spaces.rs`, permukaan palsu seperti title bar), teks "Desktop N" untuk workspace bernomor dan nama aslinya bila workspace itu bernama. Alasannya: shell tidak boleh menggambar di atas overview, dan logika tata letak harus satu sumber kebenaran.
4. [KEPUTUSAN] **Pintasan: SUPER Tab = Mission Control; switcher pindah ke SUPER K / SUPER SHIFT K; overview klasik pindah ke SUPER O; bindr SUPER polos dilepas; SUPER Up tidak disentuh.** Yang dipindah: `bind = SUPER, Tab, switcher, next` → `SUPER, K, switcher, next`; `bind = SUPER SHIFT, Tab, switcher, prev` → `SUPER SHIFT, K, switcher, prev` (sepasang, satu tombol muscle memory); `bindr = SUPER, Super_L, overview, toggle` → `bind = SUPER, O, overview, toggle`. Alasan: spec §5 memberi "SUPER Tab atau SUPER Up Mission Control"; SUPER Up sudah dipakai `snapwindow, max` (spec §8 butir 4) dan dikunci pengguna, jadi Mission Control mengambil SUPER Tab; K dan O kosong di `xaqu-keys.conf`; SUPER polos dilepas supaya ini terlihat oleh pengguna, dan dicatat di `docs/DESKTOP.md` §6.10. `bindr` tidak dipakai lagi di berkas itu: `bindr = SUPER, Super_L` adalah tombol yang ditekan tanpa tekan lain, dan Mission Control memakai `bind` biasa. Tidak ada SUPER CTRL dan tidak ada SUPER ALT (§6.4; tes tema menolak keduanya).
5. [KEPUTUSAN] **Overview klasik tetap hidup, di SUPER O; Mission Control adalah mode kedua dari kode yang sama.** Alasannya: bind lama tidak boleh dihapus (keputusan pengguna) dan dua permukaan tidak boleh berbagi satu bind; mode terpisah juga membuat regresi overview klasik mustahil. Kalau nanti overview klasik tidak dibutuhkan, ia dapat dihapus sebagai pekerjaan sendiri.
6. [KEPUTUSAN] **Klik kartu = pindah workspace lalu fokus; klik di ruang tanpa kartu dan Escape = batal.** Alasannya: kartu adalah jendela, jadi "ke sana" berarti workspace yang sama dan jendela yang fokus; `focus_window` memulihkan yang diminimalkan sendiri.

## Peta berkas dan status

| Task | Repo | Berkas | Status |
|---|---|---|---|
| 1 | ferrix | `src/user/system/linux/compositor/hyprix/src/overview.rs`: `Style`, `mission_tiles`, `Space`, `space_label` | belum |
| 2 | ferrix | `src/user/system/linux/compositor/hyprix/src/spaces.rs` (baru), `src/state/draw.rs`, `src/state.rs` | belum |
| 3 | ferrix | `src/user/system/linux/compositor/hyprix/src/overview.rs` (klik), `src/act.rs` (dispatcher `mission`) | belum |
| 4 | ferrix | `src/user/system/linux/compositor/hyprix/themes/xaqu-keys.conf`, `src/user/system/linux/compositor/hyprix/tests/theme.rs` | belum |
| 5 | ferrix | `tools/common/xtask/src/compositor/mission.rs` (baru), `tools/common/xtask/src/compositor.rs` | belum |
| 6 | ferrix | `docs/DESKTOP.md` (§6.10), `docs/superpowers/plans/2026-11-11-peta-rencana.md` (baris A4c) | belum |

## Task 1: model ruang dan kartu

- [ ] Tes gagal lebih dulu (baru, di `hyprix/src/overview.rs`, `mod tests`; pola `desktop()` yang sudah ada di berkas itu):
  - `the_spaces_are_one_for_each_workspace` — dua workspace, satu di antaranya kosong: `mission_spaces` memberi dua ruang.
  - `a_window_is_a_card_of_its_own_workspace` — jendela di workspace 2 jadi kartu ruang 2, bukan ruang 1.
  - `a_minimized_window_is_a_card_of_the_workspace_it_left` — `state.minimize` lalu kartu muncul di workspace asalnya dengan `minimized: true`.
  - `the_label_of_a_numbered_workspace_is_desktop_one` — `space_label` memberi "Desktop 1".
- [ ] Implementasi (di `hyprix/src/overview.rs`):
  - `pub enum Style { Classic, Mission }` dan `pub style: Style` pada `Overview` (default `Classic`).
  - `pub struct Space { pub workspace: WorkspaceId, pub rect: Rect, pub cards: Vec<Tile> }`
  - `pub fn mission_spaces(state: &State, area: Rect) -> Vec<Space>` — ruang dari `state.workspaces()` yang `.0 > 0`, satu baris (baris kedua bila penuh), tinggi tetap; di dalam ruang, kartu dari `state.listing()` (yang `workspace`-nya ruang itu) memakai `grid()` + `fit()` yang sudah ada, `minimized` dari `state.minimized_windows()`.
  - `pub fn space_label(state: &State, workspace: WorkspaceId) -> String` — "Desktop N" bila `state.workspace_name(ws)` angka, kalau tidak nama itu sendiri.
- [ ] Cara menjalankan: dari `/root/ferrix-c/src/user/system/linux/compositor/hyprix`, `export CARGO_TARGET_DIR=$HOME/ferrix/target`, `cargo test overview`; lalu `cargo fmt --check` dan `cargo clippy --all-targets -- -D warnings` untuk crate itu saja.
- [ ] Commit (tanpa trailer): `feat(hyprix): the mission control's spaces and cards`

## Task 2: menggambar ruang, kartu, dan label

- [ ] Tes gagal lebih dulu (baru, di `hyprix/src/spaces.rs`):
  - `a_label_is_drawn_with_the_workspace_name` — `Spaces::draw` untuk ruang bernama "Desktop 2" menghasilkan `Rendered` yang tidak kosong dan lebarnya lebih besar dari tinggi.
  - `the_space_ids_are_not_a_clients_windows` — `space_id(0)` tidak sama dengan WindowId mana pun yang dipakai klien (bit `1 << 61`, bukan `titlebar::BAR` yang `1 << 62`).
- [ ] Implementasi:
  - `hyprix/src/spaces.rs` (baru) — `pub struct Spaces { rendered: BTreeMap<WindowId, titlebar::Rendered> }`, `pub fn space_id(index: usize) -> WindowId`, `pub fn draw(&mut self, spaces: &[Space], scale: f64, style: &titlebar::Style)`: menggambar teks label dengan `compositor_text` (`FontDescription::pango`, `fonts.shape_ellipsized`, `fonts.draw_run`) ke `tiny_skia::Pixmap`, lalu `titlebar::Rendered { bytes, width, height }` (pola `titlebar::Bars::draw` dan `argb`); `pub fn rendered(&self) -> &BTreeMap<WindowId, titlebar::Rendered>`.
  - `hyprix/src/state.rs` — field baru `spaces: crate::spaces::Spaces` (dekat `bars`, seperti `bars: crate::titlebar::Bars`).
  - `hyprix/src/state/draw.rs` — ketika `self.overview.style == Style::Mission`: `crate::overview::outputs` mengembalikan ruang + kartu, label ruang di-place sebagai `Placed` dengan WindowId palsu, dan peta `decorations` digabung dengan peta label (`bars.rendered()` + peta label) supaya `frame.rs` (`scene.decorations`) menggambarnya seperti title bar. Tanpa label, kartu tetap digambar (label adalah bagian yang boleh gagal).
- [ ] Cara menjalankan: `cargo test spaces`, `cargo test overview`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
- [ ] Commit (tanpa trailer): `feat(hyprix): the mission control draws its spaces`

## Task 3: interaksi — klik kartu, batal, dispatcher

- [ ] Tes gagal lebih dulu (baru, di `hyprix/src/overview.rs`, `mod tests`):
  - `a_click_on_a_card_goes_to_its_workspace_and_focuses_it` — klik kartu jendela di workspace 2: `state.active_workspace` jadi 2 dan `state.focused_window()` jadi jendela itu.
  - `a_click_on_a_minimized_card_brings_it_back` — kartu yang diminimalkan kembali (`!state.is_minimized(window)`).
  - `the_mission_dispatcher_turns_the_mission_style_on` — `overview::mission_dispatch("toggle", &mut overview, &mut state)` memberi `overview.on == true` dan `overview.style == Style::Mission`; `"off"` menurunkannya.
- [ ] Implementasi:
  - `hyprix/src/overview.rs` — `pub fn mission_dispatch(argument: &str, overview: &mut Overview, state: &mut State) -> bool`: `toggle|on|off`, selalu menetapkan `overview.style = Style::Mission` saat menyalakan. Pada `take_input`, gaya Mission: klik kartu → `state.dispatch_str("workspace", &workspace.0.to_string())` lalu `state.focus_window(window)` lalu tutup; klik di luar dan `Escape` menutup tanpa mengubah apa pun. `Overview::style` ikut dicek di `dimmed`/`step` agar kartu yang diminimalkan tetap redup dan `Tab`/`Return` tetap memilih.
  - `hyprix/src/act.rs` — di `compositor(...)`, cabang baru: `"mission" => Some(crate::overview::mission_dispatch(argument, around.overview, state))`, tepat di sebelah cabang `"overview"`.
- [ ] Cara menjalankan: `cargo test overview`, `cargo test act` (mod `tests` act.rs), `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
- [ ] Commit (tanpa trailer): `feat(hyprix): a click on a card brings that window up`

## Task 4: pintasan

- [ ] Tes gagal lebih dulu (baru, di `hyprix/tests/theme.rs`, bentuk mengikuti `xaqu_keys_are_the_menu_bars_shortcuts` yang sudah ada):
  - `the_mission_control_has_its_own_key` — di `xaqu-keys.conf`: `SUPER` + `Tab` → dispatcher `mission`; `SUPER` + `K` → `switcher` argumen `next`; `SUPER SHIFT` + `K` → `switcher` argumen `prev`; `SUPER` + `O` → `overview`; tidak ada `bindr` dengan kunci `Super_L`; tidak ada `SUPER` + `Tab` ke `switcher` lagi; tidak ada bind yang memakai ALT atau CTRL.
  - berkas tema itu harus tetap parse tanpa diagnostik, dan nama dispatcher `mission` harus ada di allow-list tes (`["exec", "mouse", "overview", "switcher", "suspend", "resume"]` → tambah `"mission"`).
- [ ] Implementasi (`hyprix/themes/xaqu-keys.conf`):
  - `bind = SUPER, Tab, switcher, next` → `bind = SUPER, K, switcher, next`
  - `bind = SUPER SHIFT, Tab, switcher, prev` → `bind = SUPER SHIFT, K, switcher, prev`
  - `bindr = SUPER, Super_L, overview, toggle` → `bind = SUPER, O, overview, toggle`
  - tambah `bind = SUPER, Tab, mission, toggle`
  - komentar berkas ditulis ulang: Mission Control di SUPER Tab, SUPER O membuka overview bawaan, SUPER K berpindah jendela.
  - `ferrix-light.conf` **tidak** disentuh: tema polos bukan Xaqu dan punya ikatan sendiri.
- [ ] Cara menjalankan: dari crate hyprix, `cargo test --test theme`; lalu `cargo fmt --check` dan `cargo clippy --all-targets -- -D warnings`.
- [ ] Commit (tanpa trailer): `docs: the mission control's key, and the keys it moved`

## Task 5: boot `mission`

- [ ] Tes gagal lebih dulu (baru, di `tools/common/xtask/src/compositor/mission.rs`):
  - `the_size_line_says_how_many_spaces_it_shows` — `said_size("hyprix: the mission control is up, 1024x768, 2 spaces")` memberi `(1024, 768)`.
  - `the_mission_control_leaves_the_wallpaper_to_be_seen` — gambar sintetis dengan wallpaper dominan dan dua kartu kecil di baris atas lolos `judge`, sedangkan layar desktop (jendela besar) tidak.
- [ ] Implementasi:
  - `tools/common/xtask/src/compositor/mission.rs` (baru) — `fn test_mission(arch: Arch, programs: &Programs, args: &Args) -> Result<()>`: desktop Xaqu lewat `super::xaqu::desktop(arch, args, false)?`; skrip tamu menyalakan dua jendela (mis. `/bin/shelf` di workspace 1, lalu `hyprctl dispatch workspace 2` dan `/bin/shelf` lagi), kembali ke workspace 1, lalu `hyprctl dispatch mission toggle`; tuan rumah menunggu baris `the mission control is up`, mengambil layar dengan `Qmp::screendump`, menilai `judge`, lalu menekan `esc` lewat `press(&mut qmp, &["esc"])` dan memastikan layarnya kembali ke desktop (kontrol negatif kedua).
  - `fn judge(screen: &Image) -> Result<Judged>` — **angka ambang diukur lebih dulu**: hitung dari satu run nyata berapa persen piksel wallpaper yang tersisa di layar Mission Control (kartu kecil, wallpaper dominan) dan berapa pada desktop biasa (jendela besar, wallpaper sedikit); tulis angka terukur itu sebagai ambang dengan margin 10 persen, dan catat angka terukurnya di komentar. Jangan menebak ambang.
  - `tools/common/xtask/src/compositor.rs` — `mod mission;`, dan `("mission", mission::test_mission)` di tabel `BOOTS` (panjang array 76 → 77), dekat `("spotlight", ...)`.
  - Compositor tidak mencetak apa-apa saat overview ditutup; karena itu "Mission Control tertutup" dinilai dari layar, bukan dari baris log.
- [ ] Cara menjalankan: dari `/root/ferrix-c`, `export CARGO_TARGET_DIR=$HOME/ferrix/target`, `cargo xtask test-compositor --boot mission --arch x86_64` (build panjang: dijalankan terlepas dengan `setsid nohup`, log dan berkas `done`; tanpa `sleep` di depan).
- [ ] Commit (tanpa trailer): `xtask: the mission control's boot`

## Task 6: dokumen

- [ ] `docs/DESKTOP.md` — bagian baru `### 6.10 Mission Control` (setelah §6.9): Mission Control adalah overview hyprix yang digambar ulang (`mission toggle`), ruang = workspace dengan label "Desktop N", kartu = permukaan jendela yang dikecilkan (hidup), klik kartu pindah workspace dan memfokus, Escape batal; pintasan SUPER Tab (Mission Control), SUPER O (overview bawaan), SUPER K / SUPER SHIFT K (switcher), SUPER polos dilepas dari overview; SUPER Up tetap `snapwindow max`; boot `mission`; catatan bahwa terang/gelap shell tidak mengubah compositor.
- [ ] `docs/superpowers/plans/2026-11-11-peta-rencana.md` — baris A4c: isi, status, dan bergantung pada (A4b).
- [ ] Commit (tanpa trailer): `docs: the mission control`

## Yang belum atau tidak diverifikasi

- **Sudut kartu**: permukaan klien digambar persegi (compositor tidak memotong sudut permukaan klien), jadi kartunya bersudut siku; jalur upgrade: `compositor_render::canvas::fill_rounded`.
- **Multi-monitor**: Mission Control digambar pada monitor yang fokus saja (pola `switcher::outputs`); monitor lain tetap seperti biasa. Pemisahan ruang per monitor belum dirancang, dan boot memakai satu monitor.
- **Animasi**: transisi masuk/keluar ruang belum ada; layar langsung berganti.
- **Terang/gelap**: kartu mengikuti tema compositor (terang, `xaqu.conf`); tema terang/gelap shell (A4b, berkas `appearance`) tidak mengubah compositor, jadi tidak ikut.
- **Hemat (A5)**: jendela yang *tidur* masih punya kartu dan isinya basi; belum ada boot yang mengukurnya.
- **Angka ambang `judge`**: belum diukur — harus diambil dari run nyata, bukan diperkirakan.
- **Label ruang**: belum ada uji piksel untuk teksnya; hanya uji unit bahwa label digambar.
- **Anggaran RSS** (§12) tidak diukur ulang; Mission Control tidak menambah proses, tetapi menggambar lebih banyak permukaan per frame.
- **Linter**: crate `hyprix` dan `xtask` memakai linter workspace yang ketat; `cargo clippy --all-targets -- -D warnings` dan `cargo fmt --check` wajib, dan kode baru harus diberi catatan `// Ponytail:` kalau menyederhanakan.

## Catatan anggaran

A4c menambah sekitar 300 baris di repo `ferrix` saja (`overview.rs` ±120, `spaces.rs` baru ±130, `act.rs` ±10, `state/draw.rs` ±20, kunci 8 baris, tes tema ±40, xtask ±200, dokumen ±40). Repo `apps` nol baris. Yang paling berisiko adalah Task 2 (permukaan palsu seperti title bar): kalau tidak sempat, label ruang boleh ditunda dan kartu tetap digambar — Mission Control tetap berfungsi, hanya tanpa nama ruang.
