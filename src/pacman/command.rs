use std::process::Command;

use crate::error::{AppError, Result};

pub fn pacman_exists() -> bool {
    Command::new("pacman").arg("--version").output().is_ok()
}

pub fn run_pacman(args: &[&str]) -> Result<String> {
    if !pacman_exists() {
        return Err(AppError::PacmanNotFound);
    }

    run_command(args.iter().copied())
}

pub fn run_pacman_owned(args: Vec<String>) -> Result<String> {
    if !pacman_exists() {
        return Err(AppError::PacmanNotFound);
    }

    run_command(args.iter().map(String::as_str))
}

fn run_command<'a>(args: impl IntoIterator<Item = &'a str>) -> Result<String> {
    let args = args.into_iter().collect::<Vec<_>>();
    let output = Command::new("pacman").args(&args).output()?;

    if !output.status.success() {
        return Err(AppError::CommandFailed {
            command: format!("pacman {}", args.join(" ")),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
