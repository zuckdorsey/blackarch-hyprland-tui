use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::{
    error::{AppError, Result},
    utils::validate::validate_package_name,
};

pub fn clipboard_available() -> bool {
    command_exists("wl-copy") || command_exists("xclip")
}

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    if !validate_package_name(text) {
        return Err(AppError::InvalidPackageName(text.to_string()));
    }

    if command_exists("wl-copy") {
        return copy_with_command("wl-copy", &[], text);
    }

    if command_exists("xclip") {
        return copy_with_command("xclip", &["-selection", "clipboard"], text);
    }

    Err(AppError::Config(
        "No clipboard tool found. Install wl-clipboard.".to_string(),
    ))
}

fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .is_ok()
}

fn copy_with_command(command: &str, args: &[&str], text: &str) -> Result<()> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }

    let output = child.wait_with_output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(AppError::CommandFailed {
            command: command.to_string(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        })
    }
}
