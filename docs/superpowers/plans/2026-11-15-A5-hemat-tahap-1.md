# A5: Hemat tahap 1 — satu cgroup per app, keadaan Aktif/Latar/Tidur, dan pemberitahuan memori — Rencana Implementasi

> **Untuk pelaksana:** kerjakan task demi task, berurutan. Tiap task dimulai dengan tes yang gagal, lalu kode, lalu tes lulus, lalu commit. Izin commit dan eksekusi dilimpahkan koordinator untuk rencana ini. Repo yang disentuh: `ferrix` (hyprix, crate `compositor-config`, crate `compositor-ipc`, `xtask`, docs) dan **hanya di dua task terakhir** repo `apps` (`xaqu-shell`). Tidak ada push, tidak ada PR, tidak ada ISO.

**Goal:** Hemat tahap 1 membuat desktop hemat RAM seperti App Nap: tiap aplikasi Wayland punya cgroup-nya sendiri, keadaannya **Aktif**, **Latar**, atau **Tidur** according jendela yang terlihat; aplikasi yang Latar lebih dari 5 menit menjadi **Tidur** dan memorinya dilepas (`memory.reclaim`), aplikasi Latar diberi `cpu.weight` 10 dan yang Aktif 100; ada daftar pengecualian; dan ketika memori nearly habis desktop memberi tahu lewat Notices "memori aplikasi habis" dengan daftar aplikasi dan pemakaiannya. Tidak ada pembekuan (`cgroup.freeze`) di tahap ini — itu A6.

**Arsitektur:**

1. **Hemat hidup di compositor, bukan di shell.** Compositor adalah satu-satunya yang tahu pid tiap klien, kelasnya, dan jendela mana yang terlihat; ia juga sudah menulis berkas cgroup (`hyprix/src/suspend.rs`) dan sudah memulai program (`hyprix/src/state.rs::start`). Module baru: `hyprix/src/hemat.rs`, disimpan di `State` seperti `overview` dan `bars`.
2. **Pohon cgroup Hemat dibuat sendiri, bukan di bawah cgroup sesi.** Spec §6.1 menulis `<cgroup sesi>/app-<nama>-<n>`, tetapi itu **tidak bisa jalan** di Ferrix sekarang, dan alasannya dibaca di kernel, bukan diperkirakan:
   - aturan tanpa proses internal (`src/lib/fs/common/cgroupfs/src/controllers.rs::vet_enable`, `Standing::has_tasks`) menolak `+memory`/`+cpu` pada cgroup yang **memegang proses sendiri** atau yang punya anak berisi proses; cgroup sesiDesktop memegang compositor, jadi tidak bisa;
   - aturan delegasi (`src/kernel/src/fs/cgroupfs/delegation_check.rs`) menolak memindahkan proses antar pohon: "the common ancestor of the two is root's".
   Maka Hemat memakai pohonnya sendiri: `/sys/fs/cgroup/hemat/app-<nama>-<n>`, dengan `hemat` sebagai satu-satunya "cgroup sesi" yang tidak memegang proses. Deviasi dari spec §6.1 dicatat di "Keputusan" 2 dan di `docs/DESKTOP.md`.
3. **`cgroup2` belum ter-mount di boot desktop.** Yang me-mount-nya adalah init (`src/user/system/linux/init/init/src/cgroup.rs::Groups::mount`, `MOUNT = "/sys/fs/cgroup"`), sedangkan boot desktop menjalankan hyprix sebagai init kernel (`docs/AUTH.md` §1.2 baris "The desktop"), jadi tidak ada init yang mount. Hemat karena itu mount sendiri: satu syscall `mount("cgroup2", …)` di awal boot compositor, lalu `+cpu +memory +pids` di `cgroup.subtree_control` akar (akar dikecualikan aturan internal proses), lalu `hemat/`, lalu `+cpu +memory` di `hemat/cgroup.subtree_control` (cgroup kosong, jadi aturan tidak berlaku).
4. **Satu cgroup per aplikasi, dibuat saat jendela app pertama muncul.** Compositor tahu pid klien dari `slot.pid()` (lihat `hyprix/src/state.rs::as_seen`, yang mengembalikan `crate::select::Seen { class, pid, workspace, … }`). Aplikasi dikelompokkan **per kelas Wayland** (`xdg_toplevel.set_app_id`), bukan per jendela: dua jendela satu aplikasi berbagi satu cgroup dan satu keadaan. Nama cgroup: `app-<kelas>-<nomor urut>` (huruf selain `A-Za-z0-9:_.-` menjadi `_`, seperti `hyprix/src/scope.rs::name`).
5. **Keadaan app dihitung dari jendela.** **Aktif**: ada jendelanya di workspace aktif monitornya dan tidak tertutup penuh; **Latar**: semua jendelanya diminimalkan, tersembunyi, atau di workspace lain; **Tidur**: Latar selama lebih dari 5 menit. Sumber: `compositor_layout::State::listing()` → `Vec<Listed>`, `minimized_windows()`, `active_workspace(monitor)`, `focused_window()`, `workspace_of(window)` (dipakai di `hyprix/src/state.rs::as_seen`).
6. **Yang ditulis ke cgroup app.** Aktif: `cpu.weight` = 100, `memory.high` = `max`. Latar: `cpu.weight` = 10. Tidur: yang sama seperti Latar **ditambah** `memory.reclaim` (tulis ukuran, misal `1G`, ke `memory.reclaim`) — berkas tulis-saja seperti Linux (`src/lib/fs/common/cgroupfs/src/files.rs::Kind::MemoryReclaim`). Saat kembali Aktif: `memory.high` = `max` lagi dan `memory.reclaim` tidak diulang.
7. **Daftar pengecualian** dibaca dari konfigurasi compositor: `general:hemat_never = steam, …` (kelas yang dipisah koma), dan `general:hemat = 0|1` untuk mematikan Hemat seluruhnya. Kunci baru harus ditambah ke tabel opsi (`src/user/system/linux/compositor/config/src/options.rs`) karena parser konfigurasi hanya mengenal kunci yang terdaftar; dibaca lewat `Config::option("general:hemat_never")` (`src/user/system/linux/compositor/config/src/lib.rs:122`). Aplikasi di daftar pengecualian tidak pernah Tidur.
8. **Suara tidak diketahui compositor.** Hyprix tidak punya audio sama sekali (tidak ada satu pun berkas hiprix yang menyebut pulse atau audio); yang bicara dengan `pulsed` hanya shell (`xaqu-shell/src/sound.rs`, crate `pulseaudio`). Maka shell yang memberitahu compositor: shell membaca daftar sink-input dari `pulsed` tiap dua detik dan mengirim `dispatch hemat sound <nama>` / `dispatch hemat quiet <nama>` lewat IPC yang sudah ada (`.socket.sock`, seperti `hyprctl dispatch`). Dispatcher baru `"hemat"` ada di `hyprix/src/act.rs` (dekat cabang `"exec"`), bukan di `compositor_layout::Dispatcher`.
9. **Tekanan memori dilihat compositor, pemberitahuan lewat Notices.** Compositor membaca `/proc/meminfo` (`MemTotal`, `MemAvailable`, `SwapTotal`, `SwapFree`) tiap detik. Di bawah 10% selama 10 detik → app Latar dan Tidur di-reclaim lebih dulu, dari yang paling lama tidak dilihat. Masih di bawah 5% → desktop memberi tahu. Pemberitahuan **tidak** lewat notifyd: socket notifyd adalah baca-saja (`src/user/system/linux/compositor/notifyd/src/main.rs::serve_socket`, `bridge.rs` "the bridge's reading half"), dan antarmuka D-Bus-nya adalah sisi server; compositor tidak punya klien D-Bus. Jadi jalurnya dibalik: compositor menjawab permintaan IPC baru `hemat` (JSON: `{"pressure": true, "apps": [{"name": "foot", "bytes": 123456}]}`), dan shell yang memolesnya tiap dua detik lalu memunculkan banner "memori aplikasi habis" di permukaan Notices yang sudah ada (A4a). Hemat tidak pernah menutup app sendiri.
10. **Jam.** Pemeriksaan 5 menit dan tekanan memori butuh tick. Tick dipanggil dari loop utama compositor (dari `hyprix/src/wait.rs` atau dari loop frame), satu kali per detik; kalau loop compositor hanya bangun saat ada input, waktu tunggu poll-nya harus dibuat maksimum satu detik — ini harus diperiksa dan, perlu, diubah pada Task 2.

**Tech stack:** Rust 2024; repo `ferrix` (`hyprix`, `compositor-config`, `compositor-ipc`, `xtask`, docs) dan repo `apps` (`xaqu-shell`) pada dua task terakhir. Kode baru hanya Rust; tidak ada crate baru; tidak ada dependency baru (shell sudah memakai `pulseaudio`).

**Spec:** `docs/superpowers/specs/2026-11-11-desktop-gaya-macos-design.md` §6 (Hemat butir 1–4 dan tahap 2 di akhir), §3 (baris "Hemat" dan tabel unit), §7 (tabel penanganan error: "tulis ke cgroup gagal → app dibiarkan Aktif, dicatat ke log sekali"), §11 (uji: unit + boot `multiapp`), §12.3 dan §12.4 (uji banyak aplikasi dan uji tekanan memori), §14 (repositori yang disentuh). Sidebar: `docs/CGROUPS.md` §3 (berkas cgroup), §3.1 (delegasi), §7.1 (M1/M2/S1 yang sudah ada), `docs/APP-SUSPEND.md` §3.1 (cgroup di bawah scope sesi).

**Gaya kode:** "ponytail". Penyederhanaan ditandai `// Ponytail: ... Upgrade path: ...`.

---

## Aturan eksekusi

- Semua perintah di WSL Arch; repo ferrix `/root/ferrix` (hanya dibaca).
- Worktree baru `/root/ferrix-d` dari tip branch desktop yang sudah berisi A4c (`desktop/a4c-mission-control` di `/root/ferrix-c`; kalau A4c belum mendarat, dari tip `desktop/a4a-spotlight`): `git worktree add -b desktop/a5-hemat /root/ferrix-d <tip>`. Worktree A4a/A4b/A4c tidak disentuh.
- `export CARGO_TARGET_DIR=$HOME/ferrix/target` — **satu target bersama**; jangan target kedua (disk C: tinggal ~2 GB).
- Branch `desktop/a5-hemat`. Tidak ada push, tidak ada PR, tidak ada ISO, tidak ada `cargo xtask check` penuh.
- Commit ferrix **tanpa** trailer `Co-Authored-By`: hook `.githooks/commit-msg` menolak trailer itu (dan menolak penulis bermuatan "claude"/"anthropic"); hook tidak dilewati. Commit di repo `apps` mengikuti aturan repo itu sendiri.
- Hanya uji crate yang disentuh dan boot milik rencana ini (TCG). Build dan boot panjang dijalankan terlepas (`setsid nohup ... &`, log + berkas `done`), tanpa `sleep` di depan; satu QEMU pada satu waktu.
- Tidak ada kernel change di rencana ini: semua yang dipakai sudah ada (`memory.high`, `memory.reclaim`, `memory.current`, `cpu.weight`, `cgroup.procs`, `cgroup.subtree_control`).

## Keputusan

1. [KEPUTUSAN] **Hemat di compositor (`hyprix/src/hemat.rs`), bukan di `xaqu-shell` seperti spec §14 menulis.** Alasannya: compositor memegang pid dan kelas tiap klien (`hyprix/src/state.rs::as_seen` → `crate::select::Seen`), holding keadaan jendela (tata letak), dan sudah menulis berkas cgroup (`hyprix/src/suspend.rs::write`). Shell harus menebak keadaan jendela lewat `j/clients` dan tidak punya jalur menulis cgroup. Konsekuensi: repo `apps` hanya tersentuh untuk dua hal yang memang tidak bisa/kernel-side: pembacaan suara (pulsed) dan banner Notices.
2. [KEPUTUSAN] **Pohon cgroup Hemat sendiri: `/sys/fs/cgroup/hemat/app-<nama>-<n>`, bukan `<cgroup sesi>/…` seperti spec §6.1.** Alasannya dua aturan kernel yang sudah dibaca sumbernya: `ferrix_cgroupfs::controllers::vet_enable` menolak `+memory`/`+cpu` pada cgroup yang memegang proses (cgroup sesiDesktop memegang compositor), dan `delegation_check` menolak memindahkan proses antar pohon. Pohon sendiri juga membuat "cgroup sesi" tidak perlu berubah ⇒ A5 tidak bergantung pada "delegasi cgroup sesi" yang tertulis di peta rencana. Deviasi dari spec §6.1 dicatat di `docs/DESKTOP.md`.
3. [KEPUTUSAN] **`cgroup2` di-mount sendiri oleh compositor**, bukan menunggu init. Alasannya: di boot desktop tidak ada init (`docs/AUTH.md`: "hyprix is linked into the kernel as its init"), dan hemat butuh pohon cgroup ada. Mount gagal (bukan root, atau `cgroup2` sudah ada) tidak boleh mematikan desktop: Hemat lalu tidak aktif dan compositor tetap jalan.
4. [KEPUTUSAN] **Satu cgroup per kelas aplikasi, bukan per jendela.** Alasannya: spec §6.2 memakai "app" dan "app yang tidak memutar suara";App Nap juga per app. Menu dan dock mengetik app per kelas juga (`xaqu-shell/src/model.rs::Launcher::is`). Jendela-jendela satu app berbagi cgroup ⇒ satu `memory.reclaim` cukup.
5. [KEPUTUSAN] **Tidur hanya menulis `memory.reclaim`; tidak ada `cgroup.freeze`.** Alasannya: `cgroup.freeze` adalah tahap 2 (spec §6 akhir, peta rencana baris A6) dan membekukan klien yang perlu diperlakukan sebagai tidur (`docs/APP-SUSPEND.md`) — pekerjaan APP-SUSPEND S1, di luar rencana ini.
6. [KEPUTUSAN] **Suara lewat shell ke compositor, lewat dispatcher baru `hemat`.** Alasannya: compositor tidak punya audio; `pulsed` adalah PulseAudio server dan shell sudah bicara dengannya (`xaqu-shell/src/sound.rs`). Tidak ada cara lain yang lebih kecil. Catatan: nama yang dikirim adalah nama binari dari proplist sink-input, dicocokkan longgar dengan kelas (huruf besar-kecil diabaikan, bagian terakhir setelah titik) — dicatat sebagai tidak presisi.
7. [KEPUTUSAN] **Pemberitahuan memori lewat permintaan IPC baru `hemat` yang dipoles shell, bukan lewat notifyd.** Alasannya: notifyd hanya memberi socket baca-saja (`notifyd/src/main.rs::serve_socket`) dan sisi server D-Bus; compositor tidak punya klien D-Bus, dan menambah klien D-Bus lebih besar daripada menambah satu perintah IPC yang sudah ada (`compositor-ipc` sudah menjawab `j/clients`, `j/workspaces`, dan lain-lain).
8. [KEPUTUSAN] **Hemat boleh dimatikan lewat `general:hemat = 0`.** Alasannya: fitur ini menyentuh setiap app yang dibuka; kalau salah, seluruh desktop pelan. Sakelar ini membuat recover-nya satu baris konfigurasi, dan tes dapat memakainya.
9. [KEPUTUSAN] **Kegagalan menulis ke cgroup tidak boleh menggagalkan apa pun.** Alasannya: spec §7 "tulis ke cgroup gagal → app dibiarkan Aktif, dicatat ke log sekali". Semua tulisan Hemat menelan kesalahan, dan kesalahannya dicatat satu kali per app.

## Peta berkas dan status

| Task | Repo | Berkas | Status |
|---|---|---|---|
| 1 | ferrix | `src/user/system/linux/compositor/hyprix/src/hemat.rs` (baru): mount, pohon, penulisan controller | belum |
| 2 | ferrix | `hyprix/src/hemat.rs` (cgroup per app), `hyprix/src/state.rs` (field `hemat`, tick) | belum |
| 3 | ferrix | `hyprix/src/hemat.rs` (keadaan Aktif/Latar/Tidur, jam 5 menit, reclaim), `hyprix/src/state.rs` | belum |
| 4 | ferrix | `src/user/system/linux/compositor/config/src/options.rs` (`general:hemat`, `general:hemat_never`), `hyprix/src/hemat.rs` | belum |
| 5 | ferrix + apps | `hyprix/src/act.rs` (dispatcher `hemat`), `xaqu-shell/src/sound.rs` (daftar sink-input), `xaqu-shell/src/hypr.rs` | belum |
| 6 | ferrix + apps | `src/user/system/linux/compositor/ipc/src/reply.rs` (perintah `hemat`), `hyprix/src/state.rs`, `xaqu-shell/src/notices.rs` (banner) | belum |
| 7 | ferrix | `tools/common/xtask/src/compositor/multiapp.rs` (baru), `tools/common/xtask/src/compositor.rs`, `docs/DESKTOP.md`, `docs/superpowers/plans/2026-11-11-peta-rencana.md` | belum |

## Task 1: pohon cgroup Hemat

- [ ] Tes gagal lebih dulu (baru, di `hyprix/src/hemat.rs`, `mod tests`; bentuknya mengikuti tes-tes murni yang sudah ada di `hyprix/src/suspend.rs`):
  - `the_hemat_tree_is_a_slice_of_its_own` — `hemat::tree()` memberi `/sys/fs/cgroup/hemat`, dan `hemat::app_dir("foot", 3)` memberi `/sys/fs/cgroup/hemat/app-foot-3`.
  - `a_class_name_is_made_safe_for_a_cgroup` — `hemat::safe("org.example.Notes")` = `org.example.Notes`; `hemat::safe("my prog+x")` = `my_prog_x`; `hemat::safe("")` = `program` (aturan yang sama dengan `scope::name`).
  - `the_controllers_are_the_three_ferrix_has` — `hemat::CONTROLLERS` = `["+cpu", "+memory", "+pids"]`.
  - `a_write_that_fails_leaves_the_app_alone` — fungsi tulis menerima closure yang gagal dan mengembalikan `Err` berisi pesan, tanpa panik.
- [ ] Implementasi (di `hyprix/src/hemat.rs`, berkas baru):
  - `pub const CGROUP_ROOT: &str = "/sys/fs/cgroup";` (nilai yang sama dengan `hyprix/src/suspend.rs::CGROUP_ROOT`).
  - `pub const TREE: &str = "hemat";`, `pub const CONTROLLERS: [&str; 3] = ["+cpu", "+memory", "+pids"];`
  - `pub fn tree() -> PathBuf`, `pub fn app_dir(name: &str, number: u32) -> PathBuf`, `pub fn safe(class: &str) -> String`.
  - `pub fn prepare() -> Result<(), String>`: buat `/sys/fs/cgroup` bila belum ada; mount `cgroup2` (bila belum ter-mount) dengan `libc::mount` — hyprix sudah memakai `libc` (`hyprix/Cargo.toml`); tulis `CONTROLLERS` ke `cgroup.subtree_control` akar; mkdir `hemat`; tulis `["+cpu", "+memory"]` ke `hemat/cgroup.subtree_control`. Mount yang gagal (bukan root) **bukan** error fatal: kembalikan `Err` dan Hemat tidak aktif.
  - `pub fn write(dir: &Path, file: &str, value: &str) -> Result<(), String>` — satu `write` seperti `init/src/cgroup.rs::write` dan `suspend.rs::write`.
  - `pub fn make_app(name: &str, number: u32) -> Result<PathBuf, String>` — mkdir `hemat/app-<nama>-<n>` (yang sudah ada bukan kesalahan), lalu tulis `memory.high` = `max` dan `cpu.weight` = `100`.
  - `pub fn put_pid(dir: &Path, pid: u32) -> Result<(), String>` — tulis pid ke `cgroup.procs`.
- [ ] Cara menjalankan: dari `/root/ferrix-d/src/user/system/linux/compositor/hyprix`, `export CARGO_TARGET_DIR=$HOME/ferrix/target`, `cargo test hemat`, lalu `cargo fmt --check` dan `cargo clippy --all-targets -- -D warnings` untuk crate itu saja.
- [ ] Commit (tanpa trailer): `feat(hyprix): the hemat's cgroup tree and its per app groups`

## Task 2: cgroup per aplikasi

- [ ] Tes gagal lebih dulu (baru, di `hyprix/src/hemat.rs`):
  - `an_app_is_one_class_and_one_group` — dua jendela dengan kelas yang sama memakai satu cgroup; kelas berbeda memakai cgroup berbeda.
  - `the_app_of_a_window_is_the_class_it_named` — dari `crate::select::Seen { class, pid, … }` yang sudah ada, app adalah kelasnya; pid 0 (klien yang tidak procesnya diketahui) tidak membuat cgroup.
  - `the_first_window_of_an_app_makes_its_group` — app baru membuat `hemat/app-<kelas>-1`; app kedua dengan kelas yang sama memakai `-1` juga.
  - `a_window_that_went_away_takes_its_group_with_it` — saat app tidak punya jendela lagi, cgroupnya dihapus (isdir hening) dan app keluar dari daftar.
- [ ] Implementasi:
  - `hyprix/src/hemat.rs` — `pub struct App { pub name: String, pub class: String, pub number: u32, pub dir: PathBuf, pub pid: u32 }` dan `pub struct Hemat { pub apps: BTreeMap<String, App> }` dengan `Hemat::new()`, `Hemat::app_of(&mut self, class: &str, pid: u32) -> &App` (membuat cgroup bila belum ada), `Hemat::forget(&mut self, class: &str)`.
  - `hyprix/src/state.rs` — field baru `pub hemat: crate::hemat::Hemat` di `State`, dan panggilannya dari tempat jendela baru dipetakan (`as_seen`/`state.rs` sekitar baris 4955) serta dari places yang menghapus jendela. Cgroup hanya dibuat bila `general:hemat` bukan 0 dan pid bukan 0.
- [ ] Cara menjalankan: `cargo test hemat`, `cargo test state` (mod `tests` di `hyprix/src/state.rs`), `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
- [ ] Commit (tanpa trailer): `feat(hyprix): each app gets a cgroup of its own`

## Task 3: keadaan Aktif, Latar, dan Tidur

- [ ] Tes gagal lebih dulu (baru, di `hyprix/src/hemat.rs`):
  - `an_app_with_a_visible_window_is_active` — jendela di workspace aktif ⇒ `Aktif`.
  - `an_app_with_every_window_hidden_is_in_the_background` — semua jendela diminimalkan atau di workspace lain ⇒ `Latar`.
  - `a_background_app_falls_asleep_after_five_minutes` — dengan waktu yang disetel maju 5 menit 1 detik ⇒ `Tidur`; pada 4 menit 59 detik masih `Latar`.
  - `an_app_never_sleeps_while_it_is_heard` — app yang ditandai bersuara tidak menjadi `Tidur` walau sudah 5 menit Latar.
  - `a_sleeping_app_is_woken_when_it_is_shown_again` — kembali terlihat ⇒ `Aktif` dan `memory.high` = `max`.
  - `an_asleep_app_gives_its_clean_cache_back` — saat menjadi `Tidur`, `memory.reclaim` ditulis dengan ukuran yang diminta.
  - `the_background_app_is_quieter_than_the_active_one` — `cpu.weight` 10 pada `Latar`, 100 pada `Aktif`.
- [ ] Implementasi:
  - `hyprix/src/hemat.rs` — `pub enum State { Aktif, Latar, Tidur }`; `pub struct App { …, pub state: State, pub since_background: Option<Instant>, pub loud: bool }`; `pub const SLEEP_AFTER: Duration = Duration::from_secs(300);`
  - `pub fn refresh(&mut self, state: &compositor_layout::State, seen: &[crate::select::Seen], now: Instant)`: hitung keadaan tiap app dari `state.listing()`, `state.minimized_windows()`, `state.active_workspace(monitor)`, `state.workspace_of(window)`; lalu, saat keadaan berubah, tulis `cpu.weight` (100/10) dan, saat masuk `Tidur`, tulis `memory.reclaim` dengan `1G` (ukuran yang diminta; kitty Linux `memory.reclaim` menerima ukuran bytes atau `1G`).
  - `pub fn heard(&mut self, name: &str, loud: bool)` — untuk dispatcher Task 5.
  - `hyprix/src/state.rs` — tick: panggil `hemat.refresh(...)` dari loop utama. Waktu tunggu poll harus paling lama satu detik supaya tick jalan tanpa input; periksa `hyprix/src/wait.rs` dan ubah bila perlu (dengan tes yang menungguk tick-nya).
- [ ] Cara menjalankan: `cargo test hemat`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
- [ ] Commit (tanpa trailer): `feat(hyprix): the apps' states, active, background and asleep`

## Task 4: daftar pengecualian dan sakelar

- [ ] Tes gagal lebih dulu (baru, di `src/user/system/linux/compositor/config/src/options.rs` atau tes konfigurasi yang sudah ada di crate itu, dan di `hyprix/src/hemat.rs`):
  - `the_hemat_has_its_own_keys` — `general:hemat` ada di tabel opsi dengan bawaan 1, dan `general:hemat_never` ada sebagai `Str("")`.  - `an_exception_list_is_split_on_commas` — `"steam, wine"` ⇒ `["steam", "wine"]`; `""` ⇒ kosong; nama dipangkas spasi.
  - `an_excluded_app_never_sleeps` — app di daftar pengecualian tetap `Aktif`/`Latar`, tidak pernah `Tidur`, dan `cpu.weight`-nya 100.
  - `the_hemat_off_means_no_cgroup_is_made` — `general:hemat = 0` ⇒ tidak ada cgroup app yang dibuat.
- [ ] Implementasi:
  - `src/user/system/linux/compositor/config/src/options.rs` — tambahkan dua kunci ke tabel opsi: `("general:hemat", Int(1))` dan `("general:hemat_never", Str(""))`.
  - `hyprix/src/hemat.rs` — `pub fn exceptions(text: &str) -> Vec<String>`; pembacaan option memakai `compositor_config::Config::option("general:hemat_never")` (`config/src/lib.rs:122`), yangStr dikembalikan sebagai teks apa adanya.
  - `hyprix/src/state.rs` — konfigurasi dibaca sudah ada (`read_config`, `Config`); teruskan nilainya ke `Hemat` saat `State` dibuat.
  - `hyprix/themes/xaqu.conf` — tambahkan kunci baru dengan komentar satu baris (bukan berkas tema yang lain).
- [ ] Cara menjalankan: dari crate `compositor-config`: `cargo test`; dari crate `hyprix`: `cargo test hemat`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
- [ ] Commit (tanpa trailer): `feat(hyprix): the hemat's exception list and its switch`

## Task 5: suara

- [ ] Tes gagal lebih dulu:
  - di `hyprix/src/act.rs` (mod `tests` act.rs): `the_hemat_dispatcher_takes_a_sound` — `dispatch hemat sound foot` menandai app `foot` bersuara; `hemat quiet foot` meniadakannya; nama lain tidak berubah.
  - di `xaqu-shell/src/sound.rs` (apps repo): `the_playing_apps_are_named_by_their_binary` — dari daftar sink-input, nama binari (`application.process.binary`, atau `media.name` bila kosong) memberi "foot"; sink-input yang tidak ada tidak menghasilkan nama.
- [ ] Implementasi:
  - `hyprix/src/act.rs` — cabang baru `"hemat" => Some(crate::hemat::dispatch(argument, state))`, dekat `"exec"`. Dispatcher menerima `"sound <nama>"`, `"quiet <nama>"`, dan `"state"` (teks: keadaan tiap app — untuk diagnosa).
  - `hyprix/src/hemat.rs` — `pub fn dispatch(argument: &str, state: &mut State) -> bool`: pecah argumen jadi kata, panggil `state.hemat.heard(nama, loud)`.
  - `xaqu-shell/src/sound.rs` — tambah polling sink-input dari `pulsed` tiap dua detik memakai crate `pulseaudio` yang sudah dipakai shell (rev `3c0325f…` di `xaqu-shell/Cargo.toml`; **periksa** apakah crate itu menyediakan perintah daftar sink-input — `pulseaudio::protocol::SinkInputList` — dan bila tidak, tulis perintah minimum itu sendiri di `sound.rs`; jangan menambah dependency).
  - `xaqu-shell/src/hypr.rs` — kirim `dispatch hemat sound <nama>` / `dispatch hemat quiet <nama>` lewat `Hypr::dispatch` yang sudah ada, hanya saat daftarnya berubah.
  - Kalau task ini terlambat, **Jangan** biarkan app Tidur tanpa suara: tombol `general:hemat` dimatikan sampai task ini selesai (lihat "Catatan anggaran").
- [ ] Cara menjalankan: dari crate `hyprix`: `cargo test act`, `cargo test hemat`; dari repo apps, crate `xaqu-shell`: `cargo test sound`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
- [ ] Commit: ferrix `feat(hyprix): the sound the shell hears, the hemat is told`; apps `feat(xaqu-shell): the shell tells the compositor what it hears`.

## Task 6: tekanan memori dan pemberitahuannya

- [ ] Tes gagal lebih dulu:
  - di `hyprix/src/hemat.rs`: `the_available_memory_is_read_from_proc` — dari teks `/proc/meminfo` yang dipakai, `MemTotal` dan `MemAvailable` terbaca; `SwapTotal`/`SwapFree` yang tidak ada (belum ada swap di Ferrix) dibaca sebagai 0.
  - `the_pressure_below_a_tenth_holds_for_ten_seconds` — 9,9% selama 9 detik belum memicu; 10 detik memicu.
  - `the_critical_pressure_lists_the_apps_by_memory` — keadaan kritis memberi daftar app dan ukuran memorinya, yang terbesar lebih dulu.
  - di `src/user/system/linux/compositor/ipc/src/reply.rs` (tes ipc yang sudah ada): `the_hemat_is_answered_as_json` — permintaan `hemat` (tanpa `j/`) menjawab JSON dengan `pressure` dan `apps`.
  - di `xaqu-shell/src/notices.rs` (apps repo): `the_memory_notice_is_read_from_the_hemat_state` — JSON `{"pressure": true, "apps": [{"name": "foot", "bytes": 12345}]}` menjadi banner "Memori aplikasi habis" yang menyebut "foot" dan "--bytes--".
- [ ] Implementasi:
  - `hyprix/src/hemat.rs` — `pub struct Memory { pub total: u64, pub available: u64, pub swap_total: u64, pub swap_free: u64 }`, `pub fn meminfo() -> Option<Memory>` (baca `/proc/meminfo`); `pub fn under_pressure(memory: &Memory) -> bool` (tersedia < 10% selama 10 detik, keadaan disimpan di `Hemat`); `pub fn critical(memory: &Memory) -> bool` (tersedia < 5%); `pub fn apps_by_memory(&self) -> Vec<(String, u64)>` (dari `memory.current` tiap cgroup app).
  - Saat tekanan: reclaim app Latar dan Tidur, dari yang paling lama tidak dilihat (tulis `memory.reclaim`); saat kritis: tidak menutup app apa pun, hanya menyiapkan jawabannya.
  - `src/user/system/linux/compositor/ipc/src/reply.rs` — perintah baru `hemat` di tabel perintah (dekat `clients`/`workspaces`), menjawab JSON seperti `{"pressure": true, "apps": [{"name": "foot", "bytes": 12345}]}`; `bytes` = `memory.current` cgroup app.
  - `hyprix/src/state.rs` — isi jawaban itu dari `state.hemat`.
  - `xaqu-shell/src/notices.rs` — polling `hemat` tiap dua detik (shell sudah bertanya ke compositor lewat `Hypr::ask`), dan ketika `pressure` true memunculkan banner "Memori aplikasi habis" dengan daftar app dan memorinya di permukaan Notices yang sudah ada. Banner tidak interaktif: **memilih app yang ditutup belum ada di tahap 1** (lihat "Yang belum").
- [ ] Cara menjalankan: dari crate `compositor-ipc`: `cargo test`; dari crate `hyprix`: `cargo test hemat`; dari repo apps: `cargo test notices`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`.
- [ ] Commit: ferrix `feat(hyprix): the memory pressure and what it says`; apps `feat(xaqu-shell): the notices say the memory is short`.

## Task 7: boot `multiapp`, dokumen, peta

- [ ] Tes gagal lebih dulu (baru, di `tools/common/xtask/src/compositor/multiapp.rs`):
  - `the_many_apps_sleep_after_five_minutes` — boot membuka beberapa jendela, meninggalkannya di belakang, lalu menunggu lebih dari 5 menit: cgroup app ada, `memory.current` app yang Tidur lebih kecil dari saat baru diminimalkan, dan `cpu.weight` app Latar 10.
  - `the_amount_line_says_what_it_saw` — baris "the apps are asleep: 2" memberi dua app.
  - Kontrol negatif: app yang **fokus** (tidak Latar) tidak pernah Tidur, dan tidak ada cgroup yang dihapus sambil isinya hidup.
- [ ] Implementasi:
  - `tools/common/xtask/src/compositor/multiapp.rs` (baru) — `fn test_multiapp(arch: Arch, programs: &Programs, args: &Args) -> Result<()>`: skrip tamu membuka beberapa `/bin/shelf` (atau `foot`), minimized semuanya kecuali satu, menunggu lebih dari 5 menit, lalu membaca `/sys/fs/cgroup/hemat/*/memory.current` dan `cpu.weight`; tuan rumah menilai baris-baris yang tamu cetak.
  - `tools/common/xtask/src/compositor.rs` — `mod multiapp;` dan `("multiapp", multiapp::test_multiapp)` di tabel `BOOTS` (panjang array bertambah satu).
  - Boot `multiapp` butuh waktu lebih dari 5 menit ⇒ jalankan terlepas (`setsid nohup`), log dan berkas `done`; tidak ada `sleep` di depan.
  - `docs/DESKTOP.md` — bagian baru (mis. §6.11 "Hemat"): satu cgroup per app di `/sys/fs/cgroup/hemat/`, keadaan Aktif/Latar/Tidur, 5 menit, `cpu.weight` 100/10, `memory.reclaim` saat Tidur, daftar pengecualian `general:hemat_never`, sakelar `general:hemat`, dan **deviasi dari spec §6.1** (pohonnya sendiri, bukan `<cgroup sesi>`) dengan alasannya.
  - `docs/superpowers/plans/2026-11-11-peta-rencana.md` — baris A5: isi, status, dan bergantung pada.
- [ ] Cara menjalankan: dari `/root/ferrix-d`, `export CARGO_TARGET_DIR=$HOME/ferrix/target`, `cargo xtask test-compositor --boot multiapp --arch x86_64` (build panjang, dijalankan terlepas).
- [ ] Commit (tanpa trailer): `xtask: the many apps' boot` lalu `docs: the hemat`.

## Yang belum atau tidak diverifikasi

- **Kompresi dan swap (Proyek B) belum masuk desktop.** `memory.reclaim` di Ferrix hanya melepas page cache bersih (M2), dan `memory.swap.current`/`memory.swap.max` baru mendarat di branch `mm/b1-b5` (`/root/ferrix-b`), bukan di branch desktop. Maka di tahap 1: (a) hemat yang diperoleh app Tidur kecil (hanya page cache), (b) "swap penuh" tidak pernah terjadi ⇒ syarat dialog memori kritis disederhanakan jadi "MemAvailable < 5%", dan (c) `SwapTotal`/`SwapFree` dibaca 0 bila tidak ada.
- **"Halaman anonim dingin dikompres" (§6.2) belum ada.** reclaim anonim ada di B1-B5; tahap 1 tidak menerapkannya.
- **Angka "40% lebih kecil" (§12.3) tidak dijanjikan** di tahap 1; boot `multiapp` mengukur dan mencetak angka nyata, yang jadi acuan B3.
- **`cpu.weight` Ferrix bukan group entity seperti Linux.** Dokumentasi Ferrix (§7.1 "S1 done differently") menyatakan bobot diterapkan per task di scheduler-nya, bukan entity kelompok; rasio 10 terhadap 100 karena itu **belum diukur** dan boot `multiapp` yang mengukurnya.
- **Aturan "aplikasi yang bersuara tidak Tidur" hanya sekelas.** Nama binari dari proplist sink-input dicocokkan longgar dengan kelas aplikasi; app yang memutar suara lewat aplikasi lain (pembuat musik, browser) bisa tidak dikenali. Tidak ada uji yang membuktikan perilaku ini; yang ada hanya uji nama.
- **Tidak ada dialog interaktif "pilih app yang ditutup" (§6.3).** Yang ada banner yang daftar app dan memorinya; menutup app tetap lewat dock. Memilih app perlu permukaan dialog baru — di luar tahap 1.
- **Aplikasi yang dibangun dari terminal.** Di boot desktop semua proses ada di cgroup akar, jadi "app dari terminal tetap di cgroup terminal" (§6.1) tidak berlaku; yang berlaku: begitu jendela app muncul, pid-nya pindah ke cgroup app dan anak-anaknya mengikuti.
- **Boot session (sessiond) tidak bisa memakai Hemat.** Di sana compositor berjalan sebagai uid 1000 (`compositor/sessiond/src/main.rs`), tidak boleh mount `cgroup2` (`src/kernel/src/fs/mountperm_check.rs` menolak `cgroup2` bagi bukan root), dan tidak boleh menulis ke subtree milik root. Jalur: init yang melakukan pekerjaan cgroup lewat soket kontrol (`hyprix/src/scope.rs` sudah bertanya ke init), seperti yang dilakukan `Call::Scope`.
- **Boot `multiapp` belum diukur** — ambang `judge` diambil dari run nyata, bukan diperkirakan.
- **Anggaran RSS** (§12) tidak diukur ulang; Hemat menambah satu thread/tick dan beberapa berkas kecil, tetapi tidak menambah proses.

## Catatan anggaran

A5 menambah sekitar 700 baris di repo `ferrix` (`hemat.rs` baru ±380, `state.rs` ±60, `act.rs` ±15, `options.rs` ±2, `reply.rs` ±60, xtask ±180, dokumen ±60) dan sekitar 150 baris di repo `apps` (`sound.rs` ±70, `notices.rs` ±60, `hypr.rs` ±20). Yang paling berisiko adalah Task 5 (suara): kalau tidak sempat, **Hemat tidak boleh tidur tanpa suara** — matikan `general:hemat` sampai Task 5 mendarat, dan lanjutkan Task 6 dan 7 (tekanan memori dan boot) yang tidak bergantung padanya. Task 6 juga boleh ditunda seluruhnya: tekanan memori adalah cerita, sedangkan Task 1–4 sudah memberi hemat CPU dan page cache.
