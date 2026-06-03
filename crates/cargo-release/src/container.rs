//! Docker/Podman command construction. Container isolation is an execution strategy, not part of target planning.

use crate::config::{ContainerEngine, TargetConfig};
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};


// Container execution mounts the repository at a stable path so target commands stay portable.
pub fn run(
    engine: ContainerEngine,
    project_root: &Path,
    target: &TargetConfig,
    dry_run: bool,
) -> Result<()> {
    let image = target
        .image
        .as_deref()
        .context("container strategy requires image")?;

    let mut args = vec![
        "run".to_string(),
        "--rm".to_string(),
        "-v".to_string(),
        format!("{}:/workspace", project_root.canonicalize()?.display()),
        "-w".to_string(),
        "/workspace".to_string(),
    ];

    for (key, value) in &target.env {
        args.push("-e".to_string());
        args.push(format!("{key}={value}"));
    }

    args.push(image.to_string());
    args.push("sh".to_string());
    args.push("-lc".to_string());
    args.push(target.command.clone());

    if dry_run {
        println!("{} {}", engine.executable(), args.join(" "));
        return Ok(());
    }

    let status = Command::new(engine.executable())
        .args(args)
        .stdin(Stdio::null())
        .status()
        .with_context(|| format!("failed to run {}", engine.executable()))?;

    if !status.success() {
        anyhow::bail!("container build failed");
    }

    Ok(())
}
