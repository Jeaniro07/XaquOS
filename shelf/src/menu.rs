//! Files' own menus in the menu bar.

use crate::app::Action;
use tole::appmenu::{Entry, Item, Menu, Tree};

fn menu(label: &str, entries: Vec<Entry>) -> Menu {
    Menu {
        label: label.to_owned(),
        entries,
    }
}

fn item(id: u32, label: &str, keys: &str) -> Entry {
    Entry::Item(Item::new(id, label).with_keys(keys))
}

/// The menus, with View's checks from `dual` and `hidden`.
#[must_use]
pub fn tree(dual: bool, hidden: bool) -> Tree {
    Tree {
        menus: vec![
            menu(
                "File",
                vec![
                    item(1, "New Tab", "Ctrl T"),
                    item(2, "New Folder", "Ctrl Shift N"),
                    Entry::Line,
                    item(3, "Close Tab", "Ctrl W"),
                ],
            ),
            menu(
                "Edit",
                vec![
                    item(10, "Select All", "Ctrl A"),
                    Entry::Line,
                    item(11, "Rename", "F2"),
                    item(12, "Move to Trash", "Delete"),
                ],
            ),
            menu(
                "View",
                vec![
                    Entry::Item(Item::new(20, "Two Panes").with_check(dual)),
                    Entry::Item(
                        Item::new(21, "Show Hidden Files")
                            .with_keys("Ctrl H")
                            .with_check(hidden),
                    ),
                ],
            ),
            menu(
                "Go",
                vec![
                    item(30, "Back", "Alt Left"),
                    item(31, "Forward", "Alt Right"),
                    item(32, "Enclosing Folder", "Alt Up"),
                ],
            ),
        ],
    }
}

/// What picking `id` does; `focus` is the pane with the keys, `active` the
/// tab shown.
#[must_use]
pub fn action(id: u32, focus: usize, active: usize) -> Option<Action> {
    Some(match id {
        1 => Action::NewTab,
        2 => Action::NewFolder,
        3 => Action::CloseTab(active),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menus_follow_the_window_and_each_id_is_an_action() {
        let tree = tree(true, false);
        let labels: Vec<&str> = tree.menus.iter().map(|menu| menu.label.as_str()).collect();
        assert_eq!(labels, ["File", "Edit", "View", "Go"]);
        assert!(tree.has(20) && tree.has(21));
        assert_eq!(action(1, 0, 0), Some(Action::NewTab));
        assert_eq!(action(3, 1, 2), Some(Action::CloseTab(2)));
        assert_eq!(action(32, 1, 2), Some(Action::Up(1)));
        assert_eq!(action(21, 0, 0), Some(Action::Setting("hidden".to_owned())));
        assert_eq!(action(99, 0, 0), None);
        for menu in &tree.menus {
            for entry in &menu.entries {
                if let Entry::Item(item) = entry {
                    assert!(
                        action(item.id, 0, 0).is_some(),
                        "{} has no action",
                        item.label
                    );
                }
            }
        }
    }
}
