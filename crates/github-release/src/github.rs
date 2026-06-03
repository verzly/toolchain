//! GitHub CLI integration. The project uses `gh` instead of a custom API client so authentication matches local and CI usage.

use crate::config::NotesMode;
use crate::domain::ReleasePlan;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn create_release(plan: &ReleasePlan, assets_dir: Option<&Path>, dry_run: bool) -> Result<()> {
    let notes_file = if plan.github.generate_notes && plan.github.source_repository.is_some() {
        Some(write_external_notes_file(plan, dry_run)?)
    } else {
        None
    };

    let mut args = vec![
        "release".to_string(),
        "create".to_string(),
        plan.tag.clone(),
        "--title".to_string(),
        plan.release_name.clone(),
        "--target".to_string(),
        plan.target_branch.clone(),
    ];

    if let Some(repository) = plan.github.target_repository.as_ref() {
        args.push("--repo".to_string());
        args.push(repository.clone());
    }

    if let Some(path) = notes_file.as_ref() {
        args.push("--notes-file".to_string());
        args.push(path.display().to_string());
    } else if plan.github.generate_notes {
        args.push("--generate-notes".to_string());
    }

    if plan.prerelease {
        args.push("--prerelease".to_string());
    }

    if !plan.latest {
        args.push("--latest=false".to_string());
    }

    run_gh(&args, dry_run)?;

    if let Some(dir) = assets_dir {
        let assets = collect_assets(dir)?;
        if !assets.is_empty() {
            let mut upload_args = vec![
                "release".to_string(),
                "upload".to_string(),
                plan.tag.clone(),
            ];
            if let Some(repository) = plan.github.target_repository.as_ref() {
                upload_args.push("--repo".to_string());
                upload_args.push(repository.clone());
            }
            upload_args.extend(assets.iter().map(|path| path.display().to_string()));
            upload_args.push("--clobber".to_string());
            run_gh(&upload_args, dry_run)?;
        }
    }

    Ok(())
}

fn write_external_notes_file(plan: &ReleasePlan, dry_run: bool) -> Result<PathBuf> {
    let source_repository = plan
        .github
        .source_repository
        .as_ref()
        .context("source repository is required for external release notes")?;

    let generated = if dry_run {
        format!(
            "Generated release notes would be requested from `{source_repository}` for `{}`.",
            plan.github.source_tag
        )
    } else if plan.github.notes.mode == NotesMode::Scoped {
        generate_scoped_notes_from_git(plan).unwrap_or_else(|error| {
            fallback_notes(
                source_repository,
                &plan.github.source_tag,
                &error.to_string(),
            )
        })
    } else {
        generate_notes_from_source(source_repository, &plan.github.source_tag).unwrap_or_else(
            |error| {
                fallback_notes(
                    source_repository,
                    &plan.github.source_tag,
                    &error.to_string(),
                )
            },
        )
    };

    let mut body = String::new();
    body.push_str(&generated);
    body.push_str("\n\n---\n\n");
    body.push_str(&format!(
        "Source changes for this release are maintained in `{source_repository}`. Pull request links in \
         these notes intentionally point to that source repository, even when the release itself is \
         published from a distribution repository.\n"
    ));

    let path = std::env::temp_dir().join(format!("github-release-{}-notes.md", plan.tag));
    fs::write(&path, body)
        .with_context(|| format!("failed to write release notes file {}", path.display()))?;
    Ok(path)
}

#[derive(Deserialize)]
struct GeneratedNotes {
    body: Option<String>,
}

fn generate_notes_from_source(repository: &str, tag: &str) -> Result<String> {
    let endpoint = format!("repos/{repository}/releases/generate-notes");
    let output = Command::new("gh")
        .args(["api", &endpoint, "-f", &format!("tag_name={tag}")])
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("failed to generate release notes from {repository}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh api failed while generating release notes from {repository}: {stderr}");
    }

    let notes: GeneratedNotes = serde_json::from_slice(&output.stdout)
        .context("failed to parse generated release notes response")?;
    Ok(notes.body.unwrap_or_default())
}

#[derive(Clone, Debug)]
struct CommitEntry {
    hash: String,
    subject: String,
    paths: Vec<String>,
}

fn generate_scoped_notes_from_git(plan: &ReleasePlan) -> Result<String> {
    let previous_tag = previous_source_tag(plan)?;
    let range = previous_tag
        .as_ref()
        .map(|tag| format!("{tag}..{}", plan.github.source_tag))
        .unwrap_or_else(|| plan.github.source_tag.clone());
    let commits = commits_in_range(&range)?;
    let include_scopes = normalized_values(&plan.github.notes.include_scopes);
    let include_paths = normalized_paths(&plan.github.notes.include_paths);

    let mut included = Vec::new();
    for commit in commits {
        if commit_matches(&commit, &include_scopes, &include_paths) {
            included.push(commit);
        }
    }

    let mut body = String::new();
    body.push_str("## What's changed\n\n");

    if included.is_empty() {
        body.push_str("No package-specific changes were detected for this release.\n");
    } else {
        for commit in included {
            let short_hash = commit.hash.chars().take(7).collect::<String>();
            body.push_str(&format!("- {} (`{short_hash}`)\n", commit.subject));
        }
    }

    body.push('\n');
    if let Some(previous_tag) = previous_tag {
        body.push_str(&format!(
            "Compared source tags: `{previous_tag}` → `{}`.\n",
            plan.github.source_tag
        ));
    } else {
        body.push_str(&format!(
            "Compared source tag: `{}`. No earlier matching source tag was found.\n",
            plan.github.source_tag
        ));
    }

    Ok(body)
}

fn previous_source_tag(plan: &ReleasePlan) -> Result<Option<String>> {
    let pattern = format!(
        "{}*{}",
        plan.github.source_tag_prefix, plan.github.source_tag_suffix
    );
    let output = git_output(["tag", "--list", &pattern, "--sort=-creatordate"])?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .find(|tag| *tag != plan.github.source_tag)
        .map(ToOwned::to_owned))
}

fn commits_in_range(range: &str) -> Result<Vec<CommitEntry>> {
    let output = git_output(["log", "--format=%H%x1f%s", range])?;
    let mut commits = Vec::new();

    for line in output.lines() {
        let Some((hash, subject)) = line.split_once('\u{1f}') else {
            continue;
        };
        commits.push(CommitEntry {
            hash: hash.to_string(),
            subject: subject.to_string(),
            paths: commit_paths(hash)?,
        });
    }

    Ok(commits)
}

fn commit_paths(hash: &str) -> Result<Vec<String>> {
    let output = git_output(["show", "--format=", "--name-only", "--no-renames", hash])?;
    Ok(output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn commit_matches(
    commit: &CommitEntry,
    include_scopes: &[String],
    include_paths: &[String],
) -> bool {
    if let Some(scope) = commit_scope(&commit.subject) {
        let normalized = scope.to_ascii_lowercase();
        if normalized == "all" || include_scopes.iter().any(|scope| scope == &normalized) {
            return true;
        }
    }

    commit.paths.iter().any(|path| {
        let normalized = normalize_path(path);
        include_paths
            .iter()
            .any(|prefix| normalized.starts_with(prefix))
    })
}

fn commit_scope(subject: &str) -> Option<&str> {
    let open = subject.find('(')?;
    let close = subject[open + 1..].find(')')? + open + 1;
    let suffix = subject.get(close + 1..)?;
    if suffix.starts_with(':') || suffix.starts_with("!:") {
        Some(&subject[open + 1..close])
    } else {
        None
    }
}

fn normalized_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn normalized_paths(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .map(|path| normalize_path(path))
        .filter(|path| !path.is_empty())
        .collect()
}

fn normalize_path(path: &str) -> String {
    path.trim().trim_start_matches("./").replace('\\', "/")
}

fn git_output<const N: usize>(args: [&str; N]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .context("failed to run git command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git command failed: {stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn fallback_notes(repository: &str, tag: &str, error: &str) -> String {
    // External release notes are a convenience, not a reason to block publishing.
    // The fallback keeps the public release honest when the source repository is private or GitHub cannot generate notes.
    format!(
        "What's changed\n\nRelease notes could not be generated automatically from `{repository}` for \
         `{tag}`.\n\nReason: {error}\n"
    )
}

fn run_gh(args: &[String], dry_run: bool) -> Result<()> {
    if dry_run {
        println!("gh {}", args.join(" "));
        return Ok(());
    }

    let status = Command::new("gh")
        .args(args)
        .stdin(Stdio::null())
        .status()
        .with_context(|| format!("failed to run gh {}", args.join(" ")))?;

    if !status.success() {
        anyhow::bail!("gh command failed: gh {}", args.join(" "));
    }

    Ok(())
}

fn collect_assets(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        anyhow::bail!("asset directory does not exist: {}", dir.display());
    }

    let mut assets = Vec::new();
    collect_assets_recursive(dir, &mut assets)?;
    assets.sort();
    Ok(assets)
}

fn collect_assets_recursive(dir: &Path, assets: &mut Vec<PathBuf>) -> Result<()> {
    // cargo-release groups artifacts by target under dist/. GitHub Releases use
    // the file name as the asset name, so nested target folders are safe here.
    for entry in
        std::fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))?
    {
        let path = entry?.path();
        if path.is_dir() {
            collect_assets_recursive(&path, assets)?;
        } else if path.is_file() {
            assets.push(path);
        }
    }
    Ok(())
}
