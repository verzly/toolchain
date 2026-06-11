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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JsRunner {
    Aube,
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

#[derive(Clone, Debug)]
pub struct ProjectProfile {
    pub root: PathBuf,
    pub languages: Vec<Language>,
    pub js_runner: Option<JsRunner>,
    pub package_scripts: BTreeSet<String>,
    pub has_rector: bool,
    pub has_pest: bool,
}

impl ProjectProfile {
    pub fn detect(root: PathBuf, languages: &[LanguageArg], runner: JsRunnerArg) -> Result<Self> {
        let root = fs::canonicalize(&root)
            .with_context(|| format!("failed to resolve repository root: {}", root.display()))?;
        let package_scripts = read_package_scripts(&root)?;
        let composer = read_composer_dependencies(&root)?;
        let mut detected = BTreeSet::new();

        if root.join("Cargo.toml").is_file() {
            detected.insert(Language::Rust);
        }
        if root.join("package.json").is_file() || root.join("aube-workspace.yaml").is_file() {
            detected.insert(Language::Js);
        }
        if root.join("composer.json").is_file() {
            detected.insert(Language::Php);
        }

        for language in languages {
            detected.insert(match language {
                LanguageArg::Rust => Language::Rust,
                LanguageArg::Js => Language::Js,
                LanguageArg::Php => Language::Php,
            });
        }

        let languages = detected.into_iter().collect::<Vec<_>>();
        let js_runner = if languages.contains(&Language::Js) {
            Some(resolve_js_runner(&root, runner))
        } else {
            None
        };

        Ok(Self {
            root,
            languages,
            js_runner,
            package_scripts,
            has_rector: composer.contains_key("rector/rector")
                || composer.contains_key("rectorphp/rector"),
            has_pest: composer.contains_key("pestphp/pest"),
        })
    }

    pub fn has_language(&self, language: &Language) -> bool {
        self.languages.contains(language)
    }
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

fn resolve_js_runner(root: &Path, runner: JsRunnerArg) -> JsRunner {
    match runner {
        JsRunnerArg::Aube => JsRunner::Aube,
        JsRunnerArg::Npm => JsRunner::Npm,
        JsRunnerArg::Pnpm => JsRunner::Pnpm,
        JsRunnerArg::Yarn => JsRunner::Yarn,
        JsRunnerArg::Bun => JsRunner::Bun,
        JsRunnerArg::Auto => {
            if root.join("aube-workspace.yaml").is_file() {
                JsRunner::Aube
            } else if root.join("pnpm-lock.yaml").is_file() {
                JsRunner::Pnpm
            } else if root.join("yarn.lock").is_file() {
                JsRunner::Yarn
            } else if root.join("bun.lockb").is_file() || root.join("bun.lock").is_file() {
                JsRunner::Bun
            } else {
                JsRunner::Npm
            }
        }
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
            r#"{"scripts":{"format:js":"oxfmt","test:js":"vitest"}}"#,
        )
        .unwrap();

        let profile = ProjectProfile::detect(root, &[], JsRunnerArg::Auto).unwrap();

        assert!(profile.has_language(&Language::Js));
        assert_eq!(profile.js_runner, Some(JsRunner::Aube));
        assert!(profile.package_scripts.contains("format:js"));
    }

    #[test]
    fn detects_php_quality_dependencies() {
        let root = temp_repo("php");
        fs::write(
            root.join("composer.json"),
            r#"{"require-dev":{"rector/rector":"^2.0","pestphp/pest":"^3.0"}}"#,
        )
        .unwrap();

        let profile = ProjectProfile::detect(root, &[], JsRunnerArg::Auto).unwrap();

        assert!(profile.has_language(&Language::Php));
        assert!(profile.has_rector);
        assert!(profile.has_pest);
    }
}
