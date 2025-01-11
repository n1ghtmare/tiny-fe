use std::{
    collections::{HashMap, HashSet},
    env, fmt,
    fs::{DirEntry, ReadDir},
    ops::Deref,
    path::{Path, PathBuf},
};

use anyhow::Ok;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*, DefaultTerminal};
use symbols::border;

/// The preferred shortcuts for the entries in the list. These will be used to quickly jump to an
/// entry and will be chosed based on the order that they appear in this array, this way we can
/// prioritize ergonomics. In future versions, we might allow the user to customize these
/// shortcuts.
const PREFERRED_ENTRY_SHORTCUTS: [char; 29] = [
    'a', 's', 'w', 'e', 'r', 't', 'z', 'x', 'c', 'v', 'b', 'y', 'u', 'i', 'o', 'p', 'n', 'm', ',',
    '1', '2', '3', '4', '5', '6', '7', '8', '9', '0',
];

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

#[derive(Debug, PartialEq)]
pub enum InputMode {
    Normal,
    Search,
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

    /// The list state, used to keep track of the selected item
    list_state: ListState,

    /// The current directory that the user is in
    current_directory: PathBuf,

    /// A boolean used to signal if the help popup should be shown
    show_help: bool,

    /// Current input mode
    input_mode: InputMode,

    /// The search input
    search_input: SearchInput,

    /// The cursor position
    cursor_position: Option<(u16, u16)>,

    /// The map of shortcuts to entry indices
    shortcuts: HashMap<char, usize>,
}

/// The search input struct, used to store the search input value and the current index.
#[derive(Debug, Default)]
pub struct SearchInput {
    /// The search input value
    value: String,

    /// Search input character index
    index: usize,
}

impl SearchInput {
    pub fn clear(&mut self) {
        self.value.clear();
        self.index = 0;
    }

    pub fn push(&mut self, c: char) {
        self.value.push(c);
        self.index += 1;
    }

    pub fn pop(&mut self) {
        self.value.pop();
        self.index -= 1;
    }
}

impl Deref for SearchInput {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl AsRef<str> for SearchInput {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for SearchInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Default)]
struct EntryList {
    items: Vec<Entry>,
    filtered_indices: Option<Vec<usize>>,
}

impl EntryList {
    #[cfg(test)]
    fn len(&self) -> usize {
        self.items.len()
    }

    fn get_filtered_entries(&self) -> Vec<&Entry> {
        match &self.filtered_indices {
            Some(indices) => indices.iter().map(|&i| &self.items[i]).collect(),
            None => self.items.iter().collect(),
        }
    }

    fn update_filtered_indices<T: AsRef<str>>(&mut self, value: T) {
        let value = value.as_ref().to_lowercase();

        if value.is_empty() {
            self.filtered_indices = None;
        } else {
            let indices = self
                .items
                .iter()
                .enumerate()
                .filter_map(|(i, entry)| {
                    if entry.name.to_lowercase().contains(&value) {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();

            self.filtered_indices = Some(indices);
        }
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
            ..Default::default()
        })
    }
}

#[derive(Debug)]
struct Entry {
    path: PathBuf,
    kind: EntryKind,
    name: String,
}

/// This struct represents the data that will be used to render an entry in the list. It is used in
/// conjunction with the search query to determine how to render the entry.
///
/// It holds the prefix, search hit and suffix of the entry name, the next character after the
/// search hit, the kind of the entry and the shortcut assigned to the entry.
///
/// This allows us to render the entry in the UI with the search hit underlined and the shortcut
/// displayed next to the entry.
///
/// For example, if the entry name is "Cargo.toml" and the search query is "ar", the prefix will be
/// "C", the search hit will be "ar", the suffix will be "go.toml", the next character will be "g"
/// (the character immediately after the search hit)
///
/// The shortcut is assigned at a later stage and is used to quickly jump to the entry.
#[derive(Debug, PartialEq)]
struct EntryRenderData<'a> {
    /// A boolean indicating if the entry is dynamic (e.g., "..", which is not a "real" entry)
    is_dynamic: bool,
    prefix: &'a str,
    search_hit: &'a str,
    suffix: &'a str,
    /// The next character immediately after the search hit
    next_char: Option<char>,
    /// The kind of the entry, we need to keep track of this because we render directories
    /// differently than files
    kind: &'a EntryKind,
    /// The shortcut assigned to the entry, it's an optional character, some entries might not have
    /// a shortcut (files don't have shortcuts)
    shortcut: Option<char>,
}

impl EntryRenderData<'_> {
    pub fn from_entry<T: AsRef<str>>(entry: &Entry, search_query: T) -> EntryRenderData {
        if entry.name == ".." {
            return EntryRenderData {
                is_dynamic: true,
                prefix: &entry.name,
                search_hit: "",
                suffix: "",
                next_char: None,
                kind: &EntryKind::Directory,
                shortcut: None,
            };
        }

        let search_query = search_query.as_ref();
        let name = entry.name.to_lowercase();
        let search_query = search_query.to_lowercase();

        if let Some(index) = name.find(&search_query) {
            let prefix = &entry.name[..index];
            let search_hit = &entry.name[index..(index + search_query.len())];
            let suffix = &entry.name[(index + search_query.len())..];
            let next_char = suffix.chars().next();

            EntryRenderData {
                is_dynamic: false,
                prefix,
                search_hit,
                suffix,
                next_char,
                kind: &entry.kind,
                shortcut: None,
            }
        } else {
            EntryRenderData {
                is_dynamic: false,
                prefix: &entry.name,
                search_hit: "",
                suffix: "",
                next_char: entry.name.chars().next(),
                kind: &entry.kind,
                shortcut: None,
            }
        }
    }
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

impl<'a> From<EntryRenderData<'a>> for ListItem<'a> {
    fn from(value: EntryRenderData<'a>) -> Self {
        let mut spans: Vec<Span> = Vec::new();

        // we want to display the search hit with underscore
        spans.push(Span::raw(value.prefix));
        spans.push(Span::styled(
            value.search_hit,
            Style::default().underlined(),
        ));
        spans.push(Span::raw(value.suffix));

        if value.kind == &EntryKind::Directory {
            spans.push(Span::raw("/"));

            if let Some(shortcut) = value.shortcut {
                spans.push(Span::raw("  ").style(Style::default().dark_gray()));
                spans.push(Span::styled(
                    shortcut.to_string(),
                    Style::default().black().on_green(),
                ));
            }

            let line = Line::from(spans);
            let style = Style::new().bold().fg(Color::White);

            ListItem::new(line).style(style)
        } else {
            let style = Style::new().dark_gray();
            let k = Line::from(spans);
            ListItem::new(k).style(style)
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_exit: false,
            list_mode: ListMode::Directory,
            entry_list: EntryList::default(),
            list_state: ListState::default(),
            current_directory: PathBuf::new(),
            show_help: false,
            input_mode: InputMode::Normal,
            search_input: SearchInput::default(),
            cursor_position: None,
            shortcuts: HashMap::new(),
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
        self.search_input.clear();

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

        // After rendering, set the cursor position if needed
        if let Some((x, y)) = self.cursor_position {
            frame.set_cursor_position(Position::new(x, y));
        }
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
            Line::from(vec![
                Span::styled("> /", Style::default().fg(Color::Yellow)),
                Span::raw(" - Search"),
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

    fn change_directory_to_entry_index(&mut self, index: usize) -> anyhow::Result<()> {
        let entries = self.entry_list.get_filtered_entries();
        let selected_entry = entries.get(index);

        if let Some(selected_entry) = selected_entry {
            if selected_entry.kind == EntryKind::Directory {
                self.change_directory(selected_entry.path.clone())?;
            } else {
                // The user has selected a file, exit
                self.should_exit = true;
            }
        }

        Ok(())
    }

    fn update_filtered_indices(&mut self) {
        self.entry_list.update_filtered_indices(&self.search_input);
        self.list_state = ListState::default();
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> anyhow::Result<()> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        match self.input_mode {
            InputMode::Normal => {
                match key.code {
                    KeyCode::Char('/') => {
                        // Enter search mode
                        self.input_mode = InputMode::Search;
                        self.search_input.clear();
                        self.update_filtered_indices();
                    }
                    KeyCode::Char('?') => {
                        self.show_help = !self.show_help;
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        if self.show_help {
                            self.show_help = false;
                        } else {
                            self.should_exit = true;
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        self.show_help = false;
                        self.list_state.select_next();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        self.show_help = false;
                        self.list_state.select_previous();
                    }
                    KeyCode::Char('g') | KeyCode::Home => {
                        self.show_help = false;
                        self.list_state.select_first();
                    }
                    KeyCode::Char('G') | KeyCode::End => {
                        self.show_help = false;
                        self.list_state.select_last();
                    }
                    KeyCode::Char('f') | KeyCode::Right => {
                        self.show_help = false;
                        self.change_list_mode(ListMode::Frecent)?;
                    }
                    KeyCode::Char('d') | KeyCode::Left => {
                        self.show_help = false;
                        self.change_list_mode(ListMode::Directory)?;
                    }
                    KeyCode::Char('_') => {
                        // clear the search input while in search mode
                        self.search_input.clear();
                        self.update_filtered_indices();
                    }
                    KeyCode::Char(c) => {
                        // find the entry with the shortcut (if one exists) and select it
                        if let Some(entry_index) = self.shortcuts.get(&c) {
                            self.change_directory_to_entry_index(*entry_index)?;
                        }
                    }
                    KeyCode::Enter => {
                        self.show_help = false;
                        let entry_index = self.list_state.selected().unwrap_or_default();
                        self.change_directory_to_entry_index(entry_index)?;
                    }
                    // Ignore the rest
                    _ => {}
                }
            }

            // In search mode we need to handle the search input differently, we only care about
            // the characters and the backspace key
            InputMode::Search => {
                match key.code {
                    KeyCode::Enter => {
                        // Exit search mode
                        self.input_mode = InputMode::Normal;
                    }
                    KeyCode::Esc => {
                        // Exit search mode
                        self.input_mode = InputMode::Normal;
                        self.search_input.clear();
                        self.update_filtered_indices();
                    }
                    KeyCode::Char(c) => {
                        // find the entry with the shortcut (if one exists) and select it
                        if let Some(entry_index) = self.shortcuts.get(&c) {
                            self.change_directory_to_entry_index(*entry_index)?;
                            self.input_mode = InputMode::Normal;
                            self.search_input.clear();
                        } else {
                            // Add character to the search input
                            self.search_input.push(c);
                            self.update_filtered_indices();
                        }
                    }
                    KeyCode::Backspace => {
                        // Remove character from the search input
                        if self.search_input.index > 0 {
                            self.search_input.pop();
                            self.update_filtered_indices();
                        } else {
                            // Exit search mode
                            self.input_mode = InputMode::Normal;
                        }
                    }
                    // Ignore the rest
                    _ => {}
                }
            }
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
        let line = Line::from(vec![
            Span::styled("|>", Style::default().dark_gray()),
            Span::raw(" "),
            Span::styled(self.get_sub_header_title(), Style::default().green()),
        ]);

        Paragraph::new(Text::from(vec![line])).render(area, buf);
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

    fn render_footer(&mut self, area: Rect, buf: &mut Buffer) {
        let input = format!(" /{input}", input = self.search_input);

        if self.input_mode == InputMode::Search {
            Paragraph::new(input)
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Left)
                .render(area, buf);

            // Calculate the cursor poisition and account for the space and '/' characters
            let cursor_x = area.x + 2 + self.search_input.index as u16;
            let cursor_y = area.y;

            self.cursor_position = Some((cursor_x, cursor_y));
        } else {
            if self.search_input.is_empty() {
                Paragraph::new("Press ? for help")
                    .centered()
                    .render(area, buf);
            } else {
                Paragraph::new(input).left_aligned().render(area, buf);
            }

            self.cursor_position = None;
        }
    }

    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_set(border::THICK)
            .border_style(Style::new().fg(Color::DarkGray));

        let entries = self.entry_list.get_filtered_entries();

        let mut entry_render_data: Vec<EntryRenderData> = entries
            .into_iter()
            .map(|x| EntryRenderData::from_entry(x, &self.search_input))
            .collect();

        // TODO: Move the shortcuts logic to a separate module

        // Collect all the next_chars for the entries, they should all be illegal shortcuts
        let illegal_shortcuts = entry_render_data
            .iter()
            .filter_map(|x| x.next_char)
            .collect::<HashSet<_>>();

        // Reset the shortcuts
        self.shortcuts.clear();

        for (i, data) in entry_render_data.iter_mut().enumerate() {
            if data.kind != &EntryKind::Directory || data.is_dynamic {
                // We only assign shortcuts to directories since you can't jump "into" files
                // Also, we don't assign shortcuts to dynamic entries (like "..")
                continue;
            }

            // Assign a shortcut to the entry
            for shortcut in PREFERRED_ENTRY_SHORTCUTS.iter() {
                if !self.shortcuts.contains_key(shortcut) && !illegal_shortcuts.contains(shortcut) {
                    data.shortcut = Some(*shortcut);
                    self.shortcuts.insert(*shortcut, i);
                    break;
                }
            }

            if self.shortcuts.len() + illegal_shortcuts.len() >= PREFERRED_ENTRY_SHORTCUTS.len() {
                // We've assigned all the possible shortcuts, we can iterating stop now
                break;
            }
        }

        let items: Vec<ListItem> = entry_render_data.into_iter().map(ListItem::from).collect();

        if items.is_empty() {
            let empty_results_text = if self.search_input.is_empty() {
                String::from("Nothing here but digital thumbleweeds.")
            } else {
                format!("No results found for '{query}'", query = self.search_input)
            };

            Paragraph::new(empty_results_text)
                .block(block)
                .render(area, buf);
        } else {
            // Create a List from all list items and highlight the currently selected one
            let list = List::new(items)
                .block(block)
                .highlight_style(Style::new().bg(Color::Gray).fg(Color::Black))
                .highlight_symbol(">")
                .highlight_spacing(HighlightSpacing::Always);

            // If no item is selected, preselect the first item
            if self.list_state.selected().is_none() {
                self.list_state.select_first();
            }

            // We need to disambiguate this trait method as both `Widget` and `StatefulWidget` share
            // the same method name `render`.
            StatefulWidget::render(list, area, buf, &mut self.list_state);
        }
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

        self.render_footer(footer_area, buf);
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
            current_directory: PathBuf::from("/home/user"),
            list_mode: ListMode::Directory,
            entry_list: EntryList {
                items: vec![
                    Entry {
                        path: PathBuf::from("/home/"),
                        kind: EntryKind::Directory,
                        name: "..".into(),
                    },
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
                ..Default::default()
            },
            ..Default::default()
        }
    }

    // TODO: Move this to a separate "entry" module
    #[test]
    fn parse_render_data() {
        let entry = Entry {
            name: "Cargo.toml".into(),
            kind: EntryKind::File {
                extension: Some("toml".into()),
            },
            path: PathBuf::from("/home/user/Cargo.toml"),
        };

        let fragments: EntryRenderData = EntryRenderData::from_entry(&entry, "car");

        // TODO: Implement Default for EntryRenderData
        assert_eq!(
            fragments,
            EntryRenderData {
                is_dynamic: false,
                prefix: "",
                search_hit: "Car",
                suffix: "go.toml",
                next_char: Some('g'),
                kind: &EntryKind::File {
                    extension: Some("toml".into())
                },
                shortcut: None,
            }
        );

        let fragments: EntryRenderData = EntryRenderData::from_entry(&entry, "toml");

        assert_eq!(
            fragments,
            EntryRenderData {
                is_dynamic: false,
                prefix: "Cargo.",
                search_hit: "toml",
                suffix: "",
                next_char: None,
                kind: &EntryKind::File {
                    extension: Some("toml".into())
                },
                shortcut: None,
            }
        );

        let fragments: EntryRenderData = EntryRenderData::from_entry(&entry, "argo");

        assert_eq!(
            fragments,
            EntryRenderData {
                is_dynamic: false,
                prefix: "C",
                search_hit: "argo",
                suffix: ".toml",
                next_char: Some('.'),
                kind: &EntryKind::File {
                    extension: Some("toml".into())
                },
                shortcut: None,
            }
        );
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
    fn renders_correctly_with_help_popup_after_key_event() {
        let mut app = create_test_app();
        app.handle_key_event(KeyCode::Char('?').into()).unwrap();

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .unwrap();

        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn renders_correctly_without_help_popup_after_key_event_toggle() {
        let mut app = create_test_app();
        app.show_help = true;
        app.handle_key_event(KeyCode::Char('?').into()).unwrap();

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .unwrap();

        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn renders_correctly_with_search_input_after_key_events() {
        let mut app = create_test_app();
        app.handle_key_event(KeyCode::Char('/').into()).unwrap();
        app.handle_key_event(KeyCode::Char('t').into()).unwrap();
        app.handle_key_event(KeyCode::Char('e').into()).unwrap();
        app.handle_key_event(KeyCode::Char('s').into()).unwrap();
        app.handle_key_event(KeyCode::Char('t').into()).unwrap();

        let mut terminal = Terminal::new(TestBackend::new(80, 9)).unwrap();

        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .unwrap();

        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn renders_correctly_with_search_input() {
        let mut app = create_test_app();
        app.input_mode = InputMode::Search;
        app.search_input.value = "test".into();
        app.search_input.index = 4;

        let mut terminal = Terminal::new(TestBackend::new(80, 9)).unwrap();

        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .unwrap();

        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn first_item_is_preselected_after_render() {
        let mut app = create_test_app();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 79, 10));

        assert_eq!(app.list_state.selected(), None);

        app.render(buffer.area, &mut buffer);

        assert_eq!(app.list_state.selected(), Some(0));
    }

    #[test]
    fn handle_key_event() {
        let mut app = create_test_app();

        // Make sure we have 3 items
        assert_eq!(app.entry_list.len(), 3);

        let _ = app.handle_key_event(KeyCode::Char('q').into());
        assert!(app.should_exit);

        let _ = app.handle_key_event(KeyCode::Esc.into());
        assert!(app.should_exit);

        let _ = app.handle_key_event(KeyCode::Char('j').into());
        assert_eq!(app.list_state.selected(), Some(0));

        let _ = app.handle_key_event(KeyCode::Down.into());
        assert_eq!(app.list_state.selected(), Some(1));

        // press down so that we can go back up more than once
        let _ = app.handle_key_event(KeyCode::Down.into());

        let _ = app.handle_key_event(KeyCode::Char('k').into());
        assert_eq!(app.list_state.selected(), Some(1));

        let _ = app.handle_key_event(KeyCode::Up.into());
        assert_eq!(app.list_state.selected(), Some(0));

        let _ = app.handle_key_event(KeyCode::Char('G').into());
        assert_eq!(app.list_state.selected(), Some(usize::MAX));

        let _ = app.handle_key_event(KeyCode::Char('g').into());
        assert_eq!(app.list_state.selected(), Some(0));

        let _ = app.handle_key_event(KeyCode::End.into());
        assert_eq!(app.list_state.selected(), Some(usize::MAX));

        let _ = app.handle_key_event(KeyCode::Home.into());
        assert_eq!(app.list_state.selected(), Some(0));

        let _ = app.handle_key_event(KeyCode::Char('d').into());
        assert_eq!(app.list_mode, ListMode::Directory);

        let _ = app.handle_key_event(KeyCode::Char('f').into());
        assert_eq!(app.list_mode, ListMode::Frecent);

        let _ = app.handle_key_event(KeyCode::Char('d').into());
        assert_eq!(app.list_mode, ListMode::Directory);

        let _ = app.handle_key_event(KeyCode::Char('?').into());
        assert!(app.show_help);

        let _ = app.handle_key_event(KeyCode::Char('/').into());
        assert_eq!(app.input_mode, InputMode::Search);

        let _ = app.handle_key_event(KeyCode::Esc.into());
        assert_eq!(app.input_mode, InputMode::Normal);
    }
}
