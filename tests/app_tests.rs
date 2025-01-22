use std::fs::{create_dir, File};

use crossterm::event::{KeyCode, KeyModifiers};
use insta::assert_snapshot;
use ratatui::{backend::TestBackend, Terminal};

use tiny_fe::app::App;

#[test]
fn change_directory_lists_correct_directory_entires() {
    // Create a temporary directory with a static name so that test snapshots are consistent
    let temp_dir = tempfile::Builder::new()
        .prefix("tiny_fe")
        .rand_bytes(0)
        .tempdir()
        .unwrap();

    let temp_path = temp_dir.path();

    // Create some temp files in the firectory
    let file_1 = temp_path.join("file_1.txt");
    File::create(&file_1).unwrap();

    let file_2 = temp_path.join("file_2.txt");
    File::create(&file_2).unwrap();

    // Create a temporary subdirectory
    let sub_dir = temp_path.join("sub_dir");
    create_dir(&sub_dir).unwrap();

    let mut app = App::default();
    app.change_directory(temp_path).unwrap();

    let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
    terminal
        .draw(|frame| frame.render_widget(&mut app, frame.area()))
        .unwrap();

    assert_snapshot!(terminal.backend());
}
