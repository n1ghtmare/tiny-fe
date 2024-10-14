mod app;

use app::App;

fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    // Clear the terminal and store its current state
    terminal.clear()?;

    let app_result = App::default().run(&mut terminal);

    // Restore the state of the terminal before the app was opened
    ratatui::restore();

    // Return any errors that we've incountered {if any}
    app_result
}
