# Verzly Toolchain

Verzly Toolchain is the private source workspace for the release tools that build Rust binaries, coordinate Tauri releases, route build cache output, prepare Android signing material, and publish GitHub Releases.

The source stays in this repository. Public distribution repositories stay intentionally small: each one contains only a `README.md`, an `action.yml`, a `LICENSE`, and executable assets published through GitHub Releases.

- [Projects](#projects)
- [Repository model](#repository-model)
- [Release model](#release-model)
- [Release all](#release-all)
- [Toolchain release](#toolchain-release)
- [Release notes and PR links](#release-notes-and-pr-links)
- [Distribution repository contents](#distribution-repository-contents)

## Projects

`github-release` owns release branches, version-file updates, source tags, public GitHub Releases, release notes, asset uploads, and failed-release cleanup.

`cargo-release` owns Rust executable artifact builds, target-specific artifact naming, manifests, checksums, and optional Docker or Podman isolation.

`tauri-release` coordinates Tauri desktop and mobile release artifacts while keeping platform-specific build rules explicit.

`rust-cache` redirects Rust and Tauri build cache output into a workspace-local cache directory so generated files stay predictable and easy to remove.

`android-signing` generates, inspects, encodes, and exports Android release signing material for local and CI release builds.

## Repository model

This repository is the source of truth for Rust code, release workflows, and crate-specific release configuration.

The public distribution repositories are separate repositories:

```text
verzly/github-release
verzly/cargo-release
verzly/tauri-release
verzly/rust-cache
verzly/android-signing
```

Distribution repositories do not contain Rust source code, `Cargo.toml`, `CHANGELOG.md`, `VERSION`, release config, test workflows, or release workflows.

There must be no `_repos/`, `distribution/`, or orchestration `scripts/` directory in `verzly/toolchain`. In handoff ZIP files, `_repos/` may appear next to `toolchain/` as a convenience export of the public distribution repositories. That sibling directory is not part of this repository.

## Release model

Each public tool has its own tiny workflow in `.github/workflows/release-<tool>.yml`. Those workflows delegate to the reusable `.github/workflows/_release-tool.yml` workflow.

The reusable workflow intentionally calls the Rust tools directly instead of hiding release behavior in shell scripts:

```text
github-release prepare    # source branch + version update
rust-cache run -- cargo   # format, clippy, test, and build cache routing
cargo-release build       # executable assets, checksums, manifests
github-release finalize   # merge source branch + source tag
github-release publish    # public GitHub Release + uploaded assets
```

A release for one tool follows this lifecycle:

```text
1. Trigger release-<tool>.yml with the target version.
2. github-release prepare creates release/<tool>-vX.Y.Z in verzly/toolchain.
3. github-release prepare updates crates/<tool>/Cargo.toml on that branch.
4. rust-cache runs formatting, clippy, and tests from that branch.
5. cargo-release builds executable assets from that same branch.
6. github-release abort deletes the temporary source branch if tests or builds fail.
7. github-release finalize merges the source branch into master and creates <tool>-vX.Y.Z.
8. github-release publish creates vX.Y.Z in the public distribution repository and uploads assets.
```

The source release configuration lives here:

```text
crates/<tool>/source-github-release.toml
```

The public distribution release configuration lives here:

```text
crates/<tool>/github-release.toml
```

The build configuration lives here:

```text
crates/<tool>/cargo-release.toml
```

These files are not copied to distribution repositories.

## Release all

Use `.github/workflows/release-all.yml` when the same version should be released for every public tool and for the toolchain repository itself.

The workflow is intentionally sequential. Each package release updates its own crate version, creates its own source tag, builds its own assets, and publishes to its own distribution repository before the next package starts. This avoids concurrent release branches racing to merge into `master`.

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

## Distribution repository contents

Distribution repository files are maintained outside `verzly/toolchain`.

In the handoff ZIP, the sibling `_repos/<tool>` directories contain the intended public repository contents:

```text
README.md
action.yml
LICENSE
```

Those directories exist only so the public repositories can be updated with less manual work. Do not commit `_repos/` into `verzly/toolchain`.

The GitHub release workflows do not depend on `_repos/`. They publish release notes and executable assets to the already-existing distribution repositories.

## License

Copyright (C) 2020-present Zoltán Rózsa. Released under the GNU Affero General Public License v3.0 only.
