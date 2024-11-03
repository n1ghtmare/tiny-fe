use std::fs::{create_dir, File};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    widgets::Widget,
};
use tempfile::tempdir;
use tiny_fe::app::App;

#[test]
fn change_directory_lists_correct_directory_entires() {
    // Create a temporary directory
    let temp_dir = tempdir().unwrap();
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

    let mut buffer = Buffer::empty(Rect::new(0, 0, 79, 9));

    app.render(buffer.area, &mut buffer);

    let sub_header_text = app.get_sub_header_title();

    let mut expected = Buffer::with_lines(vec![
        "                                    Tiny FE                                    ",
        sub_header_text.as_ref(),
        "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓",
        "┃>../                                                                         ┃",
        "┃ sub_dir/                                                                    ┃",
        "┃ file_1.txt                                                                  ┃",
        "┃ file_2.txt                                                                  ┃",
        "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛",
        "            Use ↓↑ to move, g/G to go top/bottom, ENTER to select.             ",
    ]);

    // Apply BOLD to the entire first line (header)
    expected.set_style(Rect::new(0, 0, 79, 1), Style::new().bold());

    // Apply Green foreground color to the second line (sub-header)
    expected.set_style(Rect::new(0, 1, 79, 1), Style::new().fg(Color::Green));

    // Ensure no styles are applied to the third line (border)
    expected.set_style(Rect::new(0, 2, 79, 1), Style::new());

    // Apply DarkGray background and BOLD modifier to the highlighted item (line 3)
    expected.set_style(
        Rect::new(1, 3, 77, 1),
        Style::new().bg(Color::DarkGray).bold(),
    );

    // Clear styles at the end of the highlighted line
    expected.set_style(Rect::new(78, 3, 1, 1), Style::new());

    // Apply BOLD to the directory entry (line 4)
    expected.set_style(Rect::new(1, 4, 77, 1), Style::new().bold());

    // Clear styles at the end of line 4
    expected.set_style(Rect::new(78, 4, 1, 1), Style::new());

    // Apply LightCyan foreground color to the first file entry (line 5)
    expected.set_style(Rect::new(1, 5, 77, 1), Style::new().fg(Color::LightCyan));

    // Clear styles at the end of line 5
    expected.set_style(Rect::new(78, 5, 1, 1), Style::new());

    // Apply LightCyan foreground color to the second file entry (line 6)
    expected.set_style(Rect::new(1, 6, 77, 1), Style::new().fg(Color::LightCyan));

    // Clear styles at the end of line 6
    expected.set_style(Rect::new(78, 6, 1, 1), Style::new());

    assert_eq!(buffer, expected);
}
