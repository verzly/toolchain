# rust-cache

`rust-cache` routes Rust and Tauri build caches into a predictable project-local cache directory.

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
  - [Practical cache workflows](#practical-cache-workflows)
- [Reference](#reference)
  - [Troubleshooting](#troubleshooting)
  - [Release artifacts](#release-artifacts)
  - [Operational notes](#operational-notes)

## Overview

### Why this exists

Rust and Tauri builds generate large directories such as `target/`, Gradle caches, Android build output, and other intermediate files. In monorepos these paths can become hard to remove safely, and local builds can pollute the repository root or the developer machine.

`rust-cache` centralizes those generated files under a configurable cache directory, defaulting to `.cache`. The intent is simple: project files stay visible, disposable build output is grouped, and cache cleanup is predictable.

### How it works

The tool detects the workspace root using Cargo metadata first, then Git, then the current directory. It builds an environment plan and either prints it or runs a command with those environment variables applied.

By default it redirects `CARGO_TARGET_DIR` to a package-specific directory under `.cache`. It can also redirect `GRADLE_USER_HOME` for Android/Tauri builds and installs a Gradle init script that moves Gradle project `buildDirectory` output into `.cache/android/builds`. This matters for Tauri Android projects because `src-tauri/gen/android/**/build`, `.gradle`, `.cxx`, and related mobile build folders are otherwise written inside the generated mobile project. It can also redirect `CARGO_HOME` when a fully project-local Cargo home is desired. The `clean-generated` command removes reproducible output directories that older Cargo, Tauri, Gradle, Xcode, or Swift commands may have left in the project tree, such as `target/`, `src-tauri/gen/**/build`, `src-tauri/gen/**/.gradle`, `src-tauri/gen/**/.cxx`, and `src-tauri/gen/**/DerivedData`.

### Use cases

Use `rust-cache` when you want to:

- keep `target/` out of the repository root;
- make monorepo cache cleanup safe and predictable;
- run `cargo fmt`, `cargo test`, `cargo build`, `cargo-release`, or `tauri-release` with consistent cache paths;
- keep Android/Gradle cache files and Tauri Android project build output under `.cache`;
- make local development and CI use the same cache layout;
- delete routed cache output by removing `.cache` and scrub stale project-tree build output with `clean-generated`.

## Get started

### GitHub Action

```yaml
- uses: verzly/rust-cache@v1
  with:
    args: run --config datarose.toml -- cargo test --workspace
```

Install and use later:

```yaml
- uses: verzly/rust-cache@v1
  with:
    install-only: "true"

- run: rust-cache run --config datarose.toml -- cargo build --workspace
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
rust-cache init
rust-cache env --config datarose.toml
rust-cache run --config datarose.toml -- cargo test --workspace
rust-cache clean --config datarose.toml
rust-cache clean-generated --config datarose.toml --dry-run
rust-cache doctor --config datarose.toml
```


### Command help

Every top-level and subcommand help output points back to this README:

```sh
rust-cache --help
rust-cache <command> --help
```

Use the README for workflow-level guidance and the command help for the exact arguments supported by the installed executable version.

### CLI commands and arguments

#### `init`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `datarose.toml` | File path | Where the starter config should be written. |
| `-f`, `--force` | No | `false` | Boolean flag | Overwrite an existing config file. |

#### `env`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `datarose.toml` | File path | Prints the environment variables that would be applied by `run`. |

#### `run`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `datarose.toml` | File path | Config file to read. |
| `--` followed by command | Yes | none | Any command and arguments | Command to execute with the planned cache environment. The separator is required so the command is not parsed as `rust-cache` options. |

#### `clean`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `datarose.toml` | File path | Removes the configured cache directory. |

#### `clean-generated`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `datarose.toml` | File path | Config file to read. |
| `--dry-run` | No | `false` | Boolean flag | Print stale generated output directories without deleting them. |

#### `doctor`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `datarose.toml` | File path | Prints detected workspace root, selected package key, cache root, and planned environment. |

## Configuration

```toml
[rust_cache.cache]
dir = ".cache"
package = "auto"
redirect_cargo_home = false
redirect_gradle = true

[rust_cache.cargo]
target_dir = "rust/packages/{package}/target"

[rust_cache.generated]
paths = []
```

| Field | Accepted values | Purpose |
| --- | --- | --- |
| `cache.dir` | Path | Root directory for generated cache content. Defaults to `.cache`. |
| `cache.package` | `auto` or explicit string | Package key used below the cache directory. `auto` uses Cargo metadata when available and falls back to `workspace`. Use an explicit value when monorepo paths must remain stable. |
| `cache.redirect_cargo_home` | Boolean | When `true`, sets `CARGO_HOME` under the cache directory. Leave `false` when you want to keep using the normal user-level Cargo registry cache. |
| `cache.redirect_gradle` | Boolean | When `true`, sets `GRADLE_USER_HOME` under the cache directory and writes `.cache/android/gradle/init.d/verzly-build-cache.gradle`, which redirects workspace Gradle project build output into `.cache/android/builds`. |
| `cargo.target_dir` | String with `{package}` placeholder | Relative target directory below `cache.dir`, or an absolute path. This is written to native Cargo config and exported as `CARGO_TARGET_DIR`. |
| `generated.paths` | List of paths | Extra reproducible output directories that `clean-generated` may remove in addition to detected `target/` and Tauri mobile generated build directories. |

Generated paths normally look like this:

```text
.cache/
├── rust/
│   ├── packages/<package>/target/
│   └── cargo-home/
└── android/
    ├── gradle/
    │   └── init.d/verzly-build-cache.gradle
    └── builds/
```

## Practical workflows

### Practical cache workflows

### Run Cargo with project-local cache routing

```sh
rust-cache run --config datarose.toml -- cargo test --workspace
```

The command after `--` receives environment variables such as `CARGO_TARGET_DIR`. This keeps generated build output under the configured cache root instead of the normal project `target/` folder.

### Print the planned environment

```sh
rust-cache env --config datarose.toml
```

Use this in CI debugging to verify exactly which cache paths would be used before running a long build. This command also writes runtime helper files, including the Gradle init script when `cache.redirect_gradle = true`.

### Tauri Android mobile build output

Run Tauri Android commands through `rust-cache run` so Gradle receives both `GRADLE_USER_HOME` and the generated init script:

```sh
rust-cache run --config datarose.toml -- pnpm tauri android build --apk --aab
```

With the default config, Gradle user caches go to `.cache/android/gradle` and project-local Gradle build output goes to `.cache/android/builds`. The generated Android project under `src-tauri/gen/android` should not keep `build/`, `.gradle/`, `.cxx/`, or `.externalNativeBuild/` directories after a correctly routed build. If you have stale output from earlier builds, run `rust-cache clean-generated --dry-run` first, then `rust-cache clean-generated`.


### Clean generated cache

```sh
rust-cache clean --config datarose.toml
rust-cache clean-generated --config datarose.toml --dry-run
rust-cache clean-generated --config datarose.toml
```

`clean` removes the configured cache root. `clean-generated` removes reproducible build output that should not be part of the repository tree, including stale `target/` directories and Tauri `src-tauri/gen/**/build`, `.gradle`, `.cxx`, `.externalNativeBuild`, `.kotlin`, `DerivedData`, and `.build` directories outside `.cache`. Run it with `--dry-run` first when introducing the tool in an existing monorepo.

## Reference

### Troubleshooting

If cache folders appear in unexpected places, run `rust-cache env` and check `cache.dir`, `cache.package`, and `cargo.target_dir`. In monorepos, prefer an explicit package name when automatic package detection is not stable enough. If a command cannot find dependencies after enabling `redirect_cargo_home`, remember that `CARGO_HOME` has moved into the cache root and may need to be warmed again.

### Release artifacts

Release assets are named by tool, version, and host target. Typical examples:

```text
rust-cache-v1.2.3-linux-x64
rust-cache-v1.2.3-macos-x64
rust-cache-v1.2.3-macos-arm64
rust-cache-v1.2.3-windows-x64.exe
```

Checksum files use the same name with `.sha256` appended. The action verifies them when the runner has `sha256sum` or `shasum`.

### Operational notes

`rust-cache` does not replace GitHub Actions cache, sccache, or Cargo's dependency cache. It only chooses where build tools write their generated files. It is safe to use around `cargo-release` and `tauri-release`; those tools still own the build and release behavior.

## License

This project is licensed under the AGPL-3.0-only license.
