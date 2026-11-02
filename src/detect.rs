//! Turning "whatever window is in front" into "the app profile to show".

use crate::data::{AppProfile, Catalog};

/// The outcome of looking at the foreground window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Context {
    /// We know this app and have shortcuts for it.
    Known(String),
    /// We know this app - it is on the roster the target loads - but this build ships no rows
    /// for it. The sheet can still name the window and show the empty state, which is a
    /// better answer than nothing happening at all.
    Empty(String),
    /// The window belongs to a process, but it is not in the dataset.
    Unsupported(Option<String>),
    /// Could not tell - desktop, shell overlay, or a dead window.
    Unknown,
}

/// Normalise a process image name for matching. Windows reports the real file name, which
/// differs from the product name (`Code.exe` vs "Visual Studio Code"), and some launchers
/// reuse a host binary (`chrome.exe` under a branded name).
fn normalise(exe: &str) -> String {
    exe.trim().to_ascii_lowercase()
}

/// Resolve the foreground executable to a profile.
///
/// Two lookups, not one: a profile answers with its rows, and a roster entry answers with a
/// name and a category only. Which shelf a process sits on is the catalogue's business; what
/// to paint is ours.
pub fn resolve(catalog: &Catalog, exe: Option<&str>) -> Context {
    let Some(raw) = exe else { return Context::Unknown };
    let exe = normalise(raw);
    if exe.is_empty() {
        return Context::Unknown;
    }
    if let Some(app) = catalog.find_by_exe(&exe) {
        return Context::Known(app.name.clone());
    }
    if let Some(known) = catalog.known_app(&exe) {
        return Context::Empty(known.name.clone());
    }
    Context::Unsupported(Some(exe))
}

/// Flatten a profile into (collection index, shortcut index) pairs, skipping tombstones. The
/// overlay and the hub both list rows through this, so a removed shortcut cannot leak into one
/// of them. Indices rather than names: the search needs the row itself to read its aliases, and
/// cloning every command name in the profile to filter it is work the show path should not do.
pub fn visible_rows(app: &AppProfile) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (ci, col) in app.collections.iter().enumerate() {
        for (si, s) in col.shortcuts.iter().enumerate() {
            if !s.removed {
                out.push((ci, si));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        // A temp dir with no user files: Catalog::load falls back to the built-in dataset.
        let dir = std::env::temp_dir().join(format!("ss-detect-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Catalog::load(&dir).expect("builtin catalog")
    }

    #[test]
    fn matches_the_real_windows_binary_names() {
        let c = catalog();
        assert_eq!(resolve(&c, Some("Code.exe")), Context::Known("Visual Studio Code".into()));
        assert_eq!(resolve(&c, Some("chrome.exe")), Context::Known("Google Chrome".into()));
        assert_eq!(resolve(&c, Some("explorer.exe")), Context::Known("File Explorer".into()));
    }

    #[test]
    fn matching_is_case_insensitive_and_trims() {
        let c = catalog();
        assert_eq!(resolve(&c, Some("  CHROME.EXE ")), Context::Known("Google Chrome".into()));
    }

    #[test]
    fn unknown_app_is_reported_as_unsupported_not_silently_ignored() {
        let c = catalog();
        assert_eq!(resolve(&c, Some("somegame.exe")), Context::Unsupported(Some("somegame.exe".into())));
    }

    /// An app on the roster but with no shortcut file is the common case, not the odd one: the
    /// target loads around ninety profiles and this build ships rows for a fraction of them.
    #[test]
    fn a_roster_app_resolves_by_name_even_though_it_has_no_rows() {
        let c = catalog();
        assert_eq!(resolve(&c, Some("slack.exe")), Context::Empty("Slack".into()));
        assert_eq!(resolve(&c, Some("REAPER.EXE ")), Context::Empty("Reaper".into()));
        // It is not unsupported: the user is in an app we know, so the sheet may say so.
        assert!(!matches!(resolve(&c, Some("slack.exe")), Context::Unsupported(_)));
    }

    #[test]
    fn a_profile_always_outranks_the_roster_for_the_same_app() {
        let c = catalog();
        // Nothing in the dataset may be on both shelves; if it were, an app with rows would
        // resolve as an empty panel and the rows would become invisible.
        for app in &c.apps {
            assert_eq!(
                resolve(&c, app.exe.first().map(String::as_str)),
                Context::Known(app.name.clone()),
                "{} is a profile and must never resolve as empty",
                app.name
            );
        }
    }

    #[test]
    fn a_missing_process_name_is_unknown() {
        let c = catalog();
        assert_eq!(resolve(&c, None), Context::Unknown);
        assert_eq!(resolve(&c, Some("")), Context::Unknown);
        assert_eq!(resolve(&c, Some("   ")), Context::Unknown);
    }

    #[test]
    fn every_visible_row_points_at_a_real_shortcut_with_a_firing_combo() {
        let c = catalog();
        for app in &c.apps {
            let rows = visible_rows(app);
            assert!(!rows.is_empty(), "{} has no usable shortcuts", app.name);
            for (ci, si) in &rows {
                let s = &app.collections[*ci].shortcuts[*si];
                assert!(!s.command.is_empty(), "{} / row {} has no name", app.name, si);
                assert!(s.combo().is_ok(), "{} / {} has an unfireable combo '{}'", app.name, s.command, s.keys);
            }
        }
    }

    #[test]
    fn the_dataset_covers_the_apps_the_brief_named() {
        let c = catalog();
        // The task called out chrome.exe, code.exe and photoshop.exe specifically.
        for exe in ["chrome.exe", "code.exe", "photoshop.exe"] {
            assert!(
                matches!(resolve(&c, Some(exe)), Context::Known(_)),
                "{exe} should be supported"
            );
        }
    }

    #[test]
    fn a_context_switch_changes_the_rows_on_screen() {
        // Alt-tab Chrome -> VS Code, hold again: the second sheet must list the editor, not
        // the browser. This is the resolution half of that; the overlay rebuilds from
        // whatever visible_rows returns for the newly resolved profile.
        let c = catalog();
        let collections_for = |exe: Option<&str>| match resolve(&c, exe) {
            Context::Known(name) => {
                let app = c.app(&name).unwrap();
                let mut cols: Vec<String> =
                    visible_rows(app).into_iter().map(|(ci, _)| app.collections[ci].name.clone()).collect();
                cols.dedup();
                cols
            }
            _ => vec![],
        };
        let chrome = collections_for(Some("chrome.exe"));
        let code = collections_for(Some("Code.exe"));
        assert!(!chrome.is_empty() && !code.is_empty(), "both apps have collections");
        assert_ne!(chrome, code, "a context switch must change what is listed");
        assert!(
            collections_for(Some("mystery.exe")).is_empty(),
            "an unsupported app lists nothing rather than the previous app's rows"
        );
    }
}
