# Architecture: shortcut-sheet

Pure Rust, edition 2021, `x86_64-pc-windows-msvc`. Clean-room rebuild of the CtrlHelp
core loop. Inputs: `replica/recon.md`, `replica/design/tokens.json`.

## Decided constraints

| decision | value | why |
| --- | --- | --- |
| Renderer | `eframe` + `egui`, glow backend | user-approved |
| Idle memory budget | **≤ 60 MB**, not 15 MB | a `winit`+`glow` window carries driver working sets no amount of Rust leanness removes; still 5× under the original's measured 326 MB |
| Trigger latency | **≤ 50 ms** from hold-threshold to first painted frame | hook proc does no work; overlay window is pre-created and only shown |
| Search focus | **activate on click into the field** | user-approved; Escape/blur restores focus to the saved target hwnd |
| Hook input policy | reject `LLMHF_INJECTED` | matches observed original behaviour, and stops our own injected combos re-triggering us |
| Packaging | portable `.exe` + `HKCU\...\Run` | no MSIX, no WindowsAppSDK, no runtime installer |

## Module map

```
src/
  main.rs             entry: single instance, DPI context, wiring, run loop
  app.rs              eframe::App impl; owns View state, routes events
  platform/win32.rs   ALL unsafe. Every Win32 call, one place, each with a
                      documented safety invariant. Nothing above this is unsafe.
  input/hook.rs       safe wrapper: hook thread, hold state machine, event channel
  input/inject.rs     safe: Combo -> SendInput sequences
  detect.rs           safe: foreground exe name -> AppProfile
  data/schema.rs      serde models, no logic beyond validation
  data/store.rs       load/save, atomic write, user override merge
  data/builtin.rs     include_str! dataset
  ui/tokens.rs        tokens.json mirrored as consts; the only place a color literal appears
  ui/overlay.rs       S01 cheat sheet
  ui/keycap.rs        key rendering rules (pure, unit tested)
  tray.rs             tray icon + menu
```

Rule: `platform/win32.rs` is the only file with `unsafe`, and the only file that names a
Win32 symbol. Everything above it takes and returns plain Rust types (`hwnd::HWND` is
wrapped in an opaque `TargetWindow` newtype so no other module can dereference it).

## Threading model

Three threads, one direction of data flow, no shared mutable state.

```
[hook thread]                    [UI thread (eframe)]        [store, on demand]
 LL proc                          frame loop                   load/save JSON
   |  pure state machine            |  ^
   |  decides Arm/Show/Hide         |  |  pre-created overlay window
   +-- std::mpsc::sync_channel(8) ->+  |
                try_send                +-- Activate/Fire(TargetWindow)
```

- The hook proc **only** mutates a `static AtomicUsize` and `try_send`s. No allocation, no
  lock, no file I/O, no UI call. Exceeding `LowLevelHooksTimeout` (300 ms default) makes
  Windows silently uninstall the hook — so the proc must stay under ~1 ms or the product
  degrades invisibly. This is the one place a "it worked on my machine" bug hides.
- Channel is bounded at 8 with `try_send`: if the UI is stalled, we drop events rather than
  grow a queue that would replay stale overlays. Drop-oldest, never block the hook.
- The UI thread polls the receiver each frame and drives show/hide.
- Store access happens on the UI thread only, at startup and on mutation. No DB, no async
  runtime — a JSON read at startup is not worth a thread pool.

## The hold state machine (the heart, and pure)

`input::HoldEvent` in, `input::Command` out. No Win32 types, so it is unit-testable on any
platform:

| event | guard | action |
| --- | --- | --- |
| `TriggerDown` | not injected | `arm`, `t0 = now` |
| `OtherKeyDown` | armed | **`disarm`** — a real shortcut is being typed, not a lookup |
| `Tick` | armed && held && now-t0 ≥ hold_ms | `Show(target)` |
| `TriggerUp` | any | `disarm`, `Hide` |
| `Activate(combo)` | shown | `Fire(target, combo)`, `Hide` |
| `FocusSearch` | shown | `ActivateWindow(overlay)` |

The `OtherKeyDown ⇒ disarm` transition is the decision that makes the app usable: without
it, every `Ctrl+C` you type pops the overlay over your work.

## Overlay window lifecycle

Created once at startup, never destroyed, never shown on the taskbar:

1. `WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST`, no decorations, initially hidden.
2. On `Show`: read the **foreground window rect** (not the work area), center on it, clamp
   to the monitor, `SetWindowPos(SWP_NOACTIVATE | SWP_SHOWWINDOW)`.
3. On `Hide`: `ShowWindow(SW_HIDE)`.
4. On search click: `SetForegroundWindow(overlay)` — the one deliberate focus change, and
   we save the previous hwnd to restore it.

Pre-creating is what buys the latency budget: showing a hidden window is a repaint, while
creating one on the trigger path costs window-class registration, GL context init and font
atlas upload — easily >50 ms on the first press.

## Injection

`Fire(target, combo)` builds a `KEYEVENTF_UNICODE`-free sequence of down/down/up/up over
`VK_CONTROL`/`VK_MENU`/`VK_SHIFT` + the trigger key, via `SendInput`. Because the overlay
never holds focus on the browse path, the target is still foreground and the input lands in
it with no re-targeting. We do **not** `PostMessage` to the hwnd: `PostMessage` bypasses
the app's own accelerator table and silently fails in many apps; `SendInput` is what the
OS-level shortcut path actually honours.

## Data

```
%LOCALAPPDATA%\shortcut-sheet\
  settings.json          trigger, hold_ms, theme, startup, opacity
  apps/<app>.json        user overrides, same shape as builtin
  learning/<App>_<Collection>.json   { learnt: [..], mastered: [..] }
  logs/ss-<utc>.log
```

Builtin dataset is `include_str!`-compiled in, so the exe runs with an empty data dir.
User files **merge over** builtin by `app + collection + command_name`; a tombstone
(`"removed": true`) deletes rather than shadows. Learning keys on `command_name`, matching
the observed original format — deliberate, since it survives our own re-curation of a
collection, but breaks on renames. Recorded as a known tradeoff.

`Combo` parses from a human string (`"Ctrl+Shift+>"`, `"Win+D"`) into `Vec<Key>` at load
time, so a malformed dataset fails loudly at startup with the file and line, not silently
in the UI.

## Dependencies

| crate | why | gate |
| --- | --- | --- |
| `eframe` (glow, no default features) | immediate-mode UI, viewport API | the approved renderer choice |
| `egui` | widgets | with eframe |
| `windows` (Win32_Foundation, _UI, _UI_Input, _Graphics_GDI, _Storage_Xps, _System_LibraryLoader, _UI_Shell, _UI_HiDpi) | hook, foreground, injection, DPI, tray | one crate, feature-gated to trim the graph |
| `serde` + `serde_json` | data | required |
| `thiserror` | error types | small |
| `tracing` + `tracing-subscriber` (env-filter, json off) | the original logs to `LocalCache\Logs`; parity | only if the tree stays reasonable |
| `tauri-winrt-notification`? | **no** — out of scope for the core loop | rejected |

No C++ toolchain dependency beyond the MSVC linker Rust already needs. No Python at
runtime. `cargo tree -d` checked for duplicate versions.

## Deliberate divergences from the original

1. Keycaps render **all** modifiers as words; the original mixes `CTRL` (word) with a Shift
   glyph. Consistency beats parity here.
2. No third-party logos — letter-avatar tiles. Legal, and it removes an icon-licensing
   problem the original has to carry.
3. Portable exe, not MSIX. Costs us auto-update, buys a zero-install open-source release.
4. Own dataset, MIT upstream facts importable later with attribution (`could` row).

## Build order (vertical slice first)

1. `platform/win32.rs` + `input/hook.rs` → prove the hook fires and survives 300 ms timeout
   scrutiny, with **no UI at all** (log lines).
2. `detect.rs` + `data/*` → resolve `brave.exe` → a profile, from a real JSON file.
3. `ui/overlay.rs` pre-created + show/hide → **the loop is now demoable.** Stop and look.
4. `input/inject.rs` → `Ctrl+C` actually copies in the target app. This is the moment the
   product exists.
5. `keycap.rs` + tokens → visual parity pass.
6. `tray.rs`, settings, then practice mode (S02–S05) as its own slice.

Steps 1–4 are the entire risk. Practice mode is ordinary CRUD UI and carries none.
