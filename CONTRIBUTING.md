# Contributing to Verzly Toolchain

This guide explains how to set up the project locally, how to make changes safely, how commits and pull requests should be written, and how releases are published from the `verzly/toolchain` repository.

- [Local development](#local-development)
  - [Prerequisites](#prerequisites)
  - [Clone and prepare the workspace](#clone-and-prepare-the-workspace)
  - [Run local checks](#run-local-checks)
  - [Useful development commands](#useful-development-commands)
- [Repository model](#repository-model)
  - [Private source workspace](#private-source-workspace)
  - [Public distribution repositories](#public-distribution-repositories)
- [Working on changes](#working-on-changes)
  - [Branch naming](#branch-naming)
  - [Commit messages](#commit-messages)
  - [Pull request titles](#pull-request-titles)
  - [Testing expectations](#testing-expectations)
- [Release notes](#release-notes)
  - [Package scopes](#package-scopes)
  - [Shared and workspace scopes](#shared-and-workspace-scopes)
- [Releases](#releases)
  - [Required secret](#required-secret)
  - [Release one public tool](#release-one-public-tool)
  - [Release every public tool](#release-every-public-tool)
  - [Release the toolchain repository](#release-the-toolchain-repository)
  - [Prereleases](#prereleases)
  - [Failure handling](#failure-handling)
- [Distribution repository updates](#distribution-repository-updates)

## Local development

### Prerequisites

Install the stable Rust toolchain and the common release dependencies used by this workspace:

```sh
rustup toolchain install stable
rustup default stable
rustup component add rustfmt clippy
```

Install the GitHub CLI when testing GitHub release behavior locally:

```sh
gh auth login
```

Docker or Podman is only required when working on container-based build paths. The default CI path can still run host builds without requiring both engines locally.

### Clone and prepare the workspace

Clone the source repository and work from the workspace root:

```sh
git clone git@github.com:verzly/toolchain.git
cd toolchain
```

Build the cache helper first. The CI uses `rust-cache` to keep Cargo output under the configured workspace-local cache directory.

```sh
cargo build --release -p rust-cache
```

The default cache config lives at:

```text
crates/rust-cache/rust-cache.toml
```

It routes Cargo and Gradle output into `.cache/` instead of leaving generated build output in the normal project tree.

### Run local checks

Run the same checks expected by CI:

```sh
./target/release/rust-cache run --config crates/rust-cache/rust-cache.toml -- cargo fmt --all -- --check
./target/release/rust-cache run --config crates/rust-cache/rust-cache.toml -- cargo clippy --workspace --all-targets -- -D warnings
./target/release/rust-cache run --config crates/rust-cache/rust-cache.toml -- cargo test --workspace --all-targets
```

When `rust-cache` is not built yet, the equivalent direct commands are:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Do not submit changes that only pass formatting. A change is ready when formatting, Clippy, and tests are all green.

### Useful development commands

Preview a source release plan:

```sh
cargo run -p github-release -- plan --config crates/cargo-release/source-github-release.toml --version 1.2.3
```

Preview a public distribution release plan:

```sh
cargo run -p github-release -- plan --config crates/cargo-release/github-release.toml --version 1.2.3
```

Preview artifact build configuration:

```sh
cargo run -p cargo-release -- plan --config crates/cargo-release/cargo-release.toml
```

Inspect cache routing:

```sh
cargo run -p rust-cache -- doctor --config crates/rust-cache/rust-cache.toml
cargo run -p rust-cache -- env --config crates/rust-cache/rust-cache.toml
```

## Repository model

### Private source workspace

`verzly/toolchain` is the source of truth. It contains Rust code, tests, internal release configuration, and GitHub Actions workflows.

The source workspace must not contain these directories:

```text
_repos/
distribution/
scripts/
```

Release behavior belongs in Rust crates and reusable workflows, not in long shell scripts. Shell in workflows should stay as a thin command runner.

### Public distribution repositories

Each public repository is a thin distribution surface. It should contain only:

```text
README.md
action.yml
LICENSE
```

The public repositories must not contain Rust source code, Cargo manifests, test workflows, release workflows, `CHANGELOG.md`, `VERSION`, or internal release configuration.

In handoff ZIP files, `_repos/` may exist next to `toolchain/` as a convenience export. It is not part of the `verzly/toolchain` repository and must not be committed there.

## Working on changes

### Branch naming

Use short, descriptive branch names:

```text
feat/scoped-release-notes
fix/cargo-artifact-paths
docs/contributing-guide
chore/update-dependencies
```

Avoid branch names that look like release branches. Release branches are created by `github-release` and use the configured `release/<tool>-vX.Y.Z` pattern.

### Commit messages

Use Conventional Commits.

Good examples:

```text
feat(github-release): add scoped release note filtering
fix(cargo-release): preserve executable suffix in artifact names
docs(rust-cache): document cache routing behavior
test(android-signing): cover generated password constraints
chore(deps): update Rust dependencies
```

Use one logical change per commit when practical. Do not mix formatting-only changes, dependency upgrades, release workflow changes, and feature changes in the same commit unless they are inseparable.

### Pull request titles

Pull request titles should follow the same Conventional Commit format as commits because squash merge titles may become release-note input.

The scope matters. Package releases use scopes and changed paths to decide what belongs in each package-specific `What's changed` section.

### Testing expectations

Every behavior change should have a test at the lowest useful level.

Add or update tests when changing:

```text
release plan generation
tag or branch naming
release note filtering
version file updates
artifact discovery or naming
checksum or manifest generation
cache path routing
secret handling
configuration defaults
safety checks
```

The workspace should not rely on `cargo test` as a compile-only smoke check. Tests should protect the behavior that makes each tool useful.

## Release notes

### Package scopes

Use package scopes for changes that belong to one public tool:

```text
feat(github-release): add release note path filtering
fix(cargo-release): detect missing artifacts earlier
perf(rust-cache): avoid repeated workspace metadata lookups
docs(tauri-release): explain Android build isolation
test(android-signing): cover base64 export behavior
```

Package public releases include commits when the commit or squash-merge title uses the package scope, or when the changed files match that package path.

### Shared and workspace scopes

Use `all` only when the change should appear in every public package release:

```text
chore(all): update shared release workflow behavior
```

Use source-workspace scopes for changes that should appear in the toolchain release but not every package release:

```text
ci(toolchain): tighten repository model checks
docs(toolchain): add contributor guide
chore(deps): update Rust dependencies
refactor(workspace): remove unused shared crate
```

If a change affects multiple tools but not all of them, prefer splitting the work into package-scoped commits or PRs.

## Releases

### Required secret

The release workflows expect this repository secret:

```text
DISTRIBUTION_REPO_TOKEN
```

The token must be able to push branches and tags to `verzly/toolchain`, create releases in `verzly/toolchain`, and create releases/upload assets in the public distribution repositories.

### Release one public tool

Use the matching workflow in GitHub Actions:

```text
Release GitHub Release
Release Cargo Release
Release Tauri Release
Release Rust Cache
Release Android Signing
```

Each workflow asks for:

```text
version      # required, without the leading v; examples: 1.2.3, 1.2.3-rc.1
prerelease   # auto, true, or false
```

For example, releasing `cargo-release` version `1.2.3` performs this lifecycle:

```text
1. Create release/cargo-release-v1.2.3 in verzly/toolchain.
2. Update crates/cargo-release/Cargo.toml to 1.2.3 on that branch.
3. Run fmt, Clippy, and tests from that branch.
4. Build executable assets for the configured targets.
5. Merge the source release branch into master.
6. Create source tag cargo-release-v1.2.3 in verzly/toolchain.
7. Publish v1.2.3 in verzly/cargo-release.
8. Generate package-scoped release notes from verzly/toolchain.
9. Upload executable assets, checksums, and manifests.
```

### Release every public tool

Use this workflow when all public tools should receive the same version:

```text
Release All
```

The workflow runs sequentially in this order:

```text
github-release
cargo-release
tauri-release
rust-cache
android-signing
toolchain
```

It is intentionally sequential so multiple release branches do not race to merge into `master` at the same time.

### Release the toolchain repository

Use this workflow when only the monorepo itself needs a release:

```text
Release Toolchain
```

The toolchain release creates a clean `vX.Y.Z` tag and GitHub Release in `verzly/toolchain`. It does not upload executable assets. Its `What's changed` section may contain mixed source-workspace PRs and commits.

### Prereleases

Use SemVer prerelease versions for alpha, beta, or release-candidate builds:

```text
1.2.3-alpha.1
1.2.3-beta.1
1.2.3-rc.1
```

The `prerelease` workflow input accepts:

```text
auto   # infer from the SemVer prerelease label
true   # force prerelease
false  # force stable release
```

### Failure handling

If tests or builds fail after a source release branch has been prepared, the reusable release workflow calls `github-release abort` to delete the temporary release branch.

If a failure happens after the source tag was created, inspect the run before retrying. Do not manually create replacement tags unless the failed state is understood.

Before retrying a release, verify whether these already exist:

```text
release/<tool>-vX.Y.Z
<tool>-vX.Y.Z
vX.Y.Z in the public distribution repository
```

## Distribution repository updates

When documentation or `action.yml` changes are needed in public repositories, update the top-level handoff `_repos/<tool>` directories in the ZIP. Then copy those files into the corresponding public repositories and commit them there.

A useful commit message for documentation-only distribution updates is:

```text
docs(readme): expand usage documentation
```

Do not add source files, release configs, test workflows, or generated build output to public distribution repositories.

## License

Copyright (C) 2020-present Zoltán Rózsa. Released under the GNU Affero General Public License v3.0 only.
