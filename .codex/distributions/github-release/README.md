# github-release

`github-release` is a reusable release branch, tag, and GitHub Release orchestrator for projects that need predictable release automation without writing large workflow files.

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
  - [Practical release workflows](#practical-release-workflows)
- [Reference](#reference)
  - [Troubleshooting](#troubleshooting)
  - [Release artifacts](#release-artifacts)
  - [Operational notes](#operational-notes)
- [Contributing](#contributing)

## Overview

### Why this exists

GitHub releases usually involve the same fragile sequence in every project: create a release branch, update version files, build assets, merge the branch, tag the final commit, generate release notes, upload assets, and clean up temporary branches. When that logic lives directly in YAML, each repository slowly develops its own edge cases.

`github-release` moves that lifecycle into a typed executable. The workflow stays short, while the dangerous parts such as branch deletion, tag naming, version file updates, and release publishing are handled by one maintained tool.

It was created for the Verzly toolchain model where source code can live in `verzly/toolchain`, while public distribution repositories receive only release assets and public documentation.

### How it works

The tool has two release modes.

For a normal source repository, `prepare` creates a temporary release branch and updates configured version files. After the project-specific build succeeds, `finalize` merges the release branch into the target branch, creates the tag, optionally creates the GitHub Release, and removes the temporary branch. If the build fails, `abort` deletes the temporary release branch.

For a distribution repository, `publish` can create a GitHub Release directly from an already-prepared source tag. This is useful when the source repository and public release repository are different repositories. In that model, release notes can still point to the source repository, so pull request references stay connected to the real code review history.

### Use cases

Use `github-release` when you want to:

- keep release workflow YAML short and readable;
- standardize release branch names, tag names, release names, and cleanup behavior across repositories;
- update version files before the build runs;
- publish release assets after a successful build;
- generate public release notes from a source repository tag;
- release several public distribution repositories from one private or internal source monorepo.

## Get started

### GitHub Action

Run the executable directly from a workflow:

```yaml
- uses: verzly/github-release@v1
  with:
    args: plan --version 1.2.3 --config crates/my-tool/github-release.toml
```

Install it once and call it from later steps:

```yaml
- uses: verzly/github-release@v1
  with:
    install-only: "true"

- run: github-release prepare --version 1.2.3 --config crates/my-tool/github-release.toml
```

The composite action detects the runner operating system and CPU architecture, maps that host to a Rust-style target name, downloads the matching executable from this repository's GitHub Releases with `gh release download`, verifies a `.sha256` file when one is present, copies the executable into a temporary bin directory, and adds that directory to `PATH`.

The action does not build from source. It does not clone `verzly/toolchain`. It only consumes the release assets published here.

## Usage

### Action inputs

| Input | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `github-token` | No | `""` | Any GitHub token readable by `gh`; empty uses `${{ github.token }}` | Used only to download release assets. Public repositories normally work with the default token. Pass a custom token when downloading from a private fork or restricted environment. |
| `version` | No | `""` | Empty, `1.2.3`, `v1.2.3`, or any published release tag | Selects the release asset to download. Empty means latest release. If the value does not start with `v`, the action prefixes it with `v`. |
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
github-release --help
github-release init --config github-release.toml
github-release plan --version 1.2.3 --config github-release.toml
github-release prepare --version 1.2.3 --config github-release.toml
github-release finalize --version 1.2.3 --config github-release.toml --assets dist
github-release publish --version 1.2.3 --config github-release.toml --assets dist
github-release floating-tags --config github-release.toml --all
github-release abort --version 1.2.3 --config github-release.toml
```

Top-level automatic options include `--help` and `--version`.

### CLI commands and arguments

#### `init`

Creates a starter `github-release.toml`.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `github-release.toml` | File path | Where the starter config should be written. |
| `-f`, `--force` | No | `false` | Boolean flag | Overwrite an existing config file. |

#### `plan`

Prints the calculated release plan without changing files, branches, tags, or GitHub Releases.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-v`, `--version` | Yes | none | SemVer such as `1.2.3`, `1.2.3-rc.1`, `2.0.0-beta.1` | Version used to render branch names, tag names, release names, and configured file updates. |
| `-c`, `--config` | No | `github-release.toml` | File path | Config file to read. |
| `--target-branch` | No | Config value | Branch name | Temporary override for the branch that receives the release merge. |
| `--release-branch` | No | Generated from config and version | Branch name | Temporary override for the release branch name. |

#### `prepare`

Creates the release branch and applies configured version file changes before the project build starts.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-v`, `--version` | Yes | none | SemVer / prerelease version | Release version. Use the same value later in `finalize` or `abort`. |
| `-c`, `--config` | No | `github-release.toml` | File path | Config file to read. |
| `--target-branch` | No | Config value | Branch name | Override the target branch. |
| `--release-branch` | No | Generated | Branch name | Override the release branch. |
| `--dry-run` | No | `false` | Boolean flag | Print planned Git and file operations without executing them. |
| `--force-branch` | No | `false` | Boolean flag | Allow recreating an existing local release branch. Remote branch checks still protect against accidental collisions. |
| `--commit-message` | No | Config template | String | Override the version update commit message. Template values such as `{tag}` are normally supplied by config. |

#### `finalize`

Merges the release branch into the target branch, creates the source tag, optionally publishes a GitHub Release, and cleans up the release branch.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-v`, `--version` | Yes | none | Same version passed to `prepare` | Resolves the release branch, tag, and release name. |
| `-c`, `--config` | No | `github-release.toml` | File path | Config file to read. |
| `--target-branch` | No | Config value | Branch name | Override the target branch. |
| `--release-branch` | No | Generated | Branch name | Override the release branch. |
| `--assets` | No | none | Directory path | Directory whose files should be uploaded as release assets when GitHub Release publishing is enabled. Nested files are collected recursively. |
| `--prerelease` | No | `auto` | `auto`, `true`, `false` | Controls the GitHub Release prerelease flag. `auto` marks SemVer prerelease versions as prereleases. |
| `--dry-run` | No | `false` | Boolean flag | Print Git/GitHub commands without executing them. |
| `--keep-branch` | No | `false` | Boolean flag | Keep the release branch after success instead of deleting it. |
| `--skip-github-release` | No | `false` | Boolean flag | Merge and tag only. Use this for source monorepo tags followed by a separate public distribution release. |
| `--notes` | No | Config value | String | Use this text as the GitHub Release body instead of generated notes. Cannot be combined with `--notes-file`. |
| `--notes-file` | No | none | File path | Read the GitHub Release body from a file instead of generated notes. Cannot be combined with `--notes`. |
| `--update-floating-tags` | No | `false` | Boolean flag | Update stable major/minor floating tags such as `v1.2` and `v1` after publishing a GitHub Release. Config can enable this without the flag. |

#### `publish`

Creates a GitHub Release without preparing or merging a branch. This is the distribution-repository command.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-v`, `--version` | Yes | none | SemVer / prerelease version | Version to publish. Public distribution repositories normally publish `v{version}`. |
| `-c`, `--config` | No | `github-release.toml` | File path | Distribution config to read. |
| `--assets` | No | none | Directory path | Directory whose files should be uploaded. |
| `--prerelease` | No | `auto` | `auto`, `true`, `false` | Controls the prerelease flag. |
| `--dry-run` | No | `false` | Boolean flag | Print the GitHub command without creating the release. |
| `--notes` | No | Config value | String | Use this text as the GitHub Release body instead of generated notes. Cannot be combined with `--notes-file`. |
| `--notes-file` | No | none | File path | Read the GitHub Release body from a file instead of generated notes. Cannot be combined with `--notes`. |
| `--update-floating-tags` | No | `false` | Boolean flag | Update stable major/minor floating tags such as `v1.2` and `v1` after publishing. Config can enable this without the flag. |

#### `floating-tags`

Creates or repairs moving stable major/minor tags for already-published releases. It never updates prerelease tags. With `tag_prefix = "v"`, publishing or analyzing `v1.2.3` updates `v1.2` and `v1`. With custom prefixes and suffixes, the generated floating tags keep the same shape, for example `tool-v1.2-dist` and `tool-v1-dist` for `tool-v1.2.3-dist`.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-c`, `--config` | No | `github-release.toml` | File path | Config file to read. |
| `-v`, `--version` | No | none | Stable SemVer version | Build the full release tag from config and update its floating tags. Use exactly one of `--version`, `--tag`, or `--all`. |
| `--tag` | No | none | Full stable tag such as `v1.2.3` | Analyze one existing full release tag and update its matching floating tags. |
| `--all` | No | `false` | Boolean flag | Scan all matching stable `vX.Y.Z` tags, find the highest release for every `vX.Y` and `vX`, and update the floating tags. |
| `--repository` | No | Config value | `owner/repo` | Override `github.target_repository`. |
| `--force` | No | `false` | Boolean flag | Run even when `release.floating_tags` is disabled in config. |
| `--dry-run` | No | `false` | Boolean flag | Print planned ref updates without writing tags. |

#### `abort`

Deletes a temporary release branch after a failed build.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-v`, `--version` | No | none | Version string | Used to resolve the default release branch. Required unless `--release-branch` is provided. |
| `-c`, `--config` | No | `github-release.toml` | File path | Config file to read. |
| `--release-branch` | No | Generated from version | Branch name | Explicit branch to delete. |
| `--allow-any-branch` | No | `false` | Boolean flag | Disable the configured release branch prefix safety check. Use only when you know exactly what will be deleted. |
| `--dry-run` | No | `false` | Boolean flag | Print deletion commands without executing them. |

## Configuration

A `github-release.toml` file has three main areas.

```toml
[release]
target_branch = "master"
branch_prefix = "release/"
tag_prefix = "v"
tag_suffix = ""
name_prefix = ""
name_suffix = ""
commit_message = "chore(release): prepare {tag}"
merge_message = "chore(release): merge {tag}"
cleanup = true
latest = true
floating_tags = false

[github]
target_repository = "verzly/my-tool"
source_repository = "verzly/toolchain"
source_tag_prefix = "my-tool-v"
source_tag_suffix = ""
generate_notes = true
notes_body = ""

[[files]]
path = "Cargo.toml"
kind = "toml"
key = "package.version"
value = "{version}"
optional = false
```

| Field | Accepted values | Purpose |
| --- | --- | --- |
| `release.target_branch` | Branch name | Branch that receives the final merge. Defaults are usually `master` or `main`. |
| `release.branch_prefix` | Branch prefix string | Prefix for generated release branches, for example `release/`. Used as a safety boundary by `abort`. |
| `release.tag_prefix` / `release.tag_suffix` | String | Added around the version to create the tag. Source monorepo tags may use `cargo-release-v`; public repositories usually use `v`. |
| `release.name_prefix` / `release.name_suffix` | String | Added around the version to create the GitHub Release title. |
| `release.commit_message` | String template | Commit message for version file updates. |
| `release.merge_message` | String template | Merge commit message used by `finalize`. |
| `release.cleanup` | Boolean | Deletes the release branch after success unless `--keep-branch` is used. |
| `release.latest` | Boolean | Controls whether the GitHub Release should be marked as latest. |
| `release.floating_tags` | Boolean | Enables stable moving major/minor tags such as `v1.2` and `v1` for public releases. Defaults to `false` and is ignored for prereleases. |
| `github.target_repository` | `owner/repo` or empty | Repository where the GitHub Release is created. Empty means the current repository context. |
| `github.source_repository` | `owner/repo` or empty | Repository used for generated release notes. Useful when distribution repositories are source-free. |
| `github.source_tag_prefix` / `github.source_tag_suffix` | String | Source tag naming when release notes should be generated from a different repository. |
| `github.generate_notes` | Boolean | Use GitHub-generated notes when no custom body is provided. Set this to `false` to create a release without a description unless `github.notes_body`, `--notes`, or `--notes-file` is supplied. |
| `github.notes_body` | String template | Optional custom GitHub Release body. When non-empty, it takes precedence over generated notes. Supported placeholders are `{version}`, `{tag}`, `{release_name}`, `{target_repository}`, `{source_repository}`, `{source_tag}`, `{previous_source_tag}`, and `{source_compare_url}`. |
| `files` | Array | Version files to update during `prepare`. Use an empty array for source-free distribution repositories. |
| `files[].kind` | `toml`, `json`, `text` | File update strategy. |
| `files[].key` | Key path or search text | TOML/JSON key path or text target depending on kind. |
| `files[].value` | String template | New value to write. |
| `files[].optional` | Boolean | When `true`, missing files are skipped instead of failing the release. |

## Practical workflows

### Practical release workflows

### Source monorepo release

Use this flow when the repository contains the actual source code and version files.

```sh
github-release prepare --version 1.4.0 --config crates/my-tool/github-release.toml
cargo test --workspace
github-release finalize --version 1.4.0 --config crates/my-tool/github-release.toml --skip-github-release
```

`prepare` creates the release branch and updates configured version files before the build starts. `finalize` merges the release branch and creates the source tag only after the build and tests succeed.

### Public distribution release

Use this flow when a public repository only receives generated files and binary assets.

```sh
github-release publish --version 1.4.0 --config crates/my-tool/github-release.toml --assets dist/my-tool
```

`publish` does not create source branches or edit source files. It creates a GitHub Release from an existing release context and uploads assets.

When `release.floating_tags = true`, a stable publish also updates moving tags. For `v1.4.0`, the public repository receives or refreshes `v1.4` and `v1` so GitHub Action users can pin to a major or minor line. Prerelease versions such as `v1.4.0-rc.1` are ignored by floating tag updates.

Backfill missing floating tags after older releases already exist:

```sh
github-release floating-tags --config crates/my-tool/github-release.toml --all
```

Use a custom release body when the public repository should not show generated notes:

```sh
github-release publish \
  --version 1.4.0 \
  --config crates/my-tool/github-release.toml \
  --assets dist/my-tool \
  --notes "This version was developed in \`verzly/toolchain\`.

Source changes for this package can be reviewed from \`{previous_source_tag}\` to \`{source_tag}\`:
{source_compare_url}"
```

The same body can be stored in `github.notes_body`. In that case, `publish` renders the placeholders at release time and uses the result as the GitHub Release description.

### Failed build cleanup

```sh
github-release abort --version 1.4.0 --config crates/my-tool/source-github-release.toml
```

`abort` deletes only the configured release branch by default. Use `--allow-any-branch` only for manual recovery when you have verified the branch name.

## Reference

### Troubleshooting

If `prepare` fails because the release branch already exists, inspect the branch before using `--force-branch`. If release notes cannot be generated from a private source repository, the tool writes fallback notes instead of silently publishing misleading content. If asset upload fails, verify that `gh` is authenticated and that the `--assets` directory contains files rather than empty directories.

### Release artifacts

Release assets are named by tool, version, and host target. Typical examples:

```text
github-release-v1.2.3-x86_64-unknown-linux-gnu
github-release-v1.2.3-aarch64-unknown-linux-gnu
github-release-v1.2.3-x86_64-apple-darwin
github-release-v1.2.3-aarch64-apple-darwin
github-release-v1.2.3-x86_64-pc-windows-msvc.exe
```

Checksum files use the same name with `.sha256` appended. The action verifies them when the runner has `sha256sum` or `shasum`.

### Operational notes

`github-release` shells out to `git` and `gh`. CI jobs must check out the repository with enough history and must provide a token that can push branches, push tags, and create releases. For distribution repositories, the token also needs access to the public target repository.

## Contributing

Contribution guidelines live in the `verzly/toolchain` `CONTRIBUTING.md`. Source changes are made in `verzly/toolchain`; this repository is the public distribution surface.

## License

This project is licensed under the AGPL-3.0-only license.
