# AI Agent Guide for Verzly Toolchain

This file is the root instruction file for AI assistants working on `verzly/toolchain`.
It should be understandable by Codex, ChatGPT, Claude, and similar assistants, including
when the repository is provided as a ZIP.

Keep this guide practical. It defines repository boundaries, release rules, documentation
rules, and verification expectations. Do not create extra per-tool agent instruction files.

## Communication

- If the user writes in Hungarian, answer in Hungarian by default.
- Write issue titles, PR descriptions, commit messages, changelogs, release notes, docs,
  code comments, and product copy in natural professional English unless the user asks for
  Hungarian.
- For translations, rewrite naturally for software teams. Do not translate word-for-word.
- Keep status updates and final summaries concise and concrete.

## Repository Model

`verzly/toolchain` is the private Rust workspace and source of truth. It contains:

- Rust source code in `crates/`.
- Release workflows in `.github/workflows/`.
- Workspace release and quality configuration in `datarose.toml`.
- Public distribution templates in `.codex/distributions/<tool>/`.
- The only AI instruction file for the whole project: this root `AGENTS.md`.

Public distribution repositories such as `verzly/github-release` and
`verzly/repository` are thin distribution surfaces. Their public files are maintained
from `.codex/distributions/<tool>/` and synced by workflow.

Correct distribution template contents:

```text
.codex/distributions/<tool>/README.md
.codex/distributions/<tool>/CONTRIBUTING.md
.codex/distributions/<tool>/action.yml
.codex/distributions/<tool>/LICENSE
```

Do not add any other files to distribution templates:

```text
AGENTS.md
CLAUDE.md
Cargo.toml
Cargo.lock
src/
crates/
.github/
datarose.toml
CHANGELOG.md
VERSION
scripts/
generated assets
```

Also do not add these paths to the toolchain repository:

```text
distribution/
scripts/
crates/<tool>/README.md
```

Crate-level README files are intentionally not used. Internal context belongs in the
root `README.md` and this `AGENTS.md`; public product documentation belongs in
`.codex/distributions/<tool>/README.md`.

## Tools and Repositories

| Crate | Source path | Public repository | Source tag | Public tag |
| --- | --- | --- | --- | --- |
| `github-release` | `crates/github-release` | `verzly/github-release` | `github-release-vX.Y.Z` | `vX.Y.Z` |
| `cargo-release` | `crates/cargo-release` | `verzly/cargo-release` | `cargo-release-vX.Y.Z` | `vX.Y.Z` |
| `tauri-release` | `crates/tauri-release` | `verzly/tauri-release` | `tauri-release-vX.Y.Z` | `vX.Y.Z` |
| `rust-cache` | `crates/rust-cache` | `verzly/rust-cache` | `rust-cache-vX.Y.Z` | `vX.Y.Z` |
| `android-signing` | `crates/android-signing` | `verzly/android-signing` | `android-signing-vX.Y.Z` | `vX.Y.Z` |
| `repository` | `crates/repository` | `verzly/repository` | `repository-vX.Y.Z` | `vX.Y.Z` |
| `toolchain` | repository root | `verzly/toolchain` | `vX.Y.Z` | `vX.Y.Z` |

Do not add a vague shared `verzly-core` crate by default. Shared internal crates are
allowed only when several tools actively use the same behavior and the crate has a
small, named responsibility.

## Rust Architecture

Prefer small explicit modules over broad frameworks. Match existing crate patterns.

Typical boundaries:

- `cli.rs`: command-line shape and typed arguments.
- `commands/*`: command orchestration and user-visible behavior.
- `config.rs`: config loading, validation, and defaults.
- `domain.rs` or specific domain modules: pure rules and value objects.
- `git.rs`, `github.rs`, `process.rs`, `shell.rs`: boundary integrations.
- `output.rs`: user-visible formatting.

Avoid mixing CLI parsing, filesystem mutation, subprocess execution, and GitHub calls
inside the same function. Keep behavior testable with unit tests for pure planning,
config parsing, path handling, rendering, and safety rules.

## Release Configuration

The root `datarose.toml` is the release configuration source. Do not add per-crate
`datarose.toml` files.

Each public tool has its own version in its own `Cargo.toml`. Do not use
`version.workspace = true` for independently released public tools.

Every public distribution target in `datarose.toml` should declare:

- target name;
- public repository;
- distribution template path;
- Cargo package/binary when applicable;
- exact version file and key;
- source tag prefix;
- release-note scopes and source paths.

`github-release` must not guess version files in this monorepo. The selected release
target must explicitly declare the file to update.

## Release Lifecycle

Release workflows must perform source release work before public distribution release
work:

1. `github-release prepare` creates a temporary source release branch.
2. The selected crate version is updated on that branch.
3. Formatting, clippy, and tests run from that exact branch.
4. `cargo-release build` builds executable assets from that branch.
5. Failures call `github-release abort`.
6. `github-release finalize --merge-strategy squash --skip-github-release` merges to
   `master` and creates the package-prefixed source tag.
7. `sync-distributions.yml` syncs only the relevant public distribution repository and
   creates a release-specific distribution bump commit.
8. `github-release publish` creates `vX.Y.Z` in the public repository, uploads assets,
   generates notes from `verzly/toolchain`, and refreshes configured moving tags.

Public release notes should point back to `verzly/toolchain` for source changes. Do not
render raw PR URLs in notes. Same-repository PRs should appear as `#123`; external PRs
should use a readable repository prefix such as `toolchain#123`.

## Workflows and Tokens

Use `github.token` for source repository operations in `verzly/toolchain`.

Use `DISTRIBUTION_REPO_TOKEN` for anything that writes to public distribution
repositories: pushing template files, creating public tags, creating releases, uploading
assets, deleting public releases, or repairing public moving tags.

Do not fall back from `DISTRIBUTION_REPO_TOKEN` to `github.token` for public repository
writes. Fail early with a clear preflight error when the distribution token is missing
or lacks push permission.

Required maintainer workflows:

```text
.github/workflows/test.yml
.github/workflows/release-<tool>.yml
.github/workflows/release-all.yml
.github/workflows/_release-tool.yml
.github/workflows/_release-build-assets.yml
.github/workflows/release-toolchain.yml
.github/workflows/_release-toolchain.yml
.github/workflows/sync-distributions.yml
.github/workflows/delete-release.yml
.github/workflows/update-floating-tags.yml
```

Do not add test or release workflows to public distribution repositories.

## Distribution Documentation

Distribution README files are public product documentation. They should focus on usage,
not internal development process.

Each public distribution README should include:

- short product introduction;
- repository-boundary paragraph explaining that source lives in `verzly/toolchain`;
- grouped navigation directly after the introduction, without a `## Contents` heading;
- `Overview` with why it exists, how it works, and use cases;
- `Get started` with GitHub Action examples;
- `Usage` with every action input, action output, CLI command, and CLI argument;
- `Configuration` when the tool has TOML/config fields;
- practical copy-pasteable workflows;
- `Reference` with troubleshooting, release artifacts, and operational or security notes;
- `License` at the end, omitted from the navigation menu.

Do not add a `Contributing` section to README files. Contribution and development
process details belong in distribution `CONTRIBUTING.md` files.

Every GitHub Action input must document required status, default, accepted values, and
purpose. Every CLI argument should do the same. Boolean GitHub Action values should be
documented as strings, for example `"true"` and `"false"`.

## Repository Tool Expectations

The `repository` crate owns repository standards bootstrap behavior. It should:

- detect Rust, PHP, JavaScript, TypeScript, and Vue usage from marker files and source
  files;
- generate quality files and test workflows from `datarose.toml`;
- manage release targets through flag-based CLI commands;
- provide TUI flows only as convenience wrappers around the same CLI behavior;
- keep all operations scriptable for CI and automation.

Do not make the TUI the only way to perform an operation.

## Cargo Cache Policy

Cargo build output should stay under `.cache` through native Cargo configuration:

```toml
[build]
target-dir = ".cache/rust/packages/toolchain/target"
```

Workflows should call plain Cargo commands:

```text
cargo fmt
cargo clippy
cargo test
cargo build
```

Do not wrap normal Cargo commands in `rust-cache run`. Use `rust-cache run` and
`rust-cache env` only for environment variables Cargo cannot read from `.cargo/config.toml`.

## Tests and Verification

Before returning modified code, run the relevant checks and keep editing until they pass.

For Rust source changes, run:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For workflow, config, documentation, or repository-boundary changes, also run the
available static checks:

```bash
git diff --check
test -d .codex/distributions
test ! -d distribution
test ! -d scripts
find .codex/distributions -mindepth 2 -maxdepth 2 -type f \
  ! -name README.md \
  ! -name CONTRIBUTING.md \
  ! -name action.yml \
  ! -name LICENSE \
  -print
```

Parse TOML and YAML files with available local parsers when the environment provides
them. If the local environment lacks a parser or Rust/Cargo, say exactly which checks
could not be run and run the remaining deterministic checks.

Every crate should keep meaningful tests for the behavior it owns. Do not remove tests
to make CI pass. Fix the implementation or update tests when desired behavior changes.

## Commit, PR, and Release Note Style

Use Conventional Commits 1.0.0:

```text
feat(github-release): add scoped release notes
fix(cargo-release): correct artifact naming
docs(tauri-release): expand Android build documentation
chore(rust-cache): simplify workspace detection
fix(android-signing): avoid printing signing passwords
```

Use `all` only when a change should appear in every public package release note:

```text
chore(all): update shared release workflow behavior
```

Use source-maintenance scopes for internal changes that should not be copied into every
package's public release notes:

```text
ci(toolchain): tighten repository model checks
docs(toolchain): clarify monorepo release policy
chore(deps): update Rust dependencies
refactor(workspace): remove unused shared crate
```

Write changelogs and release notes in Keep a Changelog style. Group human-readable
changes under `Added`, `Changed`, `Removed`, `Fixed`, and `Security` when applicable.

## Dependency Maintenance

Dependency upgrades should be intentional and separate from formatting, workflow, and
README-only changes.

- Patch/minor updates are acceptable when they do not require source changes.
- Major updates are code changes: check migration notes, update source, and run full CI.
- Remove unused dependencies instead of upgrading them.
- Prefer a dedicated `chore(deps): update Rust dependencies` commit for shared updates.

## Security

- Do not print secrets.
- Do not write signing passwords or GitHub tokens to logs.
- Android signing commands may output GitHub Actions secret names and encoded values only
  when the command is explicitly designed for that purpose.
- Do not run arbitrary shell fragments from config unless that feature is explicit and
  reviewed.
- Prefer explicit allowlists for generated artifact paths and uploaded files.

## Editing Rules for AI Assistants

- Read the relevant files before editing.
- Keep edits scoped to the user request.
- Preserve existing repository patterns.
- Do not revert unrelated user changes.
- Prefer `rg` for searching when available.
- Use structured parsers for structured files when practical.
- Use `apply_patch` for manual file edits.
- Do not add generated files, broad rewrites, new frameworks, or automation unless they
  are needed for the task.

When finishing, state the changed files, why they changed, and which checks passed or
could not be run.
