//! Artifact discovery and copying. Build commands produce files; this module decides what becomes part of `dist/`.

use crate::checksums;
use anyhow::{Context, Result};
use glob::glob;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[derive(Clone, Debug, Serialize)]
pub struct ArtifactRecord {
    pub target: String,
    pub source: String,
    pub output: String,
    pub sha256: Option<String>,
}

pub struct CollectRequest<'a> {
    pub target_name: &'a str,
    pub project_root: &'a Path,
    pub out_dir: &'a Path,
    pub patterns: &'a [String],
    pub write_checksums: bool,
    pub binary: &'a str,
    pub version: &'a str,
    pub name_template: &'a str,
}

pub fn collect(request: CollectRequest<'_>) -> Result<Vec<ArtifactRecord>> {
    let target_out = request.out_dir.join(request.target_name);
    fs::create_dir_all(&target_out)?;

    let mut records = Vec::new();
    for pattern in request.patterns {
        let full_pattern = resolve_artifact_pattern(request.project_root, pattern);
        for entry in glob(&full_pattern.display().to_string()).context("invalid artifact glob")? {
            let source = entry?;
            if !source.is_file() {
                continue;
            }
            let file_name = rendered_artifact_name(
                &source,
                request.target_name,
                request.binary,
                request.version,
                request.name_template,
            )?;
            let output = target_out.join(&file_name);
            fs::copy(&source, &output).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source.display(),
                    output.display()
                )
            })?;
            let sha256 = if request.write_checksums {
                let hash = checksums::sha256_file(&output)?;
                let checksum_path = output.with_file_name(format!("{file_name}.sha256"));
                fs::write(checksum_path, format!("{hash}  {}\n", file_name))?;
                Some(hash)
            } else {
                None
            };
            records.push(ArtifactRecord {
                target: request.target_name.to_string(),
                source: source.display().to_string(),
                output: output.display().to_string(),
                sha256,
            });
        }
    }

    if records.is_empty() {
        anyhow::bail!("no artifacts found for target {}", request.target_name);
    }

    Ok(records)
}

fn resolve_artifact_pattern(project_root: &Path, pattern: &str) -> PathBuf {
    let pattern = if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        if let Some(rest) = pattern.strip_prefix("target/") {
            PathBuf::from(target_dir).join(rest).display().to_string()
        } else {
            pattern
                .replace("{cargo_target_dir}", &target_dir)
                .replace("{target_dir}", &target_dir)
        }
    } else {
        pattern
            .replace("{cargo_target_dir}", "target")
            .replace("{target_dir}", "target")
    };

    let path = PathBuf::from(pattern);
    if path.is_absolute() {
        path
    } else {
        project_root.join(path)
    }
}

fn rendered_artifact_name(
    source: &Path,
    target_name: &str,
    binary: &str,
    version: &str,
    template: &str,
) -> Result<String> {
    let original = source
        .file_name()
        .context("artifact path has no file name")?
        .to_string_lossy();
    let ext = source
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();

    let rendered = template
        .replace("{binary}", binary)
        .replace("{version}", version.trim_start_matches('v'))
        .replace("{target}", target_name)
        .replace("{ext}", &ext)
        .replace("{original}", &original);

    if rendered.trim().is_empty() {
        anyhow::bail!("artifact name template rendered an empty file name");
    }

    Ok(rendered)
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
        let path = std::env::temp_dir().join(format!("cargo-release-{name}-{suffix}"));
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn collects_artifacts_and_writes_checksums() {
        let root = temp_dir("collect");
        let source_dir = root.join("artifacts");
        let out_dir = root.join("dist");
        std::fs::create_dir_all(&source_dir).expect("create source dir");
        std::fs::write(source_dir.join("demo.bin"), b"release-binary").expect("write artifact");

        let patterns = ["artifacts/*.bin".to_string()];
        let records = collect(CollectRequest {
            target_name: "linux-x64",
            project_root: &root,
            out_dir: &out_dir,
            patterns: &patterns,
            write_checksums: true,
            binary: "demo",
            version: "v1.2.3",
            name_template: "{binary}-v{version}-{target}{ext}",
        })
        .expect("collect artifacts");

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].target, "linux-x64");
        assert!(records[0].sha256.is_some());
        assert!(out_dir.join("linux-x64/demo-v1.2.3-linux-x64.bin").exists());
        assert!(out_dir
            .join("linux-x64/demo-v1.2.3-linux-x64.bin.sha256")
            .exists());
    }

    #[test]
    fn fails_when_no_artifacts_match() {
        let root = temp_dir("missing");
        let patterns = ["artifacts/*.bin".to_string()];
        let error = collect(CollectRequest {
            target_name: "linux-x64",
            project_root: &root,
            out_dir: &root.join("dist"),
            patterns: &patterns,
            write_checksums: false,
            binary: "demo",
            version: "1.2.3",
            name_template: "{original}",
        })
        .expect_err("missing artifacts must fail");

        assert!(error.to_string().contains("no artifacts found"));
    }
}
