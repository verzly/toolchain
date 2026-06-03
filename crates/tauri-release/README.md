# verzly/tauri-release

`verzly/tauri-release` builds Tauri desktop and mobile release artifacts through one predictable command.

It is made for projects where release builds should not depend on whatever happens to be installed on the developer's machine. Linux and Android builds can be containerized well. Windows can also be containerized with a maintained image. macOS and iOS are treated differently on purpose: by default they run on a macOS host, because those builds depend on Apple tooling.

Use it when a Tauri project needs one place to describe the release build matrix. The config keeps frontend setup, platform commands, container images, artifact patterns, checksums, and the release manifest together.

The tool is intentionally honest about platform boundaries. Linux and Android are good candidates for containerized builds. Windows can use host builds or prepared images. macOS and iOS should be treated as host-first targets because the Apple toolchain is not a normal Linux container problem.

- [How it works](#how-it-works)
  - [Platform strategies](#platform-strategies)
  - [Desktop builds](#desktop-builds)
  - [Mobile builds](#mobile-builds)
  - [Artifacts](#artifacts)
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

Read on if you want the platform boundaries. Jump to [Get started](#get-started) if you already know which platforms you want to build.

## How it works

`tauri-release` is an orchestrator around your existing Tauri build. It does not replace Tauri, Cargo, Node, Android SDK, Xcode, or signing tools.

The flow is:

```text
read config -> prepare output -> run platform builds -> collect artifacts -> write checksums -> write manifest
```

The release flow stays separate:

```text
github-release prepare -> tauri-release build -> github-release finalize
```

### Platform strategies

Each platform has its own strategy:

```text
host       -> run directly on the current machine
container  -> run in Docker or Podman
auto       -> use the configured default strategy
```

Default recommendations:

```text
Linux      -> container or host
Android    -> container or host
Windows    -> host or maintained container image
macOS      -> host
 iOS       -> host
```

The Apple platforms are intentionally not hidden behind a fake “works everywhere” promise. If a project needs macOS or iOS artifacts, build them on a macOS runner or a properly prepared Mac machine.

### Desktop builds

Desktop platforms usually run commands like:

```sh
pnpm install --frozen-lockfile
pnpm tauri build
```

or:

```sh
npm ci
npm run tauri build
```

The exact commands are configured per platform.

### Mobile builds

Android and iOS are platform-specific. Android can run well in a prepared container image. iOS should run on macOS with Xcode available.

Signing is intentionally outside this project. For Android keystore generation and secret export, use `verzly/android-signing`.

### Artifacts

Artifacts are collected from explicit glob patterns. This keeps the build honest and avoids accidentally uploading temporary files.

Typical Tauri outputs include:

```text
src-tauri/target/release/bundle/**/*.deb
src-tauri/target/release/bundle/**/*.AppImage
src-tauri/target/release/bundle/**/*.msi
src-tauri/target/release/bundle/**/*.dmg
src-tauri/gen/android/app/build/outputs/**/*.apk
src-tauri/gen/android/app/build/outputs/**/*.aab
```

## Get started

### Install

```sh
cargo install --git https://github.com/verzly/tauri-release
```

### Create config

```sh
tauri-release init
```

### First build

```sh
tauri-release plan
tauri-release build --platform linux
```

Artifacts are written to `dist/`.

## Usage

### Plan

```sh
tauri-release plan
```

### Build

Build every enabled platform:

```sh
tauri-release build
```

Build one platform:

```sh
tauri-release build --platform android
```

Preview commands only:

```sh
tauri-release build --dry-run
```

### Clean

```sh
tauri-release clean
```

### Doctor

```sh
tauri-release doctor
```

`doctor` checks whether configured host tools and container engines are available. It does not install them.

### Configuration

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

[platforms.android]
enabled = false
strategy = "container"
image = "ghcr.io/verzly/tauri-release-android:latest"
command = "pnpm tauri android build --apk --aab"
artifacts = ["src-tauri/gen/android/app/build/outputs/**/*.apk", "src-tauri/gen/android/app/build/outputs/**/*.aab"]

[platforms.ios]
enabled = false
strategy = "host"
command = "pnpm tauri ios build"
artifacts = ["src-tauri/gen/apple/build/**/*.ipa"]
```

## GitHub Actions

This repository includes `action.yml` so `tauri-release` can be called from GitHub Actions and from the same release workflows used by the other Verzly tools.

The included release workflow follows the same shape as the Rust CLI tools: `github-release` prepares and publishes the release, `cargo-release` builds the `tauri-release` executable itself, and `rust-cache` can keep build output inside the workspace cache directory. Android release jobs can call `android-signing` before the Tauri build step when keystore material needs to be prepared.

## Compatibility

`tauri-release` is meant to work with the rest of the Verzly release chain, not replace it.

Use `verzly/rust-cache` when Rust, Cargo, Gradle, or Android build output should live under `.cache/`. Use `verzly/android-signing` for Android keystore generation, fingerprints, and CI secret export. Use `verzly/github-release` after the build finishes to publish the final artifacts.

## Known issues

Tauri release builds depend on platform SDKs. This project keeps those requirements visible instead of pretending they do not exist.

The generated config uses placeholder container images. A production project should maintain and pin its own images.

## Contributing

Keep platform behavior explicit. Every platform should have a readable command, artifact list, and strategy. Avoid clever auto-detection that makes CI behavior hard to debug.

## License

`verzly/tauri-release` is released under the GNU Affero General Public License v3.0 only.
