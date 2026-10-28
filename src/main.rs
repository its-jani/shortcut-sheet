//! shortcut-sheet - hold a key, see the shortcuts for the app you are in.

// A resident utility has no console window. Without this, launching from Explorer or a Run
// key entry puts a black console next to the tray icon for the whole session.
#![windows_subsystem = "windows"]

mod app;
mod data;
mod detect;
mod platform;
mod ui;

use std::path::{Path, PathBuf};
use std::time::Duration;

use app::{Sheet, WINDOW_TITLE};
use data::Catalog;

fn main() -> eframe::Result<()> {
    // Awareness must be set before the first window exists, or the overlay is positioned
    // in virtualised coordinates and lands off-centre on any non-primary monitor.
    platform::make_process_dpi_aware();

    let dir = data_dir();
    std::fs::create_dir_all(&dir).ok();
    // Kept alive for the whole run: dropping the guard early truncates the log.
    let _log = init_logging(&dir);

    let catalog = match Catalog::load(&dir) {
        Ok(c) => c,
        Err(e) => {
            // Reachable only if the compiled-in dataset is broken, which the load test in
            // data::tests rules out. Anything the user owns is degraded instead of fatal.
            tracing::error!(error = %e, "could not load the built-in dataset; stopping");
            return Err(eframe::Error::AppCreation(Box::new(e)));
        }
    };

    let settings = catalog.settings.clone();
    // One line per run in the log, so "it stopped working" has a starting point: version,
    // where the data was read from, and what the hook was told to watch for.
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        data_dir = %dir.display(),
        apps = catalog.apps.len(),
        shortcuts = catalog
            .apps
            .iter()
            .map(|a| a.all_shortcuts().filter(|s| !s.removed).count())
            .sum::<usize>(),
        trigger = ?settings.trigger,
        hold_ms = settings.hold_ms,
        "shortcut-sheet starting"
    );
    // `--detect` answers one question and leaves: which process is in front, and which
    // profile that maps to. It opens no window and installs no hook, so it is safe to run
    // beside a live instance - and it is how the foreground-detection path gets measured on
    // a real desktop instead of only against a mocked exe name in a unit test.
    if std::env::args().skip(1).any(|a| a == "--detect") {
        let target = platform::foreground_window();
        let exe = target.and_then(|t| platform::process_name_of(t));
        let context = detect::resolve(&catalog, exe.as_deref());
        tracing::info!(?exe, ?context, "foreground probe");
        println!("exe={exe:?} context={context:?}");
        return Ok(());
    }

    // `--peek [app]` opens the overlay once, centred, and closes it again. It is the only way
    // to see and screenshot S01 without a hand on the keyboard, because the hook by design
    // ignores injected input. Peek mode installs no hook and no tray, so a verification run
    // cannot interfere with the machine it is checking.
    let peek = peek_app(&catalog, std::env::args().skip(1));
    let (events, tray) = match peek {
        Some(_) => (None, None),
        None => (
            Some(platform::spawn_hook(
                settings.trigger.vk(),
                Duration::from_millis(settings.hold_ms),
            )),
            Some(platform::spawn_tray()),
        ),
    };

    let viewport = egui::ViewportBuilder::default()
        .with_title(WINDOW_TITLE)
        .with_inner_size([ui::size::OVERLAY_WIDTH, 320.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_taskbar(false)
        .with_resizable(false)
        .with_always_on_top()
        .with_active(false)
        .with_visible(false)
        .with_window_level(egui::WindowLevel::AlwaysOnTop);

    let options = eframe::NativeOptions {
        viewport,
        // The overlay drives its own repaint: idle at 8 Hz while hidden, on-demand while
        // shown. Nothing here should ask the backend for a continuous animation loop.
        ..Default::default()
    };

    eframe::run_native(
        "shortcut-sheet",
        options,
        Box::new(move |_cc| Ok(Box::new(Sheet::new(catalog, events, tray, peek)))),
    )
}

/// The app `--peek` should show. `--peek` alone takes the first one in the dataset, and
/// `--peek=Some App` keeps a name with spaces in one argument, which is how a script and a
/// shell both manage to pass one. An unknown name is passed on and reported by the overlay
/// path, which logs it and closes.
fn peek_app(catalog: &Catalog, mut args: impl Iterator<Item = String>) -> Option<String> {
    let named = match args.next().as_deref() {
        Some("--peek") => args.next().filter(|a| !a.starts_with('-')),
        Some(first) if first.starts_with("--peek=") => {
            Some(first["--peek=".len()..].to_string())
        }
        _ => return None,
    };
    Some(named.unwrap_or_else(|| catalog.apps[0].name.clone()))
}

/// `%LOCALAPPDATA%\shortcut-sheet`, falling back to the current directory so a portable
/// run with a weird environment still finds somewhere writable.
fn data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|p| p.join("shortcut-sheet"))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("ss-data"))
}

/// A rolling daily log in the data directory. This exe has no console and no window of its
/// own most of the time, so a file is the only place a warning can go; `%LOCALAPPDATA%
/// shortcut-sheet\logs` is where a user or a bug report is told to look.
fn init_logging(dir: &Path) -> tracing_appender::non_blocking::WorkerGuard {
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs).ok();
    let file = tracing_appender::rolling::daily(&logs, "shortcut-sheet.log");
    let (writer, guard) = tracing_appender::non_blocking(file);
    // RUST_LOG overrides; otherwise everything above `info`, so every skipped file and
    // every dropped SendInput burst is on disk.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .init();
    // A panic in a windowless process is otherwise silent: it dies and leaves no trace.
    std::panic::set_hook(Box::new(|info| {
        tracing::error!(panic = %info, "shortcut-sheet stopped unexpectedly");
    }));
    guard
}
