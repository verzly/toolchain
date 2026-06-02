//! Implements the success phase after artifacts already exist. This command publishes; it must not build anything.

use anyhow::Result;

use crate::cli::FinalizeArgs;
use crate::config;
use crate::domain;
use crate::git;
use crate::github;
use crate::output;


// Finalize is intentionally ordered from safest to most public operation:
// merge first, then tag, then publish the GitHub Release from that tag.
pub fn run(args: FinalizeArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let plan = domain::build_plan(
        &config,
        &args.version,
        args.target_branch.as_deref(),
        args.release_branch.as_deref(),
        Some(args.prerelease),
    )?;

    output::print_plan(&plan);

    if !args.dry_run {
        git::ensure_clean_worktree()?;
    }

    git::run(["fetch", "origin", &plan.target_branch], args.dry_run)?;
    git::run(["fetch", "origin", &plan.release_branch], args.dry_run)?;
    git::run(["checkout", &plan.target_branch], args.dry_run)?;
    git::run(["pull", "--ff-only", "origin", &plan.target_branch], args.dry_run)?;
    git::run(["merge", "--no-ff", &plan.release_branch, "-m", &plan.merge_message], args.dry_run)?;
    git::run(["push", "origin", &plan.target_branch], args.dry_run)?;
    git::run(["tag", "-a", &plan.tag, "-m", &plan.release_name], args.dry_run)?;
    git::run(["push", "origin", &plan.tag], args.dry_run)?;

// Publishing is intentionally last: by this point the target branch and tag already represent the release.
    github::create_release(&plan, args.assets.as_deref(), args.dry_run)?;

    if config.release.cleanup && !args.keep_branch {
        git::run(["push", "origin", "--delete", &plan.release_branch], args.dry_run)?;
        git::run(["branch", "-D", &plan.release_branch], args.dry_run)?;
    }

    output::write_github_outputs(&plan)?;
    Ok(())
}
