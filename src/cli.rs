use clap::{Parser, Subcommand};

use crate::{
    actions::{privilege, terminal_runner},
    cache, config,
    error::{AppError, Result},
    models::{BlackArchTool, ToolStatus},
    pacman::{command, query},
    services::tool_service,
    user_state::store as user_store,
};

#[derive(Debug, Parser)]
#[command(name = "blackarch-hypr-tui")]
#[command(about = "BlackArch tool manager for Arch Linux + Hyprland")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Doctor,
    Categories,
    Tools {
        #[arg(long)]
        category: Option<String>,
    },
    Info {
        package: String,
    },
    Search {
        query: String,
    },
    Executables {
        package: String,
    },
    Run {
        package: String,
    },
    Favorites,
    Favorite {
        package: String,
    },
    Unfavorite {
        package: String,
    },
    SyncCache,
}

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Doctor) => doctor(),
        Some(Commands::Categories) => categories(),
        Some(Commands::Tools { category }) => tools(category),
        Some(Commands::Info { package }) => info(&package),
        Some(Commands::Search { query }) => search(&query),
        Some(Commands::Executables { package }) => executables(&package),
        Some(Commands::Run { package }) => run_tool(&package),
        Some(Commands::Favorites) => favorites(),
        Some(Commands::Favorite { package }) => favorite(&package),
        Some(Commands::Unfavorite { package }) => unfavorite(&package),
        Some(Commands::SyncCache) => sync_cache(),
        None => Ok(()),
    }
}

fn doctor() -> Result<()> {
    print_check("pacman", command::pacman_exists());
    let privilege_status = privilege::check_privilege_status();

    println!("Privilege:");
    if privilege_status.pkexec_found {
        println!("[ok] pkexec found");
    } else {
        println!("[warn] pkexec not found");
    }

    if let Some(agent) = &privilege_status.detected_agent {
        println!("[ok] polkit agent detected: {agent}");
    } else {
        println!("[warn] no polkit authentication agent detected");
    }

    for warning in &privilege_status.warnings {
        println!("[warn] {warning}");
    }

    match query::list_blackarch_categories() {
        Ok(categories) => println!("[ok] BlackArch groups: {} found", categories.len()),
        Err(error) => println!("[warn] BlackArch groups: {error}"),
    }

    match query::list_all_available_blackarch_tools() {
        Ok(tools) => println!("[ok] BlackArch sync packages: {} found", tools.len()),
        Err(error) => println!("[warn] BlackArch sync packages: {error}"),
    }

    match cache::paths::ensure_cache_dir() {
        Ok(()) => println!(
            "[ok] cache directory writable: {}",
            cache::paths::cache_dir()?.display()
        ),
        Err(error) => println!("[warn] cache directory: {error}"),
    }

    match config::paths::ensure_config_dir() {
        Ok(()) => println!(
            "[ok] config directory writable: {}",
            config::paths::config_dir()?.display()
        ),
        Err(error) => println!("[warn] config directory: {error}"),
    }

    let config = config::settings::load_config()?;
    println!("[ok] config loaded: theme={}", config.ui.theme);
    Ok(())
}

fn categories() -> Result<()> {
    for category in tool_service::load_categories(false)? {
        println!("{category}");
    }
    Ok(())
}

fn tools(category: Option<String>) -> Result<()> {
    if let Some(category) = category {
        for tool in query::list_tools_by_category(&category)? {
            println!("{tool}");
        }
    } else {
        for tool in query::list_all_available_blackarch_tools()? {
            println!("{tool}");
        }
    }

    Ok(())
}

fn info(package: &str) -> Result<()> {
    let tool = tool_service::get_tool_detail(package)?;
    println!("{}", serde_json::to_string_pretty(&tool)?);
    Ok(())
}

fn search(search_query: &str) -> Result<()> {
    for result in query::search_blackarch_packages(search_query)? {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}

fn executables(package: &str) -> Result<()> {
    for executable in query::get_executables_if_installed(package)? {
        println!("{executable}");
    }
    Ok(())
}

fn run_tool(package: &str) -> Result<()> {
    let tool = tool_service::get_tool_detail(package)?;
    let executable = executable_for_run(&tool)?;
    let config = config::settings::load_config()?;

    if config.terminal.hold_after_run {
        return Err(AppError::Config(
            "hold_after_run is not supported without shell wrapping yet".to_string(),
        ));
    }

    terminal_runner::run_in_terminal(
        &config.terminal.program,
        &config.terminal.runner_class,
        &executable,
    )?;
    user_store::add_recent_tool(&tool.package_name, Some(&executable))?;
    println!("launched {executable}");
    Ok(())
}

fn executable_for_run(tool: &BlackArchTool) -> Result<String> {
    if tool.status != ToolStatus::Installed {
        return Err(AppError::Config("Package is not installed".to_string()));
    }

    tool.executable
        .clone()
        .or_else(|| tool.executables.first().cloned())
        .ok_or_else(|| AppError::Config("No executable found for this package".to_string()))
}

fn favorites() -> Result<()> {
    for favorite in user_store::load_favorites()? {
        println!("{favorite}");
    }
    Ok(())
}

fn favorite(package: &str) -> Result<()> {
    if !user_store::is_favorite(package)? {
        user_store::toggle_favorite(package)?;
    }
    println!("added {package}");
    Ok(())
}

fn unfavorite(package: &str) -> Result<()> {
    let mut favorites = user_store::load_favorites()?;
    favorites.retain(|favorite| favorite != package);
    user_store::save_favorites(&favorites)?;
    println!("removed {package}");
    Ok(())
}

fn sync_cache() -> Result<()> {
    let categories = tool_service::refresh_categories_cache()?;
    let tools = tool_service::refresh_tools_cache()?;
    println!(
        "cached {} categories and {} tools",
        categories.len(),
        tools.len()
    );
    Ok(())
}

fn print_check(name: &str, ok: bool) {
    if ok {
        println!("[ok] {name}");
    } else {
        println!("[warn] {name}");
    }
}
