# Test plan: shortcut-sheet

Build: 0.1.0 release, `x86_64-pc-windows-msvc`, 96 unit tests  Date: 2026-11-02
Env: Windows 11 26H2 desktop, single monitor at 200 % scaling, seed data = the built-in
`src/data/apps.json` (26 profiles over 38 exe names / 511 shortcuts / 64 roster names of which 60
carry an exe / 4 rows carrying 5 aliases) plus whatever is in `%LOCALAPPDATA%\shortcut-sheet`

Result key: **pass** measured and correct · **fail** measured and wrong · **unit** proven by an
automated test, not driven live · **manual** needs a human, reason in the notes · **skip** not
built.

## Parity score

Measured with `python .qoder/skills/replica-diff/parity.py replica/features.csv --markdown`,
which weights must 3 / should 2 / could 1 and counts yes 1 · partial 0.5 · no 0. `skip` rows and
rows where the original does not have the feature are left out of the number, so it only ever
measures parity.

| metric | value |
| --- | --- |
| overall | **75.4 / 100** (30 counted features) |
| must-haves | **14 of 14 done** - nothing in the must column is missing |
| overlay | **100.0** (11 features) |
| shell | **100.0** (2 features) |
| search | **100.0** (1 feature) |
| dataset | 87.5 (4 features) |
| settings | 37.5 (4 features) |
| multi-monitor | 50.0 (1 feature) |
| practice | 10.0 (6 features) |
| backup | 0.0 (1 feature) |
| missing | 12 rows, all `should` or `could` - the practice screens and the dead settings |
| not scored | 5 skipped on purpose (vendor editorial, trademarks, licensing, MSIX, a11y peers) and 2 features that are ours rather than the target's |

The whole score gap is the practice half of the product (S02/S04/S05 were never built) and three
settings fields that are parsed and then ignored. The core loop - hold or press a chord, read the
sheet, fire a shortcut, hand the caret back - is finished and measured.

## What cannot be scripted, and why

The whole of F01 cannot be driven from a script. The hook filters `LLMHF_INJECTED` on purpose -
the original does the same, and without it a held Ctrl would be simulated by every installer and
auto-updater on the machine - so no automation can summon the overlay. That now applies to **both**
trigger modes: the instant chord is the same hook watching a different key set, so a synthetic
Win+Alt is dropped for the same reason. `--peek` is the workaround: it runs the real show path
(`show_at` → `enforce_overlay_style` → `show_without_activating`) with the key-hold step skipped,
and it installs no hook and no tray so a test run cannot interfere with the machine it is checking.

| case | flow | type | steps | expected | auto | result |
| --- | --- | --- | --- | --- | --- | --- |
| F01-H1 | hold Ctrl in a supported app and fire a row | happy | hold Ctrl ~1.5 s in Chrome → S01 opens centred on Chrome → click a row → release Ctrl | overlay appears without stealing focus, the app receives its real combo, overlay gone on release | manual: needs a physical key hold | manual |
| F01-H2 | | happy, same path with the hold simulated away | `pwsh replica/tmp/peek.ps1` | S01 visible, frameless, painted, centred on the foreground window | probe: `overlay rect 720x798` = 360x399 logical, `exstyle=0x08000198` | **pass** |
| F01-H3 | | happy: the click fires | `pwsh replica/tmp/fire.ps1` (real cursor, `mouse_event` on row 0, `RUST_LOG=debug`) | log line `fired combo=Ctrl+N shows=1 fires=1 unsupported=0 hand_backs=0`, no `SendInput dropped events` | probe | **pass** |
| F01-H4 | | happy: the sheet matches the app in front | `pwsh replica/tmp/detect.ps1` | `exe=Some("brave.exe") context=Known("Brave") roster_profile=None` for the window that is actually in front | probe | **pass** |
| F01-E1 | | edge: key released before the threshold | hold Ctrl 400 ms (default threshold 1500 ms) | no overlay at all | unit: `a_tap_never_shows` | **unit** |
| F01-E2 | | edge: exactly at the threshold | hold to 1500 ms | shows once, at the threshold, not before | unit: `a_bare_hold_shows_exactly_at_the_threshold` | **unit** |
| F01-E3 | | edge: key autorepeats while held | hold Ctrl through 30 auto-repeat events | one show, not thirty | unit: `an_ongoing_hold_survives_key_repeats` | **unit** |
| F01-E4 | | edge: a real shortcut typed while held | Ctrl, then Ctrl+C before the threshold | hold disarms, no overlay for a deliberate chord | unit: `typing_a_real_shortcut_while_held_disarms_the_hold`, `key_autorepeat_does_not_re_arm_an_interrupted_hold` | **unit** |
| F01-E5 | | edge: release after a show | Ctrl up | hides exactly once | unit: `releasing_after_a_show_hides_once` | **unit** |
| F01-E6 | | edge: unrelated keys | press A, F13, arrows with no trigger held | nothing happens, no overlay | unit: `keys_without_the_trigger_do_nothing`, `an_unwatched_key_still_interrupts_a_hold` | **unit** |
| F01-E7 | | edge: app not in the dataset | hold Ctrl over an unsupported exe | stays out of the way, `overlay suppressed: no profile for the foreground app` in the log | unit: `unknown_app_is_reported_as_unsupported_not_silently_ignored` · live: `--detect` over a Notepad window read `exe=Some("notepad.exe") context=Unsupported(Some("notepad.exe")) roster_profile=None` | **pass** |
| F01-E8 | | edge: no foreground window at all | hold Ctrl on the bare desktop | no overlay rather than a panel of nothing | unit: `a_missing_process_name_is_unknown` | **unit** |
| F01-E9 | | edge: an app we know but have no rows for | `--peek Slack` (roster name, no profile) | a panel that names the app and says `No shortcuts listed for this app yet`, and the process stays alive | probe: `roster-window.png` at 720x360 device; unit `a_roster_app_opens_a_named_panel_with_nothing_in_it` · 60 of the 64 roster names carry an exe and can resolve this way; the other 4 are names only | **pass** |
| F01-N1 | | negative: search matches nothing | type `zzzz` in the box | "Nothing matches that search" + "Add your own in apps.json", no crash | unit: `non_matching_query_returns_none` · render path not captured | unit |
| F01-E11 | | edge: the list narrows below the selection | highlight row 20, then type `zoom` | the selection comes back inside the new list and Enter still fires something | unit: `narrowing_the_list_below_the_selection_does_not_leave_it_pointing_at_nothing`, `the_selection_clamps_at_both_ends_instead_of_falling_off` | **unit** |
| F01-P1 | | perf: trigger latency | measure hold-release to first painted frame | under 50 ms of the threshold | manual: needs a physical hold | manual |
| F01-P2 | | perf: the hook must never stall the desktop | hold and tap Ctrl while typing normally | no key loss, no input lag | manual: bounded in code by `LowLevelHooksTimeout` + a 50 ms thread tick | manual |
| F02-H1 | learn an app's shortcuts (hub → detail → drill) | happy | tray → S02 → S04 → S05 | drill runs, counters move | skip | **skip** - S02/S04/S05 were never built (see `replica/features.csv`) |
| F03-H1 | progress sticks | happy | fire a row → reopen the app | the command stops counting as new, then as learnt | unit: `learning_master_moves_out_of_learnt`, `learnt_progress_survives_a_reload` | **unit** |
| F04-H1 | edit the dataset | happy | add a chord to `%LOCALAPPDATA%\shortcut-sheet\apps.json` → restart | the user entry wins over the built-in of the same command | unit: `a_user_entry_overrides_the_builtin_chord_of_the_same_command` | **unit** |
| F04-E1 | | edge: a tombstone | `"removed": true` on a built-in command | gone from every list | unit: `a_tombstone_removes_a_builtin_shortcut_from_every_list` | **unit** |
| F04-E2 | | edge: a new app the build never shipped | user file adds Krita | Krita resolves and lists | unit: `a_user_file_can_add_an_app_the_build_never_shipped` | **unit** |
| F04-E3 | | edge: the user writes a profile for a roster name | user file adds a Slack profile on `slack.exe` | slack.exe stops resolving as an empty panel and starts resolving as the app they described | unit: `a_user_profile_wins_the_exe_over_the_roster` | **unit** |
| F04-N1 | | negative: a mistyped chord in the user's file | `"keys": "Ctrl+"` in `apps.json` | the bad row is dropped, the shortcut it was overriding survives, the app still starts | unit: `a_mistyped_override_is_dropped_without_deleting_the_shortcut_it_replaced` | **unit** |
| F04-N2 | | negative: corrupt files everywhere | garbage in `settings.json` and a learning file | app starts on defaults with the built-ins intact | unit: `damaged_files_degrade_the_load_instead_of_killing_the_app` | **unit** |
| F04-N3 | | negative: unknown key token | `"keys": "Ctrl+Bogus"` | parse error names the string; a built-in typo stops the build, a user typo does not | unit: `rejects_unknown_token`, `rejects_modifiers_only`, `rejects_double_plus`, `rejects_a_sequential_chord` | **unit** |
| F05-H1 | retune the trigger | happy | set `{"trigger":"alt","hold_ms":1500}` in `settings.json` → restart | hook registers on Alt with 1500 ms | unit: `settings_round_trip_with_missing_fields` · startup log prints `trigger_vk` and `hold_ms` | **unit** |
| F05-H2 | the instant chord | happy | `{"long_press_enabled":false,"instant_enabled":true,"instant_hotkey":"Win+Alt"}` → press Win+Alt | the sheet appears with no hold, and stays up when both keys come up | manual: needs a physical chord; the rules are unit-tested | manual |
| F05-E1 | | edge: no settings GUI | change the file, not a dialog | works by file edit only - there is no S07 | manual | **partial** (see BUG-010) |
| F05-E2 | | edge: half a chord | press Win, then something else | no show until the whole chord is down | unit: `half_a_chord_summon_nothing_and_a_whole_chord_summons_at_once` | **unit** |
| F05-E3 | | edge: autorepeat of a chord key | hold Win+Alt until both keys repeat | one summon, not one per repeat | unit: `a_repeated_chord_key_does_not_summon_twice` | **unit** |
| F05-E4 | | edge: the chord contains the hold key | trigger Ctrl, chord Ctrl+Alt, press both | the chord wins and the half-hold is disarmed | unit: `a_chord_that_contains_the_trigger_key_wins_over_the_hold` | **unit** |
| F05-E5 | | edge: a chord typo in the settings file | `"instant_hotkey": "Win+Banan"` | the instant mode turns off, the hold still works, the log says why | unit: `a_broken_chord_disables_the_instant_mode_rather_than_the_app` | **unit** |
| F05-E6 | | edge: a one-key "chord" | `"instant_hotkey": "Ctrl"` | refused: a bare modifier would summon the sheet over every copy-paste | unit: `a_single_modifier_is_not_a_hotkey` | **unit** |
| F05-E7 | | edge: both modes off | `{"long_press_enabled":false,"instant_enabled":false}` | no hook is installed at all, the tray still quits, the log says the sheet needs `--peek` | code path in `main.rs`; not driven live | **manual** |
| F06-H1 | follow the OS theme | happy | flip Windows app mode to light | S01 re-tokens to the LIGHT palette on the next hidden frame | code path `detect_system_dark()`; the override is measured, the OS flip is not | **manual** |
| F06-H2 | | happy, both halves of the same switch | `pwsh replica/tmp/theme.ps1` (writes `{"theme":"light"}`, then `"dark"`, restores the file) | the panel takes the token colours of the named theme | probe, re-run on the current tokens: light `surface=#FFFFFF`, dark `surface=#1F1F1F`, both captured at 720x798. The search field is **not** distinguishable by fill in the overlay - `field` and `surface` are the same measured hex in both themes - so the field's evidence is the accent rule and the magnifier in the two PNGs, and `the_palette_mirrors_the_token_file` for the values | **pass** |
| F06-E1 | | edge: contrast in both themes | check every text role on its own background | AA in both | unit: the token audit in `replica/design/tokens.json` (37 pairs, `contrast.py` exits 0) | **unit** |
| -H1 | focus discipline | happy | show and hide the overlay over a focused editor | the caret never moves, `WS_EX_NOACTIVATE` stays set | probe: `peek.ps1` → `focus not stolen: True`, `exstyle=0x08000198` | **pass** |
| -H2 | window chrome | happy | read the styles while the panel is open | no taskbar button, no alt-tab entry, always on top | probe: `TOOLWINDOW=True APPWINDOW=False TOPMOST=True` | **pass** |
| -H3 | leaving | happy | tray → Quit, or wait out the peek deadline | the process exits by itself | probe: `quit on deadline: True` (25 s, then 26 s on the repeat) | **pass** |
| -H4 | leaving | edge: quit asked before the window exists | tray Quit in the first frames | keeps retrying at 120 ms instead of silently doing nothing | code path `try_close` + `warned_no_hwnd` | **manual** |
| -H5 | rendering | happy | capture the window and read the row band | names left, keycaps right, no tofu glyph, list starts at the top | probe: `PrintWindow` shot `replica/screens/clone-S01-peek.png` | **pass** |
| -H6 | rendering | happy: the hover tint | instrument the fill, park the cursor on a row | white at alpha 16, not an opaque band | unit: `hover_fill_stays_translucent_in_both_themes` | **pass** |
| -H7 | rendering | happy: where the list starts | capture the panel on open, before any input | the first collection header and the first row are on screen; nothing opens scrolled | probe: six consecutive captures (`clone-S01-peek`, `search-enter-before`, `search-arrow-before`, both theme shots) all start at row 1 | **pass** - with a caveat: two early captures during the BUG-008 hunt showed a mid-list offset and it has not reproduced since, so this is "not seen again", not "cannot happen" |
| -E1 | DPI | edge: 200 % scaling | compare the logical panel width with the window rect | 360 logical → 720 physical, centred, not half-size | probe: `overlay rect 720x798` | **pass** |
| -E2 | footprint | perf: resident and idle | `pwsh replica/tmp/mem.ps1` | under the relaxed ~60 MB budget | probe, re-measured on this build: 103.1 MB working set / 64.1 MB private / 15 threads idle; 114.0 MB / 69.3 MB with the panel painted (`peek.ps1`) | **fail** (see BUG-006) |
| -E3 | search ranking | happy | type `new` over 30 Chrome rows | "New tab" first (shortest prefix match), then "New window", then "New incognito window" | unit: `prefix_beats_substring`, `a_shorter_name_outranks_a_longer_one_on_the_same_prefix`, `initials_match_word_boundaries`, `rank_preserves_order_for_equal_scores` · re-captured today in `search-enter-filtered.png`: exactly those three, in that order | **pass** |
| -E4 | click into the search box | happy | scripted click in the field band, then type | the overlay takes focus deliberately, the box filters, and the caret comes back | probe `replica/tmp/search.ps1 -Only enter`: `overlay focused: True`, `exstyle=0x00000198 NOACTIVATE=False`, log `overlay takes focus for the search box hwnd=853084 target=Some(592498)`, `search-enter-filtered.png` shows 3 of 30 rows, Enter → `fired combo=Ctrl+T`, `focus restored: True` | **pass** |
| -E5 | leave the box by keyboard | happy | click in, press Escape, read the foreground window | panel hides, nothing fires, the app gets its caret back | probe `-Only escape`: log `overlay hands focus back and hides` then `focus handed back to the target hwnd=592498 exe=Some("brave.exe")`, `overlay still shown: False`, `focus restored: True`, no `fired` line in the run | **pass** |
| -E6 | arrows move the selection | happy | click in, type `new`, press Down once with the pointer parked off the panel, press Enter | row 2 highlighted, Enter fires row 2 and not row 1 | probe `-Only arrow`: `search-arrow-moved.png` shows "New window" selected, log `fired combo=Ctrl+N` - row 2's chord and not row 1's - `hand_backs=1`. See the handle caveat below for why that run prints `focus restored: False` | **pass** |
| -E7 | the caret is back before the chord is sent | happy, and the bug that was fixed | Enter on a row while the search box holds focus | the hand-back happens first, so the app receives the combo and not our own text field | probe: log timestamps - `focus handed back to the target` at `11:48:17.642`, `fired combo=Ctrl+T` at `17.702`, and the fired line itself carries `hand_backs=1`. Unit: `arrows_move_the_one_row_that_enter_fires_and_records` | **pass** (BUG-014) |
| -E8 | a new session inherits nothing | edge: Alt-Tab and summon again | Chrome with a filter and row 6 selected → close → open VS Code | the second sheet is the editor's list, unfiltered, selection at row 1, focus not held | unit: `closing_forgets_the_app_the_search_and_the_selection` | **unit** |
| -E9 | clicking away ends the session | edge: the user leaves while the box holds focus | click in, then click another window | the panel stops floating over somebody else's work | code path `left_the_session`; the 500 ms grace is not measured, and this behaviour is ours and not observed in the target | **manual** |
| -E10 | the search reads a row's other names | happy | over Chrome's 30 rows type `inspect element` - no row is called that | the one row labelled `Developer tools` is what is left, because its alias list contains the phrase | probe: `search-filtered.png`; unit: `typing_the_other_name_of_a_command_finds_the_row`, `an_alias_match_and_the_label_match_reach_the_same_row`, `a_row_can_be_found_by_a_name_it_does_not_display`, `every_alias_of_a_row_is_searchable_not_just_the_first`, `a_match_on_the_visible_name_outranks_the_same_match_on_an_alias`, `alias_discount_never_flips_a_match_into_a_non_match` | **pass** |

## Probe caveat: `focus restored` is a handle comparison

`search.ps1` decides `focus restored` by comparing `GetForegroundWindow()` before the panel opened
with the same reading after it closed. That is the right test for a caret that came back to the
same window, and the wrong test for a chord that opens a **new** one: the arrow run fires `Ctrl+N`,
Brave creates a new window and takes the foreground, and the probe prints `focus restored: False`
over a run in which the hand-back demonstrably happened (`hand_backs=1`, and the log names the
hwnd it activated). The same trap is in the other direction - a fired `Ctrl+T` keeps the handle and
reads `True`.

So: **`focus restored: False` on a run whose fired chord is `Ctrl+N` is not a failed hand-back**, and
the primary evidence for the hand-back is the `focus handed back to the target hwnd=...` line plus
the `hand_backs` counter, not the handle equality. Recorded as BUG-016 so the next person to read a
red `False` here does not go hunting for a bug that is not there.

## Known open items

Six reports stay open after this pass. None of them is in the core loop; all of them are measured
rather than assumed.

- **BUG-006** - idle footprint 103.1 MB working set / 64.1 MB private against the relaxed 60 MB
  budget, re-measured on this build and unchanged since the report was written. The 15 MB figure in
  the original brief belongs to a native-drawn overlay, not to a GL-composited egui window; that
  trade was made on purpose when the renderer was chosen.
- **BUG-007** - the overlay repaints at ~55 fps while it is open, because the visible branch of
  `ui()` ends in an unconditional `ctx.request_repaint()`. Costs battery and fan noise for a lookup
  that lasts two seconds. The fix touches the same repaint discipline that BUG-003 proved
  load-bearing, so it wants its own test cycle.
- **BUG-009** - the held-key path, in both modes, cannot be tested from a script because the hook
  rejects injected input by design. Sixteen unit tests drive the state machine against a synthetic
  key stream and `--peek` drives everything except the press itself, but **the latency budget
  (F01-P1) and the no-focus-loss claim in real use (F01-H1, F05-H2) need a human with a hand on the
  keyboard.** The parity score does not claim otherwise.
- **BUG-010** - three settings are still parsed and then ignored: `fade`, `max_rows`,
  `run_on_startup`. `theme`, `trigger`, `hold_ms`, `long_press_enabled`, `instant_enabled` and
  `instant_hotkey` are all honoured now; there is no S07 to change any of them in, which is why the
  settings area scores 37.5.
- **BUG-013** - the panel keeps its open-time height after a search shrinks the list: three rows at
  the top and 300-odd points of empty panel under them, visible in `search-enter-filtered.png`.
  Cosmetic, but it is the first thing you see when you type.
- **BUG-016** - the probe caveat above: `focus restored` compares window handles, so a chord that
  opens a new window reads as a failure.

## How to re-run

```
cargo test                                    # 96 cases, under a second
python .qoder/skills/replica-diff/parity.py replica/features.csv --markdown
python replica/tmp/csvcheck.py                # the matrix's column count, before parity.py eats it
pwsh replica/tmp/peek.ps1                     # S01 state: styles, focus, screenshot, memory, quit
pwsh replica/tmp/detect.ps1                   # live foreground -> profile resolution, both shelves
pwsh replica/tmp/fire.ps1                     # click a row, read the fired line back
pwsh replica/tmp/mem.ps1                      # idle footprint, normally launched, 12 s in
pwsh replica/tmp/search.ps1                   # the focused box: enter | arrow | escape, or both
pwsh replica/tmp/roster.ps1                   # the empty panel for a roster name
pwsh replica/tmp/theme.ps1                    # light and dark, measured from the captured pixels
pwsh replica/tmp/shoot2.ps1 -App 'Slack' -Tag roster   # CopyFromScreen vs PrintWindow, any app
```

`replica/tmp/*.ps1` are probes, not shipped code, and `replica/tmp/` is gitignored - the commands
above are what this pass ran, but the scripts themselves are not in the repository. Three of them
move the real mouse cursor and two click or type into the foreground app, which sends that app's
real shortcut to it: run them on a machine where that is acceptable, never with unsaved work in the
foreground editor, and **not while someone else is at the keyboard** - `search.ps1` will say
`run discarded` rather than produce a number it cannot stand behind, and that is the third probe in
this file that has been wrong for that reason (BUG-008).

One more probe gotcha, learned while measuring F01-E7: `SetForegroundWindow` from a process that is
not itself in the foreground is ignored by Windows, so a probe that launches a subject app and then
"makes it frontmost" may measure whatever window the human had up instead. The reading is still a
real measurement of the foreground path - it just describes somebody else's window. Say in the probe
which window it found (`unsupported.ps1` prints the title it read) rather than assuming the one it
started.
