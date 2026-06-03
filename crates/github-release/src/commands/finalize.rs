//! Implements the source success phase after artifacts already exist. This command merges and tags; publishing can be skipped.

use crate::cli::FinalizeArgs;
use crate::config;
use crate::domain;
use crate::git;
use crate::github;
use crate::output;
use anyhow::Result;
// Finalize is intentionally ordered from safest to most public operation:
// merge first, then tag, then optionally publish a GitHub Release from that tag.
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

    let remote_release_branch = format!("origin/{}", plan.release_branch);
    let release_refspec = format!(
        "{}:refs/remotes/origin/{}",
        plan.release_branch, plan.release_branch
    );

    git::run(["fetch", "origin", &plan.target_branch], args.dry_run)?;
    git::run(["fetch", "origin", &release_refspec], args.dry_run)?;
    git::run(["checkout", &plan.target_branch], args.dry_run)?;
    git::run(
        ["pull", "--ff-only", "origin", &plan.target_branch],
        args.dry_run,
    )?;
    git::run(
        [
            "merge",
            "--no-ff",
            &remote_release_branch,
            "-m",
            &plan.merge_message,
        ],
        args.dry_run,
    )?;
    git::run(["push", "origin", &plan.target_branch], args.dry_run)?;
    git::run(
        ["tag", "-a", &plan.tag, "-m", &plan.release_name],
        args.dry_run,
    )?;
    git::run(["push", "origin", &plan.tag], args.dry_run)?;

    // Publishing is intentionally last: by this point the target branch and tag already represent the release.
    if args.skip_github_release {
        println!("skipping GitHub Release creation for {}", plan.tag);
    } else {
        github::create_release(&plan, args.assets.as_deref(), args.dry_run)?;
    }

    if config.release.cleanup && !args.keep_branch {
        git::run(
            ["push", "origin", "--delete", &plan.release_branch],
            args.dry_run,
        )?;
        if git::branch_exists(&plan.release_branch) || args.dry_run {
            git::run(["branch", "-D", &plan.release_branch], args.dry_run)?;
        }
    }

    output::write_github_outputs(&plan)?;
    Ok(())
}
