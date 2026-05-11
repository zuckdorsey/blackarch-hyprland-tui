use std::{sync::mpsc, thread};

use crate::{
    actions::{package_installer, package_remover, terminal_runner},
    cache::store as cache_store,
    config,
    models::{BlackArchTool, ToolStatus},
    services::tool_service,
    user_state::store as user_store,
    worker::message::{WorkerCommand, WorkerEvent},
};

pub fn start_worker(
    command_rx: mpsc::Receiver<WorkerCommand>,
    event_tx: mpsc::Sender<WorkerEvent>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while let Ok(command) = command_rx.recv() {
            let event = match command {
                WorkerCommand::LoadInitialData => load_initial_data(),
                WorkerCommand::SyncBasicCache => sync_basic_cache(),
                WorkerCommand::LoadToolDetail {
                    package_name,
                    request_id,
                } => load_tool_detail(package_name, request_id),
                WorkerCommand::RefreshToolDetail {
                    package_name,
                    request_id,
                } => load_tool_detail(package_name, request_id),
                WorkerCommand::SearchPackages { query, request_id } => {
                    search_packages(query, request_id)
                }
                WorkerCommand::ClearPackageCache {
                    package_name,
                    request_id,
                } => WorkerEvent::CacheCleared {
                    package_name,
                    request_id,
                },
                WorkerCommand::RunTool {
                    package_name,
                    executable,
                    request_id,
                } => run_tool(package_name, executable, request_id),
                WorkerCommand::InstallPackage {
                    package_name,
                    request_id,
                } => {
                    let package_names = vec![package_name];
                    if event_tx
                        .send(WorkerEvent::PackagesInstallStarted {
                            package_names: package_names.clone(),
                            request_id,
                        })
                        .is_err()
                    {
                        break;
                    }
                    install_packages(package_names, request_id)
                }
                WorkerCommand::InstallPackages {
                    package_names,
                    request_id,
                } => {
                    if event_tx
                        .send(WorkerEvent::PackagesInstallStarted {
                            package_names: package_names.clone(),
                            request_id,
                        })
                        .is_err()
                    {
                        break;
                    }
                    install_packages(package_names, request_id)
                }
                WorkerCommand::RemovePackage {
                    package_name,
                    request_id,
                } => {
                    if event_tx
                        .send(WorkerEvent::PackageRemoveStarted {
                            package_name: package_name.clone(),
                            request_id,
                        })
                        .is_err()
                    {
                        break;
                    }
                    remove_package(package_name, request_id)
                }
            };

            if event_tx.send(event).is_err() {
                break;
            }
        }
    })
}

fn load_initial_data() -> WorkerEvent {
    let config = match config::settings::load_config() {
        Ok(config) => config,
        Err(error) => {
            return WorkerEvent::TaskFailed {
                label: "initial data".to_string(),
                error: error.to_string(),
            };
        }
    };

    match tool_service::load_tools(config.pacman.prefer_cache) {
        Ok(tools) => {
            let categories = tool_service::load_categories(config.pacman.prefer_cache)
                .unwrap_or_else(|_| categories_from_tools(&tools));
            WorkerEvent::InitialDataLoaded { tools, categories }
        }
        Err(error) => WorkerEvent::TaskFailed {
            label: "initial data".to_string(),
            error: error.to_string(),
        },
    }
}

fn sync_basic_cache() -> WorkerEvent {
    let categories = match tool_service::refresh_categories_cache() {
        Ok(categories) => categories,
        Err(error) => {
            return WorkerEvent::TaskFailed {
                label: "sync cache".to_string(),
                error: error.to_string(),
            };
        }
    };

    match tool_service::refresh_tools_cache() {
        Ok(tools) => WorkerEvent::BasicCacheSynced { tools, categories },
        Err(error) => WorkerEvent::TaskFailed {
            label: "sync cache".to_string(),
            error: error.to_string(),
        },
    }
}

fn load_tool_detail(package_name: String, request_id: u64) -> WorkerEvent {
    match tool_service::get_tool_detail(&package_name) {
        Ok(tool) => WorkerEvent::ToolDetailLoaded {
            package_name,
            request_id,
            tool,
        },
        Err(error) => WorkerEvent::ToolDetailFailed {
            package_name,
            request_id,
            error: error.to_string(),
        },
    }
}

fn search_packages(query: String, request_id: u64) -> WorkerEvent {
    match crate::pacman::query::search_blackarch_packages(&query) {
        Ok(results) => WorkerEvent::SearchCompleted {
            query,
            request_id,
            results,
        },
        Err(error) => WorkerEvent::TaskFailed {
            label: format!("search {query}"),
            error: error.to_string(),
        },
    }
}

fn run_tool(package_name: String, executable: String, request_id: u64) -> WorkerEvent {
    let config = match config::settings::load_config() {
        Ok(config) => config,
        Err(error) => {
            return WorkerEvent::ToolRunFailed {
                package_name,
                request_id,
                error: error.to_string(),
            };
        }
    };

    if config.terminal.hold_after_run {
        return WorkerEvent::ToolRunFailed {
            package_name,
            request_id,
            error: "hold_after_run is not supported without shell wrapping yet".to_string(),
        };
    }

    match terminal_runner::run_in_terminal(
        &config.terminal.program,
        &config.terminal.runner_class,
        &executable,
    ) {
        Ok(()) => match user_store::add_recent_tool(&package_name, Some(&executable)) {
            Ok(()) => WorkerEvent::ToolRunStarted {
                package_name,
                executable,
                request_id,
            },
            Err(error) => WorkerEvent::ToolRunFailed {
                package_name,
                request_id,
                error: error.to_string(),
            },
        },
        Err(error) => WorkerEvent::ToolRunFailed {
            package_name,
            request_id,
            error: error.to_string(),
        },
    }
}

fn install_packages(package_names: Vec<String>, request_id: u64) -> WorkerEvent {
    match package_installer::install_packages(&package_names) {
        Ok(result) if result.success => {
            let _ = (result.packages, result.stdout, result.stderr);
            let mut refreshed_tools = Vec::new();
            for package_name in &package_names {
                if let Ok(tool) = tool_service::get_tool_detail(package_name) {
                    let _ = cache_store::save_package_detail_cache(package_name, &tool);
                    refreshed_tools.push(tool);
                }
            }
            WorkerEvent::PackagesInstallFinished {
                package_names,
                request_id,
                refreshed_tools,
            }
        }
        Ok(_result) => WorkerEvent::PackagesInstallFailed {
            package_names,
            request_id,
            error: "Install failed".to_string(),
        },
        Err(error) => WorkerEvent::PackagesInstallFailed {
            package_names,
            request_id,
            error: error.to_string(),
        },
    }
}

fn remove_package(package_name: String, request_id: u64) -> WorkerEvent {
    match package_remover::remove_package(&package_name) {
        Ok(result) if result.success => {
            let _ = (result.package_name, result.stdout, result.stderr);
            finish_successful_remove(package_name, request_id)
        }
        Ok(result) => WorkerEvent::PackageRemoveFailed {
            package_name,
            request_id,
            error: if result.stderr.trim().is_empty() {
                "Remove failed".to_string()
            } else {
                result.stderr.trim().to_string()
            },
        },
        Err(error) => WorkerEvent::PackageRemoveFailed {
            package_name,
            request_id,
            error: error.to_string(),
        },
    }
}

fn finish_successful_remove(package_name: String, request_id: u64) -> WorkerEvent {
    let (refreshed_tool, refreshed) = match tool_service::get_tool_detail(&package_name) {
        Ok(mut tool) => {
            mark_tool_removed(&mut tool);
            (tool, true)
        }
        Err(_) => (removed_tool_fallback(&package_name), false),
    };
    let _ = cache_store::save_package_detail_cache(&package_name, &refreshed_tool);

    WorkerEvent::PackageRemoveFinished {
        package_name,
        request_id,
        refreshed_tool,
        refreshed,
    }
}

fn mark_tool_removed(tool: &mut BlackArchTool) {
    tool.status = ToolStatus::NotInstalled;
    tool.executable = None;
    tool.executables = Vec::new();
}

fn removed_tool_fallback(package_name: &str) -> BlackArchTool {
    BlackArchTool {
        name: package_name.to_string(),
        package_name: package_name.to_string(),
        category: None,
        categories: Vec::new(),
        version: None,
        description: Some("Package removed; details could not be refreshed".to_string()),
        executable: None,
        executables: Vec::new(),
        status: ToolStatus::NotInstalled,
        favorite: false,
    }
}

fn categories_from_tools(tools: &[crate::models::BlackArchTool]) -> Vec<String> {
    let mut categories = Vec::new();
    for tool in tools {
        for category in &tool.categories {
            if !categories.iter().any(|item| item == category) {
                categories.push(category.clone());
            }
        }

        if let Some(category) = &tool.category {
            if !categories.iter().any(|item| item == category) {
                categories.push(category.clone());
            }
        }
    }
    categories.sort();
    categories
}
