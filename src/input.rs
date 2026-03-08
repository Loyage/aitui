use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Mode};
use crate::keymap::Action;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Clear status message on any key press
    app.status_message = None;

    // Ctrl+C always quits (hardcoded, not rebindable)
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    // Check for ToggleHelp across all modes via keymap
    if let Some(&Action::ToggleHelp) = app.keymap.normal.get(&key) {
        app.show_help = !app.show_help;
        if app.show_help {
            app.help_scroll = 0;
            app.help_searching = false;
            app.help_search_query.clear();
        }
        return;
    }

    // When help is shown, handle help-specific keys
    if app.show_help {
        handle_help(app, key);
        return;
    }

    match app.mode {
        Mode::Normal => handle_normal(app, key),
        Mode::Insert => handle_insert(app, key),
        Mode::Visual => handle_visual(app, key),
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
            Action::ScrollUp => app.select_prev_message(),
            Action::ScrollToBottom => app.select_last_message(),
            Action::ScrollToTop => app.select_first_message(),
            Action::CopyResponse => app.copy_selected_message(),
            Action::OpenInEditor => app.open_selected_in_editor(),
            Action::NewConversation => app.new_conversation(),
            Action::Search => {
                app.searching = true;
                app.search_query.clear();
            }
            Action::SwitchProvider => app.switch_provider(),
            Action::ToggleHelp => { /* handled above */ }
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
