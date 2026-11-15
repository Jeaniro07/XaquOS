//! The menu bar's menus (spec §5): the mark's, the focused program's and
//! Window, as data. `menubar.rs` draws them and does what is picked.
//! A program's own menus come with `ferrix_appmenu_v1` and go between its
//! name and Window (`with_app`).

use crate::json::Json;
use crate::model::Launcher;

/// The desktops Window's "Move to" offers.
pub const DESKTOPS: u32 = 4;
/// The mark at the bar's left: words until it is drawn (docs/DESKTOP.md §6.4).
pub const MARK: &str = "FX";
/// The program menu's title when no window has the focus.
pub const NOBODY: &str = "Desktop";

/// What an entry does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Does {
    /// A dispatcher, as `dispatch` takes it.
    Dispatch(String),
    /// The focused program's windows on this desktop minimized.
    Hide,
}

/// A line of a menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// Words, the keys that do the same, and what it does; `None` is shown
    /// but cannot be picked.
    Item {
        /// Its words.
        label: String,
        /// The keys, as shown.
        keys: String,
        /// Whether it shows a check mark.
        checked: bool,
        /// What picking it does.
        does: Option<Does>,
    },
    /// A divider.
    Line,
}

/// A menu: its title in the bar and its lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    /// The title.
    pub label: String,
    /// Drawn bold, as the mark's and the program's are.
    pub bold: bool,
    /// Its lines.
    pub entries: Vec<Entry>,
}

/// The window with the focus.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Focused {
    /// Its address, `0x..`.
    pub address: String,
    /// Its class.
    pub class: String,
}

/// The focused window from `j/activewindow`; `None` when none has it.
#[must_use]
pub fn focused(json: &Json) -> Option<Focused> {
    let address = json.get("address").and_then(Json::str)?;
    if address.is_empty() {
        return None;
    }
    let class = json.get("class").and_then(Json::str).unwrap_or_default();
    Some(Focused {
        address: address.to_owned(),
        class: class.to_owned(),
    })
}

/// A program's name: its launcher's, or its class shown.
#[must_use]
pub fn app_name(class: &str, launchers: &[Launcher]) -> String {
    launchers
        .iter()
        .find(|launcher| launcher.is(class))
        .map_or_else(
            || crate::model::shown_class(class),
            |launcher| launcher.name.clone(),
        )
}

fn item(label: &str, keys: &str, does: Option<Does>) -> Entry {
    Entry::Item {
        label: label.to_owned(),
        keys: keys.to_owned(),
        checked: false,
        does,
    }
}

fn dispatch(what: &str) -> Option<Does> {
    Some(Does::Dispatch(what.to_owned()))
}

/// The bar's menus with `app` focused (`None`: no window has the focus);
/// `own` is this program's path, for its switcher and power menu.
#[must_use]
pub fn standard(app: Option<&str>, own: &str) -> Vec<Menu> {
    let wallpaper = item(
        "Wallpaper…",
        "Super I",
        dispatch(&format!("exec {own} --walls")),
    );
    let mut menus = vec![Menu {
        label: MARK.to_owned(),
        bold: true,
        entries: vec![
            item("About Ferrix", "", None),
            Entry::Line,
            item("Settings…", "", dispatch("exec settings")),
            wallpaper.clone(),
            Entry::Line,
            item(
                "Restart or Shut Down…",
                "",
                dispatch(&format!("exec {own} --power")),
            ),
        ],
    }];
    let Some(app) = app else {
        menus.push(Menu {
            label: NOBODY.to_owned(),
            bold: true,
            entries: vec![wallpaper],
        });
        return menus;
    };
    menus.push(Menu {
        label: app.to_owned(),
        bold: true,
        entries: vec![
            item(&format!("About {app}"), "", None),
            Entry::Line,
            item(&format!("Hide {app}"), "", Some(Does::Hide)),
            Entry::Line,
            item(&format!("Quit {app}"), "Super Q", dispatch("killactive")),
        ],
    });
    let mut window = vec![
        item("Minimize", "Super M", dispatch("minimize")),
        item("Full Screen", "Super F", dispatch("fullscreen")),
        Entry::Line,
    ];
    for n in 1..=DESKTOPS {
        window.push(item(
            &format!("Move to Desktop {n}"),
            "",
            dispatch(&format!("movetoworkspace {n}")),
        ));
    }
    window.push(Entry::Line);
    window.push(item("Close", "Super W", dispatch("killactive")));
    menus.push(Menu {
        label: "Window".to_owned(),
        bold: false,
        entries: window,
    });
    menus
}

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
                    does: item
                        .enabled
                        .then(|| Does::Dispatch(format!("appmenu {address} {}", item.id))),
                },
            })
            .collect(),
    });
    let at = menus
        .iter()
        .position(|menu| menu.label == "Window")
        .unwrap_or(menus.len());
    let _ = menus.splice(at..at, own);
    menus
}

/// What hides `class` on desktop `desktop`: each of its windows there
/// focused, then minimized. Spec §5 says `movetoworkspacesilent
/// special:minimized`, which would pass by compositor-layout's own record
/// of minimized windows, so a window hidden that way would not come back
/// when focused; `minimize` keeps it.
#[must_use]
pub fn hide(clients: &Json, class: &str, desktop: i64) -> Vec<String> {
    clients
        .items()
        .iter()
        .filter(|client| {
            client.get("class").and_then(Json::str) == Some(class)
                && client
                    .get("workspace")
                    .and_then(|workspace| workspace.get("id"))
                    .and_then(Json::int)
                    == Some(desktop)
        })
        .filter_map(|client| client.get("address").and_then(Json::str))
        .flat_map(|address| {
            [
                format!("focuswindow address:{address}"),
                "minimize".to_owned(),
            ]
        })
        .collect()
}

/// Where each title sits: from 10, each as wide as its words
/// (`width_of(label, bold)`) and 18 more; `(x, w)` per menu.
pub fn place(menus: &[Menu], mut width_of: impl FnMut(&str, bool) -> f32) -> Vec<(f32, f32)> {
    let mut x = 10.0;
    menus
        .iter()
        .map(|menu| {
            let w = width_of(&menu.label, menu.bold) + 18.0;
            let at = (x, w);
            x += w;
            at
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn does(menu: &Menu, label: &str) -> Option<Does> {
        menu.entries.iter().find_map(|entry| match entry {
            Entry::Item { label: l, does, .. } if l == label => does.clone(),
            _ => None,
        })
    }

    #[test]
    fn a_focused_program_has_its_menu_and_window() {
        let menus = standard(Some("Terminal"), "/bin/xaqu-shell");
        let titles: Vec<&str> = menus.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(titles, ["FX", "Terminal", "Window"]);
        assert_eq!(
            does(&menus[1], "Quit Terminal"),
            Some(Does::Dispatch("killactive".to_owned()))
        );
        assert_eq!(does(&menus[1], "Hide Terminal"), Some(Does::Hide));
        assert_eq!(does(&menus[1], "About Terminal"), None);
        assert_eq!(
            does(&menus[2], "Minimize"),
            Some(Does::Dispatch("minimize".to_owned()))
        );
        assert_eq!(
            does(&menus[2], "Full Screen"),
            Some(Does::Dispatch("fullscreen".to_owned()))
        );
        assert_eq!(
            does(&menus[2], "Move to Desktop 3"),
            Some(Does::Dispatch("movetoworkspace 3".to_owned()))
        );
        assert_eq!(
            does(&menus[2], "Close"),
            Some(Does::Dispatch("killactive".to_owned()))
        );
        assert_eq!(
            does(&menus[0], "Wallpaper…"),
            Some(Does::Dispatch("exec /bin/xaqu-shell --walls".to_owned()))
        );
    }

    #[test]
    fn with_nothing_focused_the_desktop_has_its_own() {
        let menus = standard(None, "/bin/xaqu-shell");
        let titles: Vec<&str> = menus.iter().map(|m| m.label.as_str()).collect();
        assert_eq!(titles, ["FX", "Desktop"]);
    }

    #[test]
    fn the_focus_and_the_names() {
        let json = Json::parse(r#"{"address":"0x5","class":"foot"}"#).expect("json");
        assert_eq!(
            focused(&json),
            Some(Focused {
                address: "0x5".to_owned(),
                class: "foot".to_owned()
            })
        );
        assert_eq!(focused(&Json::parse("{}").expect("json")), None);
        let launchers = crate::model::launchers(&[]);
        assert_eq!(app_name("foot", &launchers), "Terminal");
        assert_eq!(app_name("org.example.Notes", &launchers), "Notes");
    }

    #[test]
    fn hide_minimizes_the_programs_windows_on_this_desktop_only() {
        let clients = Json::parse(
            r#"[
              {"address":"0x1","class":"foot","workspace":{"id":2}},
              {"address":"0x2","class":"foot","workspace":{"id":3}},
              {"address":"0x3","class":"other","workspace":{"id":2}},
              {"address":"0x4","class":"foot","workspace":{"id":-99}}
            ]"#,
        )
        .expect("json");
        assert_eq!(
            hide(&clients, "foot", 2),
            ["focuswindow address:0x1", "minimize"]
        );
    }

    #[test]
    fn titles_sit_one_after_another() {
        let menus = standard(Some("Files"), "/x");
        let places = place(&menus, |label, _| label.len() as f32 * 10.0);
        assert_eq!(places, [(10.0, 38.0), (48.0, 68.0), (116.0, 78.0)]);
    }

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
        let menus = with_app(standard(Some("Files"), "/x"), &tree, "0xab");
        let labels: Vec<&str> = menus.iter().map(|menu| menu.label.as_str()).collect();
        assert_eq!(labels, [MARK, "Files", "File", "Window"]);
        let file = &menus[2];
        assert!(matches!(&file.entries[0],
            Entry::Item { label, keys, checked: true, does: Some(Does::Dispatch(what)) }
            if label == "New Tab" && keys == "Ctrl T" && what == "appmenu 0xab 1"));
        assert!(matches!(file.entries[1], Entry::Line));
        assert!(matches!(&file.entries[2], Entry::Item { does: None, .. }));
    }
}
