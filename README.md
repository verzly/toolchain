# Verzly Toolchain

Verzly Toolchain is the private source workspace for the release tools that build Rust executables, prepare Tauri installers, route build caches, generate Android signing material, and publish GitHub Releases.

Public repositories stay intentionally small. Their user-facing `README.md`, `action.yml`, and `LICENSE` files are maintained in `.codex/distributions/<tool>`, then synchronized to `verzly/<tool>` with a maintainer workflow. Source code, tests, release configuration, and release workflows stay here.

- [Overview](#overview)
  - [Tools](#tools)
  - [Repository model](#repository-model)
  - [Distribution templates](#distribution-templates)
- [Use the workspace](#use-the-workspace)
  - [Local checks](#local-checks)
  - [Run a tool locally](#run-a-tool-locally)
  - [Cache layout](#cache-layout)
- [Release workflows](#release-workflows)
  - [Release one public tool](#release-one-public-tool)
  - [Release all tools](#release-all-tools)
  - [Release toolchain only](#release-toolchain-only)
  - [Sync distribution repositories](#sync-distribution-repositories)
- [Release configuration](#release-configuration)
  - [Source and public tags](#source-and-public-tags)
  - [Release notes](#release-notes)
  - [Authentication](#authentication)
- [Reference](#reference)
  - [Repository boundaries](#repository-boundaries)
  - [Public repositories](#public-repositories)
- [Contributing](#contributing)

## Overview

### Tools

`github-release` creates release branches, updates configured version files, merges successful source releases, creates tags, publishes GitHub Releases, uploads assets, and aborts failed release branches.

`cargo-release` builds Rust executable artifacts for configured targets, writes checksums, and produces release manifests. Native builds are preferred; container strategies can be configured where useful.

`tauri-release` prepares Tauri desktop and mobile release artifacts, including installer-oriented output for desktop platforms and mobile package output where the project supports it.

`rust-cache` keeps normal build output under a workspace-local cache. Cargo uses `.cargo/config.toml` directly, while optional environment caches can route tools such as Gradle, npm, pnpm, and Yarn into `.cache`.

`android-signing` generates, inspects, verifies, encodes, and exports Android release signing material for local and GitHub Actions builds.

### Repository model

`verzly/toolchain` owns the source:

```text
.github/workflows/
.cargo/config.toml
.codex/distributions/
crates/
Cargo.toml
Cargo.lock
github-release.toml
rust-cache.toml
```

Crate-level README files are intentionally not used. Maintainer documentation lives in this README, [AGENTS.md](AGENTS.md), and [CONTRIBUTING.md](CONTRIBUTING.md). Public user documentation lives in `.codex/distributions/<tool>/README.md` and is synchronized to the public repositories.

### Distribution templates

Each `.codex/distributions/<tool>` directory contains exactly:

```text
README.md
action.yml
LICENSE
```

These files are the public repository surface for the matching `verzly/<tool>` repository. They are committed here so source changes, action behavior, and public documentation can be updated together before the sync workflow pushes them out.

## Use the Workspace

### Local checks

Run the full local verification loop from the workspace root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

For workflow and repository-boundary changes, also verify the model expected by `.github/workflows/test.yml`.

### Run a tool locally

Use `cargo run -p <crate> -- ...` while developing:

```sh
cargo run -p github-release -- plan --config crates/cargo-release/github-release.toml --version 1.2.3
cargo run -p cargo-release -- build --config crates/cargo-release/cargo-release.toml --version 1.2.3
cargo run -p tauri-release -- plan --config crates/tauri-release/tauri-release.toml
cargo run -p rust-cache -- init
cargo run -p android-signing -- generate
```

Release workflows build the executables and call the same commands directly. There are no separate orchestration scripts.

### Cache layout

Cargo output is routed by the checked-in config:

```toml
[build]
target-dir = ".cache/rust/packages/toolchain/target"
```

The root `rust-cache.toml` is the policy source for regenerating or repairing cache settings. Normal development should use plain Cargo commands; `rust-cache run` is reserved for tools that need environment variables Cargo cannot read from `.cargo/config.toml`.

## Release Workflows

### Release one public tool

Use the matching workflow when one tool needs a public release:

```text
.github/workflows/release-github-release.yml
.github/workflows/release-cargo-release.yml
.github/workflows/release-tauri-release.yml
.github/workflows/release-rust-cache.yml
.github/workflows/release-android-signing.yml
```

The flow is:

```text
github-release prepare
cargo fmt / clippy / test
cargo-release build
github-release finalize --skip-github-release
github-release publish
```

`prepare` creates a temporary source branch and updates only the configured version files. If tests or builds fail, `abort` removes the branch. If everything succeeds, `finalize` merges to `master` and creates the package-prefixed source tag before `publish` creates the public release and uploads assets.

### Release all tools

Use `.github/workflows/release-all.yml` to release every public tool and then the toolchain release with one version input.

The workflow is a visible dependency graph:

```text
github-release
cargo-release
tauri-release
rust-cache
android-signing
toolchain
```

Each step waits for the previous release. Public repositories receive `vX.Y.Z`; the source repository receives package-prefixed source tags such as `cargo-release-vX.Y.Z`.

### Release toolchain only

Use `.github/workflows/release-toolchain.yml` to publish a maintainer release in `verzly/toolchain` without executable assets. It uses the root `github-release.toml` and creates the clean source tag `vX.Y.Z`.

### Sync distribution repositories

Use `.github/workflows/sync-distributions.yml` when public `README.md`, `action.yml`, or `LICENSE` files need to be pushed to the separate `verzly/<tool>` repositories without creating a release.

The workflow reads `.codex/distributions/<tool>`, clones the matching public repository with `DISTRIBUTION_REPO_TOKEN`, replaces the public surface, and commits with the configured message. The default message is:

```text
chore(distribution): bump public surface
```

## Release Configuration

### Source and public tags

Each public tool owns:

```text
crates/<tool>/github-release.toml
crates/<tool>/cargo-release.toml
```

`github-release.toml` contains both release contexts. `[source_release]` controls the temporary source branch and source tag in `verzly/toolchain`; `[release]` controls the public `vX.Y.Z` release in `verzly/<tool>`.

### Release notes

Public releases can use generated notes, scoped source notes, no notes, or a custom body. The current public tool configs use custom release text that points users back to the exact source comparison in `verzly/toolchain`, for example:

```text
https://github.com/verzly/toolchain/compare/cargo-release-v0.1.0...cargo-release-v0.2.0
```

Use Conventional Commit scopes such as `fix(cargo-release): ...` and `chore(all): ...` when a release should include generated or scoped notes.

### Authentication

Source repository operations use `github.token`. Publishing or synchronizing public distribution repositories requires `DISTRIBUTION_REPO_TOKEN` with write access to the relevant `verzly/<tool>` repositories.

Do not fall back from `DISTRIBUTION_REPO_TOKEN` to `github.token` for public repositories. `github.token` is scoped to `verzly/toolchain`.

## Reference

### Repository boundaries

Do not add these inside `verzly/toolchain`:

```text
distribution/
scripts/
crates/<tool>/README.md
```

Do not add source code, workflows, release config, `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, or `VERSION` to public distribution repositories.

### Public repositories

The public repositories are:

```text
verzly/github-release
verzly/cargo-release
verzly/tauri-release
verzly/rust-cache
verzly/android-signing
```

They are distribution surfaces only. Development happens in `verzly/toolchain`.

## Contributing

Contribution and maintainer workflow details live in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Copyright (C) 2020-present Zoltán Rózsa. Released under the GNU Affero General Public License v3.0 only.
