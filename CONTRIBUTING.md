# Contributing

Source changes for the Verzly toolchain happen in `verzly/toolchain`. Public distribution repositories are release surfaces only; their `README.md`, `CONTRIBUTING.md`, `action.yml`, and `LICENSE` files are maintained in `.codex/distributions/<tool>` and synchronized by workflow.

## Setup

Install the tools used by normal development:

```sh
rustup toolchain install stable
rustup default stable
cargo --version
git --version
gh --version
mise --version
```

Clone the source workspace and run checks from the repository root:

```sh
git clone git@github.com:verzly/toolchain.git
cd toolchain
mise install
hk install
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

The local `mise.toml` should include `hk`, `pkl`, and `rust@stable`. `repo-quality doctor` is expected to suggest missing language tools instead of silently assuming they are globally installed.

Build output is intentionally local to `.cache` through `.cargo/config.toml`. Do not wrap normal Cargo commands with `rust-cache run`.

## Development

Create a focused branch for every change:

```sh
git switch -c feat/github-release-custom-notes
```

Use package scopes in commits and PR titles when the change affects a public tool:

```text
feat(github-release): support custom release notes
fix(cargo-release): correct artifact naming
docs(rust-cache): clarify cache cleanup
chore(all): update shared release workflow behavior
```

Use source-maintenance scopes for workspace-only changes:

```text
ci(toolchain): tighten repository model checks
docs(toolchain): update maintainer workflow
chore(deps): update Rust dependencies
```

Run individual tools with `cargo run` while developing:

```sh
cargo run -p github-release -- plan --config crates/cargo-release/github-release.toml --version 1.2.3
cargo run -p cargo-release -- build --config crates/cargo-release/cargo-release.toml --version 1.2.3
cargo run -p rust-cache -- init
cargo run -p android-signing -- generate
cargo run -p repo-quality -- plan
```

A public release is not required for local testing. Cargo runs the current source directly:

```sh
cargo run -p repo-quality -- init --dry-run --skip-mise-use --skip-hk-install
cargo run -p repo-quality -- doctor
```

Build and run the local binary when you need to test the same executable path a release would expose:

```sh
cargo build -p repo-quality
.cache/rust/packages/toolchain/target/debug/repo-quality plan
```

On Windows:

```pwsh
.\.cache\rust\packages\toolchain\target\debug\repo-quality.exe plan
```

You can install the current source locally without publishing a release:

```sh
cargo install --path crates/repo-quality --force
repo-quality plan
```

For a safe self-hosting check, preview the generated model first:

```sh
cargo run -p repo-quality -- init --dry-run --skip-mise-use --skip-hk-install
```

When the preview is correct and you intentionally want to refresh the workspace hook model, run:

```sh
cargo run -p repo-quality -- init --force
mise exec -- hk check
```

Use `mise exec -- hk check` if your shell resolves an older global `hk` before the version managed by `mise`.

## Testing

Before opening or merging a PR, run:

```sh
hk check
```

This executes the same Rust formatting, clippy, and test gates configured in `hk.pkl`. The equivalent raw commands are:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

For workflow, config, and repository-boundary changes, also check the model guarded by `.github/workflows/test.yml`. Important invariants include:

```text
.codex/distributions/<tool> contains README.md, CONTRIBUTING.md, action.yml, LICENSE only
crates/<tool>/github-release.toml exists
crates/<tool>/cargo-release.toml exists
crates/<tool>/README.md does not exist
distribution/ and scripts/ do not exist
```

Prefer unit tests for planning, config, path handling, release note rendering, artifact discovery, cache routing, and signing behavior. Avoid tests that require Docker, Podman, Android SDK, Tauri, `gh`, or real signing keys unless they are explicitly guarded.

## GitHub Actions

The main checks run through `.github/workflows/test.yml` on pull requests. It runs Rust checks and repository-model checks.

Public tool release workflows are:

```text
.github/workflows/release-github-release.yml
.github/workflows/release-cargo-release.yml
.github/workflows/release-tauri-release.yml
.github/workflows/release-rust-cache.yml
.github/workflows/release-android-signing.yml
.github/workflows/release-repo-quality.yml
```

Maintainer workflows are:

```text
.github/workflows/release-all.yml
.github/workflows/release-toolchain.yml
.github/workflows/delete-release.yml
.github/workflows/update-floating-tags.yml
.github/workflows/sync-distributions.yml
```

Use `sync-distributions.yml` when only public README/CONTRIBUTING/action/LICENSE files need to be pushed to `verzly/<tool>` repositories. Use release workflows when tags, GitHub Releases, and assets should be created. Use `update-floating-tags.yml` to backfill or repair moving tags such as `vX.Y`, `vX`, `latest`, and `next` in public distribution repositories after releases already exist. Use `delete-release.yml` only for release cleanup; it uses the same no-prefix version input as release workflows, checks repository access first, removes the selected GitHub Release, and deletes the matching tag explicitly.

Generated release notes should compare against the previous full SemVer release tag from the same tag family. Do not use creation date ordering for notes ranges, because moving tags and repaired releases can make chronological tag order misleading.

## Production Tokens

Source repository operations use the workflow `github.token`.

Public repository publishing and distribution sync require `DISTRIBUTION_REPO_TOKEN`. The token must be able to push commits, create tags, create releases, and upload assets in the target `verzly/<tool>` repositories.

Do not replace `DISTRIBUTION_REPO_TOKEN` with `github.token` for public distribution repositories. The default token is scoped to `verzly/toolchain`.

## Release Branch Workflow

Make release-related changes on a normal branch first:

```sh
git switch -c release/prepare-0.1.0
```

Update code, configs, workflows, or `.codex/distributions` templates on that branch. Run the full local checks, open a PR, and merge it to `master`.

After the PR is on `master`, run the appropriate workflow:

```text
release-<tool>.yml       # one public tool
release-all.yml          # every public tool, then toolchain
release-toolchain.yml    # toolchain-only release
delete-release.yml       # destructive release and tag cleanup
sync-distributions.yml   # public README/CONTRIBUTING/action/LICENSE sync only
update-floating-tags.yml # moving tag repair for public repositories
```

Release workflows must be dispatched from `master`. They create their own temporary release branches, source tags, public distribution bump commits, public tags, GitHub Releases, and cleanup actions. Release All replaces a stale aggregate branch for the requested version before preparing a new run.

Single-tool releases squash-merge their temporary source branch back into `master`. Release All uses one aggregate `release/all-vX.Y.Z` branch for every public tool version bump and lockfile update, then squash-merges that branch into a single `master` commit before creating all package-prefixed source tags from the same commit. If a re-release has no source diff because `master` already contains the requested version, finalization skips the squash commit and tags the current `master` commit instead.

After all required builds pass and before public `vX.Y.Z` tags are created, release workflows run `sync-distributions.yml` only for the repositories being released. The sync uses a release-specific `chore(distribution)` commit message and forces a bump commit even when the public files are already up to date, so the public release tag lands on the release bump commit.

Public distribution release configs enable moving tags. Publishing `v1.2.3` updates `v1.2`, `v1`, and `latest`; publishing a preview such as `v1.3.0-rc.1` updates `next` when it is the highest preview. The root toolchain config keeps these disabled because the source repository should not receive moving distribution tags.

Public distribution actions must also support those moving refs at runtime. When a workflow uses `verzly/<tool>@latest`, `@next`, `@v1`, or `@v1.2`, the composite action should resolve the requested action ref to the concrete version release tag on the same commit, then download executable assets from that immutable release.

## Documentation

Root maintainer documentation belongs in `README.md`, `CONTRIBUTING.md`, and `AGENTS.md`.

Public user documentation belongs in `.codex/distributions/<tool>/README.md`. Public README files should explain usage, action inputs, action outputs, CLI commands, CLI arguments, config fields, practical workflows, troubleshooting, artifacts, operational notes, and license information. Keep contribution and development-process details in distribution `CONTRIBUTING.md` files.
