//! GitHub CLI integration. The project uses `gh` instead of a custom API client so authentication matches local and CI usage.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::domain::ReleasePlan;

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
        "Source changes for this release are maintained in `{source_repository}`. Pull request links in these notes intentionally point to that source repository, even when the release itself is published from a distribution repository.\n"
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

fn fallback_notes(repository: &str, tag: &str, error: &str) -> String {
    // External release notes are a convenience, not a reason to block publishing.
    // The fallback keeps the public release honest when the source repository is private or GitHub cannot generate notes.
    format!(
        "What's changed\n\nRelease notes could not be generated automatically from `{repository}` for `{tag}`.\n\nReason: {error}\n"
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
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
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
