# Verzly Toolchain

Verzly Toolchain is the private source workspace for the release tools that handle Rust builds, Tauri builds, Android signing, build cache routing, and GitHub Release publishing.

The source stays here because the tools share a common core: process execution, platform naming, artifact naming, checksum files, release branches, and GitHub Release publishing. The public distribution repositories stay intentionally small so users get a focused README, a single `action.yml`, and release artifacts — nothing else.

## Contents

- [Projects](#projects)
- [Repository model](#repository-model)
- [How releases work](#how-releases-work)
- [Release notes and PR links](#release-notes-and-pr-links)
- [Development](#development)
- [Contributing](#contributing)
- [License](#license)

## Projects

`github-release` prepares release branches, updates configured version files, finalizes builds into GitHub Releases, and aborts failed releases cleanly.

`cargo-release` builds Rust executable artifacts for multiple targets and writes output into a clean release directory with optional Docker or Podman isolation.

`tauri-release` coordinates Tauri desktop and mobile release artifacts with explicit platform boundaries and the same isolation model as `cargo-release`.

`rust-cache` routes Rust and Tauri build cache into a project-local `.cache/` directory so generated output stays predictable and easy to remove.

`android-signing` prepares and inspects Android signing keystores, encodes them for CI secrets, and writes the environment variables that a Tauri Android build expects.

`verzly-core` contains shared helpers that the tools all need. It must not grow into a framework.

## Repository model

This repository is the single source of truth. No Rust source lives in the distribution repositories.

Each tool publishes to its own public repository:

```text
verzly/github-release
verzly/cargo-release
verzly/tauri-release
verzly/rust-cache
verzly/android-signing
```

Those repositories contain a `README.md`, an `action.yml`, a `LICENSE`, and the release executable assets. Nothing else. The content is maintained under `_repos/<tool>/` in this workspace and synced during each release.

A per-tool release workflow in this repository handles the full lifecycle: test, build, tag, sync, publish. Each workflow reads its release configuration from `crates/<tool>/github-release.toml`.

## How releases work

Each tool is released independently. A release for `cargo-release` does not force a release of `rust-cache`. Each crate has its own version in `Cargo.toml`.

The lifecycle for a single tool:

```text
1. Trigger release-<tool>.yml with the target version.
2. Tests run across the workspace.
3. The tool binary is built on Linux, macOS, and Windows.
4. github-release prepare runs in the toolchain repository:
     - creates a release branch (release/<tool>-v1.2.3)
     - updates the version in crates/<tool>/Cargo.toml
     - pushes the branch
5. A source tag is pushed: <tool>-v1.2.3
6. The distribution repository verzly/<tool> is cloned.
7. _repos/<tool>/ content replaces the repository content.
8. github-release finalize runs in the distribution repository:
     - merges the release branch into master
     - creates the public tag: v1.2.3
     - publishes the GitHub Release with generated notes
     - uploads the executable assets and checksums
```

The temporary release branch is removed after a successful finalize. If a build fails before finalize, run `github-release abort` to remove the branch without touching `master`.

## Release notes and PR links

Release notes are generated from the source repository. The `source_repository` field in each `crates/<tool>/github-release.toml` is set to `verzly/toolchain`, which is where the pull requests and code review history live.

When `github-release` creates a public release on `verzly/<tool>`, the generated "What's Changed" section contains pull request links that point back to `verzly/toolchain`. This is intentional. The distribution repository has no pull requests of its own.

Source tags use the tool name as a prefix (`github-release-v1.2.3`). Public tags in the distribution repository use the plain `v` prefix (`v1.2.3`). This keeps monorepo tags unambiguous in the source while keeping the distribution tags clean.

## Development

Work happens in the workspace root. Shared code belongs in `verzly-core` only when at least two tools need it.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Run any tool directly from the workspace:

```sh
cargo run -p github-release -- plan --version 0.1.0
cargo run -p cargo-release -- plan
cargo run -p rust-cache -- doctor
```

The workspace has no build scripts and no proc macros. Keep it that way unless there is a concrete reason.

## Contributing

Each tool owns one clear responsibility. `cargo-release` builds artifacts. `github-release` publishes them. `rust-cache` routes cache. These boundaries should stay clean.

When a feature fits into more than one tool, pick the one whose job most closely matches the new behavior. Do not add a cross-tool dependency. Do not add behavior to `verzly-core` that only one tool needs.

Changes to `_repos/<tool>/` content (README, action.yml) are part of the release and are synced automatically. Update them here, not in the distribution repository directly.

## License

Copyright (C) 2020–present Zoltán Rózsa. Released under the GNU Affero General Public License v3.0 only.
