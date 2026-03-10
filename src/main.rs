mod api;
mod app;
mod config;
mod event;
mod history;
mod input;
mod keymap;
mod ui;

use std::io;

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;
use config::Config;
use event::{Event, EventLoop};
use keymap::Keymap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let keymap = Keymap::load()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let event_loop = EventLoop::new();
    let event_tx = event_loop.sender();
    event_loop.start_input_loop();

    let mut app = App::new(config, keymap, event_tx);

    let result = run_app(&mut terminal, &mut app, event_loop).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    mut event_loop: EventLoop,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Some(event) = event_loop.rx.recv().await {
            match event {
                Event::Key(key) => {
                    input::handle_key(app, key);
                }
                Event::ApiToken(token) => {
                    app.on_api_token(token);
                }
                Event::ApiDone => {
                    app.on_api_done();
                }
                Event::ApiError(err) => {
                    app.on_api_error(err);
                }
                Event::OpenEditor(path) => {
                    // Suspend TUI, open editor, resume TUI
                    disable_raw_mode()?;
                    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                    terminal.show_cursor()?;

                    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
                    let status = std::process::Command::new(&editor)
                        .arg(&path)
                        .status();

                    // Resume TUI regardless of editor result
                    enable_raw_mode()?;
                    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                    terminal.clear()?;

                    // Clean up temp file
                    let _ = std::fs::remove_file(&path);

                    if let Err(e) = status {
                        app.status_message = Some(format!("Failed to open editor: {}", e));
                    }
                }
                Event::Tick => {
                    // Just triggers a redraw
                }
            }
        }

        if app.should_quit {
            // Save current conversation before quitting
            let _ = history::save_conversation(app.conversation());
            break;
        }
    }

    Ok(())
}
