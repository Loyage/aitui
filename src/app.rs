use std::io::Write;

use chrono::Utc;
use tokio::sync::mpsc;

use crate::api;
use crate::config::Config;
use crate::event::Event;
use crate::history::{
    save_conversation, ChatMessage, Conversation, Role,
};
use crate::keymap::Keymap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Browse, // Focusing on conversation history
    Normal, // Focusing on input box (Vim normal)
    Insert, // Focusing on input box (Vim insert)
    Select, // Focusing on sidebar/conversation list
    Visual, // Keep for viewing long messages
    Setup,  // Initial configuration
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SetupStep {
    Name,
    ApiKey,
    BaseUrl,
    Model,
    Testing,
}

pub struct App {
    pub mode: Mode,
    pub input: String,
    pub cursor_pos: usize,
    pub conversations: Vec<Conversation>,
    pub active_conv_index: usize,
    pub selected_message: Option<usize>,
    pub streaming: bool,
    pub config: Config,
    pub keymap: Keymap,
    pub current_provider: usize,
    pub event_tx: mpsc::UnboundedSender<Event>,
    pub should_quit: bool,
    pub status_message: Option<String>,
    pub visual_start: Option<usize>,
    pub search_query: String,
    pub searching: bool,
    pub show_help: bool,
    pub help_scroll: usize,
    pub help_searching: bool,
    pub help_search_query: String,

    // Setup mode state
    pub setup_step: SetupStep,
    pub setup_provider: crate::config::ProviderConfig,
    pub setup_provider_index: usize,

    pub clipboard: Option<arboard::Clipboard>,

    // Sidebar state
    pub sidebar_expanded: bool,
    pub last_key: Option<crossterm::event::KeyEvent>,
}

impl App {
    pub fn new(config: Config, keymap: Keymap, event_tx: mpsc::UnboundedSender<Event>) -> Self {
        let mut conversations =
            crate::history::load_all_conversations().unwrap_or_default();
        if conversations.is_empty() {
            conversations.push(Conversation::new());
        }

        let selected_message = if !conversations.is_empty() {
             Self::last_non_system_index(&conversations[0].messages)
        } else {
            None
        };

        let mode = if config.providers.is_empty() {
            Mode::Setup
        } else {
            Mode::Browse
        };

        let clipboard = arboard::Clipboard::new().ok();

        Self {
            mode,
            input: String::new(),
            cursor_pos: 0,
            selected_message,
            conversations,
            active_conv_index: 0,
            streaming: false,
            config,
            keymap,
            current_provider: 0,
            event_tx,
            should_quit: false,
            status_message: None,
            visual_start: None,
            search_query: String::new(),
            searching: false,
            show_help: false,
            help_scroll: 0,
            help_searching: false,
            help_search_query: String::new(),
            setup_step: SetupStep::Name,
            setup_provider: crate::config::ProviderConfig::empty(),
            setup_provider_index: 0,
            clipboard,
            sidebar_expanded: false,
            last_key: None,
        }
    }

    pub fn conversation(&self) -> &Conversation {
        &self.conversations[self.active_conv_index]
    }

    pub fn conversation_mut(&mut self) -> &mut Conversation {
        &mut self.conversations[self.active_conv_index]
    }

    pub fn provider(&self) -> &crate::config::ProviderConfig {
        if self.config.providers.is_empty() {
             &self.setup_provider
        } else {
             &self.config.providers[self.current_provider]
        }
    }

    pub fn complete_setup(&mut self) {
        self.config.providers.push(self.setup_provider.clone());
        let _ = self.config.save();
        self.mode = Mode::Normal;
        self.status_message = Some("Configuration saved!".to_string());
    }

    pub fn switch_provider(&mut self) {
        if self.streaming {
            return;
        }
        self.current_provider = (self.current_provider + 1) % self.config.providers.len();
        let name = self.provider().name.clone();
        self.status_message = Some(format!("Switched to: {} ({})", name, self.provider().model));
    }

    pub fn new_conversation(&mut self) {
        // Save current before starting new
        let _ = save_conversation(self.conversation());
        let new_conv = Conversation::new();
        self.conversations.insert(0, new_conv);
        self.active_conv_index = 0;
        self.selected_message = None;
        self.status_message = Some("New conversation started".to_string());
    }

    pub fn select_next_conversation(&mut self) {
        if self.active_conv_index + 1 < self.conversations.len() {
            self.active_conv_index += 1;
            self.selected_message = Self::last_non_system_index(&self.conversation().messages);
        }
    }

    pub fn select_prev_conversation(&mut self) {
        if self.active_conv_index > 0 {
            self.active_conv_index -= 1;
            self.selected_message = Self::last_non_system_index(&self.conversation().messages);
        }
    }

    pub fn send_message(&mut self) {
        let content = self.input.trim().to_string();
        if content.is_empty() || self.streaming {
            return;
        }

        let user_msg = ChatMessage {
            role: Role::User,
            content,
            timestamp: Utc::now(),
        };
        self.conversation_mut().messages.push(user_msg);
        self.input.clear();
        self.cursor_pos = 0;

        // Start streaming response
        self.streaming = true;

        // Add placeholder for AI response
        let ai_msg = ChatMessage {
            role: Role::Assistant,
            content: String::new(),
            timestamp: Utc::now(),
        };
        self.conversation_mut().messages.push(ai_msg);

        // Select latest message
        self.selected_message = Self::last_non_system_index(&self.conversation().messages);

        api::send_chat_request(
            self.provider(),
            &self.conversation().messages[..self.conversation().messages.len() - 1],
            self.event_tx.clone(),
        );
    }

    pub fn on_api_token(&mut self, token: String) {
        if let Some(last) = self.conversation_mut().messages.last_mut() {
            if last.role == Role::Assistant {
                last.content.push_str(&token);
            }
        }
    }

    pub fn on_api_done(&mut self) {
        if self.mode == Mode::Setup {
            self.complete_setup();
            return;
        }

        self.streaming = false;
        self.conversation_mut().updated_at = Utc::now();
        self.selected_message = Self::last_non_system_index(&self.conversation().messages);

        // Auto-set title from first user message
        if self.conversation().title == "New Chat" {
            if let Some(first_user) = self
                .conversation()
                .messages
                .iter()
                .find(|m| m.role == Role::User)
            {
                self.conversation_mut().title = first_user
                    .content
                    .chars()
                    .take(30)
                    .collect::<String>();
            }
        }

        let _ = save_conversation(self.conversation());
    }

    pub fn on_api_error(&mut self, err: String) {
        if self.mode == Mode::Setup {
            self.status_message = Some(format!("Test Failed: {}", err));
            self.setup_step = SetupStep::Model; // Back to model selection to allow retry
            self.input = self.setup_provider.model.clone();
            self.cursor_pos = self.input.len();
            return;
        }

        self.streaming = false;
        self.status_message = Some(format!("API Error: {}", err));

        // Remove empty AI message if it exists
        if let Some(last) = self.conversation().messages.last() {
            if last.role == Role::Assistant && last.content.is_empty() {
                self.conversation_mut().messages.pop();
            }
        }
    }

    /// Returns the indices of non-System messages.
    fn non_system_indices(messages: &[ChatMessage]) -> Vec<usize> {
        messages
            .iter()
            .enumerate()
            .filter(|(_, m)| m.role != Role::System)
            .map(|(i, _)| i)
            .collect()
    }

    fn last_non_system_index(messages: &[ChatMessage]) -> Option<usize> {
        messages
            .iter()
            .rposition(|m| m.role != Role::System)
    }

    pub fn select_next_message(&mut self) {
        let indices = Self::non_system_indices(&self.conversation().messages);
        if indices.is_empty() {
            return;
        }
        match self.selected_message {
            Some(cur) => {
                if let Some(pos) = indices.iter().position(|&i| i == cur) {
                    if pos + 1 < indices.len() {
                        self.selected_message = Some(indices[pos + 1]);
                    }
                } else {
                    self.selected_message = Some(*indices.last().unwrap());
                }
            }
            None => {
                self.selected_message = Some(*indices.last().unwrap());
            }
        }
    }

    pub fn select_prev_message(&mut self) {
        let indices = Self::non_system_indices(&self.conversation().messages);
        if indices.is_empty() {
            return;
        }
        match self.selected_message {
            Some(cur) => {
                if let Some(pos) = indices.iter().position(|&i| i == cur) {
                    if pos > 0 {
                        self.selected_message = Some(indices[pos - 1]);
                    }
                } else {
                    self.selected_message = Some(indices[0]);
                }
            }
            None => {
                self.selected_message = Some(indices[0]);
            }
        }
    }

    pub fn select_first_message(&mut self) {
        let indices = Self::non_system_indices(&self.conversation().messages);
        if let Some(&first) = indices.first() {
            self.selected_message = Some(first);
        }
    }

    pub fn select_last_message(&mut self) {
        self.selected_message = Self::last_non_system_index(&self.conversation().messages);
    }

    pub fn copy_selected_message(&mut self) {
        let idx = match self.selected_message {
            Some(i) => i,
            None => {
                self.status_message = Some("No message selected to copy".to_string());
                return;
            }
        };
        let (content, role) = if let Some(msg) = self.conversation().messages.get(idx) {
            (msg.content.clone(), msg.role.clone())
        } else {
            return;
        };

        if let Some(ref mut clipboard) = self.clipboard {
            match clipboard.set_text(&content) {
                Ok(_) => {
                    let role_label = match role {
                        Role::User => "user",
                        Role::Assistant => "AI",
                        Role::System => "system",
                    };
                    self.status_message =
                        Some(format!("Copied {} message to clipboard", role_label));
                }
                Err(e) => {
                    self.status_message = Some(format!("Clipboard Error: {}", e));
                }
            }
        } else {
            self.status_message =
                Some("Clipboard not initialized (check dependencies like xclip/xsel)".to_string());
        }
    }

    pub fn open_selected_in_editor(&mut self) {
        let idx = match self.selected_message {
            Some(i) => i,
            None => return,
        };
        let msg = match self.conversation().messages.get(idx) {
            Some(m) => m,
            None => return,
        };

        // Write content to temp file
        let tmp_path = std::env::temp_dir().join(format!(
            "aitui_view_{}.md",
            std::process::id()
        ));
        let mut file = match std::fs::File::create(&tmp_path) {
            Ok(f) => f,
            Err(e) => {
                self.status_message = Some(format!("Failed to create temp file: {}", e));
                return;
            }
        };
        if let Err(e) = file.write_all(msg.content.as_bytes()) {
            self.status_message = Some(format!("Failed to write temp file: {}", e));
            return;
        }

        // Send event to main loop to handle terminal suspend/resume
        let _ = self
            .event_tx
            .send(Event::OpenEditor(tmp_path.to_string_lossy().to_string()));
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn delete_char_before_cursor(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.cursor_pos -= prev;
            self.input.remove(self.cursor_pos);
        }
    }

    pub fn delete_char_at_cursor(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.input.remove(self.cursor_pos);
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.cursor_pos -= prev;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            let next = self.input[self.cursor_pos..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.cursor_pos += next;
        }
    }

    pub fn move_cursor_to_start(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_cursor_to_end(&mut self) {
        self.cursor_pos = self.input.len();
    }
}
