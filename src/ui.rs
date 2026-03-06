use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Mode};
use crate::history::Role;

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

    for msg in &app.conversation.messages {
        match msg.role {
            Role::User => {
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
            }
            Role::Assistant => {
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
                        // Simple word wrapping for long lines
                        let wrapped = wrap_text(text_line, inner.width.saturating_sub(2) as usize);
                        for wl in wrapped {
                            lines.push(Line::from(vec![
                                Span::raw("  "),
                                Span::styled(wl, Style::default().fg(Color::White)),
                            ]));
                        }
                    }
                    // If content doesn't end with newline and is streaming, show cursor
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
            }
            Role::System => {}
        }
    }

    // Highlight search matches
    if !app.search_query.is_empty() {
        for line in &mut lines {
            let full_text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            if full_text
                .to_lowercase()
                .contains(&app.search_query.to_lowercase())
            {
                for span in &mut line.spans {
                    span.style = span.style.bg(Color::DarkGray);
                }
            }
        }
    }

    let total_lines = lines.len();
    let visible_height = inner.height as usize;

    // scroll_offset 0 = bottom, increasing = scroll up
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = app.scroll_offset.min(max_scroll);
    let start = total_lines.saturating_sub(visible_height + scroll);
    let end = total_lines.saturating_sub(scroll);

    let visible_lines: Vec<Line> = lines[start..end.min(total_lines)].to_vec();

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
        Span::styled(
            format!(
                " {} messages | q:quit i:insert j/k:scroll y:copy n:new Tab:switch /search",
                msg_count
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
