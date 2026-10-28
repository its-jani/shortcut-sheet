//! The only module in this crate that names a Win32 symbol or contains an `unsafe` block.
//!
//! Everything above this boundary takes and returns plain Rust types. `TargetWindow` is an
//! opaque newtype so no other module can dereference a raw handle.

use std::mem::size_of;
use std::sync::mpsc::Sender;
use std::cell::RefCell;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    CloseHandle, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::HiDpi::{GetDpiForWindow, SetProcessDpiAwarenessContext,
                                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    MAPVK_VK_TO_VSC, VIRTUAL_KEY,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIM_ADD, NIM_DELETE,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CallNextHookEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DestroyWindow, DispatchMessageW, FindWindowW, GetMessageW, GetForegroundWindow,
    GetWindowLongPtrW, GetCursorPos, GetWindowRect, GetWindowThreadProcessId, KillTimer,
    LLMHF_INJECTED, LoadIconW, PostMessageW, PostQuitMessage, RegisterClassExW, SetForegroundWindow,
    SetTimer, SetWindowLongPtrW, SetWindowPos, SetWindowsHookExW, ShowWindow, TranslateMessage,
    TrackPopupMenu, UnhookWindowsHookEx, CS_CLASSDC, GWL_EXSTYLE,
    HC_ACTION, HWND_MESSAGE, IDI_APPLICATION, KBDLLHOOKSTRUCT, MF_STRING, MSG, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SW_SHOWNOACTIVATE,
    TPM_RIGHTBUTTON, WH_KEYBOARD_LL, WM_APP, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_KEYDOWN,
    WM_KEYUP, WM_LBUTTONUP, WM_QUIT, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER,
    WNDCLASSEXW, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_OVERLAPPED,
};

use crate::data::key::{Combo, Key};

/// A window we may inspect and send input to. Opaque outside this module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetWindow(pub(crate) isize);

impl TargetWindow {
    fn hwnd(self) -> HWND {
        HWND(self.0 as *mut core::ffi::c_void)
    }
}

pub fn as_hwnd(raw: isize) -> HWND {
    HWND(raw as *mut core::ffi::c_void)
}

/// Per-monitor v2 awareness. Must run before any window exists, or the process is
/// stuck with the default (system-aware) context and the overlay lands off-centre on
/// any monitor that is not the primary.
pub fn make_process_dpi_aware() {
    // SAFETY: called once at startup before window creation. The API has no preconditions
    // beyond being called on the process's first thread; a failure here is non-fatal (the
    // context may already have been set by the manifest) so we ignore the returned BOOL.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// The window currently receiving keyboard input, if any.
pub fn foreground_window() -> Option<TargetWindow> {
    // SAFETY: GetForegroundWindow has no preconditions and returns NULL when none.
    let h = unsafe { GetForegroundWindow() };
    if h.0.is_null() {
        None
    } else {
        Some(TargetWindow(h.0 as isize))
    }
}

/// Lower-case executable name of the process owning `win` ("brave.exe").
pub fn process_name_of(win: TargetWindow) -> Option<String> {
    // SAFETY: we pass a valid HWND from foreground_window(); pid is written by the callee.
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(win.hwnd(), Some(&mut pid)) };
    if pid == 0 {
        return None;
    }
    // SAFETY: PROCESS_QUERY_LIMITED_INFORMATION is the least-privileged access right and is
    // granted across integrity levels, unlike PROCESS_QUERY_INFORMATION. The handle is
    // closed on every path below.
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()? };
    let name = read_image_name(handle);
    // SAFETY: handle came from OpenProcess and is used exactly once here.
    unsafe {
        let _ = CloseHandle(handle);
    }
    name
}

fn read_image_name(handle: HANDLE) -> Option<String> {
    const MAX: usize = 4096;
    let mut buf = [0u16; MAX];
    let mut len = MAX as u32;
    // SAFETY: buf is a valid, writable u16 buffer of MAX elements and len carries its
    // capacity on entry, which the callee replaces with the bytes written.
    let ok = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT_WIN32,
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    }
    .is_ok();
    if ok && len > 0 {
        let wide = &buf[..len as usize];
        let s = String::from_utf16_lossy(wide);
        Some(
            s.rsplit(['\\', '/'])
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase(),
        )
    } else {
        None
    }
}

/// QueryFullProcessImageNameW's first format: a Win32 path.
const PROCESS_NAME_FORMAT_WIN32: PROCESS_NAME_FORMAT = PROCESS_NAME_FORMAT(0);

/// Physical-pixel rect of `win`, as (x, y, width, height).
pub fn rect_of(win: TargetWindow) -> Option<(i32, i32, i32, i32)> {
    // SAFETY: valid HWND; the callee writes all four RECT fields.
    let mut r = RECT::default();
    unsafe { GetWindowRect(win.hwnd(), &mut r) }.ok()?;
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    if w <= 0 || h <= 0 {
        return None; // minimised or off-screen
    }
    Some((r.left, r.top, w, h))
}

/// DPI of the monitor `win` sits on; 96 when unknown.
pub fn dpi_of(win: TargetWindow) -> f32 {
    // SAFETY: valid HWND. Returns 0 on old Windows builds, which we fall back from.
    let d = unsafe { GetDpiForWindow(win.hwnd()) };
    if d == 0 { 96.0 } else { d as f32 }
}

/// Strip the taskbar and alt-tab presence, and make sure showing the window never takes
/// focus. This is the load-bearing style for the whole product: while the overlay is up,
/// the app the user was working in stays foreground, so injected input lands in it.
///
/// `no_activate` is off only while the user has deliberately clicked into the search field,
/// which is the one moment the overlay is allowed to hold the caret.
///
/// This has to be re-asserted, not set once. winit rebuilds the ex-style from its own flag
/// set on every window-state change (`WindowFlags::apply_diff` in winit 0.30 calls
/// `SetWindowLongW(GWL_EXSTYLE, ...)` with a freshly computed value), so a write made next to
/// `ViewportCommand::Visible` is gone again before the panel is on screen. Measured: after
/// the first show the window came back as `0x00040118` - topmost, but neither no-activate nor
/// tool-window. The caller keeps this true by calling it every frame the overlay is open.
pub fn apply_overlay_exstyle(raw: isize, no_activate: bool) {
    let h = as_hwnd(raw);
    // SAFETY: h is a live window handle supplied by the caller. The style is only written
    // when it differs, and SetWindowPos is told to leave the position, size, z-order and
    // focus alone.
    unsafe {
        let cur = GetWindowLongPtrW(h, GWL_EXSTYLE);
        let keep_off_taskbar = WS_EX_TOOLWINDOW.0 as isize;
        let on_top = WS_EX_TOPMOST.0 as isize;
        let drop_app_window = WS_EX_APPWINDOW.0 as isize;
        let never_activate = WS_EX_NOACTIVATE.0 as isize;
        let want = if no_activate {
            (cur | keep_off_taskbar | on_top | never_activate) & !drop_app_window
        } else {
            (cur | keep_off_taskbar | on_top) & !(drop_app_window | never_activate)
        };
        if want == cur {
            return;
        }
        let _ = SetWindowLongPtrW(h, GWL_EXSTYLE, want);
        // The shell re-reads the tool-window bit on a frame change; without this the panel
        // can still show up in alt-tab until something else touches the window.
        let _ = SetWindowPos(
            h,
            None,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

/// Find our own overlay window by its unique title. Used instead of threading a
/// raw-window-handle dependency through the app: the title is set by us at startup and is
/// specific enough that a collision is not a realistic risk.
pub fn find_window_by_title(title: &str) -> Option<isize> {
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: `wide` is a NUL-terminated UTF-16 buffer that outlives the call, and both
    // PCWSTRs point at it (class NULL means "match any class").
    let h = unsafe { FindWindowW(None, windows::core::PCWSTR(wide.as_ptr())) }.ok()?;
    if h.is_invalid() {
        None
    } else {
        Some(h.0 as isize)
    }
}

/// Reads `AppsUseLightTheme` from the current user's theme key. `None` when unavailable,
/// which lets the caller keep whatever theme it already had.
pub fn apps_use_light_theme() -> Option<bool> {
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_DWORD};
    let path: Vec<u16> =
        "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
    let name: Vec<u16> = "AppsUseLightTheme"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut value: u32 = 1;
    let mut len = std::mem::size_of::<u32>() as u32;
    // SAFETY: path/name are NUL-terminated UTF-16 buffers alive for the call, and we pass
    // the exact byte size of `value` in pcbData, which the callee respects.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(path.as_ptr()),
            windows::core::PCWSTR(name.as_ptr()),
            RRF_RT_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut len),
        )
    };
    if status.is_ok() {
        Some(value != 0)
    } else {
        None
    }
}

/// Show without activating: the target app keeps the caret.
pub fn show_without_activating(raw: isize) {
    // SAFETY: valid HWND from the caller.
    unsafe {
        let _ = ShowWindow(as_hwnd(raw), SW_SHOWNOACTIVATE);
    }
}

/// Deliberate focus change: used only when the user clicks into the search field.
pub fn activate(raw: isize) {
    // SAFETY: valid HWND. SetForegroundWindow may be refused by the foreground-lock
    // rules; we do not treat that as fatal, the overlay still works by mouse.
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(as_hwnd(raw));
    }
}

/// End the run the way Windows intends: WM_CLOSE to our own window, which winit turns into
/// `CloseRequested` and eframe turns into returning from `run_native`.
///
/// `egui::ViewportCommand::Close` does not do this for the root viewport in eframe 0.37 - it
/// records an event inside egui and leaves the window alive. Measured, not guessed: a tray
/// Quit built on that command appeared to do nothing at all.
pub fn request_close(raw: isize) {
    // SAFETY: valid HWND supplied by the caller; WM_CLOSE ignores both extra parameters.
    unsafe {
        let _ = PostMessageW(Some(as_hwnd(raw)), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}

/// Fire a chord into whatever currently holds focus.
pub fn send_combo(combo: &Combo) {
    let keys = &combo.0;
    let mut inputs: Vec<INPUT> = Vec::with_capacity(keys.len() * 2);
    for k in keys {
        inputs.push(key_event(*k, false));
    }
    for k in keys.iter().rev() {
        inputs.push(key_event(*k, true));
    }
    // SAFETY: `inputs` outlives the call, its length is exact, and cbSize matches the
    // INPUT layout this build was compiled against.
    let sent = unsafe {
        SendInput(&inputs, size_of::<INPUT>() as i32)
    };
    if sent as usize != inputs.len() {
        tracing::warn!(expected = inputs.len(), sent, "SendInput dropped events");
    }
}

fn key_event(key: Key, up: bool) -> INPUT {
    let vk = key.vk();
    // SAFETY: MapVirtualKeyW is a pure lookup for any VK value; an unmapped key yields 0,
    // which the OS resolves itself from the VK on the way down.
    let scan = unsafe { MapVirtualKeyW(u32::from(vk), MAPVK_VK_TO_VSC) } as u16;
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk as u16),
                wScan: scan,
                dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

// ---------------------------------------------------------------------------
// the keyboard hook
// ---------------------------------------------------------------------------

/// What the hook decides. Rare by design: one event per hold, not per keystroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookEvent {
    Show { target: isize },
    Hide,
}

/// State the proc and the 50 ms timer share. Both run on the hook thread, so a
/// thread-local is enough and no lock is needed anywhere on the input path.
struct HoldState {
    trigger_vk: u32,
    hold: Duration,
    armed: bool,
    held: bool,
    shown: bool,
    since: Instant,
    sink: Sender<HookEvent>,
}

thread_local! {
    static STATE: RefCell<Option<HoldState>> = const { RefCell::new(None) };
}

/// Runs on the hook thread. Does one thing: poke the state machine. Never allocates,
/// never locks, never touches the UI. A low-level hook that outruns
/// LowLevelHooksTimeout (300 ms by default) is silently uninstalled by Windows, which
/// would look like "it stopped working after a while" in the field.
unsafe extern "system" fn proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: when code is HC_ACTION the system guarantees lparam points at a valid
        // KBDLLHOOKSTRUCT for the duration of the callback.
        let info = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        // Our own SendInput arrives here too. Without this filter, firing Ctrl+C would
        // re-arm the hook and pop the overlay over the result.
        let injected = info.flags.0 & LLMHF_INJECTED != 0;
        if !injected {
            STATE.with(|s| {
                if let Some(st) = s.borrow_mut().as_mut() {
                    let down = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;
                    let up = wparam.0 as u32 == WM_KEYUP || wparam.0 as u32 == WM_SYSKEYUP;
                    if down || up {
                        evaluate(st, info.vkCode, down);
                    }
                }
            });
        }
    }
    // SAFETY: forwarding per the documented contract; the None hook handle means "call the
    // next hook in the chain", which is legal for this hook type.
    CallNextHookEx(None, code, wparam, lparam)
}

/// One transition of the hold state machine. Split out from `proc` so the timing rules
/// are readable and testable.
fn evaluate(st: &mut HoldState, vk: u32, down: bool) {
    if vk == st.trigger_vk {
        if down {
            if !st.held {
                st.held = true;
                st.armed = true;
                st.since = Instant::now();
            }
        } else {
            st.held = false;
            if st.shown {
                st.shown = false;
                let _ = st.sink.send(HookEvent::Hide);
            }
            st.armed = false;
        }
        return;
    }
    if down && st.armed {
        // Any other key while the trigger is held is a real shortcut being typed, not a
        // lookup. Without this rule every Ctrl+C pops the overlay over the user's work.
        st.armed = false;
    }
}

/// Timer tick: the hook only fires on input, so a user holding Ctrl with no other
/// keystroke would never get the overlay without this.
fn tick(st: &mut HoldState) {
    if hold_elapsed(st, Instant::now()) {
        if let Some(target) = foreground_window() {
            st.shown = true;
            let _ = st.sink.send(HookEvent::Show { target: target.0 });
        }
    }
}

/// The timing rule on its own, with the clock handed in. `tick` is the half that talks to
/// Windows; this is the half that decides, and it is what the hold-threshold tests drive.
fn hold_elapsed(st: &HoldState, now: Instant) -> bool {
    st.armed
        && st.held
        && !st.shown
        && now.saturating_duration_since(st.since) >= st.hold
}

/// Spawn the hook thread. Returns the receiving end of its decision stream and a join
/// handle; the hook stays installed until the thread exits.
pub fn spawn_hook(trigger_vk: u32, hold: Duration) -> std::sync::mpsc::Receiver<HookEvent> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("ss-hook".into())
        .spawn(move || {
            STATE.with(|s| {
                *s.borrow_mut() = Some(HoldState {
                    trigger_vk,
                    hold,
                    armed: false,
                    held: false,
                    shown: false,
                    since: Instant::now(),
                    sink: tx,
                })
            });

            // SAFETY: a thread-local hook (hMod NULL, dwThreadId 0) is valid for
            // WH_KEYBOARD_LL and is removed by UnhookWindowsHookEx below. `proc` matches
            // the HOOKPROC signature.
            let hook = unsafe {
                SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc), None, 0)
            };
            let hook = match hook {
                Ok(h) if !h.0.is_null() => h,
                _ => {
                    tracing::error!(err = ?windows::core::Error::from_thread(),
                        "could not install the keyboard hook");
                    STATE.with(|s| *s.borrow_mut() = None);
                    return;
                }
            };
            tracing::info!(trigger_vk, hold_ms = hold.as_millis(), "global keyboard hook installed");

            // A thread timer, because GetMessageW would otherwise sleep straight through
            // the hold threshold.
            const TIMER_ID: usize = 0x5501;
            // SAFETY: hwnd NULL makes this a thread timer delivered to this thread's queue.
            unsafe { SetTimer(None, TIMER_ID, 50, None) };

            let mut msg = MSG::default();
            loop {
                // SAFETY: msg is a valid, writable MSG and hWnd NULL means "this thread's
                // messages", which is what a thread timer and hook traffic use.
                let ret = unsafe { GetMessageW(&mut msg, None, 0, 0) };
                if ret.0 <= 0 || msg.message == WM_QUIT {
                    break;
                }
                if msg.message == WM_TIMER {
                    // The hold threshold can pass with no keystroke at all, and the hook
                    // only fires on input - this timer is what makes a bare hold work.
                    STATE.with(|s| {
                        if let Some(st) = s.borrow_mut().as_mut() {
                            tick(st);
                        }
                    });
                    continue;
                }
                // SAFETY: msg came from GetMessageW.
                unsafe {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            // SAFETY: hook is the handle returned by SetWindowsHookExW on this thread.
            unsafe {
                let _ = KillTimer(None, TIMER_ID);
                let _ = UnhookWindowsHookEx(hook);
            }
            STATE.with(|s| *s.borrow_mut() = None);
        })
        .expect("spawn hook thread");
    rx
}

// ---------------------------------------------------------------------------
// the tray icon
// ---------------------------------------------------------------------------

/// What the user picks in the tray menu. The overlay itself needs nothing from the tray.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    Quit,
}

/// The icon callback message. `WM_APP` is the range Windows reserves for an app's own use.
const WM_TRAYICON: u32 = WM_APP + 1;
/// The only menu item there is, for now: the product has no settings window yet.
const IDM_QUIT: usize = 0x0a01;
/// Shell_NotifyIconW keys icons by (hWnd, uID); we have one icon.
const TRAY_UID: u32 = 1;

thread_local! {
    static TRAY_SINK: RefCell<Option<Sender<TrayEvent>>> = const { RefCell::new(None) };
}

/// UTF-16 with the terminator, for the handful of strings the shell APIs want.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Copy `s` into a fixed UTF-16 buffer, truncating rather than overflowing it.
fn copy_into_wide(buf: &mut [u16], s: &str) {
    for (slot, unit) in buf.iter_mut().zip(s.encode_utf16()) {
        *slot = unit;
    }
}

unsafe extern "system" fn tray_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            // For an unversioned notify icon the shell reports the mouse message in lparam.
            let clicked = lparam.0 as u32;
            if clicked == WM_RBUTTONUP || clicked == WM_LBUTTONUP {
                show_tray_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            if wparam.0 & 0xFFFF == IDM_QUIT {
                TRAY_SINK.with(|s| {
                    if let Some(tx) = s.borrow().as_ref() {
                        let _ = tx.send(TrayEvent::Quit);
                    }
                });
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// The menu is built per click rather than cached: it is two calls and the alternative is
/// stale state.
unsafe fn show_tray_menu(hwnd: HWND) {
    let Ok(menu) = CreatePopupMenu() else { return };
    let label = wide("Quit shortcut-sheet");
    if AppendMenuW(menu, MF_STRING, IDM_QUIT, windows::core::PCWSTR(label.as_ptr())).is_err() {
        let _ = DestroyMenu(menu);
        return;
    }
    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    // Without this the menu stays open after the click, because the shell only dismisses a
    // popup when its owner window loses activation.
    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, None, hwnd, None);
    let _ = PostMessageW(Some(hwnd), WM_LBUTTONUP, WPARAM(0), LPARAM(0));
    let _ = DestroyMenu(menu);
}

/// Install the tray icon on its own thread with its own message loop, and return the
/// stream of choices. The icon is the app's only presence in the shell: the overlay window
/// is a tool window and never shows in the taskbar.
pub fn spawn_tray() -> std::sync::mpsc::Receiver<TrayEvent> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("ss-tray".into())
        .spawn(move || {
            TRAY_SINK.with(|s| *s.borrow_mut() = Some(tx));

            // SAFETY: every call below works on handles created here or on static resources,
            // and the thread owns the window and the icon for its whole life.
            unsafe {
                let hmodule = match GetModuleHandleW(None) {
                    Ok(h) => h,
                    Err(e) => {
                        tracing::error!(err = ?e, "tray: no module handle");
                        return;
                    }
                };
                let hinstance = HINSTANCE(hmodule.0);
                // The system's generic application icon. shortcut-sheet has no logo of its
                // own yet, and inventing one here would pre-empt the branding step.
                let icon = LoadIconW(None, IDI_APPLICATION).unwrap_or_default();

                let class_name = wide("shortcut-sheet-tray");
                let window_name = wide("shortcut-sheet-tray-window");
                let mut wc = WNDCLASSEXW::default();
                wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
                wc.style = CS_CLASSDC;
                wc.lpfnWndProc = Some(tray_proc);
                wc.hInstance = hinstance;
                wc.hIcon = icon;
                wc.hIconSm = icon;
                wc.lpszClassName = windows::core::PCWSTR(class_name.as_ptr());
                if RegisterClassExW(&wc) == 0 {
                    tracing::error!(err = ?windows::core::Error::from_thread(), "tray: class registration failed");
                    return;
                }

                // A message-only window: it never paints, never appears, and still receives
                // the notify-icon callback.
                let Ok(hwnd) = CreateWindowExW(
                    WS_EX_TOOLWINDOW,
                    windows::core::PCWSTR(class_name.as_ptr()),
                    windows::core::PCWSTR(window_name.as_ptr()),
                    WS_OVERLAPPED,
                    0,
                    0,
                    0,
                    0,
                    Some(HWND_MESSAGE),
                    None,
                    Some(hinstance),
                    None,
                ) else {
                    tracing::error!(err = ?windows::core::Error::from_thread(), "tray: window creation failed");
                    return;
                };

                let mut nid = NOTIFYICONDATAW::default();
                nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                nid.hWnd = hwnd;
                nid.uID = TRAY_UID;
                nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP;
                nid.uCallbackMessage = WM_TRAYICON;
                nid.hIcon = icon;
                copy_into_wide(&mut nid.szTip, "shortcut-sheet - hold Ctrl to see shortcuts");

                // explorer.exe can refuse NIM_ADD in the moment right after it starts, which
                // is exactly when a run-on-startup launch happens. A few tries covers it.
                let mut added = false;
                for attempt in 0..5 {
                    if Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
                        added = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(200 * (attempt + 1)));
                }
                if !added {
                    tracing::warn!("tray icon could not be installed; the app still runs");
                } else {
                    tracing::info!("tray icon installed");
                }

                let mut msg = MSG::default();
                while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                if added {
                    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
                }
                let _ = DestroyWindow(hwnd);
            }
        })
        .expect("spawn tray thread");
    rx
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::Receiver;

    const CTRL: u32 = 0x11;
    const C: u32 = 0x43;

    /// The hold machine driven by hand. No hook is installed and no window is touched -
    /// `evaluate` and `hold_elapsed` are pure, which is the whole point of splitting them
    /// out of `proc` and `tick`.
    fn machine(hold_ms: u64) -> (HoldState, Receiver<HookEvent>) {
        let (tx, rx) = std::sync::mpsc::channel();
        let st = HoldState {
            trigger_vk: CTRL,
            hold: Duration::from_millis(hold_ms),
            armed: false,
            held: false,
            shown: false,
            since: Instant::now(),
            sink: tx,
        };
        (st, rx)
    }

    /// The timing rule at an offset from the moment the key went down.
    fn ready_at(st: &HoldState, ms: u64) -> bool {
        hold_elapsed(st, st.since + Duration::from_millis(ms))
    }

    fn fired(rx: &Receiver<HookEvent>) -> Vec<HookEvent> {
        rx.try_iter().collect()
    }

    #[test]
    fn a_bare_hold_shows_exactly_at_the_threshold() {
        let (mut st, rx) = machine(900);
        evaluate(&mut st, CTRL, true);
        assert!(!ready_at(&st, 0), "no popup the instant the key goes down");
        assert!(!ready_at(&st, 899), "and none a millisecond before the threshold");
        assert!(ready_at(&st, 900), "the overlay appears once the hold is long enough");
        assert!(fired(&rx).is_empty(), "showing is the timer's job, not the keypress's");
    }

    #[test]
    fn a_tap_never_shows() {
        // The common case: Ctrl pressed and released on the way to Ctrl+C. F01's edge case
        // "key released before the threshold" - the answer must be no overlay at all.
        let (mut st, rx) = machine(900);
        evaluate(&mut st, CTRL, true);
        evaluate(&mut st, CTRL, false);
        assert!(!ready_at(&st, 5000), "a released hold cannot come back later");
        assert!(fired(&rx).is_empty(), "nothing to hide, so no Hide either");
    }

    #[test]
    fn typing_a_real_shortcut_while_held_disarms_the_hold() {
        // Without this rule every Ctrl+C the user types would pop the sheet over the result.
        let (mut st, rx) = machine(900);
        evaluate(&mut st, CTRL, true);
        evaluate(&mut st, C, true);
        assert!(!ready_at(&st, 3000), "an interrupted hold is not a lookup");
        evaluate(&mut st, CTRL, false);
        assert!(fired(&rx).is_empty());
    }

    #[test]
    fn key_autorepeat_does_not_re_arm_an_interrupted_hold() {
        // Holding Ctrl+C keeps repeating both VK_KEYDOWNs. The repeat of Ctrl must not
        // restart the timer, or the sheet appears on top of the copy the user just made.
        let (mut st, _rx) = machine(900);
        evaluate(&mut st, CTRL, true);
        evaluate(&mut st, C, true);
        evaluate(&mut st, CTRL, true);
        assert!(!ready_at(&st, 3000));
    }

    #[test]
    fn an_ongoing_hold_survives_key_repeats() {
        let (mut st, _rx) = machine(900);
        evaluate(&mut st, CTRL, true);
        let since = st.since;
        evaluate(&mut st, CTRL, true);
        assert_eq!(st.since, since, "a repeat must not reset the hold clock");
        assert!(ready_at(&st, 900));
    }

    #[test]
    fn releasing_after_a_show_hides_once() {
        let (mut st, rx) = machine(900);
        evaluate(&mut st, CTRL, true);
        st.shown = true; // what `tick` does when it sends Show
        evaluate(&mut st, CTRL, false);
        assert_eq!(fired(&rx), vec![HookEvent::Hide]);
        // A stray repeat of the key-up must not send a second Hide. (Note `fired` drains
        // the queue, so the read above is what empties it.)
        evaluate(&mut st, CTRL, false);
        assert!(fired(&rx).is_empty(), "one hide per hold");
    }

    #[test]
    fn keys_without_the_trigger_do_nothing() {
        let (mut st, rx) = machine(900);
        evaluate(&mut st, C, true);
        evaluate(&mut st, C, false);
        assert!(!st.held && !st.armed && !st.shown);
        assert!(fired(&rx).is_empty());
    }

    #[test]
    fn the_chord_is_released_in_the_order_windows_wants() {
        // Firing Ctrl+Shift+Z has to press modifiers first and let go in reverse, or the
        // target app sees Shift+Z with no Ctrl. Order is the whole contract of send_combo.
        let combo = Combo::parse("Shift+Ctrl+Z").unwrap();
        let vks: Vec<u16> = combo.0.iter().map(|k| k.vk()).collect();
        assert_eq!(vks, vec![0x11, 0x10, 0x5A], "modifiers first, canonical order");
    }
}
