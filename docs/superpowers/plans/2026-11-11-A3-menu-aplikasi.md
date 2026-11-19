# A3: Menu aplikasi di MenuBar - Rencana Implementasi

> **Untuk pelaksana:** kerjakan task demi task, berurutan. Tiap task dimulai dengan tes yang gagal, lalu kode, lalu tes lulus, lalu commit. Commit hanya setelah izin pengguna.

**Goal:** aplikasi Ferrix bisa memasang menunya sendiri (File, Edit, View, Go) di MenuBar yoru-shell, dan klik item menu kembali ke aplikasi pemiliknya. Aplikasi yang tidak memasang menu tetap mendapat menu standar A2.

**Arsitektur:**
1. Crate murni baru `compositor-appmenu` memegang pohon menu, batasnya, langkah wire, dan format teks untuk IPC.
2. Protokol Wayland `ferrix_appmenu_v1` membawa pohon itu dari klien ke compositor sebagai deret request kecil yang diakhiri `commit`.
3. compositor-server menyimpan pohon di `Toplevel`. hyprix mengumumkan `appmenu>>` di socket2, menjawab request `appmenu` dengan pohon jendela fokus, dan menerima `dispatch appmenu <address> <id>`.
4. tole mendapat `Window::set_menu(&Tree)` dan `Window::take_menu_picks()`.
5. MenuBar yoru-shell meminta pohon saat fokus berganti atau saat `appmenu>>` datang, tanpa polling.
6. Files (shelf) dan Settings memasang menu aslinya.

**Tech stack:** Rust (workspace compositor, tole, apps), XML protokol + generator Python yang sudah ada, `cargo xtask` untuk boot QEMU.

**Spec:** `docs/superpowers/specs/2026-11-11-desktop-gaya-macos-design.md` §4, §5, §7, §11, §13, §14.

**Gaya kode (instruksi pemilik):** "ponytail". Kode sesedikit mungkin, tanpa fitur atau opsi spekulatif, memakai ulang bagian protokol/toolkit/IPC yang sudah ada. Tiap penyederhanaan yang disengaja diberi komentar pendek `// Ponytail: ... Upgrade path: ...`.

---

## Aturan eksekusi

- Semua perintah dijalankan di WSL Arch: tulis skrip ke scratchpad, jalankan dengan `MSYS_NO_PATHCONV=1 timeout 600 wsl.exe -d archlinux -- bash -l <skrip> < /dev/null 2>&1`.
- Tiga repo, tiap repo dapat branch `desktop/a3-appmenu`:
  - ferrix: `/root/ferrix` (compositor ada di `src/user/system/linux/compositor`, xtask di `tools/common/xtask`), dipotong dari `desktop/a2-dok-menubar` (c4b234c19).
  - apps: `/root/ferrix/src/user/apps` (yoru-shell, shelf, settings), dipotong dari `desktop/a2-dok-menubar` (6adcadb).
  - tole: `/root/ferrix/src/user/system/linux/tole`, dipotong dari HEAD terpin a8b356c.
- Commit: tanpa trailer Co-Authored-By, hook tetap aktif, tidak pernah `--no-verify`, tidak pernah `git stash`, selalu `git diff --cached --stat` sebelum commit, identitas pemilik repo.
- Tidak ada push. Tidak ada pin baru di `components.toml`.
- Cargo.lock di ferrix dan apps sudah kotor oleh kenaikan versi 0.1.0 ke 0.3.0 yang tidak terkait. Hanya hunk kita yang di-stage, lewat `stage_ours` (Task 0).
- String UI dalam bahasa Inggris. Tidak ada nama atau aset Apple di berkas yang dikirim.
- Boot memakai `FERRIX_OVMF_CODE=/usr/share/edk2/x64/OVMF_CODE.4m.fd FERRIX_OVMF_VARS=/usr/share/edk2/x64/OVMF_VARS.4m.fd`.

## Keputusan pengguna yang dibutuhkan

1. [KEPUTUSAN] **Protokol bertahap, bukan `set(tree)`.** Pesan Wayland dibatasi 4096 byte (`compositor-wire` `MAX_MESSAGE`), jadi satu request tidak bisa membawa 256 item. Default: `add_menu`, `add_item`, `add_separator`, lalu `commit`. Alternatif: satu request `set` dengan `array` yang dibatasi sekitar 4 KB.
2. [KEPUTUSAN] **Submenu ditunda.** Spec mengizinkan satu tingkat submenu, tetapi Files dan Settings tidak membutuhkannya. Default: v1 tanpa submenu. Upgrade path: tambah `begin_submenu`/`end_submenu` di protokol versi 2. Alternatif: buat sekarang, dengan MenuBar meratakannya.
3. [KEPUTUSAN] **Event `appmenu>>` tanpa data.** Event socket2 dipotong pada 1024 byte, dan subscriber yang lambat dibuang. Default: `appmenu>>` hanya sinyal "ada menu berubah". MenuBar lalu meminta `appmenu` (pohon jendela fokus dalam format teks). Event tidak lewat `Snapshot`/`Watcher`, jadi `compositor-ipc` tidak berubah. Alternatif: event membawa alamat jendela, dengan field `Window.menu` di `Snapshot`.
4. [KEPUTUSAN] **Generator Python cukup ditambah satu baris `FILES`**, tidak di-port ke Rust (sesuai peta: port hanya kalau berkasnya disentuh berat). Alternatif: port ke `cargo xtask gen-protocol`.
5. [KEPUTUSAN] **Probe libwayland memakai XML tablet yang di-vendor.** `/usr/share/wayland-protocols` tidak ada di host WSL, jadi `interfaces.sh` gagal di baris tablet. Default: arahkan baris itu ke `$protocols/tablet-v2.xml`. Alternatif: `pacman -S wayland-protocols` (dengan izin).
6. [KEPUTUSAN] **Tempat menu aplikasi:** setelah menu nama aplikasi, sebelum Window.
7. [KEPUTUSAN] **`get_menu` kedua untuk toplevel yang sama menggantikan yang pertama.** tole memanggil ulang `set_menu` tiap tanda centang berubah. Upgrade path: request `clear` di objek yang sama.
8. [KEPUTUSAN] **Boot memakai Files (shelf).** Menu Settings juga dibuat dan dites di unit, tetapi tidak di boot.
9. [KEPUTUSAN] **About tetap nonaktif** seperti di A2.
10. [KEPUTUSAN] **Negasi lintas klien.** Id objek Wayland berlaku per koneksi, jadi klien B secara struktur tidak bisa menyebut toplevel milik A. Kontrol negatifnya: B memberi `wl_surface` miliknya sendiri (bukan toplevel) ke `get_menu`, menerima protocol error, dan menu A tetap utuh.

## Peta berkas

| Repo | Berkas | Task |
|---|---|---|
| ferrix | `src/user/system/linux/compositor/Cargo.toml` (members) | 1 |
| ferrix | `src/user/system/linux/compositor/appmenu/{Cargo.toml,src/lib.rs}` (baru) | 1 |
| ferrix | `.../compositor/protocol/protocols/ferrix-appmenu-v1.xml` (baru) | 1 |
| ferrix | `tools/common/gen/gen-wayland-protocol.py` (satu baris `FILES`) | 1 |
| ferrix | `.../compositor/protocol/src/{lib.rs,tests.rs,generated/mod.rs,generated/appmenu.rs}` | 1 |
| ferrix | `.../compositor/protocol/probe/{interfaces.sh,interfaces.c,interfaces.txt}` | 1 |
| ferrix | `.../compositor/server/{Cargo.toml,src/role.rs,src/xdg.rs,src/client.rs,src/client/appmenu.rs,src/client/event.rs}` | 2 |
| ferrix | `.../compositor/toolkit/src/client.rs` (`toplevel_object`) | 2 |
| ferrix | `.../compositor/hyprix/{Cargo.toml,src/state.rs,src/act.rs,src/control.rs,src/prompt/tests.rs,tests/two_clients.rs}` | 2 |
| tole | `tole/{Cargo.toml,src/lib.rs,src/window.rs}` | 3 |
| apps | `yoru-shell/{src/menus.rs,src/menubar.rs,src/hypr.rs,ui/menubar.tole,README.md}` | 4, 6 |
| apps | `shelf/src/{menu.rs,main.rs,window.rs}`, `settings/src/{menu.rs,main.rs,window.rs}` | 5 |
| ferrix | `tools/common/xtask/src/{compositor.rs,compositor/appmenu.rs}` | 6 |
| ferrix | `docs/DESKTOP.md` (§6.8) | 6 |

---

## Task 0: Branch, baseline, dan `stage_ours`

**Anggaran:** tidak ada kode produk.

- [ ] **Langkah 1: buat branch di ketiga repo**

```bash
set -e
cd /root/ferrix && git status --short | head && git switch -c desktop/a3-appmenu desktop/a2-dok-menubar
cd /root/ferrix/src/user/apps && git switch -c desktop/a3-appmenu desktop/a2-dok-menubar
cd /root/ferrix/src/user/system/linux/tole && git switch -c desktop/a3-appmenu a8b356c
for r in /root/ferrix /root/ferrix/src/user/apps /root/ferrix/src/user/system/linux/tole; do git -C $r branch --show-current; done
```

Expected: tiga baris `desktop/a3-appmenu`. `git status` hanya menunjukkan Cargo.lock yang memang sudah kotor.

- [ ] **Langkah 2: catat baseline tes**

```bash
cd /root/ferrix/src/user/system/linux/compositor && cargo test -p compositor-protocol -p compositor-server -p compositor-ipc 2>&1 | grep "test result" | head
cd /root/ferrix/src/user/apps/yoru-shell && cargo test 2>&1 | grep "test result"
```

Expected: semua `ok. N passed; 0 failed`. Simpan angkanya. Tes A3 nanti harus menambah, bukan mengganti.

- [ ] **Langkah 3: helper `stage_ours`** (ditulis ke scratchpad, bukan ke repo)

```bash
# stage_ours <Cargo.lock> <word>: stage only the -U0 hunks that mention <word>.
# Ponytail: a hunk mixing our line with an unrelated bump is kept whole.
# Upgrade path: review such a hunk by hand with `git diff --cached`.
stage_ours() {
  git diff -U0 -- "$1" | awk -v w="$2" '
    /^diff --git|^index |^--- |^\+\+\+ / { print; next }
    /^@@/ { if (keep) printf "%s", h; h = $0 "\n"; keep = 0; next }
    { h = h $0 "\n"; if (index($0, w)) keep = 1 }
    END { if (keep) printf "%s", h }' > /tmp/ours.patch
  git apply --cached --unidiff-zero /tmp/ours.patch
}
```

Expected saat dipakai: `git diff --cached -- Cargo.lock` hanya berisi baris dengan `compositor-appmenu`.

Tidak ada commit di Task 0.

---

## Task 1: crate `compositor-appmenu` dan protokol `ferrix_appmenu_v1`

**Anggaran:** sekitar 220 baris Rust (separuhnya tes) dan 60 baris XML.

**Files:**
- Create: `src/user/system/linux/compositor/appmenu/Cargo.toml`, `appmenu/src/lib.rs`
- Modify: `compositor/Cargo.toml` (members)
- Create: `compositor/protocol/protocols/ferrix-appmenu-v1.xml`
- Modify: `tools/common/gen/gen-wayland-protocol.py`, `protocol/src/lib.rs`, `protocol/src/tests.rs`, `protocol/src/generated/mod.rs`, `protocol/probe/interfaces.sh`, `protocol/probe/interfaces.c`
- Generated: `protocol/src/generated/appmenu.rs`, `protocol/probe/interfaces.txt`

- [ ] **Langkah 1: tes yang gagal untuk crate**

`appmenu/Cargo.toml` (header disalin dari `compositor/wire/Cargo.toml`):

```toml
[package]
name = "compositor-appmenu"
description = "An application's own menus: the tree, its limits, its wire steps and its IPC text"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true

[lints]
workspace = true
```

Tambahkan `"appmenu"` ke `members` di `compositor/Cargo.toml`, dengan urutan yang sama seperti member lain.

`appmenu/src/lib.rs`, untuk sementara hanya modul tes:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Tree {
        Tree {
            menus: vec![
                Menu {
                    label: "File".to_owned(),
                    entries: vec![
                        Entry::Item(Item::new(1, "New Tab").with_keys("Ctrl T")),
                        Entry::Line,
                        Entry::Item(Item::new(3, "Close Tab").with_enabled(false)),
                    ],
                },
                Menu {
                    label: "View".to_owned(),
                    entries: vec![Entry::Item(Item::new(20, "Two Panes").with_check(true))],
                },
            ],
        }
    }

    fn built(steps: &[Step]) -> Result<Option<Tree>, Wrong> {
        let mut builder = Builder::default();
        let mut last = Ok(None);
        for step in steps {
            last = builder.apply(step);
            if last.is_err() {
                break;
            }
        }
        last
    }

    #[test]
    fn steps_build_back_the_same_tree() {
        assert_eq!(built(&files().steps()), Ok(Some(files())));
    }

    #[test]
    fn the_text_answer_carries_the_tree_and_the_window() {
        let text = to_text(0xab12, &files());
        assert!(text.starts_with("window\t0xab12\nmenu\tFile\n"));
        assert_eq!(from_text(&text), Some((0xab12, files())));
        assert_eq!(from_text("none\n"), None);
    }

    #[test]
    fn only_enabled_items_can_be_activated() {
        assert!(files().has(1));
        assert!(!files().has(3));
        assert!(!files().has(99));
    }

    #[test]
    fn seventeen_menus_are_too_many() {
        let mut steps: Vec<Step> = (0..=MAX_MENUS).map(|n| Step::Menu(format!("M{n}"))).collect();
        steps.push(Step::Commit);
        assert_eq!(built(&steps), Err(Wrong::TooMany));
    }

    #[test]
    fn more_than_256_entries_are_too_many() {
        let mut steps = vec![Step::Menu("File".to_owned())];
        steps.extend((0..=MAX_ENTRIES).map(|_| Step::Line));
        assert_eq!(built(&steps), Err(Wrong::TooMany));
    }

    #[test]
    fn labels_are_short_printable_and_not_empty() {
        let long = "x".repeat(MAX_LABEL + 1);
        for label in ["", "a\tb", long.as_str()] {
            assert_eq!(built(&[Step::Menu(label.to_owned())]), Err(Wrong::BadLabel), "{label:?}");
        }
        assert_eq!(built(&[Step::Menu("x".repeat(MAX_LABEL))]), Ok(None));
    }

    #[test]
    fn an_item_before_any_menu_is_a_bad_tree() {
        assert_eq!(built(&[Step::Line]), Err(Wrong::BadTree));
    }
}
```

Run:

```bash
cd /root/ferrix/src/user/system/linux/compositor && cargo test -p compositor-appmenu 2>&1 | tail -5
```

Expected: gagal kompilasi, `cannot find type 'Tree'` dan sejenisnya.

- [ ] **Langkah 2: kode crate**, ditulis di atas modul tes pada `lib.rs`:

```rust
//! An application's own menus: the tree the menu bar shows, the limits a
//! client must keep, the steps that carry the tree over `ferrix_appmenu_v1`,
//! and the text hyprix answers the `appmenu` request with.

use std::fmt::{self, Write as _};

/// At most this many top-level menus.
pub const MAX_MENUS: usize = 16;
/// At most this many items and separators in one tree.
pub const MAX_ENTRIES: usize = 256;
/// At most this many bytes in a label or a shortcut.
pub const MAX_LABEL: usize = 128;
/// `add_item` flag: the item can be picked.
pub const ENABLED: u32 = 1;
/// `add_item` flag: the item shows a check mark.
pub const CHECKED: u32 = 2;

/// Every menu of one window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tree {
    /// The menus, left to right.
    pub menus: Vec<Menu>,
}

/// One title in the bar and what drops below it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    /// The title.
    pub label: String,
    /// What drops below it, top to bottom.
    pub entries: Vec<Entry>,
}

/// A row of a dropped menu.
// Ponytail: no submenus in v1. Upgrade path: an `Entry::Sub` and
// `begin_submenu`/`end_submenu` in protocol version 2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// Something to pick.
    Item(Item),
    /// A separator.
    Line,
}

/// Something to pick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// What `activated` sends back to the application.
    pub id: u32,
    /// The words.
    pub label: String,
    /// The shortcut shown on the right, empty for none.
    pub keys: String,
    /// Whether it can be picked.
    pub enabled: bool,
    /// Whether it shows a check mark.
    pub checked: bool,
}

impl Item {
    /// An enabled, unchecked item without a shortcut.
    #[must_use]
    pub fn new(id: u32, label: &str) -> Self {
        Self { id, label: label.to_owned(), keys: String::new(), enabled: true, checked: false }
    }

    /// The same item showing `keys`.
    #[must_use]
    pub fn with_keys(mut self, keys: &str) -> Self {
        keys.clone_into(&mut self.keys);
        self
    }

    /// The same item, checked or not.
    #[must_use]
    pub fn with_check(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// The same item, enabled or not.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    fn flags(&self) -> u32 {
        (if self.enabled { ENABLED } else { 0 }) | (if self.checked { CHECKED } else { 0 })
    }
}

/// One `ferrix_appmenu_v1` request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// `add_menu`.
    Menu(String),
    /// `add_item`.
    Item {
        /// The id.
        id: u32,
        /// The words.
        label: String,
        /// The shortcut.
        keys: String,
        /// [`ENABLED`] and [`CHECKED`].
        flags: u32,
    },
    /// `add_separator`.
    Line,
    /// `commit`.
    Commit,
}

/// Why a tree was refused. Each is a `ferrix_appmenu_v1.error`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrong {
    /// An entry came before any menu.
    BadTree,
    /// Over [`MAX_MENUS`] or [`MAX_ENTRIES`].
    TooMany,
    /// Empty, over [`MAX_LABEL`] bytes, or holding a control character.
    BadLabel,
}

impl fmt::Display for Wrong {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::BadTree => "an entry came before any menu",
            Self::TooMany => "more than 16 menus or 256 entries",
            Self::BadLabel => "a label is empty, longer than 128 bytes or holds a control character",
        })
    }
}

impl std::error::Error for Wrong {}

impl Tree {
    /// The requests that build this tree, ending in `commit`.
    #[must_use]
    pub fn steps(&self) -> Vec<Step> {
        let mut steps = Vec::new();
        for menu in &self.menus {
            steps.push(Step::Menu(menu.label.clone()));
            for entry in &menu.entries {
                steps.push(match entry {
                    Entry::Item(item) => Step::Item {
                        id: item.id,
                        label: item.label.clone(),
                        keys: item.keys.clone(),
                        flags: item.flags(),
                    },
                    Entry::Line => Step::Line,
                });
            }
        }
        steps.push(Step::Commit);
        steps
    }

    /// Whether `id` is an enabled item, the only kind that may be activated.
    #[must_use]
    pub fn has(&self, id: u32) -> bool {
        self.menus.iter().flat_map(|menu| &menu.entries).any(|entry| {
            matches!(entry, Entry::Item(item) if item.id == id && item.enabled)
        })
    }

    /// Whether there is nothing to show.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.menus.is_empty()
    }
}

/// Builds a tree from steps, keeping the limits.
#[derive(Debug, Default)]
pub struct Builder {
    menus: Vec<Menu>,
    entries: usize,
}

impl Builder {
    /// Takes one step. `commit` hands back the tree and starts afresh.
    ///
    /// # Errors
    ///
    /// [`Wrong`] when the step breaks the shape or a limit.
    pub fn apply(&mut self, step: &Step) -> Result<Option<Tree>, Wrong> {
        match step {
            Step::Menu(label) => {
                label_ok(label, false)?;
                if self.menus.len() >= MAX_MENUS {
                    return Err(Wrong::TooMany);
                }
                self.menus.push(Menu { label: label.clone(), entries: Vec::new() });
            }
            Step::Item { id, label, keys, flags } => {
                label_ok(label, false)?;
                label_ok(keys, true)?;
                self.add(Entry::Item(Item {
                    id: *id,
                    label: label.clone(),
                    keys: keys.clone(),
                    enabled: flags & ENABLED != 0,
                    checked: flags & CHECKED != 0,
                }))?;
            }
            Step::Line => self.add(Entry::Line)?,
            Step::Commit => {
                self.entries = 0;
                return Ok(Some(Tree { menus: std::mem::take(&mut self.menus) }));
            }
        }
        Ok(None)
    }

    fn add(&mut self, entry: Entry) -> Result<(), Wrong> {
        let menu = self.menus.last_mut().ok_or(Wrong::BadTree)?;
        if self.entries >= MAX_ENTRIES {
            return Err(Wrong::TooMany);
        }
        self.entries += 1;
        menu.entries.push(entry);
        Ok(())
    }
}

fn label_ok(label: &str, may_be_empty: bool) -> Result<(), Wrong> {
    let bad = (label.is_empty() && !may_be_empty)
        || label.len() > MAX_LABEL
        || label.chars().any(char::is_control);
    if bad { Err(Wrong::BadLabel) } else { Ok(()) }
}

/// The `appmenu` answer: a `window` line, then one line per menu, item and
/// separator, fields split by tabs. Labels never hold tabs or newlines.
#[must_use]
pub fn to_text(address: u64, tree: &Tree) -> String {
    let mut text = format!("window\t0x{address:x}\n");
    for menu in &tree.menus {
        let _ = writeln!(text, "menu\t{}", menu.label);
        for entry in &menu.entries {
            let _ = match entry {
                Entry::Item(item) => writeln!(
                    text,
                    "item\t{}\t{}\t{}\t{}",
                    item.id,
                    item.flags(),
                    item.label,
                    item.keys
                ),
                Entry::Line => writeln!(text, "line"),
            };
        }
    }
    text
}

/// The window and its tree back from [`to_text`]; `None` for `none` or
/// anything malformed.
#[must_use]
pub fn from_text(text: &str) -> Option<(u64, Tree)> {
    let mut lines = text.lines();
    let address = lines.next()?.strip_prefix("window\t0x")?;
    let address = u64::from_str_radix(address, 16).ok()?;
    let mut builder = Builder::default();
    for line in lines {
        let mut fields = line.split('\t');
        let step = match fields.next()? {
            "menu" => Step::Menu(fields.next()?.to_owned()),
            "item" => Step::Item {
                id: fields.next()?.parse().ok()?,
                flags: fields.next()?.parse().ok()?,
                label: fields.next()?.to_owned(),
                keys: fields.next().unwrap_or_default().to_owned(),
            },
            "line" => Step::Line,
            _ => return None,
        };
        builder.apply(&step).ok()?;
    }
    Some((address, builder.apply(&Step::Commit).ok()??))
}
```

Run lagi: `cargo test -p compositor-appmenu`. Expected: `test result: ok. 7 passed; 0 failed`.

Lalu `cargo clippy -p compositor-appmenu --all-targets -- -D warnings`. Expected: tidak ada peringatan. Lint `format_push_string` tidak kena karena kita memakai `writeln!`. Bila `missing_docs` atau lint pedantic lain dari workspace menolak sesuatu, ikuti pesan clippy, jangan menambah `allow`.

- [ ] **Langkah 3: tes protokol yang gagal**

Di `protocol/src/tests.rs`:
- tambahkan `appmenu` ke daftar `use crate::{...}`,
- tambahkan `&appmenu::FERRIX_APPMENU_MANAGER_V1, &appmenu::FERRIX_APPMENU_V1,` ke `fn tables()`.

Run: `cargo test -p compositor-protocol 2>&1 | tail -3`. Expected: gagal kompilasi, `unresolved import crate::appmenu`.

- [ ] **Langkah 4: XML protokol** `protocol/protocols/ferrix-appmenu-v1.xml`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<protocol name="ferrix_appmenu_v1">
  <copyright>
    Copyright (c) 2026 Sebastian Dauenhauer

    Permission is hereby granted, free of charge, to any person obtaining a
    copy of this software and associated documentation files (the "Software"),
    to deal in the Software without restriction, including without limitation
    the rights to use, copy, modify, merge, publish, distribute, sublicense,
    and/or sell copies of the Software, and to permit persons to whom the
    Software is furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in
    all copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
    FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
    DEALINGS IN THE SOFTWARE.
  </copyright>

  <interface name="ferrix_appmenu_manager_v1" version="1">
    <description summary="give a toplevel its own menus">
      The menu bar shows the menus of the focused toplevel. A toplevel
      without menus gets the standard ones.
    </description>
    <request name="destroy" type="destructor"/>
    <request name="get_menu">
      <description summary="the menus of one of this client's toplevels">
        A second get_menu for the same toplevel replaces the first.
      </description>
      <arg name="id" type="new_id" interface="ferrix_appmenu_v1"/>
      <arg name="toplevel" type="object" interface="xdg_toplevel"/>
    </request>
  </interface>

  <interface name="ferrix_appmenu_v1" version="1">
    <description summary="the menus of one toplevel">
      Menus are built with add_menu, add_item and add_separator, then shown
      by commit. Each commit replaces the whole tree; an empty commit brings
      back the standard menus. At most 16 menus and 256 entries; a label is
      1 to 128 bytes without control characters. The object dies with its
      toplevel.
    </description>
    <enum name="error">
      <entry name="bad_tree" value="0" summary="an entry came before any menu"/>
      <entry name="too_many" value="1" summary="more than 16 menus or 256 entries"/>
      <entry name="bad_label" value="2" summary="a label is empty, too long or has a control character"/>
    </enum>
    <request name="destroy" type="destructor"/>
    <request name="add_menu">
      <arg name="label" type="string"/>
    </request>
    <request name="add_item">
      <arg name="id" type="uint"/>
      <arg name="label" type="string"/>
      <arg name="keys" type="string" summary="the shortcut to show, empty for none"/>
      <arg name="flags" type="uint" summary="1 enabled, 2 checked"/>
    </request>
    <request name="add_separator"/>
    <request name="commit"/>
    <event name="activated">
      <description summary="the user picked an enabled item"/>
      <arg name="id" type="uint"/>
    </event>
  </interface>
</protocol>
```

- [ ] **Langkah 5: generate dan sambungkan**

1. `tools/common/gen/gen-wayland-protocol.py`: tambah `("ferrix-appmenu-v1.xml", "appmenu"),` tepat sebelum baris `("linux-dmabuf-v1.xml", "linux_dmabuf")`.
2. `protocol/src/generated/mod.rs`: tambah `pub mod appmenu;` di urutan abjad (sebelum `pub mod background_effect;`).
3. `protocol/src/lib.rs`: tambah `appmenu` ke daftar `pub use generated::{...}`, dan `&appmenu::FERRIX_APPMENU_MANAGER_V1,` di `GLOBALS` tepat sebelum `&linux_dmabuf::ZWP_LINUX_DMABUF_V1` (yang harus tetap terakhir).
4. Probe:
   - di `interfaces.sh`, tambah `scan "$protocols/ferrix-appmenu-v1.xml" ferrix-appmenu-v1`, lalu ganti sumber tablet `/usr/share/wayland-protocols/stable/tablet/tablet-v2.xml` menjadi `"$protocols/tablet-v2.xml"` ([KEPUTUSAN] 5);
   - di `interfaces.c`, tambah `#include "ferrix-appmenu-v1-client-protocol.h"`, serta `/* ferrix-appmenu-v1.xml */ &ferrix_appmenu_manager_v1_interface, &ferrix_appmenu_v1_interface,` di array, dengan pola baris yang sama seperti entri lain.

```bash
cd /root/ferrix
python3 tools/common/gen/gen-wayland-protocol.py
python3 tools/common/gen/gen-wayland-protocol.py --check && echo GEN-OK
cd src/user/system/linux/compositor/protocol && sh probe/interfaces.sh && git diff --stat probe/interfaces.txt
grep -n "pub mod event\|pub const ACTIVATED\|pub const BAD_TREE\|pub const ADD_ITEM" src/generated/appmenu.rs
cd .. && cargo test -p compositor-protocol 2>&1 | grep "test result"
```

Expected:
- `GEN-OK`,
- `interfaces.txt` hanya bertambah baris untuk dua interface baru,
- grep menemukan `event::ACTIVATED`, `error::BAD_TREE`, `request::ADD_ITEM`,
- tes lulus, termasuk `every_generated_table_is_the_one_libwayland_compiled` dan `every_global_is_bindable`.

Bila `interfaces.sh` membaca jalur probe lain yang tidak ada di host, itu "belum diketahui": jalankan dengan `sh -x` dan arahkan jalur itu ke XML di `protocols/` dengan cara yang sama.

- [ ] **Langkah 6: commit** (setelah izin pengguna)

```bash
cd /root/ferrix
git add src/user/system/linux/compositor/appmenu src/user/system/linux/compositor/Cargo.toml \
  src/user/system/linux/compositor/protocol tools/common/gen/gen-wayland-protocol.py
stage_ours src/user/system/linux/compositor/Cargo.lock compositor-appmenu
git diff --cached --stat
git commit -m "compositor: ferrix_appmenu_v1 carries an application's own menus, built by compositor-appmenu"
```

---

## Task 2: compositor-server, hyprix, dan toolkit

**Anggaran:** sekitar 150 baris produk dan 150 baris tes integrasi.

**Files:**
- Modify: `compositor/server/Cargo.toml` (dep `compositor-appmenu = { path = "../appmenu" }`)
- Modify: `server/src/role.rs`, `server/src/xdg.rs`, `server/src/client.rs`, `server/src/client/event.rs`
- Create: `server/src/client/appmenu.rs`
- Modify: `toolkit/src/client.rs`
- Modify: `hyprix/Cargo.toml` (dep `compositor-appmenu`; dev-dep `compositor-toolkit`), `hyprix/src/state.rs`, `hyprix/src/act.rs`, `hyprix/src/control.rs`, `hyprix/src/prompt/tests.rs`
- Test: `hyprix/tests/two_clients.rs`

- [ ] **Langkah 1: tes integrasi yang gagal** di `hyprix/tests/two_clients.rs`

Tes memakai helper yang sudah ada (`Desk::new`, `running`, `Ends`, `until`, `PATIENCE`) dan klien `compositor_toolkit`.

Beberapa hal belum diketahui dan harus disalin dari tes yang sudah ada di berkas yang sama, bukan dikarang:
- cara membuka klien toolkit ke `ends.socket`: cari `compositor_toolkit::Client::connect_to`, atau bila belum ada, salin dari `compositor/toolkit/examples`;
- cara menggambar satu frame supaya toplevel terpeta (`client.draw(surface, |paint| ...)`);
- cara berlangganan socket2 (lihat tes `subscribed()` dan `compositor_ctl::subscribe`).

```rust
use compositor_appmenu::{Entry, Item, Menu, Tree};
use compositor_toolkit::protocol::appmenu::{
    self as appmenu, ferrix_appmenu_manager_v1 as manager, ferrix_appmenu_v1 as menu,
};
use compositor_toolkit::{Client, Event, ToplevelOptions, Value};

fn tree() -> Tree {
    Tree {
        menus: vec![Menu {
            label: "File".to_owned(),
            entries: vec![Entry::Item(Item::new(7, "New Tab").with_keys("Ctrl T"))],
        }],
    }
}

/// A mapped toplevel and its menu object, built from `tree`.
fn menued(socket: &std::path::Path, tree: &Tree) -> (Client, compositor_toolkit::ObjectId) {
    let mut client = Client::connect_to(socket).expect("a client");
    let surface = client
        .toplevel(&ToplevelOptions { app_id: "test.menu".into(), ..ToplevelOptions::default() })
        .expect("a toplevel");
    // (draw one frame here, copied from the existing toolkit tests)
    let toplevel = client.toplevel_object(surface).expect("its xdg_toplevel");
    let (bound, _) = client.bind(&appmenu::FERRIX_APPMENU_MANAGER_V1, 1, None).expect("the global");
    let made = client.new_object(&appmenu::FERRIX_APPMENU_V1, 1);
    client
        .request(bound, manager::request::GET_MENU, &[Value::NewId(made), Value::Object(toplevel)])
        .expect("get_menu");
    for step in tree.steps() {
        let (opcode, args) = match step {
            compositor_appmenu::Step::Menu(label) => (menu::request::ADD_MENU, vec![Value::Str(Some(label))]),
            compositor_appmenu::Step::Item { id, label, keys, flags } => (
                menu::request::ADD_ITEM,
                vec![Value::Uint(id), Value::Str(Some(label)), Value::Str(Some(keys)), Value::Uint(flags)],
            ),
            compositor_appmenu::Step::Line => (menu::request::ADD_SEPARATOR, vec![]),
            compositor_appmenu::Step::Commit => (menu::request::COMMIT, vec![]),
        };
        client.request(made, opcode, &args).expect("a step");
    }
    client.roundtrip().expect("the compositor took it");
    (client, made)
}

#[test]
fn a_client_menu_reaches_the_bar_and_a_pick_comes_back() {
    let desk = Desk::new("appmenu", &[]);
    running(&desk, &desk.options(), |ends| {
        // (subscribe to socket2 here as in `subscribed()`, collecting lines)
        let (mut client, made) = menued(&ends.socket, &tree());
        until("an appmenu event", || saw_line_starting_with("appmenu>>"));
        let (address, got) = until("the focused window's menu", || {
            compositor_appmenu::from_text(&ends.ask("appmenu")).ok_or("none yet")
        });
        assert_eq!(got, tree());
        assert_eq!(ends.ask(&format!("dispatch appmenu 0x{address:x} 7")).trim(), "ok");
        assert_ne!(ends.ask(&format!("dispatch appmenu 0x{address:x} 99")).trim(), "ok");
        until("activated 7", || {
            let events = client.dispatch(Some(std::time::Duration::from_millis(100))).map_err(|_| "closed")?;
            events
                .iter()
                .any(|event| matches!(event, Event::Object { object, opcode, args, .. }
                    if *object == made && *opcode == menu::event::ACTIVATED
                    && args.first().and_then(Value::as_uint) == Some(7)))
                .then_some(())
                .ok_or("not yet")
        });
        ends.stop();
    });
}

#[test]
fn a_surface_is_not_a_toplevel_and_too_many_menus_are_refused() {
    let desk = Desk::new("appmenu-refused", &[]);
    running(&desk, &desk.options(), |ends| {
        let (_first, _) = menued(&ends.socket, &tree());
        // A surface that is not a toplevel.
        let mut other = Client::connect_to(&ends.socket).expect("a client");
        let surface = other.toplevel(&ToplevelOptions::default()).expect("a toplevel");
        let wl_surface = other.surface_object(surface).expect("its wl_surface");
        let (bound, _) = other.bind(&appmenu::FERRIX_APPMENU_MANAGER_V1, 1, None).expect("the global");
        let made = other.new_object(&appmenu::FERRIX_APPMENU_V1, 1);
        let _ = other.request(bound, manager::request::GET_MENU, &[Value::NewId(made), Value::Object(wl_surface)]);
        assert!(matches!(other.roundtrip(), Err(compositor_toolkit::Error::Protocol { .. })));
        // The first client's menu is untouched.
        assert!(compositor_appmenu::from_text(&ends.ask("appmenu")).is_some());
        // Seventeen menus.
        let many = Tree {
            menus: (0..17).map(|n| Menu { label: format!("M{n}"), entries: vec![] }).collect(),
        };
        let refused = std::panic::catch_unwind(|| menued(&ends.socket, &many));
        assert!(refused.is_err(), "17 menus must be a protocol error");
        ends.stop();
    });
}
```

Catatan untuk pelaksana:
- `saw_line_starting_with` adalah closure di atas baris socket2 yang dikumpulkan.
- `until` dipakai sesuai tanda tangan aslinya di berkas ini; bila ia mengembalikan `()`, ambil nilai lewat variabel luar.
- `catch_unwind` boleh diganti dengan memanggil langkah-langkahnya langsung dan memeriksa `roundtrip()` menghasilkan `Err(Error::Protocol { .. })`. Pilih yang lebih pendek setelah melihat helpernya.

Tambahkan ke `hyprix/Cargo.toml` `[dev-dependencies]`: `compositor-toolkit = { path = "../toolkit" }` dan `compositor-appmenu = { path = "../appmenu" }`.

Run:

```bash
cd /root/ferrix/src/user/system/linux/compositor && cargo test -p hyprix --test two_clients appmenu 2>&1 | tail -5
```

Expected: gagal kompilasi, `no method named toplevel_object` dan global yang belum diiklankan.

- [ ] **Langkah 2: toolkit**, di `toolkit/src/client.rs` dekat `surface_object`:

```rust
    /// The `xdg_toplevel` behind `surface`, for protocols that name it.
    #[must_use]
    pub fn toplevel_object(&self, surface: SurfaceId) -> Option<ObjectId> {
        match &self.surfaces.get(&surface)?.kind {
            Kind::Toplevel { toplevel, .. } => Some(*toplevel),
            _ => None,
        }
    }
```

Bila `Kind` hanya punya satu varian lain, clippy bisa meminta `if let`. Ikuti sarannya.

- [ ] **Langkah 3: server**

1. `server/src/role.rs`: tambah varian `AppMenuManager` dan `AppMenu`, masing-masing dengan doc comment satu baris.
2. `server/src/xdg.rs` `Toplevel`: tambah field

```rust
    /// The application's own menus, from `ferrix_appmenu_v1`.
    pub menu: Option<compositor_appmenu::Tree>,
```

   (`Tree` menurunkan `Clone, Debug, Default, PartialEq, Eq`, jadi derive `Toplevel` tetap berlaku.)
3. `server/src/client/event.rs`: tambah varian

```rust
    /// A toplevel's own menus changed. hyprix tells socket2 `appmenu>>`.
    // Ponytail: no window in the event; the bar asks `appmenu` for the
    // focused one. Upgrade path: carry the toplevel when a listener needs it.
    MenuChanged,
```

4. `server/src/client.rs`:
   - tambah `mod appmenu;` di daftar modul (urut abjad),
   - tambah field `appmenus: BTreeMap<ObjectId, ObjectId>` (objek menu ke toplevel) dan `builders: BTreeMap<ObjectId, compositor_appmenu::Builder>`, keduanya diinisialisasi `BTreeMap::new()` di `new()`,
   - tambah `|| self.appmenu(sender, role, version, opcode, args)` di rantai `other =>` dalam `dispatch()`, dengan argumen yang sama seperti `self.desktop(..)`,
   - di `destroy()`:
     - arm `Role::XdgToplevel`: `self.appmenus.retain(|_, toplevel| *toplevel != object);`, lalu `self.builders.retain(|menu, _| self.appmenus.contains_key(menu));` (bila borrow checker menolak, kumpulkan id yang dibuang dulu),
     - arm baru `Role::AppMenu => { self.appmenus.remove(&object); self.builders.remove(&object); }`.

5. `server/src/client/appmenu.rs`. Tanda tangan `appmenu(..)` disalin persis dari `desktop(..)`; tipe `role` dan `args` mengikuti di sana. Cara mengantre event mengikuti cara yang sudah dibaca `take_events()`:

```rust
//! `ferrix_appmenu_v1`: an application's own menus for its toplevel.

use compositor_appmenu::{Builder, Step, Tree, Wrong};
use compositor_protocol::appmenu::{
    FERRIX_APPMENU_V1, ferrix_appmenu_manager_v1 as manager, ferrix_appmenu_v1 as menu,
};
use compositor_wire::{Arg, ArgType, ObjectId};

use super::{Client, Event, Fatal, Role};

impl Client {
    pub(super) fn appmenu(
        &mut self,
        sender: ObjectId,
        role: &Role,
        version: u32,
        opcode: u16,
        args: &[Arg<'_>],
    ) -> bool {
        match role {
            Role::AppMenuManager if opcode == manager::request::GET_MENU => {
                let (Some(id), Some(toplevel)) =
                    (args.first().and_then(Arg::as_object), args.get(1).and_then(Arg::as_object))
                else {
                    return true;
                };
                if !self.toplevels.contains_key(&toplevel) {
                    self.fail(Fatal::WrongInterface { object: toplevel, wanted: "xdg_toplevel" });
                    return true;
                }
                let replaced: Vec<ObjectId> =
                    self.appmenus.iter().filter(|(_, owner)| **owner == toplevel).map(|(menu, _)| *menu).collect();
                for menu in replaced {
                    self.appmenus.remove(&menu);
                    self.builders.remove(&menu);
                }
                if self.make(id, &FERRIX_APPMENU_V1, version, Role::AppMenu) {
                    self.appmenus.insert(id, toplevel);
                    self.builders.insert(id, Builder::default());
                }
                true
            }
            Role::AppMenuManager => true,
            Role::AppMenu => {
                self.appmenu_step(sender, opcode, args);
                true
            }
            _ => false,
        }
    }

    fn appmenu_step(&mut self, sender: ObjectId, opcode: u16, args: &[Arg<'_>]) {
        let text = |at: usize| args.get(at).and_then(Arg::as_str).unwrap_or_default().to_owned();
        let uint = |at: usize| args.get(at).and_then(Arg::as_uint).unwrap_or_default();
        let step = match opcode {
            menu::request::ADD_MENU => Step::Menu(text(0)),
            menu::request::ADD_ITEM => Step::Item { id: uint(0), label: text(1), keys: text(2), flags: uint(3) },
            menu::request::ADD_SEPARATOR => Step::Line,
            menu::request::COMMIT => Step::Commit,
            _ => return,
        };
        let (Some(&toplevel), Some(builder)) = (self.appmenus.get(&sender), self.builders.get_mut(&sender))
        else {
            return;
        };
        match builder.apply(&step) {
            Ok(None) => {}
            Ok(Some(tree)) => {
                if let Some(it) = self.toplevels.get_mut(&toplevel) {
                    it.menu = (!tree.is_empty()).then_some(tree);
                }
                self.events.push(Event::MenuChanged);
            }
            Err(wrong) => {
                let code = match wrong {
                    Wrong::BadTree => menu::error::BAD_TREE,
                    Wrong::TooMany => menu::error::TOO_MANY,
                    Wrong::BadLabel => menu::error::BAD_LABEL,
                };
                self.fail(Fatal::Interface { object: sender, code, text: wrong.to_string() });
            }
        }
    }

    /// The own menus of the toplevel on `surface`, if it set any.
    #[must_use]
    pub fn menu_of(&self, surface: ObjectId) -> Option<&Tree> {
        self.toplevels.values().find(|it| it.surface == surface)?.menu.as_ref()
    }

    /// Sends `activated(id)` to the owner of the toplevel on `surface`, when
    /// `id` is one of its enabled items.
    pub fn activate_menu(&mut self, surface: ObjectId, id: u32) -> bool {
        let Some((&toplevel, _)) = self
            .toplevels
            .iter()
            .find(|(_, it)| it.surface == surface && it.menu.as_ref().is_some_and(|tree| tree.has(id)))
        else {
            return false;
        };
        let Some(&object) = self.appmenus.iter().find(|(_, owner)| **owner == toplevel).map(|(object, _)| object)
        else {
            return false;
        };
        self.out.write(object, menu::event::ACTIVATED, &[ArgType::Uint], &[Arg::Uint(id)]).is_ok()
    }
}
```

   Builder disimpan di field kedua `builders` dan dibersihkan di tempat yang sama dengan `appmenus` (di `destroy()` dan `get_menu`).
   - Ponytail: dua map, bukan struct gabungan. Upgrade path: `BTreeMap<ObjectId, (ObjectId, Builder)>` bila ada field ketiga.
   - Bila `self.events` bukan nama antrean event klien, ikuti nama yang dipakai `take_events()`.
   - Bila `Toplevel.surface` bukan `ObjectId`, sesuaikan perbandingannya.
   - Bila `menu::error::BAD_TREE` bertipe lain dari `code: u32`, pakai `.into()`.

- [ ] **Langkah 4: hyprix**

1. `hyprix/Cargo.toml` `[dependencies]`: `compositor-appmenu = { path = "../appmenu" }`.
2. `state.rs` `fn globals(..)`: tambah `(&compositor_protocol::appmenu::FERRIX_APPMENU_MANAGER_V1, 1, Role::AppMenuManager),` di sebelah tuple `XDG_TOPLEVEL_TAG_MANAGER_V1`.
3. Event: di tempat hyprix membaca event klien (cari `Event::Bound` di `hyprix/src`; nama berkasnya belum diketahui), tambah

```rust
            Event::MenuChanged => self.events.say("appmenu>>\n"),
```

   `self.events` adalah `control::Events`. Nama field-nya di `State` belum diketahui: cari `Events {` di `state.rs`.
4. `act.rs` `fn compositor(..)`: tambah arm `"appmenu" => Some(appmenu(argument, around)),` dan fungsinya:

```rust
/// `dispatch appmenu <address> <id>`: the menu bar picked `id` from the own
/// menus of the window at `address`.
fn appmenu(argument: &str, around: &mut Around) -> bool {
    let mut words = argument.split_whitespace();
    let address = words.next().and_then(|word| u64::from_str_radix(word.trim_start_matches("0x"), 16).ok());
    let id = words.next().and_then(|word| word.parse::<u32>().ok());
    let (Some(address), Some(id)) = (address, id) else {
        around.say("appmenu: want <address> <id>");
        return false;
    };
    let Some(source) = around.sources.get(&WindowId(address)).copied() else {
        around.say("appmenu: no such window");
        return false;
    };
    let picked = around
        .slots
        .get_mut(source.client)
        .and_then(|slot| slot.client_mut())
        .is_some_and(|client| client.activate_menu(source.surface, id));
    if !picked {
        around.say("appmenu: no such item");
    }
    picked
}
```

   Hal yang dicocokkan:
   - Konstruktor `WindowId(address)` mengikuti `it.window.0` di `control.rs`.
   - Bila `Source` tidak `Copy`, ambil `client` dan `surface` ke variabel lebih dulu.
   - Bila `client_mut()` tidak mengembalikan `Option`, buang `and_then`.
5. `control.rs` `serve(..)`: tambah parameter `menu: &str` (teks jawaban yang sudah jadi). Di tempat `crate::settings::answer(&request, settings)` dipanggil:

```rust
        let answered = crate::settings::answer(&request, settings).or_else(|| {
            (request.command == "appmenu").then(|| crate::settings::Answered { answer: menu.to_owned(), then: None })
        });
```

   Ponytail: teks menu dihitung sekali per batch kontrol, walau tidak diminta. Ukurannya paling banyak beberapa KB, dan batch kontrol jarang. Upgrade path: closure malas.

   Bila `Answered` belum `pub(crate)`, jadikan `pub(crate)`.
6. `state.rs` `answer_control`: sebelum memanggil `serve`, hitung:

```rust
        let menu = self
            .focused_window()
            .and_then(|window| {
                let source = self.sources.get(&window)?;
                let tree = self.slots.get(source.client)?.client()?.menu_of(source.surface)?;
                Some(compositor_appmenu::to_text(window.0, tree))
            })
            .unwrap_or_else(|| "none\n".to_owned());
```

   Nama `focused_window()` dan `slot.client()` belum diketahui. Cari sumber `active_window` yang dipakai `self.snapshot()`, lalu pakai sumber yang sama.
7. `prompt/tests.rs:345`: tambah argumen `"none\n"` di panggilan `serve`.

- [ ] **Langkah 5: jalankan**

```bash
cd /root/ferrix/src/user/system/linux/compositor
cargo test -p compositor-server -p compositor-toolkit 2>&1 | grep "test result"
cargo test -p hyprix --test two_clients appmenu 2>&1 | tail -4
cargo test -p hyprix 2>&1 | grep "test result"
cargo clippy -p compositor-server -p compositor-toolkit -p hyprix --all-targets -- -D warnings 2>&1 | tail -3
```

Expected:
- dua tes appmenu `ok`,
- seluruh hyprix lulus dengan jumlah tes = baseline + 2,
- clippy bersih.

- [ ] **Langkah 6: commit** (setelah izin pengguna)

```bash
cd /root/ferrix
git add src/user/system/linux/compositor/{server,toolkit,hyprix}
stage_ours src/user/system/linux/compositor/Cargo.lock compositor-appmenu
git diff --cached --stat
git commit -m "hyprix: an application's own menus reach socket2 as appmenu, and a pick goes back to its owner"
```

---

## Task 3: tole `Window::set_menu`

**Anggaran:** sekitar 60 baris.

**Files:** `tole/tole/Cargo.toml`, `tole/tole/src/lib.rs`, `tole/tole/src/window.rs`

- [ ] **Langkah 1: tes yang gagal** di modul tes `window.rs`. Bila modul itu tidak ada, buat `#[cfg(test)] mod tests` di akhir berkas.

```rust
    #[test]
    fn each_menu_step_is_its_request() {
        use compositor_toolkit::protocol::appmenu::ferrix_appmenu_v1::request;
        let item = crate::appmenu::Step::Item { id: 4, label: "Rename".into(), keys: "F2".into(), flags: 1 };
        assert_eq!(
            super::request_of(item),
            (request::ADD_ITEM, vec![Value::Uint(4), Value::Str(Some("Rename".into())), Value::Str(Some("F2".into())), Value::Uint(1)])
        );
        assert_eq!(super::request_of(crate::appmenu::Step::Commit), (request::COMMIT, vec![]));
    }
```

Bila `Value` tidak menurunkan `PartialEq`, bandingkan dengan `format!("{:?}")`.

Run: `cd /root/ferrix/src/user/system/linux/tole && cargo test -p tole --features wayland request 2>&1 | tail -3`. Expected: gagal kompilasi, `crate::appmenu` belum ada.

- [ ] **Langkah 2: kode**

1. `tole/Cargo.toml`: `compositor-appmenu = { path = "../../compositor/appmenu" }`, dengan jalur relatif yang sama polanya seperti `compositor-image`.
2. `lib.rs`: `pub use compositor_appmenu as appmenu;` dengan doc comment `/// An application's own menus, for [`Window::set_menu`].`
3. `window.rs`:
   - field `menu: Option<ObjectId>` dan `picks: Vec<u32>` di `struct Window`, diisi `menu: None, picks: Vec::new()` di `Window::open`,
   - gate `cfg` sama dengan `clipboard`.

```rust
    /// Shows `tree` as this window's own menus in the menu bar; an empty
    /// tree brings back the standard ones. Picks arrive through
    /// [`Window::take_menu_picks`].
    // Ponytail: binds the manager and sends the whole tree on every call.
    // Upgrade path: keep the manager and send only a changed tree.
    pub fn set_menu(&mut self, tree: &crate::appmenu::Tree) -> bool {
        use compositor_toolkit::protocol::appmenu::{
            self as protocol, ferrix_appmenu_manager_v1 as manager, ferrix_appmenu_v1 as menu,
        };
        let Some(toplevel) = self.client.toplevel_object(self.surface) else { return false };
        let Ok((bound, _)) = self.client.bind(&protocol::FERRIX_APPMENU_MANAGER_V1, 1, None) else {
            return false;
        };
        if let Some(old) = self.menu.take() {
            let _ = self.client.request(old, menu::request::DESTROY, &[]);
        }
        let made = self.client.new_object(&protocol::FERRIX_APPMENU_V1, 1);
        let mut sent = self
            .client
            .request(bound, manager::request::GET_MENU, &[Value::NewId(made), Value::Object(toplevel)])
            .is_ok();
        for step in tree.steps() {
            let (opcode, args) = request_of(step);
            sent &= self.client.request(made, opcode, &args).is_ok();
        }
        let _ = self.client.request(bound, manager::request::DESTROY, &[]);
        self.menu = Some(made);
        sent && self.client.flush().is_ok()
    }

    /// The ids picked from this window's own menus since the last call.
    pub fn take_menu_picks(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.picks)
    }
```

   Fungsi bebas di modul yang sama:

```rust
fn request_of(step: crate::appmenu::Step) -> (u16, Vec<Value>) {
    use crate::appmenu::Step;
    use compositor_toolkit::protocol::appmenu::ferrix_appmenu_v1::request;
    match step {
        Step::Menu(label) => (request::ADD_MENU, vec![Value::Str(Some(label))]),
        Step::Item { id, label, keys, flags } => (
            request::ADD_ITEM,
            vec![Value::Uint(id), Value::Str(Some(label)), Value::Str(Some(keys)), Value::Uint(flags)],
        ),
        Step::Line => (request::ADD_SEPARATOR, vec![]),
        Step::Commit => (request::COMMIT, vec![]),
    }
}
```

   Arm baru di `dispatch()`, sebelum arm `clipboard`:

```rust
                Event::Object { object, opcode, ref args, .. }
                    if Some(object) == self.menu
                        && opcode == compositor_toolkit::protocol::appmenu::ferrix_appmenu_v1::event::ACTIVATED =>
                {
                    self.picks.extend(args.first().and_then(Value::as_uint));
                }
```

   Bila `flush()` mengembalikan `()` dan bukan `Result`, hapus `.is_ok()`. Bila compositor tidak punya global ini (compositor lain), `bind` gagal dan `set_menu` mengembalikan `false` tanpa error.

- [ ] **Langkah 3: jalankan**

```bash
cd /root/ferrix/src/user/system/linux/tole
cargo test --workspace --features wayland 2>&1 | grep "test result"
cargo clippy --workspace --all-targets --features wayland -- -D warnings 2>&1 | tail -3
```

Expected: semua lulus (baseline + 1), clippy bersih. Bila nama fitur bukan `wayland` di workspace, pakai fitur yang dipakai yoru-shell (`features = ["wayland"]`).

- [ ] **Langkah 4: commit** (setelah izin pengguna)

```bash
cd /root/ferrix/src/user/system/linux/tole
git add tole/Cargo.toml tole/src/lib.rs tole/src/window.rs Cargo.lock
git diff --cached --stat
git commit -m "tole: Window::set_menu shows an application's own menus and take_menu_picks hands back what was picked"
```

(Cargo.lock tole bersih sebelum A3, jadi boleh di-stage utuh.)

---

## Task 4: MenuBar yoru-shell memakai menu aplikasi

**Anggaran:** sekitar 70 baris Rust dan 10 baris `.tole`.

**Files:** `yoru-shell/src/menus.rs`, `src/menubar.rs`, `src/hypr.rs`, `ui/menubar.tole`

- [ ] **Langkah 1: tes yang gagal** di `menus.rs`:

```rust
    #[test]
    fn an_app_menu_goes_between_the_app_and_window_and_picks_through_hyprix() {
        use tole::appmenu::{Entry as Own, Item, Menu as OwnMenu, Tree};
        let tree = Tree {
            menus: vec![OwnMenu {
                label: "File".to_owned(),
                entries: vec![
                    Own::Item(Item::new(1, "New Tab").with_keys("Ctrl T").with_check(true)),
                    Own::Line,
                    Own::Item(Item::new(3, "Close Tab").with_enabled(false)),
                ],
            }],
        };
        let menus = with_app(standard(Some("Files"), "Files"), &tree, "0xab");
        let labels: Vec<&str> = menus.iter().map(|menu| menu.label.as_str()).collect();
        assert_eq!(labels, [MARK, "Files", "File", "Window"]);
        let file = &menus[2];
        assert!(matches!(&file.entries[0],
            Entry::Item { label, keys, checked: true, does: Some(Does::Dispatch(what)) }
            if label == "New Tab" && keys == "Ctrl T" && what == "appmenu 0xab 1"));
        assert!(matches!(file.entries[1], Entry::Line));
        assert!(matches!(&file.entries[2], Entry::Item { does: None, .. }));
    }
```

Panggilan `standard(..)` disalin dari tes A2 yang sudah ada; argumennya mengikuti tanda tangan nyata. Run: `cd /root/ferrix/src/user/apps/yoru-shell && cargo test menus 2>&1 | tail -3`. Expected: gagal kompilasi, `with_app` dan field `checked` belum ada.

- [ ] **Langkah 2: kode**

1. `menus.rs`:
   - tambah field `checked: bool` ke `Entry::Item`, dengan semua konstruktor A2 diisi `checked: false`; compiler menunjuk tiap tempatnya,
   - tambah fungsi:

```rust
/// `menus` with the application's own `tree` placed before Window. A pick
/// becomes `dispatch appmenu <address> <id>`; a disabled item does nothing.
#[must_use]
pub fn with_app(mut menus: Vec<Menu>, tree: &tole::appmenu::Tree, address: &str) -> Vec<Menu> {
    use tole::appmenu::Entry as Own;
    let own = tree.menus.iter().map(|menu| Menu {
        label: menu.label.clone(),
        bold: false,
        entries: menu
            .entries
            .iter()
            .map(|entry| match entry {
                Own::Line => Entry::Line,
                Own::Item(item) => Entry::Item {
                    label: item.label.clone(),
                    keys: item.keys.clone(),
                    checked: item.checked,
                    does: item.enabled.then(|| Does::Dispatch(format!("appmenu {address} {}", item.id))),
                },
            })
            .collect(),
    });
    let at = menus.iter().position(|menu| menu.label == "Window").unwrap_or(menus.len());
    menus.splice(at..at, own);
    menus
}
```

2. `hypr.rs`: tambah `"appmenu"` ke `EVENTS`. Tes `wanted` yang ada menjadi pengingat bila jumlahnya dikunci.
3. `ui/menubar.tole`:
   - tambah `checked: bool` ke `struct MenuLine`,
   - di baris item, tambah teks ikon centang di x:-2 (kiri label) yang terlihat bila `line.checked`, memakai pola ikon yang sudah ada di berkas `.tole` lain (`Nocturne.icon(12)` dengan glyph dari `look::icon("check")`).

   Cara mengirim glyph ke `.tole` (lewat field string `checkGlyph` di `bar`, atau properti global) belum diketahui. Lihat bagaimana dok A2 mengirim glyph ikon, lalu pakai cara yang sama. Isi `checked` di `View::show` saat membangun `MenuLine` dari `Entry::Item`.
4. `menubar.rs` `run()`:

```rust
fn ask_menu(hypr: &Hypr, focused: Option<&Focused>) -> Option<tole::appmenu::Tree> {
    let focused = focused?;
    let (address, tree) = tole::appmenu::from_text(&hypr.ask("appmenu")?)?;
    (format!("0x{address:x}") == focused.address).then_some(tree)
}
```

   Di loop:
   - `let mut own = ask_menu(&hypr, focused.as_ref());` saat mulai;
   - hitung ulang `own` saat `focused` berganti, dan saat baris dimulai dengan `appmenu>>`;
   - di tempat `let menus = standard(..)`:

```rust
            let menus = match (&own, &focused) {
                (Some(tree), Some(window)) => with_app(standard(/* same arguments as now */), tree, &window.address),
                _ => standard(/* same arguments as now */),
            };
            let titles: Vec<String> = menus
                .iter()
                .zip(&view.places)
                .map(|(menu, (x, w))| format!("{}@{x}+{w}", menu.label))
                .collect();
            let titles = titles.join("|");
            if titles != said_titles {
                say(&format!("menu bar: titles {titles}"));
                said_titles = titles;
            }
```

   Bila `places` baru dihitung di `view.show`, pindahkan log ke setelah `view.show`. Saat `Asked::Open(i)`: `say(&format!("menu bar: open {}", menus[i].label))`, lewat `.get(i)` karena indexing ditolak lint.
   - Ponytail: satu request `appmenu` per pergantian fokus. Upgrade path: cache per alamat bila profil menunjukkan biayanya.

- [ ] **Langkah 3: jalankan**

```bash
cd /root/ferrix/src/user/apps/yoru-shell
cargo test 2>&1 | grep "test result"
cargo clippy --all-targets -- -D warnings 2>&1 | tail -3
```

Expected: baseline + 1 lulus. Tiga tes A2 menubar tetap lulus, karena tanpa pohon menu jalurnya tidak berubah. Clippy bersih.

- [ ] **Langkah 4: commit** (setelah izin pengguna)

```bash
cd /root/ferrix/src/user/apps
git add yoru-shell/src yoru-shell/ui
git diff --cached --stat
git commit -m "yoru-shell: the menu bar shows the focused application's own menus and sends picks back through hyprix"
```

---

## Task 5: menu asli Files dan Settings

**Anggaran:** sekitar 90 baris per aplikasi, termasuk tes.

**Files:** `shelf/src/menu.rs` (baru), `shelf/src/main.rs` (`mod menu;`), `shelf/src/window.rs`; `settings/src/menu.rs` (baru), `settings/src/main.rs`, `settings/src/window.rs`. `tole::appmenu` sudah tersedia lewat dependensi `tole`, jadi tidak ada dependensi baru.

- [ ] **Langkah 1: tes yang gagal** di `shelf/src/menu.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menus_follow_the_window_and_each_id_is_an_action() {
        let tree = tree(true, false);
        let labels: Vec<&str> = tree.menus.iter().map(|menu| menu.label.as_str()).collect();
        assert_eq!(labels, ["File", "Edit", "View", "Go"]);
        assert!(tree.has(20) && tree.has(21));
        assert_eq!(action(1, 0), Some(Action::NewTab));
        assert_eq!(action(32, 1), Some(Action::Up(1)));
        assert_eq!(action(21, 0), Some(Action::Setting("hidden".to_owned())));
        assert_eq!(action(99, 0), None);
        for menu in &tree.menus {
            for entry in &menu.entries {
                if let Entry::Item(item) = entry {
                    assert!(action(item.id, 0).is_some(), "{} has no action", item.label);
                }
            }
        }
    }
}
```

Bila `Action` tidak menurunkan `PartialEq`, pakai `matches!`.

- [ ] **Langkah 2: kode shelf**

```rust
//! Files' own menus in the menu bar.

use crate::app::Action;
use tole::appmenu::{Entry, Item, Menu, Tree};

fn menu(label: &str, entries: Vec<Entry>) -> Menu {
    Menu { label: label.to_owned(), entries }
}

fn item(id: u32, label: &str, keys: &str) -> Entry {
    Entry::Item(Item::new(id, label).with_keys(keys))
}

/// The menus, with View's checks from `dual` and `hidden`.
pub fn tree(dual: bool, hidden: bool) -> Tree {
    Tree {
        menus: vec![
            menu("File", vec![item(1, "New Tab", "Ctrl T"), item(2, "New Folder", "Ctrl Shift N"), Entry::Line, item(3, "Close Tab", "Ctrl W")]),
            menu("Edit", vec![item(10, "Select All", "Ctrl A"), Entry::Line, item(11, "Rename", "F2"), item(12, "Move to Trash", "Delete")]),
            menu("View", vec![
                Entry::Item(Item::new(20, "Two Panes").with_check(dual)),
                Entry::Item(Item::new(21, "Show Hidden Files").with_keys("Ctrl H").with_check(hidden)),
            ]),
            menu("Go", vec![item(30, "Back", "Alt Left"), item(31, "Forward", "Alt Right"), item(32, "Enclosing Folder", "Alt Up")]),
        ],
    }
}

/// What picking `id` does; `focus` is the focused pane.
pub fn action(id: u32, focus: usize) -> Option<Action> {
    Some(match id {
        1 => Action::NewTab,
        2 => Action::NewFolder,
        3 => Action::CloseTab(focus),
        10 => Action::SelectAll,
        11 => Action::Rename,
        12 => Action::Trash,
        20 => Action::ToggleDual,
        21 => Action::Setting("hidden".to_owned()),
        30 => Action::Back,
        31 => Action::Forward,
        32 => Action::Up(focus),
        _ => return None,
    })
}
```

`CloseTab(usize)` menutup tab berindeks. Di sini dipakai indeks tab aktif (`app.active`), bukan pane. Karena itu panggilan dari window memakai `action(id, app.focus())` untuk Go, sedangkan untuk 3 memakai `app.active`.
- Ponytail: satu parameter `focus` untuk keduanya.
- Upgrade path: parameter terpisah bila tes menunjukkan beda.

Pelaksana memeriksa arti argumen `CloseTab` dan `Up` di `app.rs:345`, lalu menyesuaikan tes. Tambah `mod menu;` di `main.rs`.

`shelf/src/window.rs` `run(..)`:

```rust
    let mut menued = None;
    // inside the loop, next to `actions.extend(view.actions());`:
    for id in window.take_menu_picks() {
        say(&format!("menu: picked {id}"));
        actions.extend(crate::menu::action(id, app.focus()));
    }
    // where `dirty` leads to `view.show(&mut app)`:
    let checks = (app.tab().dual, app.settings.hidden);
    if menued != Some(checks) {
        window.set_menu(&crate::menu::tree(checks.0, checks.1));
        menued = Some(checks);
    }
```

- [ ] **Langkah 3: Settings**, dengan pola yang sama. `settings/src/menu.rs`:

```rust
//! Settings' own menu: one View item per page, the shown page checked.

use crate::pages::{Nav, Page};
use crate::ui::Action;
use tole::appmenu::{Entry, Item, Menu, Tree};

const FIRST: u32 = 100;

/// The View menu with `shown` checked.
pub fn tree(shown: Page) -> Tree {
    let entries = Page::ALL
        .iter()
        .zip(FIRST..)
        .map(|(page, id)| Entry::Item(Item::new(id, page.label()).with_check(*page == shown)))
        .collect();
    Tree { menus: vec![Menu { label: "View".to_owned(), entries }] }
}

/// The page `id` opens.
pub fn action(id: u32) -> Option<Action> {
    let page = Page::ALL.get(usize::try_from(id.checked_sub(FIRST)?).ok()?)?;
    Some(Action::Navigate(Nav::Page(*page)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_is_a_view_item_and_the_shown_one_is_checked() {
        let tree = tree(Page::Power);
        let [view] = tree.menus.as_slice() else { panic!("one menu") };
        assert_eq!(view.entries.len(), Page::ALL.len());
        assert!(view.entries.iter().any(|entry| matches!(entry, Entry::Item(item) if item.checked && item.label == "Power")));
        assert!(matches!(action(FIRST), Some(Action::Navigate(Nav::Page(Page::Displays)))));
        assert!(action(FIRST + 13).is_none());
        assert!(action(5).is_none());
    }
}
```

Bila `page.label()` mengembalikan `String`, ganti menjadi `&page.label()`. Bila `Page` tidak `Copy`/`PartialEq`, ikuti derive yang ada.

`settings/src/window.rs` `run(..)`: tambah `let mut menued = None;`, ambil picks setiap putaran (`actions.extend(window.take_menu_picks().into_iter().filter_map(crate::menu::action))`), dan kirim `window.set_menu(&crate::menu::tree(state.page))` saat `menued != Some(state.page)`.

- [ ] **Langkah 4: jalankan**

```bash
cd /root/ferrix/src/user/apps
for app in shelf settings; do (cd $app && cargo test 2>&1 | grep "test result" && cargo clippy --all-targets -- -D warnings 2>&1 | tail -1); done
```

Expected: baseline + 1 di masing-masing, clippy bersih.

- [ ] **Langkah 5: commit** (setelah izin pengguna). Satu commit per aplikasi:

```bash
cd /root/ferrix/src/user/apps
git add shelf/src && git diff --cached --stat && git commit -m "shelf: Files puts File, Edit, View and Go in the menu bar"
git add settings/src && git diff --cached --stat && git commit -m "settings: the View menu opens every page and checks the shown one"
```

Bila Cargo.lock aplikasi berubah karena tole kini memuat `compositor-appmenu`, pakai `stage_ours <app>/Cargo.lock compositor-appmenu` sebelum `git diff --cached --stat`.

---

## Task 6: boot `appmenu`, ukur, dan dokumentasi

**Anggaran:** sekitar 180 baris xtask (judge murni + tes unit), dokumen sekitar 40 baris.

**Files:**
- Create: `tools/common/xtask/src/compositor/appmenu.rs`
- Modify: `tools/common/xtask/src/compositor.rs` (`mod appmenu;` dan `("appmenu", appmenu::test_appmenu)` di tabel boot)
- Modify: `docs/DESKTOP.md` (§6.8)
- Modify: `src/user/apps/yoru-shell/README.md` (bagian "The dock and the menu bar")

- [ ] **Langkah 1: tes unit judge yang gagal**, di `appmenu.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn screen(drop: bool) -> Image {
        let (width, height) = SCREEN;
        let mut pixels = vec![0x40_u8; (width * height * 3) as usize];
        if drop {
            for y in 26..26 + 120 {
                for x in 100..100 + 260 {
                    let at = ((y * width + x) * 3) as usize;
                    let dark = (y - 26) % 24 == 12 && (x - 100) % 3 == 0 && x < 300;
                    let value = if dark { 0x20 } else { 0xfa };
                    pixels[at..at + 3].copy_from_slice(&[value; 3]);
                }
            }
        }
        Image { width, height, pixels }
    }

    #[test]
    fn a_dropped_menu_is_white_with_dark_words() {
        let judged = judge(&screen(true), 100).expect("a drop");
        assert!(judged.white >= WHITE && judged.dark >= DARK, "{judged:?}");
        assert!(judge(&screen(false), 100).is_err());
    }

    #[test]
    fn the_titles_line_gives_where_file_is() {
        let line = "yoru-shell: menu bar: titles FX@10+30|Files@58+40|File@116+36|Edit@170+36";
        assert_eq!(title(line, "File"), Some((116, 36)));
        assert_eq!(title(line, "Go"), None);
    }
}
```

Bila `Image.width` bertipe `usize`, buang cast. Tipenya diperiksa di `display.rs`.

Run: `cd /root/ferrix/tools/common/xtask && cargo test appmenu 2>&1 | tail -3`. Expected: gagal kompilasi.

- [ ] **Langkah 2: judge murni**

```rust
//! `appmenu`: Files sets its own menus, the bar shows them, a click drops
//! File and picking New Tab reaches Files.

use super::display::Image;

const SCREEN: (u32, u32) = (1024, 768);
/// White pixels a dropped menu needs below its title. Only ever raised.
const WHITE: usize = 15_000;
/// Dark word pixels it needs. Only ever raised.
const DARK: usize = 200;

#[derive(Debug)]
struct Judged {
    white: usize,
    dark: usize,
}

/// Counts the drop at `x`, 260 wide and 120 high below the bar.
fn judge(screen: &Image, x: u32) -> Result<Judged, String> {
    let mut judged = Judged { white: 0, dark: 0 };
    for y in 26..26 + 120 {
        for x in x..x + 260 {
            let at = ((y * screen.width + x) * 3) as usize;
            let Some(&[r, g, b]) = screen.pixels.get(at..at + 3).and_then(|rgb| <&[u8; 3]>::try_from(rgb).ok()) else {
                continue;
            };
            let sum = u32::from(r) + u32::from(g) + u32::from(b);
            if r >= 0xf0 && g >= 0xf0 && b >= 0xf0 {
                judged.white += 1;
            } else if sum < 0x150 {
                judged.dark += 1;
            }
        }
    }
    println!("appmenu: drop white {} (need {WHITE}), dark {} (need {DARK})", judged.white, judged.dark);
    if judged.white >= WHITE && judged.dark >= DARK { Ok(judged) } else { Err(format!("no dropped menu: {judged:?}")) }
}

/// Where `label` sits in a `menu bar: titles` line.
fn title(line: &str, label: &str) -> Option<(u32, u32)> {
    let titles = line.split("menu bar: titles ").nth(1)?;
    titles.split('|').find_map(|title| {
        let (name, place) = title.rsplit_once('@')?;
        let (x, w) = place.split_once('+')?;
        (name == label).then_some((x.parse().ok()?, w.parse().ok()?))
    })
}
```

Ambang `WHITE` 15 000 dari 31 200 piksel (260 x 120) memberi ruang untuk kata dan tepi kaca. Setelah boot pertama, angka yang tercetak dicatat di commit message. Ambang hanya boleh dinaikkan mendekati nilai terukur, tidak pernah diturunkan.

Run: `cargo test appmenu`. Expected: 2 lulus.

- [ ] **Langkah 3: boot**. `fn test_appmenu(arch, args)` disalin dari `compositor/files.rs` dengan perubahan:
- probe script menunggu `hyprctl version`, lalu `exec shelf`;
- `carried.ports.extend(crate::apps::taken(arch, &["shelf"])?)` di atas desktop dari `super::yoru::desktop(arch, args, false)?`;
- urutan di `hook`, memakai `watching.read_more(deadline, |lines| ...)` seperti files.rs:
  1. tunggu baris yang memuat `menu bar: titles` dan `|File@` sekaligus, lalu ambil `title(line, "File")`. Bila tidak ada dalam `SETTLE`, gagal dengan transkrip ("Files did not reach the bar");
  2. `at(qmp, (x + w / 2, 12))`, lalu `press_here(qmp)`, lalu tunggu `menu bar: open File`;
  3. `qmp.screendump(Some(DEVICE_ID), &dump)`, `parse_ppm`, lalu `judge(&image, x)?`;
  4. `at(qmp, (x + 40, 43))`, lalu `press_here(qmp)`. Itu baris pertama: 26 + 5 + 12, sama dengan tes A2 `a_press_opens_a_menu_and_a_pick_does_its_entry`;
  5. tunggu `menu: dispatch appmenu 0x` yang diakhiri ` 1: ok`, lalu `shelf: menu: picked 1`.
- Tambah `mod appmenu;` dan `("appmenu", appmenu::test_appmenu),` di tabel boot `compositor.rs` berdampingan dengan `files`.

```bash
cd /root/ferrix
FERRIX_OVMF_CODE=/usr/share/edk2/x64/OVMF_CODE.4m.fd FERRIX_OVMF_VARS=/usr/share/edk2/x64/OVMF_VARS.4m.fd \
  cargo xtask compositor appmenu 2>&1 | tail -8
```

Expected: baris `appmenu: drop white N (need 15000), dark M (need 200)` lalu PASS.

Perintah tepat untuk menjalankan satu boot dari tabel (`cargo xtask compositor <name>` atau `cargo xtask test compositor --only appmenu`) belum diketahui. Lihat cara boot `files` dijalankan di `docs/DESKTOP.md` atau `--help` xtask, lalu pakai bentuk itu. Boot ini menulis ke `/root/ferrix/build`, jadi pastikan tidak ada agen lain yang sedang boot.

- [ ] **Langkah 4: ukur**

```bash
cd /root/ferrix
FERRIX_OVMF_CODE=/usr/share/edk2/x64/OVMF_CODE.4m.fd FERRIX_OVMF_VARS=/usr/share/edk2/x64/OVMF_VARS.4m.fd \
  cargo xtask measure-desktop --boots 2 2>&1 | tail -12
```

Expected (boot kedua):
- yoru-shell total RSS <= 12 MB,
- hyprix <= 41 MB,
- idle used <= 97 MiB,
- dok tergambar <= 0,8 s setelah graphical.target.

Bila ada yang lewat, berhenti dan laporkan angkanya. Jangan menaikkan anggaran.

- [ ] **Langkah 5: dokumentasi**

`docs/DESKTOP.md`, bagian baru setelah §6.7:

```markdown
### 6.8 Application menus

An application can put its own menus in the menu bar with
`ferrix_appmenu_v1` (`compositor/protocol/protocols/ferrix-appmenu-v1.xml`):
`get_menu` for one of its toplevels, then `add_menu`, `add_item`,
`add_separator` and `commit`. At most 16 menus, 256 entries, 128-byte labels;
anything else is a protocol error. tole wraps it as `Window::set_menu` and
`Window::take_menu_picks`.

hyprix tells socket2 `appmenu>>` when any menu changes. `hyprctl appmenu`
answers the focused window's tree as tab-separated lines (`window`, `menu`,
`item <id> <flags> <label> <keys>`, `line`) or `none`.
`hyprctl dispatch appmenu <address> <id>` sends `activated` to the owner.

The bar puts the application's menus between its name and Window. A window
without its own menus keeps the standard ones. Submenus are not in version 1.

The `appmenu` boot opens Files, checks File reaches the bar, drops it and
picks New Tab.
```

`yoru-shell/README.md`: satu paragraf di "The dock and the menu bar" yang menyatakan bar menampilkan menu milik aplikasi fokus (lewat `hyprctl appmenu`) dan klik item dikirim lewat `dispatch appmenu`.

- [ ] **Langkah 6: commit** (setelah izin pengguna)

```bash
cd /root/ferrix
git add tools/common/xtask/src/compositor.rs tools/common/xtask/src/compositor/appmenu.rs docs/DESKTOP.md
git diff --cached --stat
git commit -m "xtask: the appmenu boot drops Files' File menu from the bar and picks New Tab"
cd /root/ferrix/src/user/apps
git add yoru-shell/README.md && git diff --cached --stat
git commit -m "yoru-shell: the README says the menu bar shows an application's own menus"
```

Tidak ada push. `components.toml` tidak diubah.

---

## Risiko

1. **Cargo.lock kotor.** Hunk kita bisa bersebelahan dengan kenaikan versi yang tidak terkait. `stage_ours` lalu ikut membawa hunk itu utuh. Periksa `git diff --cached -- '*Cargo.lock'` sebelum tiap commit.
2. **Toolchain probe** (wayland-scanner, gcc, pkg-config wayland-client) memang ada di `/usr/sbin`. Yang hilang adalah `/usr/share/wayland-protocols`, sudah ditangani [KEPUTUSAN] 5. Mungkin ada jalur lain yang juga hilang (belum diketahui sampai `interfaces.sh` dijalankan).
3. **Banyak tanda tangan internal belum terbaca rinci:**
   - tanda tangan rantai dispatch server,
   - antrean event,
   - nama field `Events` di `State`,
   - `focused_window`,
   - helper `two_clients`.

   Rencana menandai tiap titik itu dengan "belum diketahui" dan sumber yang harus disalin. Penyesuaian kecil di Task 2 hampir pasti diperlukan.
4. **Biaya per pergantian fokus:** satu request `appmenu` tambahan ke `.socket.sock`, dengan timeout 2 s di `Hypr::ask`. Kalau hyprix sibuk, MenuBar terlambat, tetapi tidak macet.
5. **Event tanpa alamat:** jendela yang tidak fokus mengubah menunya, lalu MenuBar meminta ulang dengan sia-sia. Ini murah dan jarang terjadi (perubahan centang).
6. **Koordinat klik QEMU:** absolut 0x7fff, di-skala dari 1024x768. Baris pertama di y=43 berasal dari tata letak A2. Bila tinggi baris berubah, tes A2 lebih dulu gagal.
7. **Glyph centang** memakai font ikon Nocturne. Bila font itu tidak dimuat di menubar, centang tidak tampak. Tes unit hanya memeriksa data, tidak memeriksa piksel.
8. **RAM:** pohon menu per toplevel kecil (maksimal sekitar 40 KB). yoru-shell bertambah satu `Tree` dan tole memuat crate kecil. Anggaran diukur di Task 6 dan tidak boleh dinaikkan.
9. **`set_menu` mengikat ulang manager tiap panggilan.** Bila aplikasi memanggilnya tiap frame, ada lalu lintas wire tanpa guna. Shelf dan Settings hanya memanggilnya saat centang berubah.

## Self-review

- Spec §4 terpenuhi, kecuali dua penyimpangan yang ditandai: protokol bertahap ([KEPUTUSAN] 1) dan submenu ditunda ([KEPUTUSAN] 2).
- Spec §5: event `appmenu>>` ada, tetapi tanpa data ([KEPUTUSAN] 3). Klik item lewat `dispatch appmenu <address> <id>` sesuai spec.
- Spec §7: pohon invalid menjadi protocol error. Jendela tanpa menu tetap mendapat menu standar. yoru-shell yang restart langsung meminta `appmenu` saat mulai.
- Spec §11:
  - tes kontrol negatif dan batas lebih ada di Task 2,
  - boot `appmenu` ada di Task 6,
  - judge-nya hanya boleh dikencangkan.
- Tidak ada polling. Tidak ada nama atau aset Apple. Tidak ada push atau pin.
- Tiap task diawali tes yang gagal dan diakhiri satu commit per repo/aplikasi.
