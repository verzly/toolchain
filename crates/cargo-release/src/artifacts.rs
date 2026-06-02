//! Artifact discovery and copying. Build commands produce files; this module decides what becomes part of `dist/`.

use anyhow::{Context, Result};
use glob::glob;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::checksums;

#[derive(Clone, Debug, Serialize)]
pub struct ArtifactRecord {
    pub target: String,
    pub source: String,
    pub output: String,
    pub sha256: Option<String>,
}

pub fn collect(
    target_name: &str,
    project_root: &Path,
    out_dir: &Path,
    patterns: &[String],
    write_checksums: bool,
) -> Result<Vec<ArtifactRecord>> {
    let target_out = out_dir.join(target_name);
    fs::create_dir_all(&target_out)?;

    let mut records = Vec::new();
    for pattern in patterns {
        let full_pattern = project_root.join(pattern);
        for entry in glob(&full_pattern.display().to_string()).context("invalid artifact glob")? {
            let source = entry?;
            if !source.is_file() {
                continue;
            }
            let file_name = source
                .file_name()
                .context("artifact path has no file name")?;
            let output = target_out.join(file_name);
            fs::copy(&source, &output).with_context(|| {
                format!("failed to copy {} to {}", source.display(), output.display())
            })?;
            let sha256 = if write_checksums {
                let hash = checksums::sha256_file(&output)?;
                fs::write(output.with_extension("sha256"), format!("{hash}  {}\n", file_name.to_string_lossy()))?;
                Some(hash)
            } else {
                None
            };
            records.push(ArtifactRecord {
                target: target_name.to_string(),
                source: source.display().to_string(),
                output: output.display().to_string(),
                sha256,
            });
        }
    }

    if records.is_empty() {
        anyhow::bail!("no artifacts found for target {target_name}");
    }

    Ok(records)
}

pub fn prepare_out_dir(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}
