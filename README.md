# Verzly Toolchain

Verzly Toolchain is the private source workspace for the release tools that build Rust binaries, coordinate Tauri releases, route build cache output, prepare Android signing material, and publish GitHub Releases.

The source stays in this repository. Public distribution repositories stay intentionally small: each one contains only a `README.md`, an `action.yml`, a `LICENSE`, and executable assets published through GitHub Releases.

## Contents

- [Projects](#projects)
- [Repository model](#repository-model)
- [Release model](#release-model)
- [Release notes and PR links](#release-notes-and-pr-links)
- [Development](#development)
- [Distribution repository contents](#distribution-repository-contents)
- [Contributing](#contributing)

## Projects

`github-release` prepares source release branches, updates configured version files, finalizes successful builds into GitHub Releases, uploads assets, and aborts failed release branches cleanly.

`cargo-release` builds Rust executable artifacts for multiple targets, with checksums and optional Docker or Podman isolation.

`tauri-release` coordinates Tauri desktop and mobile release artifacts while keeping platform-specific build rules explicit.

`rust-cache` redirects Rust and Tauri build cache output into a workspace-local cache directory so generated files stay predictable and easy to remove.

`android-signing` generates, inspects, encodes, and exports Android release signing material for local and CI release builds.

`verzly-core` contains shared helpers that should not drift between tools. It is internal and not distributed.

## Repository model

This repository is the source of truth for all Rust code, release workflows, release scripts, and crate-specific release configuration.

The public distribution repositories are separate repositories:

```text
verzly/github-release
verzly/cargo-release
verzly/tauri-release
verzly/rust-cache
verzly/android-signing
```

Distribution repositories do not contain Rust source code, `Cargo.toml`, `CHANGELOG.md`, `VERSION`, `github-release.toml`, test workflows, or release workflows.

There must be no `_repos/` or `distribution/` directory in `verzly/toolchain`. In handoff ZIP files, `_repos/` may appear next to `toolchain/` as a convenience export of the public distribution repositories. That sibling directory is not part of this repository.

## Release model

Each public tool has its own workflow in `.github/workflows/release-<tool>.yml`. Tools are released independently, and each crate has its own version in `crates/<tool>/Cargo.toml`.

A release for one tool follows this lifecycle:

```text
1. Trigger release-<tool>.yml with the target version.
2. Create a source release branch in verzly/toolchain, such as release/cargo-release-v1.2.3.
3. Update crates/<tool>/Cargo.toml on that source release branch.
4. Run formatting, clippy, and tests from the source release branch.
5. Build the selected executable on Linux, macOS, and Windows from that same branch.
6. If tests or builds fail, delete the temporary source release branch.
7. If builds succeed, merge the source release branch into master.
8. Create a source tag in verzly/toolchain, such as cargo-release-v1.2.3.
9. Clone the matching public distribution repository, such as verzly/cargo-release.
10. Create the public distribution tag, such as v1.2.3.
11. Publish the public GitHub Release and upload executable assets.
```

The crate-specific distribution release configuration lives next to the tool source:

```text
crates/github-release/github-release.toml
crates/cargo-release/github-release.toml
crates/tauri-release/github-release.toml
crates/rust-cache/github-release.toml
crates/android-signing/github-release.toml
```

These files are not copied to distribution repositories.

## Release notes and PR links

Pull requests and code review happen in `verzly/toolchain`, not in the distribution repositories. For that reason, distribution releases generate notes from the source repository tag.

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

## Development

Run checks from the workspace root:

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
```

Keep the workspace plain and readable. Avoid build scripts, proc macros, and hidden global behavior unless there is a concrete reason.

## Distribution repository contents

Distribution repository files are maintained outside `verzly/toolchain`.

In the handoff ZIP, the sibling `_repos/<tool>` directories contain the intended public repository contents:

```text
README.md
action.yml
LICENSE
```

Those directories exist only so the public repositories can be updated with less manual work. Do not commit `_repos/` into `verzly/toolchain`.

For local/manual syncing from the handoff bundle, use:

```sh
DISTRIBUTION_REPO_CONTENT_ROOT=../_repos \
  scripts/sync-repo-template.sh cargo-release ../cargo-release
```

The GitHub release workflows do not depend on that sibling directory. They publish release notes and executable assets to the already-existing distribution repositories.

## Contributing

Keep responsibilities narrow. `github-release` owns branch, tag, release, and GitHub Release publishing behavior. `cargo-release` owns Rust executable artifact building. `tauri-release` owns Tauri release artifact coordination. `rust-cache` owns cache redirection. `android-signing` owns Android signing material. `verzly-core` should reduce duplication without becoming a framework.

Prefer incremental refactoring over rewrites. Every public-facing change must keep the source-only monorepo and source-free distribution repository model intact.

## License

Copyright (C) 2020-present Zoltán Rózsa. Released under the GNU Affero General Public License v3.0 only.
