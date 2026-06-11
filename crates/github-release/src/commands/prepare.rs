//! Implements the prepare phase: create the temporary branch, apply version changes, and stop before the project-specific build starts.

use crate::cli::PrepareArgs;
use crate::config;
use crate::domain;
use crate::git;
use crate::output;
use crate::version_files;
use anyhow::{Context, Result};
use std::process::{Command, Stdio};

// Keep every generated version change on the temporary branch.
// The target branch is not touched until the later finalize step succeeds.
pub fn run(args: PrepareArgs) -> Result<()> {
    if args.force_branch && args.reuse_branch {
        anyhow::bail!("use either --force-branch or --reuse-branch, not both");
    }

    let config = config::load(&args.config)?.source_view();
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

    let local_release_branch_exists = git::branch_exists(&plan.release_branch);
    let remote_release_branch_exists = git::remote_branch_exists(&plan.release_branch);

    if (local_release_branch_exists || remote_release_branch_exists)
        && !args.force_branch
        && !args.reuse_branch
    {
        anyhow::bail!("release branch already exists: {}", plan.release_branch);
    }
    if git::tag_exists(&plan.tag) || git::remote_tag_exists(&plan.tag) {
        anyhow::bail!("release tag already exists: {}", plan.tag);
    }

    if args.reuse_branch && (local_release_branch_exists || remote_release_branch_exists) {
        checkout_existing_release_branch(
            &plan.release_branch,
            remote_release_branch_exists,
            args.dry_run,
        )?;
    } else {
        git::run(["checkout", &plan.target_branch], args.dry_run)?;
        git::run(
            ["pull", "--ff-only", "origin", &plan.target_branch],
            args.dry_run,
        )?;
        git::run(["checkout", "-B", &plan.release_branch], args.dry_run)?;
    }

    // Version updates happen before the project build so downstream jobs build the exact release contents.
    version_files::update_all(&config.files, &plan, args.dry_run)?;
    run_prepare_commands(&config.prepare_commands, args.dry_run)?;

    git::run(["add", "--all"], args.dry_run)?;
    if args.dry_run || git::has_staged_changes()? {
        git::run(["commit", "-m", &plan.commit_message], args.dry_run)?;
    } else {
        println!("no configured version file changes to commit");
    }
    git::run(
        ["push", "--set-upstream", "origin", &plan.release_branch],
        args.dry_run,
    )?;

    output::write_github_outputs(&plan)?;

    Ok(())
}

fn run_prepare_commands(commands: &[String], dry_run: bool) -> Result<()> {
    for command in commands {
        let command = command.trim();
        if command.is_empty() {
            continue;
        }

        println!("run prepare command: {command}");
        if dry_run {
            continue;
        }

        let status = shell_command(command)
            .stdin(Stdio::null())
            .status()
            .with_context(|| format!("failed to run prepare command: {command}"))?;

        if !status.success() {
            anyhow::bail!("prepare command failed: {command}");
        }
    }

    Ok(())
}

fn shell_command(command: &str) -> Command {
    #[cfg(windows)]
    {
        let mut shell = Command::new("cmd");
        shell.args(["/d", "/s", "/c", command]);
        shell
    }

    #[cfg(not(windows))]
    {
        let mut shell = Command::new("sh");
        shell.args(["-c", command]);
        shell
    }
}

fn checkout_existing_release_branch(
    release_branch: &str,
    remote_release_branch_exists: bool,
    dry_run: bool,
) -> Result<()> {
    if remote_release_branch_exists {
        let remote_release_branch = format!("origin/{release_branch}");
        let release_refspec = format!("{release_branch}:refs/remotes/origin/{release_branch}");
        git::run(["fetch", "origin", &release_refspec], dry_run)?;
        git::run(
            ["checkout", "-B", release_branch, &remote_release_branch],
            dry_run,
        )?;
    } else {
        git::run(["checkout", release_branch], dry_run)?;
    }

    Ok(())
}
