//! Implements the failure cleanup phase. Branch deletion is guarded because this command can remove remote refs.

use anyhow::Result;

use crate::cli::AbortArgs;
use crate::config;
use crate::domain;
use crate::git;


// Abort may delete a branch, so the branch name is validated before any destructive git command runs.
pub fn run(args: AbortArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let branch = if let Some(branch) = args.release_branch.clone() {
        branch
    } else if let Some(version) = args.version.as_ref() {
        domain::build_plan(&config, version, None, None, None)?.release_branch
    } else {
        anyhow::bail!("provide --version or --release-branch");
    };

// Abort can delete a remote branch, so the configured release prefix is a hard safety rail by default.
    if !args.allow_any_branch && !branch.starts_with(&config.release.branch_prefix) {
        anyhow::bail!(
            "refusing to delete branch outside release prefix: {} (prefix: {})",
            branch,
            config.release.branch_prefix
        );
    }

    if git::remote_branch_exists(&branch) || args.dry_run {
        git::run(["push", "origin", "--delete", &branch], args.dry_run)?;
    } else {
        println!("remote release branch does not exist: {branch}");
    }

    if git::branch_exists(&branch) || args.dry_run {
        git::run(["branch", "-D", &branch], args.dry_run)?;
    }

    Ok(())
}
