//! Artifact collection for desktop bundles and mobile outputs. Patterns are configured instead of hard-coded.

use crate::checksums;
use anyhow::{Context, Result};
use glob::glob;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize)]
pub struct ArtifactRecord {
    pub platform: String,
    pub source: String,
    pub output: String,
    pub sha256: Option<String>,
}

pub fn collect(
    platform_name: &str,
    project_root: &Path,
    out_dir: &Path,
    patterns: &[String],
    write_checksums: bool,
) -> Result<Vec<ArtifactRecord>> {
    let platform_out = out_dir.join(platform_name);
    fs::create_dir_all(&platform_out)?;

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
            let output = platform_out.join(file_name);
            fs::copy(&source, &output).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source.display(),
                    output.display()
                )
            })?;
            let sha256 = if write_checksums {
                let hash = checksums::sha256_file(&output)?;
                fs::write(
                    output.with_extension("sha256"),
                    format!("{hash}  {}\n", file_name.to_string_lossy()),
                )?;
                Some(hash)
            } else {
                None
            };
            records.push(ArtifactRecord {
                platform: platform_name.to_string(),
                source: source.display().to_string(),
                output: output.display().to_string(),
                sha256,
            });
        }
    }

    if records.is_empty() {
        anyhow::bail!("no artifacts found for platform {platform_name}");
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("tauri-release-{name}-{suffix}"));
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn collects_platform_artifacts_and_checksums() {
        let root = temp_dir("collect");
        let source_dir = root.join("src-tauri/target/release/bundle/appimage");
        let out_dir = root.join("dist");
        std::fs::create_dir_all(&source_dir).expect("create source dir");
        std::fs::write(source_dir.join("demo.AppImage"), b"appimage").expect("write artifact");

        let patterns = ["src-tauri/target/release/bundle/**/*.AppImage".to_string()];
        let records =
            collect("linux", &root, &out_dir, &patterns, true).expect("collect artifacts");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].platform, "linux");
        assert!(records[0].sha256.is_some());
        assert!(out_dir.join("linux/demo.AppImage").exists());
        assert!(out_dir.join("linux/demo.sha256").exists());
    }

    #[test]
    fn prepare_out_dir_removes_existing_output() {
        let dir = temp_dir("prepare");
        let stale = dir.join("stale.txt");
        std::fs::write(&stale, "old").expect("write stale file");

        prepare_out_dir(&dir).expect("prepare output dir");

        assert!(dir.exists());
        assert!(!stale.exists());
    }
}
