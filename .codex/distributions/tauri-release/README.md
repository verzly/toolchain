# tauri-release

`tauri-release` builds and collects Tauri desktop and mobile release artifacts with explicit platform configuration.

This repository is a public distribution repository. The source code is maintained in the private `verzly/toolchain` monorepo and this repository contains only the public surface that users need: `README.md`, `action.yml`, `LICENSE`, and GitHub Release assets.

The public repository intentionally does not contain `src/`, `Cargo.toml`, build workflows, or release configuration. That separation keeps the user-facing repository small while allowing all tools to share the same release infrastructure in `verzly/toolchain`.

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
  - [Practical Tauri workflows](#practical-tauri-workflows)
- [Reference](#reference)
  - [Troubleshooting](#troubleshooting)
  - [Release artifacts](#release-artifacts)
  - [Operational notes](#operational-notes)
- [Contributing](#contributing)

## Overview

### Why this exists

Tauri releases are more complex than plain Rust CLI releases. They may involve frontend installation, Rust builds, system dependencies, platform-specific bundle formats, Android output, iOS output, and signing concerns. Keeping all of that directly in GitHub Actions makes the workflow difficult to review and reuse.

`tauri-release` puts the release build plan into a TOML file. The workflow can call one tool while platform commands, artifact globs, container usage, and output handling stay explicit and versioned.

### How it works

The tool reads `tauri-release.toml`, optionally runs a frontend install command, iterates over enabled platforms, runs each platform command on the host or in a configured Docker/Podman container, copies matching artifacts into the output directory, writes checksums when enabled, and writes a manifest when enabled.

It does not sign Android keys itself; use `android-signing` for keystore handling. It does not publish GitHub Releases; use `github-release` after artifacts are built.

### Use cases

Use `tauri-release` when you want to:

- keep Tauri release workflows short;
- build Linux, Windows, macOS, Android, and iOS outputs from one config file;
- isolate supported platforms with Docker or Podman where practical;
- keep Apple targets host-first where container builds are not realistic;
- collect `.deb`, `.AppImage`, `.msi`, `.exe`, `.dmg`, `.apk`, `.aab`, or `.ipa` files into one release directory;
- pair Tauri builds with `rust-cache`, `android-signing`, and `github-release`.

## Get started

### GitHub Action

```yaml
- uses: verzly/tauri-release@v1
  with:
    args: build --config tauri-release.toml
```

Install and use later:

```yaml
- uses: verzly/tauri-release@v1
  with:
    install-only: "true"

- run: tauri-release build --config tauri-release.toml --platform linux
```

The composite action detects the runner operating system and CPU architecture, maps that host to a Rust-style target name, downloads the matching executable from this repository's GitHub Releases with `gh release download`, verifies a `.sha256` file when one is present, copies the executable into a temporary bin directory, and adds that directory to `PATH`.

The action does not build from source. It does not clone `verzly/toolchain`. It only consumes the release assets published here.

When the action is used through a moving ref such as `@latest`, `@next`, `@v1`, or `@v1.2`, the installer resolves that ref to the concrete `vX.Y.Z` or preview release tag pointing at the same commit before downloading assets. This lets workflows use moving action refs while executable assets remain attached to immutable release tags.

## Usage

### Action inputs

| Input | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `github-token` | No | `""` | Any GitHub token readable by `gh`; empty uses `${{ github.token }}` | Used only to download release assets. Public repositories normally work with the default token. Pass a custom token when downloading from a private fork or restricted environment. |
| `version` | No | `""` | Empty, `latest`, `next`, `v1`, `v1.2`, `1.2.3`, `v1.2.3`, or any published release tag | Selects the release asset to download. Empty uses the action ref when it is a release selector, otherwise the latest release. Moving refs resolve to the concrete `vX.Y.Z` release tag that has the executable asset. |
| `install-only` | No | `"false"` | String `"true"` or `"false"` | When `"true"`, the action only installs the executable and adds it to `PATH`. When `"false"`, it installs and immediately runs the executable with `args`. |
| `args` | No | `--help` | Any valid CLI argument string for the executable | Passed to the installed executable when `install-only` is not `"true"`. Quote values carefully because this string is evaluated by the shell. |
| `working-directory` | No | `.` | Relative or absolute path | Directory where the executable runs when `install-only` is not `"true"`. |

### Action outputs

| Output | Value | Purpose |
| --- | --- | --- |
| `bin-path` | Absolute path to the installed executable | Use this when a later step should invoke the exact binary path instead of relying on `PATH`. |
| `host-target` | Rust-style host target such as `x86_64-unknown-linux-gnu` | Shows which release asset was selected for the current runner. |

### CLI usage

```sh
tauri-release init
tauri-release plan --config tauri-release.toml
tauri-release build --config tauri-release.toml
tauri-release build --config tauri-release.toml --platform android
tauri-release clean --config tauri-release.toml
tauri-release doctor --config tauri-release.toml
```

### CLI commands and arguments

#### `init`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `tauri-release.toml` | File path | Where the starter config should be written. |
| `-f`, `--force` | No | `false` | Boolean flag | Overwrite an existing config file. |

#### `plan`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `tauri-release.toml` | File path | Prints enabled platforms, strategies, commands, and artifact globs. |

#### `build`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `tauri-release.toml` | File path | Config file to read. |
| `--platform` | No | all enabled platforms | Platform key from `[platforms.<key>]`, for example `linux`, `windows`, `macos`, `android`, `ios` | Builds only one configured platform. |
| `--dry-run` | No | `false` | Boolean flag | Prints planned commands without executing build commands or copying artifacts. |

#### `clean`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `tauri-release.toml` | File path | Removes configured output and cache directories owned by this tool. |

#### `doctor`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `tauri-release.toml` | File path | Checks local tool availability and reports missing container images for enabled container platforms. |

## Configuration

```toml
[project]
root = "."
frontend_install = "pnpm install --frozen-lockfile"

[build]
out_dir = "dist"
cache_dir = ".cache/tauri-release"
default_strategy = "host"
container_engine = "podman"

[artifacts]
checksum = true
manifest = true

[platforms.linux]
enabled = true
strategy = "container"
image = "ghcr.io/verzly/tauri-release-linux:latest"
command = "pnpm tauri build"
artifacts = ["src-tauri/target/release/bundle/**/*.deb", "src-tauri/target/release/bundle/**/*.AppImage"]
```

| Field | Accepted values | Purpose |
| --- | --- | --- |
| `project.root` | Path | Project root where commands are executed. |
| `project.frontend_install` | String or omitted | Optional command run before platform builds, usually package installation. |
| `build.out_dir` | Path | Directory where collected release artifacts are written. |
| `build.cache_dir` | Path | Cache/output directory owned by `tauri-release` cleanup. |
| `build.default_strategy` | `host`, `container`, `auto` | Strategy used when a platform does not override it. |
| `build.container_engine` | `podman`, `docker` | Container runtime executable. |
| `artifacts.checksum` | Boolean | Write `.sha256` files next to collected artifacts. |
| `artifacts.manifest` | Boolean | Write `manifest.json`. |
| `platforms.<key>.enabled` | Boolean | Whether the platform participates in normal builds. |
| `platforms.<key>.strategy` | `host`, `container`, `auto` | How to run the platform command. |
| `platforms.<key>.image` | Container image | Required when `strategy = "container"`. |
| `platforms.<key>.command` | Shell command | Build command for that platform. |
| `platforms.<key>.artifacts` | List of paths or globs | Files copied into the output directory after success. |
| `platforms.<key>.env` | Key/value map | Environment variables passed to the platform command. |

## Practical workflows

### Practical Tauri workflows

### Plan before building

```sh
tauri-release plan --config tauri-release.toml
```

Use `plan` before the first real build. It shows enabled platforms, strategies, commands, and artifact globs without running expensive platform builds.

### Build one platform

```sh
tauri-release build --config tauri-release.toml --platform linux
```

This is the normal debugging path. Once one platform works, enable more platforms in the config and let CI run the full release build.

### Combine with cache routing and signing

```sh
rust-cache run --config rust-cache.toml -- tauri-release build --config tauri-release.toml --platform android
android-signing write-github-env release.jks --alias release-key
```

`tauri-release` builds and collects app artifacts. `rust-cache` keeps build output out of the repository. `android-signing` manages keystore-related CI values.

## Reference

### Troubleshooting

If no artifacts are collected, verify the platform `artifacts` globs against the real Tauri output paths. If a container platform fails, check whether the image includes the system packages needed by Tauri. If macOS or iOS builds fail inside a non-macOS environment, move those platforms to macOS runners; Apple signing and bundling are host-first by design.

### Release artifacts

Release assets are named by tool, version, and host target. Typical examples:

```text
tauri-release-v1.2.3-x86_64-unknown-linux-gnu
tauri-release-v1.2.3-aarch64-unknown-linux-gnu
tauri-release-v1.2.3-x86_64-apple-darwin
tauri-release-v1.2.3-aarch64-apple-darwin
tauri-release-v1.2.3-x86_64-pc-windows-msvc.exe
```

Checksum files use the same name with `.sha256` appended. The action verifies them when the runner has `sha256sum` or `shasum`.

### Operational notes

Container support does not make every platform magically cross-buildable. Linux and Android are good container candidates. macOS and iOS remain host-first because Apple tooling and signing requirements are tied to macOS. Windows support depends on the configured image and target project constraints.

## Contributing

Contribution guidelines live in the `verzly/toolchain` `CONTRIBUTING.md`. Source changes are made in `verzly/toolchain`; this repository is the public distribution surface.

## License

This project is licensed under the AGPL-3.0-only license.
