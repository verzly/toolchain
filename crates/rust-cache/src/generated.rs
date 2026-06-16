//! Discovery of reproducible build output directories that should not remain in the repository tree.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn discover_generated_outputs(
    workspace_root: &Path,
    cache_root: &Path,
    configured_paths: &[PathBuf],
) -> Result<Vec<PathBuf>> {
    let workspace_root = workspace_root.to_path_buf();
    let cache_root = absolutize(workspace_root.as_path(), cache_root);
    let mut paths = BTreeSet::new();

    for configured in configured_paths {
        let path = absolutize(workspace_root.as_path(), configured);
        if path.exists() && !is_cache_path(&path, &cache_root) {
            paths.insert(path);
        }
    }

    collect_from_directory(&workspace_root, &cache_root, &mut paths)?;

    Ok(paths.into_iter().collect())
}

fn collect_from_directory(
    directory: &Path,
    cache_root: &Path,
    paths: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    if is_ignored_directory(directory, cache_root) {
        return Ok(());
    }

    if is_rust_target_dir(directory) || is_tauri_generated_output_dir(directory) {
        paths.insert(directory.to_path_buf());
        return Ok(());
    }

    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?
    {
        let entry =
            entry.with_context(|| format!("failed to read entry in {}", directory.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_from_directory(&path, cache_root, paths)?;
        }
    }

    Ok(())
}

fn is_ignored_directory(directory: &Path, cache_root: &Path) -> bool {
    directory.file_name().and_then(|name| name.to_str()) == Some(".git")
        || is_cache_path(directory, cache_root)
}

fn is_cache_path(path: &Path, cache_root: &Path) -> bool {
    path == cache_root || path.starts_with(cache_root)
}

fn is_rust_target_dir(directory: &Path) -> bool {
    directory.file_name().and_then(|name| name.to_str()) == Some("target")
}

fn is_tauri_generated_output_dir(directory: &Path) -> bool {
    let Some(name) = directory.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    is_known_mobile_build_output_name(name)
        && contains_component(directory, "src-tauri")
        && contains_component(directory, "gen")
}

fn is_known_mobile_build_output_name(name: &str) -> bool {
    matches!(
        name,
        "build"
            | ".gradle"
            | ".cxx"
            | ".externalNativeBuild"
            | ".kotlin"
            | "captures"
            | "DerivedData"
            | ".build"
    )
}

fn contains_component(path: &Path, expected: &str) -> bool {
    path.components()
        .any(|component| component.as_os_str().to_string_lossy() == expected)
}

fn absolutize(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rust-cache-generated-{name}-{suffix}"));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn discovers_rust_and_tauri_generated_outputs_outside_cache() {
        let root = temp_dir("discover");
        fs::create_dir_all(root.join("target")).unwrap();
        fs::create_dir_all(root.join("apps/desktop/src-tauri/gen/android/app/build")).unwrap();
        fs::create_dir_all(root.join("apps/desktop/src-tauri/gen/android/.gradle")).unwrap();
        fs::create_dir_all(root.join("apps/desktop/src-tauri/gen/android/app/.cxx")).unwrap();
        fs::create_dir_all(root.join("apps/desktop/src-tauri/gen/apple/DerivedData")).unwrap();
        fs::create_dir_all(root.join("apps/desktop/src-tauri/gen/apple/.build")).unwrap();
        fs::create_dir_all(root.join(".cache/rust/packages/app/target")).unwrap();

        let outputs = discover_generated_outputs(&root, &root.join(".cache"), &[]).unwrap();
        let normalized = outputs
            .iter()
            .map(|path| {
                path.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert!(normalized.contains(&"target".to_string()));
        assert!(normalized.contains(&"apps/desktop/src-tauri/gen/android/app/build".to_string()));
        assert!(normalized.contains(&"apps/desktop/src-tauri/gen/android/.gradle".to_string()));
        assert!(normalized.contains(&"apps/desktop/src-tauri/gen/android/app/.cxx".to_string()));
        assert!(normalized.contains(&"apps/desktop/src-tauri/gen/apple/DerivedData".to_string()));
        assert!(normalized.contains(&"apps/desktop/src-tauri/gen/apple/.build".to_string()));
        assert!(!normalized.contains(&".cache/rust/packages/app/target".to_string()));
    }

    #[test]
    fn includes_configured_existing_paths() {
        let root = temp_dir("configured");
        fs::create_dir_all(root.join("apps/mobile/custom-build")).unwrap();

        let outputs = discover_generated_outputs(
            &root,
            &root.join(".cache"),
            &[PathBuf::from("apps/mobile/custom-build")],
        )
        .unwrap();

        assert_eq!(outputs, vec![root.join("apps/mobile/custom-build")]);
    }
}
