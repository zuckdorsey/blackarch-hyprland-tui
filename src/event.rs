use std::sync::mpsc;

use crossterm::event::{self, Event, KeyCode};

use crate::{
    app::{App, FocusPane},
    error::Result,
    worker::WorkerCommand,
};

pub fn handle_events(app: &mut App, worker_tx: &mpsc::Sender<WorkerCommand>) -> Result<()> {
    let Event::Key(key) = event::read()? else {
        return Ok(());
    };

    if app.action_menu.visible {
        return handle_action_menu_key(app, key.code, worker_tx);
    }

    if app.focus == FocusPane::Search {
        handle_search_key(app, key.code, worker_tx);
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
        KeyCode::Enter => app.open_action_menu(),
        KeyCode::Char('s') => app.begin_sync_cache(worker_tx),
        KeyCode::Char('d') => app.refresh_selected_tool_detail(worker_tx),
        KeyCode::Char('r') => {
            let result = app.run_selected_tool(worker_tx);
            handle_local_result(app, result);
        }
        KeyCode::Char('i') => {
            app.status_message = "Install action is not implemented yet".to_string()
        }
        KeyCode::Char('x') => {
            app.status_message = "Remove action is not implemented yet".to_string()
        }
        KeyCode::Char('f') => {
            let result = app.toggle_selected_favorite();
            handle_local_result(app, result);
        }
        KeyCode::Char('c') => {
            let result = app.copy_selected_command();
            handle_local_result(app, result);
        }
        KeyCode::Char('?') => {
            app.status_message =
                "Help: ↑↓ move • Tab focus/exit search • / search • s sync • d details • q quit"
                    .to_string();
        }
        _ => {}
    }

    Ok(())
}

fn handle_action_menu_key(
    app: &mut App,
    code: KeyCode,
    worker_tx: &mpsc::Sender<WorkerCommand>,
) -> Result<()> {
    match code {
        KeyCode::Up => app.select_previous_action(),
        KeyCode::Down => app.select_next_action(),
        KeyCode::Enter => {
            let result = app.execute_selected_action(worker_tx);
            handle_local_result(app, result);
        }
        KeyCode::Esc | KeyCode::Char('q') => app.close_action_menu(),
        _ => {}
    }

    Ok(())
}

fn handle_local_result(app: &mut App, result: Result<()>) {
    if let Err(error) = result {
        let message = error.to_string();
        app.error_message = Some(message.clone());
        app.status_message = message;
    }
}

fn handle_search_key(app: &mut App, code: KeyCode, worker_tx: &mpsc::Sender<WorkerCommand>) {
    match code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Tab => app.exit_search(),
        KeyCode::Backspace => app.pop_search_char(),
        KeyCode::Char(ch) => app.push_search_char(ch),
        KeyCode::Up => app.select_previous_tool(),
        KeyCode::Down => app.select_next_tool(),
        KeyCode::F(5) => app.begin_sync_cache(worker_tx),
        _ => {}
    }
}
