# AI Agent Guide for Verzly Toolchain

This guide defines the intended architecture, repository boundaries, release model, and maintenance rules for AI agents working on the Verzly toolchain ecosystem.

## Primary rule

`verzly/toolchain` is the private Rust workspace. It contains source code, release workflows, release configuration, and the committed `.codex/distributions/<tool>` public distribution templates. Crate-level README files are intentionally not used; internal context belongs in the root README and this guide.

The public distribution repositories are separate GitHub repositories. Their public `README.md`, `action.yml`, and `LICENSE` files are maintained in `.codex/distributions/<tool>` so AI agents can update the public documentation and action surface from the same workspace. The `sync-distributions.yml` workflow can push those templates into the matching `verzly/<tool>` repositories with `DISTRIBUTION_REPO_TOKEN`.

Correct repository layout:

```text
verzly/toolchain/
├── .github/workflows/
├── .cargo/config.toml
├── .codex/
│   └── distributions/
│       ├── github-release/
│       ├── cargo-release/
│       ├── tauri-release/
│       ├── rust-cache/
│       └── android-signing/
├── crates/
├── Cargo.toml
├── Cargo.lock
├── LICENSE
├── README.md
├── CONTRIBUTING.md
└── AGENTS.md
```

Incorrect repository layout:

```text
verzly/toolchain/distribution/
verzly/toolchain/scripts/
verzly/toolchain/crates/<tool>/README.md
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
| `toolchain` | repository root | `verzly/toolchain` | `vX.Y.Z` | `vX.Y.Z` |

Do not add a vague shared `verzly-core` crate by default. Shared internal crates are allowed only when multiple tools actively use the same behavior and the crate has a narrow, descriptive responsibility.

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

## Distribution templates

`.codex/distributions/<tool>` mirrors the intended committed content of the corresponding public repository:

```text
.codex/distributions/github-release/README.md
.codex/distributions/github-release/action.yml
.codex/distributions/github-release/LICENSE
```

Each template directory may contain only `README.md`, `action.yml`, and `LICENSE`. Do not put Rust source, release configs, workflow files, changelogs, or generated assets in these directories.

Use `.github/workflows/sync-distributions.yml` to copy these templates into the public `verzly/<tool>` repositories. The workflow must use `DISTRIBUTION_REPO_TOKEN`, must fail if that token cannot push to a target repository, and must commit with the configured maintainer commit message such as `chore(distribution): bump public surface`.

## Source versioning

Each public crate must have its own version in its own `Cargo.toml`:

```toml
[package]
name = "cargo-release"
version = "0.1.0"
```

Do not use `version.workspace = true` for independently released public tools.

## Release configuration

Each public tool owns two important configs:

```text
crates/<tool>/github-release.toml  # source branch/tag + Cargo.toml version update + public distribution release
crates/<tool>/cargo-release.toml   # executable asset build
```

The toolchain repository itself owns one root release config:

```text
github-release.toml                # toolchain tag + GitHub Release, no assets
```

Do not add `source-github-release.toml`. One `github-release.toml` per crate is enough. It must contain both release contexts:

```toml
[release]
tag_prefix = "v"

[source_release]
tag_prefix = "cargo-release-v"
name_prefix = "cargo-release v"
latest = false

[github]
target_repository = "verzly/cargo-release"
source_repository = "verzly/toolchain"
source_tag_prefix = "cargo-release-v"

[github.notes]
mode = "scoped"
include_scopes = ["cargo-release", "all"]
include_paths = ["crates/cargo-release/"]

[[files]]
path = "crates/cargo-release/Cargo.toml"
kind = "toml"
key = "package.version"
value = "{version}"
```

`github-release` must not guess or auto-discover version files in this monorepo. Every file that needs a version bump must be listed in `[[files]]`. Prepare/finalize commands use `[source_release]` and `[[files]]`; publish uses `[release]` and `[github]`.

These configs must stay in the source repository and must not be copied to distribution repositories.


## Cargo cache policy

The workspace must keep native Cargo build output under `.cache` without requiring wrapper commands. The committed config is:

```toml
# .cargo/config.toml
[build]
target-dir = ".cache/rust/packages/toolchain/target"
```

The root `rust-cache.toml` is the policy source used by `rust-cache init` to generate or repair that Cargo config. Do not put `rust-cache.toml` inside `crates/rust-cache/`; this is a workspace-level policy, not a crate-local fixture. Workflows should call plain `cargo fmt`, `cargo clippy`, `cargo test`, and `cargo build`; do not wrap normal Cargo commands in `rust-cache run`.

`rust-cache run` and `rust-cache env` are reserved for environment-only cache values that Cargo cannot read from `.cargo/config.toml`, such as optional `CARGO_HOME` and `GRADLE_USER_HOME` routing.

## Release lifecycle

A release workflow must perform source release work before public distribution release work.

Expected flow:

1. `github-release prepare` creates a temporary source release branch in `verzly/toolchain`.
2. `github-release prepare` updates `crates/<tool>/Cargo.toml` to the requested version on that branch.
3. Plain `cargo fmt`, `cargo clippy`, and `cargo test` run from that exact branch. Native `.cargo/config.toml` routing keeps build output under `.cache`.
4. `cargo-release build` builds executable assets from that exact branch.
5. `github-release abort` deletes the temporary source release branch if anything fails.
6. `github-release finalize --skip-github-release` merges the branch into `master` and creates `<tool>-vX.Y.Z`.
7. `github-release publish` creates `vX.Y.Z` in the public distribution repository, generates notes from `verzly/toolchain`, and uploads assets.

The source tag must exist before public release notes are generated. Pull request links in public release notes should point to `verzly/toolchain`, because that is where the actual code changes live.

A central `.github/workflows/release-all.yml` workflow must exist for releasing all public tools and the toolchain with one version input. It must stay readable as a visible two-phase graph: preflight, prepare every source release branch, test prepared branches, build `cargo-release`, build the other executable assets with that freshly built `cargo-release`, finalize all source branches and tags, publish all public distribution releases with the already-built assets, then publish the toolchain release.

A `.github/workflows/release-toolchain.yml` workflow must exist for publishing a toolchain-only release. It should create a `vX.Y.Z` tag and GitHub Release in `verzly/toolchain` without executable assets.

## Commit and PR title scopes for release notes

Package-specific release notes depend on consistent Conventional Commit scopes. AI agents and maintainers must use these scopes in commit messages and PR titles, especially when squash-merging PRs:

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

Use source-maintenance scopes for changes that should appear in the toolchain release but should not be copied into every package's public release notes:

```text
ci(toolchain): tighten repository model checks
docs(toolchain): clarify monorepo release policy
chore(deps): update Rust dependencies
refactor(workspace): remove unused shared crate
```

If a PR changes multiple packages in a meaningful user-facing way, prefer splitting it by package. If it must stay together, use `all` only when every package release should mention the change.

Package public release notes include a commit when either the commit/PR title has the package scope or the changed files are under the package path configured in `crates/<tool>/github-release.toml`. The root toolchain release can contain mixed PRs and commits.

## Dependency maintenance

Dependency upgrades must be intentional and separate from formatting, workflow, and README-only changes.

Use compatible patch/minor updates freely when they do not require source changes. Treat major version changes as code changes: update the source, check migration notes, and run the full CI locally or in GitHub Actions.

Do not keep unused dependencies. If a dependency is not used by the source code, remove it instead of upgrading it.

When upgrading common dependencies across the workspace, prefer one dedicated commit such as:

```text
chore(deps): update Rust dependencies
```

Record any required source migration in the commit body.


## Mandatory verification loop

Before returning a modified ZIP, an AI agent must verify the result and continue editing until the relevant checks pass. Do not rely on visual inspection when a deterministic check is available.

For Rust source changes, run these checks from `toolchain/` whenever the environment provides Rust/Cargo:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For workflow, configuration, and repository-boundary changes, also validate the non-Rust structure:

```bash
# Parse all TOML files.
python - <<'PY'
from pathlib import Path
import tomllib
for path in Path('.').rglob('*.toml'):
    tomllib.loads(path.read_text())
PY

# Parse all GitHub workflow YAML files with a YAML parser when available.
# Verify the repository model and committed distribution templates.
test -d .codex/distributions
test ! -d distribution
test ! -d scripts
```

If the local environment does not have Rust/Cargo, the agent must first try to use or install the required toolchain when possible. If that is impossible, the agent must say clearly that `cargo fmt`, `cargo clippy`, or `cargo test` could not be executed locally, run every available static check instead, and avoid claiming that the Rust checks passed.

When the user provides a CI failure log, the agent must update the source exactly according to the failing check, re-run the relevant local checks when possible, and produce a new ZIP only after the checked state is internally consistent. Repeated CI failures from the same class, such as `cargo fmt --check`, mean the agent must broaden verification to the whole tree instead of patching only the last visible diff.

## Testing expectations

Every Rust crate must have meaningful tests. A green `cargo test --workspace --all-targets` must not mean only that the workspace compiles.

Each crate should include tests for the behavior it owns:

- `github-release`: release plan generation, SemVer validation, prerelease detection, tag/name rendering, version file updates, scoped release-note filtering, and destructive-operation safety rules.
- `cargo-release`: config defaults, target selection, artifact discovery, artifact naming, checksum writing, manifest writing, and missing-artifact failures.
- `tauri-release`: platform defaults, platform artifact discovery, checksum writing, output cleanup, and platform strategy behavior.
- `rust-cache`: default config loading, native `.cargo/config.toml` generation, explicit package cache paths, conflict-safe target-dir updates, optional `CARGO_HOME`, optional `GRADLE_USER_HOME`, and clean/env/run planning behavior.
- `android-signing`: base64 export, generated password shape, CLI defaults, GitHub env writing behavior, and secret redaction rules.

Prefer unit tests for pure planning, config, path, and rendering behavior. Use integration-style tests only when command boundaries or filesystem behavior are the point of the test. Tests must avoid requiring Docker, Podman, Android SDK, Tauri, `gh`, or real signing keys unless the test is explicitly ignored or guarded.

Do not remove tests to make CI pass. Fix the implementation or update the test when the desired behavior intentionally changes. Any new command, config field, release-note rule, or path-routing rule should include a test in the same change.

## CI expectations

Release workflows must use `github.token` for source-repository operations in `verzly/toolchain`. Public tool releases must require `DISTRIBUTION_REPO_TOKEN` because they write release data to separate distribution repositories such as `verzly/cargo-release`. Public repository visibility does not remove the need for authenticated write access when creating tags, creating releases, or uploading assets.

Do not fall back from `DISTRIBUTION_REPO_TOKEN` to `github.token` for public distribution publishing. `github.token` is scoped to `verzly/toolchain` and cannot reliably write to `verzly/<tool>` repositories. Fail early with a clear preflight error if the distribution token is missing or lacks push permission.

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

The repository must also contain these maintainer workflows:

```text
.github/workflows/release-toolchain.yml       # publish the private/source repo release, no assets
.github/workflows/_release-toolchain.yml      # reusable toolchain release workflow
.github/workflows/release-all.yml             # prepare/build everything first, then finalize/publish releases
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

Public README files should be human, usage-oriented, and complete enough for developers who have never seen `verzly/toolchain`. They must use a structured multi-level menu with planned main sections and subsections, not a single flat list. They must explain what the tool does, why it exists, how it works, practical use cases, GitHub Action examples, every action input, every action output, every CLI command, every CLI argument, accepted values, defaults, and important configuration fields.

The public README is the product documentation for the distribution repository. The root `toolchain/README.md` is for maintainers. Do not add crate-level READMEs; public distribution READMEs live in `.codex/distributions/<tool>/README.md` and are synchronized into the separate public repositories.

Do not add `CHANGELOG.md` or `VERSION` files unless explicitly requested. Release notes are generated from GitHub releases.

## README writing standard

Public distribution README files must follow a structured, readable documentation style. The navigation must appear directly after the introduction without a separate `## Contents` heading. It must be intentionally grouped into main sections and subsections, and it must describe the reader journey instead of mirroring every heading mechanically. Do not write a single flat list of every heading.

Use this default structure for public distribution repositories:

```markdown
- [Overview](#overview)
  - [Why this exists](#why-this-exists)
  - [How it works](#how-it-works)
  - [Use cases](#use-cases)
- [Get started](#get-started)
  - [GitHub Action](#github-action)
- [Usage](#usage)
  - [Action inputs](#action-inputs)
  - [Action outputs](#action-outputs)
  - [CLI usage](#cli-usage)
  - [CLI commands and arguments](#cli-commands-and-arguments)
- [Configuration](#configuration)
- [Practical workflows](#practical-workflows)
  - [Practical release/build/cache/signing workflows](#practical-workflows)
- [Reference](#reference)
  - [Troubleshooting](#troubleshooting)
  - [Release artifacts](#release-artifacts)
  - [Operational notes](#operational-notes)
- [Contributing](#contributing)
```

If a tool does not have a TOML configuration file, omit the `Configuration` group. If a tool has security-specific behavior, replace `Operational notes` with `Security notes`.

The README must include a `License` section at the end, but the internal menu must intentionally omit it. Do not add a `## Contents` heading above the menu. This matches the preferred README convention: the license is present in the document, but it is not part of the navigation menu.

Required public README content:

1. A short product introduction after the title.
2. A repository-boundary paragraph explaining that the public repository is a distribution surface and source lives in `verzly/toolchain`.
3. `Overview`, with `Why this exists`, `How it works`, and `Use cases`.
4. `Get started`, with a minimal GitHub Action example and an install-only example when useful.
5. `Usage`, with every GitHub Action input, every GitHub Action output, CLI usage examples, every CLI command, and every CLI argument.
6. `Configuration`, when the tool has a TOML config, including a realistic example and a field table.
7. `Practical workflows`, with real copy-pasteable workflows for common situations.
8. `Reference`, with troubleshooting, release artifacts, and operational/security notes.
9. `Contributing`, limited to 2-3 short sentences that point readers to `CONTRIBUTING.md` and explain that source changes happen in `verzly/toolchain`.
10. `License`, after contributing, omitted from the menu.

Rules for argument documentation:

- Every GitHub Action input must document required status, default, accepted values, and purpose.
- Every GitHub Action output must document value and purpose.
- Every CLI command must have at least one example.
- Every CLI argument must document required status, default, accepted values, and purpose.
- Boolean workflow/action values should be documented as strings when GitHub Actions treats them as strings, for example `"true"` and `"false"`.
- Config fields must document accepted values and why the field exists, not only repeat the field name.

Tone and structure:

- Write natural professional English.
- Prefer concrete examples over abstract claims.
- Explain why the tool exists before explaining every flag.
- Keep documentation clear for first-time users and useful for senior developers.
- Avoid marketing filler, emojis, and vague claims.
- Do not expose private implementation details except where needed to explain the public distribution repository boundary or source release-note origin.
- Do not add extra contribution policy, development process, code of conduct, governance, support, or maintainer sections to README files. Keep contribution details in `CONTRIBUTING.md`; the README should only contain the short `Contributing` pointer section.

## Hard no list

Do not add `distribution/` or release orchestration `scripts/` inside `verzly/toolchain`. Keep public repository templates in `.codex/distributions/<tool>` only.

Do not put source code in public distribution repositories.

Do not make public distribution repositories responsible for testing, building, or releasing themselves.

Do not make workflows depend on files outside the checked-out `verzly/toolchain` repository.

Release All must not dispatch separate workflow runs and must not publish each tool in a full prepare/build/finalize/publish wave before the next tool starts. It should show the complete release graph in one run, finish every prepare/test/build job first, and publish releases only after all assets exist.
