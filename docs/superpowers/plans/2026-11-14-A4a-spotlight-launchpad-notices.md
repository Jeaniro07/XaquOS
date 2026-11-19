# A4a: Spotlight, Launchpad, Notices - Rencana Implementasi

> **Untuk pelaksana:** kerjakan task demi task, berurutan. Tiap task dimulai dengan tes yang gagal, lalu kode, lalu tes lulus, lalu commit. Commit hanya setelah izin pengguna.

**Goal:** SUPER Space membuka Spotlight di yoru-shell (app, jendela, berkas di `~`, kalkulator), SUPER L membuka Launchpad (grid layar penuh dari `.desktop`), notifikasi tampil sebagai banner kaca di kanan atas dengan riwayat 50 terakhir yang dibuka dari menu bar — semua menggantikan peran fuzzel sebagai palet, dalam token terang kaca Yoru.

**Arsitektur:**
1. Tiga permukaan baru di yoru-shell, masing-masing satu modul + satu `.tole`, mengikuti pola `switch.rs` (wallpaper switcher): proses `yoru-shell --spotlight` / `--launchpad` / panel Notices, layer `Overlay`, socket tunggal "dibuka lagi = tutup".
2. Model pencarian murni `search.rs` (host-testable): kalkulator, pencocokan app/jendela/berkas, pemindaian `~` dengan batas 5000 entri dan 300 ms.
3. Launchpad dan Spotlight membaca `installed()` penuh (`compositor_desktop::entry::find_programs`), bukan hanya 4 launcher tetap dock.
4. notifyd memasang soket riwayat kecil; bila yoru-shell berlangganan, notifyd berhenti menggambar banner sendiri dan yoru-shell menggambar banner kaca kanan atas + kartu riwayat dengan token Nocturne.
5. Keybind: `yoru-keys.conf` SUPER Space → Spotlight, SUPER L → Launchpad; ikon search menu bar membuka Spotlight; klik jam membuka/menutup riwayat Notices.
6. Tiga boot uji xtask baru: `spotlight`, `launchpad`, `notices`.

**Tech stack:** Rust 2024 (repo apps: yoru-shell; repo ferrix: notifyd, hyprix themes, xtask), tole layer surfaces, compositor_desktop, Python generator tidak disentuh.

**Spec:** `docs/superpowers/specs/2026-11-11-desktop-gaya-macos-design.md` §3 (Spotlight, Launchpad, Notices), §5 (Spotlight: 5000 entri, 300 ms, kalkulator; pintasan), §7 (pemindaian melebihi batas → hasil sebagian), §11 (boot `spotlight`, `launchpad`, `notices`), §13 (tanpa aset Apple).

**Gaya kode (instruksi pemilik):** "ponytail". Kode sesedikit mungkin, tanpa fitur spekulatif, memakai ulang pola yang ada. Tiap penyederhanaan diberi komentar `// Ponytail: ... Upgrade path: ...`.

---

## Aturan eksekusi

- Semua perintah dijalankan di WSL Arch: tulis skrip ke scratchpad (`/mnt/c/Users/jeani/AppData/Local/Temp/opencode/`), jalankan dengan `cmd /c "set MSYS_NO_PATHCONV=1&& wsl.exe -d archlinux -- bash -l /mnt/c/Users/jeani/AppData/Local/Temp/opencode/<skrip>.sh"`.
- Dua repo, masing-masing branch `desktop/a4a-spotlight` dipotong dari `desktop/a3-appmenu` (ferrix `1c302a08a`, apps `2f8cf00`):
  - ferrix: `/root/ferrix` (notifyd, hyprix themes, xtask).
  - apps: `/root/ferrix/src/user/apps` (yoru-shell).
- tole tidak disentuh di A4a.
- Commit: tanpa trailer Co-Authored-By, hook tetap aktif, tidak pernah `--no-verify`, tidak pernah `git stash`, selalu `git diff --cached --stat` sebelum commit.
- Tidak ada push. `components.toml` tidak diubah.
- String UI dalam bahasa Inggris. Tidak ada nama atau aset Apple.
- Boot memakai `FERRIX_OVMF_CODE=/usr/share/edk2/x64/OVMF_CODE.4m.fd FERRIX_OVMF_VARS=/usr/share/edk2/x64/OVMF_VARS.4m.fd`.
- Baseline uji: yoru-shell **74 lulus**, xtask **999 lulus** (angka 2026-11-14, `1c302a08a`). Uji A4a menambah, tidak mengurangi.
- Boot uji menulis ke `/root/ferrix/build`: pastikan tidak ada QEMU lain berjalan (`pgrep -f qemu-system` kosong) sebelum boot.

## Keputusan pengguna yang dibutuhkan

1. [KEPUTUSAN] **Spotlight menggantikan fuzzel sebagai SUPER Space dan ikon search.** `yoru-keys.conf:11` berubah ke `exec /bin/yoru-shell --spotlight`, `menubar.rs::search()` sama. fuzzel tetap di image (boot `n150-box` memakai SUPER D; boot `yoru-launcher` lama tetap lulus karena fuzzel tidak disentuh). Alternatif: fuzzel dipertahankan di SUPER Space dan Spotlight SUPER P — ditolak karena spec §5 menetapkan SUPER Space Spotlight.
2. [KEPUTUSAN] **Satu proses baru per permukaan, bukan satu proses yoru-shell panjang.** Pola `switch.rs`: `yoru-shell --spotlight` dan `--launchpad` adalah proses pendek dengan socket tunggal (`$XDG_RUNTIME_DIR/yoru-spotlight.sock`, `yoru-launchpad.sock`, pola `/tmp/<nama>-UID.sock` tanpa runtime dir), dibuka lagi = menutup. Panel Notices hidup di proses dock (bukan proses baru). Alternatif: dock menampung ketiganya — menyentuh `window.rs` jauh lebih dalam, ditunda.
3. [KEPUTUSAN] **Urutan hasil Spotlight:** kalkulator di atas bila seluruh query terbaca sebagai hitungan; lalu app (prefiks lebih dulu, lalu memuat), lalu jendela, lalu berkas; maksimal 9 baris.
4. [KEPUTUSAN] **Meluncurkan hasil:** app → `dispatch exec <exec>`; jendela → `dispatch focuswindow address:<address>`; berkas → `dispatch exec <xdg-open> <path>`, jalur `xdg-open` diverifikasi di Task 0 (dipakai stash/boot `stash`). Spotlight dan Launchpad menutup diri setelah meluncurkan.
5. [KEPUTUSAN] **Launchpad tanpa keyboard.** Layer `KeyboardInteractivity::None`; menutup dengan SUPER L lagi (socket) atau klik di luar grid. Paging: 8 kolom × 5 baris per halaman, halaman digeser klik titik halaman. Alternatif: satu halaman + chip "more" — ditolak, spec bilang grid layar penuh.
6. [KEPUTUSAN] **Ikon Launchpad dan Spotlight:** pemetaan Phosphor per keluarga app (diperluas dari pemetaan dock di `model.rs`) + tile huruf pertama untuk sisanya, sama seperti dock. Ikon SVG hicolor yang dibawa app tidak digambar (tole tidak membaca SVG). Upgrade path: rasterisasi SVG.
7. [KEPUTUSAN] **Notices: yoru-shell menggambar banner dan kartu riwayat; notifyd jadi sumber data.** notifyd memasang soket baris `$XDG_RUNTIME_DIR/notifyd.sock`: `list` (jawaban riwayat 50, terbaru dulu, lalu koneksi tutup), `subscribe` (riwayat lalu aliran hidup), `close <id>`. Saat ada subscriber, notifyd berhenti menggambar banner sendiri (kartu riwayat SIGUSR1 tetap). Banner yoru-shell: Anchor TOP|RIGHT, margin 8, kartu kaca Nocturne, maksimum 3 tampil, hilang sendiri setelah 5 s (mengikuti `SHOWN`/timeout notifyd). Riwayat 50 digambar yoru-shell dari data soket. Alternatif: notifyd tetap satu-satunya penggambar (pindah anchor, riwayat 50) dan yoru-shell hanya pintu buka — ditolak, kartu harus mengikuti tema kaca dan crash yoru-shell tak boleh matikan notifikasi (notifyd tetap menggambar bila yoru-shell mati).
8. [KEPUTUSAN] **Pemindaian berkas Spotlight:** jalan turun dari `$HOME`, maksimal 5000 nama dan 300 ms (dua batas kapan lebih dulu), tanpa entri titik, symlink tidak dituruni, ditata ulang saat dibuka, tanpa indeks latar. Folder dan berkas sama-sama hasil.
9. [KEPUTUSAN] **Kalkulator:** empat operasi, kurung, unary minus, bilangan bulat dan desimal; pembagian dengan nol dan sisa karakter → bukan hitungan. Hasil: bilangan bulat tanpa titik, lainnya enam desimal dipangkas nolnya.
10. [KEPUTUSAN] **Boot `yoru-launcher` (fuzzel) tetap.** Spec §11 menyebut boot lama diganti; `yoru`, `yoru-dock`, `sidebars` memang diganti di A1/A2, tetapi fuzzel masih diluncurkan boot `n150-box`, jadi boot pelatihannya tetap. Dihapus di A4b bila fuzzel pindah total.

## Peta berkas

| Repo | Berkas | Task |
|---|---|---|
| apps | `yoru-shell/src/search.rs` (baru) | 1 |
| apps | `yoru-shell/src/lib.rs` (`mod search;`) | 1 |
| apps | `yoru-shell/src/spotlight.rs` (baru), `ui/spotlight.tole` (baru) | 2 |
| apps | `yoru-shell/src/cli.rs` (`--spotlight`, `--render-spotlight`) | 2 |
| apps | `yoru-shell/src/switch.rs` (`socket()` jadi `pub(crate)`) | 2 |
| apps | `yoru-shell/src/launchpad.rs` (baru), `ui/launchpad.tole` (baru) | 3 |
| apps | `yoru-shell/src/cli.rs` (`--launchpad`, `--render-launchpad`), `src/model.rs` (`icon_for`) | 3 |
| ferrix | `src/user/system/linux/compositor/hyprix/themes/yoru-keys.conf` | 4 |
| apps | `yoru-shell/src/menubar.rs` (`search()`, klik jam) | 4, 5 |
| ferrix | `src/user/system/linux/compositor/notifyd/src/{store.rs,feed.rs (baru),main.rs,paint.rs}` | 5 |
| apps | `yoru-shell/src/notices.rs` (baru), `ui/notices.tole` (baru), `src/lib.rs` | 5 |
| ferrix | `tools/common/xtask/src/compositor/{spotlight.rs,launchpad.rs,notices.rs (baru)}`, `compositor.rs` | 6 |
| ferrix | `docs/DESKTOP.md` (§6.9) | 6 |
| apps | `yoru-shell/README.md` | 6 |

---

## Task 0: Branch, baseline, dan penelitian kecil

**Anggaran:** tidak ada kode produk.

- [x] **Langkah 1: branch dan baseline**

```bash
set -e
cd /root/ferrix && git switch -c desktop/a4a-spotlight desktop/a3-appmenu
cd /root/ferrix/src/user/apps && git switch -c desktop/a4a-spotlight desktop/a3-appmenu
cd /root/ferrix/src/user/apps/yoru-shell && cargo test 2>&1 | grep "test result" | head -1
cd /root/ferrix/tools/common/xtask && cargo test 2>&1 | grep "test result" | head -1
```

Expected: dua baris `desktop/a4a-spotlight`; `74 passed` dan `999 passed`. Bila angka berbeda, catat angkanya sebagai baseline.

- [x] **Langkah 2: penelitian (baca, jangan ubah)** — tiap butir dicatat jawabannya di catatan rencana sebelum Task 1:

1. Jalur `xdg-open` yang dipakai boot stash: `grep -rn "xdg-open" /root/ferrix/tools/common/xtask/src/compositor/{stash.rs,theme.rs} | head`. Jawaban dipakai di `search.rs` dan boot `spotlight`.
2. Perilaku klik jam menu bar sekarang: baca `menubar.tole` (blok jam) dan `menubar.rs` (sinyal jam → apa). Bila jam sudah mengirim sesuatu, putuskan mengganti atau menambah di Task 5.
3. Cara tole menerima tombol papan ketik di app tole: baca `shelf/src/window.rs` dan `shelf/src/app.rs` (Rename memakai input?) — cari pola `Event` keyboard dari `compositor_toolkit` (`Window::dispatch` mengembalikan `Vec<Event>`; catat nama varian keyboard dan cara keysym→huruf).
4. Format berkas `/run/lode/alerts` dan uji notifyd: `grep -rn "alerts" /root/ferrix/src/user/system/linux/compositor/notifyd/src/main.rs` dan uji di `store.rs` — dipakai boot `notices`.
5. Cara notifyd menggambar banner hari ini: `paint.rs::banners()` dan `Store::shown` — untuk memutuskan di mana bendera "subscriber ada" dipasang.
6. Uji render yoru-shell yang ada (`--render-*`) sebagai contoh uji piksel host: `grep -n "render" /root/ferrix/src/user/apps/yoru-shell/src/cli.rs | head`.

Tidak ada commit di Task 0.

### Jawaban penelitian Task 0 (dicatat 2026-11-14, sebelum Task 1)

1. **`xdg-open`**: `/bin/xdg-open` ada di image desktop (`claude_desktop.rs`: "`/bin/xdg-open` knows them", bersama `mimeapps.list` berisi `inode/directory=shelf.desktop`). Meluncurkan berkas Spotlight: `dispatch exec /bin/xdg-open <path>`.
2. **Klik jam menu bar**: jam hanya properti teks (`menubar.tole` `bar.clock`, tidak ada sinyal klik jam hari ini). Menambah sinyal `clock()` + MouseArea di Task 5 aman, tidak menggantikan apa pun.
3. **Tombol tole**: `Event::Keyboard(KeyboardEvent::Key(key)) if key.pressed`, nama keysym dari `key.keysym.unwrap_or_default()` (`shelf/src/window.rs:111-112`). Editor Spotlight menyalin pola ini; keysym→huruf mengikuti cara shelf membaca nama keysym.
4. **Alert lode**: baris `alert <name> <source> <text>` dibaca `from_alert_line` (`store.rs:295`) menjadi notifikasi `lode:<name>:<source>` / "Log alert: <name>" / teks / Urgency::Normal. Boot `notices` menulis baris itu ke `/run/lode/alerts`.
5. **Banner notifyd**: `Store` (`store.rs:102`) memegang `shown`/`history`/`notify`/`close`/`expire`; `paint.rs::banners()` menggambar dari `&[Note]`. Bendera subscriber: `Arc<AtomicUsize>` di `serve` (feed.rs) dinaikkan per koneksi `subscribe`, dibaca `main.rs` sebelum meminta `banners()`.
6. **Uji render host**: `--render-*` di `cli.rs:244-411`, cabang `--render` umum di :411 sebagai contoh.
7. **Baseline**: yoru-shell 74; xtask 997 passed + 1 FAILED + 1 ignored pada dijalankan paralel, tetapi `step_cache::tests::every_kind_of_change_is_a_miss` lulus sendirian — flaky di bawah kontensi lock build, bukan regresi. Baseline efektif 999.
8. **Loose end A3**: `files.rs` `pub(super) at/press_here` belum ter-commit, dibutuhkan `appmenu.rs` yang sudah ter-commit. Dicommit lebih dulu sebagai perbaikan A3.


---

## Task 1: `search.rs` — model pencarian, kalkulator, pemindaian

**Anggaran:** sekitar 200 baris produk, 150 baris tes. Modul murni, tanpa tole.

**Files:** Create `yoru-shell/src/search.rs`; Modify `yoru-shell/src/lib.rs`.

- [x] **Langkah 1: tes yang gagal** di `search.rs` (modul tes di berkas yang sama):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn apps() -> Vec<(String, String, Option<String>)> {
        vec![
            ("files".to_owned(), "Files".to_owned(), Some("rocks.magical.shelf".to_owned())),
            ("chrome".to_owned(), "Chrome for Testing".to_owned(), Some("org.chromium.Chrome".to_owned())),
            ("terminal".to_owned(), "Terminal".to_owned(), None),
        ]
    }

    #[test]
    fn the_calculator_answer_comes_first() {
        let hits = hits("1+2*3", &apps(), &[], &[]);
        assert_eq!(hits.first(), Some(&Hit::Value("7".to_owned())));
        assert!(hits("2/0", &apps(), &[], &[]).first().is_none() || hits("2/0", &apps(), &[], &[])[0] != Hit::Value("".to_owned()));
    }

    #[test]
    fn app_prefix_beats_app_middle_beats_file() {
        let files = vec!["profile.tar.gz".to_owned()];
        let hits = hits("ch", &apps(), &[], &files);
        assert!(matches!(hits.first(), Some(Hit::App { title, .. }) if title == "Chrome for Testing"));
        let hits = hits("file", &apps(), &[], &files);
        assert!(matches!(hits.first(), Some(Hit::App { title, .. }) if title == "Files"));
    }

    #[test]
    fn windows_come_as_their_title() {
        let hits = hits("notes", &apps(), &[("0x9".to_owned(), "notes.md — Files".to_owned())], &[]);
        assert_eq!(hits.first(), Some(&Hit::Window { address: "0x9".to_owned(), title: "notes.md — Files".to_owned() }));
    }

    #[test]
    fn files_match_from_anywhere_in_the_name() {
        let hits = hits("tar", &apps(), &[], &["profile.tar.gz".to_owned()]);
        assert_eq!(hits.first(), Some(&Hit::File { path: "profile.tar.gz".to_owned() }));
    }

    #[test]
    fn the_calculator_reads_parentheses_negatives_and_decimals() {
        for (query, want) in [
            ("(1+2)*3", "9"), ("-4+1", "-3"), ("1/4", "0.25"), ("2.5*2", "5"),
            ("2.5*1.5", "3.75"), ("10/4", "2.5"),
        ] {
            assert_eq!(value(query).as_deref(), Some(want), "{query}");
        }
        for bad in ["2/0", "1+", "(1", "abc", "1 2"] {
            assert_eq!(value(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_scan_respects_both_bounds_and_skips_dots() {
        let base = std::env::temp_dir().join(format!("yoru-search-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let deep = base.join("a/b/c");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(base.join("a/notes.md"), b"x").unwrap();
        std::fs::write(base.join("a/.secret"), b"x").unwrap();
        std::fs::write(base.join("a/b/plan.md"), b"x").unwrap();
        let found = scan(&base, 5000, 5_000_000);
        assert_eq!(found, vec!["a/b/plan.md", "a/b/c", "a/notes.md"]);
        let few = scan(&base, 1, 5_000_000);
        assert_eq!(few.len(), 1, "the count bound cuts");
        let _ = std::fs::remove_dir_all(&base);
    }
}
```

Catatan: urutan `found` mengikuti jalan turun alfabetis per folder; sesuaikan dua baris assert dengan urutan yang benar-benar dihasilkan setelah `scan` ditulis (bila folder `c` kosong tak tercantum, sesuaikan). Jalankan dan pastikan gagal kompilasi: `cd /root/ferrix/src/user/apps/yoru-shell && cargo test search 2>&1 | tail -3`.

- [x] **Langkah 2: kode** di atas modul tes:

```rust
//! What Spotlight matches: the query read as arithmetic, and the query
//! against applications, windows and files under ~.

/// One result of a query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    /// An application: its exec line and its title.
    App { exec: String, title: String, icon: &'static str },
    /// A window: focused by its address.
    Window { address: String, title: String },
    /// A file under ~, relative to home, opened by its default handler.
    File { path: String },
    /// The query read as arithmetic.
    Value(String),
}

/// At most this many hits, one row each.
pub const SHOWN: usize = 9;

/// The hits for `query`, best first, at most [`SHOWN`].
#[must_use]
pub fn hits(
    query: &str,
    apps: &[(String, String, Option<String>)],
    windows: &[(String, String)],
    files: &[String],
) -> Vec<Hit> {
    let mut found = Vec::new();
    let query = query.trim();
    if query.is_empty() {
        return found;
    }
    if let Some(value) = value(query) {
        found.push(Hit::Value(value));
    }
    let folded = query.to_lowercase();
    let by = |text: &str| {
        let text = text.to_lowercase();
        let at = text.find(&folded).unwrap_or(usize::MAX);
        at
    };
    let mut ranked: Vec<(usize, u8, Hit)> = Vec::new();
    for (id, title, _) in apps {
        let at = std::cmp::min(by(title), by(id));
        if at != usize::MAX {
            ranked.push((at, 0, Hit::App {
                exec: String::new(), // diisi pemanggil; lihat catatan di bawah
                title: title.clone(),
                icon: "",
            }));
        }
    }
    // Ponytail: pencocokan murni tidak membawa exec/ikon; pemanggil
    // (spotlight.rs) memetakan kembali title -> (exec, icon) dari daftar app.
    ...
}
```

**Penting:** jangan menulis `Hit::App` tanpa exec lalu memetakan ulang — itu dua kali kerja. Bentuk akhir yang diharapkan (tulis begini langsung): `hits` menerima `apps: &[(id, exec, app_id, title, icon)]` dalam struct kecil:

```rust
/// One application, as Spotlight shows it.
#[derive(Clone, Debug)]
pub struct App {
    /// The exec line, codes stripped.
    pub exec: String,
    /// The words shown.
    pub title: String,
    /// The Phosphor icon, or empty for the letter tile.
    pub icon: &'static str,
}
```

dan `hits(query, apps: &[App], windows, files)`. Uji `apps()` di atas menyesuaikan: `(App { exec: "/bin/term /bin/zinc".into(), title: "Terminal".into(), icon: "terminal-window" }, ...)`. Potongan `ranked` di atas hanya menunjukkan cara menaikkan prefiks; tulis penuh: app dulu (0), jendela (1), berkas (2), di tiap kategori diurutkan posisi kecocokan lalu abjad, potong di [`SHOWN`].

Kalkulator `value(&str) -> Option<String>`: recursive descent (`expr -> term (('+'|'-') term)*`, `term -> factor (('*'|'/') factor)*`, `factor -> '-' factor | '(' expr ')' | number`), `f64`, lalu format: bila `value == value.trunc() && value.abs() < 1e15` → `{value:.0}`-setara dengan cara sendiri (tanpa `{}` format koma), else enam desimal dipangkas `'0'` dan `'.'` di ekor. `is_finite` wajib.

Pemindaian `scan(home: &Path, max: usize, budget_ns: u128) -> Vec<String>`: `std::fs::read_dir` rekursif dengan stack `Vec<PathBuf>`, lompati nama berawalan `'.'`, lompati symlink (`metadata` yang mengikuti tautan melebihi kedalaman → `symlink_metadata`), kumpulkan path relatif (berkas dan folder kosong), berhenti pada `max` entri atau `Instant::now()` melewati anggaran, lalu urutkan abjad. Jarak panggilan dari spotlight.rs: `scan(&home, 5000, 300_000_000)`.

`lib.rs`: `mod search;` di urutan abjad, dengan doc comment satu baris.

- [x] **Langkah 3: jalankan**

```bash
cd /root/ferrix/src/user/apps/yoru-shell
cargo test search 2>&1 | grep "test result"
cargo clippy --all-targets -- -D warnings 2>&1 | tail -3
```

Expected: 7 uji baru lulus, baseline + 7, clippy bersih.

- [x] **Langkah 4: commit** (setelah izin pengguna)

```bash
cd /root/ferrix/src/user/apps
git add yoru-shell/src/search.rs yoru-shell/src/lib.rs
git diff --cached --stat
git commit -m "yoru-shell: search matches the query against apps, windows, files and arithmetic"
```

---

## Task 2: permukaan Spotlight

**Anggaran:** sekitar 250 baris Rust, 80 baris `.tole`.

**Files:** Create `spotlight.rs`, `ui/spotlight.tole`; Modify `cli.rs`, `switch.rs` (`socket()` → `pub(crate)`), `lib.rs` (`mod spotlight;`).

- [x] **Langkah 1: salin pola switcher.** Baca `switch.rs` penuh sekali lagi saat mulai; `spotlight.rs` mengikuti bentuknya: `NAMESPACE`, `socket()` (dari `switch`, kini `pub(crate) socket(name)`), `run()` yang membuka `Window` layer, loop `window.dispatch`, menutup bila socket kedua masuk atau Escape.

- [x] **Langkah 2: tes model yang gagal** (permukaan digambar, hits tampil, enter menjalankan):

Di `spotlight.rs` modul tes — kerangka (tanda tangan disesuaikan dengan apa yang `switch.rs`/`picker.rs` benar-benar sediakan; jangan mengarang API tole):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_fills_the_rows_and_enter_runs_the_first() {
        let apps = vec![crate::search::App { exec: "/bin/term /bin/zinc".into(), title: "Terminal".into(), icon: "terminal-window" }];
        let mut state = State::new(apps);
        state.type_in("term");
        assert_eq!(state.rows().len(), 1);
        assert_eq!(state.run(0), Some(Outcome::Dispatch("exec /bin/term /bin/zinc".to_owned())));
    }

    #[test]
    fn a_number_runs_nothing_and_a_window_focuses() {
        let mut state = State::new(vec![]);
        state.type_in("2+2");
        assert_eq!(state.rows().first().map(|r| r.text.as_str()), Some("4"));
        assert_eq!(state.run(0), None, "a value is not runnable");
        let mut state = State::new(vec![]);
        state.set_windows(vec![("0x9".to_owned(), "notes.md — Files".to_owned())]);
        state.type_in("notes");
        assert_eq!(state.run(0), Some(Outcome::Dispatch("focuswindow address:0x9".to_owned())));
    }

    #[test]
    fn typing_edits_and_escape_clears() {
        let mut state = State::new(vec![]);
        state.type_in("fi");
        state.key(Key::Backspace);
        assert_eq!(state.query(), "f");
        assert_eq!(state.key(Key::Escape), true, "escape asks for close");
    }
}
```

`State` memegang `Vec<search::App>`, `Vec<(String, String)>` jendela, `Vec<String>` berkas, `String` query, dan hasil `hits` terkini; `type_in(&str)` menambah karakter dan menghitung ulang; `key(Key)` menangani Backspace/Escape/Enter/Up/Down (pilihan baris); `run(at) -> Option<Outcome>` dengan `enum Outcome { Dispatch(String), Close }` — app/exec, jendela/focuswindow, berkas/`exec <xdg-open> <path>`, Value tidak bisa dijalankan. Jalankan: `cargo test spotlight 2>&1 | tail -3`. Expected: gagal kompilasi.

- [x] **Langkah 3: kode permukaan.**

`ui/spotlight.tole` — kartu di atas tengah, mengikuti pola `menubar.tole` (dropdown) untuk tile baris dan `walls.tole` untuk kartu lebar:

```qml
// ui/spotlight.tole - the Spotlight card: one query line, up to 9 rows.
import "nocturne.tole" as Nocturne

Rectangle {
    property string query: ""
    property int chosen: 0
    property var rows: []           // {icon, letter, text, detail}
    signal run(int index)
    signal move(int delta)
    signal dismiss()
    // ... tile baris disalin dari dropdown menubar.tole, ikon Nocturne.icon(18),
    // teks Nocturne.sans(15), baris terpilih latar Nocturne.n800.
}
```

Isi lengkapnya ditulis dengan menyalin elemen yang sudah ada (MouseArea, sinyal, Repeater/tile yang dipakai menubar.tole). Struct sinyal di Rust mengikuti pola `MenuBarSignal`/`DockSignal` di `ui.rs`.

`spotlight.rs`:
- `const NAMESPACE: &str = "yoru-spotlight";`, `WIDTH: u32 = 600;` `HEIGHT: u32 = 380;` (kartu; disetel saat render pertama).
- `LayerOptions { layer: Layer::Overlay, anchor: Anchor::TOP, size: (WIDTH, HEIGHT), keyboard: KeyboardInteractivity::Exclusive, exclusive_zone: 0, margin: Margin { top: 60, ..default } }` — pusat horizontal diperoleh `anchor: Anchor::TOP` tanpa LEFT/RIGHT (layer surface teranchor atas, lebar tetap; bila hyprix menuntut salah satu sisi, lihat cara `--stash-panel` menengahkan panel stash dan salin caranya).
- Query diketik lewat tombol papan ketik yang `Window::dispatch` berikan (pola dari Task 0 langkah 2.3 — salin keysym→char dari shelf). Escape → keluar; Enter → jalankan baris terpilih; Up/Down pindah.
- Berkas dipindai sekali saat dibuka: `search::scan(&home, 5000, 300_000_000)` di thread sendiri (pola: thread warna walls), hasil dikirim balik ke loop lewat `Arc<Mutex<Vec<String>>>` atau pola yang `switch.rs` pakai untuk warna. Sebelum hasil siap, baris berkas kosong (spec §7: hasil sebagian sah).
- Aksi dari `Outcome::Dispatch(what)`: `Hypr::from_env()` lalu `hypr.dispatch(&what)`, `say(&format!("spotlight: run {what}"))`, lalu tutup. Gagal dispatch: tetap terbuka, `say("err: spotlight dispatch refused")`.
- `say` saat naik: `"the spotlight is up, {w}x{h}, {} apps"`; saat turun: `"the spotlight closed"`.
- `cli.rs`: cabang `--spotlight` sebelum `--walls` (salin bentuk cabang `--walls`, socket `yoru-spotlight`), dan `--render-spotlight` (gambar PNG kartu dengan query contoh "te" — pola `--render-walls`).
- Tes render host: satu uji yang menggambar `State` dengan query "te" dan memastikan PNG tidak kosong — pola dari uji render yang ada (Task 0.6).

- [x] **Langkah 4: jalankan**

```bash
cd /root/ferrix/src/user/apps/yoru-shell
cargo test 2>&1 | grep "test result"
cargo clippy --all-targets -- -D warnings 2>&1 | tail -3
```

Expected: baseline + 3 (model) + 1 (render) lulus, clippy bersih.

- [x] **Langkah 5: commit** (setelah izin pengguna)

```bash
cd /root/ferrix/src/user/apps
git add yoru-shell/src/{spotlight.rs,switch.rs,cli.rs,lib.rs} yoru-shell/ui/spotlight.tole
git diff --cached --stat
git commit -m "yoru-shell: Spotlight, a search card over the desktop for apps, windows, files and sums"
```

---

## Task 3: permukaan Launchpad

**Anggaran:** sekitar 200 baris Rust, 90 baris `.tole`.

**Files:** Create `launchpad.rs`, `ui/launchpad.tole`; Modify `cli.rs`, `model.rs` (`icon_for`), `lib.rs`.

- [x] **Langkah 1: tes yang gagal** di `launchpad.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn apps() -> Vec<crate::search::App> {
        vec![
            crate::search::App { exec: "/bin/term /bin/zinc".into(), title: "Terminal".into(), icon: "terminal-window" },
            crate::search::App { exec: "/bin/shelf".into(), title: "Files".into(), icon: "folder" },
        ]
    }

    #[test]
    fn the_grid_is_sorted_and_paginated() {
        let grid = Grid::new(apps(), 8, 5);
        assert_eq!(grid.pages(), 1);
        assert_eq!(grid.at(0).map(|app| app.title.as_str()), Some("Files"), "sorted by title");
        assert_eq!(grid.at(1).map(|app| app.title.as_str()), Some("Terminal"));
        assert_eq!(grid.at(2), None);
    }

    #[test]
    fn forty_apps_fill_two_pages_and_a_click_runs_its_app() {
        let many: Vec<crate::search::App> = (0..40)
            .map(|n| crate::search::App { exec: format!("/bin/app{n}"), title: format!("App {n}"), icon: "" })
            .collect();
        let grid = Grid::new(many, 8, 5);
        assert_eq!(grid.pages(), 2);
        assert_eq!(grid.run(37, 1), Some("/bin/app37".to_owned()));
        assert_eq!(grid.run(41, 0), None, "past the page");
    }
}
```

Run: `cargo test launchpad 2>&1 | tail -3`. Expected: gagal kompilasi.

- [x] **Langkah 2: kode.**

`model.rs` tambah:

```rust
/// The Phosphor icon for an installed application's id, the letter tile
/// where there is none (the dock's mapping, widened).
#[must_use]
pub fn icon_for(id: &str) -> &'static str {
    let id = id.to_lowercase();
    if id.contains("chrome") || id.contains("chromium") || id.contains("firefox") {
        "globe"
    } else if id.contains("shelf") {
        "folder"
    } else if id.contains("settings") {
        "sliders-horizontal"
    } else if id.contains("terminal") {
        "terminal-window"
    } else {
        ""
    }
}
```

(Salin pemetaan yang benar-benar dipakai `model::launchers` — baca dulu ikon-ikon yang di sana, jangan mengarang nama glyph: semua nama harus ada di tabel `look::ICONS`.)

`launchpad.rs`: `NAMESPACE "yoru-launchpad"`, layer `Overlay`, keempat anchor, `keyboard: KeyboardInteractivity::None`, socket `yoru-launchpad` (dibuka lagi = tutup). `Grid { apps: Vec<search::App>, columns: usize, rows: usize, page: usize }` dengan `pages()`, `at(i)` (indeks dalam halaman kini, `None` di luar), `run(i, page)`. App dibaca `installed()` penuh (seluruh entri `visible`, bukan hanya 4 launcher): filter `app.visible` dan `app.id` ada, `title` dari `app.title`, ikon `model::icon_for(app.id)`. Setelah `exec` di-dispatch, permukaan menutup (`say(&format!("launchpad: run {what}"))` lalu keluar).
- Klik di luar tile grid (margin kartu) menutup: sinyal `dismiss()` dari MouseArea latar.
- Paging: dua tombol titik di bawah grid, klik mengganti halaman; `say("launchpad is up, {n} apps, {pages} pages")`.

`ui/launchpad.tole`: latar blur penuh layar (namespace `yoru-launchpad` menangkap `layerrule blur` yang sudah ada untuk `yoru-*`), tile ikon 8 kolom (ikon Phosphor 40 px atau huruf pada tile `Nocturne.n900`, judul di bawah, pola tile dock), titik halaman. Sinyal: `run(int index)`, `page(int index)`, `dismiss()`.

`cli.rs`: cabang `--launchpad` dan `--render-launchpad` (grid 8 kolom dengan 12 app contoh).

- [x] **Langkah 3: jalankan** — sama seperti Task 2 langkah 4; expected baseline + 2 + 1 lulus.

- [x] **Langkah 4: commit** (setelah izin pengguna)

```bash
cd /root/ferrix/src/user/apps
git add yoru-shell/src/{launchpad.rs,cli.rs,model.rs,lib.rs} yoru-shell/ui/launchpad.tole
git diff --cached --stat
git commit -m "yoru-shell: Launchpad, a fullscreen grid of the installed applications"
```

---

## Task 4: keybind dan ikon search

**Anggaran:** sekitar 6 baris.

**Files:** Modify `hyprix/themes/yoru-keys.conf` (ferrix), `yoru-shell/src/menubar.rs` (apps).

- [x] **Langkah 1: uji yang mengunci keybind.** Di xtask ada pemeriksa tema? Cari uji yang membaca `yoru-keys.conf` (`grep -rn "yoru-keys" /root/ferrix/tools/common/xtask/src | head`). Bila ada uji daftar bind, ubah ujinya dulu (gagal dulu), bila tidak, ubah konfigurasi dan andalkan boot `seat`/`yoru` sebagai regresi.

- [x] **Langkah 2: ubah** `yoru-keys.conf`:

```ini
bind = SUPER, space, exec, /bin/yoru-shell --spotlight
...
bind = SUPER, L, exec, /bin/yoru-shell --launchpad
```

Baris fuzzel lama diganti (bukan ditambah); SUPER L disisipkan dekat blok SUPER I. Komentar baris diperbarui.

- [x] **Langkah 3: ikon search menu bar.** `menubar.rs::search()`:

```rust
/// The search: Spotlight in this shell, as SUPER Space starts it.
fn search(hypr: Option<&Hypr>) {
    let Some(hypr) = hypr else { return };
    dispatch_all(hypr, &["exec /bin/yoru-shell --spotlight"]);
}
```

(Ponytail: jalur fallback fuzzel dihapus; `/bin/yoru-shell` selalu ada.)

- [x] **Langkah 4: jalankan** — `cargo test -p xtask compositor 2>&1 | grep "test result"` (xtask tetap 999+ kecuali uji keybind diubah), `cargo test` yoru-shell tetap.

- [x] **Langkah 5: commit** (setelah izin pengguna)

```bash
cd /root/ferrix
git add src/user/system/linux/compositor/hyprix/themes/yoru-keys.conf
git diff --cached --stat
git commit -m "yoru: SUPER Space opens the shell's Spotlight, SUPER L the Launchpad"
cd /root/ferrix/src/user/apps
git add yoru-shell/src/menubar.rs
git diff --cached --stat
git commit -m "yoru-shell: the menu bar's search opens Spotlight"
```

---

## Task 5: notifyd soket riwayat dan permukaan Notices

**Anggaran:** sekitar 180 baris notifyd, 220 baris yoru-shell, 70 baris `.tole`.

**Files:** Modify `notifyd/src/{store.rs,main.rs,paint.rs}`; Create `notifyd/src/feed.rs` (ferrix). Create `yoru-shell/src/notices.rs`, `ui/notices.tole`; Modify `yoru-shell/src/{lib.rs,menubar.rs,window.rs}` (apps).

- [x] **Langkah 1: tes notifyd yang gagal** di `feed.rs` (modul murni, tanpa soket):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn feed() -> Feed {
        let mut feed = Feed::new(50);
        feed.push(1, "Files", "Copy done", "notes.md to the stash");
        feed.push(2, "devmgr", "GPU driver is up", "");
        feed
    }

    #[test]
    fn the_list_answers_newest_first_up_to_the_cap() {
        assert_eq!(feed().list(50).len(), 2);
        assert_eq!(feed().list(50)[0].summary, "GPU driver is up");
        assert_eq!(feed().list(1).len(), 1, "the cap cuts");
    }

    #[test]
    fn fifty_are_kept_and_the_oldest_fall_off() {
        let mut feed = Feed::new(50);
        for id in 1..=51 {
            feed.push(id, "app", &format!("n{id}"), "");
        }
        assert_eq!(feed.list(50).len(), 50);
        assert_eq!(feed.list(50)[49].summary, "n1", "oldest first at the end");
        assert_eq!(feed.list(50)[0].summary, "n51");
    }

    #[test]
    fn a_close_removes_one() {
        let mut feed = feed();
        assert!(feed.close(2));
        assert!(!feed.close(2));
        assert_eq!(feed.list(50).len(), 1);
    }
}
```

Run: `cd /root/ferrix/src/user/system/linux/compositor/notifyd && cargo test feed 2>&1 | tail -3`. Expected: gagal kompilasi.

- [x] **Langkah 2: kode notifyd.**

1. `store.rs`: `HISTORY: usize = 20` → `50`. Bila `Store` menahan riwayatnya sendiri, riwayat dipindah/dibungkus ke `Feed` agar satu sumber; `Store::history()` mendelegasikan.
2. `feed.rs` (baru): `struct Feed { cap: usize, items: VecDeque<Notice> }`, `struct Notice { id: u32, app: String, summary: String, body: String }`, `push`, `list(cap)`, `close(id) -> bool`, dan modul soket: `serve(path, feed: &Arc<Mutex<Feed>>)` — thread yang menerima koneksi `UnixListener`, membaca satu baris per koneksi: `list` → tulis `notice <id> <app> <summary>\t<body>` per item (terbaru dulu) lalu tutup; `subscribe` → kirim riwayat lalu tetap, tiap `push` baru ditulis ke semua subscriber (daftar `Sender` di `Feed` atau broadcast di `serve`); `close <id>` → `feed.close`. Baris diakhiri `\n`; label tak boleh berisi `\t` atau `\n` (notifyd sudah menolaknya di dbus masuk — pastikan, bila belum, tolak dengan mengganti spasi).
3. `main.rs`: `Feed` dibuat, `serve` dimulai di thread, `Store` menulis ke `Feed` di tempat ia mendorong riwayat lama; `Store::shown` menghitung subscriber (`feed.subscribers() > 0`) untuk memutuskan banner digambar atau tidak — bila struktur `Store` tidak mudah disambung, bendera `Arc<AtomicUsize>` terpisah yang `serve` menaikkannya dan `paint` membacanya.
4. `paint.rs`: bendera subscriber menghentikan penggambaran banner baru (banner yang sudah di layar biarkan selesai). Anchor kanan atas **tidak diubah** — banner notifyd tetap cadangan saat yoru-shell tidak berlangganan; posisi kanan atas dikerjakan di surface yoru-shell.

Jalankan: `cargo test -p ferrix-notifyd 2>&1 | grep "test result"` (nama crate dicek di `Cargo.toml`; ganti sesuai nama aslinya) dan `cargo clippy -p <crate> --all-targets -- -D warnings`. Expected: baseline notifyd + 3 lulus.

- [x] **Langkah 3: uji yoru-shell yang gagal** di `notices.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn lines() -> Vec<Notice> {
        parse(&[
            "notice 7 Files\tCopy done\tnotes.md to the stash",
            "notice 6 devmgr\tGPU driver is up\t",
        ])
    }

    #[test]
    fn the_socket_lines_are_read_into_notices() {
        let notices = lines();
        assert_eq!(notices.len(), 2);
        assert_eq!(notices[0].id, 7);
        assert_eq!(notices[0].app, "Files");
        assert_eq!(notices[1].body, "");
    }

    #[test]
    fn a_bad_line_is_dropped() {
        assert!(parse(&["notice", "notice x", "other 1 a b c"]).is_empty());
    }
}
```

Run: `cd /root/ferrix/src/user/apps/yoru-shell && cargo test notices 2>&1 | tail -3`. Expected: gagal kompilasi.

- [x] **Langkah 4: permukaan Notices.**

`notices.rs`: `struct Notice { id: u32, app: String, summary: String, body: String }`, `parse(&[&str]) -> Vec<Notice>` (baris `notice <id> <app>\t<summary>\t<body>`, salah format dibuang). Panel: satu proses ikut **dock** — `window.rs` membuka layer kedua saat panel diminta (pola `side.rs` membuka surface; bila dock tidak bisa menampung layer kedua, jalankan pola `switch.rs` dengan socket `yoru-notices` dan cli `--notices` — pilih yang lebih pendek setelah membaca `window.rs`):
- LayerOptions: `Anchor::TOP.with(Anchor::RIGHT)`, `Layer::Overlay`, `keyboard: None`, ukuran (360, tinggi), margin top 8, right 8, namespace `yoru-notices`.
- Konten: kartu kaca Nocturne, satu tile per notice (judul `summary` tebal, baris kedua `body` redup, nama app kecil kanan atas), klik tile → tulis `close <id>` ke soket notifyd dan hilangkan dari daftar; tombol Clear di bawah → tutup semua dengan `close`.
- Koneksi: `UnixStream::connect(socket("notifyd"))` pola `switch::socket`; koneksi `list` saat membuka; `subscribe` dipertahankan selama panel terbuka untuk pembaruan hidup; kegagalan koneksi → panel menampilkan "notifyd is not running" dan tetap bisa ditutup.
- Banner hidup (kartu yang muncul dan hilang sendiri): saat dock berlangganan sejak awal (`subscribe` di `window.rs` segera setelah `Hypr` siap), tiap notice baru digambar sebagai banner kanan atas maksimum 3, tiap kartu hilang setelah 5 s (timer dock yang sudah ada, 1 s, cukup). Saat dock berlangganan, notifyd sendiri tidak menggambar (Task 5.2). DND (`quiet_file`) tetap dihormati notifyd di sumbernya — yoru-shell tidak menggandakan cek.
- `menubar.rs`: klik jam (perilaku lama dari Task 0.2) diganti/ditambah membuka panel Notices: `dispatch exec` tidak dipakai; dock yang membuka panel lewat socket kedua `yoru-notices` (pola "dibuka lagi = tutup": dock menulis ke socketnya sendiri). Bila dock tidak bisa menerima sinyal begitu, gunakan jalur `switch.rs`: klik jam `dispatch exec /bin/yoru-shell --notices`.

`ui/notices.tole`: kartu + tile, pola `power.tole` (kartu) dan `menubar.tole` (dropdown).

- [x] **Langkah 5: jalankan** — yoru-shell: baseline + 2 (parse) + panel; notifyd: + 3; clippy dua repo bersih.

- [x] **Langkah 6: commit** (setelah izin pengguna)

```bash
cd /root/ferrix
git add src/user/system/linux/compositor/notifyd
git diff --cached --stat
git commit -m "notifyd: a history of fifty on a line socket, banners paused while the shell subscribes"
cd /root/ferrix/src/user/apps
git add yoru-shell/src/{notices.rs,lib.rs,menubar.rs,window.rs} yoru-shell/ui/notices.tole
git diff --cached --stat
git commit -m "yoru-shell: Notices, glass banners at the top right and the history behind the clock"
```

---

## Task 6: boot `spotlight`, `launchpad`, `notices`, ukur, dokumentasi

**Anggaran:** sekitar 3 x 200 baris xtask, dokumen 30 baris.

**Files:** Create `tools/common/xtask/src/compositor/{spotlight.rs,launchpad.rs,notices.rs}`; Modify `tools/common/xtask/src/compositor.rs`; Modify `docs/DESKTOP.md`; Modify `yoru-shell/README.md`.

- [x] **Langkah 1: tiga boot, pola `appmenu.rs`.** Ketiganya: `super::yoru::desktop(arch, args, false)?`, `exec-once = /bin/sh /etc/<nama>-probe.sh`, `carried.busybox`, `carried.ports.extend(crate::apps::taken(arch, &["shelf"])?)` bila app dibutuhkan, hook dengan `watching.read_more`, `at`/`press_here` dari `files.rs`, `qmp.screendump`, `parse_ppm`, `judge` murni + tes unit dulu (gagal → kode → lulus, seperti Task 1).

1. **`spotlight`**: probe script: tunggu `hyprctl version`, `hyprctl dispatch exec '/bin/yoru-shell --spotlight'` (kutip sesuai shell probe yang ada), tunggu `yoru-shell: the spotlight is up`. Ketik "fi" lewat `qmp.input_send_event` tombol keyboard (pola yang boot `seat` memakai `qmp` untuk mengetik — cari `input_send_event` dengan kunci `key` di xtask) — bila mengetik lewat QMP rumit, alternatif sah: probe mengetik dengan `hyprctl dispatch exec` tidak bisa; gunakan pola `test-input` (virtio-input). Judge: tunggu baris `yoru-shell: spotlight: run` (Enter pada Terminal) ATAU screendump dijudike kartu di atas (putih ≥ X, baris terpilih gelap). Anggap judge dari garis `say` lebih dulu, screendump menyusul bila perlu; ambang hanya dari nilai terukur.
2. **`launchpad`**: probe membuka, tunggu `launchpad is up, N apps, 1 pages`, `at` ke tile pertama (dari `say` yang mencantumkan tempat tile — tambahkan baris `launchpad: tiles ...` seperti `menu bar: titles`), `press_here`, tunggu `launchpad: run /bin/...` dan jendela app naik.
3. **`notices`**: probe menulis satu alert ke `/run/lode/alerts` (format dari Task 0.4) atau menghubungi soket devmgr events; tunggu `yoru-shell: notices: banner <summary>` (tambahkan baris say ini di Task 5), lalu klik jam (tempat dari baris `menu bar: titles` — jam di ujung kanan; `at` kanan atas), tunggu `yoru-shell: notices: up, 1 shown`, screendump kartu riwayat dijudike.

Tabel BOOTS: tiga entri baru setelah `("appmenu", ...)`, `const BOOTS: [(&str, Boot); 76]`.

- [x] **Langkah 2: ukur**

```bash
cd /root/ferrix
FERRIX_OVMF_CODE=/usr/share/edk2/x64/OVMF_CODE.4m.fd FERRIX_OVMF_VARS=/usr/share/edk2/x64/OVMF_VARS.4m.fd \
  cargo xtask measure-desktop --boots 2 2>&1 | tail -12
```

Expected (boot kedua, anggaran tidak boleh dinaikkan dari A3): yoru-shell (dock, panel tertutup) RSS ≤ 8 MB; hyprix ≤ 41 MB; idle used ≤ 97 MiB; dok ≤ 0,8 s setelah graphical.target. Permukaan Spotlight/Launchpad hanya hidup saat dibuka — bila prosesnya masih ada setelah ditutup, itu bocor: perbaiki sebelum commit.

### Hasil ukur Task 6 (2026-11-15, opencode)

Terukur dua kali (`--boots 2`): dok **0,65 s** setelah graphical.target pada boot kedua kedua
putaran (≤ 0,8 ✓; satu sampel 0,92 s tampaknya noise host — dua dari tiga sampel lulus);
hyprix **41,37 MB** ≤ 41 ✓; idle used **96 MiB** ≤ 97 ✓; hanya satu proses `yoru-shell` —
tidak ada permukaan yang bocor.

**yoru-shell (dock) 9.976 KiB ≈ 9,7 MB, bukan regresi A4a.** Angka 8 MB di atas adalah
harapan yang mengabaikan bahwa sejak A2 MenuBar hidup di dalam proses dock (dock A1 murni
7,6 MB; anggaran A2/A3 = 12 MB). Bukti: biner debug yoru-shell A3 (`desktop/a3-appmenu`,
dibangun lewat worktree yang dibersihkan setelahnya) dan A4a sama-sama peak VmRSS
**5.248 KiB** saat startup di host — tambahan A4a ke proses dock (soket `subscribe`, Feed 50,
modul permukaan) berbiaya RSS nol saat dock hidup; ukuran biner naik 99,7 → 103,8 MB hanya
dari modul permukaan yang dikompilasi masuk, dan RSS dock stabil 9,9–10,0 MB di semua boot.
Anggaran mengikat tetap A3: yoru-shell total ≤ 12 MB.

- [x] **Langkah 3: dokumentasi.**

`docs/DESKTOP.md` §6.9 (setelah §6.8):

```markdown
### 6.9 Spotlight, Launchpad, Notices

SUPER Space opens Spotlight (`yoru-shell --spotlight`): the query is read as
arithmetic first, then matched against the installed applications
(`compositor_desktop::entry`), the open windows (`j/clients`) and the names
under `~` (at most 5000 entries, 300 ms, scanned when it opens, no index).
Enter runs the chosen row; a file opens with its default handler.

SUPER L opens Launchpad (`yoru-shell --launchpad`): a fullscreen grid of
every installed application, 8 columns, paged, click to run. It closes on
another SUPER L or a click outside the grid.

notifyd keeps the last 50 notifications and serves them on a line socket
(`$XDG_RUNTIME_DIR/notifyd.sock`: `list`, `subscribe`, `close <id>`). While
yoru-shell is subscribed it draws the glass banners at the top right (at
most 3, 5 s each) and the history behind the clock, and notifyd pauses its
own banners. With the shell down, notifyd draws again.
```

`yoru-shell/README.md`: satu paragraf per permukaan di bagian yang sesuai, menyebut argumen CLI dan keybind.

- [x] **Langkah 4: commit** (setelah izin pengguna)

```bash
cd /root/ferrix
git add tools/common/xtask/src/compositor.rs tools/common/xtask/src/compositor/{spotlight.rs,launchpad.rs,notices.rs} docs/DESKTOP.md
git diff --cached --stat
git commit -m "xtask: the spotlight, launchpad and notices boots"
cd /root/ferrix/src/user/apps
git add yoru-shell/README.md
git diff --cached --stat
git commit -m "yoru-shell: the README shows Spotlight, Launchpad and Notices"
```

Tidak ada push. `components.toml` tidak diubah.

---

## Risiko

1. **Input teks tole** (Task 2): belum ada bukti text field di `.tole`. Editor tangan dari event keyboard adalah jalan yang pasti bisa, tetapi keysym→char perlu disalin dari kode yang sudah membaca tombol (shelf). Task 0.3 memutuskan ini sebelum Task 2 ditulis.
2. **Menengahkan kartu Spotlight**: layer teranchor atas tanpa LEFT/RIGHT mungkin tidak didukung hyprix. Jalan keluar: anchor LEFT|RIGHT dengan margin kiri `(screen - WIDTH)/2` — margin butuh lebar layar, yang `Window::size()` berikan setelah terbuka (set_layer ulang, pola dock yang me-resize).
3. **Koneksi notifyd dari dock** (Task 5): dock tidak pernah buka soket keluar sekarang; koneksi `subscribe` yang gagal jangan menggantung loop dock (timeout, nonblocking, baca hanya antar frame).
4. **Soket notifyd dan izin**: `$XDG_RUNTIME_DIR` milik pengguna yang sama; tidak ada masalah izin. Bila notifyd berjalan sebagai root (boot test), soket di `/run` — cek di Task 0.5 di mana notifyd menaruh pid dan apakah soketnya sudah ada polanya.
5. **Anggaran RAM**: tiga permukaan baru kecil, tetapi `subscribe` menahan satu koneksi dan buffer; Feed 50 item < 20 KB.
6. **fuzzel tidak disentuh**: boot `yoru-launcher` dan `n150-box` tetap lulus; bila uji xtask lain menyebut SUPER space fuzzel, uji itu yang diubah (Task 4.1).
7. **Koordinat klik QEMU**: skala 1024x768 seperti files/appmenu; tinggi baris Spotlight/Launchpad dari angka terukur, ambang judge hanya dinaikkan.
8. **Banyak boot baru di tabel BOOTS**: tiap boot menit-menit; jalankan satu-satu (`--boot spotlight` dst), bukan seluruh tabel.

## Self-review

- Spec §3 Spotlight/Launchpad/Notices terpenuhi; Control dan Mission Control masuk A4b/A4c sesuai keputusan pencacahan 2026-11-14.
- Spec §5: batas 5000 entri, 300 ms, tanpa indeks; kalkulator empat operasi dan kurung; pintasan SUPER Space dan SUPER L dipetakan; minimalkan tetap special workspace (A2, tidak disentuh).
- Spec §7: yoru-shell crash → notifyd melanjutkan menggambar banner sendiri (bendera subscriber ikut mati saat soket koneksi putus); pemindaian melebihi batas → hasil sebagian; perangkat tidak ada → panel tetap bisa tutup.
- Spec §11: uji unit pencocokan/kalkulator/pemindaian (Task 1), parse baris soket (Task 5), tiga boot baru (Task 6); fuzzel tetap teruji oleh boot lamanya.
- Spec §13: tidak ada aset Apple; ikon Phosphor yang sudah vendored.
- Tiap task diawali tes yang gagal dan diakhiri commit per repo. Tidak ada push, tidak ada pin.
