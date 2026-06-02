# Verzly Toolchain

Verzly Toolchain is the private source workspace for the release tools around Rust builds, Tauri builds, Android signing, local cache routing, and GitHub Releases.

The public repositories stay separate because they are the names users install and reference from GitHub Actions. The source code stays here because the tools share the same lower-level problems: process execution, platform naming, artifact naming, checksum files, release branches, and GitHub Release publishing.

## Contents

- [Projects](#projects)
- [Repository model](#repository-model)
- [Distribution releases](#distribution-releases)
- [Release notes](#release-notes)
- [Development](#development)
- [Contributing](#contributing)

## Projects

`github-release` prepares release branches, updates version files, finalizes successful builds, creates tags, and publishes GitHub Releases.

`cargo-release` builds Rust executable artifacts and writes release output into `dist/`.

`tauri-release` coordinates Tauri desktop and mobile release artifacts while keeping platform boundaries explicit.

`rust-cache` routes Rust and Tauri build cache into a project-local cache directory.

`android-signing` helps prepare and inspect Android signing material for release builds.

`verzly-core` contains shared helpers that should not drift between the tools.

## Repository model

This repository is the source of truth.

The distribution repositories are intentionally small:

```text
verzly/github-release
verzly/cargo-release
verzly/tauri-release
verzly/rust-cache
verzly/android-signing
```

They contain project documentation, `action.yml`, release workflow entry points, license files, and their own GitHub Release assets. They do not duplicate the Rust source code.

The distribution files are maintained under `distribution/<tool>` and are synced to the public repositories during release.

## Distribution releases

The release workflow in this repository can publish every distribution repository by itself. It builds the standalone executable assets, syncs the distribution repository files, creates the release branch in that repository, publishes the GitHub Release there, and uploads the executable assets to that repository's own release.

The workflow bootstraps the local release tools first:

```text
cargo build --release -p cargo-release -p github-release
```

After that, `cargo-release` builds each public executable, and `github-release` publishes each distribution repository.

The public action files always download assets from their own repository release. For example, `verzly/cargo-release/action.yml` downloads from `verzly/cargo-release`, not from `verzly/toolchain`.

## Release notes

The source changes and pull requests live here, even when the final GitHub Release is published from a distribution repository.

For that reason the distribution release configs set `source_repository = "verzly/toolchain"`. When `github-release` creates a public release, it generates the notes from the source repository tag, then publishes those notes to the distribution repository release. Pull request links in the generated notes intentionally point back to `verzly/toolchain`, because that is where the review history exists.

Each tool uses a source tag with its own prefix:

```text
github-release-v1.2.3
cargo-release-v1.2.3
tauri-release-v1.2.3
rust-cache-v1.2.3
android-signing-v1.2.3
```

The matching distribution repository receives the public `v1.2.3` tag and executable assets.

## Development

Run the usual Rust checks from the workspace root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

During local development, the tools can be run directly from the workspace:

```sh
cargo run -p github-release -- plan --version 0.1.0
cargo run -p cargo-release -- plan
cargo run -p rust-cache -- doctor
```

The workspace is intentionally plain. Shared code should make the tools easier to maintain, not hide the flow behind a framework.

## Contributing

Changes should keep the command boundaries clear. A tool should not silently take over another tool's job. For example, `cargo-release` builds artifacts, while `rust-cache` owns cache routing.

When adding shared behavior, prefer putting it in `verzly-core` only when at least two tools actually need it. Shared code should reduce drift, not create a new abstraction layer for its own sake.

## License

This project is licensed under the AGPL-3.0-only license.
