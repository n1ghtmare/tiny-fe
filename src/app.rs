use anyhow::Ok;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*, DefaultTerminal};

/// The main application struct, will hold the state of the application.
#[derive(Debug)]
pub struct App {
    /// A boolean used to signal if the app should exit
    should_exit: bool,

    // TODO: This should be the directory entries list
    entry_list: EntryList,
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_exit: false,
            // TODO: Default to the pwd
            entry_list: EntryList::from_iter([
                "Rewrite everything with Rust!",
                "Rewrite all of your tui apps with Ratatui",
                "Pet your cat",
                "Walk with your dog",
                "Pay the bills",
                "Refactor list example",
            ]),
        }
    }
}

// TODO: This should be the directory entries list and its state.
#[derive(Debug, Default)]
struct EntryList {
    items: Vec<EntryItem>,
    state: ListState,
}

impl FromIterator<&'static str> for EntryList {
    fn from_iter<I: IntoIterator<Item = &'static str>>(iter: I) -> Self {
        let items = iter.into_iter().map(EntryItem::new).collect();
        let state = ListState::default();
        Self { items, state }
    }
}

// TODO: This should be the entry list item
#[derive(Debug)]
struct EntryItem {
    title: String,
}

impl EntryItem {
    fn new(todo: &str) -> Self {
        Self {
            title: todo.to_string(),
        }
    }
}

impl From<&EntryItem> for ListItem<'_> {
    fn from(value: &EntryItem) -> Self {
        ListItem::new(value.title.to_owned())
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

    fn render_footer(area: Rect, buf: &mut Buffer) {
        Paragraph::new("Use ↓↑ to move, g/G to go top/bottom, ENTER to select.")
            .centered()
            .render(area, buf);
    }

    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new()
            // .title(Line::raw(" My List of items ").left_aligned())
            .borders(Borders::ALL);

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<ListItem> = self
            .entry_list
            .items
            .iter()
            .enumerate()
            .map(|(_, todo_item)| {
                // let color = alternate_colors(i);
                ListItem::from(todo_item)
            })
            .collect();

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
        let [header_area, main_area, footer_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let [list_area] = Layout::vertical([Constraint::Fill(1)]).areas(main_area);

        App::render_header(header_area, buf);
        App::render_footer(footer_area, buf);

        self.render_list(list_area, buf);
    }
}
