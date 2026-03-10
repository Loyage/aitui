use std::collections::HashMap;
use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    // Normal mode
    Quit,
    EnterInsert,
    EnterInsertAfter,
    EnterInsertEnd,
    EnterInsertStart,
    EnterVisual,
    ScrollDown,
    ScrollUp,
    ScrollToTop,
    ScrollToBottom,
    CopyResponse,
    NewConversation,
    NextConversation,
    PrevConversation,
    Search,
    SwitchProvider,
    ToggleHelp,
    // Insert mode
    SendMessage,
    BackToNormal,
    DeleteCharBefore,
    DeleteCharAt,
    CursorLeft,
    CursorRight,
    CursorHome,
    CursorEnd,
    ClearInput,
    DeleteWord,
    OpenInEditor,
}

pub struct Keymap {
    pub normal: HashMap<KeyEvent, Action>,
    pub insert: HashMap<KeyEvent, Action>,
    pub visual: HashMap<KeyEvent, Action>,
}

/// Parse a key string like "Ctrl+c", "j", "F1", "Enter", "Esc" into a KeyEvent.
fn parse_key_string(s: &str) -> Option<KeyEvent> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (modifiers, key_part) = if let Some(rest) = s.strip_prefix("Ctrl+") {
        (KeyModifiers::CONTROL, rest)
    } else if let Some(rest) = s.strip_prefix("Alt+") {
        (KeyModifiers::ALT, rest)
    } else if let Some(rest) = s.strip_prefix("Shift+") {
        (KeyModifiers::SHIFT, rest)
    } else {
        (KeyModifiers::NONE, s)
    };

    let code = match key_part {
        "Enter" => KeyCode::Enter,
        "Esc" => KeyCode::Esc,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "Space" => KeyCode::Char(' '),
        k if k.starts_with('F') && k.len() > 1 => {
            let num: u8 = k[1..].parse().ok()?;
            KeyCode::F(num)
        }
        k if k.len() == 1 => {
            let c = k.chars().next().unwrap();
            KeyCode::Char(c)
        }
        _ => return None,
    };

    Some(KeyEvent::new(code, modifiers))
}

/// Format a KeyEvent back to a human-readable string.
fn format_key(key: &KeyEvent) -> String {
    let mut parts = Vec::new();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        parts.push("Ctrl".to_string());
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        parts.push("Alt".to_string());
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        parts.push("Shift".to_string());
    }

    let key_name = match key.code {
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::F(n) => format!("F{}", n),
        KeyCode::Char(' ') => "Space".to_string(),
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                c.to_uppercase().to_string()
            } else {
                c.to_string()
            }
        }
        _ => "?".to_string(),
    };
    parts.push(key_name);
    parts.join("+")
}

impl Keymap {
    fn key(s: &str) -> KeyEvent {
        parse_key_string(s).unwrap_or_else(|| panic!("invalid default key: {}", s))
    }

    pub fn default_keymap() -> Self {
        let mut normal = HashMap::new();
        let mut insert = HashMap::new();
        let mut visual = HashMap::new();

        // Normal mode defaults
        normal.insert(Self::key("q"), Action::Quit);
        normal.insert(Self::key("i"), Action::EnterInsert);
        normal.insert(Self::key("a"), Action::EnterInsertAfter);
        normal.insert(Self::key("A"), Action::EnterInsertEnd);
        normal.insert(Self::key("I"), Action::EnterInsertStart);
        normal.insert(Self::key("v"), Action::EnterVisual);
        normal.insert(Self::key("j"), Action::ScrollDown);
        normal.insert(Self::key("Down"), Action::ScrollDown);
        normal.insert(Self::key("k"), Action::ScrollUp);
        normal.insert(Self::key("Up"), Action::ScrollUp);
        normal.insert(Self::key("G"), Action::ScrollToBottom);
        normal.insert(Self::key("g"), Action::ScrollToTop);
        normal.insert(Self::key("y"), Action::CopyResponse);
        normal.insert(Self::key("n"), Action::NewConversation);
        normal.insert(Self::key("Ctrl+j"), Action::NextConversation);
        normal.insert(Self::key("Ctrl+k"), Action::PrevConversation);
        normal.insert(Self::key("/"), Action::Search);
        normal.insert(Self::key("Tab"), Action::SwitchProvider);
        normal.insert(Self::key("E"), Action::OpenInEditor);
        normal.insert(Self::key("F1"), Action::ToggleHelp);

        // Insert mode defaults
        insert.insert(Self::key("Enter"), Action::SendMessage);
        insert.insert(Self::key("Esc"), Action::BackToNormal);
        insert.insert(Self::key("Backspace"), Action::DeleteCharBefore);
        insert.insert(Self::key("Delete"), Action::DeleteCharAt);
        insert.insert(Self::key("Left"), Action::CursorLeft);
        insert.insert(Self::key("Right"), Action::CursorRight);
        insert.insert(Self::key("Home"), Action::CursorHome);
        insert.insert(Self::key("End"), Action::CursorEnd);
        insert.insert(Self::key("Ctrl+a"), Action::CursorHome);
        insert.insert(Self::key("Ctrl+e"), Action::CursorEnd);
        insert.insert(Self::key("Ctrl+u"), Action::ClearInput);
        insert.insert(Self::key("Ctrl+w"), Action::DeleteWord);

        // Visual mode defaults
        visual.insert(Self::key("Esc"), Action::BackToNormal);
        visual.insert(Self::key("y"), Action::CopyResponse);
        visual.insert(Self::key("j"), Action::ScrollDown);
        visual.insert(Self::key("Down"), Action::ScrollDown);
        visual.insert(Self::key("k"), Action::ScrollUp);
        visual.insert(Self::key("Up"), Action::ScrollUp);

        Keymap { normal, insert, visual }
    }

    /// Returns human-readable key strings bound to the given action in a mode map.
    pub fn keys_for_action(&self, mode: &str, action: Action) -> Vec<String> {
        let map = match mode {
            "normal" => &self.normal,
            "insert" => &self.insert,
            "visual" => &self.visual,
            _ => return Vec::new(),
        };
        let mut keys: Vec<String> = map
            .iter()
            .filter(|(_, a)| **a == action)
            .map(|(k, _)| format_key(k))
            .collect();
        keys.sort();
        keys
    }

    /// Load keymap: start with defaults, overlay user config if present.
    pub fn load() -> anyhow::Result<Self> {
        let mut keymap = Self::default_keymap();

        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let raw: RawKeybindings = toml::from_str(&content)?;
            keymap.apply_raw(&raw);
        }

        Ok(keymap)
    }

    fn config_path() -> PathBuf {
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".config")
            });
        base.join("aitui").join("keybindings.toml")
    }

    /// Apply user overrides from parsed TOML.
    /// For each action the user specifies, remove all existing bindings for that action
    /// in the mode, then add the new ones.
    fn apply_raw(&mut self, raw: &RawKeybindings) {
        if let Some(ref section) = raw.normal {
            Self::apply_section(&mut self.normal, section);
        }
        if let Some(ref section) = raw.insert {
            Self::apply_section(&mut self.insert, section);
        }
        if let Some(ref section) = raw.visual {
            Self::apply_section(&mut self.visual, section);
        }
    }

    fn apply_section(map: &mut HashMap<KeyEvent, Action>, section: &HashMap<String, KeyOrKeys>) {
        for (action_str, keys) in section {
            let action = match action_str.as_str() {
                "quit" => Action::Quit,
                "enter_insert" => Action::EnterInsert,
                "enter_insert_after" => Action::EnterInsertAfter,
                "enter_insert_end" => Action::EnterInsertEnd,
                "enter_insert_start" => Action::EnterInsertStart,
                "enter_visual" => Action::EnterVisual,
                "scroll_down" => Action::ScrollDown,
                "scroll_up" => Action::ScrollUp,
                "scroll_to_top" => Action::ScrollToTop,
                "scroll_to_bottom" => Action::ScrollToBottom,
                "copy_response" => Action::CopyResponse,
                "new_conversation" => Action::NewConversation,
                "next_conversation" => Action::NextConversation,
                "prev_conversation" => Action::PrevConversation,
                "search" => Action::Search,
                "switch_provider" => Action::SwitchProvider,
                "toggle_help" => Action::ToggleHelp,
                "send_message" => Action::SendMessage,
                "back_to_normal" => Action::BackToNormal,
                "delete_char_before" => Action::DeleteCharBefore,
                "delete_char_at" => Action::DeleteCharAt,
                "cursor_left" => Action::CursorLeft,
                "cursor_right" => Action::CursorRight,
                "cursor_home" => Action::CursorHome,
                "cursor_end" => Action::CursorEnd,
                "clear_input" => Action::ClearInput,
                "delete_word" => Action::DeleteWord,
                "open_in_editor" => Action::OpenInEditor,
                _ => continue, // skip unknown actions
            };

            // Remove old bindings for this action
            map.retain(|_, a| *a != action);

            // Add new bindings
            let key_strings = match keys {
                KeyOrKeys::Single(s) => vec![s.clone()],
                KeyOrKeys::Multiple(v) => v.clone(),
            };
            for ks in key_strings {
                if let Some(key_event) = parse_key_string(&ks) {
                    map.insert(key_event, action);
                }
            }
        }
    }
}

/// TOML structure for user keybindings file.
#[derive(Debug, Deserialize)]
struct RawKeybindings {
    normal: Option<HashMap<String, KeyOrKeys>>,
    insert: Option<HashMap<String, KeyOrKeys>>,
    visual: Option<HashMap<String, KeyOrKeys>>,
}

/// A value that can be either a single key string or an array of key strings.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum KeyOrKeys {
    Single(String),
    Multiple(Vec<String>),
}
