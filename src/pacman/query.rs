use std::collections::HashSet;

use crate::{
    error::{AppError, Result},
    models::{BlackArchTool, PackageInfo, ToolStatus},
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

pub fn list_all_blackarch_tools() -> Result<Vec<(String, String)>> {
    let mut seen = HashSet::new();
    let mut tools = Vec::new();

    for category in list_blackarch_categories()? {
        for tool in list_tools_by_category(&category)? {
            if seen.insert(tool.clone()) {
                tools.push((tool, category.clone()));
            }
        }
    }

    Ok(tools)
}

pub fn get_package_info(package_name: &str) -> Result<PackageInfo> {
    if !validate_package_name(package_name) {
        return Err(AppError::InvalidPackageName(package_name.to_string()));
    }

    let output = command::run_pacman_owned(vec!["-Si".to_string(), package_name.to_string()])?;
    let mut info = parser::parse_package_info(&output);
    info.installed = is_installed(package_name)?;
    info.executables = get_executables(package_name)?;
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
        Err(error) => Err(error),
    }
}

pub fn get_executables(package_name: &str) -> Result<Vec<String>> {
    if !validate_package_name(package_name) {
        return Err(AppError::InvalidPackageName(package_name.to_string()));
    }

    match command::run_pacman_owned(vec!["-Ql".to_string(), package_name.to_string()]) {
        Ok(output) => Ok(parser::parse_executables(&output)),
        Err(AppError::CommandFailed { stderr, .. })
            if stderr.contains("was not found") || stderr.contains("not found") =>
        {
            Ok(Vec::new())
        }
        Err(error) => Err(error),
    }
}

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
        category: category.to_string(),
        version: info.version,
        description: info.description,
        executable,
        executables: info.executables,
        status,
        favorite: false,
    })
}
