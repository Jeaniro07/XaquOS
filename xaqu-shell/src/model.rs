//! What the dock shows, apart from how: the launchers and whether each
//! one's program has a window, the other programs with windows, and the
//! stash; and the time, as the sidebar and the menu bar say it.

use crate::json::Json;

/// A tile's colour (the dock's markup draws each).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    /// Near black, white glyph.
    Dark = 0,
    /// The theme's accent.
    Accent = 1,
    /// A sky blue.
    Sky = 2,
    /// A cool grey.
    Grey = 3,
}

/// A program with windows that has no launcher.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Other {
    /// Its window class.
    pub class: String,
    /// Its name as shown.
    pub name: String,
    /// Its window used last.
    pub window: String,
}

/// A window class as a name: its last dotted part, the first letter up.
#[must_use]
pub fn shown_class(class: &str) -> String {
    let last = class.rsplit('.').next().unwrap_or(class);
    let mut chars = last.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

impl Launcher {
    /// Whether a window of `class` is this program's.
    #[must_use]
    pub fn is(&self, class: &str) -> bool {
        self.classes
            .iter()
            .any(|wanted| class.eq_ignore_ascii_case(wanted))
    }
}

/// A program the dock starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launcher {
    /// Phosphor's name for its icon.
    pub icon: &'static str,
    /// What the pointer is told.
    pub title: String,
    /// The window classes that are this program.
    pub classes: Vec<String>,
    /// What starts it, as `exec` takes it.
    pub exec: String,
    /// Whether it has a window anywhere, minimized ones included.
    pub running: bool,
    /// Its name in the menu bar.
    pub name: String,
    /// Its tile's colour in the dock.
    pub tile: Tile,
    /// The window of it used last, minimized or not, as `0x..`; `None`
    /// when it has none.
    pub window: Option<String>,
}

/// The time as the sidebar says it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clock {
    /// `14:05`.
    pub time: String,
    /// `Sat 24 Oct`.
    pub date: String,
}

impl Clock {
    /// The clock at `seconds` since 1970, already moved into the local
    /// zone.
    #[must_use]
    pub fn at(seconds: i64) -> Self {
        const DAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let days = seconds.div_euclid(86_400);
        let of_day = seconds.rem_euclid(86_400);
        let (_, month, day) = civil(days);
        let weekday = usize::try_from(days.rem_euclid(7)).unwrap_or(0);
        Clock {
            time: format!("{:02}:{:02}", of_day / 3600, of_day / 60 % 60),
            date: format!(
                "{} {day} {}",
                DAYS.get(weekday).copied().unwrap_or(""),
                MONTHS
                    .get(usize::try_from(month - 1).unwrap_or(0))
                    .copied()
                    .unwrap_or("")
            ),
        }
    }
}

/// The year, month and day of `days` since 1970-01-01 (Howard Hinnant's
/// `civil_from_days`).
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The local zone's offset from UTC in seconds: `TZ` written `UTC+2` or
/// `UTC-5:30`, else none (Xaqu keeps no zone database).
#[must_use]
pub fn offset(tz: Option<&str>) -> i64 {
    let Some(rest) = tz.and_then(|tz| tz.strip_prefix("UTC").or_else(|| tz.strip_prefix("GMT")))
    else {
        return 0;
    };
    let (sign, rest) = match rest.chars().next() {
        Some('+') => (1, &rest[1..]),
        Some('-') => (-1, &rest[1..]),
        _ => return 0,
    };
    let (hours, minutes) = rest.split_once(':').unwrap_or((rest, "0"));
    let hours: i64 = hours.parse().unwrap_or(0);
    let minutes: i64 = minutes.parse().unwrap_or(0);
    sign * (hours * 3600 + minutes * 60)
}

/// Everything the dock shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    /// The launchers.
    pub launchers: Vec<Launcher>,
    /// The programs with windows that have no launcher, by class.
    pub others: Vec<Other>,
    /// How many files the stash holds.
    pub stashed: usize,
}

impl State {
    /// The dock with `launchers` and nothing yet known of the compositor.
    #[must_use]
    pub fn new(launchers: Vec<Launcher>) -> Self {
        State {
            launchers,
            others: Vec::new(),
            stashed: 0,
        }
    }

    /// Which programs have windows, on any desktop or minimized, from
    /// `j/clients`: each launcher's window used last, and the rest.
    pub fn clients(&mut self, list: &Json) {
        let mut windows: Vec<(i64, String, String)> = list
            .items()
            .iter()
            .filter_map(|client| {
                let class = client.get("class").and_then(Json::str)?;
                let address = client.get("address").and_then(Json::str)?;
                let order = client
                    .get("focusHistoryID")
                    .and_then(Json::int)
                    .unwrap_or(i64::MAX);
                (!class.is_empty()).then(|| (order, class.to_owned(), address.to_owned()))
            })
            .collect();
        windows.sort_by_key(|window| window.0);
        for launcher in &mut self.launchers {
            launcher.window = windows
                .iter()
                .find(|(_, class, _)| launcher.is(class))
                .map(|(_, _, address)| address.clone());
            launcher.running = launcher.window.is_some();
        }
        let mut others: Vec<Other> = Vec::new();
        for (_, class, address) in &windows {
            let known = self.launchers.iter().any(|launcher| launcher.is(class))
                || others.iter().any(|other| other.class == *class);
            if !known {
                others.push(Other {
                    class: class.clone(),
                    name: shown_class(class),
                    window: address.clone(),
                });
            }
        }
        others.sort_by(|a, b| a.class.cmp(&b.class));
        self.others = others;
    }
}

/// The dock's launchers: the terminal always, and the browser, Files
/// (shelf) and Settings where the image has them, found as a launcher finds them.
#[must_use]
pub fn launchers(installed: &[(String, String, Option<String>)]) -> Vec<Launcher> {
    let mut out = vec![Launcher {
        icon: "terminal-window",
        title: "Terminal · Super ↵".to_owned(),
        classes: vec!["rocks.magical.term".to_owned(), "foot".to_owned()],
        exec: "/bin/term /bin/zinc".to_owned(),
        running: false,
        name: "Terminal".to_owned(),
        tile: Tile::Dark,
        window: None,
    }];
    let find = |words: &[&str]| {
        installed.iter().find(|(id, _, _)| {
            let id = id.to_lowercase();
            words.iter().any(|word| id.contains(word))
        })
    };
    if let Some((_, exec, class)) = find(&["chrome", "chromium", "firefox"]) {
        out.push(Launcher {
            icon: "globe-simple",
            title: "Browser".to_owned(),
            name: "Browser".to_owned(),
            tile: Tile::Sky,
            window: None,
            classes: class
                .iter()
                .cloned()
                .chain(["google-chrome", "chromium", "firefox"].map(str::to_owned))
                .collect(),
            exec: exec.clone(),
            running: false,
        });
    }
    if let Some((_, exec, class)) = find(&["shelf"]) {
        out.push(Launcher {
            icon: "folder-simple",
            title: "Files".to_owned(),
            name: "Files".to_owned(),
            tile: Tile::Accent,
            window: None,
            classes: class
                .iter()
                .cloned()
                .chain(std::iter::once("rocks.magical.shelf".to_owned()))
                .collect(),
            exec: exec.clone(),
            running: false,
        });
    }
    if let Some((_, exec, class)) = find(&["settings"]) {
        out.push(Launcher {
            icon: "gear-six",
            title: "Settings".to_owned(),
            name: "Settings".to_owned(),
            tile: Tile::Grey,
            window: None,
            classes: class
                .iter()
                .cloned()
                .chain(std::iter::once("rocks.magical.settings".to_owned()))
                .collect(),
            exec: exec.clone(),
            running: false,
        });
    }
    out
}

/// The Phosphor icon for an application, from its desktop-file id and its
/// window class: the dock's mapping, widened. Empty for the letter tile.
#[must_use]
pub fn icon_for(id: &str, class: Option<&str>) -> &'static str {
    let mut both = id.to_lowercase();
    if let Some(class) = class {
        both.push(' ');
        both.push_str(&class.to_lowercase());
    }
    let has = |words: &[&str]| words.iter().any(|word| both.contains(word));
    if has(&["chrome", "chromium", "firefox"]) {
        "globe-simple"
    } else if has(&["shelf", "nautilus", "finder"]) {
        "folder-simple"
    } else if has(&["settings"]) {
        "gear-six"
    } else if has(&["term", "console"]) {
        "terminal-window"
    } else {
        ""
    }
}

/// A desktop entry's `Exec` without its field codes (`%U`, `%f`).
#[must_use]
pub fn command(exec: &str) -> String {
    exec.split_whitespace()
        .filter(|word| !(word.len() == 2 && word.starts_with('%')))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::{Clock, Other, State, command, launchers, offset, shown_class};
    use crate::json::Json;

    #[test]
    fn running_wherever_the_windows_are() {
        let mut state = State::new(launchers(&[(
            "settings.desktop".to_owned(),
            "settings".to_owned(),
            None,
        )]));
        let list = Json::parse(
            r#"[
              {"address":"0xa","class":"foot","workspace":{"id":2,"name":"2"},"focusHistoryID":1},
              {"address":"0xb","class":"foot","workspace":{"id":-99,"name":"special:minimized"},"focusHistoryID":0},
              {"address":"0xc","class":"org.example.Notes","workspace":{"id":1,"name":"1"},"focusHistoryID":2},
              {"address":"0xd","class":"org.example.Notes","workspace":{"id":3,"name":"3"},"focusHistoryID":3}
            ]"#,
        )
        .expect("json");
        state.clients(&list);
        let term = &state.launchers[0];
        assert_eq!(term.name, "Terminal");
        assert!(term.running, "a window on desktop 2 counts");
        assert_eq!(
            term.window.as_deref(),
            Some("0xb"),
            "the one used last, minimized or not"
        );
        assert!(!state.launchers[1].running, "Settings has no window");
        assert_eq!(
            state.others,
            vec![Other {
                class: "org.example.Notes".to_owned(),
                name: "Notes".to_owned(),
                window: "0xc".to_owned(),
            }]
        );
        assert_eq!(shown_class("rocks.magical.shelf"), "Shelf");
        assert_eq!(shown_class("foot"), "Foot");
    }

    /// The design's clock and date.
    #[test]
    fn the_clock() {
        // 2026-10-24 14:05:00 UTC, a Saturday.
        let clock = Clock::at(1_792_850_700);
        assert_eq!(clock.time, "14:05");
        assert_eq!(clock.date, "Sat 24 Oct");
        assert_eq!(Clock::at(0).date, "Thu 1 Jan");
        assert_eq!(offset(Some("UTC+2")), 7200);
        assert_eq!(offset(Some("UTC-5:30")), -19800);
        assert_eq!(offset(Some("Europe/Berlin")), 0);
    }

    /// The browser and Settings when installed, field codes dropped.
    #[test]
    fn launchers_found() {
        let found = launchers(&[
            (
                "google-chrome.desktop".to_owned(),
                command("/bin/chrome --new-window %U"),
                None,
            ),
            ("settings.desktop".to_owned(), command("settings"), None),
            ("shelf.desktop".to_owned(), command("shelf %U"), None),
        ]);
        let titles: Vec<&str> = found.iter().map(|l| l.title.as_str()).collect();
        assert_eq!(
            titles,
            ["Terminal · Super ↵", "Browser", "Files", "Settings"]
        );
        assert_eq!(found[1].exec, "/bin/chrome --new-window");
    }
}
