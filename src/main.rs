mod app;
mod file_ops;
mod file_tree;
mod input;
#[cfg(test)]
mod integration_tests;
mod markdown;
mod palette;
#[cfg(test)]
mod test_util;
mod ui;
mod watcher;

use std::io::{self, IsTerminal, Read as _};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;

const EVENT_POLL_MS: u64 = 50;

#[derive(Parser)]
#[command(name = "mdt", about = "Terminal Markdown Viewer")]
struct Cli {
    /// Directory or file to open (defaults to current directory)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Maximum file size in bytes (default: 5000000 = 5MB)
    #[arg(long, default_value_t = App::DEFAULT_MAX_FILE_SIZE)]
    max_file_size: u64,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let path = cli.path;

    // Detect piped stdin: either stdin is not a terminal, or the user passed "-".
    let is_stdin = !io::stdin().is_terminal() || path.as_os_str() == "-";

    // Read stdin eagerly before any terminal setup (must happen while stdin is still a pipe).
    let stdin_content = if is_stdin {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        if buf.len() as u64 > cli.max_file_size {
            let mb = cli.max_file_size / 1_000_000;
            anyhow::bail!("stdin too large (>{mb}MB)");
        }
        Some(buf)
    } else {
        None
    };

    // Pre-warm syntax highlighting on a background thread: loads the SyntaxSet,
    // ScopeMatchers, and pre-compiles regex patterns for common languages so the
    // first file open doesn't stall the UI.
    let syntax_warmup = std::thread::spawn(crate::markdown::syntax::prewarm_syntaxes);

    // Detect terminal background color for solid fill (prevents transparency).
    // Must be called before enable_raw_mode() since the crate manages its own raw mode.
    // Skip when stdin is piped — terminal_colorsaurus sends/reads escape sequences
    // that conflict with piped stdin.
    let bg_color = if is_stdin {
        ratatui::style::Color::Reset
    } else {
        use terminal_colorsaurus::{background_color, QueryOptions};
        let mut opts = QueryOptions::default();
        opts.timeout = Duration::from_millis(150);
        match background_color(opts) {
            Ok(bg) => {
                ratatui::style::Color::Rgb((bg.r >> 8) as u8, (bg.g >> 8) as u8, (bg.b >> 8) as u8)
            }
            Err(_) => ratatui::style::Color::Reset,
        }
    };

    // Branch: stdin mode vs file mode.
    let (mut app, lock_path, lock_file, fs_rx, watcher_handle) = if let Some(content) =
        stdin_content
    {
        let mut app = App::from_stdin(content, bg_color);
        app.max_file_size = cli.max_file_size;
        (app, None, None, None, None)
    } else {
        let mut app = App::new(&path, bg_color)?;
        app.max_file_size = cli.max_file_size;

        // Acquire an advisory lock to prevent concurrent mdt instances on the same directory.
        let lock_path = app.root_path.join(".mdt.lock");
        let lock_file =
            std::fs::OpenOptions::new().create(true).write(true).truncate(true).open(&lock_path)?;
        use fs2::FileExt;
        if lock_file.try_lock_exclusive().is_err() {
            anyhow::bail!("another mdt instance is already running on {}", app.root_path.display());
        }

        // Spawn filesystem watcher for auto-reload.
        let (rx, handle) = watcher::spawn_watcher(&app.root_path)?;
        (app, Some(lock_path), Some(lock_file), Some(rx), Some(handle))
    };

    // --- Terminal setup ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Install panic hook to restore terminal and clean up lock file on panic
    let original_hook = std::panic::take_hook();
    let panic_lock_path = lock_path.clone();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            crossterm::cursor::Show
        );
        if let Some(ref p) = panic_lock_path {
            let _ = std::fs::remove_file(p);
        }
        original_hook(panic_info);
    }));

    // Let syntax pre-warming finish in the background; the OnceLock statics
    // guarantee thread-safe access and the warmup completes well before a user
    // can open a file.  Dropping the handle detaches the thread.
    drop(syntax_warmup);

    // Run event loop; capture result so we always tear down.
    let result = run_loop(&mut terminal, &mut app, fs_rx.as_ref());

    // Stop the watcher thread (if any).
    if let Some(handle) = watcher_handle {
        handle.shutdown();
    }

    // --- Terminal teardown (always runs) ---
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    // Release advisory lock and clean up lock file (if any).
    drop(lock_file);
    if let Some(ref p) = lock_path {
        let _ = std::fs::remove_file(p);
    }

    result
}

/// Dirty-flag event loop.
fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    fs_rx: Option<&mpsc::Receiver<watcher::FsEvent>>,
) -> anyhow::Result<()> {
    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            terminal.draw(|f| ui::draw(f, app))?;
            needs_redraw = false;
        }

        if event::poll(Duration::from_millis(EVENT_POLL_MS))? {
            let event = event::read()?;
            match event {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    app.handle_event(key);
                    needs_redraw = true;
                }
                Event::Mouse(mouse) => {
                    app.handle_mouse(mouse);
                    needs_redraw = true;
                }
                Event::Resize(_, _) => {
                    terminal.autoresize()?;
                    needs_redraw = true;
                }
                _ => {}
            }
        }

        // Drain filesystem watcher events.
        if let Some(rx) = fs_rx {
            while let Ok(fs_event) = rx.try_recv() {
                app.handle_fs_event(fs_event);
                needs_redraw = true;
            }
        }

        // Check live preview debounce timer.
        if let Some(debounce_time) = app.live_preview.debounce {
            if debounce_time.elapsed() >= Duration::from_millis(150) {
                app.update_live_preview();
                needs_redraw = true;
            }
        }

        // Force redraw when cursor-bearing overlays are active (for blink animation).
        let had_cursor = app.cursor.visible;
        app.tick_cursor();
        if app.cursor.visible != had_cursor
            && matches!(
                app.overlay,
                crate::app::Overlay::FileOp(_) | crate::app::Overlay::LinkPicker
            )
        {
            needs_redraw = true;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
