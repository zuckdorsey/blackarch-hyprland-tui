use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    cache::store,
    error::Result,
    models::{BlackArchTool, CacheMetadata, ToolStatus},
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
    let categories_by_tool = query::package_categories()?;
    let tools = query::list_all_available_blackarch_tools()?
        .into_iter()
        .filter_map(|package_name| {
            let categories = categories_by_tool
                .get(&package_name)
                .cloned()
                .unwrap_or_default();
            let fallback_category = categories.first().map(String::as_str);

            match query::build_available_tool(&package_name, fallback_category) {
                Ok(mut tool) => {
                    tool.categories = categories;
                    Some(tool)
                }
                Err(_) => Some(minimal_available_tool(package_name, categories)),
            }
        })
        .collect::<Vec<_>>();

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
    let categories = info
        .groups
        .iter()
        .filter(|group| group.starts_with("blackarch-"))
        .cloned()
        .collect::<Vec<_>>();
    let category = categories.first().cloned();

    Ok(BlackArchTool {
        name: info.name.clone(),
        package_name: info.name,
        category,
        categories,
        version: info.version,
        description: info.description,
        executable: info.executables.first().cloned(),
        executables: info.executables,
        status: if info.installed {
            ToolStatus::Installed
        } else {
            ToolStatus::NotInstalled
        },
        favorite: false,
    })
}

pub fn get_tool_detail_or_partial(
    package_name: &str,
    fallback: Option<&BlackArchTool>,
) -> BlackArchTool {
    match get_tool_detail(package_name) {
        Ok(mut detail) => {
            if let Some(fallback) = fallback {
                detail.favorite = fallback.favorite;
                if detail.category.is_none() {
                    detail.category = fallback.category.clone();
                }
                if detail.categories.is_empty() {
                    detail.categories = fallback.categories.clone();
                }
            }
            detail
        }
        Err(_) => {
            let mut tool = fallback.cloned().unwrap_or_else(|| BlackArchTool {
                name: package_name.to_string(),
                package_name: package_name.to_string(),
                category: None,
                categories: Vec::new(),
                version: None,
                description: None,
                executable: None,
                executables: Vec::new(),
                status: ToolStatus::NotInstalled,
                favorite: false,
            });
            tool.description = Some("Package info unavailable".to_string());
            tool
        }
    }
}

fn generated_at() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn minimal_available_tool(package_name: String, categories: Vec<String>) -> BlackArchTool {
    BlackArchTool {
        name: package_name.clone(),
        package_name,
        category: categories.first().cloned(),
        categories,
        version: None,
        description: None,
        executable: None,
        executables: Vec::new(),
        status: ToolStatus::NotInstalled,
        favorite: false,
    }
}
