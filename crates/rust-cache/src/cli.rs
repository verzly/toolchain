//! Command-line contract for cache routing. Parsing stays here; path decisions belong in `env_plan` and `workspace`.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rust-cache")]
#[command(bin_name = "rust-cache")]
#[command(
    author,
    version,
    about = "Project-local cache routing helper for Rust and Tauri workspaces",
    after_help = "Read the full README: https://github.com/verzly/rust-cache"
)]
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
#[command(after_help = "Read the full README: https://github.com/verzly/rust-cache")]
pub struct InitArgs {
    #[arg(short, long, default_value = "datarose.toml")]
    pub config: PathBuf,

    #[arg(short, long, default_value_t = false)]
    pub force: bool,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/rust-cache")]
pub struct CommonArgs {
    #[arg(short, long, default_value = "datarose.toml")]
    pub config: PathBuf,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/rust-cache")]
pub struct RunArgs {
    #[arg(short, long, default_value = "datarose.toml")]
    pub config: PathBuf,

    /// Command to run after `--`.
    #[arg(last = true, required = true)]
    pub command: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parses_run_command_after_separator() {
        let cli = Cli::parse_from([
            "rust-cache",
            "run",
            "--config",
            "datarose.toml",
            "--",
            "cargo",
            "test",
            "--workspace",
        ]);

        let Commands::Run(args) = cli.command else {
            panic!("expected run command");
        };

        assert_eq!(args.config, PathBuf::from("datarose.toml"));
        assert_eq!(args.command, vec!["cargo", "test", "--workspace"]);
    }
}
