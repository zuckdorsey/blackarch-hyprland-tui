use std::{fs, path::Path};

use crate::{
    cache::paths,
    error::{AppError, Result},
    models::{BlackArchTool, CacheMetadata},
    utils::validate::validate_package_name,
};

#[allow(dead_code)]
pub fn load_tools_cache() -> Result<Option<Vec<BlackArchTool>>> {
    load_json(&paths::tools_cache_path()?)
}

pub fn save_tools_cache(tools: &[BlackArchTool]) -> Result<()> {
    save_json(&paths::tools_cache_path()?, tools)
}

pub fn load_categories_cache() -> Result<Option<Vec<String>>> {
    load_json(&paths::categories_cache_path()?)
}

pub fn save_categories_cache(categories: &[String]) -> Result<()> {
    save_json(&paths::categories_cache_path()?, categories)
}

pub fn save_metadata(metadata: &CacheMetadata) -> Result<()> {
    save_json(&paths::metadata_cache_path()?, metadata)
}

pub fn save_package_detail_cache(package_name: &str, tool: &BlackArchTool) -> Result<()> {
    if !validate_package_name(package_name) {
        return Err(AppError::InvalidPackageName(package_name.to_string()));
    }

    paths::ensure_packages_cache_dir()?;
    save_json(&paths::package_detail_cache_path(package_name)?, tool)
}

#[allow(dead_code)]
pub fn cache_exists() -> bool {
    paths::tools_cache_path().is_ok_and(|path| path.exists())
        || paths::categories_cache_path().is_ok_and(|path| path.exists())
}

fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(path)?;
    Ok(Some(serde_json::from_str(&content)?))
}

fn save_json<T: serde::Serialize + ?Sized>(path: &Path, value: &T) -> Result<()> {
    paths::ensure_cache_dir()?;
    let content = serde_json::to_string_pretty(value)?;
    fs::write(path, content)?;
    Ok(())
}
