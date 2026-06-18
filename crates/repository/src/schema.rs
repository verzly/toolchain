//! Strict datarose.toml schema validation.
//!
//! The repository tool keeps `datarose.toml` as TOML, but validates it as a real schema instead of
//! silently ignoring unknown keys. This catches typos such as `langauges`, wrong value types, and
//! missing release target fields before a workflow relies on the configuration.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use toml::{Table, Value};

pub const DATAROSE_SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/verzly/toolchain/master/schemas/datarose.toml.schema.json";
pub const DATAROSE_SCHEMA_DIRECTIVE: &str =
    "#:schema https://raw.githubusercontent.com/verzly/toolchain/master/schemas/datarose.toml.schema.json";

pub fn validate_datarose_schema(path: &Path) -> Result<Vec<String>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }

    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value = match raw.parse::<Value>() {
        Ok(value) => value,
        Err(error) => {
            return Ok(vec![format!(
                "{} has invalid TOML syntax: {error}",
                path.display()
            )]);
        }
    };

    let Some(root) = value.as_table() else {
        return Ok(vec!["datarose.toml must be a TOML table".into()]);
    };

    let mut issues = Vec::new();
    validate_schema_directive(&raw, &mut issues);
    validate_root(root, &mut issues);
    Ok(issues)
}

fn validate_schema_directive(raw: &str, issues: &mut Vec<String>) {
    let mut seen_config = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == DATAROSE_SCHEMA_DIRECTIVE {
            return;
        }
        if trimmed.starts_with("#:schema ") {
            issues.push(format!(
                "datarose.toml schema directive must be `{DATAROSE_SCHEMA_DIRECTIVE}`; found `{trimmed}`"
            ));
            return;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        seen_config = true;
        break;
    }

    if seen_config {
        issues.push(format!(
            "datarose.toml is missing the schema directive; add `{DATAROSE_SCHEMA_DIRECTIVE}` as the first line"
        ));
    }
}

fn validate_root(root: &Table, issues: &mut Vec<String>) {
    validate_unknown_keys(
        root,
        "datarose.toml",
        &[
            "version",
            "quality",
            "release",
            "rust_cache",
            "tauri_release",
        ],
        issues,
    );

    match root.get("version") {
        Some(Value::Integer(value)) if *value == 1 => {}
        Some(Value::Integer(value)) => {
            issues.push(format!("datarose.toml.version must be 1; found {value}"))
        }
        Some(value) => issues.push(format!(
            "datarose.toml.version must be an integer; found {}",
            type_name(value)
        )),
        None => issues.push("datarose.toml.version is missing; set `version = 1`".into()),
    }

    if let Some(table) = table_field(root, "quality", "datarose.toml.quality", issues) {
        validate_quality(table, issues);
    }
    if let Some(table) = table_field(root, "release", "datarose.toml.release", issues) {
        validate_release(table, issues);
    }
    if let Some(table) = table_field(root, "rust_cache", "datarose.toml.rust_cache", issues) {
        validate_rust_cache(table, issues);
    }
    if let Some(table) = table_field(root, "tauri_release", "datarose.toml.tauri_release", issues) {
        validate_tauri_release(table, issues);
    }
}

fn validate_quality(table: &Table, issues: &mut Vec<String>) {
    validate_unknown_keys(
        table,
        "quality",
        &["workspace", "languages", "js_runner"],
        issues,
    );
    expect_string(table, "workspace", "quality.workspace", issues);
    if let Some(values) = expect_string_array(table, "languages", "quality.languages", issues) {
        for value in values {
            if !matches!(
                value.as_str(),
                "rust" | "js" | "javascript" | "typescript" | "vue" | "php"
            ) {
                issues.push(format!(
                    "quality.languages contains unsupported language `{value}`; expected one of rust, js, php"
                ));
            }
        }
    }
    if let Some(runner) = expect_string(table, "js_runner", "quality.js_runner", issues) {
        if !matches!(runner.as_str(), "aube" | "npm" | "pnpm" | "yarn" | "bun") {
            issues.push(format!(
                "quality.js_runner has unsupported value `{runner}`; expected one of aube, npm, pnpm, yarn, bun"
            ));
        }
    }
}

fn validate_release(table: &Table, issues: &mut Vec<String>) {
    validate_unknown_keys(
        table,
        "release",
        &[
            "enabled",
            "target_branch",
            "source_repository",
            "secret_name",
            "release_all",
            "manage_cargo_packages",
            "manage_workflows",
            "targets",
        ],
        issues,
    );
    expect_bool(table, "enabled", "release.enabled", issues);
    expect_string(table, "target_branch", "release.target_branch", issues);
    expect_string(
        table,
        "source_repository",
        "release.source_repository",
        issues,
    );
    expect_string(table, "secret_name", "release.secret_name", issues);
    expect_bool(table, "release_all", "release.release_all", issues);
    expect_bool(
        table,
        "manage_cargo_packages",
        "release.manage_cargo_packages",
        issues,
    );
    expect_bool(
        table,
        "manage_workflows",
        "release.manage_workflows",
        issues,
    );

    let release_enabled = table
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if release_enabled && !string_present(table, "source_repository") {
        issues.push("release.source_repository is required when release.enabled is true".into());
    }

    match table.get("targets") {
        Some(Value::Array(targets)) => {
            if release_enabled && targets.is_empty() {
                issues.push(
                    "release.targets must contain at least one target when release.enabled is true"
                        .into(),
                );
            }
            for (index, target) in targets.iter().enumerate() {
                let path = format!("release.targets[{index}]");
                let Some(target_table) = target.as_table() else {
                    issues.push(format!("{path} must be a table"));
                    continue;
                };
                validate_release_target(target_table, &path, issues);
            }
        }
        Some(value) => issues.push(format!(
            "release.targets must be an array of tables; found {}",
            type_name(value)
        )),
        None if release_enabled => {
            issues.push("release.targets is required when release.enabled is true".into())
        }
        None => {}
    }
}

fn validate_release_target(table: &Table, path: &str, issues: &mut Vec<String>) {
    validate_unknown_keys(
        table,
        path,
        &[
            "name",
            "path",
            "workspace",
            "strategy",
            "workflow",
            "source_kind",
            "repository",
            "target_repository",
            "source_repository",
            "distribution_path",
            "cargo_binary",
            "cargo_package",
            "cargo_out_dir",
            "cargo_targets",
            "prepare_commands",
            "version_files",
            "version_file",
            "version_file_optional",
            "version_key",
            "version_value",
            "source_tag_prefix",
            "release_tag_prefix",
            "release_name_prefix",
            "source_release",
            "generate_notes",
            "floating_tags",
            "latest_tag",
            "next_tag",
            "include_scopes",
            "include_paths",
            "files",
            "release",
            "github",
        ],
        issues,
    );

    if !string_present(table, "name") {
        issues.push(format!("{path}.name is required"));
    }

    for key in [
        "name",
        "path",
        "workspace",
        "strategy",
        "workflow",
        "source_kind",
        "repository",
        "target_repository",
        "source_repository",
        "distribution_path",
        "cargo_binary",
        "cargo_package",
        "cargo_out_dir",
        "version_file",
        "version_key",
        "version_value",
        "source_tag_prefix",
        "release_tag_prefix",
        "release_name_prefix",
    ] {
        expect_string(table, key, &format!("{path}.{key}"), issues);
    }
    for key in [
        "cargo_targets",
        "prepare_commands",
        "include_scopes",
        "include_paths",
    ] {
        expect_string_array(table, key, &format!("{path}.{key}"), issues);
    }
    for key in [
        "version_files",
        "version_file_optional",
        "generate_notes",
        "floating_tags",
        "latest_tag",
        "next_tag",
    ] {
        expect_bool(table, key, &format!("{path}.{key}"), issues);
    }

    if let Some(strategy) = table.get("strategy").and_then(Value::as_str) {
        if !matches!(strategy, "same-repo" | "distribution-repo" | "workspace") {
            issues.push(format!(
                "{path}.strategy has unsupported value `{strategy}`; expected one of same-repo, distribution-repo, workspace"
            ));
        }
    }
    if let Some(workflow) = table.get("workflow").and_then(Value::as_str) {
        if !matches!(workflow, "custom" | "managed") {
            issues.push(format!(
                "{path}.workflow has unsupported value `{workflow}`; expected one of custom, managed"
            ));
        }
    }

    if let Some(value) = table.get("source_release") {
        match value {
            Value::Boolean(_) => {}
            Value::Table(source_release) => {
                validate_release_table(source_release, &format!("{path}.source_release"), issues)
            }
            value => issues.push(format!(
                "{path}.source_release must be a boolean or table; found {}",
                type_name(value)
            )),
        }
    }
    if let Some(table) = table_field(table, "release", &format!("{path}.release"), issues) {
        validate_release_table(table, &format!("{path}.release"), issues);
    }
    if let Some(table) = table_field(table, "github", &format!("{path}.github"), issues) {
        validate_github_table(table, &format!("{path}.github"), issues);
    }

    match table.get("files") {
        Some(Value::Array(files)) => {
            for (index, file) in files.iter().enumerate() {
                let file_path = format!("{path}.files[{index}]");
                let Some(file_table) = file.as_table() else {
                    issues.push(format!("{file_path} must be a table"));
                    continue;
                };
                validate_version_file(file_table, &file_path, issues);
            }
        }
        Some(value) => issues.push(format!(
            "{path}.files must be an array of tables; found {}",
            type_name(value)
        )),
        None => {}
    }
}

fn validate_release_table(table: &Table, path: &str, issues: &mut Vec<String>) {
    validate_unknown_keys(
        table,
        path,
        &[
            "target_branch",
            "branch_prefix",
            "tag_prefix",
            "tag_suffix",
            "name_prefix",
            "name_suffix",
            "commit_message",
            "merge_message",
            "cleanup",
            "latest",
            "floating_tags",
            "latest_tag",
            "next_tag",
            "latest_tag_name",
            "next_tag_name",
        ],
        issues,
    );
    for key in [
        "target_branch",
        "branch_prefix",
        "tag_prefix",
        "tag_suffix",
        "name_prefix",
        "name_suffix",
        "commit_message",
        "merge_message",
        "latest_tag_name",
        "next_tag_name",
    ] {
        expect_string(table, key, &format!("{path}.{key}"), issues);
    }
    for key in [
        "cleanup",
        "latest",
        "floating_tags",
        "latest_tag",
        "next_tag",
    ] {
        expect_bool(table, key, &format!("{path}.{key}"), issues);
    }
}

fn validate_github_table(table: &Table, path: &str, issues: &mut Vec<String>) {
    validate_unknown_keys(
        table,
        path,
        &[
            "target_repository",
            "source_repository",
            "source_tag_prefix",
            "source_tag_suffix",
            "generate_notes",
            "notes_body",
            "notes",
        ],
        issues,
    );
    for key in [
        "target_repository",
        "source_repository",
        "source_tag_prefix",
        "source_tag_suffix",
        "notes_body",
    ] {
        expect_string(table, key, &format!("{path}.{key}"), issues);
    }
    expect_bool(
        table,
        "generate_notes",
        &format!("{path}.generate_notes"),
        issues,
    );
    if let Some(table) = table_field(table, "notes", &format!("{path}.notes"), issues) {
        validate_unknown_keys(
            table,
            &format!("{path}.notes"),
            &["mode", "include_scopes", "include_paths"],
            issues,
        );
        if let Some(mode) = expect_string(table, "mode", &format!("{path}.notes.mode"), issues) {
            if !matches!(mode.as_str(), "github" | "scoped") {
                issues.push(format!(
                    "{path}.notes.mode has unsupported value `{mode}`; expected one of github, scoped"
                ));
            }
        }
        expect_string_array(
            table,
            "include_scopes",
            &format!("{path}.notes.include_scopes"),
            issues,
        );
        expect_string_array(
            table,
            "include_paths",
            &format!("{path}.notes.include_paths"),
            issues,
        );
    }
}

fn validate_version_file(table: &Table, path: &str, issues: &mut Vec<String>) {
    validate_unknown_keys(
        table,
        path,
        &[
            "path",
            "kind",
            "key",
            "value",
            "value_type",
            "search",
            "replace",
            "package",
            "optional",
        ],
        issues,
    );
    if !string_present(table, "path") {
        issues.push(format!("{path}.path is required"));
    }
    for key in [
        "path",
        "kind",
        "key",
        "value",
        "value_type",
        "search",
        "replace",
        "package",
    ] {
        expect_string(table, key, &format!("{path}.{key}"), issues);
    }
    expect_bool(table, "optional", &format!("{path}.optional"), issues);

    if let Some(kind) = table.get("kind").and_then(Value::as_str) {
        if !matches!(kind, "toml" | "json" | "text" | "cargo-lock-package") {
            issues.push(format!(
                "{path}.kind has unsupported value `{kind}`; expected one of toml, json, text, cargo-lock-package"
            ));
        }
    }
    if let Some(value_type) = table.get("value_type").and_then(Value::as_str) {
        if !matches!(value_type, "string" | "integer") {
            issues.push(format!(
                "{path}.value_type has unsupported value `{value_type}`; expected one of string, integer"
            ));
        }
    }
}

fn validate_rust_cache(table: &Table, issues: &mut Vec<String>) {
    validate_unknown_keys(
        table,
        "rust_cache",
        &["cache", "cargo", "generated", "env"],
        issues,
    );
    if let Some(cache) = table_field(table, "cache", "rust_cache.cache", issues) {
        validate_unknown_keys(
            cache,
            "rust_cache.cache",
            &["dir", "package", "redirect_cargo_home", "redirect_gradle"],
            issues,
        );
        expect_string(cache, "dir", "rust_cache.cache.dir", issues);
        expect_string(cache, "package", "rust_cache.cache.package", issues);
        expect_bool(
            cache,
            "redirect_cargo_home",
            "rust_cache.cache.redirect_cargo_home",
            issues,
        );
        expect_bool(
            cache,
            "redirect_gradle",
            "rust_cache.cache.redirect_gradle",
            issues,
        );
    }
    if let Some(cargo) = table_field(table, "cargo", "rust_cache.cargo", issues) {
        validate_unknown_keys(cargo, "rust_cache.cargo", &["target_dir"], issues);
        expect_string(cargo, "target_dir", "rust_cache.cargo.target_dir", issues);
    }
    if let Some(generated) = table_field(table, "generated", "rust_cache.generated", issues) {
        validate_unknown_keys(generated, "rust_cache.generated", &["paths"], issues);
        expect_string_array(generated, "paths", "rust_cache.generated.paths", issues);
    }
    if let Some(env) = table_field(table, "env", "rust_cache.env", issues) {
        for (key, value) in env {
            if value.as_str().is_none() {
                issues.push(format!(
                    "rust_cache.env.{key} must be a string; found {}",
                    type_name(value)
                ));
            }
        }
    }
}

fn validate_tauri_release(table: &Table, issues: &mut Vec<String>) {
    validate_unknown_keys(table, "tauri_release", &["project", "build"], issues);
    if let Some(project) = table_field(table, "project", "tauri_release.project", issues) {
        validate_unknown_keys(
            project,
            "tauri_release.project",
            &["root", "frontend_install"],
            issues,
        );
        expect_string(project, "root", "tauri_release.project.root", issues);
        expect_string(
            project,
            "frontend_install",
            "tauri_release.project.frontend_install",
            issues,
        );
    }
    if let Some(build) = table_field(table, "build", "tauri_release.build", issues) {
        validate_unknown_keys(
            build,
            "tauri_release.build",
            &["out_dir", "cache_dir"],
            issues,
        );
        expect_string(build, "out_dir", "tauri_release.build.out_dir", issues);
        expect_string(build, "cache_dir", "tauri_release.build.cache_dir", issues);
    }
}

fn validate_unknown_keys(table: &Table, path: &str, allowed: &[&str], issues: &mut Vec<String>) {
    let allowed = allowed.iter().copied().collect::<BTreeSet<_>>();
    for key in table.keys() {
        if !allowed.contains(key.as_str()) {
            issues.push(format!("{path}.{key} is not a supported datarose.toml key"));
        }
    }
}

fn table_field<'a>(
    table: &'a Table,
    key: &str,
    path: &str,
    issues: &mut Vec<String>,
) -> Option<&'a Table> {
    match table.get(key) {
        Some(Value::Table(table)) => Some(table),
        Some(value) => {
            issues.push(format!(
                "{path} must be a table; found {}",
                type_name(value)
            ));
            None
        }
        None => None,
    }
}

fn string_present(table: &Table, key: &str) -> bool {
    table
        .get(key)
        .and_then(Value::as_str)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn expect_string(table: &Table, key: &str, path: &str, issues: &mut Vec<String>) -> Option<String> {
    match table.get(key) {
        Some(Value::String(value)) => Some(value.clone()),
        Some(value) => {
            issues.push(format!(
                "{path} must be a string; found {}",
                type_name(value)
            ));
            None
        }
        None => None,
    }
}

fn expect_bool(table: &Table, key: &str, path: &str, issues: &mut Vec<String>) -> Option<bool> {
    match table.get(key) {
        Some(Value::Boolean(value)) => Some(*value),
        Some(value) => {
            issues.push(format!(
                "{path} must be a boolean; found {}",
                type_name(value)
            ));
            None
        }
        None => None,
    }
}

fn expect_string_array(
    table: &Table,
    key: &str,
    path: &str,
    issues: &mut Vec<String>,
) -> Option<Vec<String>> {
    match table.get(key) {
        Some(Value::Array(values)) => {
            let mut strings = Vec::new();
            for (index, value) in values.iter().enumerate() {
                match value.as_str() {
                    Some(value) => strings.push(value.to_string()),
                    None => issues.push(format!(
                        "{path}[{index}] must be a string; found {}",
                        type_name(value)
                    )),
                }
            }
            Some(strings)
        }
        Some(value) => {
            issues.push(format!(
                "{path} must be an array of strings; found {}",
                type_name(value)
            ));
            None
        }
        None => None,
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "string",
        Value::Integer(_) => "integer",
        Value::Float(_) => "float",
        Value::Boolean(_) => "boolean",
        Value::Datetime(_) => "datetime",
        Value::Array(_) => "array",
        Value::Table(_) => "table",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn accepts_current_schema_surface() {
        let path = temp_config(
            "valid",
            r#"#:schema https://raw.githubusercontent.com/verzly/toolchain/master/schemas/datarose.toml.schema.json
version = 1

[quality]
workspace = "."
languages = ["rust", "js"]
js_runner = "pnpm"

[release]
enabled = true
target_branch = "master"
source_repository = "verzly/toolchain"

[[release.targets]]
name = "verzly"
path = "."
strategy = "same-repo"
workflow = "custom"
repository = "verzly/toolchain"
files = [
  { path = "Cargo.toml", kind = "toml", key = "package.version", value = "{version}" },
]

[rust_cache.cache]
dir = ".cache"
package = "toolchain"
redirect_cargo_home = false
redirect_gradle = true

[rust_cache.generated]
paths = []

[tauri_release.project]
root = "."
frontend_install = "aube install"
"#,
        );

        assert!(validate_datarose_schema(&path).unwrap().is_empty());
    }

    #[test]
    fn reports_unknown_and_missing_keys() {
        let path = temp_config(
            "invalid",
            r#"version = 1

[quality]
langauges = ["rust"]

[release]
enabled = true

[[release.targets]]
nam = "verzly"
"#,
        );

        let issues = validate_datarose_schema(&path).unwrap();
        assert!(issues
            .iter()
            .any(|issue| issue.contains("quality.langauges")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("release.source_repository")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("release.targets[0].nam")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("release.targets[0].name")));
    }

    #[test]
    fn reports_missing_schema_directive() {
        let path = temp_config("missing-directive", "version = 1\n");

        let issues = validate_datarose_schema(&path).unwrap();

        assert!(issues
            .iter()
            .any(|issue| issue.contains("missing the schema directive")));
    }

    fn temp_config(name: &str, content: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("datarose-schema-{name}-{unique}.toml"));
        fs::write(&path, content).unwrap();
        path
    }
}
