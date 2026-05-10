use std::collections::BTreeMap;

use crate::{
    error::{AppError, Result},
    models::{BlackArchTool, PackageInfo, PackageSearchResult, ToolStatus},
    pacman::{command, parser},
    utils::validate::{validate_category_name, validate_package_name},
};

pub fn list_blackarch_categories() -> Result<Vec<String>> {
    let output = command::run_pacman(&["-Sg"])?;
    let categories = parser::parse_blackarch_categories(&output);

    if categories.is_empty() {
        return Err(AppError::BlackArchRepoNotFound);
    }

    Ok(categories)
}

pub fn list_tools_by_category(category: &str) -> Result<Vec<String>> {
    if !validate_category_name(category) {
        return Err(AppError::InvalidCategoryName(category.to_string()));
    }

    let output = command::run_pacman_owned(vec!["-Sg".to_string(), category.to_string()])?;
    Ok(parser::parse_tools_from_group(&output, category))
}

pub fn list_all_available_blackarch_tools() -> Result<Vec<String>> {
    let output = command::run_pacman(&["-Sgg"])?;
    let tools = parser::parse_all_available_blackarch_tools(&output);

    if tools.is_empty() {
        return Err(AppError::BlackArchRepoNotFound);
    }

    Ok(tools)
}

pub fn list_all_blackarch_tool_categories() -> Result<Vec<(String, String)>> {
    let output = command::run_pacman(&["-Sgg"])?;
    let tools = parser::parse_all_blackarch_tool_categories(&output);

    if tools.is_empty() {
        return Err(AppError::BlackArchRepoNotFound);
    }

    Ok(tools)
}

#[allow(dead_code)]
pub fn build_tool_index() -> Result<Vec<BlackArchTool>> {
    let categories_by_tool = package_categories()?;

    Ok(list_all_available_blackarch_tools()?
        .into_iter()
        .map(|package_name| {
            let fallback_category = categories_by_tool
                .get(&package_name)
                .and_then(|categories| categories.first())
                .map(String::as_str);
            build_available_tool(&package_name, fallback_category)
        })
        .collect::<Result<Vec<_>>>()?)
}

pub fn search_blackarch_packages(query: &str) -> Result<Vec<PackageSearchResult>> {
    if !validate_package_name(query) {
        return Err(AppError::InvalidPackageName(query.to_string()));
    }

    let output = command::run_pacman_owned(vec!["-Ss".to_string(), query.to_string()])?;
    Ok(parser::parse_search_results(&output))
}

pub fn package_categories() -> Result<BTreeMap<String, Vec<String>>> {
    let mut categories_by_tool = BTreeMap::<String, Vec<String>>::new();

    for (package_name, category) in list_all_blackarch_tool_categories()? {
        let categories = categories_by_tool.entry(package_name).or_default();
        if !categories.iter().any(|item| item == &category) {
            categories.push(category);
        }
    }

    for categories in categories_by_tool.values_mut() {
        categories.sort();
    }

    Ok(categories_by_tool)
}

pub fn build_available_tool(
    package_name: &str,
    fallback_category: Option<&str>,
) -> Result<BlackArchTool> {
    if !validate_package_name(package_name) {
        return Err(AppError::InvalidPackageName(package_name.to_string()));
    }

    if let Some(category) = fallback_category {
        if !validate_category_name(category) {
            return Err(AppError::InvalidCategoryName(category.to_string()));
        }
    }

    let status = if is_installed(package_name)? {
        ToolStatus::Installed
    } else {
        ToolStatus::NotInstalled
    };
    let categories = fallback_category
        .map(|category| vec![category.to_string()])
        .unwrap_or_default();

    Ok(BlackArchTool {
        name: package_name.to_string(),
        package_name: package_name.to_string(),
        category: fallback_category.map(ToString::to_string),
        categories,
        version: None,
        description: None,
        executable: None,
        executables: Vec::new(),
        status,
        favorite: false,
    })
}

#[allow(dead_code)]
pub fn list_all_blackarch_tools() -> Result<Vec<(String, String)>> {
    list_all_blackarch_tool_categories()
}

#[allow(dead_code)]
pub fn list_installed_packages() -> Result<std::collections::HashSet<String>> {
    Ok(list_all_available_blackarch_tools()?
        .into_iter()
        .filter_map(|package_name| match is_installed(&package_name) {
            Ok(true) => Some(package_name),
            _ => None,
        })
        .collect())
}

pub fn get_package_info(package_name: &str) -> Result<PackageInfo> {
    if !validate_package_name(package_name) {
        return Err(AppError::InvalidPackageName(package_name.to_string()));
    }

    let output = command::run_pacman_owned(vec!["-Si".to_string(), package_name.to_string()])?;
    let mut info = parser::parse_package_info(&output);
    if info.name.is_empty() {
        info.name = package_name.to_string();
    }
    info.installed = is_installed(package_name)?;
    info.executables = get_executables_if_installed(package_name)?;
    Ok(info)
}

pub fn is_installed(package_name: &str) -> Result<bool> {
    if !validate_package_name(package_name) {
        return Err(AppError::InvalidPackageName(package_name.to_string()));
    }

    match command::run_pacman_owned(vec!["-Q".to_string(), package_name.to_string()]) {
        Ok(_) => Ok(true),
        Err(AppError::CommandFailed { stderr, .. })
            if stderr.contains("was not found") || stderr.contains("not found") =>
        {
            Ok(false)
        }
        Err(AppError::CommandFailed { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}

pub fn get_executables_if_installed(package_name: &str) -> Result<Vec<String>> {
    if !validate_package_name(package_name) {
        return Err(AppError::InvalidPackageName(package_name.to_string()));
    }

    if !is_installed(package_name)? {
        return Ok(Vec::new());
    }

    match command::run_pacman_owned(vec!["-Ql".to_string(), package_name.to_string()]) {
        Ok(output) => Ok(parser::parse_executables(&output)),
        Err(AppError::CommandFailed { stderr, .. })
            if stderr.contains("was not found") || stderr.contains("not found") =>
        {
            Ok(Vec::new())
        }
        Err(AppError::CommandFailed { .. }) => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

#[allow(dead_code)]
pub fn get_executables(package_name: &str) -> Result<Vec<String>> {
    get_executables_if_installed(package_name)
}

#[allow(dead_code)]
pub fn build_tool(package_name: &str, category: &str) -> Result<BlackArchTool> {
    if !validate_category_name(category) {
        return Err(AppError::InvalidCategoryName(category.to_string()));
    }

    let info = get_package_info(package_name)?;
    let executable = info.executables.first().cloned();
    let status = if info.installed {
        ToolStatus::Installed
    } else {
        ToolStatus::NotInstalled
    };

    Ok(BlackArchTool {
        name: info.name.clone(),
        package_name: info.name,
        category: Some(category.to_string()),
        categories: info.groups.clone(),
        version: info.version,
        description: info.description,
        executable,
        executables: info.executables,
        status,
        favorite: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pacman_q_command_failure_is_not_installed() {
        let error = AppError::CommandFailed {
            command: "pacman -Q missing".to_string(),
            status: "exit status: 1".to_string(),
            stderr: "error: package 'missing' was not found".to_string(),
        };

        assert_eq!(installed_from_command_result(Err(error)).unwrap(), false);
    }

    #[test]
    fn pacman_ql_not_installed_returns_empty_executables() {
        let error = AppError::CommandFailed {
            command: "pacman -Ql missing".to_string(),
            status: "exit status: 1".to_string(),
            stderr: "error: package 'missing' was not found".to_string(),
        };

        assert_eq!(
            executables_from_command_result(Err(error)).unwrap(),
            Vec::<String>::new()
        );
    }
}

#[cfg(test)]
fn installed_from_command_result(result: Result<String>) -> Result<bool> {
    match result {
        Ok(_) => Ok(true),
        Err(AppError::CommandFailed { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
fn executables_from_command_result(result: Result<String>) -> Result<Vec<String>> {
    match result {
        Ok(output) => Ok(parser::parse_executables(&output)),
        Err(AppError::CommandFailed { .. }) => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}
