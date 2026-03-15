mod app;
mod file_ops;
mod file_tree;
mod input;
#[cfg(test)]
mod integration_tests;
mod markdown;
#[cfg(test)]
mod test_util;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event, KeyEventKind};
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

    // Pre-warm syntax highlighting data on a background thread so the first
    // file open with code blocks doesn't stall the UI for 100-300ms.
    std::thread::spawn(|| {
        crate::markdown::syntax::syntax_set();
        crate::markdown::syntax::scope_matchers();
    });

    // Detect terminal background color for solid fill (prevents transparency).
    // Must be called before enable_raw_mode() since the crate manages its own raw mode.
    let bg_color = {
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

    // --- Terminal setup ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Install panic hook to restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, crossterm::cursor::Show);
        original_hook(panic_info);
    }));

    // Run event loop; capture result so we always tear down.
    let result = run_loop(&mut terminal, &mut app);

    // --- Terminal teardown (always runs) ---
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Release advisory lock and clean up lock file.
    drop(lock_file);
    let _ = std::fs::remove_file(&lock_path);

    result
}

/// Dirty-flag event loop.
fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
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
                Event::Resize(_, _) => {
                    terminal.autoresize()?;
                    needs_redraw = true;
                }
                _ => {}
            }
        }

        // Force redraw when cursor-bearing overlays are active (for blink animation).
        let had_cursor = app.cursor_visible;
        app.tick_cursor();
        if app.cursor_visible != had_cursor && (app.show_file_op || app.show_links) {
            needs_redraw = true;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
