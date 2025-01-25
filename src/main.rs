use std::{io, path::PathBuf};

use crossterm::{
    cursor, execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

use tiny_fe::app::{App, ListMode};

fn main() -> anyhow::Result<()> {
    // Enter the alternate screen and hide the cursor
    execute!(io::stderr(), EnterAlternateScreen)?;
    execute!(io::stderr(), cursor::Hide)?;

    // Enable raw mode
    terminal::enable_raw_mode()?;

    let result = run_app_ui();

    // Restore the terminal state
    terminal::disable_raw_mode()?;

    // Leave the alternate screen and show the cursor
    execute!(io::stderr(), cursor::Show)?;
    execute!(io::stderr(), LeaveAlternateScreen)?;

    match result {
        Ok(path) => {
            println!("{}", path.display());
        }
        Err(err) => {
            eprintln!("Error: {}", err);
        }
    }

    Ok(())
}

fn run_app_ui() -> anyhow::Result<PathBuf> {
    let mut app = App::try_new(ListMode::default())?;

    // Initialize the terminal backend
    let backend = ratatui::backend::CrosstermBackend::new(io::stderr());
    let mut terminal = ratatui::Terminal::new(backend)?;

    app.run(&mut terminal)
}
