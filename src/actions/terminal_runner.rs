use std::process::{Command, Stdio};

use crate::{
    error::{AppError, Result},
    utils::validate::{
        validate_executable_name, validate_terminal_class, validate_terminal_program,
    },
};

pub fn terminal_exists(program: &str) -> bool {
    validate_terminal_program(program)
        && Command::new(program)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .is_ok()
}

pub fn build_terminal_command(
    program: &str,
    class_name: &str,
    executable: &str,
) -> Result<Command> {
    validate_terminal_args(program, class_name, executable)?;

    let mut command = Command::new(program);
    match program {
        "kitty" => {
            command.args(["--class", class_name, "-e", executable]);
        }
        "foot" => {
            command.args(["--app-id", class_name, executable]);
        }
        "alacritty" => {
            command.args(["--class", class_name, "-e", executable]);
        }
        "wezterm" => {
            command.args(["start", "--class", class_name, "--", executable]);
        }
        _ => return Err(AppError::Config(format!("unsupported terminal: {program}"))),
    }

    Ok(command)
}

pub fn run_in_terminal(program: &str, class_name: &str, executable: &str) -> Result<()> {
    validate_terminal_args(program, class_name, executable)?;

    if !terminal_exists(program) {
        return Err(AppError::Config(format!(
            "terminal program not found in PATH: {program}"
        )));
    }

    build_terminal_command(program, class_name, executable)?.spawn()?;
    Ok(())
}

fn validate_terminal_args(program: &str, class_name: &str, executable: &str) -> Result<()> {
    if !validate_terminal_program(program) {
        return Err(AppError::Config(format!("unsupported terminal: {program}")));
    }

    if !validate_terminal_class(class_name) {
        return Err(AppError::Config(format!(
            "invalid terminal class/app-id: {class_name}"
        )));
    }

    if !validate_executable_name(executable) {
        return Err(AppError::InvalidPackageName(executable.to_string()));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_kitty_args() {
        assert_command_args(
            build_terminal_command("kitty", "blackarch-tool-runner", "sqlmap").unwrap(),
            "kitty",
            &["--class", "blackarch-tool-runner", "-e", "sqlmap"],
        );
    }

    #[test]
    fn builds_foot_args() {
        assert_command_args(
            build_terminal_command("foot", "blackarch-tool-runner", "sqlmap").unwrap(),
            "foot",
            &["--app-id", "blackarch-tool-runner", "sqlmap"],
        );
    }

    #[test]
    fn builds_alacritty_args() {
        assert_command_args(
            build_terminal_command("alacritty", "blackarch-tool-runner", "sqlmap").unwrap(),
            "alacritty",
            &["--class", "blackarch-tool-runner", "-e", "sqlmap"],
        );
    }

    #[test]
    fn builds_wezterm_args() {
        assert_command_args(
            build_terminal_command("wezterm", "blackarch-tool-runner", "sqlmap").unwrap(),
            "wezterm",
            &["start", "--class", "blackarch-tool-runner", "--", "sqlmap"],
        );
    }

    #[test]
    fn rejects_unsupported_terminal() {
        assert!(build_terminal_command("xterm", "runner", "sqlmap").is_err());
    }

    fn assert_command_args(command: Command, program: &str, args: &[&str]) {
        assert_eq!(command.get_program().to_string_lossy(), program);
        assert_eq!(
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            args
        );
    }
}
