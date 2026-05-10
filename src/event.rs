use std::time::Duration;

use crossterm::event::{self, Event, KeyCode};

use crate::error::Result;

pub fn should_quit() -> Result<bool> {
    if !event::poll(Duration::from_millis(200))? {
        return Ok(false);
    }

    let Event::Key(key) = event::read()? else {
        return Ok(false);
    };

    Ok(matches!(key.code, KeyCode::Char('q') | KeyCode::Esc))
}
