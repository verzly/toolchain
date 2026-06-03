# AI Agent Guide for Verzly Toolchain

This guide defines the intended architecture, repository boundaries, release model, and maintenance rules for AI agents working on the Verzly toolchain ecosystem.

## Primary rule

`verzly/toolchain` is the private Rust workspace. It contains source code, internal crate documentation, release workflows, and release configuration.

The public distribution repositories are separate repositories. They are not subdirectories of `verzly/toolchain`.

In handoff ZIP files, a sibling `_repos/` directory may be included next to `toolchain/`. That directory is a convenience export of the other repositories, not part of the toolchain project.

Correct handoff ZIP layout:

```text
toolchain.zip
├── toolchain/                         # The actual verzly/toolchain repository
│   ├── .github/workflows/
│   ├── crates/
│   ├── Cargo.toml
│   ├── LICENSE
│   ├── README.md
│   └── AGENTS.md
├── _repos/                            # Convenience export only, not committed to toolchain
│   ├── github-release/
│   ├── cargo-release/
│   ├── tauri-release/
│   ├── rust-cache/
│   └── android-signing/
└── AI_AGENT_GUIDE.md                  # Optional copy of this guide for handoff context
```

Incorrect repository layout:

```text
verzly/toolchain/_repos/
verzly/toolchain/distribution/
verzly/toolchain/scripts/
```

Do not add orchestration shell scripts for release behavior that belongs in `github-release`, `cargo-release`, or `rust-cache`.

## Tool to repository mapping

| Crate | Source location | Public repository | Source tag | Public tag |
| --- | --- | --- | --- | --- |
| `github-release` | `crates/github-release` | `verzly/github-release` | `github-release-vX.Y.Z` | `vX.Y.Z` |
| `cargo-release` | `crates/cargo-release` | `verzly/cargo-release` | `cargo-release-vX.Y.Z` | `vX.Y.Z` |
| `tauri-release` | `crates/tauri-release` | `verzly/tauri-release` | `tauri-release-vX.Y.Z` | `vX.Y.Z` |
| `rust-cache` | `crates/rust-cache` | `verzly/rust-cache` | `rust-cache-vX.Y.Z` | `vX.Y.Z` |
| `android-signing` | `crates/android-signing` | `verzly/android-signing` | `android-signing-vX.Y.Z` | `vX.Y.Z` |

`verzly-core` is internal and must not have a public distribution repository.

## Public distribution repositories

Each public repository should contain only:

```text
README.md
action.yml
LICENSE
```

Do not add these files or directories to public distribution repositories:

```text
Cargo.toml
Cargo.lock
src/
crates/
.github/workflows/test.yml
.github/workflows/release.yml
github-release.toml
cargo-release.toml
CHANGELOG.md
VERSION
```

The public repositories are thin distribution surfaces. They are not development repositories.

## Handoff `_repos` directory

When producing a ZIP for the maintainer, include `_repos/` as a top-level sibling of `toolchain/` only.

`_repos/<tool>` mirrors the intended content of the corresponding public repository:

```text
_repos/github-release/README.md
_repos/github-release/action.yml
_repos/github-release/LICENSE
```

This exists only to reduce manual work when updating multiple repositories. It is not source code and is not part of the private monorepo.

## Source versioning

Each public crate must have its own version in its own `Cargo.toml`:

```toml
[package]
name = "cargo-release"
version = "0.1.0"
```

Do not use `version.workspace = true` for independently released public tools.

## Release configuration

Each public tool owns three important configs:

```text
crates/<tool>/source-github-release.toml  # source branch, source tag, Cargo.toml version update
crates/<tool>/github-release.toml         # public distribution release
crates/<tool>/cargo-release.toml          # executable asset build
```

The source config must use a tool-prefixed source tag:

```toml
tag_prefix = "cargo-release-v"

[[files]]
path = "crates/cargo-release/Cargo.toml"
kind = "toml"
key = "package.version"
value = "{version}"
```

The distribution config must use the public repository and a clean public tag:

```toml
target_repository = "verzly/cargo-release"
source_repository = "verzly/toolchain"
source_tag_prefix = "cargo-release-v"
tag_prefix = "v"
files = []
```

These configs must stay in the source repository and must not be copied to distribution repositories.

## Release lifecycle

A release workflow must perform source release work before public distribution release work.

Expected flow:

1. `github-release prepare` creates a temporary source release branch in `verzly/toolchain`.
2. `github-release prepare` updates `crates/<tool>/Cargo.toml` to the requested version on that branch.
3. `rust-cache run -- cargo fmt`, `clippy`, and `test` run from that exact branch.
4. `cargo-release build` builds executable assets from that exact branch.
5. `github-release abort` deletes the temporary source release branch if anything fails.
6. `github-release finalize --skip-github-release` merges the branch into `master` and creates `<tool>-vX.Y.Z`.
7. `github-release publish` creates `vX.Y.Z` in the public distribution repository, generates notes from `verzly/toolchain`, and uploads assets.

The source tag must exist before public release notes are generated. Pull request links in public release notes should point to `verzly/toolchain`, because that is where the actual code changes live.

## CI expectations

Release workflows expect a token that can push to `verzly/toolchain` and create releases in the target public repository. The expected secret name is `DISTRIBUTION_REPO_TOKEN`.

Each public tool has its own small workflow:

```text
.github/workflows/release-github-release.yml
.github/workflows/release-cargo-release.yml
.github/workflows/release-tauri-release.yml
.github/workflows/release-rust-cache.yml
.github/workflows/release-android-signing.yml
```

Those files should remain thin wrappers around the reusable workflow:

```text
.github/workflows/_release-tool.yml
```

Do not reintroduce large shell scripts for release orchestration. If a workflow needs more than a small command invocation, the behavior probably belongs in one of the Rust tools.

Do not add test or release workflows to public distribution repositories.

## Rust architecture expectations

Prefer small, explicit modules over abstract frameworks.

General module boundaries:

- `cli.rs`: CLI shape and typed command arguments.
- `commands/*`: command orchestration and user-visible command behavior.
- `config.rs`: config loading, validation, and defaults.
- `domain.rs` or specific domain modules: business rules and value objects.
- `process.rs`: subprocess execution helpers.
- `github.rs`: GitHub CLI/API integration.
- `git.rs`: Git operations.
- `output.rs`: user-visible output formatting.

Avoid mixing CLI parsing, filesystem mutation, process execution, and GitHub calls in the same function.

## Error handling expectations

Use typed errors where the caller needs to react differently. Use contextual errors for boundary failures such as file IO, TOML parsing, Git commands, GitHub CLI calls, and process execution.

Never silently ignore failed shell commands, missing assets, missing release config, or dirty working trees during release operations.

## Security expectations

Do not print secrets. Do not write signing passwords or GitHub tokens to logs. Android signing commands may output GitHub Actions secret names and encoded values only when the command is explicitly designed for that purpose.

Do not run arbitrary shell fragments from config unless that is an explicit and reviewed feature with clear trust boundaries.

Prefer explicit allowlists for generated artifact paths and uploaded files.

## Documentation expectations

Public README files should be human, usage-oriented, and complete enough for developers who have never seen `verzly/toolchain`. They must explain what the tool does, why it exists, how it works, practical use cases, GitHub Action examples, every action input, every action output, every CLI command, every CLI argument, accepted values, defaults, and important configuration fields.

The public README can be longer than the crate README. The public README is the product documentation for the distribution repository; the crate README is internal developer context.

The root `toolchain/README.md` is for maintainers. Crate-level READMEs are for internal development. Public distribution READMEs live outside the project in the handoff `_repos/` export.

Do not add `CHANGELOG.md` or `VERSION` files unless explicitly requested. Release notes are generated from GitHub releases.

## Hard no list

Do not add `_repos/`, `distribution/`, or release orchestration `scripts/` inside `verzly/toolchain`.

Do not put source code in public distribution repositories.

Do not make public distribution repositories responsible for testing, building, or releasing themselves.

Do not make workflows depend on files outside the checked-out `verzly/toolchain` repository.
