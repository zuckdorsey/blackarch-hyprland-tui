use clap::{Parser, Subcommand};

use crate::{
    cache, config,
    error::Result,
    pacman::{command, query},
    services::tool_service,
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
    Executables {
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
        Some(Commands::Executables { package }) => executables(&package),
        Some(Commands::SyncCache) => sync_cache(),
        None => Ok(()),
    }
}

fn doctor() -> Result<()> {
    print_check("pacman", command::pacman_exists());

    match query::list_blackarch_categories() {
        Ok(categories) => println!("[ok] BlackArch groups: {} found", categories.len()),
        Err(error) => println!("[warn] BlackArch groups: {error}"),
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
        for (tool, category) in query::list_all_blackarch_tools()? {
            println!("{tool}\t{category}");
        }
    }

    Ok(())
}

fn info(package: &str) -> Result<()> {
    let tool = tool_service::get_tool_detail(package)?;
    println!("{}", serde_json::to_string_pretty(&tool)?);
    Ok(())
}

fn executables(package: &str) -> Result<()> {
    for executable in query::get_executables(package)? {
        println!("{executable}");
    }
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
