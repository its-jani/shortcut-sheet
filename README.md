# shortcut-sheet

**Hold a key, see the shortcuts for the app you are in — and fire one without leaving it.**

A clean-room, open-source Windows desktop utility. Hold `Ctrl` — or any key you name, from
`F13` to `Caps` to `Ctrl+Shift` — for a moment in any supported app and a focusless cheat-sheet
overlay appears centered on that app's window. Click or search a command and the real key combo
is injected back into the app you were already using. Release the key and the overlay is gone —
your focus never moved.

Written fresh in pure Rust. It is **not** a fork, rebrand, or port of any commercial
shortcut app; only factual key mappings (which combo does what) are reused, and those are
free facts. No logos, names, marketing copy, or proprietary assets are used or shipped.

## The core loop

```
work in any app -> hold Ctrl ~1.5s -> overlay of that app's shortcuts appears
-> search or click a command -> the combo fires into the app you were in
-> release Ctrl -> overlay hides, focus never left
```

The overlay never steals the caret on the browse path. It is a tool window
(`WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST`), centered on the *foreground
window's* rectangle — not the screen — across DPI scales and multiple monitors. Clicking into
the search box is the one deliberate exception, and every way out of the panel — Enter, an
arrow-selected row, Escape, the close mark, releasing the key, or clicking away — hands the
caret back to the window the session came from. On the firing paths the hand-back happens
*before* `SendInput`, which is what makes the chord land in your app instead of in our own
text field.

## Features

Implemented:

- **Two trigger modes, on any keys you like**, watched by one global low-level keyboard hook
  that rejects injected input (`LLMHF_INJECTED`), so the app's own fired combos never re-trigger
  it. A **long press** of any key — or any set of keys held together — for a configurable delay,
  and an **instant chord** (`Win+Alt` by default, either or both switchable off). Left and right
  Ctrl/Alt/Shift/Win are one trigger, not two. The hook proc does no allocation, no locking and
  no I/O to stay well under Windows' `LowLevelHooksTimeout`. See [Choosing a
  trigger](#choosing-a-trigger).
- **Foreground-app detection** — maps the active process image name (e.g. `Code.exe`,
  `chrome.exe`) to a shortcut profile, and a name on the roster to a panel that says so. A
  recognised app with no rows is a different answer from an unknown one, and gets one.
- **Cheat-sheet overlay (S01)** — title bar with a letter-avatar, live search, grouped
  shortcut rows with rendered keycap chips, hover/keyboard selection, and empty/no-match
  states. Light and dark themes, following the OS or an override.
- **Search across every name a command has** — the label plus its `aliases`, ranked prefix >
  initials > substring > subsequence, with a match on the visible label outranking the same
  match on an alias.
- **Shortcut injection** — fires the selected chord into the target app via `SendInput` in
  the canonical modifier-first order.
- **Progress recording** — firing a command marks it "learnt" in a per-(app, collection)
  store that survives restarts and app renames.
- **Tray icon** with a Quit menu — the app's only shell presence (the overlay has no taskbar
  entry). Its tooltip names the key your settings actually chose, not the one this build
  shipped with.
- **Hand-editable dataset** — user JSON files merge over the compiled-in catalog; entries
  override by command name and `"removed": true` tombstones delete them.
- **Rolling file logging** — this exe has no console, so diagnostics go to
  `%LOCALAPPDATA%\shortcut-sheet\logs`.

Planned / not yet built: the practice hub, app grid, collections page and timed drill
(S02–S05), and a settings window (settings are edited in JSON today).

## Requirements

- Windows 10 / 11 (x86_64). Uses the Win32 API directly.
- Rust **1.88** or newer (`rust-version` in `Cargo.toml`).
- The MSVC build toolchain (the Windows link Rust already needs). No C++ or Python at
  runtime.

## Build and run

```powershell
cargo build --release
target\release\shortcut-sheet.exe
```

Hold `Ctrl` in a supported app to bring up the overlay — or whatever key `trigger` names, if
you have edited `settings.json`. Right-click the tray icon to quit.

For development:

```powershell
cargo run                 # build and launch
cargo test                # run the unit + store tests
```

Release builds are `lto = "thin"`, `codegen-units = 1`, `strip = "debuginfo"`.

### Command-line flags

| flag | behaviour |
| --- | --- |
| `--detect` | Print which process is in the foreground and which profile it resolves to, then exit. Opens no window and installs no hook. |
| `--peek [App Name]` | Open the overlay once, centered, and auto-close. The hook ignores injected input by design, so this is the only way to see or screenshot the overlay from a script. `--peek` alone shows the first app; `--peek="Visual Studio Code"` names one. |

## Configuration and data

Everything lives under `%LOCALAPPDATA%\shortcut-sheet\` (falling back to `./ss-data` if that
variable is unset). The app runs fine with an empty directory — a compiled-in dataset ships
inside the exe.

```
settings.json                             trigger, hold, instant chord, theme, startup, fade, max_rows
apps/<app>.json                           user overrides/shortcuts, a list of profiles
learning/<App>_<Collection>.json          { "learnt": [...], "mastered": [...] }
logs/shortcut-sheet.log.<date>            rolling diagnostics
```

The compiled-in dataset (`src/data/apps.json`) is one document, not a bare list: `strings`
holds the words the interface says, `defaults` holds what a fresh install does before a
`settings.json` exists, `apps` holds the profiles with rows in them, and `known_apps` lists
the apps that are recognised but not filled in yet. A **user** file stays the simpler shape —
a list of profiles — because appending to a list is the friendlier thing to hand-edit.

`settings.json` example — every field optional, and a value this build cannot read degrades
that one setting rather than the app:

```json
{
  "trigger": "ctrl",
  "hold_ms": 1500,
  "long_press_enabled": true,
  "instant_enabled": false,
  "instant_hotkey": "Win+Alt",
  "theme": "system",
  "fade": 0.04,
  "run_on_startup": false,
  "max_rows": 40
}
```

The two trigger modes are independent. `long_press_enabled` is the hold: `trigger` — one key or
a set — down for `hold_ms` with nothing else pressed. `instant_enabled` is the chord:
`instant_hotkey` pressed together, no delay, and the sheet stays up until it is dismissed —
Escape, a fired row, a second press of the chord, or clicking away. A chord has to be at least
two keys, because a bare `Ctrl` as an instant trigger would pop the sheet over every
copy-paste.

### Choosing a trigger

`trigger` is any key this build can name, spelled the way every shortcut in the dataset is
spelled: case-insensitive, `+` between keys, the same words as the keycaps. One key is the
normal answer; a set of keys held together is legal too.

| `trigger` | means |
| --- | --- |
| `"ctrl"` | hold Ctrl — the default, either side of the board |
| `"alt"`, `"shift"`, `"win"` | any other modifier (`super`, `cmd`, `control`, `option` are accepted spellings) |
| `"f13"`, `"caps"`, `"prtsc"`, `"pause"`, `"menu"` | a function key, or one of the spare keys nobody else uses |
| `"grave"`, `"comma"`, `"slash"`, `"'"` | punctuation — by name, or by the glyph on the key |
| `"space"`, `"tab"`, `"esc"`, `"up"` | the ordinary keys, including the navigation cluster |
| `"ctrl+shift"` | hold both at once — the sheet comes up when the last one lands |
| `"ctrl+shift+p"` | hold a whole shortcut, if that is the gesture you want |

Both modes read the same key stream, and a completed instant chord always wins over a hold in
progress, so `"trigger": "ctrl"` beside `"instant_hotkey": "Ctrl+Shift+P"` behaves the way you
would hope: Ctrl alone looks something up, Ctrl+Shift+P summons at once.

Four things worth knowing before you pick:

- **Keys are watched, never swallowed.** The hook does not eat your trigger key, so a key that
  types or toggles keeps doing that while you hold it. That is why a modifier, a high function
  key or a spare key makes a good trigger, and why `"trigger": "a"` would write `aaa` into your
  document until the sheet appears.
- **Either side of the board is the same key.** The hook folds `VK_LCONTROL`, `VK_RMENU` and
  their siblings onto the generic code before the rules run, so a right-handed Ctrl counts.
- **A typo costs one setting, not the app.** A `trigger` this build cannot read turns the hold
  mode off and says so in the log; it never substitutes Ctrl for the key you asked for, and it
  leaves the instant chord alone. Same for `instant_hotkey`. If both come out off, the app still
  runs with a tray icon and says why once in the log.
- **The tooltip follows the setting.** Both modes resolve once at startup, and the hook and the
  tray tooltip read the same answer, so the sentence over the icon cannot name a key that does
  nothing. `--peek` still works whatever the trigger is, and
  `%LOCALAPPDATA%\shortcut-sheet\logs` records the codes the hook was told to watch.

A user shortcut file (`apps/myapp.json`) that adds or edits commands. `aliases` are extra
names the same command goes by, and the search matches them the way it matches `command`:

```json
[
  {
    "name": "Visual Studio Code",
    "category": "Development & Coding",
    "exe": ["code.exe"],
    "collections": [
      {
        "name": "Everyday",
        "shortcuts": [
          { "command": "New file", "keys": "Ctrl+Alt+N" },
          { "command": "Command palette", "keys": "Ctrl+Shift+P", "aliases": ["show commands"] },
          { "command": "Open file", "removed": true }
        ]
      }
    ]
  }
]
```

Rules that keep a background utility alive:

- Only the **compiled-in** dataset can abort startup (a typo there is a bug, caught by
  `cargo test`). Anything the user owns is **degraded, not fatal**: a mistyped chord in a
  user file drops that one row and keeps the working shortcut it would have replaced; an
  unreadable `settings.json` or damaged progress file falls back to defaults; a trigger key
  this build cannot name turns that one trigger mode off. All skips are logged.
- A damaged or half-written file never makes the app vanish on next launch.

The built-in dataset ships 26 profiles with rows across eight categories, and a roster of 64
more app names recognised by process name. Combos are written as human strings
(`"Ctrl+Shift+P"`, `"Win+D"`) and parsed once at load, so a malformed chord fails loudly with
the file and command rather than silently in the UI.

## Architecture

Three threads, one direction of data flow, no shared mutable state:

```
[hook thread]                    [UI thread (eframe)]        [store, on demand]
 LL proc + 50ms timer             frame loop                  load/save JSON
   |  pure hold state machine       |  ^
   |  decides Show/Hide             |  |  pre-created overlay window
   +-- std::mpsc::channel(8) ------>+  |
                 try_recv, drain<=8    +-- SendInput / activate / close
```

- The **hook proc** only pokes a thread-local state machine and sends. Bounded channel with
  `try_send` means a stalled UI drops stale events instead of replaying old overlays.
- The **UI thread** drains the receiver on the frames it runs. While the panel is hidden that is a
  ~8 Hz poll; while it is open there is no poll at all — the hook, the tray and a
  foreground-change watch wake the loop instead, and an idle panel paints 0 frames a second. The
  trigger path itself stays instant either way.
- The overlay window is **created once and hidden**, never destroyed — showing a hidden
  window is a repaint, whereas creating one on the trigger would blow the latency budget on
  first press.

| file | responsibility |
| --- | --- |
| `src/main.rs` | entry: DPI context, catalog load, CLI flags, hook + tray spawn, run loop |
| `src/app.rs` | the `eframe::App`; owns overlay state, routes hook/tray events, show/hide/fire |
| `src/platform/win32.rs` | **the only module with `unsafe` or a Win32 symbol** — hook, injection, tray, DPI, styles |
| `src/detect.rs` | foreground exe name → app profile (pure, tested) |
| `src/data/mod.rs` | serde models, store load/save, atomic write, user-override merge, builtin dataset |
| `src/data/key.rs` | `Combo`/`Key` parsing and canonical ordering |
| `src/ui.rs` | design tokens: palette, sizes, fonts, keycap rendering |
| `src/ui/overlay.rs` | S01 cheat-sheet overlay |

Hard constraints this project holds to:

- **Every Win32 call and every `unsafe` block lives in `platform/win32.rs`**, each with a
  documented safety invariant. Everything above the boundary takes and returns plain Rust
  types; raw handles are wrapped in an opaque `TargetWindow` newtype.
- **Trigger latency ≤ 50 ms** from the hold threshold to the first painted frame.
- **Idle working set ≤ 60 MB** — roughly 5× lighter than the ~326 MB commercial app that
  inspired the concept.
- **No network access, no telemetry.** Local files only.

## Privacy

shortcut-sheet runs entirely offline. It reads the foreground window's process name to pick
a profile, stores its settings and your learning progress in your local app-data folder, and
makes no network connections of any kind.

## Testing

```powershell
cargo test
```

Coverage focuses on the risky parts: the trigger state machine (bare hold shows exactly at the
threshold, a tap never shows, typing a real shortcut disarms, autorepeat doesn't re-arm, a
completed instant chord shows with no delay and no key release hides it, and the same rules for
a hold measured on a *set* of keys — it arms only when the last key lands, its own keys never
cancel it, and letting go of any one of them ends the session), key-name parsing for the three
things a name is used for (a chord we fire, a chord we watch, a set we hold), left/right
modifier codes folding onto one key, combo and hotkey parsing and release order,
foreground-app resolution across both shelves of the dataset, search ranking over labels *and*
aliases, the row index that Enter fires and records as one invariant, the tray tooltip agreeing
with the keys the hook was handed, and the store's degrade-don't-fail behaviour when user files
are damaged.

## License

MIT. See the license field in `Cargo.toml`.
