use std::fs;

use crate::{config::paths, error::Result, models::AppConfig};

pub fn load_config() -> Result<AppConfig> {
    save_default_config_if_missing()?;
    let content = fs::read_to_string(paths::config_path()?)?;
    Ok(toml::from_str(&content)?)
}

pub fn save_default_config_if_missing() -> Result<()> {
    let path = paths::config_path()?;
    if path.exists() {
        return Ok(());
    }

    paths::ensure_config_dir()?;
    let config = toml::to_string_pretty(&AppConfig::default())?;
    fs::write(path, config)?;
    Ok(())
}
