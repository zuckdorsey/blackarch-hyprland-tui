use std::{
    io,
    sync::mpsc,
    time::{Duration, Instant},
};

use crossterm::{
    event as crossterm_event, execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    actions::{
        action_menu, clipboard,
        privilege::{self, PrivilegeStatus},
    },
    error::{AppError, Result},
    event,
    models::{
        ActionMenuItem, ActionMenuState, BlackArchTool, ConfirmAction, ConfirmModalState,
        InstallQueue, PasswordInputModalState, RecentTool, ToolStatus, UserState,
    },
    ui,
    user_state::store as user_store,
    utils::validate::validate_package_name,
    worker::{WorkerCommand, WorkerEvent, start_worker},
};

const VIRTUAL_CATEGORIES: [&str; 4] = ["All Tools", "Installed", "Favorites", "Recent"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusPane {
    Categories,
    Tools,
    Details,
    Search,
}

pub struct App {
    pub tools: Vec<BlackArchTool>,
    pub filtered_tools: Vec<BlackArchTool>,
    pub categories: Vec<String>,
    pub selected_tool_index: usize,
    pub selected_category_index: usize,
    pub search_query: String,
    pub focus: FocusPane,
    pub loading: bool,
    pub detail_loading: bool,
    pub error_message: Option<String>,
    pub status_message: String,
    pub current_detail_request_id: u64,
    pub next_request_id: u64,
    pub pending_detail_package: Option<String>,
    pub last_selection_change: Option<Instant>,
    pub detail_debounce_ms: u64,
    pub user_state: UserState,
    pub action_menu: ActionMenuState,
    pub confirm_modal: ConfirmModalState,
    pub password_input_modal: PasswordInputModalState,
    pub install_queue: InstallQueue,
    pub install_queue_modal_visible: bool,
    pub install_queue_selected_index: usize,
    pub install_in_progress: bool,
    pub current_install_request_id: Option<u64>,
    pub current_remove_request_id: Option<u64>,
    pub privilege_status: Option<PrivilegeStatus>,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            filtered_tools: Vec::new(),
            categories: VIRTUAL_CATEGORIES
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            selected_tool_index: 0,
            selected_category_index: 0,
            search_query: String::new(),
            focus: FocusPane::Tools,
            loading: false,
            detail_loading: false,
            error_message: None,
            status_message: "Ready".to_string(),
            current_detail_request_id: 0,
            next_request_id: 1,
            pending_detail_package: None,
            last_selection_change: None,
            detail_debounce_ms: 200,
            user_state: UserState::default(),
            action_menu: action_menu::default_state(),
            confirm_modal: ConfirmModalState::default(),
            password_input_modal: PasswordInputModalState::default(),
            install_queue: InstallQueue::new(),
            install_queue_modal_visible: false,
            install_queue_selected_index: 0,
            install_in_progress: false,
            current_install_request_id: None,
            current_remove_request_id: None,
            privilege_status: None,
            should_quit: false,
        }
    }

    pub fn load_user_state(&mut self) -> Result<()> {
        self.user_state = user_store::load_user_state()?;
        self.apply_user_state_to_tools();
        Ok(())
    }

    #[allow(dead_code)]
    pub fn save_user_state(&self) -> Result<()> {
        user_store::save_user_state(&self.user_state)
    }

    pub fn begin_initial_load(&mut self, worker_tx: &mpsc::Sender<WorkerCommand>) {
        self.loading = true;
        self.status_message = "Loading BlackArch tools...".to_string();
        self.error_message = None;
        self.refresh_privilege_status();
        let _ = worker_tx.send(WorkerCommand::LoadInitialData);
    }

    pub fn begin_sync_cache(&mut self, worker_tx: &mpsc::Sender<WorkerCommand>) {
        self.loading = true;
        self.status_message = "Syncing cache...".to_string();
        self.error_message = None;
        let _ = worker_tx.send(WorkerCommand::SyncBasicCache);
    }

    pub fn apply_filters(&mut self) {
        let selected_package = self.selected_tool().map(|tool| tool.package_name.clone());
        let selected_category = self
            .categories
            .get(self.selected_category_index)
            .map(String::as_str)
            .unwrap_or("All Tools");
        let query = self.search_query.to_lowercase();

        self.filtered_tools = self
            .tools
            .iter()
            .filter(|tool| self.matches_selected_category(tool, selected_category))
            .filter(|tool| matches_search(tool, &query))
            .cloned()
            .collect();

        if selected_category == "Recent" {
            let recent_order = self
                .user_state
                .recent
                .iter()
                .map(|item| item.package_name.clone())
                .collect::<Vec<_>>();
            self.filtered_tools.sort_by_key(|tool| {
                recent_order
                    .iter()
                    .position(|package_name| package_name == &tool.package_name)
                    .unwrap_or(usize::MAX)
            });
        }

        if let Some(package_name) = selected_package {
            if let Some(index) = self
                .filtered_tools
                .iter()
                .position(|tool| tool.package_name == package_name)
            {
                self.selected_tool_index = index;
                return;
            }
        }

        if self.filtered_tools.is_empty() {
            self.selected_tool_index = 0;
        } else {
            self.selected_tool_index = self
                .selected_tool_index
                .min(self.filtered_tools.len().saturating_sub(1));
        }
    }

    pub fn selected_tool(&self) -> Option<&BlackArchTool> {
        self.filtered_tools.get(self.selected_tool_index)
    }

    pub fn select_next_tool(&mut self) {
        if self.filtered_tools.is_empty() {
            self.selected_tool_index = 0;
            return;
        }

        self.selected_tool_index = (self.selected_tool_index + 1) % self.filtered_tools.len();
        self.mark_selected_tool_pending_detail();
    }

    pub fn select_previous_tool(&mut self) {
        if self.filtered_tools.is_empty() {
            self.selected_tool_index = 0;
            return;
        }

        self.selected_tool_index = if self.selected_tool_index == 0 {
            self.filtered_tools.len() - 1
        } else {
            self.selected_tool_index - 1
        };
        self.mark_selected_tool_pending_detail();
    }

    pub fn select_next_category(&mut self) {
        if self.categories.is_empty() {
            self.selected_category_index = 0;
            return;
        }

        self.selected_category_index = (self.selected_category_index + 1) % self.categories.len();
        self.apply_filters();
        self.mark_selected_tool_pending_detail();
    }

    pub fn select_previous_category(&mut self) {
        if self.categories.is_empty() {
            self.selected_category_index = 0;
            return;
        }

        self.selected_category_index = if self.selected_category_index == 0 {
            self.categories.len() - 1
        } else {
            self.selected_category_index - 1
        };
        self.apply_filters();
        self.mark_selected_tool_pending_detail();
    }

    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        self.apply_filters();
        self.mark_selected_tool_pending_detail();
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    pub fn focus_next(&mut self) {
        self.focus = match self.focus {
            FocusPane::Categories => FocusPane::Tools,
            FocusPane::Tools => FocusPane::Details,
            FocusPane::Details | FocusPane::Search => FocusPane::Categories,
        };
    }

    pub fn enter_search(&mut self) {
        self.focus = FocusPane::Search;
        self.status_message = "Search mode • Esc/Enter/Tab to exit".to_string();
    }

    pub fn exit_search(&mut self) {
        if self.focus == FocusPane::Search {
            self.focus = FocusPane::Tools;
            self.status_message = "Ready".to_string();
        }
    }

    pub fn push_search_char(&mut self, ch: char) {
        let mut query = self.search_query.clone();
        query.push(ch);
        self.set_search_query(query);
    }

    pub fn pop_search_char(&mut self) {
        let mut query = self.search_query.clone();
        query.pop();
        self.set_search_query(query);
    }

    pub fn toggle_selected_favorite(&mut self) -> Result<()> {
        let Some(package_name) = self.selected_tool().map(|tool| tool.package_name.clone()) else {
            self.status_message = "No tool selected".to_string();
            return Ok(());
        };

        let added = user_store::toggle_favorite(&package_name)?;
        self.user_state.favorites = user_store::load_favorites()?;
        self.set_tool_favorite(&package_name, added);
        self.status_message = if added {
            format!("Added {package_name} to favorites")
        } else {
            format!("Removed {package_name} from favorites")
        };

        self.apply_filters();
        Ok(())
    }

    pub fn is_selected_tool_favorite(&self) -> bool {
        self.selected_tool().is_some_and(|tool| tool.favorite)
    }

    pub fn open_action_menu(&mut self) {
        if self.selected_tool().is_none() {
            self.status_message = "No tool selected".to_string();
            return;
        }

        self.action_menu.visible = true;
        self.action_menu.selected_index = 0;
    }

    pub fn close_action_menu(&mut self) {
        self.action_menu.visible = false;
    }

    pub fn select_next_action(&mut self) {
        if self.action_menu.items.is_empty() {
            self.action_menu.selected_index = 0;
            return;
        }
        self.action_menu.selected_index =
            (self.action_menu.selected_index + 1) % self.action_menu.items.len();
    }

    pub fn select_previous_action(&mut self) {
        if self.action_menu.items.is_empty() {
            self.action_menu.selected_index = 0;
            return;
        }
        self.action_menu.selected_index = if self.action_menu.selected_index == 0 {
            self.action_menu.items.len() - 1
        } else {
            self.action_menu.selected_index - 1
        };
    }

    pub fn selected_action(&self) -> Option<ActionMenuItem> {
        self.action_menu
            .items
            .get(self.action_menu.selected_index)
            .copied()
    }

    pub fn execute_selected_action(
        &mut self,
        worker_tx: &mpsc::Sender<WorkerCommand>,
    ) -> Result<()> {
        match self.selected_action() {
            Some(ActionMenuItem::RunInTerminal) => {
                self.close_action_menu();
                self.run_selected_tool(worker_tx)?;
            }
            Some(ActionMenuItem::InstallOrUpdate) => {
                self.close_action_menu();
                self.open_install_confirmation_for_selected_package();
            }
            Some(ActionMenuItem::Remove) => {
                self.close_action_menu();
                self.open_remove_confirmation_for_selected_package();
            }
            Some(ActionMenuItem::ToggleFavorite) => {
                self.close_action_menu();
                self.toggle_selected_favorite()?;
            }
            Some(ActionMenuItem::CopyCommand) => {
                self.close_action_menu();
                self.copy_selected_command()?;
            }
            Some(ActionMenuItem::RefreshDetails) => {
                self.close_action_menu();
                self.refresh_selected_tool_detail(worker_tx);
            }
            Some(ActionMenuItem::PackageInfo) => {
                self.close_action_menu();
                self.status_message = "Package info is shown in the details pane".to_string();
            }
            Some(ActionMenuItem::Cancel) | None => self.close_action_menu(),
        }

        Ok(())
    }

    pub fn copy_selected_command(&mut self) -> Result<()> {
        let Some(command) = self.selected_command() else {
            self.status_message = "No command available".to_string();
            return Ok(());
        };

        if !clipboard::clipboard_available() {
            return Err(crate::error::AppError::Config(
                "No clipboard tool found. Install wl-clipboard.".to_string(),
            ));
        }

        clipboard::copy_to_clipboard(&command)?;
        self.status_message = format!("Copied command: {command}");
        Ok(())
    }

    pub fn run_selected_tool(&mut self, worker_tx: &mpsc::Sender<WorkerCommand>) -> Result<()> {
        let Some(package_name) = self.selected_tool().map(|tool| tool.package_name.clone()) else {
            self.status_message = "No tool selected".to_string();
            return Ok(());
        };
        let executable = self.selected_tool_executable()?;
        let request_id = self.next_request_id();

        self.status_message = format!("Launching {executable}...");
        let _ = worker_tx.send(WorkerCommand::RunTool {
            package_name,
            executable,
            request_id,
        });
        Ok(())
    }

    pub fn toggle_selected_package_in_queue(&mut self) -> Result<()> {
        let Some(package_name) = self.selected_tool().map(|tool| tool.package_name.clone()) else {
            self.status_message = "No package selected".to_string();
            return Ok(());
        };

        self.validate_install_package(&package_name)?;
        let added = self.install_queue.toggle(package_name.clone());
        self.status_message = if added {
            format!("Added {package_name} to install queue")
        } else {
            format!("Removed {package_name} from install queue")
        };
        Ok(())
    }

    #[allow(dead_code)]
    pub fn add_selected_package_to_queue(&mut self) -> Result<()> {
        let Some(package_name) = self.selected_tool().map(|tool| tool.package_name.clone()) else {
            self.status_message = "No package selected".to_string();
            return Ok(());
        };

        self.validate_install_package(&package_name)?;
        if self.install_queue.add(package_name.clone()) {
            self.status_message = format!("Added {package_name} to install queue");
        } else {
            self.status_message = format!("{package_name} is already queued");
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove_selected_package_from_queue(&mut self) -> Result<()> {
        let Some(package_name) = self.selected_tool().map(|tool| tool.package_name.clone()) else {
            self.status_message = "No package selected".to_string();
            return Ok(());
        };

        if self.install_queue.remove(&package_name) {
            self.status_message = format!("Removed {package_name} from install queue");
        } else {
            self.status_message = format!("{package_name} is not queued");
        }
        Ok(())
    }

    pub fn open_install_queue_modal(&mut self) {
        if self.install_queue.is_empty() {
            self.status_message = "Install queue is empty".to_string();
            return;
        }

        self.install_queue_modal_visible = true;
        self.install_queue_selected_index = self
            .install_queue_selected_index
            .min(self.install_queue.len().saturating_sub(1));
    }

    pub fn close_install_queue_modal(&mut self) {
        self.install_queue_modal_visible = false;
    }

    pub fn select_next_queue_item(&mut self) {
        if self.install_queue.is_empty() {
            self.install_queue_selected_index = 0;
            return;
        }
        self.install_queue_selected_index =
            (self.install_queue_selected_index + 1) % self.install_queue.len();
    }

    pub fn select_previous_queue_item(&mut self) {
        if self.install_queue.is_empty() {
            self.install_queue_selected_index = 0;
            return;
        }
        self.install_queue_selected_index = if self.install_queue_selected_index == 0 {
            self.install_queue.len() - 1
        } else {
            self.install_queue_selected_index - 1
        };
    }

    pub fn remove_selected_queue_item(&mut self) {
        let Some(package_name) = self
            .install_queue
            .packages
            .get(self.install_queue_selected_index)
            .cloned()
        else {
            self.status_message = "Install queue is empty".to_string();
            return;
        };

        self.install_queue.remove(&package_name);
        self.install_queue_selected_index = self
            .install_queue_selected_index
            .min(self.install_queue.len().saturating_sub(1));
        self.status_message = format!("Removed {package_name} from install queue");
    }

    pub fn open_install_confirmation_for_selected_package(&mut self) {
        let Some(package_name) = self.selected_tool().map(|tool| tool.package_name.clone()) else {
            self.status_message = "No package selected".to_string();
            return;
        };

        self.open_install_confirmation_for_packages(vec![package_name]);
    }

    pub fn open_remove_confirmation_for_selected_package(&mut self) {
        let Some(tool) = self.selected_tool().cloned() else {
            self.status_message = "No package selected".to_string();
            return;
        };
        let package_name = tool.package_name.clone();

        if self.install_in_progress {
            self.status_message = "Another package operation is already in progress".to_string();
            return;
        }

        if tool.status != ToolStatus::Installed {
            self.status_message = "Package is not installed".to_string();
            return;
        }

        if let Err(error) = self.validate_remove_package(&package_name) {
            let message = error.to_string();
            self.error_message = Some(message.clone());
            self.status_message = message;
            return;
        }

        self.refresh_privilege_status();
        self.confirm_modal = ConfirmModalState {
            visible: true,
            title: "Remove Package".to_string(),
            message: format!(
                "Package: {package_name}\n\nThis will remove the package and unused dependencies using pacman."
            ),
            command_preview: Some(format!("pkexec pacman -Rns --noconfirm {package_name}")),
            confirm_label: "Remove".to_string(),
            cancel_label: "Cancel".to_string(),
            selected_confirm: false,
            action: Some(ConfirmAction::RemovePackage { package_name }),
        };
    }

    pub fn open_install_confirmation_for_queue(&mut self) {
        if self.install_queue.is_empty() {
            self.status_message = "Install queue is empty".to_string();
            return;
        }

        self.open_install_confirmation_for_packages(self.install_queue.packages.clone());
    }

    #[allow(dead_code)]
    pub fn open_install_confirmation(&mut self) {
        self.open_install_confirmation_for_selected_package();
    }

    fn open_install_confirmation_for_packages(&mut self, packages: Vec<String>) {
        for package_name in &packages {
            if let Err(error) = self.validate_install_package(package_name) {
                self.error_message = Some(error.to_string());
                self.status_message = "Install validation failed".to_string();
                return;
            }
        }

        if packages.is_empty() {
            self.status_message = "Install queue is empty".to_string();
            return;
        }

        self.refresh_privilege_status();
        let title = if packages.len() == 1 {
            "Install Package".to_string()
        } else {
            "Install Packages".to_string()
        };
        let message = if packages.len() == 1 {
            format!(
                "Package: {}\n\nThis will install or update this package using pacman.\nA system authentication prompt may appear.",
                packages[0]
            )
        } else {
            format!(
                "Packages: {}\n\nThis will install or update all queued packages using pacman.\nA system authentication prompt may appear.",
                packages.len()
            )
        };
        let command_preview = format!(
            "pkexec pacman -S --needed --noconfirm {}",
            packages.join(" ")
        );

        self.confirm_modal = ConfirmModalState {
            visible: true,
            title,
            message,
            command_preview: Some(command_preview),
            confirm_label: "Install".to_string(),
            cancel_label: "Cancel".to_string(),
            selected_confirm: false,
            action: Some(ConfirmAction::InstallPackages { packages }),
        };
    }

    pub fn refresh_privilege_status(&mut self) {
        self.privilege_status = Some(privilege::check_privilege_status());
    }

    fn ensure_install_privileges_ready(&mut self) -> Result<()> {
        self.refresh_privilege_status();
        if self
            .privilege_status
            .as_ref()
            .is_some_and(|status| !status.pkexec_found)
        {
            return Err(AppError::Config(
                "pkexec not found. Install polkit and make sure a polkit authentication agent is running."
                    .to_string(),
            ));
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn open_install_confirmation_legacy(&mut self) {
        let Some(package_name) = self.selected_tool().map(|tool| tool.package_name.clone()) else {
            self.status_message = "No package selected".to_string();
            return;
        };

        if let Err(error) = self.validate_install_package(&package_name) {
            self.error_message = Some(error.to_string());
            self.status_message = "Install validation failed".to_string();
            return;
        }

        self.confirm_modal = ConfirmModalState {
            visible: true,
            title: "Install Package".to_string(),
            message: format!(
                "Package: {package_name}\n\nThis will install or update this package using pacman."
            ),
            command_preview: Some(format!(
                "pkexec pacman -S --needed --noconfirm {package_name}"
            )),
            confirm_label: "Install".to_string(),
            cancel_label: "Cancel".to_string(),
            selected_confirm: false,
            action: Some(ConfirmAction::InstallPackages {
                packages: vec![package_name],
            }),
        };
    }

    pub fn close_confirmation_modal(&mut self) {
        self.confirm_modal = ConfirmModalState::default();
    }

    pub fn toggle_confirmation_selection(&mut self) {
        self.confirm_modal.selected_confirm = !self.confirm_modal.selected_confirm;
    }

    pub fn confirm_modal_action(&mut self, worker_tx: &mpsc::Sender<WorkerCommand>) -> Result<()> {
        let action = self.confirm_modal.action.clone();
        if !self.confirm_modal.selected_confirm {
            self.close_confirmation_modal();
            self.status_message = cancel_message_for_action(action.as_ref()).to_string();
            return Ok(());
        }

        self.close_confirmation_modal();

        match action {
            Some(ConfirmAction::InstallPackages { packages }) => {
                self.start_install_packages(packages.clone(), worker_tx)?;
            }
            Some(ConfirmAction::RemovePackage { package_name }) => {
                self.start_remove_package(package_name, worker_tx)?;
            }
            None => self.status_message = "No install action selected".to_string(),
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn start_install_package(
        &mut self,
        package_name: String,
        worker_tx: &mpsc::Sender<WorkerCommand>,
    ) -> Result<()> {
        self.start_install_packages(vec![package_name], worker_tx)
    }

    pub fn start_install_packages(
        &mut self,
        packages: Vec<String>,
        worker_tx: &mpsc::Sender<WorkerCommand>,
    ) -> Result<()> {
        if self.install_in_progress {
            self.status_message = "Another package operation is already in progress".to_string();
            return Ok(());
        }

        if packages.is_empty() {
            self.status_message = "Install queue is empty".to_string();
            return Ok(());
        }

        self.ensure_install_privileges_ready()?;
        for package_name in &packages {
            self.validate_install_package(package_name)?;
        }

        let request_id = self.next_request_id();
        self.install_in_progress = true;
        self.current_install_request_id = Some(request_id);
        self.install_queue_modal_visible = false;
        self.error_message = None;
        self.status_message = "Waiting for authentication prompt...".to_string();

        let _ = worker_tx.send(WorkerCommand::InstallPackages {
            package_names: packages,
            request_id,
        });
        Ok(())
    }

    pub fn apply_install_success(
        &mut self,
        package_name: &str,
        refreshed_tool: Option<BlackArchTool>,
    ) {
        self.apply_batch_install_success(
            &[package_name.to_string()],
            refreshed_tool.into_iter().collect(),
        );
    }

    pub fn apply_batch_install_success(
        &mut self,
        packages: &[String],
        refreshed_tools: Vec<BlackArchTool>,
    ) {
        self.install_in_progress = false;
        self.current_install_request_id = None;
        self.error_message = None;

        let refreshed_count = refreshed_tools.len();
        for tool in refreshed_tools {
            let package_name = tool.package_name.clone();
            self.update_tool_record(&package_name, tool);
        }
        for package_name in packages {
            self.update_tool_status(package_name, ToolStatus::Installed);
            self.install_queue.remove(package_name);
        }
        self.install_queue_selected_index = self
            .install_queue_selected_index
            .min(self.install_queue.len().saturating_sub(1));
        if self.install_queue.is_empty() {
            self.install_queue_modal_visible = false;
        }

        self.apply_filters();
        let message = if refreshed_count < packages.len() {
            "Installed packages, but some details could not be refreshed".to_string()
        } else {
            format!("Installed {} package(s) successfully", packages.len())
        };
        self.status_message = message.clone();
    }

    pub fn apply_install_failure(&mut self, package_name: &str, error: String) {
        self.apply_batch_install_failure(&[package_name.to_string()], error);
    }

    pub fn apply_batch_install_failure(&mut self, packages: &[String], error: String) {
        self.install_in_progress = false;
        self.current_install_request_id = None;
        self.error_message = Some(error);
        self.status_message = if packages.len() == 1 {
            format!("Failed to install {}", packages[0])
        } else {
            "Install failed".to_string()
        };
    }

    pub fn start_remove_package(
        &mut self,
        package_name: String,
        worker_tx: &mpsc::Sender<WorkerCommand>,
    ) -> Result<()> {
        if self.install_in_progress {
            self.status_message = "Another package operation is already in progress".to_string();
            return Ok(());
        }

        self.validate_remove_package(&package_name)?;
        self.refresh_privilege_status();
        if self
            .privilege_status
            .as_ref()
            .is_some_and(|status| !status.pkexec_found)
        {
            return Err(AppError::Config(
                "pkexec not found. Install polkit and make sure a polkit authentication agent is running."
                    .to_string(),
            ));
        }

        let request_id = self.next_request_id();
        self.install_in_progress = true;
        self.current_remove_request_id = Some(request_id);
        self.error_message = None;
        self.status_message = "Waiting for authentication prompt...".to_string();

        let _ = worker_tx.send(WorkerCommand::RemovePackage {
            package_name,
            request_id,
        });
        Ok(())
    }

    pub fn apply_remove_success(
        &mut self,
        package_name: &str,
        mut refreshed_tool: BlackArchTool,
        refreshed: bool,
    ) {
        self.install_in_progress = false;
        self.current_remove_request_id = None;
        self.error_message = None;

        if let Some(existing) = self
            .tools
            .iter()
            .find(|tool| tool.package_name == package_name)
        {
            if refreshed_tool.category.is_none() {
                refreshed_tool.category = existing.category.clone();
            }
            if refreshed_tool.categories.is_empty() {
                refreshed_tool.categories = existing.categories.clone();
            }
        }

        self.update_tool_record(package_name, refreshed_tool);
        self.update_tool_status(package_name, ToolStatus::NotInstalled);
        self.clear_tool_executables(package_name);
        self.apply_filters();

        self.status_message = if refreshed {
            format!("Removed {package_name} successfully")
        } else {
            "Removed package, but details could not be refreshed".to_string()
        };
    }

    pub fn apply_remove_failure(&mut self, package_name: &str, error: String) {
        self.install_in_progress = false;
        self.current_remove_request_id = None;
        self.error_message = Some(error);
        self.status_message = format!("Failed to remove {package_name}");
    }

    pub fn selected_tool_executable(&self) -> Result<String> {
        let Some(tool) = self.selected_tool() else {
            return Err(AppError::Config("No tool selected".to_string()));
        };

        if tool.status != ToolStatus::Installed {
            return Err(AppError::Config("Package is not installed".to_string()));
        }

        if tool.version.is_none() && tool.executables.is_empty() && tool.executable.is_none() {
            return Err(AppError::Config(
                "Package details are not loaded yet".to_string(),
            ));
        }

        tool.executable
            .clone()
            .or_else(|| tool.executables.first().cloned())
            .ok_or_else(|| AppError::Config("No executable found for this package".to_string()))
    }

    pub fn next_request_id(&mut self) -> u64 {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        request_id
    }

    pub fn start_detail_loading(&mut self, package_name: String, request_id: u64) {
        self.current_detail_request_id = request_id;
        self.pending_detail_package = None;
        self.last_selection_change = None;
        self.detail_loading = true;
        self.status_message = format!("Loading details for {package_name}...");
    }

    pub fn apply_tool_detail_result(
        &mut self,
        package_name: &str,
        request_id: u64,
        tool: BlackArchTool,
    ) {
        if request_id != self.current_detail_request_id || !self.is_selected_package(package_name) {
            return;
        }

        self.detail_loading = false;
        self.status_message = format!("Loaded details for {package_name}");
        self.update_tool_record(package_name, tool);
    }

    pub fn apply_tool_detail_error(&mut self, package_name: &str, request_id: u64, error: String) {
        if request_id != self.current_detail_request_id || !self.is_selected_package(package_name) {
            return;
        }

        self.detail_loading = false;
        self.status_message = format!("Package info unavailable for {package_name}");
        let description = format!("Package info unavailable\n{error}");
        self.update_selected_description(package_name, description);
    }

    pub fn handle_worker_event(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::InitialDataLoaded { tools, categories } => {
                self.set_backend_data(categories, tools);
                self.apply_user_state_to_tools();
                self.apply_filters();
                self.loading = false;
                self.error_message = None;
                self.status_message = format!("Ready • {} tools loaded", self.tools.len());
                self.mark_selected_tool_pending_detail();
            }
            WorkerEvent::BasicCacheSynced { tools, categories } => {
                self.set_backend_data(categories, tools);
                self.apply_user_state_to_tools();
                self.apply_filters();
                self.loading = false;
                self.error_message = None;
                self.status_message = "Cache synced successfully".to_string();
                self.mark_selected_tool_pending_detail();
            }
            WorkerEvent::ToolDetailLoaded {
                package_name,
                request_id,
                tool,
            } => self.apply_tool_detail_result(&package_name, request_id, tool),
            WorkerEvent::ToolDetailFailed {
                package_name,
                request_id,
                error,
            } => self.apply_tool_detail_error(&package_name, request_id, error),
            WorkerEvent::SearchCompleted {
                query,
                request_id,
                results,
            } => {
                self.status_message =
                    format!("Search {query} ({request_id}): {} result(s)", results.len());
            }
            WorkerEvent::CacheCleared {
                package_name,
                request_id,
            } => {
                self.status_message =
                    format!("Cleared detail cache for {package_name} ({request_id})");
            }
            WorkerEvent::ToolRunStarted {
                package_name,
                executable,
                request_id,
            } => {
                self.status_message = format!("Launched {executable}");
                self.error_message = None;
                self.add_recent_in_memory(package_name, Some(executable));
                let _ = request_id;
                self.apply_filters();
            }
            WorkerEvent::ToolRunFailed {
                package_name,
                error,
                request_id,
            } => {
                self.status_message = format!("Failed to launch {package_name}");
                self.error_message = Some(error);
                let _ = request_id;
            }
            WorkerEvent::PackageInstallStarted {
                package_name,
                request_id,
            } => {
                if self.current_install_request_id == Some(request_id) {
                    self.status_message = format!("Installing {package_name}...");
                }
            }
            WorkerEvent::PackagesInstallStarted {
                package_names,
                request_id,
            } => {
                if self.current_install_request_id == Some(request_id) {
                    self.status_message =
                        format!("Installing {} package(s)...", package_names.len());
                }
            }
            WorkerEvent::PackageInstallFinished {
                package_name,
                request_id,
                refreshed_tool,
            } => {
                if self.current_install_request_id == Some(request_id) {
                    self.apply_install_success(&package_name, refreshed_tool);
                }
            }
            WorkerEvent::PackagesInstallFinished {
                package_names,
                request_id,
                refreshed_tools,
            } => {
                if self.current_install_request_id == Some(request_id) {
                    self.apply_batch_install_success(&package_names, refreshed_tools);
                }
            }
            WorkerEvent::PackageInstallFailed {
                package_name,
                request_id,
                error,
            } => {
                if self.current_install_request_id == Some(request_id) {
                    self.apply_install_failure(&package_name, error);
                }
            }
            WorkerEvent::PackagesInstallFailed {
                package_names,
                request_id,
                error,
            } => {
                if self.current_install_request_id == Some(request_id) {
                    self.apply_batch_install_failure(&package_names, error);
                }
            }
            WorkerEvent::PackageRemoveStarted {
                package_name,
                request_id,
            } => {
                if self.current_remove_request_id == Some(request_id) {
                    self.status_message = format!("Removing {package_name}...");
                }
            }
            WorkerEvent::PackageRemoveFinished {
                package_name,
                request_id,
                refreshed_tool,
                refreshed,
            } => {
                if self.current_remove_request_id == Some(request_id) {
                    self.apply_remove_success(&package_name, refreshed_tool, refreshed);
                }
            }
            WorkerEvent::PackageRemoveFailed {
                package_name,
                request_id,
                error,
            } => {
                if self.current_remove_request_id == Some(request_id) {
                    self.apply_remove_failure(&package_name, error);
                }
            }
            WorkerEvent::TaskFailed { label, error } => {
                self.loading = false;
                self.detail_loading = false;
                self.error_message = Some(error);
                self.status_message = format!("{label} failed");
            }
        }
    }

    pub fn maybe_dispatch_debounced_tasks(&mut self, worker_tx: &mpsc::Sender<WorkerCommand>) {
        let Some(package_name) = self.pending_detail_package.clone() else {
            return;
        };
        let Some(last_change) = self.last_selection_change else {
            return;
        };

        if last_change.elapsed() < Duration::from_millis(self.detail_debounce_ms) {
            return;
        }

        let request_id = self.next_request_id();
        self.start_detail_loading(package_name.clone(), request_id);
        let _ = worker_tx.send(WorkerCommand::LoadToolDetail {
            package_name,
            request_id,
        });
    }

    pub fn refresh_selected_tool_detail(&mut self, worker_tx: &mpsc::Sender<WorkerCommand>) {
        let Some(package_name) = self.selected_tool().map(|tool| tool.package_name.clone()) else {
            self.status_message = "No tool selected".to_string();
            return;
        };

        let request_id = self.next_request_id();
        self.start_detail_loading(package_name.clone(), request_id);
        let _ = worker_tx.send(WorkerCommand::RefreshToolDetail {
            package_name,
            request_id,
        });
    }

    fn set_backend_data(&mut self, backend_categories: Vec<String>, tools: Vec<BlackArchTool>) {
        self.categories = VIRTUAL_CATEGORIES
            .into_iter()
            .map(ToString::to_string)
            .chain(backend_categories)
            .collect();
        self.tools = tools;
        self.selected_category_index = self
            .selected_category_index
            .min(self.categories.len().saturating_sub(1));
        self.apply_filters();
    }

    fn mark_selected_tool_pending_detail(&mut self) {
        let Some(tool) = self.selected_tool() else {
            self.pending_detail_package = None;
            self.detail_loading = false;
            return;
        };

        if !tool_needs_detail(tool) {
            self.pending_detail_package = None;
            self.detail_loading = false;
            return;
        }

        self.pending_detail_package = Some(tool.package_name.clone());
        self.last_selection_change = Some(Instant::now());
        self.detail_loading = true;
    }

    fn is_selected_package(&self, package_name: &str) -> bool {
        self.selected_tool()
            .is_some_and(|tool| tool.package_name == package_name)
    }

    fn update_tool_record(&mut self, package_name: &str, tool: BlackArchTool) {
        if let Some(existing) = self
            .tools
            .iter_mut()
            .find(|tool| tool.package_name == package_name)
        {
            let favorite = existing.favorite;
            *existing = tool.clone();
            existing.favorite = favorite;
        }

        if let Some(existing) = self
            .filtered_tools
            .iter_mut()
            .find(|tool| tool.package_name == package_name)
        {
            let favorite = existing.favorite;
            *existing = tool;
            existing.favorite = favorite;
        }
    }

    fn update_selected_description(&mut self, package_name: &str, description: String) {
        if let Some(tool) = self
            .tools
            .iter_mut()
            .find(|tool| tool.package_name == package_name)
        {
            tool.description = Some(description.clone());
        }

        if let Some(tool) = self
            .filtered_tools
            .iter_mut()
            .find(|tool| tool.package_name == package_name)
        {
            tool.description = Some(description);
        }
    }

    fn validate_install_package(&self, package_name: &str) -> Result<()> {
        if !validate_package_name(package_name) {
            return Err(AppError::InvalidPackageName(package_name.to_string()));
        }

        if !self
            .tools
            .iter()
            .any(|tool| tool.package_name == package_name)
        {
            return Err(AppError::Config(
                "Package is not available in the BlackArch tool list".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_remove_package(&self, package_name: &str) -> Result<()> {
        if !validate_package_name(package_name) {
            return Err(AppError::InvalidPackageName(package_name.to_string()));
        }

        let Some(tool) = self
            .tools
            .iter()
            .find(|tool| tool.package_name == package_name)
        else {
            return Err(AppError::Config(
                "Package is not available in the BlackArch tool list".to_string(),
            ));
        };

        if tool.status != ToolStatus::Installed {
            return Err(AppError::Config("Package is not installed".to_string()));
        }

        Ok(())
    }

    fn update_tool_status(&mut self, package_name: &str, status: ToolStatus) {
        for tool in &mut self.tools {
            if tool.package_name == package_name {
                tool.status = status.clone();
            }
        }

        for tool in &mut self.filtered_tools {
            if tool.package_name == package_name {
                tool.status = status.clone();
            }
        }
    }

    fn clear_tool_executables(&mut self, package_name: &str) {
        for tool in &mut self.tools {
            if tool.package_name == package_name {
                tool.executable = None;
                tool.executables.clear();
            }
        }

        for tool in &mut self.filtered_tools {
            if tool.package_name == package_name {
                tool.executable = None;
                tool.executables.clear();
            }
        }
    }

    fn apply_user_state_to_tools(&mut self) {
        let favorites = self.user_state.favorites.clone();
        for tool in &mut self.tools {
            tool.favorite = favorites
                .iter()
                .any(|favorite| favorite == &tool.package_name);
        }
        for tool in &mut self.filtered_tools {
            tool.favorite = favorites
                .iter()
                .any(|favorite| favorite == &tool.package_name);
        }
    }

    fn set_tool_favorite(&mut self, package_name: &str, favorite: bool) {
        for tool in &mut self.tools {
            if tool.package_name == package_name {
                tool.favorite = favorite;
            }
        }
        for tool in &mut self.filtered_tools {
            if tool.package_name == package_name {
                tool.favorite = favorite;
            }
        }
    }

    fn selected_command(&self) -> Option<String> {
        self.selected_tool().map(|tool| {
            tool.executables
                .first()
                .cloned()
                .or_else(|| tool.executable.clone())
                .unwrap_or_else(|| tool.package_name.clone())
        })
    }

    fn add_recent_in_memory(&mut self, package_name: String, executable: Option<String>) {
        self.user_state
            .recent
            .retain(|item| item.package_name != package_name);
        self.user_state.recent.insert(
            0,
            RecentTool {
                package_name,
                executable,
                last_used: current_timestamp(),
            },
        );
        self.user_state.recent.truncate(50);
    }

    fn matches_selected_category(&self, tool: &BlackArchTool, selected_category: &str) -> bool {
        match selected_category {
            "Recent" => self
                .user_state
                .recent
                .iter()
                .any(|recent| recent.package_name == tool.package_name),
            category => matches_category(tool, category),
        }
    }
}

fn cancel_message_for_action(action: Option<&ConfirmAction>) -> &'static str {
    match action {
        Some(ConfirmAction::RemovePackage { .. }) => "Remove cancelled",
        _ => "Install cancelled",
    }
}

pub fn run() -> Result<()> {
    let mut app = App::new();
    let (worker_tx, worker_rx) = mpsc::channel::<WorkerCommand>();
    let (event_tx, event_rx) = mpsc::channel::<WorkerEvent>();
    let _worker = start_worker(worker_rx, event_tx);
    if let Err(error) = app.load_user_state() {
        app.error_message = Some(error.to_string());
        app.status_message = "Failed to load local user state".to_string();
    }
    app.begin_initial_load(&worker_tx);

    let mut terminal = init_terminal()?;
    terminal.draw(|frame| ui::render(frame, &app))?;

    let result = run_loop(&mut terminal, &mut app, &worker_tx, &event_rx);
    restore_terminal(&mut terminal)?;
    result
}

pub fn display_category_name(category: &str) -> String {
    let category = category.strip_prefix("blackarch-").unwrap_or(category);
    category
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn normalize_category_filter(category: &str) -> String {
    if category.starts_with("blackarch-") || VIRTUAL_CATEGORIES.contains(&category) {
        category.to_string()
    } else {
        format!("blackarch-{}", category.to_lowercase().replace(' ', "-"))
    }
}

fn matches_category(tool: &BlackArchTool, selected_category: &str) -> bool {
    match selected_category {
        "All Tools" => true,
        "Installed" => tool.status == ToolStatus::Installed,
        "Favorites" => tool.favorite,
        "Recent" => false,
        category => {
            let normalized = normalize_category_filter(category);
            tool.category.as_deref() == Some(normalized.as_str())
                || tool
                    .categories
                    .iter()
                    .any(|category| category == &normalized)
        }
    }
}

fn matches_search(tool: &BlackArchTool, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }

    tool.name.to_lowercase().contains(query)
        || tool.package_name.to_lowercase().contains(query)
        || tool
            .category
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(query)
        || tool
            .categories
            .iter()
            .any(|category| category.to_lowercase().contains(query))
        || tool
            .description
            .as_deref()
            .unwrap_or_default()
            .to_lowercase()
            .contains(query)
}

fn tool_needs_detail(tool: &BlackArchTool) -> bool {
    tool.version.is_none()
        && tool
            .description
            .as_deref()
            .is_none_or(|description| !description.starts_with("Package info unavailable"))
}

fn current_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    worker_tx: &mpsc::Sender<WorkerCommand>,
    worker_event_rx: &mpsc::Receiver<WorkerEvent>,
) -> Result<()> {
    let tick_rate = Duration::from_millis(33);

    while !app.should_quit {
        while let Ok(worker_event) = worker_event_rx.try_recv() {
            app.handle_worker_event(worker_event);
        }

        if crossterm_event::poll(tick_rate)? {
            event::handle_events(app, worker_tx)?;
        }

        app.maybe_dispatch_debounced_tasks(worker_tx);
        terminal.draw(|frame| ui::render(frame, app))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_detail_result_is_ignored() {
        let mut app = app_with_tools(vec![tool("sqlmap"), tool("nmap")]);
        app.current_detail_request_id = 2;
        app.detail_loading = true;

        let mut stale = tool("sqlmap");
        stale.version = Some("1.0".to_string());
        app.apply_tool_detail_result("sqlmap", 1, stale);

        assert!(app.detail_loading);
        assert_eq!(app.selected_tool().unwrap().version, None);
    }

    #[test]
    fn current_detail_result_is_applied() {
        let mut app = app_with_tools(vec![tool("sqlmap")]);
        app.current_detail_request_id = 7;
        app.detail_loading = true;

        let mut detail = tool("sqlmap");
        detail.version = Some("1.7.11-1".to_string());
        app.apply_tool_detail_result("sqlmap", 7, detail);

        assert!(!app.detail_loading);
        assert_eq!(
            app.selected_tool().unwrap().version.as_deref(),
            Some("1.7.11-1")
        );
    }

    #[test]
    fn selection_change_sets_pending_detail_package() {
        let mut app = app_with_tools(vec![tool("sqlmap"), tool("nmap")]);

        app.select_next_tool();

        assert_eq!(app.selected_tool().unwrap().package_name, "nmap");
        assert_eq!(app.pending_detail_package.as_deref(), Some("nmap"));
        assert!(app.detail_loading);
        assert!(app.last_selection_change.is_some());
    }

    #[test]
    fn apply_filters_uses_in_memory_data() {
        let mut app = app_with_tools(vec![tool("sqlmap"), tool("nmap")]);
        app.search_query = "sql".to_string();

        app.apply_filters();

        assert_eq!(app.filtered_tools.len(), 1);
        assert_eq!(app.filtered_tools[0].package_name, "sqlmap");
    }

    #[test]
    fn detail_error_marks_selected_tool_without_crashing() {
        let mut app = app_with_tools(vec![tool("sqlmap")]);
        app.current_detail_request_id = 3;
        app.detail_loading = true;

        app.apply_tool_detail_error("sqlmap", 3, "pacman failed".to_string());

        assert!(!app.detail_loading);
        assert!(
            app.selected_tool()
                .unwrap()
                .description
                .as_deref()
                .is_some_and(
                    |description| description.contains("Package info unavailable")
                        && description.contains("pacman failed")
                )
        );
    }

    #[test]
    fn favorites_filter_returns_only_favorite_tools() {
        let mut app = app_with_tools(vec![tool("sqlmap"), tool("nmap")]);
        app.user_state.favorites = vec!["sqlmap".to_string()];
        app.apply_user_state_to_tools();
        app.selected_category_index = app
            .categories
            .iter()
            .position(|category| category == "Favorites")
            .unwrap();

        app.apply_filters();

        assert_eq!(app.filtered_tools.len(), 1);
        assert_eq!(app.filtered_tools[0].package_name, "sqlmap");
    }

    #[test]
    fn recent_filter_uses_recent_order() {
        let mut app = app_with_tools(vec![tool("sqlmap"), tool("nmap"), tool("amass")]);
        app.user_state.recent = vec![
            RecentTool {
                package_name: "nmap".to_string(),
                executable: Some("nmap".to_string()),
                last_used: "2".to_string(),
            },
            RecentTool {
                package_name: "sqlmap".to_string(),
                executable: Some("sqlmap".to_string()),
                last_used: "1".to_string(),
            },
        ];
        app.selected_category_index = app
            .categories
            .iter()
            .position(|category| category == "Recent")
            .unwrap();

        app.apply_filters();

        assert_eq!(
            app.filtered_tools
                .iter()
                .map(|tool| tool.package_name.as_str())
                .collect::<Vec<_>>(),
            vec!["nmap", "sqlmap"]
        );
    }

    #[test]
    fn open_install_confirmation_creates_modal_state() {
        let mut app = app_with_tools(vec![tool("sqlmap")]);

        app.open_install_confirmation();

        assert!(app.confirm_modal.visible);
        assert_eq!(app.confirm_modal.title, "Install Package");
        assert!(!app.confirm_modal.selected_confirm);
        assert_eq!(
            app.confirm_modal.command_preview.as_deref(),
            Some("pkexec pacman -S --needed --noconfirm sqlmap")
        );
    }

    #[test]
    fn cancel_confirmation_closes_modal() {
        let (tx, _rx) = mpsc::channel();
        let mut app = app_with_tools(vec![tool("sqlmap")]);
        app.open_install_confirmation();

        app.confirm_modal_action(&tx).unwrap();

        assert!(!app.confirm_modal.visible);
        assert_eq!(app.status_message, "Install cancelled");
    }

    #[test]
    fn confirmation_toggle_switches_selected_button() {
        let mut app = app_with_tools(vec![tool("sqlmap")]);
        app.open_install_confirmation();

        app.toggle_confirmation_selection();

        assert!(app.confirm_modal.selected_confirm);
    }

    #[test]
    fn confirmation_sends_install_when_confirm_selected() {
        let (tx, rx) = mpsc::channel();
        let mut app = app_with_tools(vec![tool("sqlmap")]);
        app.open_install_confirmation();
        app.toggle_confirmation_selection();
        // Simulate polkit agent being detected to use pkexec
        app.privilege_status = Some(privilege::PrivilegeStatus {
            pkexec_found: true,
            polkit_agent_detected: true,
            detected_agent: Some("test-agent".to_string()),
            warnings: vec![],
        });

        app.confirm_modal_action(&tx).unwrap();

        let command = rx.try_recv().unwrap();
        assert!(matches!(
            command,
            WorkerCommand::InstallPackages { package_names, .. } if package_names == vec!["sqlmap".to_string()]
        ));
    }

    #[test]
    fn invalid_install_package_is_rejected() {
        let app = app_with_tools(vec![tool("sqlmap")]);
        let error = app.validate_install_package("bad/name").unwrap_err();
        assert!(matches!(error, AppError::InvalidPackageName(_)));
    }

    #[test]
    fn unavailable_install_package_is_rejected() {
        let app = app_with_tools(vec![tool("sqlmap")]);
        let error = app
            .validate_install_package("nmap")
            .unwrap_err()
            .to_string();
        assert!(error.contains("Package is not available"));
    }

    #[test]
    fn duplicate_install_is_blocked() {
        let (tx, _rx) = mpsc::channel();
        let mut app = app_with_tools(vec![tool("sqlmap")]);
        app.install_in_progress = true;

        app.start_install_package("sqlmap".to_string(), &tx)
            .unwrap();

        assert_eq!(
            app.status_message,
            "Another package operation is already in progress"
        );
    }

    #[test]
    fn install_success_updates_tool_status() {
        let mut app = app_with_tools(vec![tool("sqlmap")]);

        app.apply_install_success("sqlmap", None);

        assert_eq!(app.selected_tool().unwrap().status, ToolStatus::Installed);
        assert_eq!(
            app.status_message,
            "Installed packages, but some details could not be refreshed"
        );
    }

    #[test]
    fn install_failure_sets_error_status() {
        let mut app = app_with_tools(vec![tool("sqlmap")]);
        app.install_in_progress = true;

        app.apply_install_failure("sqlmap", "auth failed".to_string());

        assert!(!app.install_in_progress);
        assert_eq!(app.status_message, "Failed to install sqlmap");
        assert_eq!(app.error_message.as_deref(), Some("auth failed"));
    }

    #[test]
    fn successful_batch_install_clears_installed_queue_items() {
        let mut app = app_with_tools(vec![tool("sqlmap"), tool("nmap")]);
        app.install_queue.add("sqlmap".to_string());
        app.install_queue.add("nmap".to_string());

        app.apply_batch_install_success(&["sqlmap".to_string()], Vec::new());

        assert!(!app.install_queue.contains("sqlmap"));
        assert!(app.install_queue.contains("nmap"));
    }

    #[test]
    fn failed_batch_install_keeps_queue() {
        let mut app = app_with_tools(vec![tool("sqlmap")]);
        app.install_queue.add("sqlmap".to_string());
        app.install_in_progress = true;

        app.apply_batch_install_failure(&["sqlmap".to_string()], "auth failed".to_string());

        assert!(app.install_queue.contains("sqlmap"));
        assert_eq!(app.status_message, "Failed to install sqlmap");
    }

    #[test]
    fn remove_modal_opens_only_for_installed_package() {
        let mut installed = tool("sqlmap");
        installed.status = ToolStatus::Installed;
        let mut app = app_with_tools(vec![installed]);

        app.open_remove_confirmation_for_selected_package();

        assert!(app.confirm_modal.visible);
        assert_eq!(app.confirm_modal.title, "Remove Package");
        assert_eq!(app.confirm_modal.confirm_label, "Remove");
        assert_eq!(
            app.confirm_modal.command_preview.as_deref(),
            Some("pkexec pacman -Rns --noconfirm sqlmap")
        );
    }

    #[test]
    fn remove_modal_does_not_open_for_not_installed_package() {
        let mut app = app_with_tools(vec![tool("sqlmap")]);

        app.open_remove_confirmation_for_selected_package();

        assert!(!app.confirm_modal.visible);
        assert_eq!(app.status_message, "Package is not installed");
    }

    #[test]
    fn remove_confirmation_defaults_to_cancel() {
        let mut installed = tool("sqlmap");
        installed.status = ToolStatus::Installed;
        let mut app = app_with_tools(vec![installed]);

        app.open_remove_confirmation_for_selected_package();

        assert!(!app.confirm_modal.selected_confirm);
    }

    #[test]
    fn cancel_remove_confirmation_closes_modal() {
        let (tx, _rx) = mpsc::channel();
        let mut installed = tool("sqlmap");
        installed.status = ToolStatus::Installed;
        let mut app = app_with_tools(vec![installed]);
        app.open_remove_confirmation_for_selected_package();

        app.confirm_modal_action(&tx).unwrap();

        assert!(!app.confirm_modal.visible);
        assert_eq!(app.status_message, "Remove cancelled");
    }

    #[test]
    fn confirm_remove_sends_worker_command() {
        let (tx, rx) = mpsc::channel();
        let mut installed = tool("sqlmap");
        installed.status = ToolStatus::Installed;
        let mut app = app_with_tools(vec![installed]);
        app.open_remove_confirmation_for_selected_package();
        app.toggle_confirmation_selection();
        app.privilege_status = Some(privilege::PrivilegeStatus {
            pkexec_found: true,
            polkit_agent_detected: true,
            detected_agent: Some("test-agent".to_string()),
            warnings: vec![],
        });

        app.confirm_modal_action(&tx).unwrap();

        let command = rx.try_recv().unwrap();
        assert!(matches!(
            command,
            WorkerCommand::RemovePackage { package_name, .. } if package_name == "sqlmap"
        ));
    }

    #[test]
    fn confirm_remove_uses_pkexec_without_polkit_agent() {
        let (tx, rx) = mpsc::channel();
        let mut installed = tool("sqlmap");
        installed.status = ToolStatus::Installed;
        let mut app = app_with_tools(vec![installed]);
        app.open_remove_confirmation_for_selected_package();
        app.toggle_confirmation_selection();
        app.privilege_status = Some(privilege::PrivilegeStatus {
            pkexec_found: true,
            polkit_agent_detected: false,
            detected_agent: None,
            warnings: vec![],
        });

        app.confirm_modal_action(&tx).unwrap();

        let command = rx.try_recv().unwrap();
        assert!(matches!(
            command,
            WorkerCommand::RemovePackage { package_name, .. } if package_name == "sqlmap"
        ));
    }

    #[test]
    fn remove_success_updates_status_and_clears_executables() {
        let mut installed = tool("sqlmap");
        installed.status = ToolStatus::Installed;
        installed.executable = Some("sqlmap".to_string());
        installed.executables = vec!["sqlmap".to_string()];
        let mut app = app_with_tools(vec![installed]);
        app.install_in_progress = true;
        app.current_remove_request_id = Some(1);
        let mut refreshed = tool("sqlmap");
        refreshed.status = ToolStatus::NotInstalled;

        app.apply_remove_success("sqlmap", refreshed, true);

        let selected = app.selected_tool().unwrap();
        assert_eq!(selected.status, ToolStatus::NotInstalled);
        assert_eq!(selected.executable, None);
        assert!(selected.executables.is_empty());
        assert!(!app.install_in_progress);
    }

    #[test]
    fn remove_failure_keeps_status_unchanged() {
        let mut installed = tool("sqlmap");
        installed.status = ToolStatus::Installed;
        let mut app = app_with_tools(vec![installed]);
        app.install_in_progress = true;

        app.apply_remove_failure("sqlmap", "auth failed".to_string());

        assert_eq!(app.selected_tool().unwrap().status, ToolStatus::Installed);
        assert_eq!(app.status_message, "Failed to remove sqlmap");
        assert_eq!(app.error_message.as_deref(), Some("auth failed"));
    }

    #[test]
    fn package_remains_visible_after_remove() {
        let mut installed = tool("sqlmap");
        installed.status = ToolStatus::Installed;
        let mut app = app_with_tools(vec![installed]);
        let refreshed = tool("sqlmap");

        app.apply_remove_success("sqlmap", refreshed, true);

        assert!(app.tools.iter().any(|tool| tool.package_name == "sqlmap"));
        assert!(
            app.filtered_tools
                .iter()
                .any(|tool| tool.package_name == "sqlmap")
        );
    }

    #[test]
    fn package_operation_blocks_duplicate_remove() {
        let (tx, _rx) = mpsc::channel();
        let mut installed = tool("sqlmap");
        installed.status = ToolStatus::Installed;
        let mut app = app_with_tools(vec![installed]);
        app.install_in_progress = true;

        app.start_remove_package("sqlmap".to_string(), &tx).unwrap();

        assert_eq!(
            app.status_message,
            "Another package operation is already in progress"
        );
    }

    #[test]
    fn action_menu_selection_wraps() {
        let mut app = app_with_tools(vec![tool("sqlmap")]);
        app.open_action_menu();

        app.select_previous_action();

        assert_eq!(app.selected_action(), Some(ActionMenuItem::Cancel));

        app.select_next_action();
        assert_eq!(app.selected_action(), Some(ActionMenuItem::RunInTerminal));
    }

    #[test]
    fn selected_command_uses_executable_then_package_fallback() {
        let mut with_executable = tool("sqlmap");
        with_executable.executables = vec!["sqlmap".to_string()];
        let app = app_with_tools(vec![with_executable]);
        assert_eq!(app.selected_command().as_deref(), Some("sqlmap"));

        let fallback = app_with_tools(vec![tool("nmap")]);
        assert_eq!(fallback.selected_command().as_deref(), Some("nmap"));
    }

    #[test]
    fn not_installed_package_cannot_run() {
        let app = app_with_tools(vec![tool("sqlmap")]);
        let error = app.selected_tool_executable().unwrap_err().to_string();
        assert!(error.contains("Package is not installed"));
    }

    #[test]
    fn installed_package_with_executable_can_run() {
        let mut installed = tool("sqlmap");
        installed.status = ToolStatus::Installed;
        installed.version = Some("1".to_string());
        installed.executables = vec!["sqlmap".to_string()];
        let app = app_with_tools(vec![installed]);

        assert_eq!(app.selected_tool_executable().unwrap(), "sqlmap");
    }

    #[test]
    fn installed_package_without_executable_returns_error() {
        let mut installed = tool("sqlmap");
        installed.status = ToolStatus::Installed;
        installed.version = Some("1".to_string());
        let app = app_with_tools(vec![installed]);

        let error = app.selected_tool_executable().unwrap_err().to_string();
        assert!(error.contains("No executable found for this package"));
    }

    #[test]
    fn installed_package_with_unloaded_details_returns_error() {
        let mut installed = tool("sqlmap");
        installed.status = ToolStatus::Installed;
        let app = app_with_tools(vec![installed]);

        let error = app.selected_tool_executable().unwrap_err().to_string();
        assert!(error.contains("Package details are not loaded yet"));
    }

    fn app_with_tools(tools: Vec<BlackArchTool>) -> App {
        let mut app = App::new();
        app.set_backend_data(vec!["blackarch-webapp".to_string()], tools);
        app
    }

    fn tool(package_name: &str) -> BlackArchTool {
        BlackArchTool {
            name: package_name.to_string(),
            package_name: package_name.to_string(),
            category: Some("blackarch-webapp".to_string()),
            categories: vec!["blackarch-webapp".to_string()],
            version: None,
            description: None,
            executable: None,
            executables: Vec::new(),
            status: ToolStatus::NotInstalled,
            favorite: false,
        }
    }
}
