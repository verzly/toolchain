# Verzly Toolchain

Verzly Toolchain is the shared source workspace for the release tools I use around Rust, Tauri, Android signing, local cache routing, and GitHub Releases.

The projects are kept together because they share the same problems: process execution, platform naming, artifact handling, checksums, release branches, and GitHub Release publishing. Keeping that code in one place makes the tools easier to maintain without turning each public repository into a copy of the same infrastructure.

The individual public repositories are still kept separate. They provide the project-specific README, GitHub Action metadata, release artifacts, and issue surface for each tool.

## Contents

- [Projects](#projects)
- [Repository model](#repository-model)
- [Development](#development)
- [Releases](#releases)
- [Contributing](#contributing)

## Projects

`github-release` prepares release branches, updates version files, finalizes successful builds, creates tags, and publishes GitHub Releases.

`cargo-release` builds Rust executable artifacts and writes release output into `dist/`.

`tauri-release` coordinates Tauri desktop and mobile release artifacts while keeping platform boundaries explicit.

`rust-cache` routes Rust and Tauri build cache into a project-local cache directory.

`android-signing` helps prepare and inspect Android signing material for release builds.

`verzly-core` contains shared helpers that should not drift between the tools.

## Repository model

The source code lives here. The public-facing repositories live separately:

```text
verzly/github-release
verzly/cargo-release
verzly/tauri-release
verzly/rust-cache
verzly/android-signing
```

Those repositories are distribution repositories. They do not need to duplicate the Rust source. They point back to this workspace, expose a stable GitHub Action interface, and publish their own executable release artifacts.

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

The workspace is intentionally plain. There is no generated project structure and no hidden build framework.

## Releases

The release workflow can publish this workspace as a source-level release. The distribution repositories are responsible for the public executable releases of each individual tool.

The intended chain is simple:

```text
github-release prepares and publishes the release
cargo-release builds executable artifacts
rust-cache keeps build output away from the repository root
android-signing supports Android release preparation where needed
```

The first release can be bootstrapped manually. After that, the tools should release themselves through the same workflow they provide to users.

## Contributing

Changes should keep the command boundaries clear. A tool should not silently take over another tool's job. For example, `cargo-release` builds artifacts, while `rust-cache` owns cache routing.

When adding shared behavior, prefer putting it in `verzly-core` only when at least two tools actually need it. Shared code should reduce drift, not create an abstract framework.

## License

This project is licensed under the AGPL-3.0-only license.
