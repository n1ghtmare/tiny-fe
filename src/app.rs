use anyhow::Ok;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{prelude::*, widgets::*, DefaultTerminal};
use symbols::border;

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

impl EntryList {
    #[cfg(test)]
    fn len(&self) -> usize {
        self.items.len()
    }
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
            .borders(Borders::ALL)
            .border_set(border::THICK);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render() {
        let mut app = App::default();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 79, 10));

        app.render(buffer.area, &mut buffer);

        let mut expected = Buffer::with_lines(vec![
            "                                    Tiny FE                                    ",
            "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓",
            "┃>Rewrite everything with Rust!                                               ┃",
            "┃ Rewrite all of your tui apps with Ratatui                                   ┃",
            "┃ Pet your cat                                                                ┃",
            "┃ Walk with your dog                                                          ┃",
            "┃ Pay the bills                                                               ┃",
            "┃ Refactor list example                                                       ┃",
            "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛",
            "            Use ↓↑ to move, g/G to go top/bottom, ENTER to select.             ",
        ]);

        // Apply BOLD to the entire first line
        expected.set_style(Rect::new(0, 0, 79, 1), Style::new().bold());

        // Clear BOLD at the beginning of the second line
        expected.set_style(Rect::new(0, 1, 79, 1), Style::new());

        // Apply DarkGray background to line 2 (index 2)
        expected.set_style(Rect::new(1, 2, 77, 1), Style::new().bg(Color::DarkGray));

        // Clear background color at the end of the highlighted line
        expected.set_style(Rect::new(78, 2, 1, 1), Style::new());

        // Reset style at the beginning of the third line
        expected.set_style(Rect::new(0, 3, 79, 1), Style::new());

        assert_eq!(buffer, expected);
    }

    #[test]
    fn first_item_is_preselected_after_render() {
        let mut app = App::default();
        let mut buffer = Buffer::empty(Rect::new(0, 0, 79, 10));

        assert_eq!(app.entry_list.state.selected(), None);

        app.render(buffer.area, &mut buffer);

        assert_eq!(app.entry_list.state.selected(), Some(0));
    }

    #[test]
    fn handle_key_event() {
        let mut app = App::default();

        // Make sure we have 6 items
        assert_eq!(app.entry_list.len(), 6);

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
