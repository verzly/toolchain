//! Docker/Podman command construction for platforms where containers are realistic. Apple targets stay host-first by design.

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::config::{ContainerEngine, PlatformConfig};


// Tauri platform builds are not equally portable; container use is explicit per platform.
pub fn run(
    engine: ContainerEngine,
    project_root: &Path,
    platform: &PlatformConfig,
    dry_run: bool,
) -> Result<()> {
    let image = platform
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

    for (key, value) in &platform.env {
        args.push("-e".to_string());
        args.push(format!("{key}={value}"));
    }

    args.push(image.to_string());
    args.push("sh".to_string());
    args.push("-lc".to_string());
    args.push(platform.command.clone());

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
