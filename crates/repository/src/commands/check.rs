use crate::cli::CheckArgs;
use crate::project::{detect_cargo_packages, ProjectProfile, DEFAULT_CONFIG_FILE};
use crate::release::{STRATEGIES, WORKFLOWS};
use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub fn run(args: CheckArgs) -> Result<()> {
    let profile = ProjectProfile::detect(
        args.root,
        args.config,
        None,
        &[],
        crate::cli::JsRunnerArg::Auto,
    )?;
    let issues = collect_config_issues(&profile)?;

    if issues.is_empty() {
        println!("datarose configuration is valid.");
        return Ok(());
    }

    eprintln!("datarose configuration has unsupported settings:");
    for issue in &issues {
        eprintln!("- {issue}");
    }
    bail!("datarose configuration check failed")
}

pub fn collect_config_issues(profile: &ProjectProfile) -> Result<Vec<String>> {
    let mut issues = Vec::new();

    if !profile.config_path.is_file() {
        issues.push(format!(
            "{} is missing; run `repository init` first",
            profile.config_path.display()
        ));
        return Ok(issues);
    }

    let text = fs::read_to_string(&profile.config_path)
        .with_context(|| format!("failed to read {}", profile.config_path.display()))?;

    collect_removed_files(&profile.root, &mut issues);
    collect_removed_fields(&text, &mut issues);
    collect_invalid_values(profile, &mut issues)?;
    collect_repository_boundary_issues(profile, &mut issues)?;
    collect_action_surface_issues(profile, &mut issues);
    collect_release_workflow_issues(profile, &mut issues);

    Ok(issues)
}

fn collect_removed_files(root: &Path, issues: &mut Vec<String>) {
    let direct_removed = [
        ".repo-quality.toml",
        "github-release.toml",
        "rust-cache.toml",
        "tauri-release.toml",
        ".github/workflows/_release-datarose-tool.yml",
        ".github/workflows/release-repo-quality.yml",
    ];
    for path in direct_removed {
        let full_path = root.join(path);
        if full_path.exists() {
            issues.push(format!(
                "{} is deprecated; move its settings into {DEFAULT_CONFIG_FILE}",
                full_path.display()
            ));
        }
    }

    for path in ["crates/repo-quality", ".verzly/distributions"] {
        let full_path = root.join(path);
        if full_path.exists() {
            issues.push(format!(
                "{} is deprecated; use the unified root action and `actions/<tool>` subpath actions in this repository",
                full_path.display()
            ));
        }
    }

    for dir in [
        root.join("crates"),
        root.join("apps"),
        root.join("packages"),
    ] {
        if !dir.is_dir() {
            continue;
        }
        collect_removed_tool_configs(&dir, root, issues);
    }
}

fn collect_removed_tool_configs(path: &Path, root: &Path, issues: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_removed_tool_configs(&path, root, issues);
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if matches!(file_name, "github-release.toml" | "cargo-release.toml") {
            let display = path
                .strip_prefix(root)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|_| path.display().to_string());
            issues.push(format!(
                "{display} is removed; move this release configuration into {DEFAULT_CONFIG_FILE}"
            ));
        }
    }
}

fn collect_removed_fields(text: &str, issues: &mut Vec<String>) {
    for (line_number, raw_line) in text.lines().enumerate() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "github_release_config" => issues.push(format!(
                "line {}: `github_release_config` is removed; use the current `[[release.targets]]` fields in {DEFAULT_CONFIG_FILE}",
                line_number + 1
            )),
            "cargo_release_config" => issues.push(format!(
                "line {}: `cargo_release_config` is removed; put cargo artifact settings directly on the release target",
                line_number + 1
            )),
            "package" if value == "\"auto\"" => issues.push(format!(
                "line {}: `rust_cache.cache.package = \"auto\"` is removed; use the repository directory name explicitly",
                line_number + 1
            )),
            _ if value.contains("repo-quality") => issues.push(format!(
                "line {}: `repo-quality` has been renamed to `repository`",
                line_number + 1
            )),
            _ => {}
        }
    }
}

fn collect_invalid_values(profile: &ProjectProfile, issues: &mut Vec<String>) -> Result<()> {
    let mut names = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for target in &profile.stored_config.release.targets {
        if target.name.trim().is_empty() {
            issues.push("release target has an empty name".into());
        } else if !names.insert(target.name.as_str()) {
            issues.push(format!("duplicate release target name `{}`", target.name));
        }

        if target.path.trim().is_empty() {
            issues.push(format!("release target `{}` has no path", target.name));
        } else if !paths.insert(target.path.as_str()) {
            issues.push(format!("duplicate release target path `{}`", target.path));
        } else {
            let full_path = profile.root.join(&target.path);
            if !full_path.exists() {
                issues.push(format!(
                    "release target `{}` path does not exist: {}",
                    target.name, target.path
                ));
            }
        }

        if !STRATEGIES.contains(&target.strategy.as_str()) {
            issues.push(format!(
                "release target `{}` has invalid strategy `{}`",
                target.name, target.strategy
            ));
        }
        if !WORKFLOWS.contains(&target.workflow.as_str()) {
            issues.push(format!(
                "release target `{}` has invalid workflow `{}`",
                target.name, target.workflow
            ));
        }
        if target.strategy == "distribution-repo" && target.repository.trim().is_empty() {
            issues.push(format!(
                "release target `{}` uses distribution-repo but has no repository",
                target.name
            ));
        }
        if target.workflow == "managed" && !profile.stored_config.release.manage_workflows {
            issues.push(format!(
                "release target `{}` is workflow=managed but release.manage_workflows is false",
                target.name
            ));
        }
        if target.workflow == "managed"
            && !matches!(target.strategy.as_str(), "same-repo" | "distribution-repo")
        {
            issues.push(format!(
                "release target `{}` uses workflow=managed with unsupported strategy `{}`",
                target.name, target.strategy
            ));
        }
    }

    if profile.stored_config.release.manage_cargo_packages {
        let configured = profile
            .stored_config
            .release
            .targets
            .iter()
            .map(|target| target.cargo_package.as_str())
            .collect::<BTreeSet<_>>();
        for package in detect_cargo_packages(&profile.root)? {
            if !configured.contains(package.as_str()) {
                issues.push(format!(
                    "Cargo package `{package}` has no `[[release.targets]]` entry"
                ));
            }
        }
    }

    let expected_package = profile.default_package_name();
    if profile.stored_config.rust_cache.package.is_none() {
        issues.push(format!(
            "rust_cache.cache.package is missing; use `{expected_package}` for this repository"
        ));
    }

    Ok(())
}

fn collect_repository_boundary_issues(
    profile: &ProjectProfile,
    issues: &mut Vec<String>,
) -> Result<()> {
    for path in ["distribution", "scripts"] {
        let full_path = profile.root.join(path);
        if full_path.exists() {
            issues.push(format!(
                "{} is not allowed; keep action surfaces in action.yml or actions/<tool>/action.yml and release behavior in Rust tools",
                display_path(profile, &full_path)
            ));
        }
    }

    let crates_dir = profile.root.join("crates");
    if crates_dir.is_dir() {
        for entry in fs::read_dir(&crates_dir)
            .with_context(|| format!("failed to read {}", crates_dir.display()))?
        {
            let entry = entry
                .with_context(|| format!("failed to read entry in {}", crates_dir.display()))?;
            let readme = entry.path().join("README.md");
            if readme.exists() {
                issues.push(format!(
                    "{} is not allowed; crate-level README files are intentionally not used",
                    display_path(profile, &readme)
                ));
            }
        }
    }

    Ok(())
}

fn collect_action_surface_issues(profile: &ProjectProfile, issues: &mut Vec<String>) {
    if !is_toolchain_repository(profile) {
        return;
    }

    for path in [
        "action.yml",
        "actions/_shared/install-verzly.sh",
        "actions/android-signing/action.yml",
        "actions/cargo-release/action.yml",
        "actions/github-release/action.yml",
        "actions/ios-signing/action.yml",
        "actions/repository/action.yml",
        "actions/rust-cache/action.yml",
        "actions/tauri-release/action.yml",
    ] {
        if !profile.root.join(path).is_file() {
            issues.push(format!("{path} is missing"));
        }
    }
}

fn collect_release_workflow_issues(profile: &ProjectProfile, issues: &mut Vec<String>) {
    if !profile.stored_config.release.enabled {
        return;
    }

    let require_toolchain_workflows = profile.stored_config.release.manage_workflows
        || is_toolchain_repository(profile)
        || profile
            .stored_config
            .release
            .targets
            .iter()
            .any(|target| target.name == "verzly");

    if !require_toolchain_workflows {
        return;
    }

    for path in [
        ".github/workflows/release.yml",
        ".github/workflows/delete-release.yml",
        ".github/workflows/test.yml",
    ] {
        if !profile.root.join(path).is_file() {
            issues.push(format!("{path} is missing"));
        }
    }
}

fn is_toolchain_repository(profile: &ProjectProfile) -> bool {
    profile.stored_config.release.source_repository == "verzly/toolchain"
        || profile
            .root
            .join("crates/github-release/Cargo.toml")
            .is_file()
}

fn display_path(profile: &ProjectProfile, path: &Path) -> String {
    path.strip_prefix(&profile.root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
