//! Machine-readable release manifest writer. This helps CI consume outputs without scraping logs.

use crate::artifacts::{ArtifactRecord, SkippedPlatform};
use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct Manifest {
    pub tool: &'static str,
    pub artifacts: Vec<ArtifactRecord>,
    pub skipped: Vec<SkippedPlatform>,
}

pub fn write(
    path: &Path,
    artifacts: Vec<ArtifactRecord>,
    skipped: Vec<SkippedPlatform>,
) -> Result<()> {
    let manifest = Manifest {
        tool: "tauri-release",
        artifacts,
        skipped,
    };
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok(())
}
