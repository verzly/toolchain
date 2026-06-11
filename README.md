# Verzly Toolchain

Verzly Toolchain is the private source workspace for the release tools that build Rust executables, prepare Tauri installers, route build caches, generate Android signing material, and publish GitHub Releases.

Public repositories stay intentionally small. Their user-facing `README.md`, `CONTRIBUTING.md`, `action.yml`, and `LICENSE` files are maintained in `.codex/distributions/<tool>`, then synchronized to `verzly/<tool>` with a maintainer workflow. Source code, tests, release configuration, and release workflows stay here.

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
  - [Delete a release](#delete-a-release)
  - [Update floating tags](#update-floating-tags)
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

`repo-quality` bootstraps repository-local quality gates for Rust, JavaScript, TypeScript, Vue, and PHP projects. It carries the shared Verzly defaults for `mise`, `hk`, GitHub Actions, `.editorconfig`, Rust formatting, Oxlint, Oxfmt, Vitest, Rector PHP, and Pest PHP.

### Repository model

`verzly/toolchain` owns the source:

```text
.github/workflows/
.cargo/config.toml
.codex/distributions/
crates/
Cargo.toml
Cargo.lock
datarose.toml
datarose.toml
hk.pkl
mise.toml
```

Crate-level README files are intentionally not used. Maintainer documentation lives in this README, [AGENTS.md](AGENTS.md), and [CONTRIBUTING.md](CONTRIBUTING.md). Public user documentation lives in `.codex/distributions/<tool>/README.md` and is synchronized to the public repositories.

### Distribution templates

Each `.codex/distributions/<tool>` directory contains exactly:

```text
README.md
CONTRIBUTING.md
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

The workspace also includes `hk.pkl` and `mise.toml` as the first self-hosted `repo-quality` result. `mise.toml` pins `hk`, `pkl`, and `rust@stable` for local quality gates. After installing `mise`, run:

```sh
mise install
mise exec -- hk install
mise exec -- hk check
```

GitHub Actions use the same `mise exec -- hk check` gate that `repo-quality` writes into repositories. The workflow cancels older PR runs when a newer push arrives and stops early for WIP commit subjects.

### Run a tool locally


`repo-quality` can also initialize monorepo subdirectories. The root `datarose.toml` stores the selected workspace path so future updates do not need it again:

```sh
cargo run -p repo-quality -- init --workspace workspace/app
cargo run -p repo-quality -- update
```

Generated project-local files are intentionally overrideable. `repo-quality update` keeps existing `.editorconfig`, `.oxfmtrc.json`, `.oxlintrc.json`, `rustfmt.toml`, and `rector.php` files unless `--force` is passed.


Validate `datarose.toml` without rewriting files:

```sh
repo-quality check
repo-quality check --config config/datarose.toml
```

The check command exits with `1` only when it finds removed, deprecated, or invalid Datarose settings. It is also included in generated `hk` pre-push checks.

`datarose.toml` also describes release targets and can manage every Cargo package when `manage_cargo_packages = true`. `repo-quality update` uses those targets to generate GitHub Actions release workflows, so repositories can share the same `github-release` / `cargo-release` orchestration model while keeping target-specific repositories, release metadata, version files, scoped notes, and distribution paths in one root TOML file. Pass `--config path/to/file.toml` when a repository needs a non-default config file; otherwise `repo-quality` reads the root `datarose.toml`.

Use `cargo run -p <crate> -- ...` while developing:

```sh
cargo run -p github-release -- plan --config datarose.toml --release-target cargo-release --version 1.2.3
cargo run -p cargo-release -- build --config datarose.toml --release-target cargo-release --version 1.2.3
cargo run -p tauri-release -- plan --config datarose.toml
cargo run -p rust-cache -- init
cargo run -p android-signing -- generate
cargo run -p repo-quality -- plan
```

A tool does not need a public release before you can test it locally. Cargo can run the current source directly:

```sh
cargo run -p repo-quality -- init --dry-run --skip-mise-use --skip-hk-install
cargo run -p repo-quality -- update --dry-run --skip-mise-use --skip-hk-install
cargo run -p repo-quality -- plan
cargo run -p repo-quality -- doctor
```

`repo-quality doctor` also reports missing `mise.toml` entries. For Rust repositories it recommends `rust@stable`; for JavaScript and TypeScript repositories it recommends `aube` unless an existing runner such as `pnpm`, `bun`, or `yarn` is already configured; for PHP repositories it recommends `php` together with Rector PHP and Pest PHP setup guidance.

Build and run the local executable when you want to test the exact binary entry point:

```sh
cargo build -p repo-quality
.cache/rust/packages/toolchain/target/debug/repo-quality plan
```

On Windows, run the built executable with `.exe`:

```pwsh
.\.cache\rust\packages\toolchain\target\debug\repo-quality.exe plan
```

You can also install the current source into your local Cargo bin directory without a GitHub Release:

```sh
cargo install --path crates/repo-quality --force
repo-quality plan
```

Release workflows build the executables and call the same commands directly. There are no separate orchestration scripts. Every executable and subcommand help output links back to the matching public README, for example `https://github.com/verzly/github-release`.

### Cache layout

Cargo output is routed by the checked-in config:

```toml
[build]
target-dir = ".cache/rust/packages/toolchain/target"
```

The root `datarose.toml` is the policy source for regenerating or repairing cache settings. Normal development should use plain Cargo commands; `rust-cache run` is reserved for tools that need environment variables Cargo cannot read from `.cargo/config.toml`.

## Release Workflows

### Release one public tool

Use the matching workflow when one tool needs a public release:

```text
.github/workflows/release-github-release.yml
.github/workflows/release-cargo-release.yml
.github/workflows/release-tauri-release.yml
.github/workflows/release-rust-cache.yml
.github/workflows/release-android-signing.yml
.github/workflows/release-repo-quality.yml
```

The flow is:

```text
github-release prepare
cargo fmt / clippy / test
cargo-release build
github-release finalize --merge-strategy squash --skip-github-release
sync released distribution repository
github-release publish
```

`prepare` creates a temporary source branch, updates the configured version files, and runs configured prepare commands such as `cargo generate-lockfile` before committing. If tests or builds fail, `abort` removes the branch. If everything succeeds, `finalize` merges to `master` and creates the package-prefixed source tag. The workflow then syncs the matching public distribution repository with a release-specific bump commit before `publish` creates the public release and uploads assets.

Source finalization uses a squash merge by default. The release branch may contain multiple preparation commits, but `master` receives one release commit whose body lists the squashed branch commits. If the release branch has no source diff because the requested version is already present in `master`, finalization skips the squash commit and creates the release tags from the existing `master` commit.

### Release all tools

Use `.github/workflows/release-all.yml` to release every configured public distribution target from `datarose.toml` with one version input.

The generated workflow runs the shared `_release-datarose-tool.yml` workflow once per target. Each target prepares its source release branch, runs the quality gate, builds assets from its `datarose.toml` target, finalizes the package-prefixed source tag, and publishes the public `vX.Y.Z` release.

```text
release-all.yml
→ _release-datarose-tool.yml for github-release
→ _release-datarose-tool.yml for cargo-release
→ _release-datarose-tool.yml for tauri-release
→ _release-datarose-tool.yml for rust-cache
→ _release-datarose-tool.yml for android-signing
→ _release-datarose-tool.yml for repo-quality
```

Public repositories receive `vX.Y.Z`; the source repository receives package-prefixed source tags such as `cargo-release-vX.Y.Z`.

Public distribution configs enable moving release tags. After publishing `v1.2.3`, `github-release publish` updates `v1.2` and `v1` in the matching public `verzly/<tool>` repository. It also keeps `latest` on the highest stable release and `next` on the highest preview release. When no preview release exists, `next` points at the same stable release as `latest`.

The public composite actions support those moving refs as action pins. A workflow can use `verzly/<tool>@latest`, `@next`, `@v1`, or `@v1.2`; the action reads the requested ref, resolves it to the concrete version tag on the same commit, and downloads the executable from that release. Executable assets remain attached to immutable `vX.Y.Z` releases instead of duplicated onto moving tags.

### Delete a release

Use `.github/workflows/delete-release.yml` only for release cleanup or rollback. The workflow takes the same version input style as release workflows: enter `X.Y.Z` without the `v` prefix, and confirm with `DELETE X.Y.Z`. It checks repository access before deleting anything, deletes the selected GitHub Release through the GitHub API, and then deletes the matching Git tag explicitly. For `all`, it removes `vX.Y.Z` from `verzly/toolchain`, removes `vX.Y.Z` from every public `verzly/<tool>` repository, and removes every package-prefixed source tag such as `cargo-release-vX.Y.Z` from `verzly/toolchain`.

Public repository cleanup requires `DISTRIBUTION_REPO_TOKEN`; source repository cleanup uses `github.token`.

### Update floating tags

Use `.github/workflows/update-floating-tags.yml` to repair or backfill moving tags in public distribution repositories without publishing a new release. The workflow uses `github-release floating-tags --config datarose.toml --release-target <tool>` and skips targets where all moving tag families are disabled.

Modes:

```text
all      scan all SemVer tags and repair every enabled moving tag
version  analyze one version input such as 1.2.3 or 1.3.0-rc.1
tag      analyze one full tag such as v1.2.3 or v1.3.0-rc.1
```

The workflow requires `DISTRIBUTION_REPO_TOKEN` because moving tags are written to the public `verzly/<tool>` repositories. The source repository does not update distribution moving tags.

### Sync distribution repositories

Use `.github/workflows/sync-distributions.yml` when public `README.md`, `action.yml`, or `LICENSE` files need to be pushed to the separate `verzly/<tool>` repositories without creating a release.

The workflow reads `.codex/distributions/<tool>`, clones the matching public repository with `DISTRIBUTION_REPO_TOKEN`, replaces the public surface, and commits with the configured message. Manual runs skip the commit when nothing changed unless `force-commit` is enabled. Release workflows call it with `force-commit: true` and a version-specific bump message before public tags and GitHub Releases are created. The default manual message is:

```text
chore(distribution): bump public surface
```

## Release Configuration

### Source and public tags

Each public tool owns:

```text
datarose.toml
datarose.toml
```

`datarose.toml` contains the per-tool `github-release` context. Each `[[release.targets]]` entry controls the source tag prefix, public repository, version file, scoped release notes, and prepare commands for one public distribution. `datarose.toml` also contains the executable artifact build configuration.

For public distribution repositories, `[release]` also enables moving tags:

```toml
floating_tags = true
latest_tag = true
next_tag = true
```

With `tag_prefix = "v"` and `tag_suffix = ""`, publishing `v1.2.3` updates `v1.2` and `v1`. Stable releases update `latest` to the highest stable `vX.Y.Z`. Preview releases such as `v1.3.0-rc.1` update `next` to the highest preview. If no preview release exists, `next` points to the same commit as `latest`.

Distribution `action.yml` files must resolve moving action refs to the concrete release tag before downloading assets. For example, `@v1.2` should download from the highest `v1.2.Z` release tag on the same commit, while `@latest` and `@next` should download from the stable or preview version tag that shares the moving tag commit.

### Release notes

Public releases can use generated notes, scoped source notes, no notes, or a custom body. Generated and scoped notes normalize pull request URLs so the visible text is `#123` for the current repository or `toolchain#123` for another repository; the full URL stays hidden behind the Markdown link.

Generated notes resolve the previous tag by SemVer within the same tag prefix and suffix, then pass that tag to GitHub as `previous_tag_name`. This keeps `v0.2.0` notes scoped to changes after `v0.1.0` instead of replaying the first release, while ignoring moving tags such as `v0`, `v0.1`, `latest`, and `next`.

The current public tool configs use custom release text that points users back to the exact source comparison in `verzly/toolchain`, for example:

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
verzly/repo-quality
```

They are distribution surfaces only. Development happens in `verzly/toolchain`.

## Contributing

Contribution and maintainer workflow details live in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Copyright (C) 2020-present Zoltán Rózsa. Released under the GNU Affero General Public License v3.0 only.
