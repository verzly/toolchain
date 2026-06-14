//! Entry point for repository standards bootstrap commands.

mod cli;
mod commands;
mod output;
mod project;
mod quality;
mod release;
mod shell;
mod standards;
mod workflow;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands, TuiArgs};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command.unwrap_or_else(|| {
        Commands::Tui(TuiArgs {
            root: ".".into(),
            config: None,
        })
    }) {
        Commands::Init(args) => commands::init::run(args),
        Commands::Update(args) => commands::init::run_update(args),
        Commands::Plan(args) => commands::plan::run(args),
        Commands::Projects(args) => commands::projects::run(args),
        Commands::Check(args) => commands::check::run(args),
        Commands::Release(args) => commands::release::run(*args),
        Commands::Tui(args) => commands::tui::run(args),
        Commands::Doctor(args) => commands::doctor::run(args),
    }
}
