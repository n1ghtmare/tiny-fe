use tiny_fe::app::App;

fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    // Clear the terminal and store its current state
    terminal.clear()?;

    let mut app = App::try_new()?;
    let app_result = app.run(&mut terminal)?;

    // Restore the state of the terminal before the app was opened
    ratatui::restore();

    // Return any errors that we've incountered {if any}
    println!("{}", app_result.to_string_lossy());

    Ok(())
}
