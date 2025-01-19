use std::{collections::HashMap, hash::Hash};

use crossterm::event::{KeyCode, KeyModifiers};

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct KeyCombo {
    pub key_code: KeyCode,
    pub modifiers: KeyModifiers,
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

    pub fn clear_entry_hotkeys(&mut self) {
        self.entry_hotkeys.clear();
        self.entry_hotkeys_count = 0;
    }

    pub fn get_hotkey_value(&self, context: C, key_combos: &[KeyCombo]) -> Option<&T> {
        // System hotkeys take priority
        if let Some(trie) = self.system_hotkeys.get(&context) {
            trie.get_value(key_combos)
        } else {
            self.entry_hotkeys.get_value(key_combos)
        }
    }

    pub fn get_hotkey_node(
        &self,
        context: C,
        key_combos: &[KeyCombo],
    ) -> Option<&HotkeysTrieNode<T>> {
        // System hotkeys take priority
        if let Some(trie) = self.system_hotkeys.get(&context) {
            trie.get_node(key_combos)
        } else {
            self.entry_hotkeys.get_node(key_combos)
        }
    }
}

impl<C, T> Default for HotkeysRegistry<C, T>
where
    C: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkeys_trie_works_correctly() {
        let mut trie = HotkeysTrie::new();
        trie.insert(
            &[
                KeyCombo {
                    key_code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyCombo {
                    key_code: KeyCode::Char('b'),
                    modifiers: KeyModifiers::NONE,
                },
            ],
            1,
        );

        trie.insert(
            &[
                KeyCombo {
                    key_code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyCombo {
                    key_code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::NONE,
                },
            ],
            2,
        );

        trie.insert(
            &[
                KeyCombo {
                    key_code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyCombo {
                    key_code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::NONE,
                },
            ],
            3,
        );

        trie.insert(
            &[
                KeyCombo {
                    key_code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::CONTROL,
                },
                KeyCombo {
                    key_code: KeyCode::Char('z'),
                    modifiers: KeyModifiers::CONTROL,
                },
            ],
            1,
        );

        assert_eq!(
            trie.get_value(&[
                KeyCombo {
                    key_code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyCombo {
                    key_code: KeyCode::Char('b'),
                    modifiers: KeyModifiers::NONE,
                },
            ]),
            Some(&1)
        );
        assert_eq!(
            trie.get_value(&[
                KeyCombo {
                    key_code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyCombo {
                    key_code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::NONE,
                },
            ]),
            Some(&2)
        );
        assert_eq!(
            trie.get_value(&[
                KeyCombo {
                    key_code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyCombo {
                    key_code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::NONE,
                },
            ]),
            Some(&3)
        );

        assert_eq!(
            trie.get_value(&[
                KeyCombo {
                    key_code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyCombo {
                    key_code: KeyCode::Char('d'),
                    modifiers: KeyModifiers::NONE,
                },
            ]),
            None
        );

        assert_eq!(
            trie.get_value(&[
                KeyCombo {
                    key_code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::CONTROL,
                },
                KeyCombo {
                    key_code: KeyCode::Char('z'),
                    modifiers: KeyModifiers::CONTROL,
                },
            ]),
            Some(&1)
        );
    }

    #[test]
    fn hotkeys_trie_clear_works_correctly() {
        let mut trie = HotkeysTrie::new();
        trie.insert(
            &[
                KeyCombo {
                    key_code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyCombo {
                    key_code: KeyCode::Char('b'),
                    modifiers: KeyModifiers::NONE,
                },
            ],
            1,
        );

        assert_eq!(
            trie.get_value(&[
                KeyCombo {
                    key_code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyCombo {
                    key_code: KeyCode::Char('b'),
                    modifiers: KeyModifiers::NONE,
                },
            ]),
            Some(&1)
        );

        trie.clear();

        assert_eq!(
            trie.get_value(&[
                KeyCombo {
                    key_code: KeyCode::Char('a'),
                    modifiers: KeyModifiers::NONE,
                },
                KeyCombo {
                    key_code: KeyCode::Char('b'),
                    modifiers: KeyModifiers::NONE,
                },
            ]),
            None
        );
    }
}
