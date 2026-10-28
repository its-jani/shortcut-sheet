//! The single eframe window, which *is* the overlay.
//!
//! Evidence for this shape: the original keeps one hidden `WinUIDesktopWin32WindowClass`
//! window carrying `WS_EX_TOPMOST` and shows it on the trigger, rather than creating a
//! window per press. Pre-creating is what keeps the first press as fast as the hundredth.

use std::time::{Duration, Instant};

use crate::data::{Catalog, Combo};
use crate::detect::{self, Context};
use crate::platform::{self, HookEvent, TargetWindow, TrayEvent};
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
    hwnd: Option<isize>,
    /// So "the window is not there yet" is said once per run, not once per frame.
    warned_no_hwnd: bool,
    palette: Palette,
    dark: bool,
    last_fire: Option<Instant>,
    stats: Stats,
}

#[derive(Debug, Default)]
pub struct Stats {
    pub shows: u64,
    pub fires: u64,
    pub unsupported: u64,
}

impl Sheet {
    /// `events` and `tray` are `None` in peek mode, where no hook is installed at all.
    pub fn new(
        catalog: Catalog,
        events: Option<std::sync::mpsc::Receiver<HookEvent>>,
        tray: Option<std::sync::mpsc::Receiver<TrayEvent>>,
        peek: Option<String>,
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
            hwnd: None,
            warned_no_hwnd: false,
            palette: if dark { uitok::DARK } else { uitok::LIGHT },
            dark,
            last_fire: None,
            stats: Stats::default(),
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
                HookEvent::Show { target } => self.on_show(ctx, TargetWindow(target)),
                HookEvent::Hide => self.on_hide(ctx),
            }
        }
    }

    fn on_show(&mut self, ctx: &egui::Context, target: TargetWindow) {
        if self.overlay.is_open() {
            return; // already up; the hook will not re-arm until the key comes up
        }
        let exe = platform::process_name_of(target);
        match detect::resolve(&self.catalog, exe.as_deref()) {
            Context::Known(name) => {
                self.target = Some(target);
                self.target_exe = exe;
                self.overlay.open(&self.catalog, &name);
                self.stats.shows += 1;
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
        self.overlay.open(&self.catalog, app);
        if !self.overlay.is_open() {
            tracing::error!(app, "--peek named an app that is not in the dataset");
            self.request_quit(ctx);
            return;
        }
        self.stats.shows += 1;
        match platform::foreground_window() {
            Some(target) => self.place_and_show(ctx, target),
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
        let was_focused = self.overlay.focused;
        self.overlay.close();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        if was_focused {
            self.restore_focus();
        }
        ctx.request_repaint();
    }

    fn restore_focus(&mut self) {
        if let Some(t) = self.target {
            platform::activate(t.0);
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
}

impl eframe::App for Sheet {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.closing {
            // Leaving. If the handle is not known yet this keeps trying, slowly, because a
            // resident app with no window and no quit is an app the user has to kill in
            // Task Manager.
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
        // Theme follows the OS, cheaply: only re-read while hidden.
        if !self.overlay.is_open() {
            let want_dark = detect_system_dark();
            if want_dark != self.dark {
                self.dark = want_dark;
                self.palette = if want_dark { uitok::DARK } else { uitok::LIGHT };
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
                if let (Some(app), Some((collection, command))) = (app, entry) {
                    self.catalog.record_learnt(&app, &collection, &command);
                }
                self.fire(c);
                self.on_hide(&ctx);
            }
            Action::WantFocus => {
                self.overlay.focused = true;
                self.ensure_hwnd();
                if let Some(h) = self.hwnd {
                    platform::activate(h);
                }
            }
            Action::GiveUpFocus => {
                self.overlay.focused = false;
                self.restore_focus();
                self.on_hide(&ctx);
            }
        }

        // Ctrl-up from the hook can still arrive while we are focused.
        self.drain_events(&ctx);
        self.drain_tray(&ctx);
        ctx.request_repaint();
    }
}

/// Best-effort read of the OS app-mode. `AppsUseLightTheme` under Themes\Personalize.
fn detect_system_dark() -> bool {
    platform::apps_use_light_theme().map(|light| !light).unwrap_or(false)
}
