use std::{
    env, fmt,
    ops::Deref,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::Ok;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{prelude::*, widgets::*};
use symbols::border;

use crate::{
    entry::{EntryKind, EntryList, EntryRenderData},
    hotkeys::{HotkeysRegistry, KeyCombo, PREFERRED_KEY_COMBOS_IN_ORDER},
    index::DirectoryIndex,
};

/// Enum representing whether the system is currently showing a directory listing or paths from the
/// database.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ListMode {
    /// The system is currently showing a directory listing.
    #[default]
    Directory,
    // TODO: Implement this mode
    /// The system is currently showing paths from the database that have been accessed frequently
    /// and recently.
    #[allow(dead_code)]
    Frecent,
    // TODO: Implement this mode
    // /// The system is currently showing the user's bookmarks.
    // #[allow(dead_code)]
    // Bookmark,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum InputMode {
    Normal,
    Search,
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    // Traverse the list
    SelectNext,
    SelectPrevious,
    SelectFirst,
    SelectLast,
    ChangeDirectoryToSelectedEntry,
    ChangeDirectoryToParent,
    ChangeDirectoryToEntryWithIndex(usize),

    // Change the list mode
    SwitchToListMode(ListMode),

    // Change Input Mode
    SwitchToInputMode(InputMode),

    // Search Actions
    ResetSearchInput,
    ExitSearchInput,
    SearchInputBackspace,

    ToggleHelp,
    Exit,
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

    /// The buffer of user collected keycodes
    collected_key_combos: Vec<KeyCombo>,

    /// The last time a key was pressed, this is used to determine when to reset the key sequence
    last_key_press_time: Option<Instant>,

    /// The hotkeys registry, used to store system and entry hotkeys as well as register new ones
    /// and assign dynamically shortcuts to entries
    hotkeys_registry: HotkeysRegistry<InputMode, Action>,

    /// The path to the directory index file
    directory_index: DirectoryIndex,
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
            collected_key_combos: Vec::new(),
            last_key_press_time: None,
            hotkeys_registry: HotkeysRegistry::new_with_default_system_hotkeys(),
            directory_index: DirectoryIndex::default(),
        }
    }
}

impl App {
    /// This timeout is used to determine when a key sequence should be reset due to inactivity.
    const INACTIVITY_TIMEOUT: Duration = Duration::from_millis(500);

    /// Tries to create a new instance of the application in a given list mode.
    pub fn try_new(mode: ListMode, directory_index: DirectoryIndex) -> anyhow::Result<Self> {
        let path = env::current_dir()?;

        match mode {
            ListMode::Directory => {
                let mut app = App {
                    directory_index,
                    ..Default::default()
                };
                app.change_directory(path)?;
                Ok(app)
            }
            ListMode::Frecent => {
                let mut app = App {
                    directory_index,
                    list_mode: ListMode::Frecent,
                    ..Default::default()
                };
                app.change_list_mode(ListMode::Frecent)?;
                Ok(app)
            }
        }
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

        self.list_state = ListState::default();
        self.should_exit = false;
        self.list_mode = ListMode::Directory;
        self.entry_list = entry_list;
        self.current_directory = path.as_ref().to_path_buf();
        self.search_input.clear();

        Ok(())
    }

    pub fn change_to_frecent(&mut self) -> anyhow::Result<()> {
        let entries = self.directory_index.get_all_entries_ordered_by_rank();
        let entry_list = EntryList::try_from(entries)?;

        self.list_state = ListState::default();
        self.should_exit = false;
        self.list_mode = ListMode::Frecent;
        self.entry_list = entry_list;
        self.current_directory = env::current_dir()?;
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
            ListMode::Frecent => self.change_to_frecent(),
        }
    }

    /// Runs the application's main loop until the user quits.
    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> anyhow::Result<PathBuf> {
        while !self.should_exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(self.current_directory.clone())
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
                Span::styled("> gg/G or Home/End", Style::default().fg(Color::Yellow)),
                Span::raw(" - Go to top/bottom"),
            ]),
            Line::from(vec![
                Span::styled("> Ctrl + d/f", Style::default().fg(Color::Yellow)),
                Span::raw(" - Switch category (d)irectory or (f)recent"),
            ]),
            Line::from(vec![
                Span::styled("> Enter, l or →", Style::default().fg(Color::Yellow)),
                Span::raw(" - Go into directory"),
            ]),
            Line::from(vec![
                Span::styled("> h or ←", Style::default().fg(Color::Yellow)),
                Span::raw(" - Go up a directory"),
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
            Line::from(vec![
                Span::styled("> _", Style::default().fg(Color::Yellow)),
                Span::raw(" - Reset search"),
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
                self.handle_key_event(key_event, key_event.modifiers)?
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

    /// Handles a key event with the given key and modifiers, it will perform the appropriate
    /// action based on the current input mode and registered hotkeys.
    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        modifiers: KeyModifiers,
    ) -> anyhow::Result<()> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        match self.input_mode {
            InputMode::Search => self.handle_key_event_for_search_mode(key, modifiers),
            InputMode::Normal => self.handle_key_event_for_normal_mode(key, modifiers),
        }
    }

    fn handle_key_event_for_search_mode(
        &mut self,
        key: KeyEvent,
        modifiers: KeyModifiers,
    ) -> anyhow::Result<()> {
        // We check for inactivity here so that we can support key sequences
        if let Some(t) = self.last_key_press_time {
            if t.elapsed() >= Self::INACTIVITY_TIMEOUT {
                for key_combo in self.collected_key_combos.iter() {
                    if let KeyCode::Char(c) = key_combo.key_code {
                        self.search_input.push(c);
                    }
                }

                if let KeyCode::Char(c) = key.code {
                    self.search_input.push(c);
                }

                self.update_filtered_indices();
                self.collected_key_combos.clear();
                self.last_key_press_time = None;

                return Ok(());
            }
        }

        self.last_key_press_time = Some(Instant::now());

        let key_combo = KeyCombo::from((key.code, modifiers));
        self.collected_key_combos.push(key_combo);

        let maybe_node = self
            .hotkeys_registry
            .get_hotkey_node(InputMode::Search, &self.collected_key_combos);

        if let Some(node) = maybe_node {
            if let Some(action) = node.value {
                self.collected_key_combos.clear();
                self.last_key_press_time = None;

                match action {
                    Action::ChangeDirectoryToEntryWithIndex(index) => {
                        self.change_directory_to_entry_index(index)?;
                        self.input_mode = InputMode::Normal;
                        self.search_input.clear();
                    }
                    Action::SearchInputBackspace => {
                        // Remove character from the search input
                        if self.search_input.index > 0 {
                            self.search_input.pop();
                            self.update_filtered_indices();
                        } else {
                            // Exit search mode
                            self.input_mode = InputMode::Normal;
                        }
                    }
                    Action::SelectNext => {
                        self.list_state.select_next();
                    }
                    Action::SelectPrevious => {
                        self.list_state.select_previous();
                    }
                    Action::ExitSearchInput => {
                        self.input_mode = InputMode::Normal;
                    }
                    Action::ChangeDirectoryToSelectedEntry => {
                        if let Some(filtered_indices) = &self.entry_list.filtered_indices {
                            if !filtered_indices.is_empty() {
                                self.input_mode = InputMode::Normal;
                                self.search_input.clear();
                                let entry_index = self.list_state.selected().unwrap_or_default();
                                self.change_directory_to_entry_index(entry_index)?;
                            }
                        }
                    }
                    _ => {}
                }
            }

            return Ok(());
        }

        // We're at a point where the user has started a sequence, but the sequence didn't
        // match with anything, in which case we should unroll the sequence into the search
        // input
        if self.collected_key_combos.len() > 1 {
            for key_combo in self.collected_key_combos.iter() {
                if let KeyCode::Char(c) = key_combo.key_code {
                    self.search_input.push(c);
                }
            }
        } else if let KeyCode::Char(c) = key.code {
            self.search_input.push(c);
        }

        self.update_filtered_indices();
        self.collected_key_combos.clear();
        self.last_key_press_time = None;

        Ok(())
    }

    fn handle_key_event_for_normal_mode(
        &mut self,
        key: KeyEvent,
        modifiers: KeyModifiers,
    ) -> anyhow::Result<()> {
        // We check for inactivity here so that we can support key sequences
        if let Some(t) = self.last_key_press_time {
            if t.elapsed() >= Self::INACTIVITY_TIMEOUT {
                self.collected_key_combos.clear();
                self.last_key_press_time = None;
            }
        }

        self.last_key_press_time = Some(Instant::now());

        self.collected_key_combos
            .push(KeyCombo::from((key.code, modifiers)));

        let maybe_action = self
            .hotkeys_registry
            .get_hotkey_value(InputMode::Normal, &self.collected_key_combos);

        let Some(&action) = maybe_action else {
            return Ok(());
        };

        self.collected_key_combos.clear();
        self.last_key_press_time = None;

        match action {
            Action::SelectNext => {
                self.show_help = false;
                self.list_state.select_next();
            }
            Action::SelectPrevious => {
                self.show_help = false;
                self.list_state.select_previous();
            }
            Action::SelectFirst => {
                self.show_help = false;
                self.list_state.select_first();
            }
            Action::SelectLast => {
                self.show_help = false;
                self.list_state.select_last();
            }
            Action::SwitchToListMode(mode) => {
                self.show_help = false;
                self.change_list_mode(mode)?;
            }
            Action::ToggleHelp => {
                self.show_help = !self.show_help;
            }
            Action::SwitchToInputMode(mode) => {
                self.show_help = false;
                self.input_mode = mode;
                self.search_input.clear();
                self.update_filtered_indices();
            }
            Action::ResetSearchInput => {
                // clear the search input while in search mode
                self.search_input.clear();
                self.update_filtered_indices();
            }
            Action::ChangeDirectoryToSelectedEntry => {
                self.show_help = false;
                let entry_index = self.list_state.selected().unwrap_or_default();
                self.change_directory_to_entry_index(entry_index)?;
            }
            Action::ChangeDirectoryToParent => {
                self.show_help = false;

                if let Some(parent) = self.current_directory.clone().parent() {
                    self.change_directory(parent)?;
                }
            }
            Action::ChangeDirectoryToEntryWithIndex(index) => {
                self.show_help = false;
                self.change_directory_to_entry_index(index)?;
            }
            Action::Exit => {
                if self.show_help {
                    self.show_help = false;
                } else if self.search_input.is_empty() {
                    self.should_exit = true;
                } else {
                    self.search_input.clear();
                    self.update_filtered_indices();
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
        let app_version = env!("CARGO_PKG_VERSION");

        let line = Line::from(vec![
            Span::styled("Tiny DC", Style::default().bold()),
            Span::styled(format!(" v{}", app_version), Style::default().dark_gray()),
        ]);

        Paragraph::new(line).centered().render(area, buf);
    }

    fn render_selected_tab_title(&mut self, area: Rect, buf: &mut Buffer) {
        let line = Line::from(vec![
            Span::styled("|>", Style::default().dark_gray()),
            Span::raw(" "),
            Span::styled(self.get_sub_header_title(), Style::default().green()),
        ]);

        Paragraph::new(Text::from(vec![line])).render(area, buf);
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
                let select_index = match self.list_mode {
                    ListMode::Directory => 0,
                    ListMode::Frecent => 1,
                };

                let block = Block::default().borders(Borders::NONE);
                block.render(area, buf);

                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Length(6),
                            Constraint::Min(1),
                            Constraint::Length(16),
                        ]
                        .as_ref(),
                    )
                    .split(area);

                Text::from(Span::styled(
                    "Ctrl + ",
                    Style::default().fg(Color::DarkGray),
                ))
                .alignment(Alignment::Left)
                .render(chunks[0], buf);

                Tabs::new(["(d)irectory", "(f)recent"])
                    .highlight_style(Style::default().fg(Color::Green))
                    .select(select_index)
                    .render(chunks[1], buf);

                Paragraph::new("Press ? for help ").render(chunks[2], buf);
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

        if self.input_mode == InputMode::Normal
            || (self.input_mode == InputMode::Search && !self.search_input.is_empty())
        {
            self.hotkeys_registry
                .assign_hotkeys(&mut entry_render_data, &PREFERRED_KEY_COMBOS_IN_ORDER);
        } else {
            self.hotkeys_registry.clear_entry_hotkeys();
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
        let [header_area, selected_tab_title_area, main_area, footer_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let [list_area] = Layout::vertical([Constraint::Fill(1)]).areas(main_area);

        App::render_header(header_area, buf);

        self.render_footer(footer_area, buf);
        self.render_selected_tab_title(selected_tab_title_area, buf);
        self.render_list(list_area, buf);

        if self.show_help {
            self.render_help_popup(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::entry::Entry;

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
                        path: PathBuf::from("/home/user/.git/"),
                        kind: EntryKind::Directory,
                        name: ".git".into(),
                    },
                    Entry {
                        path: PathBuf::from("/home/user/dir1/"),
                        kind: EntryKind::Directory,
                        name: "dir1".into(),
                    },
                    Entry {
                        path: PathBuf::from("/home/user/.gitignore"),
                        kind: EntryKind::File { extension: None },
                        name: ".gitignore".into(),
                    },
                    Entry {
                        path: PathBuf::from("/home/user/Cargo.toml"),
                        kind: EntryKind::File {
                            extension: Some("toml".into()),
                        },
                        name: "Cargo.toml".into(),
                    },
                ],
                ..Default::default()
            },
            ..Default::default()
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
    fn renders_correctly_with_help_popup_after_key_event() {
        let mut app = create_test_app();
        app.handle_key_event(KeyCode::Char('?').into(), KeyModifiers::NONE)
            .unwrap();

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
        app.handle_key_event(KeyCode::Char('?').into(), KeyModifiers::NONE)
            .unwrap();

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .unwrap();

        assert_snapshot!(terminal.backend());
    }

    #[test]
    fn renders_correctly_with_search_input_after_key_events() {
        let mut app = create_test_app();
        app.handle_key_event(KeyCode::Char('/').into(), KeyModifiers::NONE)
            .unwrap();
        app.handle_key_event(KeyCode::Char('t').into(), KeyModifiers::NONE)
            .unwrap();
        app.handle_key_event(KeyCode::Char('e').into(), KeyModifiers::NONE)
            .unwrap();
        app.handle_key_event(KeyCode::Char('s').into(), KeyModifiers::NONE)
            .unwrap();
        app.handle_key_event(KeyCode::Char('t').into(), KeyModifiers::NONE)
            .unwrap();

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

        // Make sure we have 4 items
        assert_eq!(app.entry_list.len(), 4);

        let _ = app.handle_key_event(KeyCode::Char('q').into(), KeyModifiers::NONE);
        assert!(app.should_exit);

        let _ = app.handle_key_event(KeyCode::Esc.into(), KeyModifiers::NONE);
        assert!(app.should_exit);

        let _ = app.handle_key_event(KeyCode::Char('j').into(), KeyModifiers::NONE);
        assert_eq!(app.list_state.selected(), Some(0));

        let _ = app.handle_key_event(KeyCode::Down.into(), KeyModifiers::NONE);
        assert_eq!(app.list_state.selected(), Some(1));

        // press down so that we can go back up more than once
        let _ = app.handle_key_event(KeyCode::Down.into(), KeyModifiers::NONE);

        let _ = app.handle_key_event(KeyCode::Char('k').into(), KeyModifiers::NONE);
        assert_eq!(app.list_state.selected(), Some(1));

        let _ = app.handle_key_event(KeyCode::Up.into(), KeyModifiers::NONE);
        assert_eq!(app.list_state.selected(), Some(0));

        let _ = app.handle_key_event(KeyCode::Char('G').into(), KeyModifiers::SHIFT);
        assert_eq!(app.list_state.selected(), Some(usize::MAX));

        let _ = app.handle_key_event(KeyCode::Char('g').into(), KeyModifiers::NONE);
        let _ = app.handle_key_event(KeyCode::Char('g').into(), KeyModifiers::NONE);
        assert_eq!(app.list_state.selected(), Some(0));

        let _ = app.handle_key_event(KeyCode::End.into(), KeyModifiers::NONE);
        assert_eq!(app.list_state.selected(), Some(usize::MAX));

        let _ = app.handle_key_event(KeyCode::Home.into(), KeyModifiers::NONE);
        assert_eq!(app.list_state.selected(), Some(0));

        let _ = app.handle_key_event(KeyCode::Char('d').into(), KeyModifiers::CONTROL);
        assert_eq!(app.list_mode, ListMode::Directory);

        let _ = app.handle_key_event(KeyCode::Char('f').into(), KeyModifiers::CONTROL);
        assert_eq!(app.list_mode, ListMode::Frecent);

        let _ = app.handle_key_event(KeyCode::Char('d').into(), KeyModifiers::CONTROL);
        assert_eq!(app.list_mode, ListMode::Directory);

        let _ = app.handle_key_event(KeyCode::Char('?').into(), KeyModifiers::NONE);
        assert!(app.show_help);

        let _ = app.handle_key_event(KeyCode::Char('/').into(), KeyModifiers::NONE);
        assert_eq!(app.input_mode, InputMode::Search);

        let _ = app.handle_key_event(KeyCode::Esc.into(), KeyModifiers::NONE);
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn search_input_backspace() {
        let mut app = create_test_app();
        app.input_mode = InputMode::Search;
        app.search_input.value = "test".into();
        app.search_input.index = 4;

        let _ = app.handle_key_event(KeyCode::Backspace.into(), KeyModifiers::NONE);
        assert_eq!(app.search_input.value, "tes".to_string());
        assert_eq!(app.search_input.index, 3);

        let _ = app.handle_key_event(KeyCode::Backspace.into(), KeyModifiers::NONE);
        assert_eq!(app.search_input.value, "te".to_string());
        assert_eq!(app.search_input.index, 2);

        let _ = app.handle_key_event(KeyCode::Backspace.into(), KeyModifiers::NONE);
        assert_eq!(app.search_input.value, "t".to_string());
        assert_eq!(app.search_input.index, 1);

        let _ = app.handle_key_event(KeyCode::Backspace.into(), KeyModifiers::NONE);
        assert_eq!(app.search_input.value, "".to_string());
        assert_eq!(app.search_input.index, 0);

        let _ = app.handle_key_event(KeyCode::Backspace.into(), KeyModifiers::NONE);
        assert_eq!(app.search_input.value, "".to_string());
        assert_eq!(app.search_input.index, 0);
    }

    #[test]
    fn search_input_backspace_with_no_input() {
        let mut app = create_test_app();
        app.input_mode = InputMode::Search;
        app.search_input.value = "".into();
        app.search_input.index = 0;

        let _ = app.handle_key_event(KeyCode::Backspace.into(), KeyModifiers::NONE);
        assert_eq!(app.input_mode, InputMode::Normal);
    }

    #[test]
    fn search_works_correctly() {
        let mut app = create_test_app();
        app.input_mode = InputMode::Search;

        let _ = app.handle_key_event(KeyCode::Char('g').into(), KeyModifiers::NONE);
        let _ = app.handle_key_event(KeyCode::Char('i').into(), KeyModifiers::NONE);
        let _ = app.handle_key_event(KeyCode::Char('t').into(), KeyModifiers::NONE);

        assert_eq!(app.search_input.value, "git".to_string());
        assert_eq!(app.search_input.index, 3);

        app.update_filtered_indices();

        assert_eq!(app.entry_list.filtered_indices, Some(vec![0, 2]));
    }

    #[test]
    fn search_renders_correctly() {
        let mut app = create_test_app();
        app.input_mode = InputMode::Search;

        let _ = app.handle_key_event(KeyCode::Char('g').into(), KeyModifiers::NONE);
        let _ = app.handle_key_event(KeyCode::Char('i').into(), KeyModifiers::NONE);
        let _ = app.handle_key_event(KeyCode::Char('t').into(), KeyModifiers::NONE);

        let mut terminal = Terminal::new(TestBackend::new(80, 9)).unwrap();

        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .unwrap();

        assert_snapshot!(terminal.backend());
    }
}
