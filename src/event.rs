use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};

use crate::{
    app::{App, FocusPane},
    error::Result,
};

pub fn handle_events(app: &mut App) -> Result<()> {
    if !event::poll(Duration::from_millis(200))? {
        return Ok(());
    }

    let Event::Key(key) = event::read()? else {
        return Ok(());
    };

    if app.focus == FocusPane::Search {
        handle_search_key(app, key.code);
        return Ok(());
    }

    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => app.clear_error(),
        KeyCode::Tab => app.focus_next(),
        KeyCode::Char('/') => app.enter_search(),
        KeyCode::Up => match app.focus {
            FocusPane::Categories => app.select_previous_category(),
            _ => app.select_previous_tool(),
        },
        KeyCode::Down => match app.focus {
            FocusPane::Categories => app.select_next_category(),
            _ => app.select_next_tool(),
        },
        KeyCode::Enter => app.status_message = "Action menu is not implemented yet".to_string(),
        KeyCode::Char('s') => sync_cache(app),
        KeyCode::Char('r') => app.status_message = "Run action is not implemented yet".to_string(),
        KeyCode::Char('i') => {
            app.status_message = "Install action is not implemented yet".to_string()
        }
        KeyCode::Char('x') => {
            app.status_message = "Remove action is not implemented yet".to_string()
        }
        KeyCode::Char('f') => app.toggle_selected_favorite(),
        KeyCode::Char('?') => {
            app.status_message =
                "Help: ↑↓ move • Tab focus • / search • s sync • q quit".to_string();
        }
        _ => {}
    }

    Ok(())
}

fn handle_search_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.exit_search(),
        KeyCode::Enter => app.exit_search(),
        KeyCode::Backspace => app.pop_search_char(),
        KeyCode::Char(ch) => app.push_search_char(ch),
        KeyCode::Up => app.select_previous_tool(),
        KeyCode::Down => app.select_next_tool(),
        _ => {}
    }
}

fn sync_cache(app: &mut App) {
    if let Err(error) = app.refresh_from_backend() {
        app.set_error(error);
    }
}
