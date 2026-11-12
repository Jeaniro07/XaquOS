//! Settings' own menu: one View item per page, the shown page checked.

use crate::pages::{Nav, Page};
use crate::ui::Action;
use tole::appmenu::{Entry, Item, Menu, Tree};

/// The first page's item id; the others follow in `Page::ALL` order.
const FIRST: u32 = 100;

/// The View menu with `shown` checked.
#[must_use]
pub fn tree(shown: Page) -> Tree {
    let entries = Page::ALL
        .iter()
        .zip(FIRST..)
        .map(|(page, id)| Entry::Item(Item::new(id, page.label()).with_check(*page == shown)))
        .collect();
    Tree {
        menus: vec![Menu {
            label: "View".to_owned(),
            entries,
        }],
    }
}

/// The page `id` opens.
#[must_use]
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
        let [view] = tree.menus.as_slice() else {
            panic!("one menu")
        };
        assert_eq!(view.label, "View");
        assert_eq!(view.entries.len(), Page::ALL.len());
        assert!(view.entries.iter().any(
            |entry| matches!(entry, Entry::Item(item) if item.checked && item.label == "Power")
        ));
        assert_eq!(
            view.entries
                .iter()
                .filter(|entry| matches!(entry, Entry::Item(item) if item.checked))
                .count(),
            1
        );
        assert!(matches!(
            action(FIRST),
            Some(Action::Navigate(Nav::Page(Page::Displays)))
        ));
        assert!(action(FIRST + 13).is_none());
        assert!(action(5).is_none());
    }
}
