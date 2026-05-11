use std::process::Command;

use crate::{
    actions::privilege,
    error::{AppError, Result},
    utils::validate::validate_package_name,
};

pub struct InstallResult {
    pub packages: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

#[allow(dead_code)]
pub fn build_install_command(package_name: &str) -> Result<Command> {
    build_install_command_for_packages(&[package_name.to_string()])
}

pub fn build_install_args(package_names: &[String]) -> Result<Vec<String>> {
    if package_names.is_empty() {
        return Err(AppError::Config(
            "No packages selected for installation.".to_string(),
        ));
    }

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    let mut deduplicated = Vec::new();
    for package in package_names {
        if !validate_package_name(package) {
            return Err(AppError::InvalidPackageName(package.to_string()));
        }
        if seen.insert(package.clone()) {
            deduplicated.push(package.clone());
        }
    }

    let mut args = vec![
        "pacman".to_string(),
        "-S".to_string(),
        "--needed".to_string(),
        "--noconfirm".to_string(),
    ];
    args.extend(deduplicated);

    Ok(args)
}

#[allow(dead_code)]
pub fn build_install_command_for_packages(package_names: &[String]) -> Result<Command> {
    let args = build_install_args(package_names)?;

    let mut command = Command::new("pkexec");
    command.args(args);
    Ok(command)
}

#[allow(dead_code)]
pub fn install_package(package_name: &str) -> Result<InstallResult> {
    install_packages(&[package_name.to_string()])
}

pub fn install_packages(package_names: &[String]) -> Result<InstallResult> {
    let args = build_install_args(package_names)?;

    if !privilege::pkexec_exists() {
        return Err(AppError::Config(
            "pkexec not found. Install polkit and make sure a polkit authentication agent is running."
                .to_string(),
        ));
    }

    let output = Command::new("pkexec").args(&args).output()?;

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(InstallResult {
        packages: package_names.to_vec(),
        stdout,
        stderr,
        success,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_pkexec_install_command_args() {
        let command = build_install_command("sqlmap").unwrap();
        assert_eq!(command.get_program().to_string_lossy(), "pkexec");
        assert_eq!(
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            vec!["pacman", "-S", "--needed", "--noconfirm", "sqlmap"]
        );
    }

    #[test]
    fn rejects_invalid_package_name() {
        assert!(build_install_command("bad/name").is_err());
        assert!(build_install_command("bad;name").is_err());
    }

    #[test]
    fn builds_batch_install_args() {
        assert_eq!(
            build_install_args(&["sqlmap".to_string(), "nmap".to_string()]).unwrap(),
            vec!["pacman", "-S", "--needed", "--noconfirm", "sqlmap", "nmap"]
        );
    }

    #[test]
    fn rejects_empty_package_list() {
        assert!(build_install_args(&[]).is_err());
    }

    #[test]
    fn deduplicates_package_names_while_preserving_order() {
        assert_eq!(
            build_install_args(&[
                "sqlmap".to_string(),
                "nmap".to_string(),
                "sqlmap".to_string()
            ])
            .unwrap(),
            vec!["pacman", "-S", "--needed", "--noconfirm", "sqlmap", "nmap"]
        );
    }

    #[test]
    fn ignore_unknown_lines() {
        // Removed progress parsing tests
    }
}
