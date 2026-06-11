//! Central repository standards style files.

use crate::project::{Language, ProjectProfile};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

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
            path: profile.workspace_root.join("rustfmt.toml"),
            content: rustfmt_config(),
            force,
        });
    }

    if profile.has_language(&Language::Js) {
        files.push(ManagedFile {
            path: profile.workspace_root.join(".oxfmtrc.json"),
            content: oxfmt_config(),
            force,
        });
        files.push(ManagedFile {
            path: profile.workspace_root.join(".oxlintrc.json"),
            content: oxlint_config(),
            force,
        });
    }

    if profile.has_language(&Language::Php) {
        files.push(ManagedFile {
            path: profile.workspace_root.join("rector.php"),
            content: rector_config(),
            force,
        });
    }

    if profile.ast_grep_enabled() {
        files.extend(ast_grep_files(profile, force));
    }

    files
}

fn ast_grep_files(profile: &ProjectProfile, force: bool) -> Vec<ManagedFile> {
    let config = &profile.stored_config.quality.ast_grep;
    let mut files = Vec::new();
    files.push(ManagedFile {
        path: profile.workspace_root.join(&config.config),
        content: ast_grep_config(config),
        force,
    });

    let rule_dir = config
        .rule_dirs
        .first()
        .map(String::as_str)
        .unwrap_or(".datarose/ast-grep/rules");
    files.push(ManagedFile {
        path: profile
            .workspace_root
            .join(rule_dir)
            .join("datarose-no-debugger.yml"),
        content: ast_grep_no_debugger_rule(),
        force,
    });

    if !config.test_dir.trim().is_empty() {
        files.push(ManagedFile {
            path: profile
                .workspace_root
                .join(&config.test_dir)
                .join(".gitkeep"),
            content: String::new(),
            force,
        });
    }

    for util_dir in &config.util_dirs {
        if util_dir.trim().is_empty() {
            continue;
        }
        files.push(ManagedFile {
            path: profile.workspace_root.join(util_dir).join(".gitkeep"),
            content: String::new(),
            force,
        });
    }

    files
}

fn ast_grep_config(config: &crate::project::AstGrepConfig) -> String {
    let mut out = String::new();
    out.push_str("ruleDirs:\n");
    for rule_dir in non_empty_or_default(&config.rule_dirs, ".datarose/ast-grep/rules") {
        out.push_str(&format!("  - {}\n", yaml_string(&rule_dir)));
    }
    if !config.util_dirs.is_empty() {
        out.push_str("utilDirs:\n");
        for util_dir in config.util_dirs.iter().filter(|dir| !dir.trim().is_empty()) {
            out.push_str(&format!("  - {}\n", yaml_string(util_dir)));
        }
    }
    if !config.test_dir.trim().is_empty() {
        out.push_str("testConfigs:\n");
        out.push_str(&format!("  - testDir: {}\n", yaml_string(&config.test_dir)));
    }
    out
}

fn non_empty_or_default(values: &[String], default: &str) -> Vec<String> {
    let values = values
        .iter()
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if values.is_empty() {
        vec![default.into()]
    } else {
        values
    }
}

fn ast_grep_no_debugger_rule() -> String {
    r#"id: datarose-no-debugger-js
language: JavaScript
severity: error
message: Remove debugger statements before committing.
rule:
  pattern: debugger
---
id: datarose-no-debugger-ts
language: TypeScript
severity: error
message: Remove debugger statements before committing.
rule:
  pattern: debugger
"#
    .into()
}

fn yaml_string(value: &str) -> String {
    let value = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{value}\"")
}

pub fn write_files(files: &[ManagedFile]) -> Result<Vec<WriteOutcome>> {
    let mut outcomes = Vec::new();
    for file in files {
        if file.path.exists() && !file.force {
            outcomes.push(WriteOutcome::Kept(file.path.clone()));
            continue;
        }
        if let Some(parent) = file.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
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

pub fn rector_config() -> String {
    r#"<?php

declare(strict_types=1);

use Rector\CodeQuality\Rector\Class_\InlineConstructorDefaultToPropertyRector;
use Rector\Config\RectorConfig;
use Rector\Set\ValueObject\LevelSetList;

return RectorConfig::configure()
    ->withPaths([
        __DIR__ . '/src',
        __DIR__ . '/tests',
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
