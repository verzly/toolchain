//! Implements the source success phase after artifacts already exist. This command merges and tags; publishing can be skipped.

use crate::cli::{FinalizeArgs, FinalizeBatchArgs, MergeStrategy};
use crate::config;
use crate::domain::{self, ReleasePlan};
use crate::git;
use crate::github;
use crate::output;
use anyhow::{Context, Result};
use semver::Version;

#[derive(Clone, Debug)]
struct SourceMergePlan {
    target_branch: String,
    release_branch: String,
    merge_message: String,
}

impl From<&ReleasePlan> for SourceMergePlan {
    fn from(plan: &ReleasePlan) -> Self {
        Self {
            target_branch: plan.target_branch.clone(),
            release_branch: plan.release_branch.clone(),
            merge_message: plan.merge_message.clone(),
        }
    }
}

// Finalize is intentionally ordered from safest to most public operation:
// merge first, then tag, then optionally publish a GitHub Release from that tag.
pub fn run(args: FinalizeArgs) -> Result<()> {
    if args.notes.is_some() && args.notes_file.is_some() {
        anyhow::bail!("use either --notes or --notes-file, not both");
    }

    let config = config::load(&args.config)?.source_view();
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

    let source_merge = SourceMergePlan::from(&plan);
    let remote_release_branch = fetch_and_checkout_target_branch(&source_merge, args.dry_run)?;
    merge_release_branch(
        &source_merge,
        args.merge_strategy,
        &remote_release_branch,
        args.dry_run,
    )?;
    git::run(
        ["push", "origin", &source_merge.target_branch],
        args.dry_run,
    )?;
    git::run(
        ["tag", "-a", &plan.tag, "-m", &plan.release_name],
        args.dry_run,
    )?;
    git::run(["push", "origin", &plan.tag], args.dry_run)?;

    // Publishing is intentionally last: by this point the target branch and tag already represent the release.
    if args.skip_github_release {
        println!("skipping GitHub Release creation for {}", plan.tag);
    } else {
        github::create_release(
            &plan,
            args.assets.as_deref(),
            github::ReleaseNotesInput {
                body: args.notes.as_deref(),
                file: args.notes_file.as_deref(),
            },
            args.dry_run,
        )?;
        let floating_tag_options = github::FloatingTagOptions::for_plan(&plan).with_overrides(
            args.update_floating_tags,
            args.update_latest_tag,
            args.update_next_tag,
        );
        if floating_tag_options.any() {
            github::refresh_floating_tags_for_plan(&plan, floating_tag_options, args.dry_run)?;
        }
    }

    if config.release.cleanup && !args.keep_branch {
        delete_release_branch(&source_merge.release_branch, args.dry_run)?;
    }

    output::write_github_outputs(&plan)?;
    Ok(())
}

pub fn run_batch(args: FinalizeBatchArgs) -> Result<()> {
    let release_branch = args.release_branch.clone();
    let keep_branch = args.keep_branch;
    let dry_run = args.dry_run;

    let result = run_batch_inner(args);

    if result.is_err() && !keep_branch {
        if let Err(cleanup_error) = delete_release_branch(&release_branch, dry_run) {
            eprintln!("failed to delete release branch after finalize error: {cleanup_error}");
        }
    }

    result
}

fn run_batch_inner(args: FinalizeBatchArgs) -> Result<()> {
    let clean_version = args.version.strip_prefix('v').unwrap_or(&args.version);
    let version = Version::parse(clean_version)
        .with_context(|| format!("invalid SemVer version: {}", args.version))?;

    if args.source_tags.iter().any(|tag| tag.trim().is_empty()) {
        anyhow::bail!("source tags must not be empty");
    }

    let source_merge = SourceMergePlan {
        target_branch: args.target_branch,
        release_branch: args.release_branch,
        merge_message: args
            .merge_message
            .unwrap_or_else(|| format!("chore(release): merge v{version}")),
    };

    println!("version:        {version}");
    println!("target branch:  {}", source_merge.target_branch);
    println!("release branch: {}", source_merge.release_branch);
    println!("source tags:    {}", args.source_tags.join(", "));

    if !args.dry_run {
        git::ensure_clean_worktree()?;
    }

    let remote_release_branch = fetch_and_checkout_target_branch(&source_merge, args.dry_run)?;
    squash_merge_release_branch(&source_merge, &remote_release_branch, args.dry_run)?;
    let source_tags = args
        .source_tags
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    push_target_branch_and_source_tags(&source_merge.target_branch, &source_tags, args.dry_run)?;

    if !args.keep_branch {
        delete_release_branch(&source_merge.release_branch, args.dry_run)?;
    }

    Ok(())
}

fn fetch_and_checkout_target_branch(plan: &SourceMergePlan, dry_run: bool) -> Result<String> {
    let remote_release_branch = format!("origin/{}", plan.release_branch);
    let release_refspec = format!(
        "{}:refs/remotes/origin/{}",
        plan.release_branch, plan.release_branch
    );

    git::run(["fetch", "origin", &plan.target_branch], dry_run)?;
    git::run(["fetch", "origin", &release_refspec], dry_run)?;
    git::run(["checkout", &plan.target_branch], dry_run)?;
    git::run(
        ["pull", "--ff-only", "origin", &plan.target_branch],
        dry_run,
    )?;

    Ok(remote_release_branch)
}

fn merge_release_branch(
    plan: &SourceMergePlan,
    strategy: MergeStrategy,
    remote_release_branch: &str,
    dry_run: bool,
) -> Result<()> {
    match strategy {
        MergeStrategy::Squash => squash_merge_release_branch(plan, remote_release_branch, dry_run),
        MergeStrategy::NoFf => no_ff_merge_release_branch(plan, remote_release_branch, dry_run),
    }
}

fn no_ff_merge_release_branch(
    plan: &SourceMergePlan,
    remote_release_branch: &str,
    dry_run: bool,
) -> Result<()> {
    git::run(
        [
            "merge",
            "--no-ff",
            remote_release_branch,
            "-m",
            &plan.merge_message,
        ],
        dry_run,
    )
}

fn squash_merge_release_branch(
    plan: &SourceMergePlan,
    remote_release_branch: &str,
    dry_run: bool,
) -> Result<()> {
    let summary = release_branch_commit_summary(plan, remote_release_branch, dry_run)?;

    git::run(["merge", "--squash", remote_release_branch], dry_run)?;

    if !dry_run && !git::has_staged_changes()? {
        println!(
            "release branch {} has no source changes to squash; target branch already contains the release contents",
            plan.release_branch
        );
        return Ok(());
    }

    let body = squash_merge_body(&plan.release_branch, &summary);
    let args = vec![
        "commit".to_string(),
        "-m".to_string(),
        plan.merge_message.clone(),
        "-m".to_string(),
        body,
    ];
    git::run(args, dry_run)
}

fn push_target_branch_and_source_tags(
    target_branch: &str,
    tags: &[&str],
    dry_run: bool,
) -> Result<()> {
    git::run(["push", "origin", target_branch], dry_run)?;

    for tag in tags {
        git::run(["tag", "-a", *tag, "-m", *tag], dry_run)?;
    }

    let mut push_args = vec!["push".to_string(), "origin".to_string()];
    push_args.extend(tags.iter().map(|tag| (*tag).to_string()));
    git::run(push_args, dry_run)
}

fn delete_release_branch(release_branch: &str, dry_run: bool) -> Result<()> {
    if git::remote_branch_exists(release_branch) || dry_run {
        git::run(["push", "origin", "--delete", release_branch], dry_run)?;
    } else {
        println!("remote release branch does not exist: {release_branch}");
    }

    if git::branch_exists(release_branch) || dry_run {
        git::run(["branch", "-D", release_branch], dry_run)?;
    }

    Ok(())
}

fn release_branch_commit_summary(
    plan: &SourceMergePlan,
    remote_release_branch: &str,
    dry_run: bool,
) -> Result<String> {
    if dry_run {
        return Ok("- Release branch commits would be listed here in a real run.".to_string());
    }

    let range = format!("origin/{}..{}", plan.target_branch, remote_release_branch);
    let summary = git::output(["log", "--reverse", "--format=- %s (%h)", &range])?;

    if summary.trim().is_empty() {
        Ok("- No release branch commits found.".to_string())
    } else {
        Ok(summary)
    }
}

fn squash_merge_body(release_branch: &str, summary: &str) -> String {
    format!("Release branch: {release_branch}\n\nSquashed commits:\n{summary}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squash_merge_body_contains_branch_and_commit_summary() {
        let body = squash_merge_body(
            "release/all-v0.1.0",
            "- chore(release): prepare cargo-release-v0.1.0 (abc1234)",
        );

        assert!(body.contains("Release branch: release/all-v0.1.0"));
        assert!(body.contains("Squashed commits:"));
        assert!(body.contains("prepare cargo-release-v0.1.0"));
    }
}
