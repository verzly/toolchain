//! Machine-readable release manifest writer. This helps CI consume outputs without scraping logs.

use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;

use crate::artifacts::ArtifactRecord;

#[derive(Debug, Serialize)]
pub struct Manifest {
    pub tool: &'static str,
    pub artifacts: Vec<ArtifactRecord>,
}

pub fn write(path: &Path, artifacts: Vec<ArtifactRecord>) -> Result<()> {
    let manifest = Manifest { tool: "tauri-release", artifacts };
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;
    Ok(())
}
