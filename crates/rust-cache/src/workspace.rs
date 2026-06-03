//! Workspace detection helpers. Cargo metadata is preferred, with Git and current-directory fallbacks for non-standard projects.

use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Clone, Debug)]
pub struct Workspace {
    pub root: PathBuf,
    pub package: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    workspace_root: PathBuf,
    packages: Vec<CargoPackage>,
    resolve: Option<CargoResolve>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct CargoResolve {
    root: Option<String>,
}

pub fn detect() -> Result<Workspace> {
    if let Some(workspace) = from_cargo_metadata()? {
        return Ok(workspace);
    }
    if let Some(root) = git_root() {
        return Ok(Workspace {
            root,
            package: None,
        });
    }
    Ok(Workspace {
        root: std::env::current_dir()?,
        package: None,
    })
}

fn from_cargo_metadata() -> Result<Option<Workspace>> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .stdin(Stdio::null())
        .output();

    let Ok(output) = output else {
        return Ok(None);
    };
    if !output.status.success() {
        return Ok(None);
    }

    let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)?;
    let package = metadata
        .resolve
        .and_then(|resolve| resolve.root)
        .and_then(|id| {
            metadata
                .packages
                .into_iter()
                .find(|package| package.id == id)
        })
        .map(|package| package.name);

    Ok(Some(Workspace {
        root: metadata.workspace_root,
        package,
    }))
}

fn git_root() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if root.is_empty() {
        None
    } else {
        Some(PathBuf::from(root))
    }
}
