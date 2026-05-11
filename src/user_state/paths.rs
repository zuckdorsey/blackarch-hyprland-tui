use std::path::PathBuf;

use crate::error::{AppError, Result};

pub fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| AppError::Config("local data directory not found".to_string()))?;
    Ok(base.join("blackarch-hypr-tui"))
}

pub fn favorites_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("favorites.json"))
}

pub fn recent_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("recent.json"))
}

pub fn ensure_data_dir() -> Result<()> {
    std::fs::create_dir_all(data_dir()?)?;
    Ok(())
}
