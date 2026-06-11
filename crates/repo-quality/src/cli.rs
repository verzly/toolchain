//! Command-line contract for repository quality bootstrap.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "repo-quality")]
#[command(bin_name = "repo-quality")]
#[command(
    author,
    version,
    about = "Bootstrap hk/mise quality gates for Rust, JavaScript, and PHP repositories",
    after_help = "Read the full README: https://github.com/verzly/repo-quality"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Write managed quality files, release workflows, install mise tools, and install git hooks.
    Init(InitArgs),
    /// Refresh managed quality files and release workflows from datarose.toml.
    Update(UpdateArgs),
    /// Print the detected quality profile without changing files.
    Plan(PlanArgs),
    /// Check datarose.toml for deprecated, removed, or invalid settings.
    Check(CheckArgs),
    /// Check whether the repository has the expected quality tooling.
    Doctor(DoctorArgs),
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/repo-quality")]
pub struct InitArgs {
    #[arg(short, long, default_value = ".")]
    pub root: PathBuf,

    /// Use a custom config path instead of the root datarose.toml.
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Overwrite an existing hk.pkl.
    #[arg(short, long, default_value_t = false)]
    pub force: bool,

    /// Print the planned changes without writing files or running commands.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Do not run `mise use hk@latest` and `mise use pkl@latest`.
    #[arg(long, default_value_t = false)]
    pub skip_mise_use: bool,

    /// Do not run `hk install` after writing hk.pkl.
    #[arg(long, default_value_t = false)]
    pub skip_hk_install: bool,

    /// Add or override detected language profiles. Repeatable.
    #[arg(long = "language", value_enum)]
    pub languages: Vec<LanguageArg>,

    /// Override the JavaScript runner used for package tooling detection.
    #[arg(long, value_enum, default_value_t = JsRunnerArg::Auto)]
    pub js_runner: JsRunnerArg,

    /// Configure a subdirectory as the quality workspace.
    #[arg(long)]
    pub workspace: Option<PathBuf>,

    /// Do not write editor, formatter, linter, or Rector config files.
    #[arg(long, default_value_t = false)]
    pub skip_style_configs: bool,

    /// Do not write the GitHub Actions test workflow.
    #[arg(long, default_value_t = false)]
    pub skip_actions: bool,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/repo-quality")]
pub struct UpdateArgs {
    #[arg(short, long, default_value = ".")]
    pub root: PathBuf,

    /// Use a custom config path instead of the root datarose.toml.
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Overwrite project-local quality files even when they already exist.
    #[arg(short, long, default_value_t = false)]
    pub force: bool,

    /// Print the planned changes without writing files or running commands.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Do not run `mise use` for missing tools.
    #[arg(long, default_value_t = false)]
    pub skip_mise_use: bool,

    /// Do not run `hk install` after writing hk.pkl.
    #[arg(long, default_value_t = false)]
    pub skip_hk_install: bool,

    /// Do not write editor, formatter, linter, or Rector config files.
    #[arg(long, default_value_t = false)]
    pub skip_style_configs: bool,

    /// Do not write the GitHub Actions test workflow.
    #[arg(long, default_value_t = false)]
    pub skip_actions: bool,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/repo-quality")]
pub struct PlanArgs {
    #[arg(short, long, default_value = ".")]
    pub root: PathBuf,

    /// Use a custom config path instead of the root datarose.toml.
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    #[arg(long = "language", value_enum)]
    pub languages: Vec<LanguageArg>,

    #[arg(long, value_enum, default_value_t = JsRunnerArg::Auto)]
    pub js_runner: JsRunnerArg,

    /// Configure a subdirectory as the quality workspace for the preview.
    #[arg(long)]
    pub workspace: Option<PathBuf>,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/repo-quality")]
pub struct CheckArgs {
    #[arg(short, long, default_value = ".")]
    pub root: PathBuf,

    /// Use a custom config path instead of the root datarose.toml.
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/repo-quality")]
pub struct DoctorArgs {
    #[arg(short, long, default_value = ".")]
    pub root: PathBuf,

    /// Use a custom config path instead of the root datarose.toml.
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum LanguageArg {
    Rust,
    Js,
    Php,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum JsRunnerArg {
    Auto,
    Aube,
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_init_overrides() {
        let cli = Cli::parse_from([
            "repo-quality",
            "init",
            "--root",
            "repo",
            "--force",
            "--language",
            "rust",
            "--language",
            "js",
            "--js-runner",
            "aube",
        ]);

        let Commands::Init(args) = cli.command else {
            panic!("expected init command");
        };

        assert_eq!(args.root, PathBuf::from("repo"));
        assert!(args.force);
        assert_eq!(args.languages, vec![LanguageArg::Rust, LanguageArg::Js]);
        assert_eq!(args.js_runner, JsRunnerArg::Aube);
    }
}
