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
        Self {
            id,
            label: label.to_owned(),
            keys: String::new(),
            enabled: true,
            checked: false,
        }
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
            Self::BadLabel => {
                "a label is empty, longer than 128 bytes or holds a control character"
            }
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
        self.menus
            .iter()
            .flat_map(|menu| &menu.entries)
            .any(|entry| matches!(entry, Entry::Item(item) if item.id == id && item.enabled))
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
                self.menus.push(Menu {
                    label: label.clone(),
                    entries: Vec::new(),
                });
            }
            Step::Item {
                id,
                label,
                keys,
                flags,
            } => {
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
                return Ok(Some(Tree {
                    menus: std::mem::take(&mut self.menus),
                }));
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
        let _ = builder.apply(&step).ok()?;
    }
    Some((address, builder.apply(&Step::Commit).ok()??))
}

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
        let mut steps: Vec<Step> = (0..=MAX_MENUS)
            .map(|n| Step::Menu(format!("M{n}")))
            .collect();
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
            assert_eq!(
                built(&[Step::Menu(label.to_owned())]),
                Err(Wrong::BadLabel),
                "{label:?}"
            );
        }
        assert_eq!(built(&[Step::Menu("x".repeat(MAX_LABEL))]), Ok(None));
    }

    #[test]
    fn an_item_before_any_menu_is_a_bad_tree() {
        assert_eq!(built(&[Step::Line]), Err(Wrong::BadTree));
    }
}
