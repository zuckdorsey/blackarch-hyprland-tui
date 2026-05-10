use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolStatus {
    Installed,
    NotInstalled,
    UpdateAvailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlackArchTool {
    pub name: String,
    pub package_name: String,
    pub category: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub executable: Option<String>,
    pub executables: Vec<String>,
    pub status: ToolStatus,
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageInfo {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub groups: Vec<String>,
    pub installed: bool,
    pub executables: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlackArchCategory {
    pub name: String,
    pub tool_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub ui: UiConfig,
    pub pacman: PacmanConfig,
    pub terminal: TerminalConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiConfig {
    pub theme: String,
    pub show_icons: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PacmanConfig {
    pub prefer_cache: bool,
    pub sync_on_start: bool,
    pub max_package_info_jobs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TerminalConfig {
    pub program: String,
    pub runner_class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheMetadata {
    pub generated_at: String,
    pub tool_count: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ui: UiConfig {
                theme: "catppuccin-mocha".to_string(),
                show_icons: true,
            },
            pacman: PacmanConfig {
                prefer_cache: true,
                sync_on_start: false,
                max_package_info_jobs: 8,
            },
            terminal: TerminalConfig {
                program: "kitty".to_string(),
                runner_class: "blackarch-tool-runner".to_string(),
            },
        }
    }
}
