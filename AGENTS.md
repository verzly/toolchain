# AI Agent Guide for Verzly Toolchain

This guide defines the intended architecture, repository boundaries, release model, and maintenance rules for AI agents working on the Verzly toolchain ecosystem.

## Primary rule

`verzly/toolchain` is the private Rust workspace. It contains source code, internal crate documentation, release workflows, release scripts, and release configuration.

The public distribution repositories are separate repositories. They are not subdirectories of `verzly/toolchain`.

In handoff ZIP files, a sibling `_repos/` directory may be included next to `toolchain/`. That directory is a convenience export of the other repositories, not part of the toolchain project.

Correct handoff ZIP layout:

```text
toolchain.zip
├── toolchain/                         # The actual verzly/toolchain repository
│   ├── .github/workflows/
│   ├── crates/
│   ├── scripts/
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
```

Never place `_repos/` or `distribution/` inside the actual `verzly/toolchain` repository.

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

Do not use `version.workspace = true` for independently released public tools. The tools are released independently, so their versions must be independent.

Shared workspace metadata may still be used for edition, license, repository URL, authors, lint policy, and dependency versions when appropriate.

## Release configuration

Each public tool owns one release config:

```text
crates/github-release/github-release.toml
crates/cargo-release/github-release.toml
crates/tauri-release/github-release.toml
crates/rust-cache/github-release.toml
crates/android-signing/github-release.toml
```

These configs must stay in the source repository and must not be copied to distribution repositories.

The config should model two repositories:

```toml
source_repository = "verzly/toolchain"
target_repository = "verzly/cargo-release"
source_tag_prefix = "cargo-release-v"
tag_prefix = "v"
```

The source tag identifies the monorepo tool. The public tag stays clean inside the public distribution repository.

## Release lifecycle

A release workflow must perform source release work before public distribution release work.

Expected flow:

1. Create a temporary source release branch in `verzly/toolchain`.
2. Update `crates/<tool>/Cargo.toml` to the requested version on that branch.
3. Run formatting, clippy, and tests from that branch.
4. Build artifacts from that exact branch.
5. If anything fails, delete the temporary source release branch.
6. If everything succeeds, merge the branch into `master`.
7. Create the source tag in `verzly/toolchain`, for example `cargo-release-v1.2.3`.
8. Clone the target public repository.
9. Run `github-release prepare` in the target public repository using the absolute config path from `crates/<tool>/github-release.toml`.
10. Run `github-release finalize` in the target public repository with the executable assets.
11. Create the public tag, for example `v1.2.3`.
12. Publish the public GitHub Release.

The source tag must exist before public release notes are generated. Pull request links in public release notes should point to `verzly/toolchain`, because that is where the actual code changes live.

## Distribution file syncing

CI releases do not require `_repos/` because `_repos/` is not part of `verzly/toolchain`.

If local/manual syncing is needed from the handoff ZIP, use the helper script with an explicit external content root:

```sh
cd toolchain
DISTRIBUTION_REPO_CONTENT_ROOT=../_repos \
  scripts/sync-repo-template.sh cargo-release ../cargo-release
```

Do not make GitHub Actions depend on a sibling directory that cannot exist after checking out `verzly/toolchain` alone.

## CI expectations

Release workflows expect a token that can push to both `verzly/toolchain` and the target public repository. The expected secret name is `DISTRIBUTION_REPO_TOKEN`.

Each public tool has its own workflow:

```text
.github/workflows/release-github-release.yml
.github/workflows/release-cargo-release.yml
.github/workflows/release-tauri-release.yml
.github/workflows/release-rust-cache.yml
.github/workflows/release-android-signing.yml
```

There is one normal workspace test workflow:

```text
.github/workflows/test.yml
```

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

Public README files should be human, concise, and usage-oriented. They should explain what the tool does, when to use it, and show practical GitHub Actions examples.

The root `toolchain/README.md` is for maintainers. Crate-level READMEs are for internal development. Public distribution READMEs live outside the project in the handoff `_repos/` export.

Do not add `CHANGELOG.md` or `VERSION` files unless explicitly requested. Release notes are generated from GitHub releases.

## Validation checklist

Before handing over a ZIP, verify:

```sh
test -d toolchain
test -d _repos
test ! -d toolchain/_repos
test ! -d toolchain/distribution

for tool in github-release cargo-release tauri-release rust-cache android-signing; do
  test -f "toolchain/crates/${tool}/Cargo.toml"
  test -f "toolchain/crates/${tool}/github-release.toml"
  grep -q 'source_repository = "verzly/toolchain"' "toolchain/crates/${tool}/github-release.toml"
  grep -q "target_repository = \"verzly/${tool}\"" "toolchain/crates/${tool}/github-release.toml"

  test -f "_repos/${tool}/README.md"
  test -f "_repos/${tool}/action.yml"
  test -f "_repos/${tool}/LICENSE"
  test ! -d "_repos/${tool}/.github"
  test ! -f "_repos/${tool}/Cargo.toml"
  test ! -d "_repos/${tool}/src"
  test ! -f "_repos/${tool}/github-release.toml"
done
```

Also run, when Rust is available:

```sh
cd toolchain
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

## Forbidden changes

Do not add `_repos/` inside `verzly/toolchain`.

Do not add `distribution/` inside `verzly/toolchain`.

Do not add Rust source code to public distribution repositories.

Do not publish public executable assets from `verzly/toolchain` for end users to consume. Public users should consume releases from the distribution repositories.

Do not make public release notes point to distribution repositories when the merged pull requests live in `verzly/toolchain`.

Do not use one global version for independently released public tools.
