//! Command-line contract for cache routing. Parsing stays here; path decisions belong in `env_plan` and `workspace`.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rust-cache")]
#[command(bin_name = "rust-cache")]
#[command(author, version, about = "Project-local cache routing helper for Rust and Tauri workspaces")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Init(InitArgs),
    Env(CommonArgs),
    Run(RunArgs),
    Clean(CommonArgs),
    Doctor(CommonArgs),
}

#[derive(Args, Debug)]
pub struct InitArgs {
    #[arg(short, long, default_value = "rust-cache.toml")]
    pub config: PathBuf,

    #[arg(short, long, default_value_t = false)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct CommonArgs {
    #[arg(short, long, default_value = "rust-cache.toml")]
    pub config: PathBuf,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    #[arg(short, long, default_value = "rust-cache.toml")]
    pub config: PathBuf,

    /// Command to run after `--`.
    #[arg(last = true, required = true)]
    pub command: Vec<String>,
}
