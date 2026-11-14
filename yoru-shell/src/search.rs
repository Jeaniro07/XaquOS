//! What Spotlight matches: the query read as arithmetic first, then the
//! query against applications, open windows and the names under ~.

/// At most this many hits, one row each.
pub const SHOWN: usize = 9;

/// One application, as Spotlight shows it.
#[derive(Clone, Debug)]
pub struct App {
    /// The exec line, the `%` codes stripped.
    pub exec: String,
    /// The words shown.
    pub title: String,
    /// The Phosphor icon, or empty for the letter tile.
    pub icon: &'static str,
}

/// One result of a query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    /// An application, run by its exec line.
    App {
        /// The exec line to dispatch.
        exec: String,
        /// The words shown.
        title: String,
        /// The Phosphor icon, or empty for the letter tile.
        icon: &'static str,
    },
    /// A window, focused by its address.
    Window {
        /// The window's address, as `j/clients` prints it.
        address: String,
        /// The window's title.
        title: String,
    },
    /// A file under ~, relative to home, opened by its default handler.
    File {
        /// The path as it is shown.
        path: String,
    },
    /// The query read as arithmetic.
    Value(String),
}

/// The hits for `query`, best first, at most [`SHOWN`]: the query read as
/// arithmetic first, then applications (a prefix beats a middle match),
/// windows, files. Apps are `installed()`'s list, windows `j/clients`'s,
/// files [`scan`]'s.
#[must_use]
pub fn hits(query: &str, apps: &[App], windows: &[(String, String)], files: &[String]) -> Vec<Hit> {
    let query = query.trim();
    let mut found = Vec::new();
    if query.is_empty() {
        return found;
    }
    if let Some(answer) = value(query) {
        found.push(Hit::Value(answer));
    }
    let folded = query.to_lowercase();
    // (where it matches, which kind, the words for the order, the hit).
    let mut ranked: Vec<(usize, u8, String, Hit)> = Vec::new();
    for app in apps {
        let Some(at) = app
            .title
            .to_lowercase()
            .find(&folded)
            .or_else(|| app.exec.to_lowercase().find(&folded))
        else {
            continue;
        };
        ranked.push((
            at,
            0,
            app.title.clone(),
            Hit::App {
                exec: app.exec.clone(),
                title: app.title.clone(),
                icon: app.icon,
            },
        ));
    }
    for (address, title) in windows {
        let Some(at) = title.to_lowercase().find(&folded) else {
            continue;
        };
        ranked.push((
            at,
            1,
            title.clone(),
            Hit::Window {
                address: address.clone(),
                title: title.clone(),
            },
        ));
    }
    for path in files {
        let Some(at) = path.to_lowercase().find(&folded) else {
            continue;
        };
        ranked.push((at, 2, path.clone(), Hit::File { path: path.clone() }));
    }
    ranked.sort_by(|(a_at, a_kind, a_title, _), (b_at, b_kind, b_title, _)| {
        (a_kind, a_at, a_title).cmp(&(b_kind, b_at, b_title))
    });
    found.extend(ranked.into_iter().map(|(_, _, _, hit)| hit));
    found.truncate(SHOWN);
    found
}

/// Files under `home`, as paths relative to it, at most `max` names and
/// `budget_ns` nanoseconds of walking, whichever stops first: a folder
/// before its children, folders and files together, dot names skipped,
/// symlinks never descended into. The order is the walk's, made stable by
/// sorting each folder's entries by name.
// Ponytail: no index and no cache; scanned again every time it opens.
// Upgrade path: a mtime-kept cache when a profile shows the walk is slow.
#[must_use]
pub fn scan(home: &std::path::Path, max: usize, budget_ns: u128) -> Vec<String> {
    let started = std::time::Instant::now();
    let mut found: Vec<String> = Vec::new();
    let mut stack = vec![home.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if found.len() >= max || started.elapsed().as_nanos() >= budget_ns {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut children: Vec<(std::path::PathBuf, bool)> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().into_string().ok()?;
                if name.starts_with('.') {
                    return None;
                }
                // `DirEntry::metadata` is lstat: a link to a folder is not
                // a folder, so it is never descended into.
                let folder = entry.metadata().ok()?.is_dir();
                Some((entry.path(), folder))
            })
            .collect();
        children.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));
        for (path, folder) in children {
            if found.len() >= max || started.elapsed().as_nanos() >= budget_ns {
                break;
            }
            let Ok(relative) = path.strip_prefix(home) else {
                continue;
            };
            let Some(text) = relative.to_str() else {
                continue;
            };
            let text = text.to_owned();
            if folder {
                stack.push(path);
            }
            found.push(text);
        }
    }
    found
}

/// The query read as arithmetic: `+ - * /`, parentheses, a unary minus,
/// integers and decimals. `None` for anything else, a zero divisor or a
/// number too large to be exact. The answer: a whole number without a
/// point, anything else with up to six decimals and no trailing zeros.
#[must_use]
pub fn value(query: &str) -> Option<String> {
    let mut chars = query.chars().peekable();
    let total = sum(&mut chars)?;
    skip(&mut chars);
    if chars.next().is_some() || !total.is_finite() {
        return None;
    }
    if total == total.trunc() && total.abs() < 1e15 {
        return Some(format!("{total:.0}"));
    }
    let mut text = format!("{total:.6}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    Some(text)
}

/// The query's characters, looked at one ahead.
type Chars<'a> = std::iter::Peekable<std::str::Chars<'a>>;

/// `expr`: `term (('+' | '-') term)*`.
fn sum(chars: &mut Chars<'_>) -> Option<f64> {
    let mut left = product(chars)?;
    loop {
        skip(chars);
        match chars.peek() {
            Some('+') => {
                chars.next();
                left += product(chars)?;
            }
            Some('-') => {
                chars.next();
                left -= product(chars)?;
            }
            _ => return Some(left),
        }
    }
}

/// `term`: `factor (('*' | '/') factor)*`.
fn product(chars: &mut Chars<'_>) -> Option<f64> {
    let mut left = factor(chars)?;
    loop {
        skip(chars);
        match chars.peek() {
            Some('*') => {
                chars.next();
                left *= factor(chars)?;
            }
            Some('/') => {
                chars.next();
                let right = factor(chars)?;
                if right == 0.0 {
                    return None;
                }
                left /= right;
            }
            _ => return Some(left),
        }
    }
}

/// `factor`: `'-' factor | '(' expr ')' | number`.
fn factor(chars: &mut Chars<'_>) -> Option<f64> {
    skip(chars);
    match chars.peek()? {
        '-' => {
            chars.next();
            factor(chars).map(|inner| -inner)
        }
        '(' => {
            chars.next();
            let inner = sum(chars)?;
            skip(chars);
            if chars.next() != Some(')') {
                return None;
            }
            Some(inner)
        }
        '0'..='9' | '.' => number(chars),
        _ => None,
    }
}

/// A run of digits with at most one point, read as a number.
fn number(chars: &mut Chars<'_>) -> Option<f64> {
    let mut text = String::new();
    let mut point = false;
    while let Some(c) = chars.peek() {
        match c {
            '0'..='9' => text.push(*c),
            '.' if !point => {
                point = true;
                text.push('.');
            }
            _ => break,
        }
        chars.next();
    }
    // A point with nothing after it is not a number yet: `1.` typed on
    // its own shows no answer until `1.5` is there.
    if text.is_empty() || text == "." || text.ends_with('.') {
        return None;
    }
    text.parse().ok()
}

/// Spaces before the next token.
fn skip(chars: &mut Chars<'_>) {
    while matches!(chars.peek(), Some(' ')) {
        chars.next();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apps() -> Vec<App> {
        vec![
            App {
                exec: "/bin/shelf".to_owned(),
                title: "Files".to_owned(),
                icon: "folder",
            },
            App {
                exec: "/bin/chrome".to_owned(),
                title: "Chrome for Testing".to_owned(),
                icon: "globe",
            },
            App {
                exec: "/bin/term /bin/zinc".to_owned(),
                title: "Terminal".to_owned(),
                icon: "terminal-window",
            },
        ]
    }

    #[test]
    fn the_calculator_answer_comes_first() {
        let found = hits("1+2*3", &apps(), &[], &[]);
        assert_eq!(found.first(), Some(&Hit::Value("7".to_owned())));
        assert!(hits("2/0", &apps(), &[], &[]).is_empty());
    }

    #[test]
    fn app_prefix_beats_app_middle() {
        let found = hits("te", &apps(), &[], &[]);
        assert_eq!(
            found.first(),
            Some(&Hit::App {
                exec: "/bin/term /bin/zinc".to_owned(),
                title: "Terminal".to_owned(),
                icon: "terminal-window",
            })
        );
        let found = hits("o", &apps(), &[], &[]);
        assert!(
            matches!(found.first(), Some(Hit::App { title, .. }) if title == "Chrome for Testing")
        );
    }

    #[test]
    fn windows_come_as_their_title() {
        let found = hits(
            "notes",
            &apps(),
            &[("0x9".to_owned(), "notes.md — Files".to_owned())],
            &[],
        );
        assert_eq!(
            found.first(),
            Some(&Hit::Window {
                address: "0x9".to_owned(),
                title: "notes.md — Files".to_owned(),
            })
        );
    }

    #[test]
    fn files_match_from_anywhere_in_the_name() {
        let found = hits("tar", &apps(), &[], &["profile.tar.gz".to_owned()]);
        assert_eq!(
            found.first(),
            Some(&Hit::File {
                path: "profile.tar.gz".to_owned()
            })
        );
    }

    #[test]
    fn at_most_nine_hits() {
        let many: Vec<App> = (0..40)
            .map(|n| App {
                exec: String::new(),
                title: format!("app{n}word"),
                icon: "",
            })
            .collect();
        assert_eq!(hits("word", &many, &[], &[]).len(), SHOWN);
    }

    #[test]
    fn the_calculator_reads_parentheses_negatives_and_decimals() {
        for (query, want) in [
            ("(1+2)*3", "9"),
            ("-4+1", "-3"),
            ("1/4", "0.25"),
            ("2.5*2", "5"),
            ("2.5*1.5", "3.75"),
            ("10/4", "2.5"),
            ("1 -2", "-1"),
            (".5+.5", "1"),
        ] {
            assert_eq!(value(query).as_deref(), Some(want), "{query}");
        }
        for bad in ["2/0", "1+", "(1", "abc", "1 2", "1.", "2.5.1"] {
            assert_eq!(value(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_scan_respects_the_count_bound_and_skips_dots() {
        let base = std::env::temp_dir().join(format!("yoru-search-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("a/b/c")).expect("made");
        std::fs::write(base.join("a/notes.md"), b"x").expect("written");
        std::fs::write(base.join("a/.secret"), b"x").expect("written");
        std::fs::write(base.join("a/b/plan.md"), b"x").expect("written");
        let found = scan(&base, 5000, 5_000_000_000);
        assert_eq!(
            found,
            vec!["a", "a/b", "a/notes.md", "a/b/c", "a/b/plan.md",]
        );
        assert_eq!(
            scan(&base, 1, 5_000_000_000),
            vec!["a"],
            "the count bound cuts"
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
