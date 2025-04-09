use std::{env, io, path::PathBuf};

use clap::{Parser, Subcommand};
use crossterm::{
    cursor, execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};

use tiny_dc::{
    app::{App, ListMode},
    index::{DirectoryIndex, DEFAULT_INDEX_FILE_NAME},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    directory_command: Option<DirectoryCommand>,
}

/// An optional directory sub-command, to launch into the TUI instead, use `tiny-dc` without
/// passing any args
#[derive(Subcommand, Debug)]
enum DirectoryCommand {
    /// Pushes a directory to the index
    Push { path: PathBuf },
    /// Prints the path of the first indexed directory matching the query (intended to be used with
    /// shell integration), if no match is found, the current directory is printed
    Z { query: String },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(directory_command) = cli.directory_command {
        // TODO: Make this cross platform, this won't work on Windows the way it is
        let home_dir = env::var("HOME")?;

        let index_file_path = format!("{home_dir}/{DEFAULT_INDEX_FILE_NAME}");
        let mut directory_index = DirectoryIndex::load_from_disk(PathBuf::from(index_file_path))?;

        match directory_command {
            DirectoryCommand::Push { path } => {
                directory_index.push_entry(&path);
            }
            DirectoryCommand::Z { query } => {
                let result = directory_index.find_top_ranked(&query);
                if let Some(path) = result {
                    println!("{}", path.display());
                } else {
                    let current_dir = env::current_dir()?;
                    println!("{}", current_dir.display());
                }
            }
        }

        directory_index.save_to_disk()?;
    } else {
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
