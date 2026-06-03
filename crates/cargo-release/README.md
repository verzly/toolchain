# verzly/cargo-release

`verzly/cargo-release` builds Rust executable artifacts in a repeatable release directory, with optional Docker or Podman isolation.

It exists for projects where `cargo build --release` is not enough anymore. Release builds need a known target list, a clean output directory, checksums, a manifest, and a way to avoid polluting the developer machine when builds run locally.

Use this tool when a Rust binary needs more than a plain `cargo build --release`. The config says which targets exist, how each target should be built, where artifacts are collected, and whether checksums and a manifest should be written. The build can run on the host machine or inside Docker/Podman when isolation is useful.

The tool does not publish releases, tag commits, or edit Cargo versions. Use `verzly/github-release` for that after the artifacts exist.

- [How it works](#how-it-works)
  - [Build strategies](#build-strategies)
  - [Targets](#targets)
  - [Artifacts](#artifacts)
  - [Cache boundaries](#cache-boundaries)
- [Get started](#get-started)
  - [Install](#install)
  - [Create config](#create-config)
  - [First build](#first-build)
- [Usage](#usage)
  - [Plan](#plan)
  - [Build](#build)
  - [Clean](#clean)
  - [Doctor](#doctor)
  - [Configuration](#configuration)
- [GitHub Actions](#github-actions)
- [Compatibility](#compatibility)
- [Known issues](#known-issues)
- [Contributing](#contributing)

Read on if you want to understand the boundaries. Jump to [Get started](#get-started) if you already have a Rust binary project.

## How it works

`cargo-release` reads `cargo-release.toml`, prepares an output directory, runs the configured target builds, then copies the selected artifacts into `dist/`.

The flow is deliberately simple:

```text
read config -> plan targets -> run builds -> collect artifacts -> write checksums -> write manifest
```

The tool does not publish releases, tag commits, or modify Cargo versions. That is the job of `verzly/github-release`.

### Build strategies

Each target can use one of these strategies:

```text
host       -> run cargo directly on the current machine
container  -> run the configured command inside Docker or Podman
auto       -> use container when configured, otherwise host
```

Container builds mount the project into `/workspace` and set the working directory there. That keeps the command readable and makes local and CI builds behave the same way.

### Targets

A target describes one release artifact family. A Linux target might build `x86_64-unknown-linux-gnu`. A Windows target might build `x86_64-pc-windows-msvc`. The target name is not magical; it is a configuration key that appears in logs and in the manifest.

### Artifacts

Artifacts are collected from glob patterns. The tool does not guess which files matter.

A typical Rust CLI project collects:

```text
target/*/release/my-tool
target/*/release/my-tool.exe
```

The collected files are copied into `dist/<target>/`.

### Cache boundaries

`cargo-release` does not redirect Cargo's build cache. Its responsibility is to run the configured builds and collect release artifacts into `dist/`.

If a project needs Cargo targets, Cargo home, Gradle files, or Tauri build output routed into a project-local `.cache/` directory, run the build through `verzly/rust-cache`. Keeping this boundary explicit prevents the release builder from owning cleanup rules that belong to the workspace cache layer.

## Get started

### Install

```sh
cargo install --git https://github.com/verzly/cargo-release
```

### Create config

```sh
cargo-release init
```

### First build

```sh
cargo-release plan
cargo-release build
```

The artifacts will be written to `dist/`.

## Usage

### Plan

```sh
cargo-release plan --config cargo-release.toml
```

`plan` prints the targets, strategy, command, output directory, and artifact patterns.

### Build

```sh
cargo-release build
```

Build only one target:

```sh
cargo-release build --target linux-x64
```

Run the command without executing it:

```sh
cargo-release build --dry-run
```

### Clean

```sh
cargo-release clean
```

This removes the configured output directory. It does not remove Cargo target directories or project cache directories.

### Doctor

```sh
cargo-release doctor
```

`doctor` checks whether the configured container engine is available and whether Cargo can be found for host builds.

### Configuration

```toml
[project]
root = "."
binary = "my-tool"

[build]
out_dir = "dist"
default_strategy = "host"
container_engine = "podman"

[artifacts]
checksum = true
manifest = true

[targets.linux-x64]
enabled = true
triple = "x86_64-unknown-linux-gnu"
strategy = "host"
command = "cargo build --release --target x86_64-unknown-linux-gnu"
artifacts = ["target/x86_64-unknown-linux-gnu/release/my-tool"]

[targets.windows-x64]
enabled = false
triple = "x86_64-pc-windows-msvc"
strategy = "container"
image = "ghcr.io/verzly/cargo-release-windows-msvc:latest"
command = "cargo build --release --target x86_64-pc-windows-msvc"
artifacts = ["target/x86_64-pc-windows-msvc/release/my-tool.exe"]
```

## GitHub Actions

This repository includes `action.yml` so `cargo-release` can be called from GitHub Actions without maintaining a separate wrapper.

The included release workflow uses the same public flow expected from other Verzly projects: `github-release` prepares and publishes the release, while `cargo-release` builds the artifacts. In CI, `rust-cache` can wrap the build so Cargo output stays under the configured project-local cache directory instead of leaking into the repository root.

## Compatibility

`cargo-release` is the Rust binary builder in the Verzly toolchain. It intentionally does not publish GitHub Releases and does not own general cache routing.

Use it with `verzly/rust-cache` when local or CI builds should write Cargo output into `.cache/`. Use it with `verzly/github-release` when the built artifacts should be attached to an official GitHub Release.

## Known issues

Cross-compilation depends on the target toolchain. This project does not hide that. If a target needs a linker, SDK, or system library, the host or container image must provide it.

macOS release builds are usually best done on macOS runners. The project supports a target definition for macOS, but it does not pretend that every macOS build is portable to every Linux container.

## Contributing

Keep target behavior explicit. A new feature should make builds easier to reason about, not more magical.

The preferred shape is small modules with clear side effects: configuration loading, command execution, artifact collection, checksum writing, and manifest writing.

## License

`verzly/cargo-release` is released under the GNU Affero General Public License v3.0 only.
