# Bug reports: shortcut-sheet

Build under test: 0.1.0 release, `x86_64-pc-windows-msvc`, egui 0.37 / eframe 0.37 / winit 0.30.13.
Env: Windows 11 26H2, single monitor at 200 %. Case IDs refer to `replica/test-plan.md`.

Sixteen reports. Ten are found-and-fixed, six are open: two real defects in the product
(BUG-006 memory, BUG-007 repaint rate), three dead settings (BUG-010), one cosmetic issue waiting
on a design decision (BUG-013), one deliberate filter that makes the core loop untestable from a
script (BUG-009), and one probe that measures the wrong thing (BUG-016). BUG-008 is not an app bug
at all - it is a probe that lied - and it is written up anyway, because it sent an investigation
the wrong way for hours and the next person to touch this will meet it again. BUG-015 is that
finding a second time: a colour measurement in Step 2 quietly invalidated a probe's pixel scan.

The last group (BUG-010 to BUG-016) came out of the parity and engine passes, and several of them
are the same finding wearing different clothes: a field that existed in a struct, a theme nobody
had ever rendered, and a dev mode that could not reach the code it was supposed to be proving.
Wiring the theme is what found the black search box; capturing the light panel is what found it;
the search box is what found the missing saved target. And chasing that saved target is what found
BUG-014 - the only bug in this file a user would have felt, where the shortcut you picked was typed
into our own search box instead of into your app.

---

### BUG-001: the overlay stole focus because winit kept erasing its ex-style

- Severity: S1 - it breaks the one promise the product is built on
- Flow / case: F01 / -H1
- Screen: S01
- Build: 0.1.0  Device: Windows 11 26H2, 200 %

Steps
1. Launch normally, hold Ctrl over a focused editor.
2. Read `GWL_EXSTYLE` on the overlay window after the first show.

Expected: `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST` set, editor keeps the caret.
Actual: `0x00040118` - topmost, but `NOACTIVATE=False TOOLWINDOW=False APPWINDOW=True WS_CAPTION=True`. The overlay activated, the editor lost the caret, and the fired chord went to the overlay instead of the app.
Evidence: `replica/tmp/peek.ps1` before the fix; after it, `exstyle=0x08000198` with `NOACTIVATE=True TOOLWINDOW=True TOPMOST=True APPWINDOW=False` and `focus not stolen: True`.
Suspected cause: `WindowFlags::apply_diff` in winit 0.30.13 (`window_state.rs:317-426`) ends with `SetWindowLongW(window, GWL_EXSTYLE, style_ex)` - a wholesale rewrite from winit's own flag set, on *every* window-state change. Anything we set once is wiped the first time the viewport resizes or shows.
Status: fixed. `apply_overlay_exstyle(raw, no_activate)` is now idempotent, is followed by `SetWindowPos(..., SWP_FRAMECHANGED)`, and is re-asserted every frame the panel is open (`Sheet::enforce_overlay_style`). Two parts of the "before" reading turned out to be winit design, not damage: `WS_CAPTION` stays set because framelessness is implemented in `WM_NCCALCSIZE` (`window_state.rs:435`), and `WS_EX_APPWINDOW` stays set because skip-taskbar goes through `ITaskbarList::DeleteTab`, not the style bits.

### BUG-002: "Quit" from the tray did nothing

- Severity: S2 - the only exit for a windowless resident app
- Flow / case: -H3
- Screen: S06 tray menu
- Build: 0.1.0

Steps
1. Launch normally. 2. Tray icon → Quit shortcut-sheet.

Expected: the process exits.
Actual: nothing visible happened; the process stayed resident.
Evidence: `ViewportCommand::Close` is documented as a no-op for the root viewport in eframe 0.37; the log showed no exit line.
Suspected cause: eframe routes viewport close per-viewport and the root window has no viewport to close.
Status: fixed. `try_close()` resolves the overlay's `HWND` by its unique title and posts `WM_CLOSE` directly, and because the handle is not always known yet the `closing` branch retries every 120 ms rather than giving up.

### BUG-003: the app could not be made to leave once a quit was pending

- Severity: S2
- Flow / case: -H3, -H4
- Screen: S01
- Build: 0.1.0

Steps
1. `shortcut-sheet.exe --peek` 2. Wait out the 25 s peek deadline.

Expected: the process exits on its own.
Actual: `STILL RUNNING after 45 s` from `replica/tmp/quit.ps1`.
Suspected cause: the deadline branch called `request_quit()`, which only set `closing = true`, and then `return`ed - *before* any `ctx.request_repaint()` in that frame. No further frame ran, so the `closing` branch that performs the actual close never executed. A windowless egui app that stops repainting stops existing, functionally.
Status: fixed. `request_quit(&egui::Context)` schedules its own follow-up frame, and the `closing` branch keeps retrying at 120 ms. Measured twice: exit at 25 s and at 26 s, `quit on deadline: True`. Every early `return` from `ui()` now has to own its repaint - that is the rule this bug bought.

### BUG-004: command names were right-aligned

- Severity: S2 - parity with the design tokens
- Flow / case: -H5
- Screen: S01
- Build: 0.1.0

Steps
1. Open S01 over any app. 2. Look at the rows.

Expected: `replica/design/tokens.json` → `overlay_row.layout`: "command name left, keycap cluster right". The vendor reference shot agrees.
Actual: every name sat against the keycaps, so the panel read as one ragged column of text with chips bolted on.
Evidence: the screenshot taken before the fix; `replica/screens/clone-S01-peek.png` after it.
Suspected cause: `ui.put(name_rect, Label)` inherits the *enclosing* layout, which was vertical with centred cross-axis alignment. The `max_rect` moved the box, not the alignment inside it.
Status: fixed with an explicit `ui.scope_builder(UiBuilder::new().max_rect(name_rect).layout(Layout::left_to_right(Align::Center)), ...)`.

### BUG-005: the close button rendered as a tofu box

- Severity: S3
- Flow / case: -H5
- Screen: S01 title bar
- Build: 0.1.0

Steps
1. Open S01. 2. Look at the top-right corner.

Expected: a ✕.
Actual: `□` - the notdef glyph.
Evidence: `epaint-0.37.0/src/text/family.rs:137` renders `.notdef` for any character no loaded font has; egui 0.37's bundled faces (Ubuntu-Light, Hack) have no U+2715 and its emoji/symbol fonts sit behind an optional feature that this build does not enable.
Suspected cause: assumed a glyph that was never in the font set.
Status: fixed by drawing it - two `painter.line_segment` strokes with a hover tint, no font involved. Cheaper than shipping a symbol font for one glyph.

### BUG-006: idle memory is 64 MB private against a 60 MB budget

- Severity: S3 - a stated constraint, missed by ~10 %
- Flow / case: -E2
- Screen: n/a (resident)
- Build: 0.1.0 release

Steps
1. Launch normally, hold nothing, wait 10 s. 2. Read the process.

Expected: the brief asked for under 15 MB; when egui was chosen that was relaxed to ~60 MB on purpose.
Actual: **103.1 MB working set, 64.1 MB private, 15 threads** idle, re-measured on the build after
the engine pass. In peek mode (panel painted): 114.0 MB / 69.3 MB. The figures have not moved across
four rebuilds, which is its own evidence: this is the renderer's floor, not a leak.
Evidence: `replica/tmp/mem.ps1` (normal launch, nothing painted, read after 12 s) and the memory line in `replica/tmp/peek.ps1`; the startup log confirms only the hook thread and the tray thread are added - and that no hook is installed at all when both trigger modes are off.
Suspected cause: a GPU-composited egui viewport. The 15 MB figure belongs to a native XAML or Win32-drawn overlay, not to a GL surface with a tessellation cache; that trade was made explicitly when the renderer was chosen.
Status: open. Options, cheapest first: cap the repaint rate (BUG-007) and re-measure, drop `tracing-appender`'s non-blocking worker thread, or replace eframe's glow context with the software backend (`egui-wgpu`/`software` painting is not smaller, but `egui` without a GL context can be). Getting under 15 MB means not using egui, which is a different decision than a bug.

### BUG-007: the overlay repaints at ~55 fps while it is open

- Severity: S3 - battery and fan noise for a lookup that lasts two seconds
- Flow / case: F01-P2, -E2
- Screen: S01
- Build: 0.1.0 with temporary instrumentation, since removed

Steps
1. `RUST_LOG=debug`, open S01, count paint passes per second.

Expected: frames on demand - the panel is static text with no animation, and `main.rs` says so in its own comment ("Nothing here should ask the backend for a continuous animation loop").
Actual: 269 list passes in a ~10 s run, timestamps 18 ms apart, because the visible branch of `ui()` ends in an unconditional `ctx.request_repaint()`. The hidden path is correct: `request_repaint_after(120 ms)`, 8 Hz.
Evidence: the instrumented log lines from `Sheet::ui`'s visible branch; the DIAG was removed once it had answered BUG-008.
Suspected cause: `request_repaint()` was written to guarantee the peek deadline and the caret blink keep running; it also guarantees a frame when nothing has changed.
Status: open. The fix is `request_repaint_after` with a small delay, plus an immediate repaint only on the paths that change something (hover, query, action, deadline). Not done in this pass because it touches the same repaint discipline that BUG-003 proved load-bearing, and it deserves its own test cycle.

### BUG-008: the probes lied about the panel (not an app bug)

- Severity: S4 for the product, S1 for anyone trusting a screenshot
- Flow / case: -H5, -H6
- Screen: S01
- Build: 0.1.0

Steps
1. Grab the screen with `CopyFromScreen` while the overlay is open. 2. Crop to the window rect.

Expected: what the user sees.
Actual: twice, wrong. Once the crop landed on the desktop with the panel missing - because PowerShell was DPI-unaware and `GetWindowRect` returned half-scale coordinates on a 200 % display. Once a row came back as a solid white band with unreadable text, which looked exactly like an opaque hover fill.
Evidence: `replica/tmp/shoot2.ps1` - the same frame grabbed both ways. `CopyFromScreen` and `PrintWindow(PW_RENDERFULLCONTENT)` agree on every row pixel once the probe is DPI-aware, and the white band does not appear in either. The instrumented build settled it independently: the hover colour is `#FF_FF_FF_10`, i.e. white at alpha 16, exactly as `tokens.json` specifies.
Suspected cause: a GL-composited, per-pixel-alpha window is not reliably captured by a screen BitBlt; strips of the frame behind can come back in place of the panel.
Status: fixed in the tooling. `peek.ps1` now captures with `PrintWindow`, and every probe sets `SetProcessDpiAwarenessContext(-4)` before it touches the desktop. `hover_fill_stays_translucent_in_both_themes` pins the colour so the argument cannot come back as a screenshot again.

Addendum, same category, found later: the input probes lied the same way. Three search runs reported confident results while a person was sitting at the machine. One fired `Ctrl+Shift+N` because the human's pointer was hovering the third row (hover moves the selection - menu semantics, deliberate) and their keyboard had reached our field. One missed the click entirely because the cursor moved in the 300 ms between `SetCursorPos` and the button-down, then reported "overlay focused: False" as if the feature were broken. One found "no visible overlay" because GL init took 6 s and the probe waited 5. `search.ps1` now polls for the window, re-reads the cursor before every click and retries, refuses to type into a box it did not take focus on, parks the pointer off the panel before using the keyboard, and compares the foreground window title before and after the click - if it moved, the run says `run discarded: the desktop is being used` instead of producing a number. A probe that cannot tell that it was interrupted is worse than no probe, because its output looks like evidence.

### BUG-009: the held-key path cannot be tested from a script

- Severity: S4, and by design
- Flow / case: F01-H1, F01-P1, F01-P2
- Screen: S01
- Build: 0.1.0

Steps
1. `SendInput` a Ctrl down-hold. 2. Wait for the overlay.

Expected by the tester: the overlay shows, and the core loop is covered end to end.
Actual: nothing ever shows. The hook callback drops events with `LLMHF_INJECTED` set.
Evidence: `replica/features.csv` row 2 requires it ("must reject injected input like the original does"); dropping it would let any installer or auto-updater summon and drive the sheet.
Suspected cause: not a bug. A deliberate filter that makes the highest-value flow the least testable one.
Status: open as a verification gap, and it now covers **both** trigger modes - the instant chord is
the same hook watching a different key set, so a synthetic Win+Alt is dropped for the same reason a
synthetic Ctrl is. The state machine behind them is covered by sixteen pure unit tests against a
synthetic event stream (`a_bare_hold_shows_exactly_at_the_threshold`, `half_a_chord_summon_nothing_and_a_whole_chord_summons_at_once`,
`a_repeated_chord_key_does_not_summon_twice` and thirteen more), and `--peek` exercises the entire
show path minus the press itself, but **the latency budget (F01-P1) and the no-focus-loss claim in
real use (F01-H1, F05-H2) need a human with a hand on the keyboard.** They are unmeasured, and the
parity score does not claim otherwise.

### BUG-010: four settings were parsed, saved and then ignored

- Severity: S3 - a documented-looking knob that does nothing is worse than no knob
- Flow / case: F05-E1
- Screen: S07 (never built)
- Build: 0.1.0

Steps
1. Set `{"fade": 0.2, "max_rows": 10, "run_on_startup": true, "theme": "light"}` in `settings.json`. 2. Restart.

Expected: a see-through panel, a capped list, an HKCU Run entry, a forced light panel.
Actual: none of the four changed anything when this was written. `hold_ms` and `trigger` *were* honoured (they go to `spawn_hook`), and the rest were read into `Settings` and dropped on the floor. One of the four has since been wired - see Status.
Evidence: `grep` for the field names found them only in `src/data/mod.rs` - the struct, the defaults and the round-trip test - and nowhere in `app.rs`, `ui/overlay.rs` or `platform/win32.rs`. Still true for `fade`, `max_rows` and `run_on_startup`; `theme` now reaches `Sheet::wants_dark()`.
Suspected cause: the schema was written from the vendor's documented settings list before the build was cut down to the core loop.
Status: open, narrowed to **three** fields. `theme` is wired as of that pass - `Sheet::wants_dark()` reads it and `replica/tmp/theme.ps1` measures both overrides (the figures it printed then, `#F7F7F7`/`#FFFFFF` light and `#1F1F1F`/`#2A2A2A` dark, are the *pre-measurement* palette; the same probe on the current tokens reads `surface=#FFFFFF` light and `surface=#1F1F1F` dark, and the overlay's field is no longer separable from its panel by fill at all - see BUG-015). `fade`, `max_rows` and `run_on_startup` are the dead ones left.

Amendment from the engine pass: the trigger side of this report is now six wired fields, not three.
`long_press_enabled`, `instant_enabled` and `instant_hotkey` join `trigger`, `hold_ms` and `theme` in
reaching the hook, and each has a test that fails if it stops (`both_trigger_modes_are_configurable_and_independent`,
`an_off_instant_mode_never_reads_the_chord`, `a_broken_chord_disables_the_instant_mode_rather_than_the_app`,
`a_fresh_install_takes_its_settings_from_the_dataset`). The reason this row is still open is the
missing S07: a setting you can only change by hand-editing JSON is a documented-looking knob with no
dialogue behind it, which is exactly the complaint this report was written about.

### BUG-011: the light theme drew a black search box

- Severity: S2 - the whole light half of a must-have feature had never been painted
- Flow / case: F06-H2, -H5
- Screen: S01
- Build: 0.1.0, found in `replica/screens/clone-S01-light.png`

Steps
1. Set `{"theme": "light"}` in `settings.json`. 2. `--peek`, and look at the search field.

Expected: a white field with grey placeholder text on a `#F7F7F7` panel.
Actual: a near-black slab. Everything else - rows, headers, keycaps, title - came from the token palette and was correct, which made the one wrong widget look like a deliberate design choice rather than an oversight.
Evidence: the capture. The pixel reading after the fix is the field's modal colour over a scanned band: `search-field=#FFFFFF` in light and `#2A2A2A` in dark, i.e. `color.overlay.card` in both.
Suspected cause: egui's widget colours (`Visuals`) are a second system, and only `Palette` was being switched. `TextEdit` fills its frame with `visuals.extreme_bg_color`, which is near-black in `Visuals::dark()` - the mode egui ships with. Our palette never touched it.
Status: fixed, and fixed at the right level: `ui::visuals(&Palette, dark)` builds the `Visuals` *from the tokens* (field, panel, borders, hint text, selection, accent states) instead of swapping in one of egui's two stock themes, and `app.rs` applies it on the first frame and on every theme flip. `egui_visuals_are_painted_from_the_palette` asserts each mapped role in both themes, so a new widget cannot silently fall back to egui's colours.
Amendment: the two field hexes quoted above came from the provisional palette. After the Step 2 measurements the overlay's `field` and `surface` are the **same** colour in both themes (#FFFFFF light, #1F1F1F dark - the target's own recessed field is drawn with a rule and a magnifier, not a tint), so this fix is now verified by the rule, the placeholder contrast and `the_palette_mirrors_the_token_file` rather than by a fill sample. The bug it reported is still fixed; the probe that reported it changed under it (BUG-015).

### BUG-012: peek mode could not reach the code the focus claim lives in

- Severity: S3 - a verification hole, not a user-facing defect
- Flow / case: F01-H2, -E4
- Screen: S01
- Build: 0.1.0

Steps
1. `--peek`, click into the search box, press Escape.

Expected: the panel closes and the caret goes back to the app it came from.
Actual: the panel closed and the caret went nowhere. `restore_focus()` activates `self.target`, and `peek_show()` never set it - only `on_show()` (the hook path) did, and the hook path cannot be driven from a script (BUG-009).
Suspected cause: peek was written to exercise the *show* path (`place_and_show`) and nobody noticed that the hand-back needs the same piece of state the show path saves.
Status: fixed. `peek_show` now records the foreground window as the target, which is what makes "clean window focus restoration" measurable at all. Measured on all three exits: `focus restored: True` after Enter, after Down+Enter, and after Escape, each with the foreground window read before the panel opened and again after it closed.

Amendment from the engine pass: the claim above was two parts right and one part luck. The Down+Enter
run has since been re-measured and prints **`focus restored: False`** - not because the caret failed to
come back, but because the chord that run fires is `Ctrl+N`, Brave answered by making a new window, and
the probe compares window handles (BUG-016). The exit set also grew: there are now four ways out of a
session (browse-mode click, Enter on a filtered row, Arrow then Enter, Escape) plus the two the peek
deadline and a click away give you, and they all route through one `dismiss()` that hands the focus back
*before* anything else. `peek_show` records the foreground exe as well as the window, so `--peek Slack`
is validated against the dataset instead of painting a panel for an app that does not exist, and every
hand-back is counted (`hand_backs`) so the ordering is auditable in one log line rather than inferred
from two. What this report was actually about - that the dev flag could not reach the code the claim
lives in - is still fixed.

### BUG-013: the panel keeps its height after a search shrinks the list

- Severity: S4 - cosmetic, and visible the moment you type
- Flow / case: -E4, -H5
- Screen: S01
- Build: 0.1.0

Steps
1. Open S01 over Chrome (30 rows), click the box, type `new` (3 rows).

Expected: the panel closes up around the three rows.
Actual: three rows at the top and 300-odd points of empty panel below them, visible in `replica/screens/search-enter-filtered.png` (re-captured on this build at 720x798 - the defect looks exactly as it did when it was written up).
Suspected cause: `panel_size()` is computed once, in `show_at`, from `row_count()` at open time. The filter changes `rows` and nothing recomputes the viewport.
Status: open. The fix is one `InnerSize` command on the frames where `row_count()` changed - but it resizes a topmost window the user is looking at, so it wants a decision about re-centring (does the panel grow downward, or stay centred?) before it is written. Recorded rather than guessed at.

### BUG-014: the shortcut you picked was typed into our own search box

- Severity: S1 - it breaks the one promise the focused path is built on
- Flow / case: -E7, F01-H3
- Screen: S01
- Build: 0.1.0, found while wiring the focus hand-back in the engine pass

Steps
1. `--peek` over a browser. 2. Click into the search box - the overlay takes the caret, on purpose. 3. Type `new`. 4. Press Enter.

Expected: the caret goes back to the app first, then `Ctrl+T` is typed into the app, and a new tab opens.
Actual: the keys were sent while the overlay still held the caret. `SendInput` delivers to whatever
has keyboard focus, and that was our `TextEdit`, so the chord landed in the box the user had just typed
into instead of in their app. Nothing in the panel closes the loop on this: `Ctrl+T` in a text field
does nothing visible, so the run *looks* like a pass - the panel closes, the log says `fired`, and the
browser sits there with no new tab.
Evidence: the old ordering is visible in the code that shipped - `self.fire(c)` ran, and the re-activation
ran after it, inside `on_hide()` -> `restore_focus()`. After the fix the same run logs `focus handed back
to the target hwnd=592498 exe=Some("brave.exe")` at `11:48:17.642` and
`fired combo=Ctrl+T shows=1 fires=1 unsupported=0 hand_backs=1` at `17.702` - the hand-back is 60 ms
*earlier* than the send, in the same frame, and the counter that proves it is printed inside the `fired`
line rather than left to be inferred from two records. The arrow run says the same thing one row later:
hand-back at `11:48:47.747`, `fired combo=Ctrl+N ... hand_backs=1` at `47.807`.
Suspected cause: the fire branch was written in the order the *state* was convenient - do the thing,
then clean up - and nothing in the code or the tests said the app has to own the caret before the keys
go. The browse-mode click path never had this bug, which is how it hid: with no click in the box the
overlay never takes focus, so sending first is harmless and `hand_backs=0` is the correct reading.
Status: fixed. `give_back_focus()` runs before `fire()` on every path that can fire - Enter, the arrow
key that turns a highlighted row into a `Fire`, and the row click - and `give_back_focus()` no-ops when
the session never had focus, so the browse path stays untouched and keeps reading `hand_backs=0`. The
same call re-asserts `WS_EX_NOACTIVATE` on the spot instead of waiting for the next frame's per-frame
enforcement, which is what makes the deliberate focus exception end with the session. Covered by
`arrows_move_the_one_row_that_enter_fires_and_records` for the selection half and measured live for the
ordering half; the app-side effect (did the tab actually open) is still a human check.

### BUG-015: the search probe could no longer find the search box (tooling)

- Severity: S4 for the product, S2 for anyone trusting a probe
- Flow / case: -E4, -E6
- Screen: S01
- Build: 0.1.0, found when the measured palette landed

Steps
1. Re-run `replica/tmp/search.ps1` after the token colours were replaced with the ones read off the target's own pixels.

Expected: it clicks the field and drives the keyboard path, as it had before.
Actual: `no field band found (longest run: 2 rows)` - the probe refused to click at all. It had located
the box by scanning a column for a run of pixels whose colour differed from the panel around them, and
in the measured palette the overlay's `field` and `surface` are the *same* hex (#FFFFFF light,
#1F1F1F dark). The thing being looked for was gone; nothing was broken.
Suspected cause: the probe encoded a design assumption - a search field is a tinted pill - that the
measurement said the target does not have. The target's field is drawn with a rule and a magnifier.
Status: fixed in the tooling. The click point is now computed from `tokens.json` geometry instead of
from a colour scan: title 36 + gap 2 + half the 38-point band = logical y 69, scaled by `width / 360`.
Same category as BUG-008, same lesson: when the design changes under a probe, the probe's silence is
not a result.

### BUG-016: `focus restored` compares handles, so a chord that opens a window reads as a failure

- Severity: S4 for the product, S2 for anyone reading a red - it invites hunting a bug that is not there
- Flow / case: -E6, -E4
- Screen: S01
- Build: 0.1.0

Steps
1. `pwsh replica/tmp/search.ps1 -Only arrow` - click in, type `new`, Down, Enter. The row that fires is "New window", so the chord is `Ctrl+N`.

Expected: a report of whether the caret came back to the app.
Actual: `focus restored: False`, on a run where the hand-back demonstrably happened - the same run logs
`focus handed back to the target hwnd=592498 exe=Some("brave.exe")` and prints `hand_backs=1`. Brave
answered `Ctrl+N` by creating a new window, that window took the foreground, and the probe's assertion
is `GetForegroundWindow()` after == `GetForegroundWindow()` before.
Suspected cause: handle equality is a proxy for "the focus came back" that breaks exactly when the
shortcut works - a chord that changes which window is front is the point of the product. The mirror
image is `Ctrl+T`, which keeps the handle and reads `True`.
Status: open. The honest fix is to assert on the `focus handed back` line and the `hand_backs` counter,
and keep the handle comparison as a secondary signal that says what changed. Not done in this pass
because the handle check is also the only thing that catches a *real* focus leak, and trading it away
needs a run against a deliberately broken build to prove the replacement still bites. Until then:
**a `False` on a run whose fired chord opens a window is not a failed hand-back** - see the caveat
block in `replica/test-plan.md`.
