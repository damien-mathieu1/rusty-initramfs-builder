mod app;
mod events;
mod screens;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Some(msg) = &app.build_progress {
        println!("{}", msg);
    }
    if let Some(err) = &app.build_error {
        eprintln!("{}", err);
    }

    result
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()>
where
    <B as ratatui::backend::Backend>::Error: Send + Sync + 'static,
{
    loop {
        app.on_tick();
        app.check_build_status();
        terminal.draw(|f| ui::render_app(f, app))?;

        if let Some(action) = events::handle_events(app)? {
            match action {
                events::Action::Quit => break,
                events::Action::Build => {
                    app.start_build();
                }
                events::Action::None => {}
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
