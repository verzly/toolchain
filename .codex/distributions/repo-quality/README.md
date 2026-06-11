# repo-quality

`repo-quality` prepares repository-local quality gates for projects that use `hk`, `mise`, Rust, JavaScript, TypeScript, Vue, PHP, Rector, and Pest.

It is intended for repositories where commits should stay lightweight, while pushes should run the full project quality gate before code reaches GitHub.

- [Purpose](#purpose)
- [Install](#install)
- [How it works](#how-it-works)
- [Commands](#commands)
  - [init](#init)
  - [plan](#plan)
  - [doctor](#doctor)
- [Generated hooks](#generated-hooks)
- [Language profiles](#language-profiles)
- [GitHub Actions](#github-actions)
- [Command help](#command-help)
- [License](#license)

## Purpose

`repo-quality` centralizes the setup that would otherwise be repeated manually in every repository:

- install `hk` and `pkl` through `mise`;
- generate a Windows-safe `hk.pkl`;
- configure formatting hooks for `pre-commit`;
- configure lint/test/build quality gates for `pre-push`;
- calibrate commands for Rust, JavaScript, and PHP projects;
- run `hk install` so Git hooks are active.

The tool does not replace project-specific package scripts. It connects existing scripts and standard tool commands into a consistent hook model.

## Install

Use the GitHub Action to install the executable in CI:

```yaml
- uses: verzly/repo-quality@v0.2
  with:
    install-only: true
```

For local usage, install the published executable or make it available through your preferred `mise` setup, then run it from the repository root.

## How it works

A typical setup is:

```sh
repo-quality init
```

The command detects repository files such as:

```text
Cargo.toml
package.json
aube-workspace.yaml
composer.json
```

Then it prepares quality hooks:

```text
repo-quality
→ mise use hk@latest
→ mise use pkl@latest
→ write hk.pkl
→ hk install
```

If a repository already has `hk.pkl`, pass `--force` to replace it.

## Commands

### init

Prepare the repository:

```sh
repo-quality init
```

Preview without writing files:

```sh
repo-quality init --dry-run
```

Overwrite an existing hook configuration:

```sh
repo-quality init --force
```

Force selected language profiles:

```sh
repo-quality init --language rust --language js --language php
```

Skip tool installation when `hk` and `pkl` are already managed elsewhere:

```sh
repo-quality init --skip-mise-use
```

Skip Git hook installation and only write `hk.pkl`:

```sh
repo-quality init --skip-hk-install
```

### plan

Print the detected repository profile and generated `hk.pkl` without changing the repository:

```sh
repo-quality plan
```

### doctor

Check whether the repository has the expected setup:

```sh
repo-quality doctor
```

## Generated hooks

The generated model is intentionally simple:

```text
pre-commit
→ format only

pre-push
→ format check
→ lint
→ tests
→ builds, when configured by the repository profile
```

Manual commands are also available through `hk`:

```sh
hk fix
hk check
hk run pre-push
```

This avoids long test runs during every commit while still preventing bad code from being pushed.

## Language profiles

### Rust

The Rust profile uses standard Cargo commands:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

### JavaScript and TypeScript

The JavaScript profile detects the package runner and existing package scripts.

Runner detection prefers:

```text
aube-workspace.yaml → aube
pnpm-lock.yaml     → pnpm
yarn.lock          → yarn
bun.lock / bun.lockb → bun
fallback           → npm
```

Supported script names include:

```text
format:js
format:js:check
format
format:check
lint:js
lint
test:js
test:unit
test
```

### PHP

The PHP profile uses Composer-managed tools when present:

```text
composer exec rector -- --dry-run
composer exec rector
composer exec pest
```

## GitHub Actions

Use the action when a workflow needs the executable:

```yaml
- uses: verzly/repo-quality@v0.2
  with:
    args: doctor
```

For most repositories, CI should call the repository’s own package scripts directly. `repo-quality` is primarily a local bootstrapper for consistent hooks.

## Command help

Every executable and command help screen points back to this README:

```sh
repo-quality --help
repo-quality <command> --help
```

Read the full README:

```text
https://github.com/verzly/repo-quality
```

## License

This project is licensed under `AGPL-3.0-only`.
