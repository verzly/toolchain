# Verzly Toolchain

`verzly/toolchain` is the single source repository and release surface for the Verzly release, cache, signing, and repository-maintenance tooling.

The public contract is intentionally small: one repository, one released executable named `verzly`, and first-class GitHub Actions stored in this same repository. Projects such as Nutrino should consume `verzly/toolchain` directly instead of wiring several separate `verzly/<tool>` distribution repositories together.

## What it provides

- `verzly`, the unified executable.
- Subcommands for every public tool: `github-release`, `cargo-release`, `tauri-release`, `rust-cache`, `android-signing`, `ios-signing`, and `repository`.
- Compatibility command names for existing scripts: `github-release`, `cargo-release`, `tauri-release`, `rust-cache`, `android-signing`, `ios-signing`, and `repository`.
- A root GitHub Action at `verzly/toolchain@vX` for installing the toolchain and optionally running a command.
- Tool-specific GitHub Actions under `verzly/toolchain/actions/<tool>@vX`.

There is no `.verzly/distributions/*` release model. There are no separate public distribution repositories to sync, release, or configure in downstream projects.

## Install locally

Build from source while developing the toolchain:

```sh
cargo build --release -p verzly
.cache/rust/packages/toolchain/target/release/verzly --help
```

Run from source without installing:

```sh
cargo run -p verzly -- repository check
cargo run -p verzly -- github-release plan --config datarose.toml --release-target verzly --version 1.2.3
```

Cargo output is routed by `.cargo/config.toml` into `.cache/rust/packages/toolchain/target` so build artifacts stay outside the repository source tree.

## Install in GitHub Actions

Use the root action when a workflow needs the toolchain installed once:

```yaml
- uses: verzly/toolchain@v1
```

Install and run a single command:

```yaml
- uses: verzly/toolchain@v1
  with:
    command: verzly repository check
```

Install once, then run multiple commands:

```yaml
- uses: verzly/toolchain@v1

- run: verzly rust-cache env >> "$GITHUB_ENV"
- run: verzly repository check
- run: verzly github-release plan --config datarose.toml --release-target app --version 1.2.3
```

Use a specific toolchain version when release workflows must be reproducible:

```yaml
- uses: verzly/toolchain@v1
  with:
    version: v1.2.3
```

The action downloads the matching `verzly` asset from GitHub Releases and adds it to `PATH`. By default it also creates compatibility shims such as `github-release`, `tauri-release`, and `ios-signing`.

## Unified CLI

All tools are available through the `verzly` executable:

```sh
verzly github-release --help
verzly cargo-release --help
verzly tauri-release --help
verzly rust-cache --help
verzly android-signing --help
verzly ios-signing --help
verzly repository --help
```

Short aliases are available for common use:

```sh
verzly repo check
verzly cache env
verzly tauri build --platform linux
verzly android check-env
verzly ios check-env
```

Existing standalone command names remain supported as compatibility entrypoints. They delegate to the same Rust library entrypoints used by `verzly`, so projects can migrate gradually.

## GitHub Actions

Every public tool has a matching action:

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

The common action inputs are:

| Input | Default | Description |
| --- | --- | --- |
| `version` | `latest` | Verzly release version to install. Accepts `latest`, `v1.2.3`, or `1.2.3`. |
| `repository` | `verzly/toolchain` | Repository that publishes the `verzly` release assets. |
| `github-token` | `github.token` | Token used to download release assets and, for release commands, call GitHub. |
| `install-only` | `false` | Install Verzly without running the tool command. |
| `args` | `--help` | Arguments passed after the selected tool name. |
| `working-directory` | `.` | Directory where the command runs. |
| `install-dir` | runner temp | Directory where the binary and shims are installed. |
| `create-shims` | `true` | Create compatibility commands next to `verzly`. |

The common action outputs are:

| Output | Description |
| --- | --- |
| `path` | Absolute path to the installed `verzly` executable. |
| `install-dir` | Directory added to `PATH`. |
| `version` | Installed Verzly version. |
| `target` | Resolved release asset target for the current runner. |

Use tool-specific actions when readability matters:

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

## repository

`repository` manages repository standards: `datarose.toml`, `hk.pkl`, quality workflows, release target metadata, and project inventory checks.

CLI usage:

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

GitHub Action usage:

```yaml
- uses: verzly/toolchain/actions/repository@v1
  with:
    args: check
```

Use `repository` first in downstream projects. It should describe the project layout, release targets, quality rules, cache conventions, and generated workflow expectations before release tooling is wired in.

## github-release

`github-release` prepares and finalizes GitHub releases. It updates configured version files, creates release branches, merges release branches, tags source, publishes GitHub Releases, uploads assets, updates floating tags, and can abort failed release branches.

CLI usage:

```sh
verzly github-release init
verzly github-release plan --version 1.2.3 --config datarose.toml --release-target app
verzly github-release prepare --version 1.2.3 --config datarose.toml --release-target app
verzly github-release finalize --version 1.2.3 --config datarose.toml --release-target app --assets dist/release
verzly github-release publish --version 1.2.3 --config datarose.toml --release-target app --assets dist/release
verzly github-release floating-tags --config datarose.toml --release-target app --all --prune
verzly github-release delete --version 1.2.3 --config datarose.toml --release-target app
verzly github-release abort --version 1.2.3 --config datarose.toml --release-target app
```

Batch finalization for a single aggregate release branch with multiple source tags:

```sh
verzly github-release finalize-batch \
  --version 1.2.3 \
  --target-branch master \
  --release-branch release/all-v1.2.3 \
  --source-tag app-v1.2.3 \
  --source-tag cli-v1.2.3
```

GitHub Action usage:

```yaml
- uses: verzly/toolchain/actions/github-release@v1
  with:
    args: prepare --version 1.2.3 --config datarose.toml --release-target app
```

`github-release` expects GitHub CLI authentication through `GH_TOKEN`/`GITHUB_TOKEN` in CI. Release workflows in this repository use `github.token`; no separate distribution repository token is required.

## cargo-release

`cargo-release` builds Rust executable release assets for configured targets and writes checksums/manifests next to the generated artifacts.

CLI usage:

```sh
verzly cargo-release init
verzly cargo-release plan --config datarose.toml --release-target verzly
verzly cargo-release build --config datarose.toml --release-target verzly --version 1.2.3
verzly cargo-release build --config datarose.toml --release-target verzly --version 1.2.3 --target linux-x64
verzly cargo-release clean --config datarose.toml --release-target verzly
verzly cargo-release doctor --config datarose.toml --release-target verzly
```

GitHub Action usage:

```yaml
- uses: verzly/toolchain/actions/cargo-release@v1
  with:
    args: build --config datarose.toml --release-target verzly --version 1.2.3 --target linux-x64
```

Use this for toolchain-style Rust binaries. Use `tauri-release` for Tauri desktop/mobile app artifacts.

## tauri-release

`tauri-release` builds Tauri desktop and mobile release artifacts from declarative platform configuration.

CLI usage:

```sh
verzly tauri-release init
verzly tauri-release plan --config .github/release/app.tauri-release.toml
verzly tauri-release build --config .github/release/app.tauri-release.toml
verzly tauri-release build --config .github/release/app.tauri-release.toml --platform linux
verzly tauri-release build --config .github/release/app.tauri-release.toml --platform android
verzly tauri-release build --config .github/release/app.tauri-release.toml --platform ios
verzly tauri-release clean --config .github/release/app.tauri-release.toml
verzly tauri-release doctor --config .github/release/app.tauri-release.toml
```

GitHub Action usage:

```yaml
- uses: verzly/toolchain/actions/tauri-release@v1
  with:
    args: build --config .github/release/app.tauri-release.toml --platform linux
```

Mobile release workflows should gate Android and iOS builds behind the matching signing action output so missing signing secrets skip mobile artifacts instead of failing the whole release.

## rust-cache

`rust-cache` routes generated and build output into a project-local `.cache` tree. It covers Cargo target output, optional Cargo home routing, Gradle cache/build routing, package-manager caches, and configured generated paths.

CLI usage:

```sh
verzly rust-cache init
verzly rust-cache env
verzly rust-cache run -- cargo test --workspace
verzly rust-cache clean
verzly rust-cache clean-generated
verzly rust-cache clean-generated --dry-run
verzly rust-cache doctor
```

GitHub Action usage:

```yaml
- uses: verzly/toolchain/actions/rust-cache@v1
  with:
    args: env
```

Export cache environment variables in workflows:

```yaml
- name: Export Verzly cache environment
  run: verzly rust-cache env >> "$GITHUB_ENV"
```

Run commands through the cache wrapper locally:

```sh
verzly rust-cache run -- cargo build --workspace
verzly rust-cache run -- pnpm install
```

A downstream Tauri project can use this to keep `target`, Gradle output, package manager caches, and generated mobile directories under `.cache` instead of committing or scattering generated files through the project tree.

## android-signing

`android-signing` helps create, encode, inspect, and validate Android release signing material. It is designed so CI can detect whether Android signing is configured and skip Android artifacts when the secrets are intentionally missing.

CLI usage:

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

Expected environment variables for the standard CI path:

```text
ANDROID_KEYSTORE_BASE64
ANDROID_KEYSTORE_PASSWORD
ANDROID_KEY_ALIAS
ANDROID_KEY_PASSWORD
```

Optional fingerprint verification:

```text
ANDROID_SIGNING_CERT_SHA256
```

GitHub Action usage:

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
| --- | --- | --- |
| `optional` | `true` | Missing signing variables produce outputs and notices instead of failing. |
| `run-doctor` | `true` | Run `verzly android-signing doctor` before checking secrets. |
| `check-env` | `true` | Validate required signing environment variables. |
| `require-fingerprint` | `false` | Also require `ANDROID_SIGNING_CERT_SHA256`. |
| `extra-required-env` | empty | Additional env names to require; supports newline, comma, or space separated values. |

Action-specific outputs:

| Output | Description |
| --- | --- |
| `signing-ready` | `true`, `false`, or `unknown`. |
| `missing-secrets` | Comma-separated missing variable names. |

## ios-signing

`ios-signing` validates iOS signing environments and helps encode existing Apple signing files for GitHub Actions secrets. Like Android signing, it supports optional preflight behavior so projects can skip iOS artifacts when signing is not configured.

CLI usage:

```sh
verzly ios-signing doctor
verzly ios-signing base64 ios-release.p12
verzly ios-signing print-secrets --certificate ios-release.p12 --provisioning-profile app.mobileprovision
verzly ios-signing write-github-env --certificate ios-release.p12 --provisioning-profile app.mobileprovision
verzly ios-signing check-env
verzly ios-signing check-env --skip-apple-team-id
verzly ios-signing check-env --require APPLE_ID
```

Expected environment variables for the standard CI path:

```text
IOS_SIGNING_CERTIFICATE_BASE64
IOS_SIGNING_CERTIFICATE_PASSWORD
IOS_SIGNING_PROVISIONING_PROFILE_BASE64
IOS_SIGNING_KEYCHAIN_PASSWORD
APPLE_TEAM_ID
```

GitHub Action usage:

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
| --- | --- | --- |
| `optional` | `true` | Missing signing variables produce outputs and notices instead of failing. |
| `run-doctor` | `true` | Run `verzly ios-signing doctor` before checking secrets. |
| `check-env` | `true` | Validate required signing environment variables. |
| `skip-apple-team-id` | `false` | Do not require `APPLE_TEAM_ID`. |
| `extra-required-env` | empty | Additional env names to require; supports newline, comma, or space separated values. |
| `args` | empty | Optional additional `ios-signing` command to run after preflight. |
| `command` | empty | Deprecated alias for `args`. |

Action-specific outputs:

| Output | Description |
| --- | --- |
| `signing-ready` | `true`, `false`, or `unknown`. |
| `missing-secrets` | Comma-separated missing variable names. |

## Example app release workflow

This is the intended downstream shape for a Tauri app that can publish desktop artifacts and only publish mobile artifacts when signing is configured:

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
      - uses: actions/checkout@v4
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

## Release model for this repository

This repository has one public release target: `verzly`.

The visible workflow surface is intentionally small:

```text
.github/workflows/release.yml          Publishes the Verzly toolchain release.
.github/workflows/delete-release.yml   Deletes a published release and its tags after confirmation.
.github/workflows/test.yml             Runs pull request quality checks.
```

The single public release workflow is `.github/workflows/release.yml`. It:

1. prepares a release branch through `verzly github-release prepare`,
2. updates workspace crate versions and matching `Cargo.lock` package entries,
3. runs formatting, Clippy, and tests,
4. builds one `verzly` executable for Linux x64, macOS x64, macOS arm64, and Windows x64,
5. finalizes the release in `verzly/toolchain`, uploads assets, and updates floating tags through `github-release`.

The workflow writes only to `verzly/toolchain` through `github.token`. It must not require `DISTRIBUTION_REPO_TOKEN`, a distribution repository token, or a separate PAT for the main release path. Floating tags are part of `verzly github-release finalize`; there is no separate floating-tag workflow.

## Repository layout

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
```

## Development

Run workspace checks from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Run the unified executable from source:

```sh
cargo run -p verzly -- repository check
cargo run -p verzly -- github-release plan --config datarose.toml --release-target verzly --version 1.2.3
cargo run -p verzly -- cargo-release build --config datarose.toml --release-target verzly --version 1.2.3 --target linux-x64 --dry-run
cargo run -p verzly -- ios-signing check-env --skip-apple-team-id
```

Keep implementation boundaries clear:

- `crates/verzly` is the unified executable and should mostly dispatch.
- Each tool crate owns its CLI contract, command logic, tests, and reusable `run_from` entrypoint.
- Standalone binaries remain compatibility wrappers.
- GitHub Actions should call the released `verzly` binary, not duplicate tool logic.

## Action quality rules

- Do not print secret values.
- Prefer explicit inputs and documented outputs.
- Signing checks should support optional mode so release workflows can skip unavailable mobile artifacts.
- Use `actions/_shared/install-verzly.sh` for installing released assets.
- Keep action examples copy-pasteable for downstream repositories.

## Contributing

Source changes, action surfaces, documentation, and releases all live in this repository.

Before opening a PR, run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Do not reintroduce `.verzly/distributions`, distribution sync workflows, or release workflows that publish to separate `verzly/<tool>` repositories. Public usage must go through `verzly/toolchain`, either with the root action, subpath actions under `actions/`, or the `verzly` release assets.

## License

AGPL-3.0-only.
