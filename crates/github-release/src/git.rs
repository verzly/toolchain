//! Small wrapper around the Git CLI. Keeping Git calls here makes dry-run output and error handling consistent.

use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::process::{Command, Stdio};

pub fn run<I, S>(args: I, dry_run: bool) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args: Vec<_> = args.into_iter().collect();
    if dry_run {
        println!("git {}", printable(&args));
        return Ok(());
    }

    let status = Command::new("git")
        .args(&args)
        .stdin(Stdio::null())
        .status()
        .with_context(|| format!("failed to run git {}", printable(&args)))?;

    if !status.success() {
        anyhow::bail!("git command failed: git {}", printable(&args));
    }

    Ok(())
}

pub fn output<I, S>(args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args: Vec<_> = args.into_iter().collect();
    let output = Command::new("git")
        .args(&args)
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("failed to run git {}", printable(&args)))?;

    if !output.status.success() {
        anyhow::bail!("git command failed: git {}", printable(&args));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn ensure_clean_worktree() -> Result<()> {
    let status = output(["status", "--porcelain"])?;
    if !status.is_empty() {
        anyhow::bail!("working tree is not clean");
    }
    Ok(())
}

pub fn has_staged_changes() -> Result<bool> {
    let status = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to check staged git changes")?;

    match status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => anyhow::bail!("git diff --cached --quiet failed"),
    }
}

pub fn branch_exists(name: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", name])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn tag_exists(name: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", &format!("refs/tags/{name}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn remote_branch_exists(name: &str) -> bool {
    remote_ref_exists("--heads", name)
}

pub fn remote_tag_exists(name: &str) -> bool {
    remote_ref_exists("--tags", name)
}

fn remote_ref_exists(kind: &str, name: &str) -> bool {
    Command::new("git")
        .args(["ls-remote", "--exit-code", kind, "origin", name])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn printable<S: AsRef<OsStr>>(args: &[S]) -> String {
    args.iter()
        .map(|arg| arg.as_ref().to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}
