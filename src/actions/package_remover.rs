use std::process::Command;

use crate::{
    actions::privilege,
    error::{AppError, Result},
    utils::validate::validate_package_name,
};

pub struct RemoveResult {
    pub package_name: String,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

pub fn build_remove_args(package_name: &str) -> Result<Vec<String>> {
    if !validate_package_name(package_name) {
        return Err(AppError::InvalidPackageName(package_name.to_string()));
    }

    Ok(vec![
        "pacman".to_string(),
        "-Rns".to_string(),
        "--noconfirm".to_string(),
        package_name.to_string(),
    ])
}

pub fn remove_package(package_name: &str) -> Result<RemoveResult> {
    let args = build_remove_args(package_name)?;

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

    if success {
        return Ok(RemoveResult {
            package_name: package_name.to_string(),
            stdout,
            stderr,
            success,
        });
    }

    let readable_stderr = stderr.trim();
    let message = if readable_stderr.is_empty()
        || readable_stderr.contains("Authentication is needed")
        || readable_stderr.contains("dismissed")
        || readable_stderr.contains("cancel")
        || readable_stderr.contains("not authorized")
    {
        "Remove cancelled or authentication failed".to_string()
    } else {
        format!(
            "pacman remove failed with status {}: {readable_stderr}",
            output.status
        )
    };

    Err(AppError::CommandFailed {
        command: format!("pkexec {}", args.join(" ")),
        status: output.status.to_string(),
        stderr: message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_remove_args_for_one_package() {
        assert_eq!(
            build_remove_args("cai").unwrap(),
            vec!["pacman", "-Rns", "--noconfirm", "cai"]
        );
    }

    #[test]
    fn rejects_invalid_package_with_semicolon() {
        assert!(build_remove_args("bad;name").is_err());
    }

    #[test]
    fn rejects_invalid_package_with_slash() {
        assert!(build_remove_args("bad/name").is_err());
    }

    #[test]
    fn rejects_invalid_package_with_space() {
        assert!(build_remove_args("bad name").is_err());
    }

    #[test]
    fn rejects_empty_package_name() {
        assert!(build_remove_args("").is_err());
    }

    #[test]
    fn remove_args_do_not_contain_forbidden_tokens() {
        let args = build_remove_args("cai").unwrap();
        for forbidden in ["sudo", "pkexec", "sh", "bash", "-S", "--overwrite"] {
            assert!(!args.iter().any(|arg| arg == forbidden));
        }
    }

    #[test]
    fn remove_args_contain_required_tokens_once() {
        let args = build_remove_args("cai").unwrap();
        assert_eq!(
            args.iter().filter(|arg| arg.as_str() == "pacman").count(),
            1
        );
        assert_eq!(args.iter().filter(|arg| arg.as_str() == "-Rns").count(), 1);
        assert!(args.iter().any(|arg| arg == "--noconfirm"));
    }
}
