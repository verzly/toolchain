//! Central repository quality style files.

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

    files
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
