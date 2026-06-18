# verzly/toolchain

`verzly/toolchain` is the single source repository and release surface for Verzly release automation, Rust executable packaging, Tauri artifact builds, repository standards, cache routing, and mobile signing preflight checks.

It provides a compact toolchain surface with built-in support for:

- **one executable**: `verzly`
- **one repository-hosted action surface**: `verzly/toolchain` and `verzly/toolchain/actions/*`
- **GitHub release orchestration** through `github-release`
- **Rust executable packaging** through `cargo-release`
- **Tauri desktop and mobile artifact builds** through `tauri-release`
- **project-local build/cache routing** through `rust-cache`
- **Android and iOS signing preflight checks** through `android-signing` and `ios-signing`
- **repository structure and workflow checks** through `repository`

Use it from CI as a GitHub Action, or locally as a Rust workspace executable. Consuming repositories should depend on `verzly/toolchain` directly instead of wiring multiple `verzly/<tool>` distribution repositories together.

- [How it works](#how-it-works)
  - [Single repository](#single-repository)
  - [Unified executable](#unified-executable)
  - [Action-first surface](#action-first-surface)
  - [Compatibility commands](#compatibility-commands)
  - [Cache and generated output](#cache-and-generated-output)
- [Get started](#get-started)
  - [Use in GitHub Actions](#use-in-github-actions)
  - [Run one command](#run-one-command)
  - [Install once and run multiple commands](#install-once-and-run-multiple-commands)
  - [Pin a toolchain version](#pin-a-toolchain-version)
  - [Build locally](#build-locally)
- [Usage](#usage)
  - [CLI overview](#cli-overview)
  - [Root action](#root-action)
  - [Tool actions](#tool-actions)
  - [repository](#repository)
  - [datarose.toml schema](#datarosetoml-schema)
  - [github-release](#github-release)
  - [cargo-release](#cargo-release)
  - [tauri-release](#tauri-release)
  - [rust-cache](#rust-cache)
  - [android-signing](#android-signing)
  - [ios-signing](#ios-signing)
  - [Example app release workflow](#example-app-release-workflow)
- [Release management](#release-management)
  - [Toolchain release flow](#toolchain-release-flow)
  - [Release assets](#release-assets)
  - [Tokens and permissions](#tokens-and-permissions)
  - [Delete release](#delete-release)
  - [Floating tag maintenance](#floating-tag-maintenance)
- [Development](#development)
  - [Repository layout](#repository-layout)
  - [Quality checks](#quality-checks)
  - [Implementation boundaries](#implementation-boundaries)
  - [Action quality rules](#action-quality-rules)
  - [Contribution rules](#contribution-rules)
- [Debugging](#debugging)
  - [Repository check failures](#repository-check-failures)
  - [Release failures](#release-failures)
  - [Signing failures](#signing-failures)
  - [Cache issues](#cache-issues)

Read on to learn how the toolchain is structured and how each tool is intended to be used. Or jump straight to [Get started](#get-started) for GitHub Actions usage, or to [Release management](#release-management) if you are maintaining `verzly/toolchain` itself.

## How it works

`verzly/toolchain` keeps the source, GitHub Actions, release workflow, and published binary in one repository. The public contract is intentionally small: install `verzly`, then call the tool you need as a subcommand.

### Single repository

The toolchain no longer uses a `.verzly/distributions/*` model. There are no separate public distribution repositories to sync, release, or configure in downstream projects.

The repository itself is the release surface:

```text
verzly/toolchain
├── action.yml
├── actions/*/action.yml
├── crates/verzly
├── crates/*
└── .github/workflows/release.yml
```

Downstream projects such as Tauri apps should consume this repository directly:

```yaml
- uses: verzly/toolchain@v1
```

or use a focused tool action:

```yaml
- uses: verzly/toolchain/actions/tauri-release@v1
  with:
    args: build --config .github/release/app.tauri-release.toml --platform linux
```

### Unified executable

All public tools are available through one executable:

```sh
verzly github-release --help
verzly cargo-release --help
verzly tauri-release --help
verzly rust-cache --help
verzly android-signing --help
verzly ios-signing --help
verzly repository --help
```

Common aliases are available for shorter local usage:

```sh
verzly repo check
verzly cache env
verzly tauri build --platform linux
verzly android check-env
verzly ios check-env
```

### Action-first surface

The root action installs `verzly` and can optionally run a command. Tool-specific actions install the same binary and run one tool with a cleaner workflow surface.

```text
action.yml
actions/github-release/action.yml
actions/cargo-release/action.yml
actions/tauri-release/action.yml
actions/rust-cache/action.yml
actions/android-signing/action.yml
actions/ios-signing/action.yml
actions/repository/action.yml
```

The actions download the matching `verzly` release asset, add it to `PATH`, and optionally create compatibility shims such as `github-release`, `tauri-release`, and `ios-signing`.

### Compatibility commands

Standalone command names remain available for migration compatibility:

```sh
github-release --help
cargo-release --help
tauri-release --help
rust-cache --help
android-signing --help
ios-signing --help
repository --help
```

These commands are compatibility entrypoints. Internally they delegate to the same Rust command logic that powers `verzly <tool>`.

### Cache and generated output

The toolchain is designed to keep build output outside the source tree. In this repository, Cargo output is routed to:

```text
.cache/rust/packages/toolchain/target
```

Downstream projects can use `rust-cache` to route Cargo, Gradle, package manager caches, and configured generated output under `.cache`.

```sh
verzly rust-cache env
verzly rust-cache run -- cargo build --workspace
verzly rust-cache clean-generated --dry-run
```

## Get started

Use `verzly/toolchain` from GitHub Actions for released toolchain usage. Build from source only when developing this repository.

### Use in GitHub Actions

Install the latest released `verzly` binary:

```yaml
- uses: verzly/toolchain@v1
```

This adds `verzly` to `PATH`. By default, the action also creates compatibility shims for the previous standalone command names.

### Run one command

Use the root action when a job only needs one command:

```yaml
- uses: verzly/toolchain@v1
  with:
    command: verzly repository check
```

### Install once and run multiple commands

Use the root action once, then call `verzly` normally:

```yaml
- uses: verzly/toolchain@v1

- name: Export cache environment
  run: verzly rust-cache env >> "$GITHUB_ENV"

- name: Check repository configuration
  run: verzly repository check

- name: Preview release
  run: verzly github-release plan --config datarose.toml --release-target app --version 1.2.3
```

### Pin a toolchain version

Use a fixed release version for reproducible release workflows:

```yaml
- uses: verzly/toolchain@v1
  with:
    version: v1.2.3
```

Accepted version values:

```text
latest
v1.2.3
1.2.3
```

### Build locally

Build the unified executable from the workspace:

```sh
cargo build --release -p verzly
.cache/rust/packages/toolchain/target/release/verzly --help
```

Run from source without installing:

```sh
cargo run -p verzly -- repository check
cargo run -p verzly -- github-release plan --config datarose.toml --release-target verzly --version 1.2.3
```

## Usage

Each tool is available through the unified executable and through a matching GitHub Action. Prefer the `verzly <tool>` form for new scripts.

### CLI overview

```sh
verzly repository check
verzly github-release prepare --config datarose.toml --release-target app --version 1.2.3
verzly cargo-release build --config datarose.toml --release-target verzly --version 1.2.3 --target linux-x64
verzly tauri-release build --config .github/release/app.tauri-release.toml --platform linux
verzly rust-cache env
verzly android-signing check-env
verzly ios-signing check-env
```

### Root action

Use the root action when a workflow wants to install the toolchain or run arbitrary `verzly` commands.

```yaml
- uses: verzly/toolchain@v1
  with:
    command: verzly repository check
```

Inputs:

| Input | Default | Description |
|---|---|---|
| `version` | `latest` | Verzly release version to install. |
| `repository` | `verzly/toolchain` | Repository that publishes the `verzly` release assets. |
| `github-token` | `github.token` | Token used to download release assets. |
| `command` | empty | Optional command to run after installing Verzly. |
| `working-directory` | `.` | Directory used when running the optional command. |
| `install-dir` | runner temp | Directory where the executable and shims are installed. |
| `create-shims` | `true` | Create compatibility command names next to `verzly`. |

Outputs:

| Output | Description |
|---|---|
| `path` | Absolute path to the installed `verzly` executable. |
| `install-dir` | Directory added to `PATH`. |
| `version` | Installed Verzly version. |
| `target` | Resolved release asset target for the current runner. |

### Tool actions

Use tool actions when the workflow should read naturally and only needs one tool.

```yaml
- uses: verzly/toolchain/actions/repository@v1
  with:
    args: check

- uses: verzly/toolchain/actions/github-release@v1
  with:
    args: plan --config datarose.toml --release-target app --version 1.2.3

- uses: verzly/toolchain/actions/tauri-release@v1
  with:
    args: build --config .github/release/app.tauri-release.toml --platform linux
```

Common inputs for tool actions:

| Input | Default | Description |
|---|---|---|
| `version` | `latest` | Verzly release version to install. |
| `repository` | `verzly/toolchain` | Repository that publishes the release assets. |
| `github-token` | `github.token` | Token used to download assets and exposed as `GH_TOKEN` for GitHub-aware commands. |
| `install-only` | `false` | Install Verzly without running the selected tool. |
| `args` | `--help` | Arguments passed after `verzly <tool>`. |
| `working-directory` | `.` | Directory where the command runs. |
| `install-dir` | runner temp | Directory where the executable and shims are installed. |
| `create-shims` | `true` | Create compatibility command names next to `verzly`. |

Signing actions have additional inputs and outputs documented in [android-signing](#android-signing) and [ios-signing](#ios-signing).

### repository

`repository` manages repository standards: `datarose.toml`, `hk.pkl`, quality workflows, release target metadata, project inventory, and expected action surfaces.

```sh
verzly repository init
verzly repository init --language rust --language js --js-runner pnpm
verzly repository update
verzly repository plan
verzly repository projects
verzly repository check
verzly repository doctor
verzly repository tui
```

Manage release targets:

```sh
verzly repository release list
verzly repository release show app
verzly repository release set \
  --name app \
  --path . \
  --repository owner/app \
  --strategy same-repo \
  --workflow managed \
  --source-kind tauri-app
verzly repository release remove app --yes
```

GitHub Action:

```yaml
- uses: verzly/toolchain/actions/repository@v1
  with:
    args: check
```

### datarose.toml schema

`datarose.toml` stays TOML. There is no `datarose.json` runtime configuration.

Each `datarose.toml` should be self-describing and point editors to the public schema directly from the file header:

```toml
"$schema" = "https://raw.githubusercontent.com/verzly/toolchain/master/schemas/datarose.toml.schema.json"
version = 1
```

No `.taplo.toml`, `.vscode/settings.json`, or repository-local schema mapping is required. The schema URL lives directly in `datarose.toml` as a valid TOML `"$schema"` key, while `verzly repository check` validates the same configuration offline through the executable.

`repository check` validates the TOML file before relying on its values. It reports a missing or wrong `$schema` value, unknown sections, unknown keys, wrong value types, unsupported enum values, invalid arrays, and required release fields that are missing. This prevents typos from being silently ignored in workflow or release configuration.

```sh
verzly repository check
```

Examples of errors the validator catches:

```toml
[quality]
langauges = ["rust"] # typo: should be languages

[release]
manage_workflowz = true # typo: should be manage_workflows

[[release.targets]]
nam = "app" # typo: should be name
```

The runtime validator is implemented in Rust and parses the actual TOML document, so CI does not depend on network access to the schema URL. The public `schemas/datarose.toml.schema.json` file exists for editor integrations and review tooling; it is not a replacement config format and must not be used as `datarose.json`. When a supported config key changes, update the Rust validator and the public schema together.

Use `repository` first in downstream projects. It should describe the project layout, release targets, quality rules, cache conventions, and generated workflow expectations before release tooling is wired in.

### github-release

`github-release` prepares, finalizes, publishes, deletes, and aborts GitHub releases. It can update configured version files, create release branches, merge prepared source, tag releases, upload assets, and update floating tags.

```sh
verzly github-release init
verzly github-release plan --version 1.2.3 --config datarose.toml --release-target app
verzly github-release prepare --version 1.2.3 --config datarose.toml --release-target app
verzly github-release finalize --version 1.2.3 --config datarose.toml --release-target app --assets dist/release
verzly github-release publish --version 1.2.3 --config datarose.toml --release-target app --assets dist/release
verzly github-release delete --version 1.2.3 --config datarose.toml --release-target app
verzly github-release abort --version 1.2.3 --config datarose.toml --release-target app
```

Configured floating tags are updated as part of finalization:

```sh
verzly github-release finalize \
  --version 1.2.3 \
  --config datarose.toml \
  --release-target app \
  --assets dist/release
```

The explicit `--update-floating-tags`, `--update-latest-tag`, and `--update-next-tag` flags are escape hatches for one-off overrides. Normal workflows should let `datarose.toml` decide which tag families are managed.

Manage floating tags directly when needed:

```sh
verzly github-release update-floating-tags --config datarose.toml --release-target app --all --prune
```

GitHub Action:

```yaml
- uses: verzly/toolchain/actions/github-release@v1
  with:
    args: prepare --version 1.2.3 --config datarose.toml --release-target app
```

`github-release` expects `GH_TOKEN` or `GITHUB_TOKEN` in CI for GitHub CLI operations. The toolchain release workflow uses `github.token`; no separate distribution repository token is required.

### cargo-release

`cargo-release` builds Rust executable release assets for configured targets and writes checksums/manifests next to the generated artifacts.

```sh
verzly cargo-release init
verzly cargo-release plan --config datarose.toml --release-target verzly
verzly cargo-release build --config datarose.toml --release-target verzly --version 1.2.3
verzly cargo-release build --config datarose.toml --release-target verzly --version 1.2.3 --target linux-x64
verzly cargo-release build --config datarose.toml --release-target verzly --version 1.2.3 --target windows-x64
verzly cargo-release clean --config datarose.toml --release-target verzly
verzly cargo-release doctor --config datarose.toml --release-target verzly
```

GitHub Action:

```yaml
- uses: verzly/toolchain/actions/cargo-release@v1
  with:
    args: build --config datarose.toml --release-target verzly --version 1.2.3 --target linux-x64
```

Use this for toolchain-style Rust binaries. Use `tauri-release` for Tauri desktop/mobile app artifacts.

### tauri-release

`tauri-release` builds Tauri desktop and mobile release artifacts from declarative platform configuration.

```sh
verzly tauri-release init
verzly tauri-release plan --config .github/release/app.tauri-release.toml
verzly tauri-release build --config .github/release/app.tauri-release.toml
verzly tauri-release build --config .github/release/app.tauri-release.toml --platform linux
verzly tauri-release build --config .github/release/app.tauri-release.toml --platform macos
verzly tauri-release build --config .github/release/app.tauri-release.toml --platform windows
verzly tauri-release build --config .github/release/app.tauri-release.toml --platform android
verzly tauri-release build --config .github/release/app.tauri-release.toml --platform ios
verzly tauri-release clean --config .github/release/app.tauri-release.toml
verzly tauri-release doctor --config .github/release/app.tauri-release.toml
```

GitHub Action:

```yaml
- uses: verzly/toolchain/actions/tauri-release@v1
  with:
    args: build --config .github/release/app.tauri-release.toml --platform linux
```

Mobile release workflows should gate Android and iOS builds behind signing-action outputs so missing signing secrets skip mobile artifacts instead of failing the whole release.

### rust-cache

`rust-cache` routes generated and build output into a project-local `.cache` tree. It covers Cargo target output, optional Cargo home routing, Gradle cache/build routing, package-manager caches, and configured generated paths.

```sh
verzly rust-cache init
verzly rust-cache env
verzly rust-cache run -- cargo test --workspace
verzly rust-cache run -- pnpm install
verzly rust-cache clean
verzly rust-cache clean-generated
verzly rust-cache clean-generated --dry-run
verzly rust-cache doctor
```

GitHub Action:

```yaml
- uses: verzly/toolchain/actions/rust-cache@v1
  with:
    args: env
```

Export cache variables in workflows:

```yaml
- name: Export Verzly cache environment
  run: verzly rust-cache env >> "$GITHUB_ENV"
```

Example config shape:

```toml
[rust_cache.cache]
dir = ".cache"
package = "app"
redirect_cargo_home = false
redirect_gradle = true

[rust_cache.cargo]
target_dir = "rust/packages/{package}/target"

[rust_cache.env]
GRADLE_USER_HOME = "android/gradle"
PNPM_STORE_PATH = "js/pnpm-store"
```

### android-signing

`android-signing` helps create, encode, inspect, and validate Android release signing material. CI can use it to detect whether Android signing is configured and skip Android artifacts when secrets are intentionally missing.

```sh
verzly android-signing doctor
verzly android-signing generate --output android-release.jks --alias release-key --generate-passwords
verzly android-signing base64 android-release.jks
verzly android-signing fingerprint android-release.jks --alias release-key
verzly android-signing verify-fingerprint android-release.jks --alias release-key --expected-sha256 AA:BB:CC
verzly android-signing print-secrets android-release.jks --alias release-key
verzly android-signing write-github-env android-release.jks --alias release-key
verzly android-signing check-env
verzly android-signing check-env --require-fingerprint --require ANDROID_KEYSTORE_PATH
```

Standard CI variables:

```text
ANDROID_KEYSTORE_BASE64
ANDROID_KEYSTORE_PASSWORD
ANDROID_KEY_ALIAS
ANDROID_KEY_PASSWORD
```

Optional fingerprint variable:

```text
ANDROID_SIGNING_CERT_SHA256
```

GitHub Action:

```yaml
- id: android-signing
  uses: verzly/toolchain/actions/android-signing@v1
  with:
    optional: "true"
    run-doctor: "true"
    check-env: "true"
  env:
    ANDROID_KEYSTORE_BASE64: ${{ secrets.ANDROID_KEYSTORE_BASE64 }}
    ANDROID_KEYSTORE_PASSWORD: ${{ secrets.ANDROID_KEYSTORE_PASSWORD }}
    ANDROID_KEY_ALIAS: ${{ secrets.ANDROID_KEY_ALIAS }}
    ANDROID_KEY_PASSWORD: ${{ secrets.ANDROID_KEY_PASSWORD }}

- name: Build Android artifact
  if: ${{ steps.android-signing.outputs.signing-ready == 'true' }}
  run: verzly tauri-release build --config .github/release/app.tauri-release.toml --platform android
```

Action-specific inputs:

| Input | Default | Description |
|---|---|---|
| `optional` | `true` | Missing signing variables produce outputs and notices instead of failing. |
| `run-doctor` | `true` | Run `verzly android-signing doctor` before checking secrets. |
| `check-env` | `true` | Validate required signing environment variables. |
| `require-fingerprint` | `false` | Also require `ANDROID_SIGNING_CERT_SHA256`. |
| `extra-required-env` | empty | Additional env names to require; supports newline, comma, or space separated values. |
| `args` | empty | Optional additional `android-signing` command to run after preflight. |

Action-specific outputs:

| Output | Description |
|---|---|
| `signing-ready` | `true`, `false`, or `unknown`. |
| `missing-secrets` | Comma-separated missing variable names. |

### ios-signing

`ios-signing` validates iOS signing environments and helps encode existing Apple signing files for GitHub Actions secrets. It supports optional preflight behavior so projects can skip iOS artifacts when signing is not configured.

```sh
verzly ios-signing doctor
verzly ios-signing base64 ios-release.p12
verzly ios-signing print-secrets --certificate ios-release.p12 --provisioning-profile app.mobileprovision
verzly ios-signing write-github-env --certificate ios-release.p12 --provisioning-profile app.mobileprovision
verzly ios-signing check-env
verzly ios-signing check-env --skip-apple-team-id
verzly ios-signing check-env --require APPLE_ID
```

Standard CI variables:

```text
IOS_SIGNING_CERTIFICATE_BASE64
IOS_SIGNING_CERTIFICATE_PASSWORD
IOS_SIGNING_PROVISIONING_PROFILE_BASE64
IOS_SIGNING_KEYCHAIN_PASSWORD
APPLE_TEAM_ID
```

GitHub Action:

```yaml
- id: ios-signing
  uses: verzly/toolchain/actions/ios-signing@v1
  with:
    optional: "true"
    run-doctor: "true"
    check-env: "true"
  env:
    IOS_SIGNING_CERTIFICATE_BASE64: ${{ secrets.IOS_SIGNING_CERTIFICATE_BASE64 }}
    IOS_SIGNING_CERTIFICATE_PASSWORD: ${{ secrets.IOS_SIGNING_CERTIFICATE_PASSWORD }}
    IOS_SIGNING_PROVISIONING_PROFILE_BASE64: ${{ secrets.IOS_SIGNING_PROVISIONING_PROFILE_BASE64 }}
    IOS_SIGNING_KEYCHAIN_PASSWORD: ${{ secrets.IOS_SIGNING_KEYCHAIN_PASSWORD }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}

- name: Build iOS artifact
  if: ${{ steps.ios-signing.outputs.signing-ready == 'true' }}
  run: verzly tauri-release build --config .github/release/app.tauri-release.toml --platform ios
```

Action-specific inputs:

| Input | Default | Description |
|---|---|---|
| `optional` | `true` | Missing signing variables produce outputs and notices instead of failing. |
| `run-doctor` | `true` | Run `verzly ios-signing doctor` before checking secrets. |
| `check-env` | `true` | Validate required signing environment variables. |
| `skip-apple-team-id` | `false` | Do not require `APPLE_TEAM_ID`. |
| `extra-required-env` | empty | Additional env names to require; supports newline, comma, or space separated values. |
| `args` | empty | Optional additional `ios-signing` command to run after preflight. |
| `command` | empty | Deprecated alias for `args`. |

Action-specific outputs:

| Output | Description |
|---|---|
| `signing-ready` | `true`, `false`, or `unknown`. |
| `missing-secrets` | Comma-separated missing variable names. |

### Example app release workflow

This is the intended downstream shape for a Tauri app that publishes desktop artifacts and only publishes mobile artifacts when signing is configured.

```yaml
name: Release

on:
  workflow_dispatch:
    inputs:
      version:
        description: Version to release
        required: true

permissions:
  contents: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0

      - uses: verzly/toolchain@v1

      - name: Export cache environment
        run: verzly rust-cache env >> "$GITHUB_ENV"

      - name: Check repository configuration
        run: verzly repository check

      - name: Prepare release
        run: verzly github-release prepare --version "${{ inputs.version }}" --config datarose.toml --release-target app

      - id: android-signing
        uses: verzly/toolchain/actions/android-signing@v1
        with:
          optional: "true"
        env:
          ANDROID_KEYSTORE_BASE64: ${{ secrets.ANDROID_KEYSTORE_BASE64 }}
          ANDROID_KEYSTORE_PASSWORD: ${{ secrets.ANDROID_KEYSTORE_PASSWORD }}
          ANDROID_KEY_ALIAS: ${{ secrets.ANDROID_KEY_ALIAS }}
          ANDROID_KEY_PASSWORD: ${{ secrets.ANDROID_KEY_PASSWORD }}

      - id: ios-signing
        uses: verzly/toolchain/actions/ios-signing@v1
        with:
          optional: "true"
        env:
          IOS_SIGNING_CERTIFICATE_BASE64: ${{ secrets.IOS_SIGNING_CERTIFICATE_BASE64 }}
          IOS_SIGNING_CERTIFICATE_PASSWORD: ${{ secrets.IOS_SIGNING_CERTIFICATE_PASSWORD }}
          IOS_SIGNING_PROVISIONING_PROFILE_BASE64: ${{ secrets.IOS_SIGNING_PROVISIONING_PROFILE_BASE64 }}
          IOS_SIGNING_KEYCHAIN_PASSWORD: ${{ secrets.IOS_SIGNING_KEYCHAIN_PASSWORD }}
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}

      - name: Build desktop artifacts
        run: verzly tauri-release build --config .github/release/app.tauri-release.toml --platform linux

      - name: Build Android artifacts
        if: ${{ steps.android-signing.outputs.signing-ready == 'true' }}
        run: verzly tauri-release build --config .github/release/app.tauri-release.toml --platform android

      - name: Build iOS artifacts
        if: ${{ steps.ios-signing.outputs.signing-ready == 'true' }}
        run: verzly tauri-release build --config .github/release/app.tauri-release.toml --platform ios

      - name: Finalize release
        run: verzly github-release finalize --version "${{ inputs.version }}" --config datarose.toml --release-target app --assets dist/release
```

## Release management

This section describes how `verzly/toolchain` releases itself. Consuming projects normally only need the usage examples above.

### Toolchain release flow

The visible workflow surface is intentionally small:

```text
.github/workflows/release.yml                 Publishes the Verzly toolchain release.
.github/workflows/delete-release.yml          Deletes a published release and its tags after confirmation.
.github/workflows/update-floating-tags.yml    Reconciles moving tags after manual tag edits.
.github/workflows/test.yml                    Runs pull request quality checks.
```

The release workflow:

1. validates that the workflow is running from `master`,
2. checks that the target release/tag does not already exist,
3. prepares a release branch through `verzly github-release prepare`,
4. updates workspace crate versions and matching `Cargo.lock` package entries,
5. runs formatting, Clippy, and tests,
6. builds release assets through `verzly cargo-release build`,
7. finalizes the release in `verzly/toolchain`, uploads assets, and updates floating tags through `verzly github-release finalize`.

### Release assets

The toolchain publishes one executable, `verzly`, for these targets:

```text
linux-x64
macos-x64
macos-arm64
windows-x64
```

The configured release target is `verzly` in `datarose.toml`:

```toml
[[release.targets]]
name = "verzly"
repository = "verzly/toolchain"
cargo_binary = "verzly"
cargo_package = "verzly"
cargo_targets = ["linux-x64", "macos-x64", "macos-arm64", "windows-x64"]
floating_tags = true
latest_tag = true
next_tag = true
```

### Tokens and permissions

The main release path writes only to `verzly/toolchain` through `github.token`.

It must not require:

```text
DISTRIBUTION_REPO_TOKEN
separate distribution repository tokens
separate PATs for the normal release path
```

Floating tags are handled by `github-release finalize` during normal releases according to the release target settings in `datarose.toml`:

```sh
verzly github-release finalize \
  --config datarose.toml \
  --release-target verzly \
  --version 1.2.3 \
  --assets dist/verzly
```

Manual maintenance is handled by `github-release update-floating-tags`:

```sh
verzly github-release update-floating-tags \
  --config datarose.toml \
  --release-target verzly \
  --all \
  --prune
```

Tag maintenance follows the configured release target. If `next_tag = false`, the `next` tag is not created, updated, deleted, or pruned by finalize, delete, or maintenance runs. The same rule applies to `latest_tag = false` and `floating_tags = false`. To manage a disabled tag family, enable it in `datarose.toml` first.

### Delete release

Use the `Delete Release` workflow when a release must be removed after confirmation. The underlying command is:

```sh
verzly github-release delete \
  --config datarose.toml \
  --release-target verzly \
  --version 1.2.3
```

The delete command removes the configured release/tag surface and repairs configured floating tags from the remaining release tags.

### Floating tag maintenance

`github-release` owns moving tag reconciliation. Normal releases update floating tags from `finalize`, and `Delete Release` repairs them after deleting a version.

Use `Update Floating Tags` when tags were changed outside the normal release workflow, for example after manually deleting `v1.2.3` from GitHub or pushing a historical tag. The workflow runs the same command locally:

```sh
verzly github-release update-floating-tags \
  --config datarose.toml \
  --release-target verzly \
  --all \
  --prune
```

For a single newly added tag, use either the version or full tag form:

```sh
verzly github-release update-floating-tags --config datarose.toml --release-target verzly --version 1.2.3
verzly github-release update-floating-tags --config datarose.toml --release-target verzly --tag v1.2.3
```

`--all --prune` recalculates `vX`, `vX.Y`, `latest`, and `next` from the release tags that still exist, and removes stale moving tags that no longer have a matching release line.

## Development

Source changes, action surfaces, documentation, and releases all live in this repository.

### Repository layout

```text
.github/workflows/        Quality and release workflows
actions/                  Public GitHub Actions stored in this repo
actions/_shared/          Shared composite-action installer scripts
crates/verzly/            Unified executable entrypoint
crates/*/                 Modular tool implementations
action.yml                Root setup/run action
Cargo.toml                Rust workspace
datarose.toml             Release, quality, cache, and build configuration
hk.pkl                    Git hook and quality gate configuration
mise.toml                 Local tool/task configuration
schemas/                 Public online schema for TOML-backed config
```

### Quality checks

Run workspace checks from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Run selected commands from source:

```sh
cargo run -p verzly -- repository check
cargo run -p verzly -- github-release plan --config datarose.toml --release-target verzly --version 1.2.3
cargo run -p verzly -- cargo-release build --config datarose.toml --release-target verzly --version 1.2.3 --target linux-x64 --dry-run
cargo run -p verzly -- ios-signing check-env --skip-apple-team-id
```

### Implementation boundaries

Keep the boundary between executable, tools, and actions clear:

- `crates/verzly` is the unified executable and should mostly dispatch.
- Each tool crate owns its CLI contract, command logic, tests, and reusable `run_from` entrypoint.
- Standalone binaries remain compatibility wrappers.
- GitHub Actions should call the released `verzly` binary, not duplicate tool logic.
- Repository standards should be enforced through `repository check`, not copied into ad hoc scripts.

### Action quality rules

- Do not print secret values.
- Prefer explicit inputs and documented outputs.
- Signing checks should support optional mode so release workflows can skip unavailable mobile artifacts.
- Use `actions/_shared/install-verzly.sh` for installing released assets.
- Keep action examples copy-pasteable for downstream repositories.
- Keep tool-specific actions thin; command behavior belongs in Rust.

### Contribution rules

Before opening a PR, run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Do not reintroduce:

```text
.verzly/distributions
release workflows that publish to separate verzly/<tool> repositories
distribution sync workflows
DISTRIBUTION_REPO_TOKEN requirements for the normal release path
```

Public usage must go through `verzly/toolchain`, either with the root action, subpath actions under `actions/`, or the `verzly` release assets.

## Debugging

Start with the command closest to the failing surface: `repository check` for repository structure, `github-release plan` for release configuration, `cargo-release doctor` for Rust binary assets, `tauri-release doctor` for Tauri builds, and signing `check-env` commands for mobile secrets.

### Repository check failures

Run:

```sh
verzly repository check
verzly repository doctor
```

If a workflow was intentionally removed, update the repository expectations in the `repository` crate and `datarose.toml` together. The check should reflect the current public surface, not old migration leftovers.

### Release failures

Preview the release before preparing it:

```sh
verzly github-release plan --config datarose.toml --release-target verzly --version 1.2.3
```

Check whether the release/tag already exists:

```sh
gh release view v1.2.3 --repo verzly/toolchain
git ls-remote --tags https://github.com/verzly/toolchain.git refs/tags/v1.2.3
```

Abort a prepared release branch if build or test jobs fail:

```sh
verzly github-release abort --config datarose.toml --release-target verzly --version 1.2.3
```

### Signing failures

Check Android signing:

```sh
verzly android-signing doctor
verzly android-signing check-env
verzly android-signing check-env --require-fingerprint
```

Check iOS signing:

```sh
verzly ios-signing doctor
verzly ios-signing check-env
verzly ios-signing check-env --skip-apple-team-id
```

Use `optional: "true"` in app workflows when mobile signing is not required for every release.

### Cache issues

Print the resolved environment:

```sh
verzly rust-cache env
```

Run a command through the cache wrapper:

```sh
verzly rust-cache run -- cargo build --workspace
```

Preview generated cleanup before deleting anything:

```sh
verzly rust-cache clean-generated --dry-run
```

## License & Acknowledgments

This project would not exist without the Rust, Tauri, GitHub Actions, and open-source release tooling ecosystems.

It is open source and released under the [GNU Affero General Public License v3.0 (AGPL-3.0)](LICENSE). We are grateful to the maintainers and contributors of Rust, Tauri, Cargo, GitHub CLI, and GitHub Actions for the tooling foundations that make this project possible.

Copyright (C) 2020–present [Zoltán Rózsa](https://github.com/rozsazoltan) & [Verzly](https://github.com/verzly)
