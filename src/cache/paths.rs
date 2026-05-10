use std::path::PathBuf;

use crate::error::{AppError, Result};

pub fn cache_dir() -> Result<PathBuf> {
    let base = dirs::cache_dir()
        .ok_or_else(|| AppError::Config("cache directory not found".to_string()))?;
    Ok(base.join("blackarch-hypr-tui"))
}

pub fn tools_cache_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("tools.json"))
}

pub fn categories_cache_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("categories.json"))
}

pub fn metadata_cache_path() -> Result<PathBuf> {
    Ok(cache_dir()?.join("metadata.json"))
}

pub fn ensure_cache_dir() -> Result<()> {
    std::fs::create_dir_all(cache_dir()?)?;
    Ok(())
}
