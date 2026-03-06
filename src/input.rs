use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Mode};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Clear status message on any key press
    app.status_message = None;

    // Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match app.mode {
        Mode::Normal => handle_normal(app, key),
        Mode::Insert => handle_insert(app, key),
        Mode::Visual => handle_visual(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent) {
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

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('i') => {
            app.mode = Mode::Insert;
            app.move_cursor_to_end();
        }
        KeyCode::Char('a') => {
            app.mode = Mode::Insert;
            app.move_cursor_right();
        }
        KeyCode::Char('A') => {
            app.mode = Mode::Insert;
            app.move_cursor_to_end();
        }
        KeyCode::Char('I') => {
            app.mode = Mode::Insert;
            app.move_cursor_to_start();
        }
        KeyCode::Char('v') => {
            app.mode = Mode::Visual;
            app.visual_start = Some(app.cursor_pos);
        }
        KeyCode::Char('j') | KeyCode::Down => app.scroll_down(),
        KeyCode::Char('k') | KeyCode::Up => app.scroll_up(),
        KeyCode::Char('G') => app.scroll_to_bottom(),
        KeyCode::Char('g') => app.scroll_to_top(),
        KeyCode::Char('y') => app.copy_last_response(),
        KeyCode::Char('n') => app.new_conversation(),
        KeyCode::Char('/') => {
            app.searching = true;
            app.search_query.clear();
        }
        KeyCode::Tab => app.switch_provider(),
        _ => {}
    }
}

fn handle_insert(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            app.send_message();
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => app.delete_char_before_cursor(),
        KeyCode::Delete => app.delete_char_at_cursor(),
        KeyCode::Left => app.move_cursor_left(),
        KeyCode::Right => app.move_cursor_right(),
        KeyCode::Home => app.move_cursor_to_start(),
        KeyCode::End => app.move_cursor_to_end(),
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match c {
                    'a' => app.move_cursor_to_start(),
                    'e' => app.move_cursor_to_end(),
                    'u' => {
                        app.input.clear();
                        app.cursor_pos = 0;
                    }
                    'w' => {
                        // Delete word before cursor
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
            } else {
                app.insert_char(c);
            }
        }
        _ => {}
    }
}

fn handle_visual(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.visual_start = None;
        }
        KeyCode::Char('y') => {
            app.copy_last_response();
            app.mode = Mode::Normal;
            app.visual_start = None;
        }
        KeyCode::Char('j') | KeyCode::Down => app.scroll_down(),
        KeyCode::Char('k') | KeyCode::Up => app.scroll_up(),
        _ => {}
    }
}
