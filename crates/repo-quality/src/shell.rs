//! Small process helpers for setup commands.

use anyhow::{anyhow, Context, Result};
use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn command_exists(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn succeeds<I, S>(root: &Path, command: &str, args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut process = Command::new(command);
    process.current_dir(root);
    process.stdout(Stdio::null());
    process.stderr(Stdio::null());
    for arg in args {
        process.arg(arg);
    }

    process
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn run<I, S>(root: &Path, command: &str, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut process = Command::new(command);
    process.current_dir(root);
    for arg in args {
        process.arg(arg);
    }

    let status = process
        .status()
        .with_context(|| format!("failed to start {command}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("{command} exited with {status}"))
    }
}

pub fn output<I, S>(root: &Path, command: &str, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut process = Command::new(command);
    process.current_dir(root);
    for arg in args {
        process.arg(arg);
    }

    let output = process
        .output()
        .with_context(|| format!("failed to start {command}"))?;

    if !output.status.success() {
        return Err(anyhow!("{command} exited with {}", output.status));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
