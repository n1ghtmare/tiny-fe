use std::{
    collections::{HashMap, HashSet},
    fs::{DirEntry, ReadDir},
    path::PathBuf,
};

use ratatui::{prelude::*, widgets::*};

#[derive(Debug, PartialEq)]
pub enum EntryKind {
    File { extension: Option<String> },
    Directory,
}

#[derive(Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub kind: EntryKind,
    pub name: String,
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
pub struct EntryRenderData<'a> {
    /// A boolean indicating if the entry is dynamic (e.g., "..", which is not a "real" entry)
    pub is_dynamic: bool,
    prefix: &'a str,
    search_hit: &'a str,
    suffix: &'a str,
    /// The next character immediately after the search hit
    pub next_char: Option<char>,
    /// The kind of the entry, we need to keep track of this because we render directories
    /// differently than files
    pub kind: &'a EntryKind,
    /// The shortcut assigned to the entry, it's an optional character, some entries might not have
    /// a shortcut (files don't have shortcuts)
    pub shortcut: Option<char>,
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

        if search_query.as_ref().is_empty() {
            return EntryRenderData {
                is_dynamic: false,
                prefix: &entry.name,
                search_hit: "",
                suffix: "",
                next_char: entry.name.chars().next(),
                kind: &entry.kind,
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

#[derive(Debug, Default)]
pub struct EntryList {
    pub items: Vec<Entry>,
    pub filtered_indices: Option<Vec<usize>>,
}

impl EntryList {
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    pub fn get_filtered_entries(&self) -> Vec<&Entry> {
        match &self.filtered_indices {
            Some(indices) => indices.iter().map(|&i| &self.items[i]).collect(),
            None => self.items.iter().collect(),
        }
    }

    pub fn update_filtered_indices<T: AsRef<str>>(&mut self, value: T) {
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

// TODO: Need to switch this from chars to `KeyCode` in order to support more keys (F1, F2, etc.)
// TODO: Move this to a separate module (shortcuts.rs or hotkeys.rs) and also move the shortcuts
// handler in app.rs to that module (along with everything that is here)
// TODO: In future it will be nice if we support key combinations or sequences. So we need to
// account for that.

/// The preferred shortcuts for the entries in the list. These will be used to quickly jump to an
/// entry and will be chosed based on the order that they appear in this array, this way we can
/// prioritize ergonomics. In future versions, we might allow the user to customize these
/// shortcuts.
const PREFERRED_SHORTCUTS: [char; 33] = [
    'a', 's', 'w', 'e', 'r', 't', 'z', 'x', 'c', 'v', 'b', 'y', 'u', 'i', 'o', 'p', 'n', 'm', ',',
    '.', '/', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=',
];

#[derive(Debug)]
pub struct EntryShortcutRegistry {
    /// The map of shortcuts to entry indices
    shortcuts: HashMap<char, usize>,
    preferred_shortcuts: &'static [char],
}

impl Default for EntryShortcutRegistry {
    fn default() -> Self {
        EntryShortcutRegistry {
            shortcuts: HashMap::new(),
            preferred_shortcuts: &PREFERRED_SHORTCUTS,
        }
    }
}

impl EntryShortcutRegistry {
    pub fn with_custom_preferred_shortcuts(preferred_shortcuts: &'static [char]) -> Self {
        EntryShortcutRegistry {
            shortcuts: HashMap::new(),
            preferred_shortcuts,
        }
    }

    pub fn get(&self, shortcut: &char) -> Option<&usize> {
        self.shortcuts.get(shortcut)
    }

    pub fn assign_shortcuts(&mut self, entry_render_data: &mut [EntryRenderData]) {
        // Reset the shortcuts
        self.shortcuts.clear();

        // Collect all the next_chars for the entries, they should all be illegal shortcuts
        let illegal_shortcuts = entry_render_data
            .iter()
            .filter_map(|x| x.next_char)
            .collect::<HashSet<_>>();

        // TODO: Revisit this and see if: a) we can make it more efficient OR b) remove the early
        // break in the loop where this is used
        // Illegal shortcuts that are in the preferred shortcuts
        let illegal_shortcuts_in_preferred_count = illegal_shortcuts
            .iter()
            .filter(|x| self.preferred_shortcuts.contains(x))
            .count();

        for (i, data) in entry_render_data.iter_mut().enumerate() {
            if data.kind != &EntryKind::Directory || data.is_dynamic {
                // We only assign shortcuts to directories since you can't jump "into" files
                // Also, we don't assign shortcuts to dynamic entries (like "..")
                continue;
            }

            // Assign a shortcut to the entry
            for shortcut in self.preferred_shortcuts.iter() {
                if !self.shortcuts.contains_key(shortcut) && !illegal_shortcuts.contains(shortcut) {
                    data.shortcut = Some(*shortcut);
                    self.shortcuts.insert(*shortcut, i);
                    break;
                }
            }

            if self.shortcuts.len() + illegal_shortcuts_in_preferred_count
                >= self.preferred_shortcuts.len()
            {
                // We've assigned all the possible shortcuts, we can iterating stop now
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod entry_shortcut_registry {
        use super::*;

        #[test]
        pub fn assign_works_correctly_with_defaults() {
            let entries = [
                Entry {
                    name: "s-dir1".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/s-dir/"),
                },
                Entry {
                    name: "d-dir2".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/d-dir/"),
                },
                Entry {
                    name: "w-dir3".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/w-dir/"),
                },
                Entry {
                    name: "e-dir4".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/e-dir/"),
                },
                Entry {
                    name: "r-dir5".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/Cargo.toml"),
                },
                Entry {
                    name: "Cargo.toml".into(),
                    kind: EntryKind::File {
                        extension: Some("toml".into()),
                    },
                    path: PathBuf::from("/home/user/Cargo.toml"),
                },
            ];

            let mut entry_render_data: Vec<EntryRenderData> = entries
                .iter()
                .map(|entry| EntryRenderData::from_entry(entry, ""))
                .collect();

            let mut entry_shortcut_registry = EntryShortcutRegistry::default();

            entry_shortcut_registry.assign_shortcuts(&mut entry_render_data);

            assert_eq!(
                entry_render_data,
                vec![
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "s-dir1",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('s'),
                        kind: &EntryKind::Directory,
                        shortcut: Some('a'),
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "d-dir2",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('d'),
                        kind: &EntryKind::Directory,
                        shortcut: Some('t'),
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "w-dir3",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('w'),
                        kind: &EntryKind::Directory,
                        shortcut: Some('z'),
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "e-dir4",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('e'),
                        kind: &EntryKind::Directory,
                        shortcut: Some('x'),
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "r-dir5",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('r'),
                        kind: &EntryKind::Directory,
                        shortcut: Some('c'),
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "Cargo.toml",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('C'),
                        kind: &EntryKind::File {
                            extension: Some("toml".into())
                        },
                        shortcut: None,
                    },
                ]
            );
        }

        #[test]
        fn assign_works_correctly_with_custom_preferred_shortcuts_single() {
            let entries = [
                Entry {
                    name: "a-dir1".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/a-dir/"),
                },
                Entry {
                    name: "d-dir2".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/d-dir/"),
                },
                Entry {
                    name: "w-dir3".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/w-dir/"),
                },
                Entry {
                    name: "e-dir4".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/e-dir/"),
                },
                Entry {
                    name: "r-dir5".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/Cargo.toml"),
                },
                Entry {
                    name: "Cargo.toml".into(),
                    kind: EntryKind::File {
                        extension: Some("toml".into()),
                    },
                    path: PathBuf::from("/home/user/Cargo.toml"),
                },
            ];

            let mut entry_render_data: Vec<EntryRenderData> = entries
                .iter()
                .map(|entry| EntryRenderData::from_entry(entry, ""))
                .collect();

            let mut entry_shortcut_registry =
                EntryShortcutRegistry::with_custom_preferred_shortcuts(&['a', 't']);

            entry_shortcut_registry.assign_shortcuts(&mut entry_render_data);

            assert_eq!(
                entry_render_data,
                vec![
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "a-dir1",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('a'),
                        kind: &EntryKind::Directory,
                        shortcut: Some('t'),
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "d-dir2",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('d'),
                        kind: &EntryKind::Directory,
                        shortcut: None,
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "w-dir3",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('w'),
                        kind: &EntryKind::Directory,
                        shortcut: None,
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "e-dir4",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('e'),
                        kind: &EntryKind::Directory,
                        shortcut: None,
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "r-dir5",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('r'),
                        kind: &EntryKind::Directory,
                        shortcut: None,
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "Cargo.toml",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('C'),
                        kind: &EntryKind::File {
                            extension: Some("toml".into())
                        },
                        shortcut: None,
                    },
                ]
            );
        }

        #[test]
        fn assign_works_correctly_with_custom_preferred_shortcuts_max() {
            let entries = [
                Entry {
                    name: "s-dir1".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/a-dir/"),
                },
                Entry {
                    name: "d-dir2".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/d-dir/"),
                },
                Entry {
                    name: "w-dir3".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/w-dir/"),
                },
                Entry {
                    name: "e-dir4".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/e-dir/"),
                },
                Entry {
                    name: "r-dir5".into(),
                    kind: EntryKind::Directory,
                    path: PathBuf::from("/home/user/Cargo.toml"),
                },
                Entry {
                    name: "Cargo.toml".into(),
                    kind: EntryKind::File {
                        extension: Some("toml".into()),
                    },
                    path: PathBuf::from("/home/user/Cargo.toml"),
                },
            ];

            let mut entry_render_data: Vec<EntryRenderData> = entries
                .iter()
                .map(|entry| EntryRenderData::from_entry(entry, ""))
                .collect();

            let mut entry_shortcut_registry =
                EntryShortcutRegistry::with_custom_preferred_shortcuts(&['a', 't']);

            entry_shortcut_registry.assign_shortcuts(&mut entry_render_data);

            assert_eq!(
                entry_render_data,
                vec![
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "s-dir1",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('s'),
                        kind: &EntryKind::Directory,
                        shortcut: Some('a'),
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "d-dir2",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('d'),
                        kind: &EntryKind::Directory,
                        shortcut: Some('t'),
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "w-dir3",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('w'),
                        kind: &EntryKind::Directory,
                        shortcut: None,
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "e-dir4",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('e'),
                        kind: &EntryKind::Directory,
                        shortcut: None,
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "r-dir5",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('r'),
                        kind: &EntryKind::Directory,
                        shortcut: None,
                    },
                    EntryRenderData {
                        is_dynamic: false,
                        prefix: "Cargo.toml",
                        search_hit: "",
                        suffix: "",
                        next_char: Some('C'),
                        kind: &EntryKind::File {
                            extension: Some("toml".into())
                        },
                        shortcut: None,
                    },
                ]
            );
        }
    }

    mod entry_render_data {
        use super::*;

        #[test]
        fn entry_render_data_from_entry_works_correctly_with_search_query() {
            let entry = Entry {
                name: "Cargo.toml".into(),
                kind: EntryKind::File {
                    extension: Some("toml".into()),
                },
                path: PathBuf::from("/home/user/Cargo.toml"),
            };

            let entry_render_data: EntryRenderData = EntryRenderData::from_entry(&entry, "car");

            assert_eq!(
                entry_render_data,
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

            let entry_render_data: EntryRenderData = EntryRenderData::from_entry(&entry, "toml");

            assert_eq!(
                entry_render_data,
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

            let entry_render_data: EntryRenderData = EntryRenderData::from_entry(&entry, "argo");

            assert_eq!(
                entry_render_data,
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

            let entry_render_data: EntryRenderData = EntryRenderData::from_entry(&entry, "");

            assert_eq!(
                entry_render_data,
                EntryRenderData {
                    is_dynamic: false,
                    prefix: "Cargo.toml",
                    search_hit: "",
                    suffix: "",
                    next_char: Some('C'),
                    kind: &EntryKind::File {
                        extension: Some("toml".into())
                    },
                    shortcut: None,
                }
            );
        }
    }
}
