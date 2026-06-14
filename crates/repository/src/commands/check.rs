use crate::cli::CheckArgs;
use crate::project::{detect_cargo_packages, ProjectProfile, ReleaseTarget, DEFAULT_CONFIG_FILE};
use crate::release::{STRATEGIES, WORKFLOWS};
use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED_DISTRIBUTION_FILES: &[&str] =
    &["README.md", "CONTRIBUTING.md", "action.yml", "LICENSE"];

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
    collect_distribution_template_issues(profile, &mut issues)?;
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

    for path in ["crates/repo-quality", ".codex/distributions/repo-quality"] {
        let full_path = root.join(path);
        if full_path.exists() {
            issues.push(format!(
                "{} is deprecated; rename it to use `repository`",
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
                "{} is not allowed; keep distribution templates in .codex/distributions and release behavior in Rust tools",
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

fn collect_distribution_template_issues(
    profile: &ProjectProfile,
    issues: &mut Vec<String>,
) -> Result<()> {
    let distributions_dir = profile.root.join(".codex/distributions");
    if !distributions_dir.exists() {
        if has_distribution_targets(profile) {
            issues.push(".codex/distributions is missing".into());
        }
        return Ok(());
    }

    if !distributions_dir.is_dir() {
        issues.push(".codex/distributions exists but is not a directory".into());
        return Ok(());
    }

    let referenced_paths = profile
        .stored_config
        .release
        .targets
        .iter()
        .filter_map(normalized_distribution_path)
        .collect::<BTreeSet<_>>();

    for entry in fs::read_dir(&distributions_dir)
        .with_context(|| format!("failed to read {}", distributions_dir.display()))?
    {
        let entry = entry
            .with_context(|| format!("failed to read entry in {}", distributions_dir.display()))?;
        let path = entry.path();
        if !path.is_dir() {
            issues.push(format!(
                "{} is not allowed; distribution templates must be directories",
                display_path(profile, &path)
            ));
            continue;
        }

        let display = display_path(profile, &path);
        if !referenced_paths.is_empty() && !referenced_paths.contains(&normalize_path(&display)) {
            issues.push(format!(
                "distribution template `{display}` is not referenced by any release target"
            ));
        }
        collect_single_distribution_template_issues(profile, &path, issues)?;
    }

    for target in &profile.stored_config.release.targets {
        let Some(distribution_path) = normalized_distribution_path(target) else {
            continue;
        };
        let full_path = profile.root.join(&distribution_path);
        if !full_path.is_dir() {
            issues.push(format!(
                "release target `{}` distribution path does not exist: {}",
                target.name, distribution_path
            ));
            continue;
        }
        collect_action_readme_contract_issues(profile, target, &full_path, issues)?;
    }

    Ok(())
}

fn collect_single_distribution_template_issues(
    profile: &ProjectProfile,
    path: &Path,
    issues: &mut Vec<String>,
) -> Result<()> {
    let mut seen = BTreeSet::new();

    for required in REQUIRED_DISTRIBUTION_FILES {
        let full_path = path.join(required);
        if !full_path.is_file() {
            issues.push(format!(
                "{} is missing required distribution file `{required}`",
                display_path(profile, path)
            ));
        }
    }

    for entry in fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))? {
        let entry = entry.with_context(|| format!("failed to read entry in {}", path.display()))?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        seen.insert(file_name.clone());

        if !REQUIRED_DISTRIBUTION_FILES.contains(&file_name.as_str()) {
            issues.push(format!(
                "{} contains unsupported distribution file `{file_name}`",
                display_path(profile, path)
            ));
        }
    }

    if seen.contains("AGENTS.md") || seen.contains("CLAUDE.md") {
        issues.push(format!(
            "{} must not contain AI instruction files; use the root AGENTS.md and optional root CLAUDE.md only",
            display_path(profile, path)
        ));
    }

    Ok(())
}

fn collect_action_readme_contract_issues(
    profile: &ProjectProfile,
    target: &ReleaseTarget,
    distribution_path: &Path,
    issues: &mut Vec<String>,
) -> Result<()> {
    let action_path = distribution_path.join("action.yml");
    let readme_path = distribution_path.join("README.md");
    if !action_path.is_file() || !readme_path.is_file() {
        return Ok(());
    }

    let action = fs::read_to_string(&action_path)
        .with_context(|| format!("failed to read {}", action_path.display()))?;
    let readme = fs::read_to_string(&readme_path)
        .with_context(|| format!("failed to read {}", readme_path.display()))?;

    for input in parse_yaml_section_keys(&action, "inputs") {
        if !documents_table_item(&readme, &input) {
            issues.push(format!(
                "{} README.md does not document action input `{input}`",
                display_path(profile, distribution_path)
            ));
        }
    }

    for output in parse_yaml_section_keys(&action, "outputs") {
        if !documents_table_item(&readme, &output) {
            issues.push(format!(
                "{} README.md does not document action output `{output}`",
                display_path(profile, distribution_path)
            ));
        }
    }

    let normalized_readme = readme.to_lowercase();
    let explains_source_boundary = normalized_readme.contains("source lives in `verzly/toolchain`")
        || (normalized_readme.contains("source code")
            && normalized_readme.contains("verzly/toolchain"));
    if !explains_source_boundary {
        issues.push(format!(
            "{} README.md should explain that source lives in verzly/toolchain",
            display_path(profile, distribution_path)
        ));
    }

    if readme.contains("\n## Contributing") || readme.contains("\n## Install") {
        issues.push(format!(
            "{} README.md must stay usage-focused; keep contributing and install policy out of the README",
            display_path(profile, distribution_path)
        ));
    }

    if target.repository.trim().is_empty() {
        issues.push(format!(
            "release target `{}` has a distribution template but no public repository",
            target.name
        ));
    }

    Ok(())
}

fn collect_release_workflow_issues(profile: &ProjectProfile, issues: &mut Vec<String>) {
    if !profile.stored_config.release.enabled {
        return;
    }

    let public_targets = profile
        .stored_config
        .release
        .targets
        .iter()
        .filter(|target| normalized_distribution_path(target).is_some())
        .collect::<Vec<_>>();

    let require_public_release_workflows = profile.stored_config.release.manage_workflows
        || is_toolchain_repository(profile)
        || public_targets.iter().any(|target| {
            profile
                .root
                .join(format!(".github/workflows/release-{}.yml", target.name))
                .is_file()
        });

    if require_public_release_workflows {
        for path in [
            ".github/workflows/_release-tool.yml",
            ".github/workflows/_release-build-assets.yml",
            ".github/workflows/sync-distributions.yml",
            ".github/workflows/delete-release.yml",
            ".github/workflows/update-floating-tags.yml",
        ] {
            if !profile.root.join(path).is_file() {
                issues.push(format!("{path} is missing"));
            }
        }
    }

    if require_public_release_workflows
        && profile.stored_config.release.release_all
        && public_targets.len() > 1
    {
        let path = profile.root.join(".github/workflows/release-all.yml");
        if !path.is_file() {
            issues.push(".github/workflows/release-all.yml is missing".into());
        }
    }

    for target in public_targets {
        let workflow = format!(".github/workflows/release-{}.yml", target.name);
        let path = profile.root.join(&workflow);
        let require_target_workflow =
            target.workflow == "managed" || is_toolchain_repository(profile) || path.is_file();

        if !require_target_workflow {
            continue;
        }

        if !path.is_file() {
            issues.push(format!(
                "release target `{}` is missing workflow {workflow}",
                target.name
            ));
            continue;
        }

        let Ok(content) = fs::read_to_string(&path) else {
            issues.push(format!("failed to read {workflow}"));
            continue;
        };
        if !content.contains(&format!("tool: {}", target.name)) {
            issues.push(format!(
                "{workflow} does not dispatch release target `{}`",
                target.name
            ));
        }
    }

    if profile
        .stored_config
        .release
        .targets
        .iter()
        .any(|target| target.name == "toolchain")
    {
        for path in [
            ".github/workflows/release-toolchain.yml",
            ".github/workflows/_release-toolchain.yml",
        ] {
            if !profile.root.join(path).is_file() {
                issues.push(format!("{path} is missing"));
            }
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

fn has_distribution_targets(profile: &ProjectProfile) -> bool {
    profile
        .stored_config
        .release
        .targets
        .iter()
        .any(|target| normalized_distribution_path(target).is_some())
}

fn normalized_distribution_path(target: &ReleaseTarget) -> Option<String> {
    let path = target.distribution_path.trim();
    if path.is_empty() {
        None
    } else {
        Some(normalize_path(path))
    }
}

fn parse_yaml_section_keys(text: &str, section: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut in_section = false;

    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }

        if !raw_line.starts_with(' ') && line.ends_with(':') {
            let name = line.trim_end_matches(':');
            in_section = name == section;
            continue;
        }

        if !in_section {
            continue;
        }

        if raw_line.starts_with("  ") && !raw_line.starts_with("    ") {
            if let Some((key, _)) = line.trim().split_once(':') {
                if !key.is_empty() {
                    keys.push(key.trim_matches('"').trim_matches('\'').to_string());
                }
            }
        }
    }

    keys
}

fn documents_table_item(readme: &str, name: &str) -> bool {
    readme.contains(&format!("| `{name}` |"))
}

fn display_path(profile: &ProjectProfile, path: &Path) -> String {
    path.strip_prefix(&profile.root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn normalize_path(path: &str) -> String {
    PathBuf::from(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .filter(|component| component != ".")
        .collect::<Vec<_>>()
        .join("/")
}
