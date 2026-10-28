# Recon map: CtrlHelp (Windows desktop)

Scope: the hold-a-modifier **cheat-sheet overlay** plus the **practice/drill** loop and the
**shortcut dataset** that feeds both. Not the vendor's commerce or licensing backend.
For: `shortcut-sheet` — a free, open-source, pure-Rust rebuild with a hard <15 MB idle /
<50 ms trigger budget (the original sits at ~326 MB working set).
Date: 2026-10-26

## Sources

| # | source | URL | notes |
| --- | --- | --- | --- |
| 1 | The installed app itself | `D:\OneDrive\Desktop\CtrlHelp.lnk` | Resolves to AUMID `64360VelerSoftware.CtrlHelp_j80j2txgjg9dj!App`, MSIX `64360VelerSoftware.CtrlHelp_1.0.6.0_x64__j80j2txgjg9dj`. Was running during recon (PID 33484). |
| 2 | Runtime window inspection | `replica/tmp/windows.txt` | EnumWindows over the live process: window classes, styles, rects. Strongest evidence in this map. |
| 3 | User's own app data | `%LOCALAPPDATA%\Packages\64360VelerSoftware.CtrlHelp_j80j2txgjg9dj\LocalState\` | `LearningData\Microsoft Visual Studio Code_Editing.json` — the user's own progress file, read directly. |
| 4 | Official product site | https://ctrlhelp.velersoftware.com/ | Feature set, the 10 app categories, trial/pricing model, accessibility claim. |
| 5 | Public repo README (docs only) | https://github.com/veler/CtrlHelpApp | README read for behaviour captions and license. **Source not read — the app is closed source.** |
| 6 | Public reference screenshots | `replica/screens/reference-*.webp|png` | Vendor-published marketing shots of S01, S02, S04, S05. Reference only, never shipped. |
| 7 | Press coverage | https://www.windowscentral.com/software-apps/this-simple-little-app-should-be-built-into-windows-11 | Independent confirmation of the 80-app / hold-Ctrl loop. |

**Trigger note:** a synthetic `keybd_event` Ctrl-hold did **not** open the overlay. The hook
filters injected input (`LLMHF_INJECTED`). Every geometry figure below therefore comes from
public reference shots and live window inspection, not from a captured live overlay. A
human-held capture is still outstanding — see *Gaps*.

## Core loop

Work in any app → hold the trigger key ~1 s → a focusless overlay of that app's shortcuts
appears centered on it → search or click a command → the overlay fires the real key combo
into the app you were already using → release the key, overlay gone, focus never moved.

## Screens

| ID | screen | route / how to reach | purpose | key components | states seen |
| --- | --- | --- | --- | --- | --- |
| S01 | Cheat-sheet overlay | Hold `Ctrl` ~1 s in any supported foreground app | Look up and fire a shortcut without leaving the app | Title bar (target app icon + name, minimize, close), search box, collection header, shortcut rows, keycap chips | Filled (Excel, 8 rows). Not observed: empty, no-match, unsupported-app, loading |
| S02 | Practice hub / app grid | Tray icon → main window | Browse every supported app grouped by category | Custom title bar with app search + settings gear, category section headers, 4-col app-card grid | Filled. Composite shot shows light and dark simultaneously |
| S03 | App search | Type in the S02 title-bar box | Filter the grid to the app you want | Same grid, filtered | Filtered (inferred from the always-present search box) |
| S04 | App detail / collections | Click an app card on S02 | Pick which collection to drill | Back arrow, app header (icon + name), "collections" label, "Contribute on GitHub" link, "Improve or change shortcuts" button, collection rows with mastered counts | Filled (Excel: 7 collections, 38/9/25/33/31/24/+1 items) |
| S05 | Drill | Open a collection on S04 | Recall and physically type the combo within 2 s | Breadcrumb, command name, large keycaps, coaching tooltip w/ dismiss, "n remaining" + "n mastered" counters, "Skip for now" | Idle prompt. Not observed: correct, wrong, timeout, session complete |
| S06 | Tray flyout | Left/right-click the tray icon | Entry point to practice, settings, quit | Menu items (inferred) | Not observed |
| S07 | Settings | Gear on S02 title bar | Change hotkey, hold duration, theme, startup | Hotkey picker, duration slider, theme selector | **Not observed at all** — existence and the hotkey+duration fields are documented by the vendor README caption only |

## Flows

```
F01 Recall and fire a shortcut while working   [core loop, 0 summon clicks + 1 fire click]
    any app -> hold Ctrl ~1s -> S01 -> (type search) -> click row -> combo injected into app
    -> release Ctrl -> S01 hides
    edge: app unsupported; no search matches; target app loses foreground while overlay open;
          overlay must not take focus (WS_EX_NOACTIVATE); multi-monitor; DPI scaling;
          key released before the 1s threshold -> no overlay at all

F02 Learn an app's shortcuts
    S06 tray -> S02 -> search/pick category -> S04 -> pick collection -> S05 drill
    edge: app with one collection; very long collection list (scroll)

F03 Progress sticks
    S05 correct combo within 2s -> command moves learnt -> mastered
    -> counts on S02 card and S04 rows update ("0 / 51 mastered")
    edge: re-mastering; deleting progress

F04 Edit or add a shortcut
    S04 -> "Improve or change shortcuts" -> local JSON edit -> overlay + drills use it
    -> "Contribute on GitHub" upstream
    edge: invalid key syntax; duplicate command name

F05 Retune the trigger
    S07 -> change hotkey (not Ctrl) and hold duration -> hook re-registers
    edge: hotkey collides with an app's own binding; duration 0 = instant popup

F06 Follow the OS theme
    system light/dark flip -> S01 and S02 re-token both themes
```

## Components

| component | variants | states | used on |
| --- | --- | --- | --- |
| Overlay window chrome | floating (no taskbar entry) | open, closing | S01 |
| Search input | overlay-scoped, hub-scoped | empty, typing, focused (accent underline), no-match | S01, S02 |
| Shortcut row | normal | default, hover, keyboard-selected, firing | S01 |
| Keycap chip | modifier-word (`CTRL`), glyph (`⇧`), character (`4`) | default, pressed/correct, wrong | S01, S05 |
| Large drill keycap | single | idle, held, correct, wrong | S05 |
| Collection group header | overlay, page | static | S01, S04 |
| Category section header + rule | page | static | S02 |
| App card | grid cell | default, hover, mastered>0 | S02 |
| Collection list row | with count + chevron | default, hover | S04 |
| Progress counter | pill, inline text | zero, partial, complete | S02, S04, S05 |
| Coaching tooltip (Flyout) | dismissible | shown, dismissed | S05 |
| Breadcrumb | 2-level | static | S05 |
| Text/link button | accent link, standard, ghost | default, hover, disabled | S04, S05 |
| Tray icon + flyout | single | running, menu open | S06 |
| Toggle / slider / combo | settings | on, off, value | S07 (inferred) |

## Inferred data model

```
Shortcut    command_name, keys[] (VK + modifiers, ordered), collection_id, app_id,
            is_frequent (bool, drives the "Frequent Shortcuts" collection)
            evidence: S01 rows and their keycap chips; S05 prompt "Copy" -> Ctrl+C;
                      vendor's MIT-licensed JSON dataset format
            confidence: high

AppProfile  display_name, category (one of the 10), exe_names[] (case-insensitive,
            matched against the foreground process image name), icon_source,
            collections[]
            evidence: S02 categories and card names; S01 title bar showing the *active*
                      app's name; the site's 10-category list
            confidence: high for name/category, MEDIUM for exe_names[] — the vendor's own
                      mapping was not read; shortcut-sheet defines its own

Collection  app_id, name, order, shortcut_count
            evidence: S04 rows (Frequent Shortcuts 38, Inside the Ribbon 9, Cell Navigation
                      25, Formatting Cells 33, Making Selections 31, Working with Formulas
                      24, Ribbon Tabs ...)
            confidence: high

Learning    app_name, collection_name, learnt_shortcut[] (command names),
            mastered_shortcut[] (command names)
            stored per file: LocalState\LearningData\<App>_<Collection>.json
            evidence: the user's own real file, read verbatim:
                      {"learntShortcut":["Go to Beginning of File","Scroll Line Down"],
                       "masteredShortcut":[]}
            confidence: high (direct observation)

Settings    trigger_vk (default LControl), hold_duration_ms (default ~1000),
            drill_window_ms (2000), theme (system|light|dark), run_on_startup,
            overlay_opacity, max_rows_before_scroll
            evidence: vendor README caption "You can configure the hotkey and duration in
                      the app settings"; the 2-second figure in the drill caption
            confidence: MEDIUM for hotkey/duration/drill window, GUESS for the rest

WindowGeometry  foreground hwnd -> GetWindowRect -> overlay centered on THAT rect,
            ~340x390 px, never on the taskbar, never activated
            evidence: live ex-style 0x08200008 = WS_EX_NOACTIVATE |
                      WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOPMOST on the process's
                      PopupWindowSiteBridge; main window rect 1200x696 @147,147 with
                      WS_EX_TOPMOST while hidden
            confidence: high
```

Relationships: `AppProfile 1-n Collection`, `Collection 1-n Shortcut`,
`AppProfile 1-n ExeName`, `Learning (app, collection) 1-1`, `Shortcut n-1 CommandName`
(command name is the stable key Learning records progress against — observed, not assumed).

## Feature matrix

See `features.csv`. Must: 14, should: 8, could: 6, skip: 5.

## Out of scope (cannot or should not be cloned)

- **The vendor's curated 8,000-shortcut dataset, re-divided as they made it.** Their data
  files are MIT-licensed, so *facts* (which combo does what) are usable with attribution and
  the license text. Their collection curation, ordering and naming is their editorial work —
  shortcut-sheet ships its own dataset and treats the MIT facts as one permitted input.
- **Third-party app icons and names** (Excel, Chrome, Photoshop logos). Trademarks of others.
  The clone ships generic letter-avatar tiles; users may drop in icons they have rights to.
- **Name, logo, "C" tile art, marketing copy, screenshots.** All owned by Veler Software.
- **Trial, purchase, redemption codes, Store acquisition-license plumbing.** Their commerce.
  The clone is free and open source; no license check exists to rebuild.
- **MSIX packaging + WindowsAppSDK runtime.** Replaced by a plain portable exe plus a
  `HKCU\...\Run` startup entry.
- **Screen-reader automation-peer parity.** egui exposes accesskit, but full WinUI-level
  narration parity is not a clone target; basic keyboard operability is.

## Gaps (honest list)

1. S07 settings was never observed — only documented. Field list is partly guessed.
2. S06 tray flyout items not observed.
3. No live capture of S01 (injected input is rejected by the hook). Needs one human-held
   Ctrl capture to measure the real overlay geometry, opacity and shadow.
4. Unsupported-app behaviour (does the overlay stay hidden, or show a generic Windows list?)
   unknown.
5. Whether clicking a row keeps the overlay open for a second pick, or always closes it,
   unknown.

## Size

Screens 7, flows 6, entities 6. Hard parts: the zero-lag LL hook that also rejects injected
input; firing a combo into another process's window without stealing focus; centering on the
*foreground window* rect across DPI scales and multiple monitors; hidden-until-triggered
window that still paints instantly on first show; holding the whole thing under 15 MB with
egui; a JSON dataset + progress store that survives app renames.

Size: **M** for the full pack (a few weeks). The core loop alone — F01 with S01, the hook,
detection, injection and a hand-written dataset for ~5 apps — is **S**: one weekend.
Build F01 first and stop to look at it before touching practice mode.
