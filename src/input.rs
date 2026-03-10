use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Mode, SetupStep};
use crate::keymap::Action;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Clear status message on any key press, but keep it in setup mode until next step
    if app.mode != Mode::Setup {
        app.status_message = None;
    }

    // Ctrl+C always quits (hardcoded, not rebindable)
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    // Sequential key combinations (e.g., Space + e)
    if app.mode != Mode::Insert && app.mode != Mode::Setup {
        if let Some(prev) = app.last_key {
            if prev.code == KeyCode::Char(' ') && key.code == KeyCode::Char('e') {
                app.sidebar_expanded = !app.sidebar_expanded;
                app.mode = if app.sidebar_expanded { Mode::Select } else { Mode::Browse };
                app.last_key = None;
                return;
            }
        }
        if key.code == KeyCode::Char(' ') {
            app.last_key = Some(key);
            return;
        } else {
            app.last_key = None;
        }
    }

    // When help is shown, handle help-specific keys
    if app.show_help {
        handle_help(app, key);
        return;
    }

    match app.mode {
        Mode::Browse => handle_browse(app, key),
        Mode::Normal => handle_normal(app, key),
        Mode::Insert => handle_insert(app, key),
        Mode::Select => handle_select(app, key),
        Mode::Visual => handle_visual(app, key),
        Mode::Setup => handle_setup(app, key),
    }
}

fn handle_select(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.select_next_conversation();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.select_prev_conversation();
        }
        KeyCode::Enter | KeyCode::Esc => {
            app.mode = Mode::Browse;
            app.sidebar_expanded = false;
        }
        _ => {}
    }
}

fn handle_browse(app: &mut App, key: KeyEvent) {
    if let Some(&action) = app.keymap.normal.get(&key) {
        match action {
            Action::Quit => app.should_quit = true,
            Action::EnterInsert => {
                app.mode = Mode::Insert;
                app.move_cursor_to_end();
            }
            Action::ScrollDown => app.select_next_message(),
            Action::ScrollUp => app.select_prev_message(),
            Action::ScrollToBottom => app.select_last_message(),
            Action::ScrollToTop => app.select_first_message(),
            Action::CopyResponse => app.copy_selected_message(),
            Action::OpenInEditor => app.open_selected_in_editor(),
            Action::NewConversation => app.new_conversation(),
            Action::SwitchProvider => app.switch_provider(),
            Action::ToggleHelp => { /* handled in handle_key */ }
            _ => {}
        }
    } else {
        match key.code {
            KeyCode::Enter => app.mode = Mode::Normal,
            KeyCode::Char('i') => app.mode = Mode::Insert,
            KeyCode::Char('j') | KeyCode::Down => app.select_next_message(),
            KeyCode::Char('k') | KeyCode::Up => app.select_prev_message(),
            _ => {}
        }
    }
}

fn handle_setup(app: &mut App, key: KeyEvent) {
    if app.setup_step == SetupStep::Testing {
        // While testing, only allow quitting with Ctrl+C (already handled)
        return;
    }

    match key.code {
        KeyCode::Char('j') | KeyCode::Down if app.setup_step == SetupStep::Name => {
            app.setup_provider_index = (app.setup_provider_index + 1) % crate::config::PRESET_PROVIDERS.len();
        }
        KeyCode::Char('k') | KeyCode::Up if app.setup_step == SetupStep::Name => {
            if app.setup_provider_index > 0 {
                app.setup_provider_index -= 1;
            } else {
                app.setup_provider_index = crate::config::PRESET_PROVIDERS.len() - 1;
            }
        }
        KeyCode::Enter => {
            let value = app.input.trim().to_string();
            match app.setup_step {
                SetupStep::Name => {
                    let (name, base_url, model) = crate::config::PRESET_PROVIDERS[app.setup_provider_index];
                    app.setup_provider.name = name.to_string();
                    app.setup_provider.base_url = base_url.to_string();
                    app.setup_provider.model = model.to_string();
                    
                    app.setup_step = SetupStep::ApiKey;
                    app.input.clear();
                    app.cursor_pos = 0;
                    app.status_message = None;
                }
                SetupStep::ApiKey => {
                    if value.is_empty() {
                        app.status_message = Some("API Key cannot be empty".to_string());
                        return;
                    }
                    app.setup_provider.api_key = value;
                    app.setup_step = SetupStep::BaseUrl;
                    app.input.clear();
                    app.cursor_pos = 0;
                    app.status_message = None;
                }
                SetupStep::BaseUrl => {
                    if value.is_empty() {
                        app.status_message = Some("Base URL cannot be empty".to_string());
                        return;
                    }
                    app.setup_provider.base_url = value;
                    app.setup_step = SetupStep::Model;
                    app.input.clear();
                    app.cursor_pos = 0;
                    app.status_message = None;
                }
                SetupStep::Model => {
                    if value.is_empty() {
                        app.status_message = Some("Model name cannot be empty".to_string());
                        return;
                    }
                    app.setup_provider.model = value;
                    app.setup_step = SetupStep::Testing;
                    app.input.clear();
                    app.cursor_pos = 0;
                    app.status_message = Some("Testing connection...".to_string());
                    crate::api::test_connection(&app.setup_provider, app.event_tx.clone());
                }
                SetupStep::Testing => {}
            }
        }
        KeyCode::Esc => {
            match app.setup_step {
                SetupStep::Name => app.should_quit = true,
                SetupStep::ApiKey => {
                    app.setup_step = SetupStep::Name;
                    app.input = app.setup_provider.name.clone();
                    app.cursor_pos = app.input.len();
                }
                SetupStep::BaseUrl => {
                    app.setup_step = SetupStep::ApiKey;
                    app.input = app.setup_provider.api_key.clone();
                    app.cursor_pos = app.input.len();
                }
                SetupStep::Model => {
                    app.setup_step = SetupStep::BaseUrl;
                    app.input = app.setup_provider.base_url.clone();
                    app.cursor_pos = app.input.len();
                }
                SetupStep::Testing => {}
            }
        }
        KeyCode::Backspace => {
            app.delete_char_before_cursor();
        }
        KeyCode::Char(c) => {
            if !key.modifiers.contains(KeyModifiers::CONTROL) {
                app.insert_char(c);
            }
        }
        _ => {}
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
    // Search sub-mode stays hardcoded
    if app.searching {
        match key.code {
            KeyCode::Enter => {
                app.searching = false;
            }
            KeyCode::Esc => {
                app.searching = false;
                app.search_query.clear();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
            }
            KeyCode::Backspace => {
                app.search_query.pop();
            }
            _ => {}
        }
        return;
    }

    if let Some(&action) = app.keymap.normal.get(&key) {
        match action {
            Action::Quit => app.should_quit = true,
            Action::EnterInsert => {
                app.mode = Mode::Insert;
                app.move_cursor_to_end();
            }
            Action::EnterInsertAfter => {
                app.mode = Mode::Insert;
                app.move_cursor_right();
            }
            Action::EnterInsertEnd => {
                app.mode = Mode::Insert;
                app.move_cursor_to_end();
            }
            Action::EnterInsertStart => {
                app.mode = Mode::Insert;
                app.move_cursor_to_start();
            }
            Action::EnterVisual => {
                app.mode = Mode::Visual;
                app.visual_start = Some(app.cursor_pos);
            }
            Action::ScrollDown => app.select_next_message(),
            Action::ScrollUp => {
                app.mode = Mode::Browse;
                app.select_prev_message();
            }
            Action::ScrollToBottom => app.select_last_message(),
            Action::ScrollToTop => {
                app.mode = Mode::Browse;
                app.select_first_message();
            }
            Action::CopyResponse => app.copy_selected_message(),
            Action::OpenInEditor => app.open_selected_in_editor(),
            Action::NewConversation => app.new_conversation(),
            Action::NextConversation => app.select_next_conversation(),
            Action::PrevConversation => app.select_prev_conversation(),
            Action::Search => {
                app.searching = true;
                app.search_query.clear();
            }
            Action::SwitchProvider => app.switch_provider(),
            Action::ToggleHelp => { /* handled above */ }
            _ => {}
        }
    } else {
        // Direct key checks for mode transitions
        match key.code {
            KeyCode::Enter => {
                app.mode = Mode::Browse;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.mode = Mode::Browse;
                app.select_prev_message();
            }
            KeyCode::Char('i') => {
                app.mode = Mode::Insert;
            }
            _ => {}
        }
    }
}

fn handle_insert(app: &mut App, key: KeyEvent) {
    if let Some(&action) = app.keymap.insert.get(&key) {
        match action {
            Action::BackToNormal => {
                app.mode = Mode::Normal;
            }
            Action::SendMessage => {
                app.send_message();
                app.mode = Mode::Normal;
            }
            Action::DeleteCharBefore => app.delete_char_before_cursor(),
            Action::DeleteCharAt => app.delete_char_at_cursor(),
            Action::CursorLeft => app.move_cursor_left(),
            Action::CursorRight => app.move_cursor_right(),
            Action::CursorHome => app.move_cursor_to_start(),
            Action::CursorEnd => app.move_cursor_to_end(),
            Action::ClearInput => {
                app.input.clear();
                app.cursor_pos = 0;
            }
            Action::DeleteWord => {
                let before = &app.input[..app.cursor_pos];
                let trimmed = before.trim_end();
                let new_end = trimmed
                    .rfind(|c: char| c.is_whitespace())
                    .map(|i| i + 1)
                    .unwrap_or(0);
                app.input = format!(
                    "{}{}",
                    &app.input[..new_end],
                    &app.input[app.cursor_pos..]
                );
                app.cursor_pos = new_end;
            }
            _ => {}
        }
    } else if let KeyCode::Char(c) = key.code {
        // Fallback: insert character if no action matched and no modifiers
        if !key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT)
        {
            app.insert_char(c);
        }
    }
}

fn handle_help(app: &mut App, key: KeyEvent) {
    if app.help_searching {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                app.help_searching = false;
            }
            KeyCode::Char(c) => {
                app.help_search_query.push(c);
            }
            KeyCode::Backspace => {
                app.help_search_query.pop();
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.show_help = false;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.help_scroll = app.help_scroll.saturating_add(1);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.help_scroll = app.help_scroll.saturating_sub(1);
        }
        KeyCode::Char('G') => {
            app.help_scroll = usize::MAX / 2;
        }
        KeyCode::Char('g') => {
            app.help_scroll = 0;
        }
        KeyCode::Char('f') | KeyCode::Char('/') => {
            app.help_searching = true;
            app.help_search_query.clear();
        }
        _ => {}
    }
}

fn handle_visual(app: &mut App, key: KeyEvent) {
    if let Some(&action) = app.keymap.visual.get(&key) {
        match action {
            Action::BackToNormal => {
                app.mode = Mode::Normal;
                app.visual_start = None;
            }
            Action::CopyResponse => {
                app.copy_selected_message();
                app.mode = Mode::Normal;
                app.visual_start = None;
            }
            Action::ScrollDown => app.select_next_message(),
            Action::ScrollUp => app.select_prev_message(),
            _ => {}
        }
    }
}
