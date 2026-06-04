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
    /// Squash merge one prepared source branch and create multiple source tags.
    FinalizeBatch(FinalizeBatchArgs),
    /// Publish a GitHub Release without preparing or merging a branch.
    Publish(PublishArgs),
    /// Create or refresh moving tags for published releases.
    FloatingTags(FloatingTagsArgs),
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

    /// Continue an existing release branch instead of recreating it from the target branch.
    #[arg(long, default_value_t = false)]
    pub reuse_branch: bool,

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

    /// How the release branch is merged back to the target branch.
    #[arg(long, value_enum, default_value_t = MergeStrategy::Squash)]
    pub merge_strategy: MergeStrategy,

    /// Do not create a GitHub Release. Useful for source monorepo tags that are followed by a public distribution release.
    #[arg(long, default_value_t = false)]
    pub skip_github_release: bool,

    /// Use this text as the GitHub Release body instead of generated notes.
    #[arg(long)]
    pub notes: Option<String>,

    /// Read the GitHub Release body from this file instead of generated notes.
    #[arg(long)]
    pub notes_file: Option<PathBuf>,

    /// Update stable major/minor floating tags such as v1 and v1.2 after finalization.
    #[arg(long, default_value_t = false)]
    pub update_floating_tags: bool,

    /// Update the configured latest tag after finalization.
    #[arg(long, default_value_t = false)]
    pub update_latest_tag: bool,

    /// Update the configured next tag after finalization.
    #[arg(long, default_value_t = false)]
    pub update_next_tag: bool,
}

#[derive(Args, Debug)]
pub struct FinalizeBatchArgs {
    /// Version to release. Use SemVer such as 1.2.3 or 1.2.3-rc.1.
    #[arg(short, long)]
    pub version: String,

    /// Target branch to receive the squash merge.
    #[arg(long, default_value = "master")]
    pub target_branch: String,

    /// Prepared aggregate source release branch to squash merge.
    #[arg(long)]
    pub release_branch: String,

    /// Source tag to create from the finalized target branch. Repeat for multiple tags.
    #[arg(long = "source-tag", required = true)]
    pub source_tags: Vec<String>,

    /// Override squash merge commit message.
    #[arg(long)]
    pub merge_message: Option<String>,

    /// Print commands without executing them.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Keep the release branch after success.
    #[arg(long, default_value_t = false)]
    pub keep_branch: bool,
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

    /// Update stable major/minor floating tags such as v1 and v1.2 after publishing.
    #[arg(long, default_value_t = false)]
    pub update_floating_tags: bool,

    /// Update the configured latest tag after publishing.
    #[arg(long, default_value_t = false)]
    pub update_latest_tag: bool,

    /// Update the configured next tag after publishing.
    #[arg(long, default_value_t = false)]
    pub update_next_tag: bool,
}

#[derive(Args, Debug)]
pub struct FloatingTagsArgs {
    /// Config path.
    #[arg(short, long, default_value = "github-release.toml")]
    pub config: PathBuf,

    /// Release version to resolve through the configured prefix and suffix.
    #[arg(short, long)]
    pub version: Option<String>,

    /// Existing full stable tag to analyze, such as v1.2.3.
    #[arg(long)]
    pub tag: Option<String>,

    /// Scan all stable vX.Y.Z tags and update the highest matching vX.Y and vX tags.
    #[arg(long, default_value_t = false)]
    pub all: bool,

    /// Override target repository instead of using github.target_repository.
    #[arg(long)]
    pub repository: Option<String>,

    /// Run and enable all floating tag families even when they are disabled in the config.
    #[arg(long, default_value_t = false)]
    pub force: bool,

    /// Print commands without executing them.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum MergeStrategy {
    Squash,
    NoFf,
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
            "--update-floating-tags",
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
        assert!(args.update_floating_tags);
        assert!(args.dry_run);
    }

    #[test]
    fn parses_prepare_reuse_branch() {
        let cli = Cli::parse_from([
            "github-release",
            "prepare",
            "--version",
            "1.2.3",
            "--release-branch",
            "release/all-v1.2.3",
            "--reuse-branch",
        ]);

        let Commands::Prepare(args) = cli.command else {
            panic!("expected prepare command");
        };

        assert_eq!(args.release_branch.as_deref(), Some("release/all-v1.2.3"));
        assert!(args.reuse_branch);
    }

    #[test]
    fn parses_finalize_merge_strategy() {
        let cli = Cli::parse_from([
            "github-release",
            "finalize",
            "--version",
            "1.2.3",
            "--merge-strategy",
            "no-ff",
        ]);

        let Commands::Finalize(args) = cli.command else {
            panic!("expected finalize command");
        };

        assert_eq!(args.merge_strategy, MergeStrategy::NoFf);
    }

    #[test]
    fn parses_finalize_batch_source_tags() {
        let cli = Cli::parse_from([
            "github-release",
            "finalize-batch",
            "--version",
            "1.2.3",
            "--release-branch",
            "release/all-v1.2.3",
            "--source-tag",
            "cargo-release-v1.2.3",
            "--source-tag",
            "github-release-v1.2.3",
        ]);

        let Commands::FinalizeBatch(args) = cli.command else {
            panic!("expected finalize-batch command");
        };

        assert_eq!(args.release_branch, "release/all-v1.2.3");
        assert_eq!(
            args.source_tags,
            vec![
                "cargo-release-v1.2.3".to_string(),
                "github-release-v1.2.3".to_string()
            ]
        );
    }

    #[test]
    fn parses_floating_tags_all() {
        let cli = Cli::parse_from([
            "github-release",
            "floating-tags",
            "--config",
            "release.toml",
            "--all",
            "--dry-run",
        ]);

        let Commands::FloatingTags(args) = cli.command else {
            panic!("expected floating-tags command");
        };

        assert_eq!(args.config, PathBuf::from("release.toml"));
        assert!(args.all);
        assert!(args.dry_run);
    }
}
