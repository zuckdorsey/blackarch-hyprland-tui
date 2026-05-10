use std::path::PathBuf;

use crate::error::{AppError, Result};

pub fn config_dir() -> Result<PathBuf> {
    let base = dirs::config_dir()
        .ok_or_else(|| AppError::Config("config directory not found".to_string()))?;
    Ok(base.join("blackarch-hypr-tui"))
}

pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn ensure_config_dir() -> Result<()> {
    std::fs::create_dir_all(config_dir()?)?;
    Ok(())
}
