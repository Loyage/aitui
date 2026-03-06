mod api;
mod app;
mod config;
mod event;
mod history;
mod input;
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let event_loop = EventLoop::new();
    let event_tx = event_loop.sender();
    event_loop.start_input_loop();

    let mut app = App::new(config, event_tx);

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
                Event::Tick => {
                    // Just triggers a redraw
                }
            }
        }

        if app.should_quit {
            // Save conversation before quitting
            let _ = history::save_conversation(&app.conversation);
            break;
        }
    }

    Ok(())
}
