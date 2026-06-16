# cargo-release

`cargo-release` builds and collects standalone Rust executable release assets in a predictable, repository-independent way.

This repository is a public distribution repository. The source code is maintained in the private `verzly/toolchain` monorepo and this repository contains only the public surface that users need: `README.md`, `CONTRIBUTING.md`, `action.yml`, `LICENSE`, and GitHub Release assets.

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
  - [Command help](#command-help)
  - [CLI commands and arguments](#cli-commands-and-arguments)
- [Configuration](#configuration)
- [Practical workflows](#practical-workflows)
  - [Practical build workflows](#practical-build-workflows)
- [Reference](#reference)
  - [Troubleshooting](#troubleshooting)
  - [Release artifacts](#release-artifacts)
  - [Operational notes](#operational-notes)

## Overview

### Why this exists

Rust projects often need Linux, macOS, and Windows executables. The build commands, artifact paths, checksums, container choices, and output names are usually repeated in YAML. That makes releases hard to review and easy to break.

`cargo-release` puts the build matrix and artifact rules into a TOML config and exposes a small CLI. Workflows can call one command instead of embedding long platform-specific shell blocks.

It is intentionally focused on building and collecting artifacts. It does not create GitHub Releases; that is `github-release`'s responsibility.

### How it works

`cargo-release` reads `datarose.toml`, resolves enabled targets, runs each target's build command either on the host or inside a configured Docker/Podman container, copies configured artifact paths into `dist/<tool>`, writes `.sha256` files when enabled, and writes a JSON manifest when enabled.

The tool separates target planning from release publishing. That makes it useful locally, in CI, and inside larger workflows that publish artifacts only after all builds succeed.

### Use cases

Use `cargo-release` when you want to:

- build Rust CLI executables for multiple operating systems;
- keep build command definitions out of GitHub Actions YAML;
- use Docker or Podman for isolated Linux/Windows-like builds where possible;
- collect artifacts into a predictable `dist` directory;
- create checksums and a build manifest for later publication;
- build tools without polluting the developer machine more than necessary.

## Get started

### GitHub Action

```yaml
- uses: verzly/cargo-release@v1
  with:
    args: build --version 1.2.3 --config datarose.toml
```

Install and run later:

```yaml
- uses: verzly/cargo-release@v1
  with:
    install-only: "true"

- run: cargo-release build --version 1.2.3 --config datarose.toml
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
| `host-target` | Asset target such as `linux-x64`, `macos-arm64`, or `windows-x64` | Shows which release asset was selected for the current runner. |

### CLI usage

```sh
cargo-release init
cargo-release plan --config datarose.toml
cargo-release build --version 1.2.3 --config datarose.toml
cargo-release build --version 1.2.3 --target linux-x64
cargo-release clean --config datarose.toml
cargo-release doctor --config datarose.toml
```


### Command help

Every top-level and subcommand help output points back to this README:

```sh
cargo-release --help
cargo-release <command> --help
```

Use the README for workflow-level guidance and the command help for the exact arguments supported by the installed executable version.

### CLI commands and arguments

#### `init`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `datarose.toml` | File path | Where the starter config should be written. |
| `-f`, `--force` | No | `false` | Boolean flag | Overwrite an existing config file. |

#### `plan`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `datarose.toml` | File path | Config file to read. Prints enabled targets, strategies, commands, and artifacts. |
| `--release-target` | No | inferred when only one release target exists | Release target name from `[[release.targets]]` | Selects which Datarose release target supplies cargo package, binary, output, and enabled target keys. |

#### `build`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `datarose.toml` | File path | Config file to read. |
| `--release-target` | No | inferred when only one release target exists | Release target name from `[[release.targets]]` | Selects which Datarose release target to build. Required when the config has multiple release targets. |
| `-v`, `--version` | No | Package/runtime value when available | Version string such as `1.2.3` | Used in artifact file names through the `{version}` template value. |
| `--target` | No | all enabled targets | Target key from `[targets.<key>]`, for example `linux-x64` | Builds only one configured target. Fails if the key is unknown or disabled. |
| `--output` | No | configured output directory | Directory path | Overrides the configured output directory for this invocation. |
| `--dry-run` | No | `false` | Boolean flag | Prints commands and planned artifact work without running build commands or copying files. |

#### `clean`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `datarose.toml` | File path | Reads the config and removes generated output directories owned by this tool. |
| `--release-target` | No | inferred when only one release target exists | Release target name from `[[release.targets]]` | Selects which Datarose release target owns the output directory. |

#### `doctor`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `datarose.toml` | File path | Checks local tool availability and reports obvious target configuration issues. |
| `--release-target` | No | inferred when only one release target exists | Release target name from `[[release.targets]]` | Selects which Datarose release target to inspect. |

## Configuration

```toml
[project]
root = "."
binary = "my-tool"

[build]
out_dir = "dist/my-tool"
default_strategy = "host"
container_engine = "podman"

[artifacts]
checksum = true
manifest = true
name_template = "{binary}-v{version}-{target}{ext}"

[targets.linux-x64]
enabled = true
triple = "x86_64-unknown-linux-gnu"
strategy = "host"
command = "cargo build --release -p my-tool"
artifacts = ["target/release/my-tool"]
required_env = []
```

When using `datarose.toml` release targets, keep public release metadata in `[[release.targets]]` and put build-specific overrides under `[cargo_release]`:

```toml
[[release.targets]]
name = "my-tool"
cargo_binary = "my-tool"
cargo_package = "my-tool"
cargo_targets = ["linux-x64", "windows-x64"]

[cargo_release.build]
container_engine = "podman"
default_strategy = "container"

[cargo_release.targets.windows-x64]
strategy = "container"
image = "ghcr.io/acme/rust-windows-cross:latest"
command = "cargo build --release -p my-tool --target x86_64-pc-windows-gnu"
artifacts = ["target/x86_64-pc-windows-gnu/release/my-tool.exe"]
```

| Field | Accepted values | Purpose |
| --- | --- | --- |
| `project.root` | Path | Repository or package root where build commands run. |
| `project.binary` | String | Executable name used by naming templates and defaults. |
| `build.out_dir` | Path | Directory where collected release artifacts are written. |
| `build.default_strategy` | `host`, `container`, `auto` | Strategy used when a target does not provide an explicit strategy. |
| `build.container_engine` | `podman`, `docker` | Container runtime executable to use for container targets. |
| `artifacts.checksum` | Boolean | Write `.sha256` files next to copied artifacts. |
| `artifacts.manifest` | Boolean | Write `manifest.json` describing collected artifacts. |
| `artifacts.name_template` | String template | Output file name template. Supported values include `{binary}`, `{version}`, `{target}`, and `{ext}`. |
| `targets.<key>.enabled` | Boolean | Whether the target participates in normal builds. |
| `targets.<key>.triple` | Rust target triple | Documents the target platform and can be used by build commands. |
| `targets.<key>.strategy` | `host`, `container`, `auto` | How to execute the target build. |
| `targets.<key>.image` | Container image | Required when `strategy = "container"`. |
| `targets.<key>.command` | Shell command | Build command to execute. |
| `targets.<key>.artifacts` | List of paths or globs | Files to copy into the output directory after a successful build. |
| `targets.<key>.env` | Key/value map | Environment variables passed to the build command. |
| `targets.<key>.required_env` | List of names | Environment variables that must be present before the target runs. Missing values fail the target early with a clear message. |

## Practical workflows

### Practical build workflows

### Build all configured targets

```sh
cargo-release build --config datarose.toml --version 1.4.0
```

This runs every enabled target, copies matching artifacts into the configured output directory, writes checksums when enabled, and writes a manifest when enabled.

### Build one target while debugging

```sh
cargo-release build --config datarose.toml --version 1.4.0 --target linux-x64 --dry-run
cargo-release build --config datarose.toml --version 1.4.0 --target linux-x64
```

Use `--dry-run` before changing container images or commands. It shows what would run without creating artifacts.

### Configure a container target

```toml
[cargo_release.targets.linux-x64]
strategy = "container"
image = "ghcr.io/acme/rust-linux-release:latest"
command = "cargo build --release -p my-tool --target x86_64-unknown-linux-gnu"
artifacts = ["target/x86_64-unknown-linux-gnu/release/my-tool"]
```

Container targets are explicit by design. You choose the Docker or Podman image and the exact command; `cargo-release` runs it, then collects the configured files.

### Keep local machines clean

```sh
rust-cache run --config datarose.toml -- cargo-release build --config datarose.toml --version 1.4.0
```

`cargo-release` owns artifact production. `rust-cache` can wrap it when you want `target/` and related cache paths outside the source tree.

## Reference

### Troubleshooting

If a target is skipped, check `targets.<name>.enabled`. If a container target fails before running Cargo, check `build.container_engine`, `targets.<name>.image`, and whether Docker or Podman can pull that image. If no files are copied, the build may have succeeded but the `targets.<name>.artifacts` globs do not match the produced files.

### Release artifacts

Release assets are named by tool, version, and host target. Typical examples:

```text
cargo-release-v1.2.3-linux-x64
cargo-release-v1.2.3-macos-x64
cargo-release-v1.2.3-macos-arm64
cargo-release-v1.2.3-windows-x64.exe
```

Checksum files use the same name with `.sha256` appended. The action verifies them when the runner has `sha256sum` or `shasum`.

### Operational notes

Host builds can only produce artifacts supported by the current machine and installed Rust targets. Container builds improve isolation but still depend on the configured image. `cargo-release` does not install Rust targets, system packages, or container images automatically; those choices stay explicit in config and CI.

## License

This project is licensed under the AGPL-3.0-only license.
