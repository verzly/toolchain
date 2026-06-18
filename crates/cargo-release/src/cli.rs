//! Command-line interface for the builder. Keep this file focused on flags and subcommands.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "cargo-release")]
#[command(bin_name = "cargo-release")]
#[command(
    author,
    version,
    about = "Container-aware release builder for Rust executable artifacts",
    after_help = "Read the full README: https://github.com/verzly/toolchain#cargo-release"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Write cargo-release defaults into a datarose.toml file.
    Init(InitArgs),
    /// Print the configured build plan.
    Plan(CommonArgs),
    /// Build release artifacts.
    Build(BuildArgs),
    /// Remove generated release artifacts.
    Clean(CommonArgs),
    /// Check local tooling.
    Doctor(CommonArgs),
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/toolchain#cargo-release")]
pub struct InitArgs {
    #[arg(short, long, default_value = "datarose.toml")]
    pub config: PathBuf,

    #[arg(short, long, default_value_t = false)]
    pub force: bool,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/toolchain#cargo-release")]
pub struct CommonArgs {
    #[arg(short, long, default_value = "datarose.toml")]
    pub config: PathBuf,

    /// Release target to read from datarose.toml.
    #[arg(long)]
    pub release_target: Option<String>,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/toolchain#cargo-release")]
pub struct BuildArgs {
    #[arg(short, long, default_value = "datarose.toml")]
    pub config: PathBuf,

    /// Release target to read from datarose.toml.
    #[arg(long)]
    pub release_target: Option<String>,

    /// Version used when naming release artifacts.
    #[arg(short, long)]
    pub version: Option<String>,

    /// Build only this target key.
    #[arg(long)]
    pub target: Option<String>,

    /// Override configured output directory.
    #[arg(long)]
    pub output: Option<PathBuf>,

    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parses_build_options() {
        let cli = Cli::parse_from([
            "cargo-release",
            "build",
            "--config",
            "release.toml",
            "--release-target",
            "my-tool",
            "--version",
            "1.2.3",
            "--target",
            "linux-x64",
            "--output",
            "dist/release",
            "--dry-run",
        ]);

        let Commands::Build(args) = cli.command else {
            panic!("expected build command");
        };

        assert_eq!(args.config, PathBuf::from("release.toml"));
        assert_eq!(args.release_target.as_deref(), Some("my-tool"));
        assert_eq!(args.version.as_deref(), Some("1.2.3"));
        assert_eq!(args.target.as_deref(), Some("linux-x64"));
        assert_eq!(args.output, Some(PathBuf::from("dist/release")));
        assert!(args.dry_run);
    }
}
