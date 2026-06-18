# Verzly

`verzly` is the unified command-line entrypoint and GitHub Action installer for the Verzly toolchain. It exposes the release, signing, cache, and repository helpers through one executable while keeping each tool internally modular.

## Usage

Install the toolchain in a workflow:

```yaml
- uses: verzly/verzly@v1
  with:
    install-only: "true"
```

Run any tool through the unified executable:

```yaml
- uses: verzly/verzly@v1
  with:
    tool: github-release
    args: prepare --config datarose.toml --release-target app --version 1.2.3
```

Local commands use the same shape:

```bash
verzly github-release prepare --config datarose.toml --release-target app --version 1.2.3
verzly tauri-release build --config datarose.toml --platform desktop
verzly android-signing check-env --require-fingerprint
verzly ios-signing check-env
verzly repository check
verzly rust-cache env
```

The standalone public tools remain available during migration. New repositories should prefer the `verzly` action and executable so workflows install one toolchain version.

## GitHub Action inputs

| Input | Required | Default | Description |
| --- | --- | --- | --- |
| `github-token` | No | `${{ github.token }}` | Token used to download release assets. |
| `version` | No | empty | Release selector for the executable. Supports `1.2.3`, `v1.2.3`, `latest`, `next`, `v1`, or `v1.2`. Empty uses the action ref when possible, otherwise latest. |
| `install-only` | No | `false` | Install `verzly` and add it to `PATH` without running a command. |
| `tool` | No | empty | Tool subcommand to run, for example `github-release`, `tauri-release`, `ios-signing`, or `repository`. Required when `install-only` is false and `args` does not start with a tool name. |
| `args` | No | `--help` | Arguments passed to `verzly`. When `tool` is set, these are passed after the tool name. |
| `working-directory` | No | `.` | Directory where the command should run. |

## Outputs

| Output | Description |
| --- | --- |
| `bin-path` | Absolute path to the installed `verzly` executable. |
| `host-target` | Release asset target selected for the current runner. |
| `tool-version` | Version output reported by the installed executable when available. |

## Release assets

Releases publish standalone executables named like:

```text
verzly-v1.2.3-linux-x64
verzly-v1.2.3-macos-x64
verzly-v1.2.3-macos-arm64
verzly-v1.2.3-windows-x64.exe
```

Each executable may also have a `.sha256` checksum file.

## License

AGPL-3.0-only.
