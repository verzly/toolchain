# Verzly Toolchain

Verzly Toolchain is a single source repository and release surface for the `verzly` executable, its GitHub Actions, and the modular Rust crates behind the release workflow helpers.

The public contract is intentionally one repository: `verzly/toolchain`. Consumers install the unified binary from this repository's GitHub Releases or use the checked-in actions directly from this repository.

## What this repo publishes

- `verzly`, the unified executable.
- Compatibility shims for existing command names such as `github-release`, `tauri-release`, `android-signing`, `ios-signing`, `rust-cache`, and `repository`.
- A root GitHub Action at `verzly/toolchain@vX` for setup and optional command execution.
- Subpath actions such as `verzly/toolchain/actions/ios-signing@vX` for workflow-first integrations.

The former `.verzly/distributions/<tool>` model is removed. There are no separate distribution repositories to sync or release.

## Tools

### github-release

`verzly github-release` prepares release branches, updates version files, merges releases, tags source, creates GitHub Releases, uploads assets, and repairs floating tags.

### cargo-release

`verzly cargo-release` builds Rust executable release assets for Linux, macOS, and Windows, then writes checksums and manifests.

### tauri-release

`verzly tauri-release` builds Tauri desktop and mobile release artifacts.

### rust-cache

`verzly rust-cache` routes Cargo, Gradle, JavaScript package-manager caches, and generated output into project-local `.cache` directories.

### android-signing

`verzly android-signing` manages Android release signing material and validates Android signing environments.

### ios-signing

`verzly ios-signing` validates iOS signing environments and helps encode existing Apple signing files for CI secrets.

### repository

`verzly repository` manages repository quality configuration, `hk.pkl`, GitHub Actions, and `datarose.toml` conventions.

## Local development

Run the workspace checks from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Run the unified executable from source:

```sh
cargo run -p verzly -- repository check
cargo run -p verzly -- github-release plan --config datarose.toml --release-target verzly --version 1.2.3
cargo run -p verzly -- cargo-release build --config datarose.toml --release-target verzly --version 1.2.3 --target linux-x64
cargo run -p verzly -- ios-signing check-env --skip-apple-team-id
```

Build the local binary:

```sh
cargo build --release -p verzly
.cache/rust/packages/toolchain/target/release/verzly --help
```

Cargo output is kept under `.cache/rust/packages/toolchain/target` by `.cargo/config.toml`.

## GitHub Actions usage

Install the toolchain once and run commands through the unified executable:

```yaml
- uses: verzly/toolchain@v1
  with:
    command: verzly repository check
```

Install only, then run multiple commands:

```yaml
- uses: verzly/toolchain@v1

- run: verzly rust-cache env >> "$GITHUB_ENV"
- run: verzly github-release plan --config datarose.toml --release-target nutrino --version 1.2.3
```

Use the iOS signing action when a workflow should continue without iOS artifacts if signing secrets are not configured:

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
  run: verzly tauri-release build --platform ios --config .github/release/app.tauri-release.toml
```

## Release workflow

The repository has one release target: `verzly`.

Run `Release Verzly` or `Release Toolchain` with a SemVer version. The workflow:

1. prepares a release branch through `verzly github-release prepare`,
2. updates every workspace crate version and matching `Cargo.lock` package entry,
3. runs formatting, clippy, and tests,
4. builds `verzly` assets for Linux, macOS, and Windows,
5. finalizes the release in `verzly/toolchain`, uploads assets, and updates floating tags.

No `DISTRIBUTION_REPO_TOKEN` is required. The workflow only writes to `verzly/toolchain` through `github.token`.

## Repository layout

```text
.github/workflows/        Release and quality workflows
actions/                  Public GitHub Actions stored in this repo
actions/_shared/          Shared composite-action scripts
crates/verzly/            Unified executable entrypoint
crates/*/                 Modular tool implementations
action.yml                Root setup/run action
Cargo.toml                Rust workspace
datarose.toml             Release, quality, cache, and build configuration
```

## Compatibility

Standalone binaries remain as compatibility entrypoints. Internally they delegate to the same library entrypoints used by `verzly`, so existing scripts can migrate incrementally.
