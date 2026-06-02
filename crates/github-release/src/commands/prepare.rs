//! Implements the prepare phase: create the temporary branch, apply version changes, and stop before the project-specific build starts.

use anyhow::Result;

use crate::cli::PrepareArgs;
use crate::config;
use crate::domain;
use crate::git;
use crate::output;
use crate::version_files;


// Keep every generated version change on the temporary branch.
// The target branch is not touched until the later finalize step succeeds.
pub fn run(args: PrepareArgs) -> Result<()> {
    let config = config::load(&args.config)?;
    let mut plan = domain::build_plan(
        &config,
        &args.version,
        args.target_branch.as_deref(),
        args.release_branch.as_deref(),
        None,
    )?;

    if let Some(message) = args.commit_message.as_ref() {
        plan.commit_message = domain::render_template(message, &plan.tag, &plan.version_text);
    }

    output::print_plan(&plan);

    if !args.dry_run {
        git::ensure_clean_worktree()?;
    }

    git::run(["fetch", "origin", &plan.target_branch], args.dry_run)?;

    if (git::branch_exists(&plan.release_branch) || git::remote_branch_exists(&plan.release_branch)) && !args.force_branch {
        anyhow::bail!("release branch already exists: {}", plan.release_branch);
    }
    if git::tag_exists(&plan.tag) || git::remote_tag_exists(&plan.tag) {
        anyhow::bail!("release tag already exists: {}", plan.tag);
    }

    git::run(["checkout", &plan.target_branch], args.dry_run)?;
    git::run(["pull", "--ff-only", "origin", &plan.target_branch], args.dry_run)?;
    git::run(["checkout", "-B", &plan.release_branch], args.dry_run)?;

    // Version updates happen before the project build so downstream jobs build the exact release contents.
    version_files::update_all(&config.files, &plan, args.dry_run)?;

    git::run(["add", "--all"], args.dry_run)?;
    if args.dry_run || git::has_staged_changes()? {
        git::run(["commit", "-m", &plan.commit_message], args.dry_run)?;
    } else {
        println!("no configured version file changes to commit");
    }
    git::run(["push", "--set-upstream", "origin", &plan.release_branch], args.dry_run)?;

    output::write_github_outputs(&plan)?;

    Ok(())
}
