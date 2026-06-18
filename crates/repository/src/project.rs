//! Project detection and datarose.toml parsing for quality profiles.

use crate::cli::{JsRunnerArg, LanguageArg};
use crate::schema::DATAROSE_SCHEMA_DIRECTIVE;
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_CONFIG_FILE: &str = "datarose.toml";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Language {
    Rust,
    Js,
    Php,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Js => "js",
            Self::Php => "php",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "rust" => Some(Self::Rust),
            "js" | "javascript" | "typescript" | "vue" => Some(Self::Js),
            "php" => Some(Self::Php),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JsRunner {
    Aube,
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl JsRunner {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aube => "aube",
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "aube" => Some(Self::Aube),
            "npm" => Some(Self::Npm),
            "pnpm" => Some(Self::Pnpm),
            "yarn" => Some(Self::Yarn),
            "bun" => Some(Self::Bun),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MiseToolRecommendation {
    pub tool: String,
    pub version: String,
    pub reason: String,
}

impl MiseToolRecommendation {
    pub fn command(&self) -> String {
        format!("mise use {}@{}", self.tool, self.version)
    }
}

#[derive(Clone, Debug, Default)]
pub struct QualityConfig {
    pub workspace: Option<PathBuf>,
    pub languages: Vec<Language>,
    pub js_runner: Option<JsRunner>,
}

#[derive(Clone, Debug)]
pub struct ReleaseConfig {
    pub enabled: bool,
    pub target_branch: String,
    pub source_repository: String,
    pub secret_name: String,
    pub release_all: bool,
    pub manage_cargo_packages: bool,
    pub manage_workflows: bool,
    pub targets: Vec<ReleaseTarget>,
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            target_branch: "master".into(),
            source_repository: String::new(),
            secret_name: String::new(),
            release_all: true,
            manage_cargo_packages: false,
            manage_workflows: false,
            targets: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ReleaseTarget {
    pub name: String,
    pub path: String,
    pub workspace: String,
    pub strategy: String,
    pub workflow: String,
    pub source_kind: String,
    pub repository: String,
    pub source_repository: Option<String>,
    pub distribution_path: String,
    pub cargo_binary: String,
    pub cargo_package: String,
    pub cargo_out_dir: String,
    pub cargo_targets: Vec<String>,
    pub prepare_commands: Vec<String>,
    pub version_files: Option<bool>,
    pub version_file: String,
    pub version_key: String,
    pub version_value: String,
    pub source_tag_prefix: String,
    pub release_name_prefix: String,
    pub source_release: Option<bool>,
    pub generate_notes: Option<bool>,
    pub floating_tags: Option<bool>,
    pub latest_tag: Option<bool>,
    pub next_tag: Option<bool>,
    pub include_scopes: Vec<String>,
    pub include_paths: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct RustCacheConfig {
    pub dir: String,
    pub package: Option<String>,
    pub redirect_cargo_home: bool,
    pub redirect_gradle: bool,
    pub cargo_target_dir: String,
    pub env: BTreeMap<String, String>,
}

impl Default for RustCacheConfig {
    fn default() -> Self {
        let mut env = BTreeMap::new();
        env.insert("GRADLE_USER_HOME".into(), "android/gradle".into());
        env.insert("NPM_CONFIG_CACHE".into(), "js/npm".into());
        env.insert("PNPM_STORE_PATH".into(), "js/pnpm-store".into());
        env.insert("YARN_CACHE_FOLDER".into(), "js/yarn".into());
        Self {
            dir: ".cache".into(),
            package: None,
            redirect_cargo_home: false,
            redirect_gradle: true,
            cargo_target_dir: "rust/packages/{package}/target".into(),
            env,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TauriReleaseConfig {
    pub project_root: String,
    pub frontend_install: String,
    pub out_dir: String,
    pub cache_dir: String,
}

impl Default for TauriReleaseConfig {
    fn default() -> Self {
        Self {
            project_root: ".".into(),
            frontend_install: "aube install".into(),
            out_dir: "dist".into(),
            cache_dir: ".cache/tauri-release".into(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DataroseConfig {
    pub quality: QualityConfig,
    pub release: ReleaseConfig,
    pub rust_cache: RustCacheConfig,
    pub tauri_release: TauriReleaseConfig,
}

#[derive(Clone, Debug)]
pub struct ProjectProfile {
    pub root: PathBuf,
    pub workspace: PathBuf,
    pub workspace_root: PathBuf,
    pub config_path: PathBuf,
    pub languages: Vec<Language>,
    pub js_runner: Option<JsRunner>,
    pub has_rector: bool,
    pub has_pest: bool,
    pub has_mise_toml: bool,
    pub mise_tools: BTreeSet<String>,
    pub stored_config: DataroseConfig,
}

impl ProjectProfile {
    pub fn detect(
        root: PathBuf,
        config_path: Option<PathBuf>,
        workspace: Option<PathBuf>,
        languages: &[LanguageArg],
        runner: JsRunnerArg,
    ) -> Result<Self> {
        let root = fs::canonicalize(&root)
            .with_context(|| format!("failed to resolve repository root: {}", root.display()))?;
        let config_path = resolve_config_path(&root, config_path);
        let stored_config = read_datarose_config(&config_path)?;
        let workspace = workspace
            .or_else(|| stored_config.quality.workspace.clone())
            .unwrap_or_else(|| PathBuf::from("."));
        let workspace_root = fs::canonicalize(root.join(&workspace)).with_context(|| {
            format!(
                "failed to resolve repository standards workspace: {}",
                root.join(&workspace).display()
            )
        })?;
        let composer = read_composer_dependencies(&workspace_root)?;
        let (has_mise_toml, mise_tools) = read_mise_tools(&root)?;
        let mut detected = BTreeSet::new();

        if workspace_root.join("Cargo.toml").is_file() || has_source_file(&workspace_root, &["rs"])?
        {
            detected.insert(Language::Rust);
        }
        if workspace_root.join("package.json").is_file()
            || workspace_root.join("aube-workspace.yaml").is_file()
            || has_source_file(&workspace_root, &["js", "mjs", "cjs", "ts", "tsx", "vue"])?
        {
            detected.insert(Language::Js);
        }
        if workspace_root.join("composer.json").is_file()
            || has_source_file(&workspace_root, &["php"])?
        {
            detected.insert(Language::Php);
        }

        let language_overrides = if languages.is_empty() {
            stored_config.quality.languages.clone()
        } else {
            languages
                .iter()
                .map(|language| match language {
                    LanguageArg::Rust => Language::Rust,
                    LanguageArg::Js => Language::Js,
                    LanguageArg::Php => Language::Php,
                })
                .collect()
        };
        for language in language_overrides {
            detected.insert(language);
        }

        let languages = detected.into_iter().collect::<Vec<_>>();
        let js_runner = if languages.contains(&Language::Js) {
            Some(resolve_js_runner(
                &workspace_root,
                runner,
                stored_config.quality.js_runner.as_ref(),
                &mise_tools,
            ))
        } else {
            None
        };

        Ok(Self {
            root,
            workspace,
            workspace_root,
            config_path,
            languages,
            js_runner,
            has_rector: composer.contains_key("rector/rector")
                || composer.contains_key("rectorphp/rector"),
            has_pest: composer.contains_key("pestphp/pest"),
            has_mise_toml,
            mise_tools,
            stored_config,
        })
    }

    pub fn has_language(&self, language: &Language) -> bool {
        self.languages.contains(language)
    }

    pub fn workspace_is_root(&self) -> bool {
        self.workspace == Path::new(".") || self.workspace.as_os_str().is_empty()
    }

    pub fn workspace_display(&self) -> String {
        normalize_path(&self.workspace)
    }

    pub fn config_display(&self) -> String {
        self.config_path
            .strip_prefix(&self.root)
            .map(normalize_path)
            .unwrap_or_else(|_| normalize_path(&self.config_path))
    }

    pub fn default_package_name(&self) -> String {
        self.root
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.is_empty())
            .unwrap_or("repository")
            .to_string()
    }

    pub fn release_enabled(&self) -> bool {
        self.stored_config.release.enabled && !self.stored_config.release.targets.is_empty()
    }

    pub fn missing_mise_tools(&self) -> Vec<MiseToolRecommendation> {
        let mut recommendations = Vec::new();
        push_missing_tool(
            &mut recommendations,
            &self.mise_tools,
            "hk",
            "latest",
            "required to run repository git hooks",
        );
        push_missing_tool(
            &mut recommendations,
            &self.mise_tools,
            "pkl",
            "latest",
            "required by hk.pkl on machines where hk needs the Pkl CLI",
        );

        if self.has_language(&Language::Rust) {
            push_missing_tool(
                &mut recommendations,
                &self.mise_tools,
                "rust",
                "stable",
                "Rust files were detected; rustfmt, clippy, and cargo test need Rust stable",
            );
        }

        if let Some(runner) = &self.js_runner {
            match runner {
                JsRunner::Aube => push_missing_tool(
                    &mut recommendations,
                    &self.mise_tools,
                    "aube",
                    "latest",
                    "JavaScript/TypeScript files were detected and aube is the preferred runner",
                ),
                JsRunner::Pnpm => push_missing_tool(
                    &mut recommendations,
                    &self.mise_tools,
                    "pnpm",
                    "latest",
                    "pnpm project files were detected; use the existing package runner",
                ),
                JsRunner::Yarn => push_missing_tool(
                    &mut recommendations,
                    &self.mise_tools,
                    "yarn",
                    "latest",
                    "Yarn project files were detected; use the existing package runner",
                ),
                JsRunner::Bun => push_missing_tool(
                    &mut recommendations,
                    &self.mise_tools,
                    "bun",
                    "latest",
                    "Bun project files were detected; use the existing package runner",
                ),
                JsRunner::Npm => push_missing_tool(
                    &mut recommendations,
                    &self.mise_tools,
                    "node",
                    "latest",
                    "npm project files were detected; npm is provided by Node.js",
                ),
            }
            push_missing_tool(
                &mut recommendations,
                &self.mise_tools,
                "npm:oxlint",
                "latest",
                "preferred JavaScript/TypeScript linter",
            );
            push_missing_tool(
                &mut recommendations,
                &self.mise_tools,
                "npm:oxfmt",
                "latest",
                "preferred JavaScript/TypeScript/Vue formatter",
            );
            push_missing_tool(
                &mut recommendations,
                &self.mise_tools,
                "npm:vitest",
                "latest",
                "preferred JavaScript/TypeScript unit test runner",
            );
        }

        if self.has_language(&Language::Php) {
            push_missing_tool(
                &mut recommendations,
                &self.mise_tools,
                "php",
                "latest",
                "PHP files were detected; Rector and Pest need a PHP runtime",
            );
            push_missing_tool(
                &mut recommendations,
                &self.mise_tools,
                "composer",
                "latest",
                "PHP files were detected; Rector and Pest are installed through Composer",
            );
        }

        recommendations
    }
}

fn push_missing_tool(
    recommendations: &mut Vec<MiseToolRecommendation>,
    tools: &BTreeSet<String>,
    tool: &str,
    version: &str,
    reason: &str,
) {
    if !tools.contains(tool) {
        recommendations.push(MiseToolRecommendation {
            tool: tool.into(),
            version: version.into(),
            reason: reason.into(),
        });
    }
}

pub fn datarose_config_path(root: &Path) -> PathBuf {
    root.join(DEFAULT_CONFIG_FILE)
}

pub fn resolve_config_path(root: &Path, config_path: Option<PathBuf>) -> PathBuf {
    match config_path {
        Some(path) if path.is_absolute() => path,
        Some(path) => root.join(path),
        None => datarose_config_path(root),
    }
}

pub fn render_datarose_config(profile: &ProjectProfile) -> String {
    let languages = profile
        .languages
        .iter()
        .map(|language| format!("\"{}\"", language.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = String::new();
    out.push_str(DATAROSE_SCHEMA_DIRECTIVE);
    out.push_str("\n# Managed by repository. Project-specific overrides are allowed.\n");
    out.push_str("version = 1\n\n");
    out.push_str("[quality]\n");
    out.push_str(&format!(
        "workspace = \"{}\"\n",
        escape_toml(&profile.workspace_display())
    ));
    out.push_str(&format!("languages = [{languages}]\n"));
    if let Some(runner) = &profile.js_runner {
        out.push_str(&format!("js_runner = \"{}\"\n", runner.as_str()));
    }

    out.push_str("\n[release]\n");
    out.push_str(&format!(
        "enabled = {}\n",
        bool_literal(profile.stored_config.release.enabled)
    ));
    out.push_str(&format!(
        "target_branch = \"{}\"\n",
        escape_toml(&profile.stored_config.release.target_branch)
    ));
    out.push_str(&format!(
        "source_repository = \"{}\"\n",
        escape_toml(&profile.stored_config.release.source_repository)
    ));
    out.push_str(&format!(
        "secret_name = \"{}\"\n",
        escape_toml(&profile.stored_config.release.secret_name)
    ));
    out.push_str(&format!(
        "release_all = {}\n",
        bool_literal(profile.stored_config.release.release_all)
    ));
    out.push_str(&format!(
        "manage_cargo_packages = {}\n",
        bool_literal(profile.stored_config.release.manage_cargo_packages)
    ));
    out.push_str(&format!(
        "manage_workflows = {}\n",
        bool_literal(profile.stored_config.release.manage_workflows)
    ));

    for target in &profile.stored_config.release.targets {
        out.push_str("\n[[release.targets]]\n");
        out.push_str(&format!("name = \"{}\"\n", escape_toml(&target.name)));
        out.push_str(&format!("path = \"{}\"\n", escape_toml(&target.path)));
        if !target.workspace.is_empty() {
            out.push_str(&format!(
                "workspace = \"{}\"\n",
                escape_toml(&target.workspace)
            ));
        }
        if !target.strategy.is_empty() {
            out.push_str(&format!(
                "strategy = \"{}\"\n",
                escape_toml(&target.strategy)
            ));
        }
        if !target.workflow.is_empty() {
            out.push_str(&format!(
                "workflow = \"{}\"\n",
                escape_toml(&target.workflow)
            ));
        }
        if !target.source_kind.is_empty() {
            out.push_str(&format!(
                "source_kind = \"{}\"\n",
                escape_toml(&target.source_kind)
            ));
        }
        out.push_str(&format!(
            "repository = \"{}\"\n",
            escape_toml(&target.repository)
        ));
        if let Some(source_repository) = &target.source_repository {
            out.push_str(&format!(
                "source_repository = \"{}\"\n",
                escape_toml(source_repository)
            ));
        }
        out.push_str(&format!(
            "distribution_path = \"{}\"\n",
            escape_toml(&target.distribution_path)
        ));
        out.push_str(&format!(
            "cargo_binary = \"{}\"\n",
            escape_toml(&target.cargo_binary)
        ));
        out.push_str(&format!(
            "cargo_package = \"{}\"\n",
            escape_toml(&target.cargo_package)
        ));
        out.push_str(&format!(
            "cargo_out_dir = \"{}\"\n",
            escape_toml(&target.cargo_out_dir)
        ));
        out.push_str(&format!(
            "cargo_targets = [{}]\n",
            render_string_array(&target.cargo_targets)
        ));
        out.push_str(&format!(
            "prepare_commands = [{}]\n",
            render_string_array(&target.prepare_commands)
        ));
        if target.version_files == Some(false) {
            out.push_str("version_files = false\n");
        } else {
            out.push_str(&format!(
                "version_file = \"{}\"\n",
                escape_toml(&target.version_file)
            ));
            out.push_str(&format!(
                "version_key = \"{}\"\n",
                escape_toml(&target.version_key)
            ));
            out.push_str(&format!(
                "version_value = \"{}\"\n",
                escape_toml(&target.version_value)
            ));
        }
        if !target.source_tag_prefix.is_empty() {
            out.push_str(&format!(
                "source_tag_prefix = \"{}\"\n",
                escape_toml(&target.source_tag_prefix)
            ));
        }
        if !target.release_name_prefix.is_empty() {
            out.push_str(&format!(
                "release_name_prefix = \"{}\"\n",
                escape_toml(&target.release_name_prefix)
            ));
        }
        if let Some(source_release) = target.source_release {
            out.push_str(&format!(
                "source_release = {}\n",
                bool_literal(source_release)
            ));
        }
        if let Some(generate_notes) = target.generate_notes {
            out.push_str(&format!(
                "generate_notes = {}\n",
                bool_literal(generate_notes)
            ));
        }
        if let Some(floating_tags) = target.floating_tags {
            out.push_str(&format!(
                "floating_tags = {}\n",
                bool_literal(floating_tags)
            ));
        }
        if let Some(latest_tag) = target.latest_tag {
            out.push_str(&format!("latest_tag = {}\n", bool_literal(latest_tag)));
        }
        if let Some(next_tag) = target.next_tag {
            out.push_str(&format!("next_tag = {}\n", bool_literal(next_tag)));
        }
        out.push_str(&format!(
            "include_scopes = [{}]\n",
            render_string_array(&target.include_scopes)
        ));
        out.push_str(&format!(
            "include_paths = [{}]\n",
            render_string_array(&target.include_paths)
        ));
    }

    let rust_cache_package = profile
        .stored_config
        .rust_cache
        .package
        .as_deref()
        .filter(|package| !package.trim().is_empty() && *package != "auto")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| profile.default_package_name());

    out.push_str("\n[rust_cache.cache]\n");
    out.push_str(&format!(
        "dir = \"{}\"\n",
        escape_toml(&profile.stored_config.rust_cache.dir)
    ));
    out.push_str(&format!(
        "package = \"{}\"\n",
        escape_toml(&rust_cache_package)
    ));
    out.push_str(&format!(
        "redirect_cargo_home = {}\n",
        bool_literal(profile.stored_config.rust_cache.redirect_cargo_home)
    ));
    out.push_str(&format!(
        "redirect_gradle = {}\n\n",
        bool_literal(profile.stored_config.rust_cache.redirect_gradle)
    ));
    out.push_str("[rust_cache.cargo]\n");
    out.push_str(&format!(
        "target_dir = \"{}\"\n\n",
        escape_toml(&profile.stored_config.rust_cache.cargo_target_dir)
    ));
    out.push_str("[rust_cache.env]\n");
    for (key, value) in &profile.stored_config.rust_cache.env {
        out.push_str(&format!("{key} = \"{}\"\n", escape_toml(value)));
    }
    out.push_str("\n[tauri_release.project]\n");
    out.push_str(&format!(
        "root = \"{}\"\n",
        escape_toml(&profile.stored_config.tauri_release.project_root)
    ));
    out.push_str(&format!(
        "frontend_install = \"{}\"\n\n",
        escape_toml(&profile.stored_config.tauri_release.frontend_install)
    ));
    out.push_str("[tauri_release.build]\n");
    out.push_str(&format!(
        "out_dir = \"{}\"\n",
        escape_toml(&profile.stored_config.tauri_release.out_dir)
    ));
    out.push_str(&format!(
        "cache_dir = \"{}\"\n",
        escape_toml(&profile.stored_config.tauri_release.cache_dir)
    ));

    out
}

pub fn read_datarose_config(path: &Path) -> Result<DataroseConfig> {
    if !path.is_file() {
        return Ok(DataroseConfig::default());
    }

    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut config = DataroseConfig::default();
    let mut section = String::new();
    let mut current_target: Option<ReleaseTarget> = None;

    for raw_line in text.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if line == "[[release.targets]]" {
            if let Some(target) = current_target.take() {
                config.release.targets.push(target);
            }
            section = "release.targets".into();
            current_target = Some(ReleaseTarget::default());
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            if let Some(target) = current_target.take() {
                config.release.targets.push(target);
            }
            section = line.trim_matches(['[', ']']).to_string();
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match section.as_str() {
            "quality" | "" => apply_quality_value(&mut config.quality, key, value),
            "release" => apply_release_value(&mut config.release, key, value),
            "rust_cache.cache" => apply_rust_cache_value(&mut config.rust_cache, key, value),
            "rust_cache.cargo" => apply_rust_cache_cargo_value(&mut config.rust_cache, key, value),
            "rust_cache.env" => apply_rust_cache_env_value(&mut config.rust_cache, key, value),
            "tauri_release.project" => {
                apply_tauri_release_project_value(&mut config.tauri_release, key, value)
            }
            "tauri_release.build" => {
                apply_tauri_release_build_value(&mut config.tauri_release, key, value)
            }
            "release.targets" => {
                if let Some(target) = current_target.as_mut() {
                    apply_release_target_value(target, key, value);
                }
            }
            _ => {}
        }
    }

    if let Some(target) = current_target.take() {
        config.release.targets.push(target);
    }

    normalize_release_targets(&mut config.release.targets);

    Ok(config)
}

pub fn detect_cargo_packages(root: &Path) -> Result<Vec<String>> {
    let mut packages = BTreeSet::new();
    let crates_dir = root.join("crates");
    if crates_dir.is_dir() {
        for entry in fs::read_dir(&crates_dir)
            .with_context(|| format!("failed to read {}", crates_dir.display()))?
        {
            let entry = entry
                .with_context(|| format!("failed to read entry in {}", crates_dir.display()))?;
            let manifest = entry.path().join("Cargo.toml");
            if let Some(name) = read_cargo_package_name(&manifest)? {
                packages.insert(name);
            }
        }
    }

    if let Some(name) = read_cargo_package_name(&root.join("Cargo.toml"))? {
        packages.insert(name);
    }

    Ok(packages.into_iter().collect())
}

pub fn read_cargo_package_name(path: &Path) -> Result<Option<String>> {
    if !path.is_file() {
        return Ok(None);
    }

    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut in_package = false;
    for raw_line in text.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line == "[package]" {
            in_package = true;
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_package = false;
            continue;
        }
        if in_package {
            if let Some(("name", value)) = line
                .split_once('=')
                .map(|(key, value)| (key.trim(), value.trim()))
            {
                return Ok(parse_string(value));
            }
        }
    }

    Ok(None)
}

pub fn normalize_release_targets(targets: &mut [ReleaseTarget]) {
    for target in targets {
        let assetless = target.version_files == Some(false)
            || (!target.source_kind.is_empty() && target.source_kind != "cargo-package")
            || (target.cargo_binary.is_empty()
                && target.cargo_package.is_empty()
                && target.cargo_out_dir.is_empty()
                && target.cargo_targets.is_empty()
                && target.version_file.is_empty());

        if target.path.is_empty() {
            if !target.version_file.is_empty() {
                target.path = parent_path_string(&target.version_file);
            } else if let Some(include_path) = target.include_paths.first() {
                target.path = include_path.trim_end_matches('/').to_string();
            } else if !target.name.is_empty() && !assetless {
                target.path = format!("crates/{}", target.name);
            } else {
                target.path = ".".into();
            }
        }
        if target.workflow.is_empty() {
            target.workflow = "custom".into();
        }
        if target.strategy.is_empty() {
            target.strategy = if assetless {
                "same-repo".into()
            } else {
                "distribution-repo".into()
            };
        }
        if target.source_kind.is_empty() && !assetless {
            target.source_kind = "cargo-package".into();
        }

        if !assetless {
            if target.cargo_binary.is_empty() && !target.name.is_empty() {
                target.cargo_binary = target.name.clone();
            }
            if target.cargo_package.is_empty() && !target.name.is_empty() {
                target.cargo_package = target.name.clone();
            }
            if target.cargo_out_dir.is_empty() && !target.name.is_empty() {
                target.cargo_out_dir = format!("dist/{}", target.name);
            }
            if target.cargo_targets.is_empty() {
                target.cargo_targets = vec![
                    "linux-x64".into(),
                    "macos-x64".into(),
                    "macos-arm64".into(),
                    "windows-x64".into(),
                ];
            }
        }
        if target.prepare_commands.is_empty() && target.version_files != Some(false) {
            target
                .prepare_commands
                .push("cargo generate-lockfile".into());
        }
        if target.version_files != Some(false) {
            if target.version_file.is_empty() {
                target.version_file = format!("{}/Cargo.toml", target.path.trim_end_matches('/'));
            }
            if target.version_key.is_empty() {
                target.version_key = "package.version".into();
            }
            if target.version_value.is_empty() {
                target.version_value = "{version}".into();
            }
        }
        if target.source_tag_prefix.is_empty() && !target.name.is_empty() {
            target.source_tag_prefix = format!("{}-v", target.name);
        }
        if target.include_scopes.is_empty() && !target.name.is_empty() {
            target.include_scopes.push(target.name.clone());
            target.include_scopes.push("all".into());
        }
        if target.include_paths.is_empty() && !target.path.is_empty() {
            if target.path == "." {
                target.include_paths.push(".".into());
            } else {
                target
                    .include_paths
                    .push(format!("{}/", target.path.trim_end_matches('/')));
            }
        }
    }
}

fn parent_path_string(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|path| path.display().to_string().replace('\\', "/"))
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| ".".into())
}

fn apply_quality_value(config: &mut QualityConfig, key: &str, value: &str) {
    match key {
        "workspace" => config.workspace = parse_string(value).map(PathBuf::from),
        "js_runner" => {
            config.js_runner = parse_string(value).and_then(|value| JsRunner::from_str(&value));
        }
        "languages" => {
            config.languages = parse_string_array(value)
                .into_iter()
                .filter_map(|value| Language::from_str(&value))
                .collect();
        }
        _ => {}
    }
}

fn apply_release_value(config: &mut ReleaseConfig, key: &str, value: &str) {
    match key {
        "enabled" => config.enabled = parse_bool(value).unwrap_or(config.enabled),
        "target_branch" => {
            if let Some(value) = parse_string(value) {
                config.target_branch = value;
            }
        }
        "source_repository" => {
            if let Some(value) = parse_string(value) {
                config.source_repository = value;
            }
        }
        "secret_name" => {
            if let Some(value) = parse_string(value) {
                config.secret_name = value;
            }
        }
        "release_all" => config.release_all = parse_bool(value).unwrap_or(config.release_all),
        "manage_cargo_packages" => {
            config.manage_cargo_packages =
                parse_bool(value).unwrap_or(config.manage_cargo_packages);
        }
        "manage_workflows" => {
            config.manage_workflows = parse_bool(value).unwrap_or(config.manage_workflows);
        }
        _ => {}
    }
}

fn apply_rust_cache_value(config: &mut RustCacheConfig, key: &str, value: &str) {
    match key {
        "dir" => config.dir = parse_string(value).unwrap_or_else(|| config.dir.clone()),
        "package" => config.package = parse_string(value),
        "redirect_cargo_home" => {
            config.redirect_cargo_home = parse_bool(value).unwrap_or(config.redirect_cargo_home);
        }
        "redirect_gradle" => {
            config.redirect_gradle = parse_bool(value).unwrap_or(config.redirect_gradle);
        }
        _ => {}
    }
}

fn apply_rust_cache_cargo_value(config: &mut RustCacheConfig, key: &str, value: &str) {
    if key == "target_dir" {
        config.cargo_target_dir =
            parse_string(value).unwrap_or_else(|| config.cargo_target_dir.clone());
    }
}

fn apply_rust_cache_env_value(config: &mut RustCacheConfig, key: &str, value: &str) {
    if let Some(value) = parse_string(value) {
        config.env.insert(key.to_string(), value);
    }
}

fn apply_tauri_release_project_value(config: &mut TauriReleaseConfig, key: &str, value: &str) {
    match key {
        "root" => {
            config.project_root =
                parse_string(value).unwrap_or_else(|| config.project_root.clone());
        }
        "frontend_install" => {
            config.frontend_install =
                parse_string(value).unwrap_or_else(|| config.frontend_install.clone());
        }
        _ => {}
    }
}

fn apply_tauri_release_build_value(config: &mut TauriReleaseConfig, key: &str, value: &str) {
    match key {
        "out_dir" => {
            config.out_dir = parse_string(value).unwrap_or_else(|| config.out_dir.clone());
        }
        "cache_dir" => {
            config.cache_dir = parse_string(value).unwrap_or_else(|| config.cache_dir.clone());
        }
        _ => {}
    }
}

fn apply_release_target_value(target: &mut ReleaseTarget, key: &str, value: &str) {
    match key {
        "prepare_commands" => target.prepare_commands = parse_string_array(value),
        "cargo_targets" => target.cargo_targets = parse_string_array(value),
        "include_scopes" => target.include_scopes = parse_string_array(value),
        "include_paths" => target.include_paths = parse_string_array(value),
        "version_files" => target.version_files = parse_bool(value),
        "source_release" => target.source_release = parse_bool(value),
        "generate_notes" => target.generate_notes = parse_bool(value),
        "floating_tags" => target.floating_tags = parse_bool(value),
        "latest_tag" => target.latest_tag = parse_bool(value),
        "next_tag" => target.next_tag = parse_bool(value),
        _ => {
            let Some(value) = parse_string(value) else {
                return;
            };
            match key {
                "name" => target.name = value,
                "path" => target.path = value,
                "workspace" => target.workspace = value,
                "strategy" => target.strategy = value,
                "workflow" => target.workflow = value,
                "source_kind" => target.source_kind = value,
                "repository" | "target_repository" => target.repository = value,
                "source_repository" => target.source_repository = Some(value),
                "distribution_path" => target.distribution_path = value,
                "cargo_binary" => target.cargo_binary = value,
                "cargo_package" => target.cargo_package = value,
                "cargo_out_dir" => target.cargo_out_dir = value,
                "version_file" => target.version_file = value,
                "version_key" => target.version_key = value,
                "version_value" => target.version_value = value,
                "source_tag_prefix" => target.source_tag_prefix = value,
                "release_name_prefix" => target.release_name_prefix = value,
                _ => {}
            }
        }
    }
}

fn read_composer_dependencies(root: &Path) -> Result<BTreeMap<String, String>> {
    let path = root.join("composer.json");
    if !path.is_file() {
        return Ok(BTreeMap::new());
    }

    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let json: Value = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let mut dependencies = BTreeMap::new();

    for section in ["require", "require-dev"] {
        if let Some(map) = json.get(section).and_then(Value::as_object) {
            for (name, version) in map {
                dependencies.insert(
                    name.clone(),
                    version.as_str().unwrap_or_default().to_string(),
                );
            }
        }
    }

    Ok(dependencies)
}

fn read_mise_tools(root: &Path) -> Result<(bool, BTreeSet<String>)> {
    let paths = [root.join("mise.toml"), root.join(".mise.toml")];
    let Some(path) = paths.into_iter().find(|path| path.is_file()) else {
        return Ok((false, BTreeSet::new()));
    };

    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut in_tools = false;
    let mut tools = BTreeSet::new();

    for raw_line in text.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            in_tools = line == "[tools]";
            continue;
        }
        if !in_tools {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            let name = name.trim().trim_matches('"').trim_matches('\'');
            if !name.is_empty() {
                tools.insert(name.to_string());
            }
        }
    }

    Ok((true, tools))
}

fn resolve_js_runner(
    root: &Path,
    runner: JsRunnerArg,
    stored_runner: Option<&JsRunner>,
    mise_tools: &BTreeSet<String>,
) -> JsRunner {
    match runner {
        JsRunnerArg::Aube => JsRunner::Aube,
        JsRunnerArg::Npm => JsRunner::Npm,
        JsRunnerArg::Pnpm => JsRunner::Pnpm,
        JsRunnerArg::Yarn => JsRunner::Yarn,
        JsRunnerArg::Bun => JsRunner::Bun,
        JsRunnerArg::Auto => {
            if let Some(stored_runner) = stored_runner {
                stored_runner.clone()
            } else if root.join("aube-workspace.yaml").is_file() || mise_tools.contains("aube") {
                JsRunner::Aube
            } else if root.join("pnpm-lock.yaml").is_file() || mise_tools.contains("pnpm") {
                JsRunner::Pnpm
            } else if root.join("yarn.lock").is_file() || mise_tools.contains("yarn") {
                JsRunner::Yarn
            } else if root.join("bun.lockb").is_file()
                || root.join("bun.lock").is_file()
                || mise_tools.contains("bun")
            {
                JsRunner::Bun
            } else if root.join("package-lock.json").is_file() {
                JsRunner::Npm
            } else {
                JsRunner::Aube
            }
        }
    }
}

fn has_source_file(root: &Path, extensions: &[&str]) -> Result<bool> {
    let mut found = false;
    visit_source_files(root, extensions, &mut found)?;
    Ok(found)
}

fn visit_source_files(path: &Path, extensions: &[&str], found: &mut bool) -> Result<()> {
    if *found {
        return Ok(());
    }

    for entry in fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))? {
        let entry = entry.with_context(|| format!("failed to read entry in {}", path.display()))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if path.is_dir() {
            if should_skip_dir(&name) {
                continue;
            }
            visit_source_files(&path, extensions, found)?;
            if *found {
                return Ok(());
            }
            continue;
        }

        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extensions.contains(&extension))
            .unwrap_or(false)
        {
            *found = true;
            return Ok(());
        }
    }

    Ok(())
}

fn should_skip_dir(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".hg"
            | ".svn"
            | ".cache"
            | ".gradle"
            | "build"
            | "coverage"
            | "dist"
            | "gen"
            | "node_modules"
            | "target"
            | "vendor"
    )
}

fn parse_string(value: &str) -> Option<String> {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .map(|value| value.replace("\\\"", "\"").replace("\\\\", "\\"))
}

fn render_string_array(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", escape_toml(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_string_array(value: &str) -> Vec<String> {
    let value = value.trim();
    let Some(value) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Vec::new();
    };

    value
        .split(',')
        .filter_map(|item| parse_string(item.trim()))
        .collect()
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn bool_literal(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn escape_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn detects_languages_from_source_files() {
        let root = temp_repo("detect");
        fs::write(root.join("src.rs"), "fn main() {}\n").unwrap();
        fs::write(root.join("app.ts"), "export {};\n").unwrap();
        fs::write(root.join("index.php"), "<?php\n").unwrap();

        let profile = ProjectProfile::detect(root, None, None, &[], JsRunnerArg::Auto).unwrap();

        assert!(profile.has_language(&Language::Rust));
        assert!(profile.has_language(&Language::Js));
        assert!(profile.has_language(&Language::Php));
    }

    #[test]
    fn detects_languages_from_project_markers() {
        let root = temp_repo("detect-markers");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
        fs::write(root.join("package.json"), "{}\n").unwrap();
        fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").unwrap();
        fs::write(root.join("composer.json"), "{}\n").unwrap();

        let profile = ProjectProfile::detect(root, None, None, &[], JsRunnerArg::Auto).unwrap();

        assert_eq!(
            profile.languages,
            vec![Language::Rust, Language::Js, Language::Php]
        );
        assert_eq!(profile.js_runner, Some(JsRunner::Pnpm));
    }

    #[test]
    fn reads_datarose_quality_and_release_config() {
        let root = temp_repo("datarose");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
        fs::write(
            root.join(DEFAULT_CONFIG_FILE),
            r#"version = 1

[quality]
workspace = "."
languages = ["rust"]

[release]
enabled = true
source_repository = "verzly/toolchain"

[[release.targets]]
name = "repository"
repository = "verzly/repository"
cargo_binary = "repository"
"#,
        )
        .unwrap();

        let profile = ProjectProfile::detect(root, None, None, &[], JsRunnerArg::Auto).unwrap();

        assert_eq!(profile.languages, vec![Language::Rust]);
        assert!(profile.release_enabled());
        assert_eq!(profile.stored_config.release.targets[0].name, "repository");
    }

    #[test]
    fn renders_root_directory_name_as_default_cache_package() {
        let root = temp_repo("cache-package");
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
        fs::write(
            root.join(DEFAULT_CONFIG_FILE),
            r#"version = 1

[rust_cache.cache]
package = "auto"
"#,
        )
        .unwrap();

        let profile = ProjectProfile::detect(root, None, None, &[], JsRunnerArg::Auto).unwrap();
        let rendered = render_datarose_config(&profile);

        assert!(!rendered.contains("package = \"auto\""));
        assert!(rendered.contains(&format!("package = \"{}\"", profile.default_package_name())));
    }

    #[test]
    fn recommends_mise_tools_for_detected_languages() {
        let root = temp_repo("mise");
        fs::write(
            root.join("mise.toml"),
            "[tools]\nhk = \"latest\"\npkl = \"latest\"\n",
        )
        .unwrap();
        fs::write(root.join("src.rs"), "fn main() {}\n").unwrap();
        fs::write(root.join("app.ts"), "export {};\n").unwrap();
        fs::write(root.join("index.php"), "<?php\n").unwrap();

        let profile = ProjectProfile::detect(root, None, None, &[], JsRunnerArg::Auto).unwrap();
        let tools = profile
            .missing_mise_tools()
            .into_iter()
            .map(|recommendation| recommendation.tool)
            .collect::<BTreeSet<_>>();

        assert!(tools.contains("rust"));
        assert!(tools.contains("aube"));
        assert!(tools.contains("php"));
        assert!(tools.contains("composer"));
    }

    fn temp_repo(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("repository-{name}-{unique}"));
        fs::create_dir_all(&root).unwrap();
        root
    }
}
