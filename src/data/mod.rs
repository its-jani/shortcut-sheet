//! Data model, storage and the compiled-in dataset.

pub mod key;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub use key::{Combo, Key};

/// One command and the chord that fires it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcut {
    pub command: String,
    /// Human-written chord, e.g. `"Ctrl+Shift+P"`. Parsed at load, never at paint time.
    /// Optional so a tombstone can be written as `{ "command": "...", "removed": true }`.
    #[serde(default)]
    pub keys: String,
    /// Other names the same command goes by, matched by the search the way `command` is.
    /// "Developer tools" and "Inspect element" are one command with two names, and which one
    /// a user types depends on what they were taught - so both have to find it.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Tombstone: a user file can delete a built-in entry instead of shadowing it.
    #[serde(default)]
    pub removed: bool,
}

impl Shortcut {
    pub fn combo(&self) -> Result<Combo, key::ComboError> {
        Combo::parse(&self.keys)
    }
}

/// A named group of shortcuts - the row you see in S04 and the header you see in S01.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub name: String,
    #[serde(default)]
    pub shortcuts: Vec<Shortcut>,
}

/// An application the overlay knows how to serve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppProfile {
    pub name: String,
    /// One of the categories in [`CATEGORY_ORDER`].
    pub category: String,
    /// Lower-case executable names that map to this profile. Several per app is normal
    /// (browsers ship `chrome.exe` and `chrome_beta.exe`, for instance).
    pub exe: Vec<String>,
    #[serde(default)]
    pub collections: Vec<Collection>,
}

impl AppProfile {
    pub fn all_shortcuts(&self) -> impl Iterator<Item = &Shortcut> {
        self.collections.iter().flat_map(|c| c.shortcuts.iter())
    }

    /// Count used by the app-card tiles on the hub (S02), which is a `should` feature and
    /// not part of this build yet.
    #[allow(dead_code)]
    pub fn total(&self) -> usize {
        self.all_shortcuts().filter(|s| !s.removed).count()
    }
}

/// An app the target supports that we have no rows for yet: enough to recognise the window
/// and name it, not enough to fill the overlay. The list was read off the target's own
/// startup log, which names every profile it loads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownApp {
    pub name: String,
    pub category: String,
    /// Empty when the process name has not been verified. Guessing one would make the
    /// overlay claim an unrelated app is this one.
    #[serde(default)]
    pub exe: Vec<String>,
}

/// Words the interface says. Kept in the dataset rather than in the paint code so a
/// rebranding pass has one file to edit, and so the wording can be checked against the
/// recording without running the app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Strings {
    /// The overlay's search field. The target's own says one word; matching it is the
    /// difference between a screenshot diff passing on layout and failing on pixels.
    pub overlay_search_placeholder: String,
    /// The hub's app filter, which is a longer phrase because the field is wider.
    pub hub_search_placeholder: String,
    pub overlay_empty: String,
    pub overlay_no_match: String,
    pub overlay_no_match_hint: String,
    pub collections_label: String,
    pub contribute_label: String,
    pub edit_shortcuts_label: String,
    pub drill_remaining_label: String,
    pub drill_mastered_label: String,
    pub drill_skip_label: String,
}

impl Default for Strings {
    fn default() -> Self {
        Self {
            overlay_search_placeholder: "Search".into(),
            hub_search_placeholder: "Search an app".into(),
            overlay_empty: "No shortcuts listed for this app yet".into(),
            overlay_no_match: "Nothing matches that search".into(),
            overlay_no_match_hint: "Add your own in apps.json".into(),
            collections_label: "collections".into(),
            contribute_label: "Contribute on GitHub".into(),
            edit_shortcuts_label: "Improve or change shortcuts".into(),
            drill_remaining_label: "remaining".into(),
            drill_mastered_label: "mastered".into(),
            drill_skip_label: "Skip for now".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Trigger {
    Ctrl,
    Alt,
    Shift,
    Win,
}

impl Trigger {
    pub fn vk(self) -> u32 {
        match self {
            // Left-hand codes: the common case, and the hook also sees the right-hand
            // variant because we match on the generic VK the OS reports.
            Trigger::Ctrl => Key::Ctrl.vk() as u32,
            Trigger::Alt => Key::Alt.vk() as u32,
            Trigger::Shift => Key::Shift.vk() as u32,
            Trigger::Win => Key::Win.vk() as u32,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub trigger: Trigger,
    /// How long the trigger must be held before the overlay appears.
    pub hold_ms: u64,
    pub theme: Theme,
    /// 0.0 - 0.25: how much to fade the overlay background.
    pub fade: f32,
    pub run_on_startup: bool,
    pub max_rows: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            trigger: Trigger::Ctrl,
            // 1.5 s, not 1 s: the target's own settings log writes a first hold of
            // `00:00:01.5` before any of the user's changes, and a shorter threshold fires
            // the overlay every time someone uses Ctrl+C.
            hold_ms: 1500,
            theme: Theme::default(),
            fade: 0.04,
            run_on_startup: false,
            max_rows: 40,
        }
    }
}

/// Progress for one (app, collection) pair. Shape mirrors what the original keeps:
/// two lists of command names, no timestamps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Learning {
    #[serde(default)]
    pub learnt: Vec<String>,
    #[serde(default)]
    pub mastered: Vec<String>,
}

impl Learning {
    /// Never seen in the overlay: not in either list. Drives the hub's "n remaining" count.
    #[allow(dead_code)]
    pub fn is_new(&self, command: &str) -> bool {
        !self.learnt.iter().any(|c| c == command) && !self.is_mastered(command)
    }

    pub fn is_mastered(&self, command: &str) -> bool {
        self.mastered.iter().any(|c| c == command)
    }

    /// Fired at least once from the overlay.
    pub fn learn(&mut self, command: &str) {
        if !self.learnt.iter().any(|c| c == command) && !self.is_mastered(command) {
            self.learnt.push(command.to_string());
        }
    }

    /// Used correctly in the drill (S05) - the list the original's own LearningData keeps.
    pub fn master(&mut self, command: &str) {
        self.learnt.retain(|c| c != command);
        if !self.mastered.iter().any(|c| c == command) {
            self.mastered.push(command.to_string());
        }
    }
}

/// The hub groups apps by category; kept here so the dataset owns the ordering. The target
/// sorts both categories and the cards inside them alphabetically, and this list is that
/// order - two names came out of its own section headers rather than out of ours:
/// "Digital Audio Workstations (DAWs)" and "Miscellaneous".
#[allow(dead_code)]
pub const CATEGORY_ORDER: &[&str] = &[
    "Browsers & Internet",
    "Communication",
    "Development & Coding",
    "Digital Audio Workstations (DAWs)",
    "Graphics & Design",
    "Media Players",
    "Miscellaneous",
    "Productivity & Office",
    "Utilities & Tools",
    "Video Games",
];

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("could not read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not parse {path}: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("could not write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("bad shortcut in {context}: {source}")]
    Key {
        context: String,
        #[source]
        source: key::ComboError,
    },
}

/// Everything the built-in `apps.json` holds: the rows, the words, and what a fresh install
/// starts with. The user's own files stay a bare array of profiles - they are the edit
/// surface, and asking someone to find the right object to nest their shortcut in is a worse
/// interface than appending to a list.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Dataset {
    pub strings: Strings,
    pub defaults: Settings,
    pub apps: Vec<AppProfile>,
    pub known_apps: Vec<KnownApp>,
}

/// Everything the app knows, merged and indexed.
pub struct Catalog {
    pub apps: Vec<AppProfile>,
    by_exe: HashMap<String, usize>,
    pub settings: Settings,
    /// The words the interface says, read by the overlay on every frame it paints.
    pub strings: Strings,
    /// Profiles with no rows yet. Detection reads them in the engine pass; until then they
    /// are carried, not ignored, so the roster the target loads is on disk in one place.
    #[allow(dead_code)]
    pub known_apps: Vec<KnownApp>,
    learning: HashMap<(String, String), Learning>,
    dir: PathBuf,
}

impl Catalog {
    /// Built-in dataset, then user files merged over it.
    ///
    /// Only the compiled-in dataset can fail the load. Everything the user or the app writes
    /// is degraded instead: a background utility that disappears because of one mistyped
    /// comma is a worse failure than one ignored file. What is skipped gets logged.
    pub fn load(dir: &Path) -> Result<Self, StoreError> {
        let mut doc: Dataset =
            serde_json::from_str(BUILTIN).map_err(|e| StoreError::Parse {
                path: dir.join("builtin"),
                message: e.to_string(),
            })?;

        validate(&doc.apps)?;

        let mut apps = std::mem::take(&mut doc.apps);

        let user_dir = dir.join("apps");
        if user_dir.is_dir() {
            for entry in std::fs::read_dir(&user_dir).map_err(|s| StoreError::Read {
                path: user_dir.clone(),
                source: s,
            })? {
                let path = entry.map_err(|s| StoreError::Read {
                    path: user_dir.clone(),
                    source: s,
                })?.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let text = match std::fs::read_to_string(&path) {
                    Ok(t) => t,
                    Err(source) => {
                        tracing::error!(file = %path.display(), error = %source,
                            "could not read a shortcut file; skipping it");
                        continue;
                    }
                };
                let extra: Vec<AppProfile> = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(e) => {
                        // Names the file and the parse position, so the user can fix it.
                        tracing::error!(file = %path.display(), error = %e,
                            "could not parse a shortcut file; skipping it");
                        continue;
                    }
                };
                let extra = screen_out_bad_rows(extra, &path);
                for app in extra {
                    match apps.iter_mut().find(|a| a.name == app.name) {
                        Some(existing) => merge_app(existing, app),
                        None => apps.push(app),
                    }
                }
            }
        }

        let mut by_exe = HashMap::new();
        for (i, app) in apps.iter().enumerate() {
            for exe in &app.exe {
                by_exe.insert(exe.to_ascii_lowercase(), i);
            }
        }

        let settings = Settings::load(dir, &doc.defaults);
        let learning = Learning::load_all(dir);

        Ok(Self {
            apps,
            by_exe,
            settings,
            strings: doc.strings,
            known_apps: doc.known_apps,
            learning,
            dir: dir.to_path_buf(),
        })
    }

    pub fn find_by_exe(&self, exe: &str) -> Option<&AppProfile> {
        self.by_exe.get(&exe.to_ascii_lowercase()).map(|i| &self.apps[*i])
    }

    pub fn app(&self, name: &str) -> Option<&AppProfile> {
        self.apps.iter().find(|a| a.name == name)
    }

    /// Category grouping for the hub grid (S02). Not wired into the overlay build yet.
    #[allow(dead_code)]
    pub fn by_category(&self) -> Vec<(&'static str, Vec<&AppProfile>)> {
        CATEGORY_ORDER
            .iter()
            .filter_map(|cat| {
                let apps: Vec<&AppProfile> =
                    self.apps.iter().filter(|a| &a.category == cat).collect();
                if apps.is_empty() {
                    None
                } else {
                    Some((*cat, apps))
                }
            })
            .collect()
    }

    /// Read side of the progress file. The overlay does not show mastery state yet, so the
    /// first consumer is the hub and drill (S02, S05).
    #[allow(dead_code)]
    pub fn learning(&self, app: &str, collection: &str) -> &Learning {
        static EMPTY: Learning = Learning { learnt: Vec::new(), mastered: Vec::new() };
        self.learning.get(&(app.into(), collection.into())).unwrap_or(&EMPTY)
    }

    /// The user picked this row out of the overlay and had it fired: that is the "learnt"
    /// half of the original's progress model.
    pub fn record_learnt(&mut self, app: &str, collection: &str, command: &str) {
        let key = (app.to_string(), collection.to_string());
        let entry = self.learning.entry(key).or_default();
        let before = entry.learnt.len();
        entry.learn(command);
        if entry.learnt.len() == before {
            return; // already recorded, nothing to rewrite
        }
        if let Err(e) = self.save_learning(app, collection) {
            tracing::warn!(error = %e, "could not persist progress");
        }
    }

    /// Promoted by the timed drill (S05).
    #[allow(dead_code)]
    pub fn record_mastered(&mut self, app: &str, collection: &str, command: &str) {
        let key = (app.to_string(), collection.to_string());
        let entry = self.learning.entry(key).or_default();
        entry.master(command);
        if let Err(e) = self.save_learning(app, collection) {
            tracing::warn!(error = %e, "could not persist progress");
        }
    }

    fn save_learning(&self, app: &str, collection: &str) -> Result<(), StoreError> {
        let key = (app.to_string(), collection.to_string());
        let Some(l) = self.learning.get(&key) else { return Ok(()) };
        let dir = self.dir.join("learning");
        std::fs::create_dir_all(&dir).ok();
        // One file per app+collection, named with the first `_` as the split point - the
        // shape found in the original's own LearningData folder. Names coming out of the
        // dataset can contain characters Windows refuses in a filename, so they are dropped.
        let path = dir.join(format!(
            "{}_{}.json",
            sanitize_filename(app),
            sanitize_filename(collection)
        ));
        write_atomic(&path, &serde_json::to_vec_pretty(l).unwrap())
    }

    #[allow(dead_code)]
    pub fn save_settings(&self) -> Result<(), StoreError> {
        Settings::save(&self.dir, &self.settings)
    }
}

/// Strip what NTFS will not accept in a file name.
fn sanitize_filename(s: &str) -> String {
    s.chars().filter(|c| !matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')).collect()
}

fn validate(apps: &[AppProfile]) -> Result<(), StoreError> {
    for app in apps {
        for col in &app.collections {
            for s in &col.shortcuts {
                s.combo().map_err(|source| StoreError::Key {
                    context: format!("{} / {} / {}", app.name, col.name, s.command),
                    source,
                })?;
            }
        }
    }
    Ok(())
}

/// A built-in typo is our bug and must stop startup; a typo in the user's own file is not.
/// One mistyped line there must not send a background utility quietly away, and must not
/// delete the working shortcut the user was trying to override - so the bad rows are dropped
/// here, before the merge, name by name, and the rest of the file still lands.
fn screen_out_bad_rows(mut apps: Vec<AppProfile>, path: &Path) -> Vec<AppProfile> {
    for app in &mut apps {
        for col in &mut app.collections {
            col.shortcuts.retain(|s| match s.combo() {
                Ok(_) => true,
                // A tombstone's chord is never parsed, drawn or fired, so a removal survives
                // even when the user leaves a meaningless string in `keys`.
                Err(_) if s.removed => true,
                Err(e) => {
                    tracing::warn!(
                        file = %path.display(),
                        app = %app.name,
                        collection = %col.name,
                        command = %s.command,
                        keys = %s.keys,
                        error = %e,
                        "skipping a shortcut this build cannot parse or fire"
                    );
                    false
                }
            });
        }
    }
    apps
}

/// User entries win by command name; `removed: true` deletes.
fn merge_app(into: &mut AppProfile, from: AppProfile) {
    for col in from.collections {
        let target = match into.collections.iter_mut().find(|c| c.name == col.name) {
            Some(c) => c,
            None => {
                into.collections.push(Collection { name: col.name.clone(), shortcuts: vec![] });
                into.collections.last_mut().unwrap()
            }
        };
        for s in col.shortcuts {
            match target.shortcuts.iter_mut().find(|x| x.command == s.command) {
                Some(existing) => {
                    if s.removed {
                        existing.removed = true;
                    } else {
                        *existing = s;
                    }
                }
                None => target.shortcuts.push(s),
            }
        }
    }
    for exe in from.exe {
        let exe = exe.to_ascii_lowercase();
        if !into.exe.iter().any(|e| e.eq_ignore_ascii_case(&exe)) {
            into.exe.push(exe);
        }
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes).map_err(|source| StoreError::Write {
        path: tmp.clone(),
        source,
    })?;
    std::fs::rename(&tmp, path).map_err(|source| StoreError::Write {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

impl Settings {
    /// A settings file this build cannot read is not worth refusing to start over: the
    /// defaults are safe, and the broken file is left alone for the user to fix.
    ///
    /// `fallback` is the dataset's own `defaults` block, so what a fresh install does is
    /// written in the data file and not duplicated in two languages.
    fn load(dir: &Path, fallback: &Settings) -> Settings {
        let path = dir.join("settings.json");
        if !path.exists() {
            return fallback.clone();
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(file = %path.display(), error = %e,
                    "could not read settings; falling back to defaults");
                return fallback.clone();
            }
        };
        match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(file = %path.display(), error = %e,
                    "could not parse settings; falling back to defaults");
                fallback.clone()
            }
        }
    }

    #[allow(dead_code)]
    fn save(dir: &Path, s: &Settings) -> Result<(), StoreError> {
        std::fs::create_dir_all(dir).ok();
        write_atomic(&dir.join("settings.json"), &serde_json::to_vec_pretty(s).unwrap())
    }
}

impl Learning {
    /// Progress is the least important file in the directory and the most likely to be
    /// half-written by a crash, so a damaged one is skipped rather than fatal.
    fn load_all(dir: &Path) -> HashMap<(String, String), Learning> {
        let mut out = HashMap::new();
        let ldir = dir.join("learning");
        if !ldir.is_dir() {
            return out;
        }
        let entries = match std::fs::read_dir(&ldir) {
            Ok(entries) => entries,
            Err(source) => {
                tracing::error!(dir = %ldir.display(), error = %source,
                    "could not list the progress folder; starting with no progress");
                return out;
            }
        };
        for entry in entries {
            let path = match entry {
                Ok(e) => e.path(),
                Err(source) => {
                    tracing::warn!(dir = %ldir.display(), error = %source,
                        "unreadable entry in the progress folder");
                    continue;
                }
            };
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
            let Some((app, collection)) = stem.split_once('_') else { continue };
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(source) => {
                    tracing::warn!(file = %path.display(), error = %source,
                        "could not read a progress file; skipping it");
                    continue;
                }
            };
            match serde_json::from_str(&text) {
                Ok(l) => {
                    out.insert((app.to_string(), collection.to_string()), l);
                }
                Err(e) => {
                    tracing::warn!(file = %path.display(), error = %e,
                        "could not parse a progress file; skipping it");
                }
            }
        }
        out
    }
}

/// The compiled-in dataset. Ships inside the exe so a fresh machine with no data
/// directory still shows something the first time you hold the key.
pub const BUILTIN: &str = include_str!("apps.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_dataset_is_valid_json_and_parses() {
        let doc: Dataset = serde_json::from_str(BUILTIN).expect("builtin apps.json");
        assert!(!doc.apps.is_empty());
        validate(&doc.apps).expect("every built-in combo parses");
    }

    #[test]
    fn every_builtin_app_has_an_exe_and_a_category_in_order() {
        let doc: Dataset = serde_json::from_str(BUILTIN).unwrap();
        for a in &doc.apps {
            assert!(!a.exe.is_empty(), "{} names no executable", a.name);
            assert!(
                CATEGORY_ORDER.contains(&a.category.as_str()),
                "{} has category '{}' which is not in CATEGORY_ORDER",
                a.name,
                a.category
            );
            for e in &a.exe {
                assert_eq!(e, &e.to_ascii_lowercase(), "{} lists exe '{e}' not lower-case", a.name);
                assert!(e.ends_with(".exe"), "{} lists '{e}' without .exe", a.name);
            }
        }
    }

    #[test]
    fn no_exe_is_claimed_twice() {
        let doc: Dataset = serde_json::from_str(BUILTIN).unwrap();
        let mut seen = HashMap::new();
        // The roster and the profiles share one namespace: a name in both, or an exe claimed
        // by both, would make "which app is this" have two answers.
        let claims = doc
            .apps
            .iter()
            .map(|a| (a.name.as_str(), a.exe.iter()))
            .chain(doc.known_apps.iter().map(|a| (a.name.as_str(), a.exe.iter())));
        for (name, exes) in claims {
            for e in exes {
                if let Some(prev) = seen.insert(e.clone(), name.to_string()) {
                    panic!("'{e}' maps to both '{prev}' and '{name}'");
                }
            }
        }
    }

    /// The roster is the target's own app list, read from the names it logs while loading.
    /// Every entry has to be placeable on the hub, and an unverified process name has to stay
    /// empty rather than become a guess.
    #[test]
    fn every_known_app_is_placeable_and_claims_no_unverified_exe() {
        let doc: Dataset = serde_json::from_str(BUILTIN).unwrap();
        assert!(doc.known_apps.len() > 40, "the roster lost most of the target's apps");
        let mut names = std::collections::HashSet::new();
        for k in &doc.known_apps {
            assert!(
                CATEGORY_ORDER.contains(&k.category.as_str()),
                "roster entry '{}' has category '{}'",
                k.name,
                k.category
            );
            for e in &k.exe {
                assert_eq!(e, &e.to_ascii_lowercase(), "'{e}' is not lower-case");
                assert!(e.ends_with(".exe"), "'{e}' is not a process name");
            }
            assert!(
                names.insert(k.name.clone()),
                "'{}' is in the roster twice",
                k.name
            );
            assert!(
                doc.apps.iter().all(|a| a.name != k.name),
                "'{}' is both a profile and a roster entry",
                k.name
            );
        }
    }

    /// Wording is part of the visual diff: a field that says something else fails the
    /// screenshot even when every pixel of layout lines up.
    #[test]
    fn the_dataset_carries_the_words_the_interface_says() {
        let doc: Dataset = serde_json::from_str(BUILTIN).unwrap();
        assert_eq!(doc.strings.overlay_search_placeholder, "Search");
        assert_eq!(doc.strings.hub_search_placeholder, "Search an app");
        for (name, value) in [
            ("overlay_empty", &doc.strings.overlay_empty),
            ("overlay_no_match", &doc.strings.overlay_no_match),
            ("overlay_no_match_hint", &doc.strings.overlay_no_match_hint),
            ("collections_label", &doc.strings.collections_label),
            ("contribute_label", &doc.strings.contribute_label),
            ("edit_shortcuts_label", &doc.strings.edit_shortcuts_label),
            ("drill_remaining_label", &doc.strings.drill_remaining_label),
            ("drill_mastered_label", &doc.strings.drill_mastered_label),
            ("drill_skip_label", &doc.strings.drill_skip_label),
        ] {
            assert!(!value.trim().is_empty(), "{name} is empty in the dataset");
        }
        // And the file, not the Rust default, is what a fresh install reads.
        assert_eq!(doc.strings, Strings::default());
    }

    #[test]
    fn a_fresh_install_takes_its_settings_from_the_dataset() {
        // No settings.json at all: the defaults block is the only place this can come from,
        // which is what keeps "what the app does on day one" reviewable in the data file.
        let dir = tempdir("fresh-defaults");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = Catalog::load(&dir).unwrap();
        assert_eq!(c.settings.hold_ms, 1500);
        assert_eq!(c.settings.trigger, Trigger::Ctrl);
        assert_eq!(c.settings.theme, Theme::System);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn aliases_are_extra_names_for_the_same_command() {
        let doc: Dataset = serde_json::from_str(BUILTIN).unwrap();
        let with_aliases: Vec<&Shortcut> = doc
            .apps
            .iter()
            .flat_map(|a| a.all_shortcuts())
            .filter(|s| !s.aliases.is_empty())
            .collect();
        assert!(
            !with_aliases.is_empty(),
            "the alias field is dead: nothing in the dataset uses it"
        );
        for s in with_aliases {
            for alias in &s.aliases {
                assert!(!alias.trim().is_empty(), "'{}' has a blank alias", s.command);
                assert_ne!(alias, &s.command, "'{}' lists itself as an alias", s.command);
            }
        }
    }

    #[test]
    fn learning_master_moves_out_of_learnt() {
        let mut l = Learning { learnt: vec!["Copy".into()], mastered: vec![] };
        l.master("Copy");
        assert!(l.is_mastered("Copy"));
        assert!(l.learnt.is_empty());
        l.master("Copy");
        assert_eq!(l.mastered.len(), 1, "mastering twice must not duplicate");
    }

    #[test]
    fn learnt_stops_counting_as_new_and_mastered_stops_counting_as_learnt() {
        let mut l = Learning::default();
        assert!(l.is_new("Copy"));
        l.learn("Copy");
        assert!(!l.is_new("Copy"));
        assert!(l.learnt.contains(&"Copy".to_string()));
        l.learn("Copy");
        assert_eq!(l.learnt.len(), 1, "firing twice must not duplicate");
        l.master("Copy");
        assert!(!l.is_new("Copy"), "a mastered shortcut is not new again");
        l.learn("Copy");
        assert!(l.learnt.is_empty(), "mastered must not fall back to learnt");
    }

    #[test]
    fn learnt_progress_survives_a_reload() {
        // The whole point of the store: a restart must not lose what the user has fired.
        let dir = std::env::temp_dir().join(format!("ss-learn-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut catalog = Catalog::load(&dir).unwrap();
        let app = catalog.apps[0].name.clone();
        let collection = catalog.apps[0].collections[0].name.clone();
        let command = catalog.apps[0].collections[0].shortcuts[0].command.clone();
        assert!(catalog.learning(&app, &collection).is_new(&command));

        catalog.record_learnt(&app, &collection, &command);
        drop(catalog);

        let reloaded = Catalog::load(&dir).unwrap();
        assert!(
            reloaded.learning(&app, &collection).is_mastered(&command)
                || reloaded.learning(&app, &collection).learnt.contains(&command),
            "progress written by record_learnt did not come back"
        );
        assert!(
            dir.join("learning").join(format!("{}_{}.json", sanitize_filename(&app), sanitize_filename(&collection)))
                .exists(),
            "the progress file is not where load_all reads it from"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_defaults_are_sane() {
        let s = Settings::default();
        assert_eq!(s.trigger, Trigger::Ctrl);
        assert!(s.hold_ms > 300 && s.hold_ms < 2000, "hold_ms {} is not usable", s.hold_ms);
    }

    #[test]
    fn settings_round_trip_with_missing_fields() {
        // An older settings file must still load: every field is #[serde(default)].
        let s: Settings = serde_json::from_str(r#"{"trigger":"alt"}"#).unwrap();
        assert_eq!(s.trigger, Trigger::Alt);
        assert_eq!(s.hold_ms, 1500);
    }

    /// A data dir holding the built-in dataset plus one user file (F04: edit or add a shortcut).
    fn catalog_with(user_json: &str) -> Catalog {
        static SEQ: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = tempdir(&format!("user-{n}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("apps")).unwrap();
        std::fs::write(dir.join("apps").join("override.json"), user_json).unwrap();
        Catalog::load(&dir).expect("a catalog with a user file")
    }

    fn tempdir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ss-data-{}-{}", std::process::id(), tag))
    }

    /// The keys a given command ends up with after the merge, or None if it is not listed.
    fn keys_of(catalog: &Catalog, app: &str, command: &str) -> Option<String> {
        catalog
            .app(app)?
            .all_shortcuts()
            .filter(|s| !s.removed)
            .find(|s| s.command == command)
            .map(|s| s.keys.clone())
    }

    #[test]
    fn a_user_entry_overrides_the_builtin_chord_of_the_same_command() {
        let c = catalog_with(r#"[{
            "name": "Visual Studio Code",
            "category": "Development & Coding",
            "exe": ["code.exe"],
            "collections": [{"name": "Everyday", "shortcuts": [
                {"command": "New file", "keys": "Ctrl+Alt+N"}
            ]}]
        }]"#);
        assert_eq!(keys_of(&c, "Visual Studio Code", "New file").as_deref(), Some("Ctrl+Alt+N"));
        // Untouched neighbours survive the merge.
        assert_eq!(keys_of(&c, "Visual Studio Code", "Save").as_deref(), Some("Ctrl+S"));
    }

    #[test]
    fn a_mistyped_override_is_dropped_without_deleting_the_shortcut_it_replaced() {
        // The file mixes one bad chord with one good one. Losing the whole app, or losing the
        // working "Save" because its replacement could not be parsed, are both unacceptable
        // answers for a hand-edited file.
        let c = catalog_with(r#"[{
            "name": "Visual Studio Code",
            "category": "Development & Coding",
            "exe": ["code.exe"],
            "collections": [{"name": "Everyday", "shortcuts": [
                {"command": "Save", "keys": "Ctrl+Banan"},
                {"command": "Go to line", "keys": "Ctrl+G"}
            ]}]
        }]"#);
        assert_eq!(keys_of(&c, "Visual Studio Code", "Save").as_deref(), Some("Ctrl+S"));
        assert_eq!(keys_of(&c, "Visual Studio Code", "Go to line").as_deref(), Some("Ctrl+G"));
    }

    #[test]
    fn a_tombstone_removes_a_builtin_shortcut_from_every_list() {
        let dir = tempdir("tombstone");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("apps")).unwrap();
        std::fs::write(
            dir.join("apps").join("cut.json"),
            r#"[{
                "name": "Visual Studio Code",
                "category": "Development & Coding",
                "exe": ["code.exe"],
                "collections": [{"name": "Everyday", "shortcuts": [
                    {"command": "Open file", "removed": true}
                ]}]
            }]"#,
        )
        .unwrap();
        let c = Catalog::load(&dir).unwrap();
        assert_eq!(keys_of(&c, "Visual Studio Code", "Open file"), None, "still listed after removal");
        let app = c.app("Visual Studio Code").unwrap();
        assert!(
            crate::detect::visible_rows(app)
                .into_iter()
                .all(|(_, ci, si)| app.collections[ci].shortcuts[si].command != "Open file"),
            "the overlay would still show a removed shortcut"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_repeated_command_in_one_file_wins_once_not_twice() {
        // F04's other edge: the same command name twice in the same edit.
        let c = catalog_with(r#"[{
            "name": "Visual Studio Code",
            "category": "Development & Coding",
            "exe": ["code.exe"],
            "collections": [{"name": "Everyday", "shortcuts": [
                {"command": "Zoom in", "keys": "Ctrl+Shift+Equals"},
                {"command": "Zoom in", "keys": "Ctrl+Equals"}
            ]}]
        }]"#);
        let app = c.app("Visual Studio Code").unwrap();
        let hits: Vec<&str> = app
            .all_shortcuts()
            .filter(|s| !s.removed && s.command == "Zoom in")
            .map(|s| s.keys.as_str())
            .collect();
        assert_eq!(hits, vec!["Ctrl+Equals"], "the last write should win, and only once");
    }

    #[test]
    fn a_user_file_can_add_an_app_the_build_never_shipped() {
        let c = catalog_with(r#"[{
            "name": "Krita",
            "category": "Graphics & Design",
            "exe": ["krita.exe"],
            "collections": [{"name": "Layers", "shortcuts": [
                {"command": "Duplicate layer", "keys": "Shift+Ctrl+D"}
            ]}]
        }]"#);
        assert_eq!(
            crate::detect::resolve(&c, Some("krita.exe")),
            crate::detect::Context::Known("Krita".into())
        );
        assert_eq!(keys_of(&c, "Krita", "Duplicate layer").as_deref(), Some("Shift+Ctrl+D"));
        // And the built-in dataset is still there behind it.
        assert!(c.app("Visual Studio Code").is_some());
    }

    #[test]
    fn damaged_files_degrade_the_load_instead_of_killing_the_app() {
        // Every one of these is a file the user or a crash can produce. The answer to each is
        // "lose that file, start anyway" - a resident utility that vanishes because of one
        // lost comma is a worse failure than an ignored override.
        let dir = tempdir("damaged");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("apps")).unwrap();
        std::fs::create_dir_all(dir.join("learning")).unwrap();
        std::fs::write(dir.join("apps").join("half-written.json"), b"").unwrap();
        std::fs::write(dir.join("apps").join("truncated.json"), r#"[{"name": "Krita", "categor"#)
            .unwrap();
        std::fs::write(
            dir.join("apps").join("good.json"),
            r#"[{
                "name": "Krita",
                "category": "Graphics & Design",
                "exe": ["krita.exe"],
                "collections": [{"name": "Layers", "shortcuts": [
                    {"command": "Duplicate layer", "keys": "Shift+Ctrl+D"}
                ]}]
            }]"#,
        )
        .unwrap();
        std::fs::write(dir.join("settings.json"), r#"{"hold_ms": "soon"}"#).unwrap();
        std::fs::write(dir.join("learning").join("broken_Chrome.json"), "not json at all").unwrap();

        let c = Catalog::load(&dir).expect("the app has to start anyway");
        assert_eq!(c.settings.hold_ms, 1500, "an unreadable settings file falls back to defaults");
        assert!(c.app("Visual Studio Code").is_some(), "the built-ins survive");
        assert!(c.app("Krita").is_some(), "the file that was still valid got merged");
        assert!(c.learning("broken", "Chrome").learnt.is_empty(), "the damaged progress file is skipped");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
