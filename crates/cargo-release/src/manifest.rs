//! Release manifest writer. The manifest records what was built without trying to replace human release notes.

use crate::artifacts::ArtifactRecord;
use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct Manifest {
    pub tool: &'static str,
    pub artifacts: Vec<ArtifactRecord>,
}

pub fn write(path: &Path, artifacts: Vec<ArtifactRecord>) -> Result<()> {
    let manifest = Manifest {
        tool: "cargo-release",
        artifacts,
    };
    fs::write(
        path,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok(())
}
