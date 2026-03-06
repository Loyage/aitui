use chrono::Utc;
use tokio::sync::mpsc;

use crate::api;
use crate::config::Config;
use crate::event::Event;
use crate::history::{
    save_conversation, ChatMessage, Conversation, Role,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal,
    Insert,
    Visual,
}

pub struct App {
    pub mode: Mode,
    pub input: String,
    pub cursor_pos: usize,
    pub conversation: Conversation,
    pub scroll_offset: usize,
    pub streaming: bool,
    pub config: Config,
    pub current_provider: usize,
    pub event_tx: mpsc::UnboundedSender<Event>,
    pub should_quit: bool,
    pub status_message: Option<String>,
    pub visual_start: Option<usize>,
    pub search_query: String,
    pub searching: bool,
}

impl App {
    pub fn new(config: Config, event_tx: mpsc::UnboundedSender<Event>) -> Self {
        let conversation =
            crate::history::load_latest_conversation().unwrap_or_else(Conversation::new);

        Self {
            mode: Mode::Normal,
            input: String::new(),
            cursor_pos: 0,
            conversation,
            scroll_offset: 0,
            streaming: false,
            config,
            current_provider: 0,
            event_tx,
            should_quit: false,
            status_message: None,
            visual_start: None,
            search_query: String::new(),
            searching: false,
        }
    }

    pub fn provider(&self) -> &crate::config::ProviderConfig {
        &self.config.providers[self.current_provider]
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
        let _ = save_conversation(&self.conversation);
        self.conversation = Conversation::new();
        self.scroll_offset = 0;
        self.status_message = Some("New conversation started".to_string());
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
        self.conversation.messages.push(user_msg);
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
        self.conversation.messages.push(ai_msg);

        // Auto-scroll to bottom
        self.scroll_offset = 0;

        api::send_chat_request(
            self.provider(),
            &self.conversation.messages[..self.conversation.messages.len() - 1],
            self.event_tx.clone(),
        );
    }

    pub fn on_api_token(&mut self, token: String) {
        if let Some(last) = self.conversation.messages.last_mut() {
            if last.role == Role::Assistant {
                last.content.push_str(&token);
            }
        }
    }

    pub fn on_api_done(&mut self) {
        self.streaming = false;
        self.conversation.updated_at = Utc::now();

        // Auto-set title from first user message
        if self.conversation.title == "New Chat" {
            if let Some(first_user) = self
                .conversation
                .messages
                .iter()
                .find(|m| m.role == Role::User)
            {
                self.conversation.title = first_user
                    .content
                    .chars()
                    .take(30)
                    .collect::<String>();
            }
        }

        let _ = save_conversation(&self.conversation);
    }

    pub fn on_api_error(&mut self, err: String) {
        self.streaming = false;
        self.status_message = Some(format!("API Error: {}", err));

        // Remove empty AI message if it exists
        if let Some(last) = self.conversation.messages.last() {
            if last.role == Role::Assistant && last.content.is_empty() {
                self.conversation.messages.pop();
            }
        }
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_to_top(&mut self) {
        // Will be clamped during rendering
        self.scroll_offset = usize::MAX / 2;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn copy_last_response(&mut self) {
        if let Some(last_ai) = self
            .conversation
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
        {
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    if clipboard.set_text(&last_ai.content).is_ok() {
                        self.status_message =
                            Some("Copied AI response to clipboard".to_string());
                    }
                }
                Err(_) => {
                    self.status_message =
                        Some("Failed to access clipboard".to_string());
                }
            }
        }
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
