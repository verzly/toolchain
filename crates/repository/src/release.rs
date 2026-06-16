//! Release target helpers for datarose.toml.

use crate::cli::{ReleaseSetArgs, ReleaseStrategyArg, ReleaseWorkflowArg};
use crate::project::{
    normalize_release_targets, read_cargo_package_name, ProjectProfile, ReleaseTarget,
};
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

pub const STRATEGIES: &[&str] = &["same-repo", "distribution-repo", "self-hosted", "custom"];
pub const WORKFLOWS: &[&str] = &["managed", "preserve", "custom"];

pub fn upsert_release_target(
    profile: &mut ProjectProfile,
    args: &ReleaseSetArgs,
) -> Result<String> {
    let target_path = normalize_relative_path(&args.path);
    let full_path = profile.root.join(&target_path);
    if !args.allow_missing_path && !full_path.exists() {
        bail!(
            "release target path does not exist: {}; pass --allow-missing-path to keep it anyway",
            full_path.display()
        );
    }

    let name = args
        .name
        .clone()
        .unwrap_or_else(|| derive_target_name(&target_path));
    let strategy = args
        .strategy
        .map(ReleaseStrategyArg::as_str)
        .map(|value| value.to_string())
        .unwrap_or_else(|| {
            infer_strategy(args.repository.as_deref(), &profile.stored_config.release)
        });
    let workflow = args
        .workflow
        .map(ReleaseWorkflowArg::as_str)
        .unwrap_or("custom")
        .to_string();
    let source_kind = args
        .source_kind
        .clone()
        .unwrap_or_else(|| infer_source_kind(&full_path));
    let cargo_package = args
        .cargo_package
        .clone()
        .or_else(|| detect_cargo_package(&full_path).ok().flatten())
        .unwrap_or_else(|| name.clone());
    let cargo_binary = args
        .cargo_binary
        .clone()
        .unwrap_or_else(|| cargo_package.clone());
    let repository = args.repository.clone().unwrap_or_default();
    let cargo_out_dir = args
        .cargo_out_dir
        .clone()
        .unwrap_or_else(|| format!("dist/{name}"));
    let distribution_path = args
        .distribution_path
        .clone()
        .unwrap_or_else(|| default_distribution_path(&strategy, &name));
    let version_file = args
        .version_file
        .clone()
        .unwrap_or_else(|| default_version_file(&target_path, &source_kind));
    let source_tag_prefix = args
        .source_tag_prefix
        .clone()
        .unwrap_or_else(|| format!("{name}-v"));

    let mut target = ReleaseTarget {
        name: name.clone(),
        path: target_path.clone(),
        workspace: args.workspace.clone().unwrap_or_default(),
        strategy,
        workflow,
        source_kind,
        repository,
        source_repository: None,
        distribution_path,
        cargo_binary,
        cargo_package,
        cargo_out_dir,
        cargo_targets: Vec::new(),
        prepare_commands: Vec::new(),
        version_files: None,
        version_file,
        version_key: "package.version".into(),
        version_value: "{version}".into(),
        source_tag_prefix,
        release_name_prefix: String::new(),
        source_release: None,
        generate_notes: Some(false),
        floating_tags: None,
        latest_tag: None,
        next_tag: None,
        include_scopes: vec![name.clone(), "all".into()],
        include_paths: vec![include_path(&target_path)],
    };

    if target.source_kind != "cargo-package" {
        target.cargo_binary.clear();
        target.cargo_package.clear();
        target.cargo_out_dir.clear();
        target.cargo_targets.clear();
        target.prepare_commands.clear();
        target.version_files = Some(false);
        target.version_file.clear();
        target.version_key.clear();
        target.version_value.clear();
    }

    if target.workflow == "managed" {
        profile.stored_config.release.manage_workflows = true;
    }

    let targets = &mut profile.stored_config.release.targets;
    if let Some(index) = targets
        .iter()
        .position(|existing| existing.name == name || existing.path == target_path)
    {
        targets[index] = merge_existing_target(targets[index].clone(), target);
    } else {
        targets.push(target);
    }
    normalize_release_targets(targets);

    Ok(name)
}

pub fn remove_release_target(
    profile: &mut ProjectProfile,
    target: Option<&str>,
    path: Option<&Path>,
) -> Result<Option<ReleaseTarget>> {
    let normalized_path = path.map(normalize_relative_path);
    let index = profile
        .stored_config
        .release
        .targets
        .iter()
        .position(|release_target| {
            target
                .map(|target| release_target.name == target)
                .unwrap_or(false)
                || normalized_path
                    .as_deref()
                    .map(|path| release_target.path == path)
                    .unwrap_or(false)
        });

    Ok(index.map(|index| profile.stored_config.release.targets.remove(index)))
}

pub fn write_profile_config(profile: &ProjectProfile) -> Result<()> {
    let content = crate::project::render_datarose_config(profile);
    if let Some(parent) = profile.config_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&profile.config_path, content)
        .with_context(|| format!("failed to write {}", profile.config_path.display()))
}

pub fn normalize_relative_path(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        let text = component.as_os_str().to_string_lossy();
        match text.as_ref() {
            "" | "." => {}
            _ => parts.push(text.to_string()),
        }
    }
    if parts.is_empty() {
        ".".into()
    } else {
        parts.join("/")
    }
}

pub fn derive_target_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && *name != ".")
        .unwrap_or("release")
        .to_string()
}

fn infer_strategy(repository: Option<&str>, release: &crate::project::ReleaseConfig) -> String {
    if let Some(repository) = repository {
        if !release.source_repository.is_empty() && repository == release.source_repository.as_str()
        {
            "same-repo".into()
        } else {
            "distribution-repo".into()
        }
    } else {
        "same-repo".into()
    }
}

fn infer_source_kind(path: &Path) -> String {
    if path.join("Cargo.toml").is_file() {
        "cargo-package".into()
    } else if path.join("src-tauri/tauri.conf.json").is_file()
        || path.join("src-tauri/tauri.conf.json5").is_file()
    {
        "tauri-app".into()
    } else if path.join("package.json").is_file() {
        "js-package".into()
    } else if path.join("composer.json").is_file() {
        "php-package".into()
    } else {
        "custom".into()
    }
}

fn detect_cargo_package(path: &Path) -> Result<Option<String>> {
    read_cargo_package_name(&path.join("Cargo.toml"))
}

fn default_distribution_path(strategy: &str, name: &str) -> String {
    if strategy == "distribution-repo" {
        format!(".verzly/distributions/{name}")
    } else {
        String::new()
    }
}

fn default_version_file(target_path: &str, source_kind: &str) -> String {
    if source_kind == "cargo-package" {
        format!("{}/Cargo.toml", target_path.trim_end_matches('/'))
    } else {
        String::new()
    }
}

fn include_path(path: &str) -> String {
    if path == "." {
        ".".into()
    } else {
        format!("{}/", path.trim_end_matches('/'))
    }
}

fn merge_existing_target(mut existing: ReleaseTarget, incoming: ReleaseTarget) -> ReleaseTarget {
    replace_if_nonempty(&mut existing.path, incoming.path);
    replace_if_nonempty(&mut existing.workspace, incoming.workspace);
    replace_if_nonempty(&mut existing.strategy, incoming.strategy);
    replace_if_nonempty(&mut existing.workflow, incoming.workflow);
    replace_if_nonempty(&mut existing.source_kind, incoming.source_kind);
    replace_if_nonempty(&mut existing.repository, incoming.repository);
    replace_if_nonempty(&mut existing.distribution_path, incoming.distribution_path);
    replace_if_nonempty(&mut existing.cargo_binary, incoming.cargo_binary);
    replace_if_nonempty(&mut existing.cargo_package, incoming.cargo_package);
    replace_if_nonempty(&mut existing.cargo_out_dir, incoming.cargo_out_dir);
    replace_if_nonempty(&mut existing.version_file, incoming.version_file);
    replace_if_nonempty(&mut existing.version_key, incoming.version_key);
    replace_if_nonempty(&mut existing.version_value, incoming.version_value);
    replace_if_nonempty(&mut existing.source_tag_prefix, incoming.source_tag_prefix);
    existing.generate_notes = incoming.generate_notes.or(existing.generate_notes);
    if !incoming.include_scopes.is_empty() {
        existing.include_scopes = incoming.include_scopes;
    }
    if !incoming.include_paths.is_empty() {
        existing.include_paths = incoming.include_paths;
    }
    existing
}

fn replace_if_nonempty(target: &mut String, value: String) {
    if !value.is_empty() {
        *target = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn derives_target_name_from_path() {
        assert_eq!(derive_target_name("crates/repository"), "repository");
        assert_eq!(derive_target_name("apps/mobile"), "mobile");
    }

    #[test]
    fn normalizes_relative_path() {
        assert_eq!(
            normalize_relative_path(&PathBuf::from("./packages/api")),
            "packages/api"
        );
    }
}
