use std::fs::{create_dir, File};

use crossterm::event::{KeyCode, KeyModifiers};
use insta::assert_snapshot;
use ratatui::{backend::TestBackend, Terminal};

use tiny_dc::app::App;

#[test]
fn change_directory_lists_correct_directory_entires() {
    // Create a temporary directory with a static name so that test snapshots are consistent
    let temp_dir = tempfile::Builder::new()
        .prefix("tiny_dc")
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

#[test]
fn entry_hotkey_jumps_successfully() {
    // Create a temporary directory with a static name so that test snapshots are consistent
    let temp_dir = tempfile::Builder::new()
        .prefix("tiny_dc_jump")
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
    create_dir(temp_path.join("sub_dir")).unwrap();

    let mut app = App::default();

    app.change_directory(temp_path).unwrap();

    let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();

    terminal
        .draw(|frame| frame.render_widget(&mut app, frame.area()))
        .unwrap();

    // Jump to the subdirectory using the hotkey `a` (it gets the highest priority because it's the
    // first and `a` is the first in the preferred hotkeys)
    app.handle_key_event(KeyCode::Char('a').into(), KeyModifiers::NONE)
        .unwrap();

    terminal
        .draw(|frame| frame.render_widget(&mut app, frame.area()))
        .unwrap();

    assert_snapshot!(terminal.backend());
}

#[test]
fn entry_hotkey_missing_when_in_search_mode_and_search_input_is_empty() {
    // Create a temporary directory with a static name so that test snapshots are consistent
    let temp_dir = tempfile::Builder::new()
        .prefix("tiny_dc_jump_search_1")
        .rand_bytes(0)
        .tempdir()
        .unwrap();

    let temp_path = temp_dir.path();

    // Create some temp files in the firectory
    let file_1 = temp_path.join("file_1_s.txt");
    File::create(&file_1).unwrap();

    let file_2 = temp_path.join("file_2_s.txt");
    File::create(&file_2).unwrap();

    // Create a temporary subdirectory
    create_dir(temp_path.join("sub_dir")).unwrap();

    let mut app = App::default();

    app.change_directory(temp_path).unwrap();

    let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();

    terminal
        .draw(|frame| frame.render_widget(&mut app, frame.area()))
        .unwrap();

    // Enter search mode
    app.handle_key_event(KeyCode::Char('/').into(), KeyModifiers::NONE)
        .unwrap();

    terminal
        .draw(|frame| frame.render_widget(&mut app, frame.area()))
        .unwrap();

    assert_snapshot!(terminal.backend());
}

#[test]
fn entry_hotkey_jumps_successfully_in_search_mode() {
    // Create a temporary directory with a static name so that test snapshots are consistent
    let temp_dir = tempfile::Builder::new()
        .prefix("tiny_dc_jump_search_2")
        .rand_bytes(0)
        .tempdir()
        .unwrap();

    let temp_path = temp_dir.path();

    // Create some temp files in the firectory
    let file_1 = temp_path.join("file_1_s.txt");
    File::create(&file_1).unwrap();

    let file_2 = temp_path.join("file_2_s.txt");
    File::create(&file_2).unwrap();

    // Create a temporary subdirectory
    create_dir(temp_path.join("sub_dir")).unwrap();

    let mut app = App::default();

    app.change_directory(temp_path).unwrap();

    let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();

    terminal
        .draw(|frame| frame.render_widget(&mut app, frame.area()))
        .unwrap();

    // Enter search mode
    app.handle_key_event(KeyCode::Char('/').into(), KeyModifiers::NONE)
        .unwrap();

    // Search for an entry that contains s (in this case it should be all of them)
    app.handle_key_event(KeyCode::Char('s').into(), KeyModifiers::NONE)
        .unwrap();

    terminal
        .draw(|frame| frame.render_widget(&mut app, frame.area()))
        .unwrap();

    // Jump to the subdirectory using the hotkey `a` (it gets the highest priority because it's the
    // first and `a` is the first in the preferred hotkeys)
    app.handle_key_event(KeyCode::Char('a').into(), KeyModifiers::NONE)
        .unwrap();

    terminal
        .draw(|frame| frame.render_widget(&mut app, frame.area()))
        .unwrap();

    assert_snapshot!(terminal.backend());
}

#[test]
fn app_returns_expected_path_after_exit() {
    // Create a temporary directory with a static name so that test snapshots are consistent
    let temp_dir = tempfile::Builder::new().tempdir().unwrap();

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
    app.change_directory(&temp_dir).unwrap();

    let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();

    // Move into the subdirectory
    app.handle_key_event(KeyCode::Enter.into(), KeyModifiers::NONE)
        .unwrap();

    // Exit the app
    app.handle_key_event(KeyCode::Esc.into(), KeyModifiers::NONE)
        .unwrap();

    let result = app.run(&mut terminal).unwrap();

    // The app should return the path of the subdirectory since that's where we exited
    assert_eq!(result, sub_dir);
}
