//! Command-line contract for repository standards bootstrap.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "repository")]
#[command(bin_name = "repository")]
#[command(
    author,
    version,
    about = "Manage Datarose repository standards, quality gates, and release targets",
    after_help = "Read the full README: https://github.com/verzly/repository"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
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
    /// Manage datarose.toml release targets.
    Release(Box<ReleaseArgs>),
    /// Open an interactive terminal dashboard for common repository operations.
    Tui(TuiArgs),
    /// Check whether the repository has the expected quality tooling.
    Doctor(DoctorArgs),
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/repository")]
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
#[command(after_help = "Read the full README: https://github.com/verzly/repository")]
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
#[command(after_help = "Read the full README: https://github.com/verzly/repository")]
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
#[command(after_help = "Read the full README: https://github.com/verzly/repository")]
pub struct CheckArgs {
    #[arg(short, long, default_value = ".")]
    pub root: PathBuf,

    /// Use a custom config path instead of the root datarose.toml.
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/repository")]
pub struct ReleaseArgs {
    #[arg(short, long, default_value = ".")]
    pub root: PathBuf,

    /// Use a custom config path instead of the root datarose.toml.
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<ReleaseCommand>,
}

#[derive(Subcommand, Debug)]
pub enum ReleaseCommand {
    /// List configured release targets.
    List,
    /// Show one configured release target.
    Show(ReleaseTargetSelectorArgs),
    /// Add or update a release target.
    Set(Box<ReleaseSetArgs>),
    /// Remove a release target.
    Remove(ReleaseRemoveArgs),
    /// Open an interactive terminal release target editor.
    Tui,
}

#[derive(Args, Debug)]
pub struct ReleaseTargetSelectorArgs {
    /// Release target name.
    pub target: String,
}

#[derive(Args, Debug)]
pub struct ReleaseSetArgs {
    /// Release target name. Defaults to the target path directory name.
    #[arg(long)]
    pub name: Option<String>,

    /// Repository-relative target path, for example crates/github-release or apps/mobile.
    #[arg(long)]
    pub path: PathBuf,

    /// Publish repository, for example verzly/github-release. No default repository is invented.
    #[arg(long)]
    pub repository: Option<String>,

    /// Release strategy for this target.
    #[arg(long, value_enum)]
    pub strategy: Option<ReleaseStrategyArg>,

    /// Workflow management mode for this target.
    #[arg(long, value_enum)]
    pub workflow: Option<ReleaseWorkflowArg>,

    /// Workspace id/path this release target belongs to.
    #[arg(long)]
    pub workspace: Option<String>,

    /// Source kind, for example cargo-package, tauri-app, js-package, php-package, or custom.
    #[arg(long)]
    pub source_kind: Option<String>,

    /// Cargo package to build when source_kind is cargo-package.
    #[arg(long)]
    pub cargo_package: Option<String>,

    /// Cargo binary to package when source_kind is cargo-package.
    #[arg(long)]
    pub cargo_binary: Option<String>,

    /// Cargo release output directory.
    #[arg(long)]
    pub cargo_out_dir: Option<String>,

    /// Distribution template path.
    #[arg(long)]
    pub distribution_path: Option<String>,

    /// Version file path. Defaults to <path>/Cargo.toml when that file exists.
    #[arg(long)]
    pub version_file: Option<String>,

    /// Source tag prefix, for example my-tool-v.
    #[arg(long)]
    pub source_tag_prefix: Option<String>,

    /// Keep this target even when --path does not exist yet.
    #[arg(long, default_value_t = false)]
    pub allow_missing_path: bool,
}

#[derive(Args, Debug)]
pub struct ReleaseRemoveArgs {
    /// Release target name.
    pub target: Option<String>,

    /// Remove the release target configured for this repository-relative path.
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// Do not ask for confirmation.
    #[arg(short, long, default_value_t = false)]
    pub yes: bool,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/repository")]
pub struct DoctorArgs {
    #[arg(short, long, default_value = ".")]
    pub root: PathBuf,

    /// Use a custom config path instead of the root datarose.toml.
    #[arg(short, long)]
    pub config: Option<PathBuf>,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/repository")]
pub struct TuiArgs {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ReleaseStrategyArg {
    /// Publish into the same repository as the source repository.
    SameRepo,
    /// Publish built distribution files into a separate public distribution repository.
    DistributionRepo,
    /// Release tooling or metadata from the repository's own source before publishing targets.
    SelfHosted,
    /// Keep release orchestration outside generated repository workflows.
    Custom,
}

impl ReleaseStrategyArg {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SameRepo => "same-repo",
            Self::DistributionRepo => "distribution-repo",
            Self::SelfHosted => "self-hosted",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ReleaseWorkflowArg {
    /// repository owns and may update the generated release workflow.
    Managed,
    /// repository preserves existing workflow files and does not overwrite them.
    Preserve,
    /// release orchestration is custom; repository validates config only.
    Custom,
}

impl ReleaseWorkflowArg {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::Preserve => "preserve",
            Self::Custom => "custom",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_init_overrides() {
        let cli = Cli::parse_from([
            "repository",
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

        let Some(Commands::Init(args)) = cli.command else {
            panic!("expected init command");
        };

        assert_eq!(args.root, PathBuf::from("repo"));
        assert!(args.force);
        assert_eq!(args.languages, vec![LanguageArg::Rust, LanguageArg::Js]);
        assert_eq!(args.js_runner, JsRunnerArg::Aube);
    }

    #[test]
    fn parses_release_set() {
        let cli = Cli::parse_from([
            "repository",
            "release",
            "set",
            "--path",
            "crates/repository",
            "--repository",
            "verzly/repository",
            "--strategy",
            "distribution-repo",
            "--workflow",
            "custom",
        ]);

        let Some(Commands::Release(args)) = cli.command else {
            panic!("expected release command");
        };
        let Some(ReleaseCommand::Set(set_args)) = args.command else {
            panic!("expected release set command");
        };

        assert_eq!(set_args.path, PathBuf::from("crates/repository"));
        assert_eq!(set_args.repository.as_deref(), Some("verzly/repository"));
        assert_eq!(
            set_args.strategy,
            Some(ReleaseStrategyArg::DistributionRepo)
        );
        assert_eq!(set_args.workflow, Some(ReleaseWorkflowArg::Custom));
    }

    #[test]
    fn parses_tui_command() {
        let cli = Cli::parse_from(["repository", "tui", "--root", "repo"]);

        let Some(Commands::Tui(args)) = cli.command else {
            panic!("expected tui command");
        };

        assert_eq!(args.root, PathBuf::from("repo"));
    }

    #[test]
    fn accepts_no_subcommand_for_default_tui() {
        let cli = Cli::parse_from(["repository"]);

        assert!(cli.command.is_none());
    }
}
