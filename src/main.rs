mod app;
mod file_tree;
mod input;
mod markdown;
mod ui;

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;

fn main() -> anyhow::Result<()> {
    // CLI args: `mdt [path]` defaulting to current directory.
    let path = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));

    let mut app = App::new(path)?;

    // --- Terminal setup ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

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

        if event::poll(Duration::from_millis(250))? {
            let event = event::read()?;
            match &event {
                Event::Key(key) if key.kind == crossterm::event::KeyEventKind::Press => {
                    app.handle_event(event);
                    needs_redraw = true;
                }
                Event::Resize(_, _) => {
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
