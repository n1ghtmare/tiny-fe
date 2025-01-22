use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use crossterm::event::{KeyCode, KeyModifiers};

use crate::{
    app::{Action, InputMode, ListMode},
    entry::{EntryKind, EntryRenderData},
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct KeyCombo {
    pub key_code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl From<KeyCode> for KeyCombo {
    fn from(key_code: KeyCode) -> Self {
        KeyCombo {
            key_code,
            modifiers: KeyModifiers::NONE,
        }
    }
}

impl From<char> for KeyCombo {
    fn from(c: char) -> Self {
        KeyCombo {
            key_code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
        }
    }
}

impl From<(char, KeyModifiers)> for KeyCombo {
    fn from((c, modifiers): (char, KeyModifiers)) -> Self {
        KeyCombo {
            key_code: KeyCode::Char(c),
            modifiers,
        }
    }
}

impl From<(KeyCode, KeyModifiers)> for KeyCombo {
    fn from((key_code, modifiers): (KeyCode, KeyModifiers)) -> Self {
        KeyCombo {
            key_code,
            modifiers,
        }
    }
}

#[derive(Debug)]
pub struct HotkeysTrieNode<T> {
    pub children: HashMap<KeyCombo, HotkeysTrieNode<T>>,
    pub value: Option<T>,
}

#[derive(Debug)]
struct HotkeysTrie<T> {
    root: HotkeysTrieNode<T>,
}

impl<T> HotkeysTrie<T> {
    pub fn new() -> Self {
        HotkeysTrie {
            root: HotkeysTrieNode {
                children: HashMap::new(),
                value: None,
            },
        }
    }

    pub fn insert(&mut self, key_combos: &[KeyCombo], value: T) {
        // we start at the root
        let mut current_node = &mut self.root;

        for &key_combo in key_combos {
            // if the node doesn't exist create it and move to it
            current_node = current_node
                .children
                .entry(key_combo)
                .or_insert(HotkeysTrieNode {
                    children: HashMap::new(),
                    value: None,
                });
        }

        // we've reached the end, we can now append the value
        current_node.value = Some(value);
    }

    pub fn get_value(&self, key_combos: &[KeyCombo]) -> Option<&T> {
        // we start at the root
        let node = self.get_node(key_combos)?;
        node.value.as_ref()
    }

    pub fn get_node(&self, key_combos: &[KeyCombo]) -> Option<&HotkeysTrieNode<T>> {
        // we start at the root
        let mut current_node = &self.root;

        for &key_combo in key_combos {
            if let Some(node) = current_node.children.get(&key_combo) {
                current_node = node;
            } else {
                return None;
            }
        }

        Some(current_node)
    }

    pub fn clear(&mut self) {
        self.root.children.clear();
        self.root.value = None;
    }
}

impl<T> Default for HotkeysTrie<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct HotkeysRegistry<C, T>
where
    C: Eq + Hash,
    T: std::fmt::Debug,
{
    /// System hotkeys are those needed for interracting with the app (for example: NextItem,
    /// PrevItem, FirstItem, LastItem etc.)
    system_hotkeys: HashMap<C, HotkeysTrie<T>>,
    system_hotkeys_count: usize,

    /// Entry hotkeys are those that are assigned on each entry so thtat the user can quickly jump
    /// into an entry without going to it
    entry_hotkeys: HotkeysTrie<T>,
    entry_hotkeys_count: usize,
}

impl<C, T> HotkeysRegistry<C, T>
where
    C: Eq + Hash,
    T: std::fmt::Debug,
{
    pub fn new() -> Self {
        HotkeysRegistry {
            system_hotkeys: HashMap::new(),
            system_hotkeys_count: 0,
            entry_hotkeys: HotkeysTrie::new(),
            entry_hotkeys_count: 0,
        }
    }

    pub fn register_system_hotkey(&mut self, context: C, key_combos: &[KeyCombo], value: T) {
        self.system_hotkeys_count += 1;
        let trie = self.system_hotkeys.entry(context).or_default();
        trie.insert(key_combos, value);
    }

    pub fn register_entry_hotkey(&mut self, key_combos: &[KeyCombo], value: T) {
        self.entry_hotkeys_count += 1;
        self.entry_hotkeys.insert(key_combos, value);
    }

    pub fn clear_entry_hotkeys(&mut self) {
        self.entry_hotkeys.clear();
        self.entry_hotkeys_count = 0;
    }

    pub fn get_hotkey_value(&self, context: C, key_combos: &[KeyCombo]) -> Option<&T> {
        if self.system_hotkeys_count == 0 && self.entry_hotkeys_count == 0 {
            return None;
        }

        // System hotkeys take priority
        self.system_hotkeys
            .get(&context)
            .and_then(|trie| trie.get_value(key_combos))
            .or_else(|| self.entry_hotkeys.get_value(key_combos))
    }

    pub fn get_hotkey_node(
        &self,
        context: C,
        key_combos: &[KeyCombo],
    ) -> Option<&HotkeysTrieNode<T>> {
        if self.system_hotkeys_count == 0 && self.entry_hotkeys_count == 0 {
            return None;
        }

        // System hotkeys take priority
        self.system_hotkeys
            .get(&context)
            .and_then(|trie| trie.get_node(key_combos))
            .or_else(|| self.entry_hotkeys.get_node(key_combos))
    }
}

impl<C, T> Default for HotkeysRegistry<C, T>
where
    C: Eq + Hash,
    T: std::fmt::Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

const fn key_combo_from_char(c: char) -> KeyCombo {
    KeyCombo {
        key_code: KeyCode::Char(c),
        modifiers: KeyModifiers::NONE,
    }
}

/// The preferred shortcuts for the entries in the list. These will be used to quickly jump to an
/// entry and will be chosed based on the order that they appear in this array, this way we can
/// prioritize ergonomics. In future versions, we might allow the user to customize these
/// shortcuts.
pub const PREFERRED_KEY_COMBOS_IN_ORDER: [KeyCombo; 31] = [
    key_combo_from_char('a'),
    key_combo_from_char('s'),
    key_combo_from_char('w'),
    key_combo_from_char('e'),
    key_combo_from_char('r'),
    key_combo_from_char('t'),
    key_combo_from_char('z'),
    key_combo_from_char('x'),
    key_combo_from_char('c'),
    key_combo_from_char('v'),
    key_combo_from_char('b'),
    key_combo_from_char('y'),
    key_combo_from_char('u'),
    key_combo_from_char('i'),
    key_combo_from_char('o'),
    key_combo_from_char('p'),
    key_combo_from_char('n'),
    key_combo_from_char('m'),
    key_combo_from_char(','),
    key_combo_from_char('1'),
    key_combo_from_char('2'),
    key_combo_from_char('3'),
    key_combo_from_char('4'),
    key_combo_from_char('5'),
    key_combo_from_char('6'),
    key_combo_from_char('7'),
    key_combo_from_char('8'),
    key_combo_from_char('9'),
    key_combo_from_char('0'),
    key_combo_from_char('-'),
    key_combo_from_char('='),
];

impl HotkeysRegistry<InputMode, Action> {
    pub fn new_with_default_system_hotkeys() -> Self {
        let mut registry = HotkeysRegistry::new();

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from('g'), KeyCombo::from('g')],
            Action::SelectFirst,
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from(KeyCode::Home)],
            Action::SelectFirst,
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from(('G', KeyModifiers::SHIFT))],
            Action::SelectLast,
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from(KeyCode::End)],
            Action::SelectLast,
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from('j')],
            Action::SelectNext,
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from(KeyCode::Down)],
            Action::SelectNext,
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from('k')],
            Action::SelectPrevious,
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from(KeyCode::Up)],
            Action::SelectPrevious,
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from(('d', KeyModifiers::CONTROL))],
            Action::SwitchToListMode(ListMode::Directory),
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from('l')],
            Action::ChangeDirectoryToSelectedEntry,
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from('h')],
            Action::ChangeDirectoryToParent,
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from(('f', KeyModifiers::CONTROL))],
            Action::SwitchToListMode(ListMode::Frecent),
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from('?')],
            Action::ToggleHelp,
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from('/')],
            Action::SwitchToInputMode(InputMode::Search),
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from(KeyCode::Esc)],
            Action::Exit,
        );

        registry.register_system_hotkey(InputMode::Normal, &[KeyCombo::from('q')], Action::Exit);

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from(KeyCode::Enter)],
            Action::ChangeDirectoryToSelectedEntry,
        );

        registry.register_system_hotkey(
            InputMode::Normal,
            &[KeyCombo::from('_')],
            Action::ResetSearchInput,
        );

        registry.register_system_hotkey(
            InputMode::Search,
            &[KeyCombo::from(KeyCode::Esc)],
            Action::ExitSearchMode,
        );

        registry.register_system_hotkey(
            InputMode::Search,
            &[KeyCombo::from(KeyCode::Enter)],
            Action::ExitSearchInput,
        );

        registry.register_system_hotkey(
            InputMode::Search,
            &[KeyCombo::from(KeyCode::Backspace)],
            Action::SearchInputBackspace,
        );

        registry
    }

    fn generate_sequence_permutations(
        key_combos: &[KeyCombo],
        length: usize,
    ) -> Vec<Vec<KeyCombo>> {
        let mut result = Vec::new();
        let mut current = vec![key_combos[0]; length];

        fn generate(
            key_combos: &[KeyCombo],
            current: &mut Vec<KeyCombo>,
            result: &mut Vec<Vec<KeyCombo>>,
            pos: usize,
        ) {
            if pos == current.len() {
                result.push(current.clone());
                return;
            }

            for &key_combo in key_combos {
                current[pos] = key_combo;
                generate(key_combos, current, result, pos + 1);
            }
        }

        generate(key_combos, &mut current, &mut result, 0);
        result
    }

    pub fn assign_hotkeys(
        &mut self,
        entry_render_data: &mut [EntryRenderData],
        preferred_key_combos_in_order: &[KeyCombo],
    ) {
        self.clear_entry_hotkeys();

        let mut directory_indexes: Vec<usize> = Vec::new();

        for (i, entry_render_datum) in entry_render_data.iter().enumerate() {
            if *entry_render_datum.kind == EntryKind::Directory {
                directory_indexes.push(i);
            }
        }

        let directory_indexes_count = directory_indexes.len();

        if directory_indexes_count == 0 {
            // We don't even need hotkeys, we don't a have any directories
            return;
        }

        // Collect all the next_chars for the entries, they should all be illegal hotkeys (so that
        // the user can continue typing if in search mode)
        let illegal_key_codes = entry_render_data
            .iter()
            .filter_map(|x| x.illegal_char_for_hotkey)
            .map(KeyCode::Char)
            .collect::<HashSet<_>>();

        let mut available_key_combos: Vec<KeyCombo> = Vec::new();

        for &key_combo in preferred_key_combos_in_order.iter() {
            if !illegal_key_codes.contains(&key_combo.key_code) {
                available_key_combos.push(key_combo);
            }
        }

        let available_key_codes_count = available_key_combos.len();
        if available_key_codes_count < 2 && directory_indexes_count > 1 {
            // We can't generate key sequences if we have a single key code and more than one
            // directory
            return;
        }

        let mut sequence_length = 1;

        while available_key_codes_count.pow(sequence_length) < directory_indexes_count {
            sequence_length += 1;
        }

        let permutations = Self::generate_sequence_permutations(
            available_key_combos.as_slice(),
            sequence_length as usize,
        );

        assert!(permutations.len() >= directory_indexes_count);

        let mut i = 0;
        while i < directory_indexes_count {
            // TODO: See if we can remove this clone
            let directory_index = directory_indexes[i];
            entry_render_data[directory_index].key_combo_sequence = Some(permutations[i].clone());
            self.register_entry_hotkey(
                permutations[i].as_slice(),
                Action::ChangeDirectoryToEntryWithIndex(directory_index),
            );
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::entry::Entry;

    use super::*;

    #[test]
    fn hotkeys_trie_works_correctly() {
        let mut trie = HotkeysTrie::new();
        trie.insert(&[KeyCombo::from('a'), KeyCombo::from('b')], 1);

        trie.insert(&[KeyCombo::from('a'), KeyCombo::from('c')], 2);

        trie.insert(&[KeyCombo::from('c'), KeyCombo::from('d')], 3);

        trie.insert(&[KeyCombo::from('a'), KeyCombo::from('z')], 1);

        assert_eq!(
            trie.get_value(&[KeyCombo::from('a'), KeyCombo::from('b'),]),
            Some(&1)
        );
        assert_eq!(
            trie.get_value(&[KeyCombo::from('a'), KeyCombo::from('c'),]),
            Some(&2)
        );
        assert_eq!(
            trie.get_value(&[KeyCombo::from('c'), KeyCombo::from('d'),]),
            Some(&3)
        );

        assert_eq!(
            trie.get_value(&[KeyCombo::from('a'), KeyCombo::from('d'),]),
            None
        );

        assert_eq!(
            trie.get_value(&[KeyCombo::from('a'), KeyCombo::from('z'),]),
            Some(&1)
        );
    }

    #[test]
    fn hotkeys_trie_clear_works_correctly() {
        let mut trie = HotkeysTrie::new();
        trie.insert(&[KeyCombo::from('a'), KeyCombo::from('b')], 1);

        assert_eq!(
            trie.get_value(&[KeyCombo::from('a'), KeyCombo::from('b'),]),
            Some(&1)
        );

        trie.clear();

        assert_eq!(
            trie.get_value(&[KeyCombo::from('a'), KeyCombo::from('b'),]),
            None
        );
    }

    #[test]
    fn generate_sequence_permutations_works_correctly() {
        let available_key_combos = &[
            KeyCombo::from('a'),
            KeyCombo::from('b'),
            KeyCombo::from('c'),
        ];

        let result: Vec<Vec<KeyCombo>> =
            HotkeysRegistry::generate_sequence_permutations(available_key_combos, 1);

        assert_eq!(result.len(), 3);
        assert_eq!(
            result[0],
            vec![KeyCombo {
                key_code: KeyCode::Char('a'),
                modifiers: KeyModifiers::NONE
            }]
        );
        assert_eq!(
            result[1],
            vec![KeyCombo {
                key_code: KeyCode::Char('b'),
                modifiers: KeyModifiers::NONE
            }]
        );
        assert_eq!(
            result[2],
            vec![KeyCombo {
                key_code: KeyCode::Char('c'),
                modifiers: KeyModifiers::NONE
            }]
        );

        let result: Vec<Vec<KeyCombo>> =
            HotkeysRegistry::generate_sequence_permutations(available_key_combos, 2);

        assert_eq!(result.len(), 9);

        let expected_characters = [
            ['a', 'a'],
            ['a', 'b'],
            ['a', 'c'],
            ['b', 'a'],
            ['b', 'b'],
            ['b', 'c'],
            ['c', 'a'],
            ['c', 'b'],
            ['c', 'c'],
        ];

        for (i, key_combos) in result.iter().enumerate() {
            assert_eq!(key_combos.len(), 2);
            assert_eq!(
                key_combos[0].key_code,
                KeyCode::Char(expected_characters[i][0])
            );
            assert_eq!(
                key_combos[1].key_code,
                KeyCode::Char(expected_characters[i][1])
            );
        }

        let result: Vec<Vec<KeyCombo>> =
            HotkeysRegistry::generate_sequence_permutations(available_key_combos, 3);

        assert_eq!(result.len(), 27);

        let expected_characters = [
            ['a', 'a', 'a'],
            ['a', 'a', 'b'],
            ['a', 'a', 'c'],
            ['a', 'b', 'a'],
            ['a', 'b', 'b'],
            ['a', 'b', 'c'],
            ['a', 'c', 'a'],
            ['a', 'c', 'b'],
            ['a', 'c', 'c'],
            ['b', 'a', 'a'],
            ['b', 'a', 'b'],
            ['b', 'a', 'c'],
            ['b', 'b', 'a'],
            ['b', 'b', 'b'],
            ['b', 'b', 'c'],
            ['b', 'c', 'a'],
            ['b', 'c', 'b'],
            ['b', 'c', 'c'],
            ['c', 'a', 'a'],
            ['c', 'a', 'b'],
            ['c', 'a', 'c'],
            ['c', 'b', 'a'],
            ['c', 'b', 'b'],
            ['c', 'b', 'c'],
            ['c', 'c', 'a'],
            ['c', 'c', 'b'],
            ['c', 'c', 'c'],
        ];

        for (i, key_combos) in result.iter().enumerate() {
            assert_eq!(key_combos.len(), 3);
            assert_eq!(
                key_combos[0].key_code,
                KeyCode::Char(expected_characters[i][0])
            );
            assert_eq!(
                key_combos[1].key_code,
                KeyCode::Char(expected_characters[i][1])
            );
            assert_eq!(
                key_combos[2].key_code,
                KeyCode::Char(expected_characters[i][2])
            );
        }

        let result: Vec<Vec<KeyCombo>> =
            HotkeysRegistry::generate_sequence_permutations(available_key_combos, 4);

        assert_eq!(result.len(), 81);
    }

    #[test]
    fn assign_hotkeys_works_correctly() {
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

        let mut hotkeys_registry = HotkeysRegistry::new();

        hotkeys_registry.assign_hotkeys(
            &mut entry_render_data,
            &[
                KeyCombo::from('b'),
                KeyCombo::from('a'),
                KeyCombo::from('c'),
                KeyCombo::from('y'),
            ],
        );

        assert_eq!(hotkeys_registry.entry_hotkeys_count, 5);

        assert_eq!(
            entry_render_data[0].key_combo_sequence,
            Some(vec![KeyCombo::from('b'), KeyCombo::from('b')])
        );

        assert_eq!(
            entry_render_data[1].key_combo_sequence,
            Some(vec![KeyCombo::from('b'), KeyCombo::from('a')])
        );

        assert_eq!(
            entry_render_data[2].key_combo_sequence,
            Some(vec![KeyCombo::from('b'), KeyCombo::from('y')])
        );

        assert_eq!(
            entry_render_data[3].key_combo_sequence,
            Some(vec![KeyCombo::from('a'), KeyCombo::from('b')])
        );

        assert_eq!(
            entry_render_data[4].key_combo_sequence,
            Some(vec![KeyCombo::from('a'), KeyCombo::from('a')])
        );

        assert_eq!(entry_render_data[5].key_combo_sequence, None);
    }
}
