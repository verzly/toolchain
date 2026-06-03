# Verzly Toolchain

Verzly Toolchain is the private source workspace for the release tools that build Rust binaries, coordinate Tauri releases, route build cache output, prepare Android signing material, and publish GitHub Releases.

The source stays in this repository. Public distribution repositories stay intentionally small: each one contains only a `README.md`, an `action.yml`, a `LICENSE`, and executable assets published through GitHub Releases.

- [Projects](#projects)
- [Repository model](#repository-model)
- [Release model](#release-model)
- [Release all](#release-all)
- [Toolchain release](#toolchain-release)
- [Release authentication](#release-authentication)
- [Release notes and PR links](#release-notes-and-pr-links)
- [Development](#development)
- [Distribution repository contents](#distribution-repository-contents)
- [Contributing](#contributing)

## Projects

`github-release` owns release branches, version-file updates, source tags, public GitHub Releases, release notes, asset uploads, and failed-release cleanup.

`cargo-release` owns Rust executable artifact builds, target-specific artifact naming, manifests, checksums, and optional Docker or Podman isolation.

`tauri-release` coordinates Tauri desktop and mobile release artifacts while keeping platform-specific build rules explicit.

`rust-cache` configures native Cargo target directories and optional environment-based cache paths so regular `cargo` commands keep generated files inside the workspace cache.

`android-signing` generates, inspects, encodes, and exports Android release signing material for local and CI release builds.


## Repository model

This repository is the source of truth for Rust code, release workflows, and crate-specific release configuration. Crates do not carry their own README files; internal documentation belongs in this root README and AGENTS.md, while public user documentation lives in the distribution repositories.

The public distribution repositories are separate repositories:

```text
verzly/github-release
verzly/cargo-release
verzly/tauri-release
verzly/rust-cache
verzly/android-signing
```

Distribution repositories do not contain Rust source code, `Cargo.toml`, `CHANGELOG.md`, `VERSION`, release config, test workflows, or release workflows.

The `_repos/<tool>` directories are committed distribution templates. They contain the public repository surface for each tool and nothing else: `README.md`, `action.yml`, and `LICENSE`. Release workflows sync the matching `_repos/<tool>` directory into the corresponding public repository before creating the GitHub Release. There must be no `distribution/` or orchestration `scripts/` directory in `verzly/toolchain`.

## Release model

Each public tool has its own tiny workflow in `.github/workflows/release-<tool>.yml`. Those workflows delegate to the reusable `.github/workflows/_release-tool.yml` workflow.

The reusable workflow intentionally calls the Rust tools directly instead of hiding release behavior in shell scripts:

```text
github-release prepare    # source branch + version update
cargo fmt/clippy/test     # plain Cargo commands, routed by .cargo/config.toml
cargo-release build       # executable assets, checksums, manifests
github-release finalize   # merge source branch + source tag
github-release publish    # public GitHub Release + uploaded assets
```

A release for one tool follows this lifecycle:

```text
1. Trigger release-<tool>.yml with the target version.
2. github-release prepare creates release/<tool>-vX.Y.Z in verzly/toolchain.
3. github-release prepare updates crates/<tool>/Cargo.toml on that branch.
4. Plain Cargo formatting, clippy, and tests run from that branch using the checked-in `.cargo/config.toml`.
5. cargo-release builds executable assets from that same branch.
6. github-release abort deletes the temporary source branch if tests or builds fail.
7. github-release finalize merges the source branch into master and creates <tool>-vX.Y.Z.
8. The workflow syncs `_repos/<tool>` to the matching public distribution repository.
9. github-release publish creates vX.Y.Z in the public distribution repository and uploads assets.
```

Each public tool has one release configuration file:

```text
crates/<tool>/github-release.toml
```

This one file contains both release contexts. The `[source_release]` section controls the temporary source branch and the package-prefixed source tag in `verzly/toolchain`. The `[release]` section controls the clean public `vX.Y.Z` GitHub Release in the distribution repository.

Version files are still explicit. `github-release` does not scan the workspace and guess which `Cargo.toml` belongs to a crate. Every crate config lists its own manifest under `[[files]]`, for example `crates/cargo-release/Cargo.toml`.


The build configuration lives here:

```text
crates/<tool>/cargo-release.toml
```

These files are not copied to distribution repositories.

## Release all

Use `.github/workflows/release-all.yml` when the same version should be released for every public tool and for the toolchain repository itself.

The workflow is intentionally a dispatcher with one visible job. It starts the existing per-tool release workflows one after another, watches each dispatched run, and stops on the first failure. This keeps the Release All graph readable while preserving the safer sequential release order.

The dispatcher passes the repository explicitly to GitHub CLI commands, so it does not depend on a checked-out `.git` directory.

Release order:

```text
github-release
cargo-release
tauri-release
rust-cache
android-signing
toolchain
```

Trigger it with one version, for example `1.2.3`. Public package repositories receive `v1.2.3`; the source monorepo receives package-prefixed source tags such as `cargo-release-v1.2.3`; the final toolchain release receives `v1.2.3` in `verzly/toolchain`.

## Toolchain release

The toolchain repository also has its own release workflow: `.github/workflows/release-toolchain.yml`.

A toolchain release does not upload executable assets. It publishes a GitHub Release in `verzly/toolchain` using the clean tag `vX.Y.Z` and regular mixed GitHub-generated notes from the source repository. This release is for maintainers and monorepo history, not for public executable distribution.

The root config for this release is:

```text
github-release.toml
```

## Release authentication

Release workflows use the built-in `GITHUB_TOKEN` for operations in `verzly/toolchain` by default. This avoids requiring a custom token just to run the workflow.

Publishing into separate distribution repositories requires `DISTRIBUTION_REPO_TOKEN` with write access to those repositories. Define it as a repository or organization secret before running any `release-<tool>.yml` workflow or `release-all.yml`. Public repository visibility only makes repositories readable; pushing `README.md`/`action.yml`/`LICENSE`, creating tags, creating releases, and uploading assets still require authenticated write access.

## Release notes and PR links

Pull requests and code review happen in `verzly/toolchain`, not in the distribution repositories. For that reason, public distribution releases generate notes from the source repository tag.

Source tags use the tool name as a prefix:

```text
github-release-v1.2.3
cargo-release-v1.2.3
tauri-release-v1.2.3
rust-cache-v1.2.3
android-signing-v1.2.3
```

Public distribution tags stay clean:

```text
v1.2.3
```

This keeps monorepo tags unambiguous while giving public users the conventional tag names they expect in each distribution repository.

Package distribution releases use scoped release notes. A package release includes commits when either of these is true:

```text
1. The Conventional Commit or squash-merge title uses the package scope, for example `fix(cargo-release): ...`.
2. The commit changes files under the configured package path, for example `crates/cargo-release/`.
```

Use the special scope `all` for changes that should appear in every package release note:

```text
chore(all): update shared release infrastructure
```

Use `toolchain`, `ci`, `docs`, `deps`, or `workspace` for source-repository maintenance changes that should appear in the toolchain release but not every package release.

## Development

Run checks from the workspace root. The checked-in `.cargo/config.toml` routes normal Cargo build output to `.cache/rust/packages/toolchain/target`, so no wrapper command is needed:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Useful local commands:

```sh
cargo run -p github-release -- plan --config crates/cargo-release/github-release.toml --version 1.2.3
cargo run -p cargo-release -- plan --config crates/cargo-release/cargo-release.toml
cargo run -p rust-cache -- doctor
cargo run -p rust-cache -- init
```

Keep the workspace plain and readable. Avoid build scripts, proc macros, hidden global behavior, and shell orchestration unless there is a concrete reason.

## Distribution repository contents

Distribution repository files are maintained in `_repos/<tool>` inside `verzly/toolchain`:

```text
README.md
action.yml
LICENSE
```

Release workflows treat these directories as authoritative public-repository templates. Before publishing a public GitHub Release, `_release-tool.yml` checks out the target distribution repository, replaces its contents from `_repos/<tool>`, commits any changes, and pushes `master`. The public repositories should not receive manual source-code changes.

## Contributing

Contribution guidelines live in [CONTRIBUTING.md](CONTRIBUTING.md). Source changes happen in `verzly/toolchain`; public distribution repositories are generated release surfaces and should not receive source changes directly.

## License

Copyright (C) 2020-present Zoltán Rózsa. Released under the GNU Affero General Public License v3.0 only.
