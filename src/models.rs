use serde::{Deserialize, Serialize};

use crate::utils::validate::validate_package_name;

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
    pub category: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub executable: Option<String>,
    pub executables: Vec<String>,
    pub status: ToolStatus,
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageSearchResult {
    pub repository: String,
    pub name: String,
    pub version: Option<String>,
    pub groups: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageInfo {
    pub repository: Option<String>,
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub groups: Vec<String>,
    pub licenses: Vec<String>,
    pub installed: bool,
    pub executables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UserState {
    pub favorites: Vec<String>,
    pub recent: Vec<RecentTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecentTool {
    pub package_name: String,
    #[serde(default)]
    pub executable: Option<String>,
    pub last_used: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionMenuItem {
    RunInTerminal,
    InstallOrUpdate,
    Remove,
    ToggleFavorite,
    CopyCommand,
    RefreshDetails,
    PackageInfo,
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionMenuState {
    pub visible: bool,
    pub selected_index: usize,
    pub items: Vec<ActionMenuItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConfirmAction {
    InstallPackages { packages: Vec<String> },
    RemovePackage { package_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct InstallQueue {
    pub packages: Vec<String>,
}

impl InstallQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, package_name: String) -> bool {
        if !validate_package_name(&package_name) || self.contains(&package_name) {
            return false;
        }
        self.packages.push(package_name);
        true
    }

    pub fn remove(&mut self, package_name: &str) -> bool {
        if let Some(index) = self
            .packages
            .iter()
            .position(|package| package == package_name)
        {
            self.packages.remove(index);
            true
        } else {
            false
        }
    }

    pub fn toggle(&mut self, package_name: String) -> bool {
        if self.remove(&package_name) {
            false
        } else {
            self.add(package_name)
        }
    }

    pub fn contains(&self, package_name: &str) -> bool {
        self.packages.iter().any(|package| package == package_name)
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.packages.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }

    pub fn len(&self) -> usize {
        self.packages.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfirmModalState {
    pub visible: bool,
    pub title: String,
    pub message: String,
    pub command_preview: Option<String>,
    pub confirm_label: String,
    pub cancel_label: String,
    pub selected_confirm: bool,
    pub action: Option<ConfirmAction>,
}

impl Default for ActionMenuState {
    fn default() -> Self {
        Self {
            visible: false,
            selected_index: 0,
            items: vec![
                ActionMenuItem::RunInTerminal,
                ActionMenuItem::InstallOrUpdate,
                ActionMenuItem::Remove,
                ActionMenuItem::ToggleFavorite,
                ActionMenuItem::CopyCommand,
                ActionMenuItem::RefreshDetails,
                ActionMenuItem::PackageInfo,
                ActionMenuItem::Cancel,
            ],
        }
    }
}

impl Default for ConfirmModalState {
    fn default() -> Self {
        Self {
            visible: false,
            title: String::new(),
            message: String::new(),
            command_preview: None,
            confirm_label: "Confirm".to_string(),
            cancel_label: "Cancel".to_string(),
            selected_confirm: false,
            action: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PasswordInputModalState {
    pub visible: bool,
    pub title: String,
    pub message: String,
    pub password: String,
    pub confirm_label: String,
    pub cancel_label: String,
    pub action: Option<ConfirmAction>,
}

impl Default for PasswordInputModalState {
    fn default() -> Self {
        Self {
            visible: false,
            title: String::new(),
            message: String::new(),
            password: String::new(),
            confirm_label: "OK".to_string(),
            cancel_label: "Cancel".to_string(),
            action: None,
        }
    }
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
    #[serde(default)]
    pub hold_after_run: bool,
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
                hold_after_run: false,
            },
        }
    }
}

// #[test]
// fn install_queue_removes_package() {
//     let mut queue = InstallQueue::new();
//     assert!(queue.add("sqlmap".to_string()));
//     assert!(queue.remove("sqlmap"));
//     assert!(queue.is_empty());
// }

// #[test]
// fn install_queue_toggles_package() {
//     let mut queue = InstallQueue::new();
//     assert!(queue.toggle("sqlmap".to_string()));
//     assert!(queue.contains("sqlmap"));
//     assert!(!queue.toggle("sqlmap".to_string()));
//     assert!(!queue.contains("sqlmap"));
// }

// #[test]
// fn install_queue_rejects_invalid_package() {
//     let mut queue = InstallQueue::new();
//     assert!(!queue.add("bad/name".to_string()));
//     assert!(queue.is_empty());
// }
