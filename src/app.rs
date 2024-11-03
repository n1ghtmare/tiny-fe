use std::{
    env,
    fs::{DirEntry, ReadDir},
    path::{Path, PathBuf},
};

use anyhow::Ok;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*, DefaultTerminal};
use symbols::border;

/// Enum representing whether the system is currently showing a directory listing or paths from the
/// database.
#[derive(Debug, Clone, PartialEq)]
pub enum ListMode {
    /// The system is currently showing a directory listing.
    Directory,
    // TODO: Implement this mode
    /// The system is currently showing paths from the database that have been accessed frequently
    /// and recently.
    #[allow(dead_code)]
    Frecent,
}

/// The main application struct, will hold the state of the application.
#[derive(Debug)]
pub struct App {
    /// A boolean used to signal if the app should exit
    should_exit: bool,

    /// The current mode of the list
    list_mode: ListMode,

    /// A list representing the entries in the current working directory
    entry_list: EntryList,

    /// The current directory that the user is in
    current_directory: PathBuf,

    /// A boolean used to signal if the help popup should be shown
    show_help: bool,
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
    name: String,
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
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();

        let item = if file_type.is_dir() {
            Entry {
                path,
                kind: EntryKind::Directory,
                name,
            }
        } else {
            let extension = path.extension().map(|x| x.to_string_lossy().into_owned());

            Entry {
                path,
                kind: EntryKind::File { extension },
                name,
            }
        };

        Ok(item)
    }
}

impl From<&Entry> for ListItem<'_> {
    fn from(value: &Entry) -> Self {
        if value.kind == EntryKind::Directory {
            let style = Style::new().bold().fg(Color::White);
            ListItem::new(format!("{name}/", name = value.name)).style(style)
        } else {
            let style = Style::new().dark_gray();
            ListItem::new(value.name.clone()).style(style)
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_exit: false,
            list_mode: ListMode::Directory,
            entry_list: EntryList::default(),
            current_directory: PathBuf::new(),
            show_help: false,
        }
    }
}

impl App {
    /// Tries to create a new instance of the application - this will read the current directory
    /// and populate the entry list.
    pub fn try_new() -> anyhow::Result<Self> {
        let path = env::current_dir()?;
        let mut app = App::default();

        app.change_directory(path)?;

        Ok(app)
    }

    /// Changes the current directory and sorts the entries in the new directory.
    pub fn change_directory<T: AsRef<Path>>(&mut self, path: T) -> anyhow::Result<()> {
        let entries = std::fs::read_dir(path.as_ref())?;
        let mut entry_list = EntryList::try_from(entries)?;

        entry_list.items.sort_by(|a, b| {
            match (&a.kind, &b.kind) {
                (EntryKind::Directory, EntryKind::Directory)
                | (EntryKind::File { .. }, EntryKind::File { .. }) => a
                    .name
                    .to_lowercase()
                    .partial_cmp(&b.name.to_lowercase())
                    .unwrap(),
                // Otherwise, put folders first
                (EntryKind::Directory, EntryKind::File { .. }) => std::cmp::Ordering::Less,
                (EntryKind::File { .. }, EntryKind::Directory) => std::cmp::Ordering::Greater,
            }
        });

        // Add the parent directory after sorting so that it's always the first item
        if let Some(parent) = path.as_ref().parent() {
            entry_list.items.insert(
                0,
                Entry {
                    path: parent.to_path_buf(),
                    kind: EntryKind::Directory,
                    name: "..".into(),
                },
            );
        }

        self.should_exit = false;
        self.list_mode = ListMode::Directory;
        self.entry_list = entry_list;
        self.current_directory = path.as_ref().to_path_buf();

        Ok(())
    }

    fn change_list_mode(&mut self, mode: ListMode) -> anyhow::Result<()> {
        if self.list_mode == mode {
            return Ok(());
        }

        self.list_mode = mode;

        match self.list_mode {
            ListMode::Directory => self.change_directory(self.current_directory.clone()),
            ListMode::Frecent => {
                // TODO: Fetch the most frecent paths from the database
                self.entry_list = EntryList::default();
                Ok(())
            }
        }
    }

    /// Runs the application's main loop until the user quits.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        while !self.should_exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(&mut *self, frame.area());
    }

    fn render_help_popup(&self, buf: &mut Buffer) {
        let size = buf.area();

        // Define the popup area (e.g., centered and smaller than full screen)
        let popup_area = Rect {
            x: size.width / 4,
            y: size.height / 4,
            width: size.width / 2,
            height: size.height / 2,
        };

        let block = Block::default()
            .title(" Help ")
            .title_style(Style::default().bold().fg(Color::Red))
            .borders(Borders::ALL)
            .border_type(BorderType::Plain);

        let help_paragraph = Paragraph::new(Text::from(vec![
            Line::from("Key Bindings:"),
            Line::from(""),
            Line::from(vec![
                Span::styled("> j/k or ↓/↑", Style::default().fg(Color::Yellow)),
                Span::raw(" - Move down/up"),
            ]),
            Line::from(vec![
                Span::styled("> g/G or Home/End", Style::default().fg(Color::Yellow)),
                Span::raw(" - Go to top/bottom"),
            ]),
            Line::from(vec![
                Span::styled("> d/r", Style::default().fg(Color::Yellow)),
                Span::raw(" - Switch category (d)irectory or (f)recent"),
            ]),
            Line::from(vec![
                Span::styled("> Enter", Style::default().fg(Color::Yellow)),
                Span::raw(" - Select item"),
            ]),
            Line::from(vec![
                Span::styled("> ?", Style::default().fg(Color::Yellow)),
                Span::raw(" - Toggle help"),
            ]),
            Line::from(vec![
                Span::styled("> q or Esc", Style::default().fg(Color::Yellow)),
                Span::raw(" - Quit"),
            ]),
        ]))
        .reset()
        .block(block)
        .wrap(Wrap { trim: true })
        .alignment(Alignment::Left);

        // Render the help popup in the buffer
        help_paragraph.render(popup_area, buf);
    }

    /// Updates the application's state based on the user input.
    fn handle_events(&mut self) -> anyhow::Result<()> {
        match event::read()? {
            // It's important to check that the event is a key press event as crossterm also emits
            // key release and repeat events on Windows
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)?
            }
            // Ignore the rest
            _ => {}
        }

        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        match key.code {
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }

            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_exit = true;
            }

            KeyCode::Char('j') | KeyCode::Down => {
                self.show_help = false;
                self.entry_list.state.select_next();
            }

            KeyCode::Char('k') | KeyCode::Up => {
                self.show_help = false;
                self.entry_list.state.select_previous();
            }

            KeyCode::Char('g') | KeyCode::Home => {
                self.show_help = false;
                self.entry_list.state.select_first();
            }

            KeyCode::Char('G') | KeyCode::End => {
                self.show_help = false;
                self.entry_list.state.select_last();
            }

            KeyCode::Char('f') | KeyCode::Right => {
                self.show_help = false;
                self.change_list_mode(ListMode::Frecent)?;
            }

            KeyCode::Char('d') | KeyCode::Left => {
                self.show_help = false;
                self.change_list_mode(ListMode::Directory)?;
            }

            KeyCode::Enter => {
                self.show_help = false;
                let entry_index = self.entry_list.state.selected().unwrap_or_default();
                let selected_entry = &self.entry_list.items[entry_index];

                if selected_entry.kind == EntryKind::Directory {
                    // TODO: See if we can remove the clone here
                    self.change_directory(selected_entry.path.clone())?;
                } else {
                    // The user has selected a file, exit
                    self.should_exit = true;
                }
            }

            // Ignore the rest
            _ => {}
        }

        Ok(())
    }

    pub fn get_sub_header_title(&self) -> String {
        match &self.list_mode {
            ListMode::Directory => self.current_directory.to_string_lossy().into_owned(),
            ListMode::Frecent => "Most accessed paths".into(),
        }
    }

    fn render_header(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Tiny FE")
            .bold()
            .centered()
            .render(area, buf);
    }

    fn render_selected_tab_title(&mut self, area: Rect, buf: &mut Buffer) {
        let title = self.get_sub_header_title();

        Paragraph::new(title)
            .green()
            .left_aligned()
            .render(area, buf);
    }

    fn render_tabs(&mut self, area: Rect, buf: &mut Buffer) {
        let select_index = match self.list_mode {
            ListMode::Directory => 0,
            ListMode::Frecent => 1,
        };

        Tabs::new(["(d)irectory", "(f)recent"])
            .highlight_style(Style::new().fg(Color::Green))
            .select(select_index)
            .render(area, buf);
    }

    fn render_footer(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Press ? for help")
            .centered()
            .render(area, buf);
    }

    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_set(border::THICK)
            .border_style(Style::new().fg(Color::DarkGray));

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<ListItem> = self.entry_list.items.iter().map(ListItem::from).collect();

        // Create a List from all list items and highlight the currently selected one
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().bg(Color::Gray).fg(Color::Black))
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
        let [header_area, selected_tab_title_area, main_area, tabs_area, footer_area] =
            Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(area);

        let [list_area] = Layout::vertical([Constraint::Fill(1)]).areas(main_area);

        App::render_header(header_area, buf);
        App::render_footer(footer_area, buf);

        self.render_selected_tab_title(selected_tab_title_area, buf);
        self.render_tabs(tabs_area, buf);
        self.render_list(list_area, buf);

        if self.show_help {
            self.render_help_popup(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    fn create_test_app() -> App {
        App {
            should_exit: false,
            current_directory: PathBuf::from("/home/user"),
            list_mode: ListMode::Directory,
            entry_list: EntryList {
                items: vec![
                    Entry {
                        path: PathBuf::from("/home/user/.git/"),
                        kind: EntryKind::Directory,
                        name: ".git".into(),
                    },
                    Entry {
                        path: PathBuf::from("/home/user/.gitignore"),
                        kind: EntryKind::File { extension: None },
                        name: ".gitignore".into(),
                    },
                ],
                state: ListState::default(),
            },
            show_help: false,
        }
    }

    #[test]
    fn renders_correctly() {
        let mut app = create_test_app();
        let mut terminal = Terminal::new(TestBackend::new(80, 9)).unwrap();

        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .unwrap();

        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn renders_correctly_with_help_popup() {
        let mut app = create_test_app();
        app.show_help = true;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .unwrap();

        assert_snapshot!(terminal.backend());
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

        let _ = app.handle_key_event(KeyCode::Char('q').into());
        assert!(app.should_exit);

        let _ = app.handle_key_event(KeyCode::Esc.into());
        assert!(app.should_exit);

        let _ = app.handle_key_event(KeyCode::Char('j').into());
        assert_eq!(app.entry_list.state.selected(), Some(0));

        let _ = app.handle_key_event(KeyCode::Down.into());
        assert_eq!(app.entry_list.state.selected(), Some(1));

        // press down so that we can go back up more than once
        let _ = app.handle_key_event(KeyCode::Down.into());

        let _ = app.handle_key_event(KeyCode::Char('k').into());
        assert_eq!(app.entry_list.state.selected(), Some(1));

        let _ = app.handle_key_event(KeyCode::Up.into());
        assert_eq!(app.entry_list.state.selected(), Some(0));

        let _ = app.handle_key_event(KeyCode::Char('G').into());
        assert_eq!(app.entry_list.state.selected(), Some(usize::MAX));

        let _ = app.handle_key_event(KeyCode::Char('g').into());
        assert_eq!(app.entry_list.state.selected(), Some(0));

        let _ = app.handle_key_event(KeyCode::End.into());
        assert_eq!(app.entry_list.state.selected(), Some(usize::MAX));

        let _ = app.handle_key_event(KeyCode::Home.into());
        assert_eq!(app.entry_list.state.selected(), Some(0));

        let _ = app.handle_key_event(KeyCode::Char('d').into());
        assert_eq!(app.list_mode, ListMode::Directory);

        let _ = app.handle_key_event(KeyCode::Char('f').into());
        assert_eq!(app.list_mode, ListMode::Frecent);

        let _ = app.handle_key_event(KeyCode::Char('d').into());
        assert_eq!(app.list_mode, ListMode::Directory);

        let _ = app.handle_key_event(KeyCode::Char('?').into());
        assert!(app.show_help);
    }
}
