//! GitHub CLI integration. The project uses `gh` instead of a custom API client so authentication matches local and CI usage.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::domain::ReleasePlan;

pub fn create_release(plan: &ReleasePlan, assets_dir: Option<&Path>, dry_run: bool) -> Result<()> {
    let mut args = vec![
        "release".to_string(),
        "create".to_string(),
        plan.tag.clone(),
        "--title".to_string(),
        plan.release_name.clone(),
        "--generate-notes".to_string(),
        "--target".to_string(),
        plan.target_branch.clone(),
    ];

    if plan.prerelease {
        args.push("--prerelease".to_string());
    }

    run_gh(&args, dry_run)?;

    if let Some(dir) = assets_dir {
        let assets = collect_assets(dir)?;
        if !assets.is_empty() {
            let mut upload_args = vec!["release".to_string(), "upload".to_string(), plan.tag.clone()];
            upload_args.extend(assets.iter().map(|path| path.display().to_string()));
            upload_args.push("--clobber".to_string());
            run_gh(&upload_args, dry_run)?;
        }
    }

    Ok(())
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
    for entry in std::fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            collect_assets_recursive(&path, assets)?;
        } else if path.is_file() {
            assets.push(path);
        }
    }
    Ok(())
}
