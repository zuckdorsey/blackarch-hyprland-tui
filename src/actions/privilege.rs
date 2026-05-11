use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivilegeStatus {
    pub pkexec_found: bool,
    pub polkit_agent_detected: bool,
    pub detected_agent: Option<String>,
    pub warnings: Vec<String>,
}

const POLKIT_AGENTS: [&str; 5] = [
    "hyprpolkitagent",
    "polkit-kde-authentication-agent-1",
    "polkit-gnome-authentication-agent-1",
    "lxqt-policykit-agent",
    "xfce-polkit",
];

pub fn pkexec_exists() -> bool {
    command_exists("pkexec")
}

pub fn detect_polkit_agent() -> Option<String> {
    POLKIT_AGENTS
        .iter()
        .find(|agent| process_exists(agent))
        .map(|agent| (*agent).to_string())
}

pub fn check_privilege_status() -> PrivilegeStatus {
    let pkexec_found = pkexec_exists();
    let detected_agent = detect_polkit_agent();
    let polkit_agent_detected = detected_agent.is_some();
    let mut warnings = Vec::new();

    if !pkexec_found {
        warnings.push(
            "pkexec not found. Install polkit or configure a supported privilege prompt."
                .to_string(),
        );
    }

    if !polkit_agent_detected {
        warnings.push(
            "No polkit authentication agent detected. pkexec may not show a password prompt."
                .to_string(),
        );
        warnings.push("Hint: add `exec-once = hyprpolkitagent` to hyprland.conf".to_string());
    }

    PrivilegeStatus {
        pkexec_found,
        polkit_agent_detected,
        detected_agent,
        warnings,
    }
}

fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .is_ok()
}

fn process_exists(process_name: &str) -> bool {
    Command::new("pgrep")
        .args(["-x", process_name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|output| output.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_agent_status_contains_warning() {
        let status = PrivilegeStatus {
            pkexec_found: false,
            polkit_agent_detected: false,
            detected_agent: None,
            warnings: vec![
                "No polkit authentication agent detected. pkexec may not show a password prompt."
                    .to_string(),
            ],
        };

        assert!(!status.pkexec_found);
        assert!(!status.polkit_agent_detected);
        assert!(!status.warnings.is_empty());
    }
}
