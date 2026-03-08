use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Mode};
use crate::history::Role;
use crate::keymap::Action;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Chat area
            Constraint::Length(3), // Input area
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    draw_chat(f, app, chunks[0]);
    draw_input(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);

    if app.show_help {
        draw_help(f, app);
    }
}

fn draw_chat(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" AiTUI - {} ({}) ", app.provider().name, app.provider().model))
        .title_alignment(ratatui::layout::Alignment::Center);

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.conversation.messages.is_empty() {
        let welcome = Paragraph::new(Line::from(vec![Span::styled(
            "Press 'i' to start typing, Enter to send. 'q' to quit.",
            Style::default().fg(Color::DarkGray),
        )]))
        .wrap(Wrap { trim: false });
        f.render_widget(welcome, inner);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    // Track (message_index, start_line, end_line) for each non-System message
    let mut msg_ranges: Vec<(usize, usize, usize)> = Vec::new();

    for (msg_idx, msg) in app.conversation.messages.iter().enumerate() {
        match msg.role {
            Role::User => {
                let start = lines.len();
                lines.push(Line::from(vec![Span::styled(
                    "You:",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )]));
                for text_line in msg.content.lines() {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(text_line.to_string(), Style::default().fg(Color::White)),
                    ]));
                }
                lines.push(Line::from(""));
                msg_ranges.push((msg_idx, start, lines.len()));
            }
            Role::Assistant => {
                let start = lines.len();
                lines.push(Line::from(vec![Span::styled(
                    "AI:",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )]));
                if msg.content.is_empty() && app.streaming {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled("...", Style::default().fg(Color::DarkGray)),
                    ]));
                } else {
                    for text_line in msg.content.lines() {
                        let wrapped = wrap_text(text_line, inner.width.saturating_sub(2) as usize);
                        for wl in wrapped {
                            lines.push(Line::from(vec![
                                Span::raw("  "),
                                Span::styled(wl, Style::default().fg(Color::White)),
                            ]));
                        }
                    }
                    if app.streaming {
                        if let Some(last_line) = lines.last_mut() {
                            last_line.spans.push(Span::styled(
                                "▌",
                                Style::default().fg(Color::Yellow),
                            ));
                        }
                    }
                }
                lines.push(Line::from(""));
                msg_ranges.push((msg_idx, start, lines.len()));
            }
            Role::System => {}
        }
    }

    // Apply selection highlight — add left marker ▎ to selected message lines
    if let Some(sel_idx) = app.selected_message {
        if let Some(&(_, start, end)) = msg_ranges.iter().find(|(i, _, _)| *i == sel_idx) {
            let highlight_bg = Style::default().bg(Color::DarkGray);
            for line in &mut lines[start..end] {
                // Prepend a highlight marker
                let mut new_spans = vec![Span::styled("▎", Style::default().fg(Color::Yellow))];
                for span in &line.spans {
                    new_spans.push(Span::styled(
                        span.content.clone(),
                        span.style.bg(Color::DarkGray),
                    ));
                }
                // If the line is empty (separator), still highlight
                if new_spans.len() == 1 {
                    new_spans.push(Span::styled(" ", highlight_bg));
                }
                line.spans = new_spans;
            }
        }
    }

    // Highlight search matches
    if !app.search_query.is_empty() {
        let query_lower = app.search_query.to_lowercase();
        for line in &mut lines {
            let full_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            if full_text.to_lowercase().contains(&query_lower) {
                for span in &mut line.spans {
                    span.style = span.style.bg(Color::Rgb(80, 80, 0));
                }
            }
        }
    }

    let total_lines = lines.len();
    let visible_height = inner.height as usize;

    // Auto-scroll: compute scroll offset to keep selected message visible
    let scroll = if let Some(sel_idx) = app.selected_message {
        if let Some(&(_, sel_start, _sel_end)) = msg_ranges.iter().find(|(i, _, _)| *i == sel_idx) {
            // Target: place message top at ~1/3 from viewport top
            let target_top = visible_height / 3;
            if sel_start <= target_top {
                // Message is near the top, just show from beginning
                0
            } else if total_lines <= visible_height {
                // Everything fits
                0
            } else {
                let ideal = sel_start.saturating_sub(target_top);
                // Don't scroll past the end
                ideal.min(total_lines.saturating_sub(visible_height))
            }
        } else {
            // Selected message not found in ranges, show bottom
            total_lines.saturating_sub(visible_height)
        }
    } else {
        // No selection — show bottom (latest messages)
        total_lines.saturating_sub(visible_height)
    };

    let end = (scroll + visible_height).min(total_lines);
    let visible_lines: Vec<Line> = lines[scroll..end].to_vec();

    let paragraph = Paragraph::new(visible_lines);
    f.render_widget(paragraph, inner);
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let mode_label = match app.mode {
        Mode::Insert => "[INSERT]",
        Mode::Normal => "[NORMAL]",
        Mode::Visual => "[VISUAL]",
    };

    let mode_color = match app.mode {
        Mode::Insert => Color::Green,
        Mode::Normal => Color::Blue,
        Mode::Visual => Color::Magenta,
    };

    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let input_line = Line::from(vec![
        Span::styled(
            if app.searching {
                "/"
            } else {
                mode_label
            },
            Style::default()
                .fg(mode_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(if app.searching {
            &app.search_query
        } else if app.streaming {
            " Streaming..."
        } else {
            ""
        }),
        if !app.searching && !app.streaming {
            Span::raw(format!(" > {}", &app.input))
        } else {
            Span::raw("")
        },
    ]);

    let paragraph = Paragraph::new(input_line);
    f.render_widget(paragraph, inner);

    // Show cursor in insert mode
    if app.mode == Mode::Insert && !app.streaming {
        let prefix_width = format!("{} > ", mode_label).width() as u16;
        let cursor_x = inner.x + prefix_width + app.input[..app.cursor_pos].width() as u16;
        let cursor_y = inner.y;
        f.set_cursor_position((cursor_x, cursor_y));
    } else if app.searching {
        let cursor_x = inner.x + 1 + app.search_query.width() as u16;
        f.set_cursor_position((cursor_x, inner.y));
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let status = if let Some(ref msg) = app.status_message {
        Span::styled(msg.as_str(), Style::default().fg(Color::Yellow))
    } else if app.streaming {
        Span::styled("Receiving response...", Style::default().fg(Color::Cyan))
    } else {
        let msg_count = app.conversation.messages.len();
        let km = &app.keymap;
        let quit_keys = km.keys_for_action("normal", Action::Quit).join("/");
        let insert_keys = km.keys_for_action("normal", Action::EnterInsert).join("/");
        let nav_d = km.keys_for_action("normal", Action::ScrollDown);
        let nav_u = km.keys_for_action("normal", Action::ScrollUp);
        let nav_str = if !nav_d.is_empty() && !nav_u.is_empty() {
            format!("{}/{}:select", nav_d[0], nav_u[0])
        } else {
            String::new()
        };
        let copy_keys = km.keys_for_action("normal", Action::CopyResponse).join("/");
        let editor_keys = km.keys_for_action("normal", Action::OpenInEditor).join("/");
        let new_keys = km.keys_for_action("normal", Action::NewConversation).join("/");
        let switch_keys = km.keys_for_action("normal", Action::SwitchProvider).join("/");
        let search_keys = km.keys_for_action("normal", Action::Search).join("/");
        let help_keys = km.keys_for_action("normal", Action::ToggleHelp).join("/");
        Span::styled(
            format!(
                " {} messages | {}:quit {}:insert {} {}:copy {}:view {}:new {}:switch {}:search {}:help",
                msg_count, quit_keys, insert_keys, nav_str, copy_keys, editor_keys, new_keys, switch_keys, search_keys, help_keys
            ),
            Style::default().fg(Color::DarkGray),
        )
    };

    let paragraph = Paragraph::new(Line::from(status));
    f.render_widget(paragraph, area);
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;

    for word in text.split_inclusive(|c: char| c.is_whitespace()) {
        let word_width = word.width();
        if current_width + word_width > max_width && !current.is_empty() {
            lines.push(current);
            current = String::new();
            current_width = 0;
        }
        current.push_str(word);
        current_width += word_width;
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn draw_help(f: &mut Frame, app: &App) {
    let area = f.area();

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let dim = Style::default().fg(Color::DarkGray);

    let km = &app.keymap;

    let fmt = |mode: &str, action: Action| -> String {
        let keys = km.keys_for_action(mode, action);
        if keys.is_empty() {
            "(unbound)".to_string()
        } else {
            keys.join(" / ")
        }
    };

    let pad = |s: String, w: usize| -> String {
        if s.len() >= w { s } else { format!("{}{}", s, " ".repeat(w - s.len())) }
    };

    let kw = 16usize; // key column width

    let help_toggle_keys = fmt("normal", Action::ToggleHelp);

    let mut help_lines = vec![
        Line::from(Span::styled(" Keybindings Reference", bold)),
        Line::from(""),
        Line::from(Span::styled(" ── Normal Mode ──", bold.fg(Color::Blue))),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::EnterInsert)), kw), key_style), Span::raw("Enter Insert mode")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::EnterInsertAfter)), kw), key_style), Span::raw("Insert after cursor")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::EnterInsertEnd)), kw), key_style), Span::raw("Insert at end")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::EnterInsertStart)), kw), key_style), Span::raw("Insert at start")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::ScrollDown)), kw), key_style), Span::raw("Select next message")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::ScrollUp)), kw), key_style), Span::raw("Select previous message")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::ScrollToBottom)), kw), key_style), Span::raw("Select last message")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::ScrollToTop)), kw), key_style), Span::raw("Select first message")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::CopyResponse)), kw), key_style), Span::raw("Copy selected message")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::OpenInEditor)), kw), key_style), Span::raw("View in $EDITOR")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::NewConversation)), kw), key_style), Span::raw("New conversation")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::SwitchProvider)), kw), key_style), Span::raw("Switch provider")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::Search)), kw), key_style), Span::raw("Search in conversation")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::EnterVisual)), kw), key_style), Span::raw("Enter Visual mode")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("normal", Action::Quit)), kw), key_style), Span::raw("Quit")]),
        Line::from(""),
        Line::from(Span::styled(" ── Insert Mode ──", bold.fg(Color::Green))),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("insert", Action::SendMessage)), kw), key_style), Span::raw("Send message")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("insert", Action::BackToNormal)), kw), key_style), Span::raw("Back to Normal mode")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("insert", Action::DeleteCharBefore)), kw), key_style), Span::raw("Delete char before cursor")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("insert", Action::DeleteCharAt)), kw), key_style), Span::raw("Delete char at cursor")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("insert", Action::CursorLeft)), kw), key_style), Span::raw("Move cursor left")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("insert", Action::CursorRight)), kw), key_style), Span::raw("Move cursor right")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("insert", Action::CursorHome)), kw), key_style), Span::raw("Move to line start")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("insert", Action::CursorEnd)), kw), key_style), Span::raw("Move to line end")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("insert", Action::ClearInput)), kw), key_style), Span::raw("Clear input")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("insert", Action::DeleteWord)), kw), key_style), Span::raw("Delete word")]),
        Line::from(""),
        Line::from(Span::styled(" ── Visual Mode ──", bold.fg(Color::Magenta))),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("visual", Action::BackToNormal)), kw), key_style), Span::raw("Back to Normal mode")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("visual", Action::CopyResponse)), kw), key_style), Span::raw("Copy selected message")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("visual", Action::ScrollDown)), kw), key_style), Span::raw("Select next message")]),
        Line::from(vec![Span::styled(pad(format!("  {}", fmt("visual", Action::ScrollUp)), kw), key_style), Span::raw("Select previous message")]),
    ];

    // Highlight search matches
    if !app.help_search_query.is_empty() {
        let query = app.help_search_query.to_lowercase();
        for line in &mut help_lines {
            let full_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            if full_text.to_lowercase().contains(&query) {
                for span in &mut line.spans {
                    span.style = span.style.bg(Color::DarkGray);
                }
            }
        }
    }

    // Layout: full screen with border, bottom status line
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // help content
            Constraint::Length(1), // status line
        ])
        .split(area);

    let content_area = chunks[0];
    let status_area = chunks[1];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Help ({}) ", help_toggle_keys))
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(content_area);
    let visible_height = inner.height as usize;
    let total_lines = help_lines.len();
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = app.help_scroll.min(max_scroll);

    let visible_lines: Vec<Line> = help_lines
        .into_iter()
        .skip(scroll)
        .take(visible_height)
        .collect();

    f.render_widget(Clear, area);
    f.render_widget(block, content_area);
    let paragraph = Paragraph::new(visible_lines);
    f.render_widget(paragraph, inner);

    // Bottom status bar
    let status_line = if app.help_searching {
        Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw(&app.help_search_query),
        ])
    } else {
        Line::from(Span::styled(
            " j/k:scroll  g/G:top/bottom  f:search  q/Esc:close",
            dim,
        ))
    };
    f.render_widget(Paragraph::new(status_line), status_area);

    // Show cursor when searching
    if app.help_searching {
        let cursor_x = status_area.x + 1 + app.help_search_query.width() as u16;
        f.set_cursor_position((cursor_x, status_area.y));
    }
}
