# repo-quality

`repo-quality` bootstraps repository-local quality gates for Rust, JavaScript, TypeScript, Vue, and PHP projects.

It carries the Verzly default quality model as an executable: `mise` tools, `hk` hooks, GitHub Actions, `.editorconfig`, Rust formatting, Oxlint, Oxfmt, Vitest, Rector PHP, and Pest PHP.

- [Purpose](#purpose)
- [Install](#install)
- [How it works](#how-it-works)
- [Commands](#commands)
  - [init](#init)
  - [update](#update)
  - [plan](#plan)
  - [doctor](#doctor)
- [Generated files](#generated-files)
- [Language profiles](#language-profiles)
- [Monorepos and workspaces](#monorepos-and-workspaces)
- [Project overrides](#project-overrides)
- [GitHub Actions](#github-actions)
- [Command help](#command-help)
- [License](#license)

## Purpose

`repo-quality` centralizes setup that would otherwise be repeated manually in every repository:

- install `hk` and `pkl` through `mise`;
- add language tools such as `rust@stable`, `aube`, `php`, `composer`, `npm:oxlint`, `npm:oxfmt`, and `npm:vitest` when needed;
- generate a Windows-safe `hk.pkl`;
- generate `.editorconfig` and formatter/linter config files;
- generate a pull request test workflow with concurrency cancellation and WIP guarding;
- run `hk install` so Git hooks are active.

The tool intentionally does not add package scripts to `package.json` or `composer.json`. Hook commands call tools directly through `hk check` and the `mise` environment.

## Install

Use the GitHub Action to install the executable in CI:

```yaml
- uses: verzly/repo-quality@v0.2
  with:
    install-only: true
```

For local usage, install the published executable or make it available through your preferred `mise` setup, then run it from the repository root.

When developing `repo-quality` inside `verzly/toolchain`, a public release is not required:

```sh
cargo run -p repo-quality -- plan
cargo run -p repo-quality -- init --dry-run --skip-mise-use --skip-hk-install
cargo run -p repo-quality -- update --dry-run --skip-mise-use --skip-hk-install
```

## How it works

A typical setup is:

```sh
repo-quality init
```

For monorepos, store quality files under a specific workspace folder and remember that folder for future updates:

```sh
repo-quality init --workspace apps/mobile
repo-quality update
```

`repo-quality init` writes `datarose.toml` at the repository root. `repo-quality update` reads it, so the workspace path does not need to be repeated.

## Configuration file

By default, `repo-quality` reads and writes `datarose.toml` in the repository root. Use `--config <path>` with `init`, `update`, `plan`, or `doctor` when a repository needs a custom config location.

The same file stores quality settings and optional release targets:

```toml
version = 1

[quality]
workspace = "."
languages = ["rust", "js", "php"]
js_runner = "aube"

[release]
enabled = true
target_branch = "master"
source_repository = "verzly/toolchain"
secret_name = "DISTRIBUTION_REPO_TOKEN"
release_all = true

[[release.targets]]
name = "repo-quality"
repository = "verzly/repo-quality"
distribution_path = ".codex/distributions/repo-quality"
prepare_commands = ["cargo generate-lockfile"]
version_file = "crates/repo-quality/Cargo.toml"
version_key = "package.version"
version_value = "{version}"
source_tag_prefix = "repo-quality-v"
generate_notes = false
include_scopes = ["repo-quality", "all"]
include_paths = ["crates/repo-quality/"]
```

`repo-quality update` refreshes generated `hk.pkl`, test workflows, release workflows, and missing style/config files from this model. Existing project-local formatter/linter configs are preserved unless `--force` is passed.

## Commands

### init

Prepare the repository:

```sh
repo-quality init
```

Preview without writing files:

```sh
repo-quality init --dry-run
```

Initialize a monorepo workspace:

```sh
repo-quality init --workspace workspace/app
```

Force selected language profiles:

```sh
repo-quality init --language rust --language js --language php
```

Overwrite managed files:

```sh
repo-quality init --force
```

Skip tool or hook installation:

```sh
repo-quality init --skip-mise-use
repo-quality init --skip-hk-install
```

### update


Validate the Datarose configuration without writing files:

```sh
repo-quality check
repo-quality check --config config/datarose.toml
```

`repo-quality check` exits with `1` for removed, deprecated, or invalid settings and exits with `0` when the configuration is clean. Generated `hk` pre-push hooks run this check before language quality gates.

Refresh managed files from `datarose.toml`:

```sh
repo-quality update
```

Use this after updating `repo-quality` to roll the latest central standards into a repository.

Project-local overrides are preserved by default. Pass `--force` only when you intentionally want to replace existing local config files.

### plan

Print the detected repository profile, managed files, generated `hk.pkl`, and generated test workflow without changing the repository:

```sh
repo-quality plan
```

### doctor

Check whether the repository has the expected setup:

```sh
repo-quality doctor
```

`doctor` reports missing required pieces and prints setup recommendations. It can suggest `mise.toml`, `rust@stable`, `aube`, `php`, Composer, Oxlint, Oxfmt, Vitest, Rector PHP, Pest PHP, workspace config files, and GitHub Actions workflow files.

## Generated files

Depending on detected languages, `repo-quality` can manage:

```text
datarose.toml
hk.pkl
.github/workflows/test.yml
.github/workflows/release-*.yml
.github/workflows/release-all.yml
.github/workflows/_release-datarose-tool.yml
.editorconfig
rustfmt.toml
.oxfmtrc.json
.oxlintrc.json
rector.php
```

Default indentation:

```text
JavaScript / TypeScript / Vue: 2 spaces
PHP / Rust: 4 spaces
```

## Language profiles

### Rust

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

### JavaScript, TypeScript, and Vue

```text
oxfmt --check .
oxlint .
vitest run
```

The generated Oxfmt config enables semicolons and uses two-space indentation. Oxlint uses project-local `.oxlintrc.json` so repositories can adjust rules as needed.

### PHP

```text
composer exec rector -- --dry-run
composer exec rector
composer exec pest
```

Rector and Pest must be installed as project development dependencies:

```sh
composer require --dev rector/rector pestphp/pest
```

## Monorepos and workspaces

Use `--workspace` when the quality configuration should live below a subdirectory:

```sh
repo-quality init --workspace workspace/app
```

Generated hook commands run from that workspace:

```text
cd "workspace/app" && oxfmt --check .
cd "workspace/app" && vitest run
```

The root `datarose.toml` stores the workspace path for future updates.

## Project overrides

Every generated config file is project-local and can be edited.

`repo-quality update` preserves existing local config files unless `--force` is passed, and prints warnings when deprecated or removed Datarose settings are still present. This lets each project override central defaults without changing the executable.

Examples:

```text
.oxlintrc.json     project-specific Oxlint rules and overrides
.oxfmtrc.json      project-specific formatter options
rector.php         project-specific Rector sets and skips
rustfmt.toml       project-specific Rust formatting options
.editorconfig      project-specific editor behavior
```

## GitHub Actions

The generated test workflow exposes one pull request check:

```text
Test / Quality
```

It cancels older in-progress runs when a new push arrives and stops early for commits whose subject starts with `wip`.

The workflow runs:

```sh
mise exec -- hk check
```

## Command help

```sh
repo-quality --help
repo-quality <command> --help
```

Full documentation is available in the repository README:

```text
https://github.com/verzly/repo-quality
```

## License

AGPL-3.0-only.
