mod app;
mod cache;
mod cli;
mod config;
mod error;
mod event;
mod models;
mod pacman;
mod services;
mod ui;
mod utils;

use clap::Parser;

use crate::{cli::Cli, error::Result};

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.command.is_some() {
        cli::run(cli)
    } else {
        app::run()
    }
}
