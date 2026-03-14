mod app;
mod file_tree;
mod input;
mod markdown;
#[cfg(test)]
mod test_util;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;

const EVENT_POLL_MS: u64 = 250;

fn main() -> anyhow::Result<()> {
    // CLI args: `mdt [path]` defaulting to current directory.
    let path = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));

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

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
