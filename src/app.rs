//! The single eframe window, which *is* the overlay.
//!
//! Evidence for this shape: the original keeps one hidden `WinUIDesktopWin32WindowClass`
//! window carrying `WS_EX_TOPMOST` and shows it on the trigger, rather than creating a
//! window per press. Pre-creating is what keeps the first press as fast as the hundredth.

use std::time::{Duration, Instant};

use crate::data::{Catalog, Combo, Theme};
use crate::detect::{self, Context};
use crate::platform::{self, HookEvent, Summon, TargetWindow, TrayEvent};
use crate::ui::overlay::{Action, KeyFlags, Overlay};
use crate::ui::{self as uitok, Palette};

/// Unique enough that FindWindowW cannot plausibly hit someone else's window.
pub const WINDOW_TITLE: &str = "shortcut-sheet-overlay-7c1f";

pub struct Sheet {
    catalog: Catalog,
    overlay: Overlay,
    events: Option<std::sync::mpsc::Receiver<HookEvent>>,
    tray: Option<std::sync::mpsc::Receiver<TrayEvent>>,
    /// `--peek`: the app to show on the first frame, without a key being held.
    peek: Option<String>,
    /// And the moment that peek closes itself, so a verification run cannot linger on screen.
    peek_until: Option<Instant>,
    /// Quit was asked for (tray menu, or a peek that ran out its time) and is waiting on a
    /// window handle to close.
    closing: bool,
    /// The app the user was in when the overlay came up. Input goes back here.
    target: Option<TargetWindow>,
    target_exe: Option<String>,
    /// Which of the two trigger modes opened this session, because only one of them ends on
    /// a key release.
    summoned: Option<Summon>,
    /// When the overlay deliberately took the caret. The activation the OS was asked for is
    /// asynchronous, so "the caret is not here" is not a departure until it has had time to
    /// land - without this, the click into the search box would dismiss itself on the frame
    /// after it asked for focus.
    focused_since: Option<Instant>,
    hwnd: Option<isize>,
    /// So "the window is not there yet" is said once per run, not once per frame.
    warned_no_hwnd: bool,
    /// egui's widget visuals have been synced to the palette at least once.
    styled: bool,
    palette: Palette,
    dark: bool,
    last_fire: Option<Instant>,
    /// How the hook and tray threads ask for a frame. Holding it is what lets the open panel
    /// stop painting to find out whether something happened (BUG-007).
    wake: platform::UiWake,
    /// The context only exists from the first frame on, and only the first hand-over matters.
    wake_bound: bool,
    stats: Stats,
}

#[derive(Debug, Default)]
pub struct Stats {
    pub shows: u64,
    pub fires: u64,
    pub unsupported: u64,
    /// Sessions that ended by handing the caret back. The focus promise is the one thing in
    /// this app that has no visible symptom when it breaks, so it gets a counter.
    pub hand_backs: u64,
}

impl Sheet {
    /// `events` and `tray` are `None` in peek mode, where no hook is installed at all.
    pub fn new(
        catalog: Catalog,
        events: Option<std::sync::mpsc::Receiver<HookEvent>>,
        tray: Option<std::sync::mpsc::Receiver<TrayEvent>>,
        peek: Option<String>,
        wake: platform::UiWake,
    ) -> Self {
        let dark = false;
        Self {
            catalog,
            overlay: Overlay::default(),
            events,
            tray,
            peek,
            peek_until: None,
            closing: false,
            target: None,
            target_exe: None,
            summoned: None,
            focused_since: None,
            hwnd: None,
            warned_no_hwnd: false,
            styled: false,
            palette: if dark { uitok::DARK } else { uitok::LIGHT },
            dark,
            last_fire: None,
            wake,
            wake_bound: false,
            stats: Stats::default(),
        }
    }

    /// Which palette to paint with: the user's override when `settings.json` names one, the
    /// OS app mode otherwise. `Theme::System` is the default, so the override branch is the
    /// only way this ever deviates from the desktop - and reading the field here is what makes
    /// it a setting rather than a string in a file (BUG-010 had four of those).
    fn wants_dark(&self) -> bool {
        match self.catalog.settings.theme {
            Theme::Light => false,
            Theme::Dark => true,
            Theme::System => detect_system_dark(),
        }
    }

    fn drain_events(&mut self, ctx: &egui::Context) {
        // Take at most a handful per frame: a backlog of stale Show events replaying
        // overlays over whatever the user is doing now would be worse than dropping them.
        // They are collected first because handling one mutates self, which owns the receiver.
        let mut batch: Vec<HookEvent> = Vec::with_capacity(8);
        if let Some(rx) = self.events.as_ref() {
            for _ in 0..8 {
                match rx.try_recv() {
                    Ok(event) => batch.push(event),
                    Err(_) => break,
                }
            }
        }
        for event in batch {
            match event {
                HookEvent::Show { target, via } => self.on_show(ctx, TargetWindow(target), via),
                HookEvent::Hide => self.on_hide(ctx),
            }
        }
    }

    fn on_show(&mut self, ctx: &egui::Context, target: TargetWindow, via: Summon) {
        if self.overlay.is_open() {
            // A second complete press of the summon chord. What it means depends on where the
            // user is standing: over the window the sheet is already about, it is the keyboard's
            // way of putting the sheet away - the only one an instant session has in browse mode,
            // where no key is being held to release. Over a different window, it is a request for
            // that app's sheet, and dismissing instead would make the hotkey look dead.
            //
            // The UI owns this truth rather than the hook, because a "shown" flag on the hook
            // side goes stale the moment the panel is dismissed by Escape, by a click, or by
            // firing a row - and the next press would then do nothing at all.
            if via != Summon::Chord || self.summoned != Some(Summon::Chord) {
                return; // already up, and this is not the chord asking for a change
            }
            let moved_on = self.target.map(|t| t != target).unwrap_or(false);
            self.dismiss(ctx);
            if !moved_on {
                tracing::debug!("summon chord pressed again over the same window; sheet put away");
                return;
            }
            tracing::debug!("summon chord pressed over another window; showing that app");
        }
        let exe = platform::process_name_of(target);
        match detect::resolve(&self.catalog, exe.as_deref()) {
            // Both named cases show a panel. A roster app has no rows, so it shows the empty
            // state under its own name - "we know this window and have nothing for it yet" has
            // to look different from "this key does nothing here".
            Context::Known(name) | Context::Empty(name) => {
                self.target = Some(target);
                self.target_exe = exe;
                self.summoned = Some(via);
                self.overlay.open(&self.catalog, &name);
                self.stats.shows += 1;
                tracing::debug!(
                    app = %name,
                    via = ?via,
                    rows = self.overlay.row_count(),
                    "overlay shown"
                );
                self.place_and_show(ctx, target);
            }
            Context::Unsupported(_) => {
                // Behaviour for an app we do not know is a recon gap (recon.md #4); we
                // choose to stay out of the way rather than show an empty panel.
                self.stats.unsupported += 1;
                tracing::debug!(?exe, "overlay suppressed: no profile for the foreground app");
            }
            Context::Unknown => {
                self.stats.unsupported += 1;
            }
        }
    }

    /// The panel height the current rows earn, capped by the overlay's max height.
    fn panel_size(&self) -> egui::Vec2 {
        egui::vec2(
            uitok::size::OVERLAY_WIDTH,
            (uitok::size::OVERLAY_MAX_HEIGHT
                * (self.overlay.row_count() as f32 / 12.0).clamp(0.35, 1.0))
                .max(180.0),
        )
    }

    /// Size and place the panel, then show it without taking the caret. This is the whole
    /// show path in one place, so the peek mode below exercises the production code rather
    /// than a copy of it.
    fn show_at(&mut self, ctx: &egui::Context, panel: egui::Vec2, pos: egui::Pos2) {
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(panel));
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));

        // Undo the activation winit performs on show: the target must keep the caret so
        // the fired chord lands in it.
        self.enforce_overlay_style();
        if let Some(h) = self.hwnd {
            platform::show_without_activating(h);
        }
        ctx.request_repaint();
    }

    /// The overlay's promise - "you never left your app" - lives in three ex-style bits, and
    /// winit overwrites them with its own computed value every time it touches the window.
    /// So this is re-asserted while the panel is on screen rather than set once at show time:
    /// one read per frame, and no write at all once the bits have settled.
    fn enforce_overlay_style(&mut self) {
        self.ensure_hwnd();
        if let Some(h) = self.hwnd {
            platform::apply_overlay_exstyle(h, !self.overlay.focused);
        }
    }

    /// Center on the *foreground window* rect, in logical points - not on the screen, which
    /// is what makes the sheet read as part of the app you are holding a key inside.
    fn place_and_show(&mut self, ctx: &egui::Context, target: TargetWindow) {
        let Some((x, y, w, h)) = platform::rect_of(target) else { return };
        let scale = platform::dpi_of(target) / 96.0;
        let panel = self.panel_size();
        let (cx, cy) = ((x as f32 + w as f32 / 2.0) / scale, (y as f32 + h as f32 / 2.0) / scale);
        self.show_at(ctx, panel, egui::pos2(cx - panel.x / 2.0, cy - panel.y / 2.0));
    }

    /// `--peek`: bring the panel up without a key being held, for screenshots and for
    /// scripted checks - the hook refuses injected input by design, so there is no other way
    /// to see S01 from a machine. Lands on the current foreground window, and near the top-left
    /// when there is no window in front at all.
    fn peek_show(&mut self, ctx: &egui::Context, app: &str) {
        if !self.catalog.knows(app) {
            // Named on a command line, so a typo is the honest explanation, and the only
            // useful answer is to say it in the log and leave rather than show a blank panel.
            tracing::error!(app, "--peek named an app that is not in the dataset");
            self.request_quit(ctx);
            return;
        }
        self.overlay.open(&self.catalog, app);
        self.stats.shows += 1;
        match platform::foreground_window() {
            Some(target) => {
                // Recorded, which `on_show` also does and this path did not. Without it, Escape
                // out of a focused search box closes the panel and leaves the caret nowhere: the
                // hand-back has no saved window to give it to, and "clean focus restoration" -
                // the one thing the overlay promises - could not be measured at all, because the
                // hook that normally sets `target` ignores injected input by design.
                self.target = Some(target);
                self.target_exe = platform::process_name_of(target);
                self.place_and_show(ctx, target);
            }
            None => {
                let panel = self.panel_size();
                self.show_at(ctx, panel, egui::pos2(80.0, 80.0));
            }
        }
        self.peek_until = Some(Instant::now() + Duration::from_secs(25));
    }

    /// Ask to leave. Deferred to the frame check in `ui`, because the window handle is not
    /// always known yet - and a quit that silently does nothing is the worst possible answer
    /// from the only exit a windowless resident app has.
    ///
    /// It has to schedule the follow-up frame itself. The real bug this line prevents: the
    /// deadline path set `closing` and returned early, before any `request_repaint`, so no
    /// next frame ever ran and the app sat on screen forever.
    fn request_quit(&mut self, ctx: &egui::Context) {
        self.closing = true;
        ctx.request_repaint();
    }

    /// Post WM_CLOSE to our own window. True once it has actually gone out.
    fn try_close(&mut self) -> bool {
        self.ensure_hwnd();
        match self.hwnd {
            Some(h) => {
                platform::request_close(h);
                true
            }
            None => {
                tracing::warn!("quit requested before the overlay window could be found");
                false
            }
        }
    }

    fn ensure_hwnd(&mut self) {
        if self.hwnd.is_some() {
            return;
        }
        self.hwnd = platform::find_window_by_title(WINDOW_TITLE);
        if self.hwnd.is_none() && !self.warned_no_hwnd {
            self.warned_no_hwnd = true;
            tracing::warn!("overlay window not found by title yet");
        }
    }

    fn on_hide(&mut self, ctx: &egui::Context) {
        if !self.overlay.is_open() {
            return;
        }
        // The trigger key came up. In browse mode the target never lost the caret, so this is
        // just a hide; in a focused search session `dismiss` hands the caret back first, which
        // is the case that used to leave the cursor nowhere.
        self.dismiss(ctx);
    }

    /// End the session. Every way out of the overlay goes through here - key release, Escape,
    /// the close mark, a fired row, a click away - because the promise is the same on each one
    /// and four copies of it are four places it can be broken.
    fn dismiss(&mut self, ctx: &egui::Context) {
        self.give_back_focus();
        self.overlay.close();
        self.summoned = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        ctx.request_repaint();
    }

    /// Move the caret back to the window the session came from.
    ///
    /// Does nothing unless the overlay actually took the focus. That is not shyness: in browse
    /// mode the target is still foreground and never stopped being, so touching focus here would
    /// spend the one thing this product is promising to leave alone.
    ///
    /// Idempotent because `target` is *taken*: the fire path and the dismiss path both call this
    /// and must not activate the same window twice, or hand back to a window the user has since
    /// left. The ex-style goes back to "never activate" afterwards, so a hidden panel cannot be
    /// picked up as a focus candidate by whatever the shell does next.
    fn give_back_focus(&mut self) {
        if !self.overlay.focused {
            return;
        }
        self.overlay.focused = false;
        self.focused_since = None;
        let Some(target) = self.target.take() else {
            tracing::warn!("focused session ended with no window to hand focus back to");
            return;
        };
        platform::activate(target.0);
        self.enforce_overlay_style();
        self.stats.hand_backs += 1;
        tracing::debug!(hwnd = target.0, exe = ?self.target_exe, "focus handed back to the target");
    }

    /// True once the window in front has stopped being either the overlay or the app the
    /// session came from, which means the user is working somewhere else now.
    ///
    /// The two modes have different grace. A focused search session is the case where the
    /// overlay *is* the expected foreground, and the activation it asked for lands a frame or
    /// two late - so a departure there is only counted after 500 ms, or every click into the
    /// box would dismiss itself the moment it started. A browse session never expects to be
    /// foreground at all, so any other window in front is already a move away: that is the half
    /// that an instant chord summon needs, because a chord press has no key release to hide on.
    fn left_the_session(&self) -> bool {
        let Some(front) = platform::foreground_window() else {
            return false;
        };
        if self.overlay.focused {
            let (Some(hwnd), Some(since)) = (self.hwnd, self.focused_since) else {
                return false;
            };
            front.0 != hwnd && since.elapsed() > GRACE
        } else {
            // Browse mode. The app the session came from is the expected foreground, and our
            // own window showing up there is winit's doing rather than the user's - that is
            // what the per-frame ex-style re-assertion exists to fix - so only a *third* window
            // in front counts as the user having moved on.
            let Some(target) = self.target else {
                return false;
            };
            front.0 != target.0 && Some(front.0) != self.hwnd
        }
    }

    /// The tray only ever carries one decision, but it arrives on another thread, so the
    /// UI has to come back for it: 125 ms worst case, from the idle poll below.
    fn drain_tray(&mut self, ctx: &egui::Context) {
        let quit = if let Some(rx) = self.tray.as_ref() {
            matches!(rx.try_recv(), Ok(TrayEvent::Quit))
        } else {
            false
        };
        if quit {
            tracing::info!(
                shows = self.stats.shows,
                fires = self.stats.fires,
                unsupported = self.stats.unsupported,
                hand_backs = self.stats.hand_backs,
                "quitting from the tray"
            );
            self.request_quit(ctx);
        }
    }

    fn fire(&mut self, combo: Combo) {
        platform::send_combo(&combo);
        self.stats.fires += 1;
        self.last_fire = Some(Instant::now());
        tracing::debug!(
            combo = %combo,
            shows = self.stats.shows,
            fires = self.stats.fires,
            unsupported = self.stats.unsupported,
            hand_backs = self.stats.hand_backs,
            "fired"
        );
    }

    /// Read the arrow/Enter/Esc keys out of egui's own input, which only sees them while
    /// the overlay deliberately holds focus.
    fn key_flags(ctx: &egui::Context) -> KeyFlags {
        ctx.input(|i| {
            let pressed = |k: egui::Key| i.key_pressed(k);
            KeyFlags {
                up: pressed(egui::Key::ArrowUp),
                down: pressed(egui::Key::ArrowDown),
                enter: pressed(egui::Key::Enter),
                escape: pressed(egui::Key::Escape),
            }
        })
    }

    /// The frames the open panel still has to ask for itself, because nothing else will.
    ///
    /// Everything the old 55 fps loop was covering has an event now: pointer and key input
    /// arrive as OS events, which eframe turns into a frame on its own; the caret blink
    /// schedules its own frames inside egui; the hook and the tray wake this thread through
    /// `UiWake`; and a foreground change arrives as a WinEvent. What is left is the two
    /// clocks the app owns, and both are exact timers rather than a poll - so an idle panel
    /// paints zero times a second.
    fn schedule_next_frame(&self, ctx: &egui::Context) {
        if let Some(wait) = self.next_timer(Instant::now()) {
            ctx.request_repaint_after(wait);
        }
    }

    /// The pure half, with the clock handed in: the shortest wait still outstanding, or
    /// `None` when nothing is running and the loop should sleep until something happens.
    /// `None` is the whole point of this function - it is the assertion that an open,
    /// idle, unheld panel asks for no frames at all.
    fn next_timer(&self, now: Instant) -> Option<Duration> {
        let mut wait: Option<Duration> = None;
        if let Some(deadline) = self.peek_until {
            // `--peek` closes itself, and a verification run must not linger on screen. A
            // deadline already gone asks for a zero wait: the next frame is the one that
            // notices and quits, and nothing may sleep past it.
            sooner(&mut wait, deadline.saturating_duration_since(now));
        }
        if self.overlay.focused {
            // The activation we asked for lands a frame or two late, so a foreground we do not
            // own is only a departure once the grace has run out. If it has not, this frame's
            // answer is not final - and with no free-running loop, the frame that looks again
            // has to be the one that schedules it.
            if let Some(since) = self.focused_since {
                let left = GRACE.saturating_sub(now.saturating_duration_since(since));
                if left > Duration::ZERO {
                    sooner(&mut wait, left);
                }
            }
        }
        wait
    }
}

/// How long a deliberate focus change is given to land before a foreign foreground counts
/// as the user having moved on.
const GRACE: Duration = Duration::from_millis(500);

/// Keep the sooner of two waits.
fn sooner(best: &mut Option<Duration>, candidate: Duration) {
    if best.is_none_or(|b| candidate < b) {
        *best = Some(candidate);
    }
}

impl eframe::App for Sheet {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // The hook and tray threads were spawned before this context existed. Handing it over
        // once is how they learn about it, and it is what lets the open panel stop polling
        // them (BUG-007).
        if !self.wake_bound {
            self.wake_bound = true;
            let painter = ctx.clone();
            self.wake.attach(move || painter.request_repaint());
        }
        if self.closing {
            // Leaving. If the handle is not known yet this keeps trying, slowly, because a
            // resident app with no window and no quit is an app the user has to kill in
            // Task Manager.
            if self.overlay.focused {
                // Do not quit out from under a window that is holding our caret: give it back
                // first, then close, so the last thing the run does is the promise.
                self.give_back_focus();
            }
            if !self.try_close() {
                ctx.request_repaint_after(Duration::from_millis(120));
            }
            return;
        }
        if let Some(deadline) = self.peek_until {
            if Instant::now() >= deadline {
                self.request_quit(&ctx);
                return;
            }
        }
        // The user has moved on: another window is in front, so the sheet is now just something
        // floating over somebody else's work. Dismiss it without touching focus, because focus
        // is already where the user is.
        if self.overlay.is_open() && self.left_the_session() {
            tracing::debug!("the foreground moved elsewhere; dismissing the sheet");
            self.overlay.focused = false;
            self.focused_since = None;
            self.target = None;
            self.dismiss(&ctx);
            return;
        }
        // Theme follows the OS unless the settings file says otherwise, and cheaply: only
        // re-read while hidden.
        if !self.overlay.is_open() {
            let want_dark = self.wants_dark();
            if want_dark != self.dark || !self.styled {
                self.dark = want_dark;
                self.palette = if want_dark { uitok::DARK } else { uitok::LIGHT };
                self.styled = true;
                // egui's own widget visuals - the text-edit field, the selection, the
                // scrollbars - are a separate system from the token palette and default to
                // egui's colours. Left unsynced, the light theme drew a near-black search box
                // on a white panel. Applied on the first frame and on every flip, not per
                // frame, because set_visuals invalidates nothing but is not free.
                ctx.set_visuals(uitok::visuals(&self.palette, want_dark));
            }
            if let Some(app) = self.peek.take() {
                self.peek_show(&ctx, &app);
            }
            self.drain_events(&ctx);
            self.drain_tray(&ctx);
            // Idle poll. The hook cannot wake the event loop from another thread without
            // an event-proxy dependency, so we check at 8 Hz while hidden - a no-op frame
            // costs microseconds and the trigger path itself stays instant.
            ctx.request_repaint_after(Duration::from_millis(120));
            return;
        }

        let flags = Self::key_flags(&ctx);
        self.enforce_overlay_style();
        let action = self.overlay.draw(ui, &self.catalog, &self.palette, &flags);
        match action {
            Action::Nothing => {}
            Action::Fire(c) => {
                // Recording progress before firing, so the write happens while we still know
                // which row the user chose.
                let app = self.overlay.app.clone();
                let entry = self.overlay.selected_entry(&self.catalog);
                // Then the order that decides where the chord actually lands. SendInput goes to
                // whatever holds the caret, and on the Enter path that is our own search box:
                // handing the caret back first is what makes the app receive the shortcut it was
                // chosen for. The three exits that fire a row - Enter, arrows then Enter, and a
                // click - are one code path here, so all three get it or none of them do.
                self.give_back_focus();
                if let (Some(app), Some((collection, command))) = (app, entry) {
                    self.catalog.record_learnt(&app, &collection, &command);
                }
                self.fire(c);
                self.dismiss(&ctx);
            }
            Action::WantFocus => {
                self.overlay.focused = true;
                self.focused_since = Some(Instant::now());
                self.ensure_hwnd();
                if let Some(h) = self.hwnd {
                    // The one deliberate focus change in the product.
                    tracing::debug!(
                        hwnd = h,
                        target = ?self.target.map(|t| t.0),
                        "overlay takes focus for the search box"
                    );
                    platform::activate(h);
                }
            }
            Action::GiveUpFocus => {
                // Escape, or the drawn X. `dismiss` sees that the session still holds the caret
                // and hands it back before hiding, which is the order the hand-back needs: the
                // target must still exist and the overlay must still be the window giving focus
                // up. In browse mode nothing holds the caret and this is just a hide.
                tracing::debug!("overlay hands focus back and hides");
                self.dismiss(&ctx);
            }
        }

        // Ctrl-up from the hook can still arrive while we are focused.
        self.drain_events(&ctx);
        self.drain_tray(&ctx);
        // And that is the whole frame. The loop used to end in an unconditional
        // `ctx.request_repaint()`, which bought the two things this now gets from events -
        // the hook's key release and the tray's quit - at 55 frames a second for as long as
        // the sheet sat open (BUG-007).
        self.schedule_next_frame(&ctx);
    }
}

/// Best-effort read of the OS app-mode. `AppsUseLightTheme` under Themes\Personalize.
fn detect_system_dark() -> bool {
    platform::apps_use_light_theme().map(|light| !light).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `Sheet` with no channels and no window, which is everything the frame schedule
    /// looks at. The tag keeps parallel tests off each other's temporary data directory.
    fn sheet(tag: &str) -> Sheet {
        let dir = std::env::temp_dir().join(format!("ss-app-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Sheet::new(
            Catalog::load(&dir).expect("builtin catalog"),
            None,
            None,
            None,
            platform::UiWake::default(),
        )
    }

    /// The whole point of BUG-007: a sheet that is open, idle and not being held must ask
    /// for nothing. Before the fix this returned "repaint now" for as long as the panel was up.
    #[test]
    fn an_idle_open_panel_asks_for_no_frames() {
        let now = Instant::now();
        let s = sheet("idle");
        assert_eq!(s.next_timer(now), None, "nothing is running, so nothing may be scheduled");
    }

    #[test]
    fn a_browse_session_under_a_hold_has_no_clock_either() {
        // The hold itself is watched by the hook, not by the UI loop, so even a session that
        // was summoned by a key needs no frames from here.
        let now = Instant::now();
        let mut s = sheet("hold");
        s.summoned = Some(Summon::Hold);
        s.target = Some(TargetWindow(1));
        assert_eq!(s.next_timer(now), None);
    }

    #[test]
    fn the_peek_deadline_is_an_exact_timer_not_a_poll() {
        let now = Instant::now();
        let mut s = sheet("deadline");
        s.peek_until = Some(now + Duration::from_secs(25));
        let wait = s.next_timer(now).expect("a deadline is a reason to come back");
        assert!(
            wait > Duration::from_secs(24) && wait <= Duration::from_secs(25),
            "the wait should be the deadline itself, got {wait:?}"
        );
    }

    #[test]
    fn a_deadline_that_has_passed_asks_for_a_frame_immediately() {
        // Sleeping past a passed deadline is how a --peek run sat on screen forever: the
        // frame that notices it has to be the next one (BUG-003).
        let now = Instant::now();
        let mut s = sheet("passed");
        s.peek_until = Some(now - Duration::from_millis(10));
        assert_eq!(s.next_timer(now), Some(Duration::ZERO));
    }

    #[test]
    fn the_focus_grace_comes_back_and_looks_again() {
        let now = Instant::now();
        let mut s = sheet("grace");
        s.overlay.focused = true;
        s.focused_since = Some(now);
        let wait = s.next_timer(now).expect("the activation has not landed yet");
        assert!(wait > GRACE / 2 && wait <= GRACE, "roughly the rest of the grace, got {wait:?}");
    }

    #[test]
    fn the_grace_stops_scheduling_once_it_has_run_out() {
        // Past the grace the answer is final: either the foreground moved on and the next
        // event-driven frame dismisses, or it did not and there is nothing to watch for.
        let now = Instant::now();
        let mut s = sheet("grace-out");
        s.overlay.focused = true;
        s.focused_since = Some(now - GRACE);
        assert_eq!(s.next_timer(now), None);
    }

    #[test]
    fn no_grace_frame_is_scheduled_for_a_session_that_never_took_focus() {
        // The browse path must not start poking at focus it never had: that is the promise.
        let now = Instant::now();
        let mut s = sheet("never-focused");
        s.focused_since = Some(now);
        assert!(!s.overlay.focused, "a stale timestamp is not the same as holding the caret");
        assert_eq!(s.next_timer(now), None);
    }

    #[test]
    fn the_soonest_clock_wins_when_both_are_running() {
        let now = Instant::now();
        let mut s = sheet("both");
        s.peek_until = Some(now + Duration::from_secs(9));
        s.overlay.focused = true;
        s.focused_since = Some(now);
        let wait = s.next_timer(now).expect("two clocks");
        assert!(wait <= GRACE, "the grace is the shorter wait, got {wait:?}");
    }

    #[test]
    fn sooner_takes_the_first_and_then_only_the_smaller() {
        let mut best: Option<Duration> = None;
        sooner(&mut best, Duration::from_millis(300));
        assert_eq!(best, Some(Duration::from_millis(300)));
        sooner(&mut best, Duration::from_millis(900));
        assert_eq!(best, Some(Duration::from_millis(300)), "a longer wait is not a reason");
        sooner(&mut best, Duration::from_millis(120));
        assert_eq!(best, Some(Duration::from_millis(120)));
    }

    #[test]
    fn a_zero_wait_survives_a_longer_one() {
        // A passed deadline must not be pushed out by an unrelated longer timer, or the
        // quit is late by that timer.
        let mut best: Option<Duration> = Some(Duration::ZERO);
        sooner(&mut best, Duration::from_secs(4));
        assert_eq!(best, Some(Duration::ZERO));
    }
}
