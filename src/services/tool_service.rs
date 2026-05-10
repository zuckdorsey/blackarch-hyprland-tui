use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    cache::store,
    error::Result,
    models::{BlackArchTool, CacheMetadata},
    pacman::query,
};

#[allow(dead_code)]
pub fn load_tools(prefer_cache: bool) -> Result<Vec<BlackArchTool>> {
    if prefer_cache {
        if let Some(tools) = store::load_tools_cache()? {
            return Ok(tools);
        }
    }

    refresh_tools_cache()
}

pub fn refresh_tools_cache() -> Result<Vec<BlackArchTool>> {
    let tools = query::list_all_blackarch_tools()?
        .into_iter()
        .map(|(package_name, category)| query::build_tool(&package_name, &category))
        .collect::<Result<Vec<_>>>()?;

    store::save_tools_cache(&tools)?;
    store::save_metadata(&CacheMetadata {
        generated_at: generated_at(),
        tool_count: tools.len(),
    })?;

    Ok(tools)
}

pub fn load_categories(prefer_cache: bool) -> Result<Vec<String>> {
    if prefer_cache {
        if let Some(categories) = store::load_categories_cache()? {
            return Ok(categories);
        }
    }

    refresh_categories_cache()
}

pub fn refresh_categories_cache() -> Result<Vec<String>> {
    let categories = query::list_blackarch_categories()?;
    store::save_categories_cache(&categories)?;
    Ok(categories)
}

pub fn get_tool_detail(package_name: &str) -> Result<BlackArchTool> {
    let info = query::get_package_info(package_name)?;
    let category = info
        .groups
        .iter()
        .find(|group| group.starts_with("blackarch-"))
        .cloned()
        .unwrap_or_else(|| "blackarch-unknown".to_string());

    Ok(BlackArchTool {
        name: info.name.clone(),
        package_name: info.name,
        category,
        version: info.version,
        description: info.description,
        executable: info.executables.first().cloned(),
        executables: info.executables,
        status: if info.installed {
            crate::models::ToolStatus::Installed
        } else {
            crate::models::ToolStatus::NotInstalled
        },
        favorite: false,
    })
}

fn generated_at() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
