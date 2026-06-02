# verzly/github-release

`verzly/github-release` is a small release lifecycle helper for projects that need a clean GitHub Release flow, but still want to keep the build itself project-specific.

It does not try to build Rust, Tauri, Android, or frontend artifacts. Other tools should do that. This tool prepares the release branch before the build starts, and finalizes the GitHub Release after the build has already succeeded.

Use it when the release process is always the same, but the build process is not. A release starts on a temporary branch, version files are updated there, and the project build can run before anything reaches `master`. If the build succeeds, the branch is merged back, tagged, and published as a GitHub Release. If it fails, the temporary branch can be removed without touching the target branch.

The tag format and the release title are configured separately. That keeps monorepo tags technical when they need to be, while keeping the GitHub Release name readable.

- [How it works](#how-it-works)
  - [Release branches](#release-branches)
  - [Version files](#version-files)
  - [Build insertion point](#build-insertion-point)
  - [Finalize](#finalize)
  - [Abort](#abort)
- [Get started](#get-started)
  - [Install](#install)
  - [Create config](#create-config)
  - [First release](#first-release)
- [Usage](#usage)
  - [Plan a release](#plan-a-release)
  - [Prepare a release branch](#prepare-a-release-branch)
  - [Finalize after the build](#finalize-after-the-build)
  - [Abort after a failed build](#abort-after-a-failed-build)
  - [Tag and release name formatting](#tag-and-release-name-formatting)
  - [Configuration](#configuration)
- [GitHub Actions](#github-actions)
- [Compatibility](#compatibility)
- [Known issues](#known-issues)
- [Contributing](#contributing)

Read on to understand the flow, or jump straight to [Get started](#get-started) if you already know why you need a two-step release.

## How it works

A release build should not change `master` until every required artifact has been built successfully.

`verzly/github-release` keeps that boundary clear:

```text
target branch       -> master
release branch      -> release/v1.2.3
version changes     -> committed only on the release branch
build jobs          -> run on the release branch
success             -> merge release branch into target branch
release tag         -> created on the final target branch commit
GitHub Release      -> published from the tag
failure             -> delete the release branch without merging
```

The normal lifecycle is:

```text
prepare -> your build -> finalize
```

The failure lifecycle is:

```text
prepare -> failed build -> abort
```

### Release branches

`prepare` creates a branch from the configured target branch. By default, the branch name is derived from the final tag:

```text
release/v1.2.3
```

The branch is meant to be temporary. It is the place where generated version changes can live while the build jobs are running.

### Version files

Version files are configured explicitly. The tool does not scan your repository and guess what should be changed.

A configured file can be TOML, JSON, or plain text. TOML and JSON files use dotted keys. Text files use literal replacement.

### Build insertion point

The build is intentionally outside this project.

A Rust CLI can call `cargo-release` after `prepare`. A Tauri app can call `tauri-release`. A custom project can run its own scripts. The only important detail is that the build checks out the release branch created by `prepare`.

### Finalize

`finalize` checks out the target branch, merges the release branch, pushes the merge, creates the tag, creates the GitHub Release, uploads assets, and optionally removes the temporary release branch.

### Abort

`abort` deletes the temporary release branch. It refuses to delete arbitrary branch names unless you explicitly allow it.

## Get started

### Install

Build the executable from source:

```sh
cargo install --git https://github.com/verzly/github-release
```

Or use the binary produced by your own release workflow.

### Create config

Create the default config:

```sh
github-release init
```

This writes `github-release.toml`.

### First release

Plan the release first:

```sh
github-release plan --version 1.2.3
```

Prepare the release branch:

```sh
github-release prepare --version 1.2.3
```

Run your project build on the generated branch. After the build succeeds, finalize the release:

```sh
github-release finalize --version 1.2.3 --assets dist
```

If the build fails:

```sh
github-release abort --version 1.2.3
```

## Usage

### Plan a release

`plan` prints the resolved version, tag, release name, branch, target branch, and file updates without changing the repository.

```sh
github-release plan --version 1.2.3-rc.1
```

Prerelease versions are detected from SemVer. `1.2.3-rc.1` becomes a GitHub prerelease by default.

### Prepare a release branch

```sh
github-release prepare --version 1.2.3
```

By default this command:

1. checks that the worktree is clean,
2. fetches the target branch,
3. creates a release branch,
4. updates configured version files,
5. commits the changes,
6. pushes the release branch,
7. writes GitHub Actions outputs when running in CI.

Use `--dry-run` to inspect the Git commands without running them.

### Finalize after the build

```sh
github-release finalize --version 1.2.3 --assets dist
```

The command expects the build artifacts to already exist. It does not build anything.

### Abort after a failed build

```sh
github-release abort --version 1.2.3
```

This is safe by default. The branch must match the configured release branch prefix.

### Tag and release name formatting

Tag and release names are intentionally separate.

```toml
[release]
tag_prefix = "desktop-v"
tag_suffix = ""
name_prefix = "Desktop v"
name_suffix = ""
```

For version `1.2.3`, the resolved values are:

```text
tag:          desktop-v1.2.3
release name: Desktop v1.2.3
```

If `name_prefix` is empty, it falls back to `tag_prefix`. If `name_suffix` is empty, it falls back to `tag_suffix`.

This is useful in monorepos where the tag needs a technical prefix, while the release name should remain readable.

### Configuration

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

[[files]]
path = "Cargo.toml"
kind = "toml"
key = "package.version"
value = "{version}"

[[files]]
path = "package.json"
kind = "json"
key = "version"
value = "{version}"

[[files]]
path = "VERSION"
kind = "text"
search = "{current}"
replace = "{version}"
```

`{version}` is the raw SemVer version without the configured tag prefix. `{tag}` is the full tag.

## GitHub Actions

This repository includes `action.yml` so the tool can be used directly from GitHub Actions as well as from a local shell.

The release workflow is intentionally built around the same tools that the project is about. `github-release` prepares and publishes its own GitHub releases, while `cargo-release` builds the executable artifacts and `rust-cache` keeps local build output out of the repository root.

A release for this project may involve artifacts produced by `cargo-release`, `tauri-release`, `android-signing`, or other project-specific build steps. The important part is that `github-release prepare` runs before those builds, and `github-release finalize` runs only after the artifacts are ready.

## Compatibility

`github-release` is designed to sit at the end of the Verzly release flow. It does not replace the builders; it coordinates the branch, version update, tag, and GitHub Release.

It works well with `verzly/cargo-release` for Rust binaries, `verzly/tauri-release` for Tauri application bundles, `verzly/rust-cache` for project-local build cache routing, and `verzly/android-signing` when Android signing material has to be prepared before release artifacts are built.

## Known issues

This project uses `git` and `gh` as external executables instead of reimplementing Git and GitHub APIs. That keeps the code smaller and easier to audit, but it also means both tools must be available in CI.

The tool does not try to resolve merge conflicts. A release branch should be created from the current target branch and finalized shortly after the build finishes.

## Contributing

Keep the release flow boring. The project should remain small enough that a senior developer can read the code in one sitting and understand where every side effect happens.

New features should fit into the prepare, finalize, or abort lifecycle without hiding Git operations behind surprising behavior.

## License

`verzly/github-release` is released under the GNU Affero General Public License v3.0 only.
