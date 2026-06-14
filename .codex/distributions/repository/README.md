# repository

`repository` bootstraps and maintains repository-local quality gates for Rust, JavaScript, TypeScript, Vue, and PHP projects. It detects the project shape, writes the expected Datarose configuration, creates quality workflows, and can manage release targets from CLI flags or a fullscreen terminal command center.

This public repository is a distribution surface. The source code, release configuration, and distribution templates live in [`verzly/toolchain`](https://github.com/verzly/toolchain); this repository contains the public `README.md`, `CONTRIBUTING.md`, `action.yml`, and `LICENSE`.

- [Overview](#overview)
  - [Why this exists](#why-this-exists)
  - [How it works](#how-it-works)
  - [Use cases](#use-cases)
- [Get started](#get-started)
  - [GitHub Action](#github-action)
- [Usage](#usage)
  - [Action inputs](#action-inputs)
  - [Action outputs](#action-outputs)
  - [CLI usage](#cli-usage)
  - [CLI commands and arguments](#cli-commands-and-arguments)
- [Configuration](#configuration)
- [Practical workflows](#practical-workflows)
  - [Practical repository workflows](#practical-repository-workflows)
- [Reference](#reference)
  - [Troubleshooting](#troubleshooting)
  - [Release artifacts](#release-artifacts)
  - [Operational notes](#operational-notes)

## Overview

### Why this exists

Most engineering teams repeat the same repository setup work: formatter config, lint rules, test commands, hooks, CI workflows, and release workflow boilerplate. `repository` keeps that setup explicit and repeatable without hiding it in project-specific shell scripts.

The tool is intentionally repository-local. It writes normal files such as `datarose.toml`, `hk.pkl`, `.github/workflows/test.yml`, `.editorconfig`, `rustfmt.toml`, `.oxfmtrc.json`, `.oxlintrc.json`, and `rector.php`. Teams can review those files, commit them, and override local formatter or linter settings when a project needs a narrower rule.

### How it works

`repository` reads the repository root, detects supported language profiles, and renders a consistent quality model:

- Rust: `cargo fmt`, `cargo clippy`, and `cargo test`.
- JavaScript, TypeScript, and Vue: Oxfmt, Oxlint, and Vitest.
- PHP: Rector and Pest.
- Git hooks: `hk` pre-commit and pre-push hooks.
- CI: a pull request quality workflow that runs `mise exec -- hk check`.
- Releases: optional managed release workflow files derived from `[[release.targets]]` in `datarose.toml`.

Every operation is available through CLI flags. Running `repository` opens the fullscreen TUI for local work, while subcommands such as `repository check`, `repository projects`, and `repository release set` remain scriptable for CI and automation.

### Use cases

- Bootstrap quality gates in a new Rust, PHP, JavaScript, TypeScript, Vue, or mixed repository.
- Keep repository standards current with `repository update`.
- Preview generated files before writing them with `--dry-run`.
- Validate Datarose configuration in CI with `repository check`.
- Diagnose local setup drift with `repository doctor`.
- Add release targets for monorepos and generate managed release workflow files.
- Use an interactive terminal dashboard locally while keeping CI fully flag-driven.

## Get started

### GitHub Action

Run `repository check` in CI after installing the executable through the action:

```yaml
name: Repository Standards

on:
  pull_request:

permissions:
  contents: read

jobs:
  repository:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: verzly/repository@v0.2
        with:
          install-only: "true"
      - run: repository check
```

Run a command directly through the action:

```yaml
- uses: verzly/repository@v0.2
  with:
    args: plan --root .
```

Use `repository init --dry-run` in CI when you want to verify what would be generated without changing files:

```yaml
- uses: verzly/repository@v0.2
  with:
    args: init --dry-run --skip-mise-use --skip-hk-install
```

## Usage

### Action inputs

| Input | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `github-token` | No | `""` | Any GitHub token with read access to this repository's releases | Used by the action to download the published executable asset. The default `github.token` is enough for public releases. |
| `version` | No | `""` | `1.2.3`, `v1.2.3`, `latest`, `next`, `v1`, `v1.2`, or an empty string | Selects the release asset to download. Empty uses the action ref when it is a release selector, otherwise the latest release. |
| `install-only` | No | `"false"` | `"true"` or `"false"` | When `"true"`, installs `repository` and adds it to `PATH` without running it. |
| `args` | No | `"--help"` | Any valid `repository` CLI argument string | Arguments passed to `repository` when `install-only` is not `"true"`. |
| `working-directory` | No | `"."` | Any checkout-relative directory | Directory where the command from `args` runs. |

### Action outputs

| Output | Value | Purpose |
| --- | --- | --- |
| `bin-path` | Absolute path to the installed executable | Lets later workflow steps invoke the exact downloaded binary. |
| `host-target` | Resolved asset target such as `linux-x64`, `macos-arm64`, or `windows-x64` | Shows which release asset matched the current runner. |

### CLI usage

Preview the detected quality profile:

```sh
repository plan
```

Inspect detected projects and release coverage:

```sh
repository projects
```

Prepare a repository:

```sh
repository init
```

Refresh generated files from `datarose.toml`:

```sh
repository update
```

Open the interactive command center:

```sh
repository
repository tui
```

Configure a release target from flags:

```sh
repository release set \
  --path crates/repository \
  --repository verzly/repository \
  --strategy distribution-repo \
  --workflow managed
```

Generate the managed release workflows after changing release targets:

```sh
repository update
```

### CLI commands and arguments

#### `init`

Writes `datarose.toml`, `hk.pkl`, quality config files, and GitHub Actions workflows.

```sh
repository init
repository init --dry-run
repository init --workspace apps/web --language js --js-runner pnpm
```

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `--root <path>`, `-r <path>` | No | `.` | Any repository root path | Repository root to inspect and modify. |
| `--config <path>`, `-c <path>` | No | `<root>/datarose.toml` | Any TOML file path | Custom Datarose config location. |
| `--force`, `-f` | No | `false` | Flag | Overwrites existing managed files. |
| `--dry-run` | No | `false` | Flag | Prints planned changes without writing files or running commands. |
| `--skip-mise-use` | No | `false` | Flag | Skips `mise use` for missing tool recommendations. |
| `--skip-hk-install` | No | `false` | Flag | Skips `hk install` after writing files. |
| `--language <value>` | No | detected languages | `rust`, `js`, `php`; repeatable | Adds or overrides detected language profiles. |
| `--js-runner <value>` | No | `auto` | `auto`, `aube`, `npm`, `pnpm`, `yarn`, `bun` | Selects the JavaScript package runner used in recommendations. |
| `--workspace <path>` | No | `.` or `[quality].workspace` | Any repository-relative path | Subdirectory that owns language quality files and commands. |
| `--skip-style-configs` | No | `false` | Flag | Skips `.editorconfig`, formatter, linter, and Rector files. |
| `--skip-actions` | No | `false` | Flag | Skips generated GitHub Actions workflows. |

#### `update`

Refreshes managed files from the existing `datarose.toml`.

```sh
repository update
repository update --dry-run --skip-mise-use --skip-hk-install
```

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `--root <path>`, `-r <path>` | No | `.` | Any repository root path | Repository root to modify. |
| `--config <path>`, `-c <path>` | No | `<root>/datarose.toml` | Any TOML file path | Custom Datarose config location. |
| `--force`, `-f` | No | `false` | Flag | Overwrites existing project-local quality files. |
| `--dry-run` | No | `false` | Flag | Prints planned changes without writing files or running commands. |
| `--skip-mise-use` | No | `false` | Flag | Skips missing tool installation through `mise use`. |
| `--skip-hk-install` | No | `false` | Flag | Skips `hk install`. |
| `--skip-style-configs` | No | `false` | Flag | Leaves style config files untouched. |
| `--skip-actions` | No | `false` | Flag | Leaves GitHub Actions workflow files untouched. |

#### `plan`

Prints the detected profile and generated file contents without changing the repository.

```sh
repository plan
repository plan --workspace packages/api --language rust --language php
```

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `--root <path>`, `-r <path>` | No | `.` | Any repository root path | Repository root to inspect. |
| `--config <path>`, `-c <path>` | No | `<root>/datarose.toml` | Any TOML file path | Custom Datarose config location. |
| `--language <value>` | No | detected languages | `rust`, `js`, `php`; repeatable | Adds or overrides language profiles for the preview. |
| `--js-runner <value>` | No | `auto` | `auto`, `aube`, `npm`, `pnpm`, `yarn`, `bun` | Selects JavaScript runner detection for the preview. |
| `--workspace <path>` | No | `.` or `[quality].workspace` | Any repository-relative path | Preview a subdirectory workspace. |

#### `projects`

Prints detected languages, Cargo packages, and release target coverage without changing files.

```sh
repository projects
repository projects --root apps/mobile
```

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `--root <path>`, `-r <path>` | No | `.` | Any repository root path | Repository root to inspect. |
| `--config <path>`, `-c <path>` | No | `<root>/datarose.toml` | Any TOML file path | Custom Datarose config location. |

#### `check`

Validates `datarose.toml` and fails for deprecated, removed, duplicate, or invalid settings.

```sh
repository check
repository check --config config/datarose.toml
```

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `--root <path>`, `-r <path>` | No | `.` | Any repository root path | Repository root to validate. |
| `--config <path>`, `-c <path>` | No | `<root>/datarose.toml` | Any TOML file path | Custom Datarose config location. |

#### `doctor`

Checks whether the local repository setup is ready.

```sh
repository doctor
```

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `--root <path>`, `-r <path>` | No | `.` | Any repository root path | Repository root to inspect. |
| `--config <path>`, `-c <path>` | No | `<root>/datarose.toml` | Any TOML file path | Custom Datarose config location. |

#### `tui`

Opens the fullscreen terminal command center. Running `repository` with no subcommand opens the same TUI. The TUI starts in `PLAN` mode, where write operations run as dry-runs or print commands. Switch to `ACT` mode with `/mode act` when you want write-capable commands; mutating commands still ask for confirmation before changing files, installing hooks, building artifacts, or running release steps.

```sh
repository
repository tui
repository tui --root apps/mobile
```

The command palette accepts slash commands and number shortcuts:

| Command | CLI equivalent | Purpose |
| --- | --- | --- |
| `/projects` | `repository projects --root .` | Inspect languages, Cargo packages, and release target coverage. |
| `/plan` | `repository plan --root .` | Show detected quality settings and the release graph. |
| `/customize` in `PLAN` mode | `repository init --dry-run --workspace <path> --language <lang>` | Preview personalized workspace, language, and JS runner settings. |
| `/customize` in `ACT` mode | `repository init --force --workspace <path> --language <lang>` | Write personalized managed files after prompts. |
| `/check` | `repository check --root .` | Validate configuration, distribution templates, README/action docs, and release workflows. |
| `/doctor` | `repository doctor --root .` | Inspect local tool availability and quality setup readiness. |
| `/update` in `PLAN` mode | `repository update --dry-run --skip-mise-use --skip-hk-install` | Preview managed-file changes without writing files or running installers. |
| `/update` in `ACT` mode | `repository update` | Refresh managed files after interactive confirmations. |
| `/init` in `PLAN` mode | `repository init --dry-run --skip-mise-use --skip-hk-install` | Preview bootstrap output. |
| `/init` in `ACT` mode | `repository init` | Bootstrap managed files after interactive confirmations. |
| `/targets` | `repository release` | Open the release target editor. |
| `/release` in `PLAN` mode | `github-release plan` and `cargo-release build --dry-run` | Pick version and target, then print the release/build/publish commands. |
| `/release` in `ACT` mode | `github-release plan`, `cargo-release build`, or printed workflow commands | Run the selected release planning or build step after prompts. |
| `/mode plan` | Not needed in automation | Return to preview-only behavior. |
| `/mode act` | Not needed in automation | Enable write-capable behavior with prompts. |
| `/quit` | Not needed in automation | Exit the TUI. |

The fullscreen TUI uses lazygit-style navigation and cancellation rules:

| Key | Action |
| --- | --- |
| `j`, `k`, arrow keys | Move the selected action. |
| `PgUp`, `PgDn` | Move five actions at a time. |
| `g`, `G`, `<`, `>` | Jump to the first or last action. |
| `1`-`9` | Run an action directly. |
| `/` | Type a slash command. |
| `?` | Open or close the keybindings modal. |
| `R` | Refresh the dashboard state without writing files. |
| `Tab`, `p`, `a` | Toggle or set `PLAN` and `ACT` mode. |
| `Esc`, `q` | Close the active modal first; from the top level, exit the TUI. |
| `Ctrl+C` | Exit immediately and restore the terminal. |

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `--root <path>`, `-r <path>` | No | `.` | Any repository root path | Repository root for the dashboard. |
| `--config <path>`, `-c <path>` | No | `<root>/datarose.toml` | Any TOML file path | Custom Datarose config location. |

#### `release list`

Lists configured release targets.

```sh
repository release list
```

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `--root <path>`, `-r <path>` | No | `.` | Any repository root path | Repository root containing `datarose.toml`. |
| `--config <path>`, `-c <path>` | No | `<root>/datarose.toml` | Any TOML file path | Custom Datarose config location. |

#### `release show`

Shows one release target.

```sh
repository release show api
```

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `<target>` | Yes | none | A configured release target name | Target to display. |
| `--root <path>`, `-r <path>` | No | `.` | Any repository root path | Repository root containing `datarose.toml`. |
| `--config <path>`, `-c <path>` | No | `<root>/datarose.toml` | Any TOML file path | Custom Datarose config location. |

#### `release set`

Adds or updates a release target. This command writes `datarose.toml`; run `repository update` afterward to write managed workflow files.

```sh
repository release set \
  --path packages/api \
  --name api \
  --repository acme/api \
  --strategy distribution-repo \
  --workflow managed
```

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `--path <path>` | Yes | none | Repository-relative app, package, crate, or library path | Source path owned by the release target. |
| `--name <name>` | No | Directory name from `--path` | Any release target name | Stable target name used in workflow names, tag prefixes, scopes, and release selection. |
| `--repository <owner/repo>` | No | `""` | GitHub repository name | Public or same-repo release repository. Required for `distribution-repo`. |
| `--strategy <value>` | No | inferred from repository | `same-repo`, `distribution-repo`, `self-hosted`, `custom` | Release ownership model. Managed workflow generation supports `same-repo` and `distribution-repo`. |
| `--workflow <value>` | No | `custom` | `managed`, `preserve`, `custom` | Whether `repository update` may own workflow files for this target. |
| `--workspace <path>` | No | `""` | Any workspace id or path | Groups targets in monorepos when a release target belongs to a workspace. |
| `--source-kind <value>` | No | detected from files | `cargo-package`, `tauri-app`, `js-package`, `php-package`, `custom` | Source type used for version-file and artifact defaults. |
| `--cargo-package <name>` | No | detected Cargo package or target name | Any Cargo package name | Cargo package to build for `cargo-package` targets. |
| `--cargo-binary <name>` | No | Cargo package name | Any Cargo binary name | Binary packaged for release assets. |
| `--cargo-out-dir <path>` | No | `dist/<name>` | Any output path | Directory for built release assets. |
| `--distribution-path <path>` | No | `.codex/distributions/<name>` for distribution repos | Any repository-relative path | Public distribution template directory. |
| `--version-file <path>` | No | `<path>/Cargo.toml` for Cargo packages | Any version file path | File updated during release preparation. |
| `--source-tag-prefix <prefix>` | No | `<name>-v` | Any tag prefix | Source repository tag prefix. |
| `--allow-missing-path` | No | `false` | Flag | Allows configuring a target before its source path exists. |

#### `release remove`

Removes a release target by name or path.

```sh
repository release remove api --yes
repository release remove --path packages/api --yes
```

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `<target>` | No | none | A configured release target name | Target to remove. |
| `--path <path>` | No | none | A configured release target path | Removes the target associated with a path. |
| `--yes`, `-y` | No | `false` | Flag | Skips the confirmation prompt. |

#### `release tui`

Opens the interactive release target editor.

```sh
repository release tui
```

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `--root <path>`, `-r <path>` | No | `.` | Any repository root path | Repository root containing `datarose.toml`. |
| `--config <path>`, `-c <path>` | No | `<root>/datarose.toml` | Any TOML file path | Custom Datarose config location. |

## Configuration

`repository` reads and writes `datarose.toml` by default. The file stores quality detection overrides, release workflow targets, and cache/build defaults used by the wider Datarose toolchain.

```toml
version = 1

[quality]
workspace = "."
languages = ["rust", "js", "php"]
js_runner = "pnpm"

[release]
enabled = true
target_branch = "main"
source_repository = "acme/platform"
secret_name = "DISTRIBUTION_REPO_TOKEN"
release_all = true
manage_cargo_packages = false
manage_workflows = true

[[release.targets]]
name = "api"
path = "packages/api"
strategy = "distribution-repo"
workflow = "managed"
source_kind = "cargo-package"
repository = "acme/api"
distribution_path = ".codex/distributions/api"
cargo_binary = "api"
cargo_package = "api"
cargo_out_dir = "dist/api"
cargo_targets = ["linux-x64", "macos-x64", "macos-arm64", "windows-x64"]
prepare_commands = ["cargo generate-lockfile"]
version_file = "packages/api/Cargo.toml"
version_key = "package.version"
version_value = "{version}"
source_tag_prefix = "api-v"
generate_notes = false
include_scopes = ["api", "all"]
include_paths = ["packages/api/"]

[rust_cache.cache]
dir = ".cache"
package = "platform"
redirect_cargo_home = false
redirect_gradle = true

[rust_cache.cargo]
target_dir = "rust/packages/{package}/target"

[rust_cache.env]
NPM_CONFIG_CACHE = "js/npm"
PNPM_STORE_PATH = "js/pnpm-store"
YARN_CACHE_FOLDER = "js/yarn"

[tauri_release.project]
root = "."
frontend_install = "pnpm install --frozen-lockfile"

[tauri_release.build]
out_dir = "dist"
cache_dir = ".cache/tauri-release"
```

| Field | Accepted values | Purpose |
| --- | --- | --- |
| `quality.workspace` | Repository-relative path | Directory where language files and commands should be evaluated. |
| `quality.languages` | `rust`, `js`, `php` | Explicit language profiles. Missing values are detected from source files. |
| `quality.js_runner` | `aube`, `npm`, `pnpm`, `yarn`, `bun` | JavaScript runner used for tool recommendations. |
| `release.enabled` | `true`, `false` | Enables release target validation and workflow generation. |
| `release.target_branch` | Branch name | Target branch for generated release workflows. |
| `release.source_repository` | `owner/repo` | Source repository used by release tooling and notes. |
| `release.secret_name` | GitHub secret name | Secret passed to generated release workflows as `DISTRIBUTION_REPO_TOKEN`. |
| `release.release_all` | `true`, `false` | Writes `release-all.yml` when more than one managed target exists. |
| `release.manage_cargo_packages` | `true`, `false` | Requires every detected Cargo package to have a release target during `repository check`. |
| `release.manage_workflows` | `true`, `false` | Allows `repository update` to write managed release workflow files. |
| `release.targets[].name` | Unique target name | Release selector, workflow suffix, default scope, and default tag prefix source. |
| `release.targets[].path` | Repository-relative path | Source path owned by the release target. |
| `release.targets[].strategy` | `same-repo`, `distribution-repo`, `self-hosted`, `custom` | Defines where release publishing happens and whether managed workflows can be generated. |
| `release.targets[].workflow` | `managed`, `preserve`, `custom` | Controls whether `repository update` owns workflow files. |
| `release.targets[].source_kind` | `cargo-package`, `tauri-app`, `js-package`, `php-package`, `custom` | Defines version and artifact defaults. |
| `release.targets[].repository` | `owner/repo` or empty string | Public or same-repo release repository. |
| `release.targets[].distribution_path` | Repository-relative path | Distribution template path for public repository surfaces. |
| `release.targets[].version_file` | Repository-relative file path | File updated to the requested release version. |
| `release.targets[].include_scopes` | List of Conventional Commit scopes | Scopes included in generated release notes. |
| `release.targets[].include_paths` | List of repository-relative paths | Source paths included in generated release notes. |
| `rust_cache.cache.*` | Cache paths and booleans | Defaults consumed by `rust-cache` when this repository uses the wider toolchain. |
| `tauri_release.*` | Paths and commands | Defaults consumed by `tauri-release` for Tauri projects. |

## Practical workflows

### Practical repository workflows

Bootstrap a mixed repository without running external installation commands:

```sh
repository init --dry-run
repository init --skip-mise-use --skip-hk-install
repository doctor
```

Bootstrap a monorepo workspace:

```sh
repository init --workspace apps/web --language js --js-runner pnpm
repository update
```

Use the TUI locally to inspect and choose the workflow, then run the equivalent CLI in automation:

```sh
repository
repository tui
repository projects
repository plan
repository update --dry-run --skip-mise-use --skip-hk-install
```

Customize a workspace interactively, then commit the generated files:

```sh
repository
# /customize
# /mode act
# /customize
repository check
```

Add a managed release workflow for a package:

```sh
repository release set \
  --path packages/api \
  --name api \
  --repository acme/api \
  --strategy distribution-repo \
  --workflow managed

repository update
repository check
```

Plan and build a release from the TUI, then use the printed command in automation:

```sh
repository
# /release
cargo-release build --config datarose.toml --release-target api --version 1.2.3 --dry-run
cargo-release build --config datarose.toml --release-target api --version 1.2.3
```

Validate repository standards in GitHub Actions:

```yaml
name: Repository Standards

on:
  pull_request:

permissions:
  contents: read

jobs:
  standards:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: verzly/repository@v0.2
        with:
          install-only: "true"
      - uses: jdx/mise-action@v4
        with:
          cache: true
      - run: repository check
      - run: mise exec -- hk check
```

## Reference

### Troubleshooting

If no language profile is detected, pass one or more explicit language flags:

```sh
repository init --language rust --language js --language php
```

If `repository update` refuses to run because `datarose.toml` is missing, run `repository init` once first. `update` is intentionally config-driven.

If generated release workflow files are missing, check these settings:

```toml
[release]
enabled = true
manage_workflows = true

[[release.targets]]
workflow = "managed"
strategy = "distribution-repo"
```

If `doctor` reports `core.hooksPath`, remove the local Git override before installing `hk` hooks:

```sh
git config --local --unset-all core.hooksPath
```

### Release artifacts

Each public release publishes standalone executables named with this pattern:

```text
repository-vX.Y.Z-linux-x64
repository-vX.Y.Z-macos-x64
repository-vX.Y.Z-macos-arm64
repository-vX.Y.Z-windows-x64.exe
```

Checksum files use the same name with `.sha256` appended. The composite action downloads the asset that matches the current runner OS and architecture.

### Operational notes

`repository` preserves existing project-local formatter and linter config files unless `--force` is passed. This keeps central defaults useful without replacing deliberate project decisions.

Generated release workflows are only written for targets with `workflow = "managed"` and `strategy` set to `same-repo` or `distribution-repo`. `self-hosted` and `custom` targets stay under project-owned workflow files.

The action passes `args` through the shell so workflow authors can provide normal CLI strings. Do not pass untrusted user input into `args`.

## License

AGPL-3.0-only.
