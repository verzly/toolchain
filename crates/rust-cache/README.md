# verzly/rust-cache

`verzly/rust-cache` keeps Rust and Tauri build output in one predictable project-local cache directory.

Rust projects can generate a lot of files that are not part of the source code. In small projects this is usually just `target/`. In Tauri projects it can also include Android Gradle caches and other generated build output. In monorepos it becomes harder to see what belongs to the repository and what can be deleted safely.

This tool does not delete your source files. It sets environment variables and runs your command so build output goes under one `.cache/` directory.

Use it when you want build output to be easy to remove and easy to ignore. Instead of moving folders after the build, the tool sets the environment variables that Cargo and Gradle already understand, then runs your command with those paths.

The default layout uses one `.cache/` directory at the workspace root. Each package gets its own target directory inside that cache, so a single-package project can later move into a monorepo without changing the basic cache model.

- [How it works](#how-it-works)
  - [Workspace root](#workspace-root)
  - [Cache layout](#cache-layout)
  - [Environment variables](#environment-variables)
- [Get started](#get-started)
  - [Install](#install)
  - [Run a command](#run-a-command)
  - [Create config](#create-config)
- [Usage](#usage)
  - [Run](#run)
  - [Env](#env)
  - [Clean](#clean)
  - [Doctor](#doctor)
  - [Configuration](#configuration)
- [GitHub Actions](#github-actions)
- [Compatibility](#compatibility)
- [Known issues](#known-issues)
- [Contributing](#contributing)

Read on if you want the layout. Jump to [Run](#run) if you just want to use it.

## How it works

`rust-cache` builds an environment, then either prints it or runs a command with it.

```text
detect workspace -> choose package key -> build cache paths -> export env -> run command
```

The command is still your command:

```sh
rust-cache run -- cargo build
rust-cache run -- cargo test
rust-cache run -- pnpm tauri build
```

### Workspace root

The workspace root is detected in this order:

1. `cargo metadata --no-deps`
2. Git repository root
3. current directory

This makes the default behavior useful for normal Cargo projects, Tauri projects, and monorepos.

### Cache layout

Default layout:

```text
.cache/
  rust/
    cargo-home/
    packages/
      my-package/
        target/
  android/
    gradle/
```

Even a single-package project uses the package layout. That makes it easier to move the project into a monorepo later.

### Environment variables

By default the tool sets:

```text
CARGO_TARGET_DIR=.cache/rust/packages/<package>/target
GRADLE_USER_HOME=.cache/android/gradle
```

`CARGO_HOME` is optional because moving the Cargo registry cache is not always desirable.

## Get started

### Install

```sh
cargo install --git https://github.com/verzly/rust-cache
```

### Run a command

```sh
rust-cache run -- cargo build --release
```

### Create config

```sh
rust-cache init
```

## Usage

### Run

```sh
rust-cache run -- cargo test
```

For Tauri:

```sh
rust-cache run -- pnpm tauri build
```

### Env

Print shell exports:

```sh
rust-cache env
```

Output format:

```sh
export CARGO_TARGET_DIR="/path/to/project/.cache/rust/packages/app/target"
export GRADLE_USER_HOME="/path/to/project/.cache/android/gradle"
```

### Clean

```sh
rust-cache clean
```

This removes the configured cache root. It does not remove `target/` folders that were created before the project started using `rust-cache`.

### Doctor

```sh
rust-cache doctor
```

`doctor` prints the detected workspace root, package key, and final cache paths.

### Configuration

```toml
[cache]
dir = ".cache"
package = "auto"
redirect_cargo_home = false
redirect_gradle = true
```

`package = "auto"` uses the current Cargo package name when available. In a monorepo, you can pin a stable package key:

```toml
[cache]
package = "desktop-app"
```

## GitHub Actions

This repository includes `action.yml` so `rust-cache` can be used in workflows as well as locally.

In a release workflow it usually runs before the actual builder. The builder still decides what to build; `rust-cache` only decides where Cargo, Gradle, and related output should go.

## Compatibility

`rust-cache` is intentionally small and sits below the build tools.

It can wrap `verzly/cargo-release` when building Rust executable artifacts, and it can wrap `verzly/tauri-release` when building Tauri desktop or mobile artifacts. It does not create GitHub Releases; that remains the job of `verzly/github-release`.

## Known issues

Not every generated Tauri file can be moved safely with environment variables. `rust-cache` focuses on cache directories that the toolchain already supports redirecting.

If a build script writes files directly into the project, that script needs to be fixed separately.

## Contributing

Keep the tool conservative. It should redirect known cache locations, not move arbitrary folders after the fact.

## License

`verzly/rust-cache` is released under the GNU Affero General Public License v3.0 only.
