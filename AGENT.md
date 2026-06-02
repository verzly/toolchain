# Agent Guide — Verzly Toolchain

This document is written for AI coding agents. It describes the repository purpose, the exact structure, the design rules, and the conventions that must be followed when making changes. Read it before touching any file.

## What this repository is

`verzly/toolchain` is a private Rust workspace. It contains the source for five CLI tools that Zoltán uses to release his own projects. The tools are distributed via separate public GitHub repositories. The source never leaves this workspace.

The five tools and their public distribution repositories:

| Crate | Distribution repo | Purpose |
|---|---|---|
| `github-release` | `verzly/github-release` | Release branch lifecycle and GitHub Release publishing |
| `cargo-release` | `verzly/cargo-release` | Cross-platform Rust binary builds via Docker/Podman |
| `tauri-release` | `verzly/tauri-release` | Cross-platform Tauri builds via Docker/Podman |
| `rust-cache` | `verzly/rust-cache` | Routes Rust/Tauri build cache to `.cache/` |
| `android-signing` | `verzly/android-signing` | Android keystore generation and CI secret preparation |

A sixth internal crate, `verzly-core`, provides shared helpers. It is not distributed.

## Repository layout

```text
toolchain/
├── .github/
│   └── workflows/
│       ├── test.yml                        # Runs on push/PR to master
│       ├── release-github-release.yml      # One workflow per tool
│       ├── release-cargo-release.yml
│       ├── release-tauri-release.yml
│       ├── release-rust-cache.yml
│       └── release-android-signing.yml
├── .gitignore
├── Cargo.toml                              # Workspace root
├── LICENSE
├── README.md
├── AGENT.md                                # This file
├── crates/
│   ├── verzly-core/                        # Shared library, not distributed
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── github-release/
│   │   ├── Cargo.toml                      # Own version field
│   │   ├── github-release.toml            # Release config for this crate
│   │   ├── README.md                       # User-facing docs (copied to _repos/)
│   │   └── src/
│   ├── cargo-release/
│   │   ├── Cargo.toml
│   │   ├── github-release.toml
│   │   ├── README.md
│   │   └── src/
│   ├── tauri-release/     (same structure)
│   ├── rust-cache/        (same structure)
│   └── android-signing/   (same structure)
├── _repos/
│   ├── github-release/
│   │   ├── README.md                       # Public-facing README
│   │   ├── action.yml                      # GitHub Action
│   │   ├── LICENSE
│   │   └── .gitignore
│   ├── cargo-release/     (same structure)
│   ├── tauri-release/     (same structure)
│   ├── rust-cache/        (same structure)
│   └── android-signing/   (same structure)
└── scripts/
    ├── build-release-asset.sh              # Build one binary for the current platform
    └── publish-distribution-repo.sh       # Sync _repos/<tool>/ and publish release
```

## What goes where

### `crates/<tool>/README.md`

Developer-facing. This is the full documentation for the tool: how it works, all commands, all config options, usage examples, GitHub Actions integration. This file is **not** what ends up in the distribution repository.

### `_repos/<tool>/README.md`

User-facing. This is what users see in `verzly/<tool>` on GitHub. It describes the tool from the outside: what it does, how to use the Action, what the release assets look like, where to file issues, where the source lives. It should not duplicate the full crate README but should orient a newcomer in under two minutes.

### `_repos/<tool>/action.yml`

The GitHub Action definition. It downloads the tool's release binary from the distribution repository and optionally runs it. It must not reference `verzly/toolchain` or build from source. The binary always comes from `verzly/<tool>` releases.

### `crates/<tool>/github-release.toml`

The release configuration for publishing this specific tool to its distribution repository. This is the config that `github-release` reads during the release workflow. Key fields:

- `release.tag_prefix` — source tag prefix in toolchain, e.g. `github-release-v`
- `github.target_repository` — the distribution repo, e.g. `verzly/github-release`
- `github.source_repository` — always `verzly/toolchain`
- `github.source_tag_prefix` — matches `release.tag_prefix`
- `files` — which file to update with the new version (always the crate's own `Cargo.toml`)

## Versioning

Each crate has its own `version` field in its `Cargo.toml`. Tools are released independently. Releasing `cargo-release` does not require releasing `rust-cache`.

The workspace `Cargo.toml` shares `edition`, `license`, and `repository` via `[workspace.package]`. It does not share `version`.

`verzly-core` has its own version and is always updated when the tools that depend on it are released, if the shared code changed.

## Release lifecycle

A release is triggered by running the matching workflow in the GitHub Actions UI:

1. `release-<tool>.yml` accepts a `version` input (bare SemVer, no `v` prefix).
2. Workspace tests run.
3. The tool binary is compiled natively on Linux, macOS, and Windows.
4. `github-release prepare` runs in the toolchain repository, creating a `release/<tool>-v<version>` branch and updating `crates/<tool>/Cargo.toml`.
5. A source tag `<tool>-v<version>` is pushed to this repository.
6. The distribution repository `verzly/<tool>` is cloned.
7. `_repos/<tool>/` content is written into the clone, replacing all non-`.git` content.
8. `github-release finalize` runs in the distribution repository, merging the release branch, creating the public `v<version>` tag, publishing the GitHub Release, and uploading the executable assets.

The full PR history lives in `verzly/toolchain`. The "What's Changed" section in each distribution release links back to this repository because `source_repository = "verzly/toolchain"` in the per-crate config.

## What does not exist in this repository

- `CHANGELOG.md` — not used anywhere
- `VERSION` files — not used anywhere
- A top-level `github-release.toml` — each crate has its own under `crates/<tool>/`
- A `distribution/` folder — replaced by `_repos/`
- `release.yml` or `test.yml` inside `_repos/` — distribution repos have no CI; the toolchain owns all workflows
- A `cargo-release.toml` at the workspace root or crate level — the build scripts use inline configuration

## Coding conventions

### Rust

- No `unwrap()` in library or command code. Use `anyhow::Result` and `?`.
- `cli.rs` contains only struct and enum definitions with `clap` derives. No logic.
- `main.rs` contains only the dispatch `match`. No logic.
- Command modules under `src/commands/` contain the actual logic.
- Configuration structs live in `config.rs` with `serde` derives and explicit `Default` impls.
- Shared utilities belong in `verzly-core` only if two or more tools need them.
- No framework-style abstractions. The code should be readable without knowing a specific pattern.

### Shell scripts

- Always `set -euo pipefail` at the top.
- Named positional arguments with explicit error messages: `TOOL="${1:?tool name is required}"`.
- No silent failures. Every significant step should echo progress or the output file path.

### GitHub Actions

- `permissions` is always declared explicitly.
- Secrets are accessed only by name; do not echo them or pass them as positional arguments.
- `GITHUB_PATH` and `GITHUB_OUTPUT` writes use `>>` not `>`.
- Every shell step uses `set -euo pipefail` when running bash.

### READMEs

- The main `README.md` at the workspace root has a `## Contents` section linking to every other `##` section. The `License` section is present but not listed in Contents.
- Each `_repos/<tool>/README.md` follows the same pattern: Contents menu, no License in the menu, License section at the bottom.
- No CHANGELOG, no VERSION, no badge clutter.
- Human, readable, not a reference dump. Show the flow; let the reader understand the purpose before the flags.

## Adding a new tool

1. Create `crates/<name>/` with `Cargo.toml` (own version), `src/main.rs`, `src/cli.rs`, `src/commands/mod.rs`, and whatever modules the tool needs.
2. Add `"crates/<name>"` to `[workspace.members]` in the root `Cargo.toml`.
3. Create `crates/<name>/github-release.toml` following the existing pattern.
4. Create `_repos/<name>/` with `README.md`, `action.yml`, `LICENSE`, `.gitignore`.
5. Create `.github/workflows/release-<name>.yml` following the existing per-tool workflow pattern.
6. Update the workspace-level `README.md` Projects section.
7. Update this file: add the new tool to the table above and mention it in the layout.

## What to avoid

- Do not add logic to `main.rs` beyond dispatch.
- Do not cross-reference one tool's internal types from another tool's source.
- Do not add workflow steps that build or install tools from the internet unless they are necessary and pinned.
- Do not commit generated files (`target/`, `.cache/`, `.release-assets/`).
- Do not add `test.yml` to `_repos/<tool>/`. Tests live in the toolchain.
- Do not add `release.yml` to `_repos/<tool>/`. Releases are published from the toolchain.
- Do not create a `CHANGELOG.md` or `VERSION` file. They are not used.
- Do not put a `github-release.toml` in `_repos/<tool>/`. The distribution repository never runs `github-release` on its own.
