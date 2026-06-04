//! Command-line contract for the executable. This file should stay declarative: parsing only, no release workflow logic.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "github-release")]
#[command(bin_name = "github-release")]
#[command(
    author,
    version,
    about = "Reusable GitHub release branch and publishing orchestrator"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Write a starter github-release.toml file.
    Init(InitArgs),
    /// Print the release plan without changing the repository.
    Plan(PlanArgs),
    /// Create a release branch and update configured version files.
    Prepare(PrepareArgs),
    /// Merge the release branch and tag the source repository after artifacts were built successfully.
    Finalize(FinalizeArgs),
    /// Publish a GitHub Release without preparing or merging a branch.
    Publish(PublishArgs),
    /// Delete a temporary release branch after a failed build.
    Abort(AbortArgs),
}

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Config path to create.
    #[arg(short, long, default_value = "github-release.toml")]
    pub config: PathBuf,

    /// Overwrite the config file if it already exists.
    #[arg(short, long, default_value_t = false)]
    pub force: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PlanArgs {
    /// Version to release. Use SemVer such as 1.2.3 or 1.2.3-rc.1.
    #[arg(short, long)]
    pub version: String,

    /// Config path.
    #[arg(short, long, default_value = "github-release.toml")]
    pub config: PathBuf,

    /// Override target branch.
    #[arg(long)]
    pub target_branch: Option<String>,

    /// Override release branch.
    #[arg(long)]
    pub release_branch: Option<String>,
}

#[derive(Args, Debug, Clone)]
pub struct PrepareArgs {
    /// Version to release. Use SemVer such as 1.2.3 or 1.2.3-rc.1.
    #[arg(short, long)]
    pub version: String,

    /// Config path.
    #[arg(short, long, default_value = "github-release.toml")]
    pub config: PathBuf,

    /// Override target branch.
    #[arg(long)]
    pub target_branch: Option<String>,

    /// Override release branch.
    #[arg(long)]
    pub release_branch: Option<String>,

    /// Print commands and file updates without executing them.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Allow recreating a local release branch.
    #[arg(long, default_value_t = false)]
    pub force_branch: bool,

    /// Override version commit message.
    #[arg(long)]
    pub commit_message: Option<String>,
}

#[derive(Args, Debug)]
pub struct FinalizeArgs {
    /// Version to release. Use the same value that was passed to prepare.
    #[arg(short, long)]
    pub version: String,

    /// Config path.
    #[arg(short, long, default_value = "github-release.toml")]
    pub config: PathBuf,

    /// Override target branch.
    #[arg(long)]
    pub target_branch: Option<String>,

    /// Override release branch.
    #[arg(long)]
    pub release_branch: Option<String>,

    /// Directory containing release assets.
    #[arg(long)]
    pub assets: Option<PathBuf>,

    /// Override prerelease handling.
    #[arg(long, value_enum, default_value_t = PrereleaseMode::Auto)]
    pub prerelease: PrereleaseMode,

    /// Print commands without executing them.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Keep the release branch after success.
    #[arg(long, default_value_t = false)]
    pub keep_branch: bool,

    /// Do not create a GitHub Release. Useful for source monorepo tags that are followed by a public distribution release.
    #[arg(long, default_value_t = false)]
    pub skip_github_release: bool,

    /// Use this text as the GitHub Release body instead of generated notes.
    #[arg(long)]
    pub notes: Option<String>,

    /// Read the GitHub Release body from this file instead of generated notes.
    #[arg(long)]
    pub notes_file: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct PublishArgs {
    /// Version to publish. Use SemVer such as 1.2.3 or 1.2.3-rc.1.
    #[arg(short, long)]
    pub version: String,

    /// Config path.
    #[arg(short, long, default_value = "github-release.toml")]
    pub config: PathBuf,

    /// Directory containing release assets.
    #[arg(long)]
    pub assets: Option<PathBuf>,

    /// Override prerelease handling.
    #[arg(long, value_enum, default_value_t = PrereleaseMode::Auto)]
    pub prerelease: PrereleaseMode,

    /// Print commands without executing them.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Use this text as the GitHub Release body instead of generated notes.
    #[arg(long)]
    pub notes: Option<String>,

    /// Read the GitHub Release body from this file instead of generated notes.
    #[arg(long)]
    pub notes_file: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct AbortArgs {
    /// Version to release. Used to resolve the default release branch.
    #[arg(short, long)]
    pub version: Option<String>,

    /// Config path.
    #[arg(short, long, default_value = "github-release.toml")]
    pub config: PathBuf,

    /// Release branch to delete.
    #[arg(long)]
    pub release_branch: Option<String>,

    /// Allow deleting a branch that does not match the configured release branch prefix.
    #[arg(long, default_value_t = false)]
    pub allow_any_branch: bool,

    /// Print commands without executing them.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum PrereleaseMode {
    Auto,
    True,
    False,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parses_publish_options() {
        let cli = Cli::parse_from([
            "github-release",
            "publish",
            "--version",
            "1.2.3-rc.1",
            "--config",
            "release.toml",
            "--assets",
            "dist",
            "--prerelease",
            "true",
            "--notes",
            "Custom release body",
            "--dry-run",
        ]);

        let Commands::Publish(args) = cli.command else {
            panic!("expected publish command");
        };

        assert_eq!(args.version, "1.2.3-rc.1");
        assert_eq!(args.config, PathBuf::from("release.toml"));
        assert_eq!(args.assets, Some(PathBuf::from("dist")));
        assert_eq!(args.prerelease, PrereleaseMode::True);
        assert_eq!(args.notes.as_deref(), Some("Custom release body"));
        assert!(args.dry_run);
    }
}
