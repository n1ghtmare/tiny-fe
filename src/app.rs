use std::{
    env,
    fs::{DirEntry, ReadDir},
    path::PathBuf,
};

use anyhow::Ok;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*, DefaultTerminal};
use symbols::border;

/// The main application struct, will hold the state of the application.
#[derive(Debug)]
pub struct App {
    /// A boolean used to signal if the app should exit
    should_exit: bool,

    /// Current working directory
    current_working_directory: PathBuf,

    /// A list representing the entries in the current working directory
    entry_list: EntryList,
}

impl App {
    pub fn try_new() -> anyhow::Result<Self> {
        let current_working_directory = env::current_dir()?;
        let entries = std::fs::read_dir(&current_working_directory)?;

        let entry_list = EntryList::try_from(entries)?;

        Ok(Self {
            should_exit: false,
            current_working_directory,
            entry_list,
        })
    }
}

#[derive(Debug, Default)]
struct EntryList {
    items: Vec<Entry>,
    state: ListState,
}

impl EntryList {
    #[cfg(test)]
    fn len(&self) -> usize {
        self.items.len()
    }
}

impl TryFrom<ReadDir> for EntryList {
    type Error = anyhow::Error;

    fn try_from(value: ReadDir) -> Result<Self, Self::Error> {
        let mut items = Vec::new();

        for dir_entry_result in value.into_iter() {
            let dir_entry = dir_entry_result?;
            let item = Entry::try_from(dir_entry)?;
            items.push(item);
        }

        Ok(EntryList {
            items,
            state: ListState::default(),
        })
    }
}

#[derive(Debug)]
struct Entry {
    path: PathBuf,
    kind: EntryKind,
}

#[derive(Debug, PartialEq)]
enum EntryKind {
    File { extension: Option<String> },
    Directory,
}

impl TryFrom<DirEntry> for Entry {
    type Error = anyhow::Error;

    fn try_from(value: DirEntry) -> Result<Self, Self::Error> {
        let file_type = value.file_type()?;

        let path = value.path();
        let item = if file_type.is_dir() {
            Entry {
                path,
                kind: EntryKind::Directory,
            }
        } else {
            let extension = path.extension().map(|x| x.to_string_lossy().into_owned());

            Entry {
                path,
                kind: EntryKind::File { extension },
            }
        };

        Ok(item)
    }
}

impl From<&Entry> for ListItem<'_> {
    fn from(value: &Entry) -> Self {
        let name = value
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        if value.kind == EntryKind::Directory {
            let style = Style::new().bold();
            ListItem::new(format!("{name}/")).style(style)
        } else {
            let style = Style::new().light_cyan();
            ListItem::new(name).style(style)
        }
    }
}

impl App {
    /// Runs the application's main loop until the user quits.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        while !self.should_exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    /// Updates the application's state based on the user input.
    fn handle_events(&mut self) -> anyhow::Result<()> {
        match event::read()? {
            // It's important to check that the event is a key press event as crossterm also emits
            // key release and repeat events on Windows
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            // Ignore the rest
            _ => {}
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_exit = true;
            }

            KeyCode::Char('j') | KeyCode::Down => {
                self.entry_list.state.select_next();
            }

            KeyCode::Char('k') | KeyCode::Up => {
                self.entry_list.state.select_previous();
            }

            KeyCode::Char('g') | KeyCode::Home => {
                self.entry_list.state.select_first();
            }

            KeyCode::Char('G') | KeyCode::End => {
                self.entry_list.state.select_last();
            }

            KeyCode::Enter => {
                todo!()
            }

            // Ignore the rest
            _ => {}
        }
    }

    fn render_header(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Tiny FE")
            .bold()
            .centered()
            .render(area, buf);
    }

    fn render_sub_header(&mut self, area: Rect, buf: &mut Buffer) {
        let title = self.current_working_directory.to_string_lossy();

        Paragraph::new(title)
            .green()
            .left_aligned()
            .render(area, buf);
    }

    fn render_footer(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Use ↓↑ to move, g/G to go top/bottom, ENTER to select.")
            .centered()
            .render(area, buf);
    }

    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new().borders(Borders::ALL).border_set(border::THICK);

        // TODO: The items need to be ordered by type (file or directory) and then alphabetically.

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<ListItem> = self.entry_list.items.iter().map(ListItem::from).collect();

        // Create a List from all list items and highlight the currently selected one
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().bg(Color::DarkGray))
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        // If no item is selected, preselect the first item
        if self.entry_list.state.selected().is_none() {
            self.entry_list.state.select_first();
        }

        // We need to disambiguate this trait method as both `Widget` and `StatefulWidget` share
        // the same method name `render`.
        StatefulWidget::render(list, area, buf, &mut self.entry_list.state);
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let [header_area, sub_header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let [list_area] = Layout::vertical([Constraint::Fill(1)]).areas(main_area);

        App::render_header(header_area, buf);
        App::render_footer(footer_area, buf);

        self.render_sub_header(sub_header_area, buf);
        self.render_list(list_area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> App {
        App {
            should_exit: false,
            current_working_directory: PathBuf::from("/home/user"),
            entry_list: EntryList {
                items: vec![
                    Entry {
                        path: PathBuf::from("/home/user/.gitignore"),
                        kind: EntryKind::File { extension: None },
                    },
                    Entry {
                        path: PathBuf::from("/home/user/.git/"),
                        kind: EntryKind::Directory,
                    },
                ],
                state: ListState::default(),
            },
        }
    }

    #[test]
    fn render() {
        let mut app = create_test_app();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 79, 7));

        app.render(buffer.area, &mut buffer);

        let sub_header_text = app.current_working_directory.to_string_lossy();

        let mut expected = Buffer::with_lines(vec![
            "                                    Tiny FE                                    ",
            sub_header_text.as_ref(),
            "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓",
            "┃>.gitignore                                                                  ┃",
            "┃ .git/                                                                       ┃",
            "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛",
            "            Use ↓↑ to move, g/G to go top/bottom, ENTER to select.             ",
        ]);

        // Apply BOLD to the entire first line
        expected.set_style(Rect::new(0, 0, 79, 1), Style::new().bold());

        // Apply Green foreground color to the second line
        expected.set_style(Rect::new(0, 1, 79, 1), Style::new().fg(Color::Green));

        // Ensure no styles are applied to the third line
        expected.set_style(Rect::new(0, 2, 79, 1), Style::new());

        // Apply DarkGray background and LightCyan foreground to the highlighted line
        expected.set_style(
            Rect::new(1, 3, 77, 1),
            Style::new().bg(Color::DarkGray).fg(Color::LightCyan),
        );

        // Clear styles at the end of the highlighted line
        expected.set_style(Rect::new(78, 3, 1, 1), Style::new());

        // Reset styles at the beginning of line 4
        expected.set_style(Rect::new(0, 4, 79, 1), Style::new());

        // Apply BOLD to text at line 4, starting at x=1
        expected.set_style(Rect::new(1, 4, 77, 1), Style::new().bold());

        // Clear BOLD at the end of line 4
        expected.set_style(Rect::new(78, 4, 1, 1), Style::new());

        assert_eq!(buffer, expected);
    }

    #[test]
    fn first_item_is_preselected_after_render() {
        let mut app = create_test_app();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 79, 10));

        assert_eq!(app.entry_list.state.selected(), None);

        app.render(buffer.area, &mut buffer);

        assert_eq!(app.entry_list.state.selected(), Some(0));
    }

    #[test]
    fn handle_key_event() {
        let mut app = create_test_app();

        // Make sure we have 2 items
        assert_eq!(app.entry_list.len(), 2);

        app.handle_key_event(KeyCode::Char('q').into());
        assert!(app.should_exit);

        app.handle_key_event(KeyCode::Esc.into());
        assert!(app.should_exit);

        app.handle_key_event(KeyCode::Char('j').into());
        assert_eq!(app.entry_list.state.selected(), Some(0));

        app.handle_key_event(KeyCode::Down.into());
        assert_eq!(app.entry_list.state.selected(), Some(1));

        // press down so that we can go back up more than once
        app.handle_key_event(KeyCode::Down.into());

        app.handle_key_event(KeyCode::Char('k').into());
        assert_eq!(app.entry_list.state.selected(), Some(1));

        app.handle_key_event(KeyCode::Up.into());
        assert_eq!(app.entry_list.state.selected(), Some(0));

        app.handle_key_event(KeyCode::Char('G').into());
        assert_eq!(app.entry_list.state.selected(), Some(usize::MAX));

        app.handle_key_event(KeyCode::Char('g').into());
        assert_eq!(app.entry_list.state.selected(), Some(0));

        app.handle_key_event(KeyCode::End.into());
        assert_eq!(app.entry_list.state.selected(), Some(usize::MAX));

        app.handle_key_event(KeyCode::Home.into());
        assert_eq!(app.entry_list.state.selected(), Some(0));
    }
}
