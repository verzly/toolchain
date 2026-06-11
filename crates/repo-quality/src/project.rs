//! Project detection for quality profiles.

use crate::cli::{JsRunnerArg, LanguageArg};
use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

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
pub struct RepoQualityConfig {
    pub workspace: Option<PathBuf>,
    pub languages: Vec<Language>,
    pub js_runner: Option<JsRunner>,
}

#[derive(Clone, Debug)]
pub struct ProjectProfile {
    pub root: PathBuf,
    pub workspace: PathBuf,
    pub workspace_root: PathBuf,
    pub languages: Vec<Language>,
    pub js_runner: Option<JsRunner>,
    pub package_scripts: BTreeSet<String>,
    pub has_rector: bool,
    pub has_pest: bool,
    pub has_mise_toml: bool,
    pub mise_tools: BTreeSet<String>,
    pub stored_config: RepoQualityConfig,
}

impl ProjectProfile {
    pub fn detect(
        root: PathBuf,
        workspace: Option<PathBuf>,
        languages: &[LanguageArg],
        runner: JsRunnerArg,
    ) -> Result<Self> {
        let root = fs::canonicalize(&root)
            .with_context(|| format!("failed to resolve repository root: {}", root.display()))?;
        let stored_config = read_repo_quality_config(&root)?;
        let workspace = workspace
            .or_else(|| stored_config.workspace.clone())
            .unwrap_or_else(|| PathBuf::from("."));
        let workspace_root = fs::canonicalize(root.join(&workspace)).with_context(|| {
            format!(
                "failed to resolve repository quality workspace: {}",
                root.join(&workspace).display()
            )
        })?;
        let package_scripts = read_package_scripts(&workspace_root)?;
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
            stored_config.languages.clone()
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
                stored_config.js_runner.as_ref(),
                &mise_tools,
            ))
        } else {
            None
        };

        Ok(Self {
            root,
            workspace,
            workspace_root,
            languages,
            js_runner,
            package_scripts,
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
        self.workspace == PathBuf::from(".") || self.workspace.as_os_str().is_empty()
    }

    pub fn workspace_display(&self) -> String {
        normalize_path(&self.workspace)
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

pub fn repo_quality_config_path(root: &Path) -> PathBuf {
    root.join(".repo-quality.toml")
}

pub fn render_repo_quality_config(profile: &ProjectProfile) -> String {
    let languages = profile
        .languages
        .iter()
        .map(|language| format!("\"{}\"", language.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    let mut out = String::new();
    out.push_str("# Managed by repo-quality. Project-specific overrides are allowed.\n");
    out.push_str("version = 1\n");
    out.push_str(&format!(
        "workspace = \"{}\"\n",
        escape_toml(&profile.workspace_display())
    ));
    out.push_str(&format!("languages = [{languages}]\n"));
    if let Some(runner) = &profile.js_runner {
        out.push_str(&format!("js_runner = \"{}\"\n", runner.as_str()));
    }
    out
}

fn read_repo_quality_config(root: &Path) -> Result<RepoQualityConfig> {
    let path = repo_quality_config_path(root);
    if !path.is_file() {
        return Ok(RepoQualityConfig::default());
    }

    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut config = RepoQualityConfig::default();

    for raw_line in text.lines() {
        let line = raw_line.split('#').next().unwrap_or_default().trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
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

    Ok(config)
}

fn read_package_scripts(root: &Path) -> Result<BTreeSet<String>> {
    let path = root.join("package.json");
    if !path.is_file() {
        return Ok(BTreeSet::new());
    }

    let text =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let json: Value = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let scripts = json
        .get("scripts")
        .and_then(Value::as_object)
        .map(|scripts| scripts.keys().cloned().collect())
        .unwrap_or_default();

    Ok(scripts)
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

fn escape_toml(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn normalize_path(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    if value.is_empty() {
        ".".into()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_repo(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be available")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("repo-quality-{name}-{suffix}"));
        fs::create_dir_all(&path).expect("repo should be created");
        path
    }

    #[test]
    fn detects_aube_javascript_project() {
        let root = temp_repo("aube");
        fs::write(root.join("aube-workspace.yaml"), "packages: []\n").unwrap();
        fs::write(
            root.join("package.json"),
            r#"{"scripts":{"test":"vitest"}}"#,
        )
        .unwrap();

        let profile = ProjectProfile::detect(root, None, &[], JsRunnerArg::Auto).unwrap();

        assert!(profile.has_language(&Language::Js));
        assert_eq!(profile.js_runner, Some(JsRunner::Aube));
    }

    #[test]
    fn detects_php_quality_dependencies() {
        let root = temp_repo("php");
        fs::write(
            root.join("composer.json"),
            r#"{"require-dev":{"rector/rector":"^2.0","pestphp/pest":"^3.0"}}"#,
        )
        .unwrap();

        let profile = ProjectProfile::detect(root, None, &[], JsRunnerArg::Auto).unwrap();

        assert!(profile.has_language(&Language::Php));
        assert!(profile.has_rector);
        assert!(profile.has_pest);
    }

    #[test]
    fn detects_languages_from_source_files() {
        let root = temp_repo("sources");
        fs::write(root.join("main.rs"), "fn main() {}\n").unwrap();
        fs::write(root.join("app.ts"), "export {};\n").unwrap();
        fs::write(root.join("index.php"), "<?php\n").unwrap();

        let profile = ProjectProfile::detect(root, None, &[], JsRunnerArg::Auto).unwrap();

        assert!(profile.has_language(&Language::Rust));
        assert!(profile.has_language(&Language::Js));
        assert!(profile.has_language(&Language::Php));
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

        let profile = ProjectProfile::detect(root, None, &[], JsRunnerArg::Auto).unwrap();
        let tools = profile
            .missing_mise_tools()
            .into_iter()
            .map(|recommendation| recommendation.tool)
            .collect::<BTreeSet<_>>();

        assert!(tools.contains("rust"));
        assert!(tools.contains("aube"));
        assert!(tools.contains("npm:oxlint"));
        assert!(tools.contains("npm:oxfmt"));
        assert!(tools.contains("npm:vitest"));
        assert!(tools.contains("php"));
        assert!(tools.contains("composer"));
        assert!(!tools.contains("hk"));
        assert!(!tools.contains("pkl"));
    }

    #[test]
    fn skips_aube_recommendation_when_other_runner_is_configured() {
        let root = temp_repo("pnpm");
        fs::write(root.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\n").unwrap();
        fs::write(root.join("mise.toml"), "[tools]\npnpm = \"latest\"\n").unwrap();
        fs::write(root.join("app.ts"), "export {};\n").unwrap();

        let profile = ProjectProfile::detect(root, None, &[], JsRunnerArg::Auto).unwrap();
        let tools = profile
            .missing_mise_tools()
            .into_iter()
            .map(|recommendation| recommendation.tool)
            .collect::<BTreeSet<_>>();

        assert_eq!(profile.js_runner, Some(JsRunner::Pnpm));
        assert!(!tools.contains("aube"));
        assert!(!tools.contains("pnpm"));
        assert!(tools.contains("npm:oxlint"));
    }

    #[test]
    fn uses_stored_workspace_for_update_detection() {
        let root = temp_repo("workspace");
        fs::create_dir_all(root.join("workspace/app")).unwrap();
        fs::write(
            root.join(".repo-quality.toml"),
            concat!(
                "version = 1\n",
                "workspace = \"workspace/app\"\n",
                "languages = [\"js\"]\n",
                "js_runner = \"aube\"\n",
            ),
        )
        .unwrap();
        fs::write(root.join("workspace/app/main.ts"), "export {};\n").unwrap();

        let profile = ProjectProfile::detect(root, None, &[], JsRunnerArg::Auto).unwrap();

        assert_eq!(profile.workspace_display(), "workspace/app");
        assert!(profile.has_language(&Language::Js));
        assert_eq!(profile.js_runner, Some(JsRunner::Aube));
    }
}
