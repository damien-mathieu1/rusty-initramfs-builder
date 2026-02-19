use crate::tui::app::{App, Screen};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::time::Duration;

pub enum Action {
    None,
    Quit,
    Build,
}

pub fn handle_events(app: &mut App) -> Result<Option<Action>> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                return Ok(Some(Action::None));
            }

            if key.code == KeyCode::Char('q') && !matches!(app.screen, Screen::Image) {
                return Ok(Some(Action::Quit));
            }

            match app.screen {
                Screen::Language => handle_language_keys(app, key.code),
                Screen::Image => handle_image_keys(app, key.code),
                Screen::Summary => {
                    return handle_summary_keys(app, key.code);
                }
                Screen::Build => {
                    if (app.build_success || app.build_error.is_some())
                        && (key.code == KeyCode::Enter || key.code == KeyCode::Char('q'))
                    {
                        return Ok(Some(Action::Quit));
                    }
                }
            }
        }
    }
    Ok(Some(Action::None))
}

fn handle_language_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Up => app.language_screen.prev(),
        KeyCode::Down => app.language_screen.next(),
        KeyCode::Left => app.language_screen.prev_version(),
        KeyCode::Right => app.language_screen.next_version(),
        KeyCode::Enter => app.next_screen(),
        KeyCode::Esc => app.should_quit = true,
        _ => {}
    }
}

fn handle_image_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => {
            if app.validate_current_screen() {
                app.next_screen();
            } else {
                app.image_screen.set_error("Image cannot be empty");
            }
        }
        KeyCode::Esc => app.prev_screen(),
        KeyCode::Backspace => {
            app.image_screen.input.pop();
            app.image_screen.error = None;
        }
        KeyCode::Char(c) => {
            app.image_screen.input.push(c);
            app.image_screen.error = None;
        }
        _ => {}
    }
}

fn handle_summary_keys(app: &mut App, key: KeyCode) -> Result<Option<Action>> {
    match key {
        KeyCode::Enter => {
            if app.validate_current_screen() {
                return Ok(Some(Action::Build));
            }
        }
        KeyCode::Esc => app.prev_screen(),
        _ => {}
    }
    Ok(Some(Action::None))
}
