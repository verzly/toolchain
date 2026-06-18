//! Central repository standards style files.

use crate::project::{Language, ProjectProfile};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

#[derive(Clone, Debug)]
pub struct ManagedFile {
    pub path: PathBuf,
    pub content: String,
    pub force: bool,
}

#[derive(Clone, Debug)]
pub enum WriteOutcome {
    Wrote(PathBuf),
    Kept(PathBuf),
}

pub fn style_files(profile: &ProjectProfile, force: bool) -> Vec<ManagedFile> {
    let mut files = Vec::new();
    files.push(ManagedFile {
        path: profile.workspace_root.join(".editorconfig"),
        content: editorconfig(),
        force,
    });

    if profile.has_language(&Language::Rust) {
        files.push(ManagedFile {
            path: profile.quality_config_path("rustfmt.toml"),
            content: rustfmt_config(),
            force,
        });

        if profile.stored_config.quality.rust.manage_clippy_config {
            files.push(ManagedFile {
                path: profile.quality_config_path(".clippy.toml"),
                content: clippy_config(),
                force,
            });
        }

        if profile.stored_config.quality.rust.manage_cargo_lints {
            files.extend(cargo_toml_policy_files(profile, force));
        }
    }

    if profile.has_language(&Language::Js) {
        files.push(ManagedFile {
            path: profile.quality_config_path(".oxfmtrc.json"),
            content: oxfmt_config(),
            force,
        });
        files.push(ManagedFile {
            path: profile.quality_config_path(".oxlintrc.json"),
            content: oxlint_config(),
            force,
        });
        files.push(ManagedFile {
            path: profile.quality_config_path("vitest.config.ts"),
            content: vitest_config(),
            force,
        });
    }

    if profile.has_language(&Language::Php) {
        files.push(ManagedFile {
            path: profile.quality_config_path("rector.php"),
            content: rector_config(),
            force,
        });
        files.push(ManagedFile {
            path: profile.quality_config_path("phpunit.xml.dist"),
            content: pest_phpunit_config(),
            force,
        });
    }

    files
}

pub fn move_existing_style_configs(
    profile: &ProjectProfile,
    files: &[ManagedFile],
) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut moved = Vec::new();
    let movable_names = [
        "rustfmt.toml",
        ".clippy.toml",
        ".oxfmtrc.json",
        ".oxlintrc.json",
        "vitest.config.ts",
        "rector.php",
        "phpunit.xml.dist",
    ];

    for file in files {
        let Some(name) = file.path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !movable_names.contains(&name) || file.path.exists() {
            continue;
        }

        let candidates = [
            profile.workspace_root.join(name),
            profile
                .workspace_root
                .join(&profile.stored_config.quality.configs.directory)
                .join(name),
        ];

        for candidate in candidates {
            if candidate == file.path || !candidate.is_file() {
                continue;
            }
            if let Some(parent) = file.path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!(
                        "failed to create parent directory for {}",
                        file.path.display()
                    )
                })?;
            }
            fs::rename(&candidate, &file.path)
                .or_else(|_| -> std::io::Result<()> {
                    fs::copy(&candidate, &file.path)?;
                    fs::remove_file(&candidate)
                })
                .with_context(|| {
                    format!(
                        "failed to move {} to {}",
                        candidate.display(),
                        file.path.display()
                    )
                })?;
            moved.push((candidate, file.path.clone()));
            break;
        }
    }

    Ok(moved)
}

pub fn cargo_toml_policy_files(profile: &ProjectProfile, force: bool) -> Vec<ManagedFile> {
    let manifest = profile.workspace_root.join("Cargo.toml");
    let Ok(content) = fs::read_to_string(&manifest) else {
        return Vec::new();
    };

    let table = content.parse::<toml::Table>().ok();
    let is_workspace = table
        .as_ref()
        .and_then(|table| table.get("workspace"))
        .and_then(Value::as_table)
        .is_some();
    let prefix = if is_workspace {
        "workspace.lints"
    } else {
        "lints"
    };
    let content = upsert_toml_table_keys(
        &content,
        &format!("{prefix}.rust"),
        &profile.stored_config.quality.rust.rust_lints,
        force,
    );
    let content = upsert_toml_table_keys(
        &content,
        &format!("{prefix}.clippy"),
        &profile.stored_config.quality.rust.clippy_lints,
        force,
    );

    let mut files = vec![ManagedFile {
        path: manifest,
        content,
        force: true,
    }];

    if is_workspace {
        if let Some(table) = table.as_ref() {
            for member_manifest in cargo_workspace_member_manifests(&profile.workspace_root, table)
            {
                let Ok(content) = fs::read_to_string(&member_manifest) else {
                    continue;
                };
                let content = upsert_toml_table_keys(
                    &content,
                    "lints",
                    &BTreeMap::from([("workspace".into(), "true".into())]),
                    force,
                );
                files.push(ManagedFile {
                    path: member_manifest,
                    content,
                    force: true,
                });
            }
        }
    }

    files
}

fn cargo_workspace_member_manifests(root: &Path, manifest: &toml::Table) -> Vec<PathBuf> {
    let Some(members) = manifest
        .get("workspace")
        .and_then(Value::as_table)
        .and_then(|workspace| workspace.get("members"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    let mut manifests = Vec::new();
    for member in members.iter().filter_map(Value::as_str) {
        if member.contains('*') {
            manifests.extend(expand_member_glob(root, member));
            continue;
        }

        let manifest = root.join(member).join("Cargo.toml");
        if manifest.is_file() {
            manifests.push(manifest);
        }
    }
    manifests.sort();
    manifests.dedup();
    manifests
}

fn expand_member_glob(root: &Path, pattern: &str) -> Vec<PathBuf> {
    let Some((before_star, after_star)) = pattern.split_once('*') else {
        return Vec::new();
    };
    let base = before_star.trim_end_matches('/');
    let suffix = after_star.trim_start_matches('/');
    let search_root = if base.is_empty() {
        root.to_path_buf()
    } else {
        root.join(base)
    };
    let Ok(entries) = fs::read_dir(search_root) else {
        return Vec::new();
    };

    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(|path| {
            if suffix.is_empty() {
                path.join("Cargo.toml")
            } else {
                path.join(suffix).join("Cargo.toml")
            }
        })
        .filter(|path| path.is_file())
        .collect()
}

fn upsert_toml_table_keys(
    content: &str,
    section: &str,
    defaults: &BTreeMap<String, String>,
    force: bool,
) -> String {
    let mut lines = content.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
    let Some((start, end)) = find_toml_section(&lines, section) else {
        if !lines
            .last()
            .map(|line| line.trim().is_empty())
            .unwrap_or(true)
        {
            lines.push(String::new());
        }
        lines.push(format!("[{section}]"));
        for (key, value) in defaults {
            lines.push(format_toml_key_value(key, value));
        }
        return finish_lines(lines, content.ends_with('\n'));
    };

    let mut insertions = Vec::new();
    for (key, value) in defaults {
        if let Some(index) = find_key_line(&lines, start + 1, end, key) {
            if force {
                lines[index] = format_toml_key_value(key, value);
            }
        } else {
            insertions.push(format_toml_key_value(key, value));
        }
    }

    for line in insertions.into_iter().rev() {
        lines.insert(end, line);
    }

    finish_lines(lines, content.ends_with('\n'))
}

fn find_toml_section(lines: &[String], section: &str) -> Option<(usize, usize)> {
    let header = format!("[{section}]");
    let start = lines.iter().position(|line| line.trim() == header)?;
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, line)| is_toml_header(line))
        .map(|(index, _)| index)
        .unwrap_or(lines.len());
    Some((start, end))
}

fn find_key_line(lines: &[String], start: usize, end: usize, key: &str) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .take(end)
        .skip(start)
        .find_map(|(index, line)| {
            let line = line.split('#').next().unwrap_or_default().trim();
            let (candidate, _) = line.split_once('=')?;
            (candidate.trim() == key).then_some(index)
        })
}

fn is_toml_header(line: &str) -> bool {
    let line = line.trim();
    line.starts_with('[') && line.ends_with(']')
}

fn format_toml_key_value(key: &str, value: &str) -> String {
    if matches!(value, "true" | "false") {
        format!("{key} = {value}")
    } else {
        format!(
            "{key} = \"{}\"",
            value.replace('\\', "\\\\").replace('"', "\\\"")
        )
    }
}

fn finish_lines(lines: Vec<String>, trailing_newline: bool) -> String {
    let mut content = lines.join("\n");
    if trailing_newline || !content.ends_with('\n') {
        content.push('\n');
    }
    content
}

pub fn write_files(files: &[ManagedFile]) -> Result<Vec<WriteOutcome>> {
    let mut outcomes = Vec::new();

    for file in files {
        if file.path.exists() && !file.force {
            outcomes.push(WriteOutcome::Kept(file.path.clone()));
            continue;
        }

        if let Some(parent) = file.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create parent directory for {}",
                    file.path.display()
                )
            })?;
        }

        fs::write(&file.path, &file.content)
            .with_context(|| format!("failed to write {}", file.path.display()))?;
        outcomes.push(WriteOutcome::Wrote(file.path.clone()));
    }

    Ok(outcomes)
}

pub fn editorconfig() -> String {
    r#"root = true

[*]
charset = utf-8
end_of_line = lf
indent_style = space
indent_size = 2
insert_final_newline = true
trim_trailing_whitespace = true

[*.{js,jsx,ts,tsx,vue,json,jsonc,yaml,yml,md,css,scss,html}]
indent_size = 2

[*.{php,rs}]
indent_size = 4
"#
    .into()
}

pub fn rustfmt_config() -> String {
    r#"hard_tabs = false
tab_spaces = 4
newline_style = "Unix"
"#
    .into()
}

pub fn clippy_config() -> String {
    r#"avoid-breaking-exported-api = false

too-many-arguments-threshold = 8
type-complexity-threshold = 250
cognitive-complexity-threshold = 30

disallowed-names = [
  "foo",
  "bar",
  "baz",
  "tmp",
]
"#
    .into()
}

pub fn oxfmt_config() -> String {
    r#"{
  "$schema": "./node_modules/oxfmt/configuration_schema.json",
  "printWidth": 100,
  "tabWidth": 2,
  "useTabs": false,
  "semi": true,
  "singleQuote": false,
  "jsxSingleQuote": false,
  "bracketSpacing": true,
  "bracketSameLine": false,
  "singleAttributePerLine": false,
  "objectWrap": "preserve",
  "proseWrap": "preserve",
  "ignorePatterns": [
    "node_modules/**",
    "dist/**",
    "build/**",
    "coverage/**",
    ".cache/**",
    "target/**",
    "vendor/**"
  ]
}
"#
    .into()
}

pub fn oxlint_config() -> String {
    r#"{
  "$schema": "./node_modules/oxlint/configuration_schema.json",
  "categories": {
    "correctness": "error",
    "suspicious": "error",
    "perf": "warn"
  },
  "env": {
    "browser": true,
    "builtin": true,
    "node": true
  },
  "ignorePatterns": [
    "node_modules/**",
    "dist/**",
    "build/**",
    "coverage/**",
    ".cache/**",
    "target/**",
    "vendor/**"
  ],
  "rules": {
    "eqeqeq": "error",
    "no-var": "error",
    "prefer-const": "error"
  },
  "overrides": [
    {
      "files": ["*.test.ts", "*.spec.ts", "**/*.test.ts", "**/*.spec.ts"],
      "rules": {
        "no-console": "off"
      }
    }
  ]
}
"#
    .into()
}

pub fn vitest_config() -> String {
    r#"import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    coverage: {
      reporter: ['text', 'json', 'html'],
      reportsDirectory: '.cache/vitest/coverage',
    },
    exclude: [
      '**/node_modules/**',
      '**/dist/**',
      '**/build/**',
      '**/.cache/**',
      '**/target/**',
      '**/vendor/**',
    ],
  },
});
"#
    .into()
}

pub fn pest_phpunit_config() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<phpunit bootstrap="vendor/autoload.php" colors="true" cacheDirectory=".cache/phpunit">
  <testsuites>
    <testsuite name="Application">
      <directory>tests</directory>
    </testsuite>
  </testsuites>
  <source>
    <include>
      <directory>src</directory>
    </include>
  </source>
</phpunit>
"#
    .into()
}

pub fn rector_config() -> String {
    r#"<?php

declare(strict_types=1);

use Rector\CodeQuality\Rector\Class_\InlineConstructorDefaultToPropertyRector;
use Rector\Config\RectorConfig;
use Rector\Set\ValueObject\LevelSetList;

return RectorConfig::configure()
    ->withPaths([
        getcwd() . '/src',
        getcwd() . '/tests',
    ])
    ->withPhpSets()
    ->withPreparedSets(
        deadCode: true,
        codeQuality: true,
        codingStyle: true,
        typeDeclarations: true,
        privatization: true,
        naming: true,
        instanceOf: true,
        earlyReturn: true,
        strictBooleans: true,
    )
    ->withSets([
        LevelSetList::UP_TO_PHP_84,
    ])
    ->withSkip([
        InlineConstructorDefaultToPropertyRector::class,
    ]);
"#
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upserts_workspace_lints_without_overwriting_project_overrides() {
        let mut defaults = BTreeMap::new();
        defaults.insert("all".to_string(), "deny".to_string());
        defaults.insert("unwrap_used".to_string(), "warn".to_string());
        let content = r#"[workspace]
resolver = "2"

[workspace.lints.clippy]
all = "warn"
"#;

        let updated = upsert_toml_table_keys(content, "workspace.lints.clippy", &defaults, false);

        assert!(updated.contains("all = \"warn\""));
        assert!(updated.contains("unwrap_used = \"warn\""));
    }

    #[test]
    fn force_updates_managed_lint_defaults() {
        let mut defaults = BTreeMap::new();
        defaults.insert("all".to_string(), "deny".to_string());
        let content = r#"[workspace.lints.clippy]
all = "warn"
"#;

        let updated = upsert_toml_table_keys(content, "workspace.lints.clippy", &defaults, true);

        assert!(updated.contains("all = \"deny\""));
    }
}
