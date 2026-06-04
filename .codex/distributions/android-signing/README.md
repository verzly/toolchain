# android-signing

`android-signing` helps generate, inspect, and export Android release keystores for CI workflows.

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
- [Practical workflows](#practical-workflows)
  - [Practical signing workflows](#practical-signing-workflows)
- [Reference](#reference)
  - [Troubleshooting](#troubleshooting)
  - [Release artifacts](#release-artifacts)
  - [Security notes](#security-notes)
- [Contributing](#contributing)

## Overview

### Why this exists

Android release signing is easy to get wrong because signing keys are long-lived secrets. Projects need repeatable commands for generating a keystore, printing fingerprints, exporting base64 for CI secrets, and writing non-password environment values without accidentally leaking passwords.

`android-signing` gives those tasks one small executable. It is designed to be used before Tauri Android releases, but it is not tied to Tauri.

### How it works

The tool wraps Android `keytool` operations and keeps secret handling explicit. It can generate a keystore, prompt for passwords when they are not provided, generate random passwords, export the keystore as base64, print SHA fingerprints, and write safe values to `$GITHUB_ENV`.

It never treats base64 as encryption. Base64 output is meant for CI secret transport only. Password values should be stored separately in your CI secret store.

### Use cases

Use `android-signing` when you want to:

- create an Android release keystore for a new app;
- avoid accidentally overwriting an existing signing key;
- export a keystore as base64 for GitHub Actions secrets;
- print a release key fingerprint for Play Console or documentation;
- write non-password Android signing environment variables into GitHub Actions;
- standardize signing setup across Tauri Android projects.

## Get started

### GitHub Action

```yaml
- uses: verzly/android-signing@v1
  with:
    args: doctor
```

Install and use later:

```yaml
- uses: verzly/android-signing@v1
  with:
    install-only: "true"

- run: android-signing fingerprint release.jks --alias release-key
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
android-signing doctor
android-signing generate --output android-release.jks --alias release-key
android-signing generate --generate-passwords --print-base64
android-signing base64 android-release.jks --output keystore.base64
android-signing fingerprint android-release.jks --alias release-key
android-signing print-secrets android-release.jks --alias release-key
android-signing write-github-env android-release.jks --alias release-key
```

### CLI commands and arguments

#### `generate`

Generates a keystore using `keytool -genkeypair`.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `-o`, `--output` | No | `android-release.jks` | File path | Keystore file to create. Existing files are refused unless `--force` is used. |
| `-a`, `--alias` | No | `release-key` | String | Key alias inside the keystore. This value is later used by Android build configuration. |
| `--store-type` | No | `JKS` | Store type accepted by `keytool`, commonly `JKS` or `PKCS12` | Keystore format passed to `keytool`. |
| `--key-alg` | No | `RSA` | Algorithm accepted by `keytool`, commonly `RSA` | Key algorithm. |
| `--key-size` | No | `2048` | Positive integer | Key size in bits. |
| `--validity` | No | `10000` | Positive integer | Validity period in days. |
| `--dname` | No | Android release placeholder DN | Distinguished name passed to `keytool`. |
| `--store-password` | No | Prompted | String | Keystore password. Prefer prompts locally and CI secrets in automation. |
| `--key-password` | No | Prompted | String | Key password. Can be the same as the store password if your Android setup expects that. |
| `--generate-passwords` | No | `false` | Boolean flag | Generate random passwords instead of prompting. Store the generated values immediately. |
| `-f`, `--force` | No | `false` | Boolean flag | Allow overwriting an existing output file. Use with extreme care. |
| `--print-base64` | No | `false` | Boolean flag | Print `ANDROID_KEYSTORE_BASE64=...` after generation. Ignored for dry runs. |
| `--dry-run` | No | `false` | Boolean flag | Print the `keytool` command shape without writing files or printing passwords. |

#### `base64`

Exports a keystore file as base64.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `path` | Yes | none | File path | Keystore file to encode. |
| `-o`, `--output` | No | stdout | File path | Write the base64 value to a file instead of stdout. |

#### `fingerprint`

Prints the key fingerprint by calling `keytool` for an existing keystore.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `path` | Yes | none | File path | Keystore file to inspect. |
| `-a`, `--alias` | No | `release-key` | String | Key alias to inspect. |
| `--store-password` | No | Prompted | String | Keystore password. |

#### `print-secrets`

Prints the environment variable names normally needed by CI. Password placeholders are printed as placeholders, not recovered from the keystore.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `path` | Yes | none | File path | Keystore file to encode as `ANDROID_KEYSTORE_BASE64`. |
| `-a`, `--alias` | No | `release-key` | String | Value printed as `ANDROID_KEY_ALIAS`. |

#### `write-github-env`

Writes non-password signing values to the file referenced by `$GITHUB_ENV`.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `path` | Yes | none | File path | Keystore file to encode. |
| `-a`, `--alias` | No | `release-key` | String | Alias written as `ANDROID_KEY_ALIAS`. |

#### `doctor`

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| none | No | none | none | Checks whether `keytool` is available. |

## Practical workflows

### Practical signing workflows

### Generate a new keystore locally

```sh
android-signing generate --output release.jks --alias release-key
```

The command prompts for passwords when they are not passed as arguments. This is safer for local use because passwords are not left in shell history.

### Prepare GitHub Actions secrets

```sh
android-signing base64 release.jks --output release.jks.base64
android-signing fingerprint release.jks --alias release-key
android-signing print-secrets release.jks --alias release-key
```

Store the base64 keystore value, store password, key password, and alias as separate CI secrets. Base64 is transport encoding, not encryption.

### Write CI environment values

```sh
android-signing write-github-env release.jks --alias release-key
```

This writes non-password values to `$GITHUB_ENV`. Passwords should stay in the secret store and be referenced explicitly by the workflow.

## Reference

### Troubleshooting

If generation fails, run `android-signing doctor` and confirm `keytool` is installed. If fingerprint inspection fails, verify the alias and store password. If Android builds cannot read the keystore from CI, check that the base64 value was not wrapped, truncated, or committed as a plain file.

### Release artifacts

Release assets are named by tool, version, and host target. Typical examples:

```text
android-signing-v1.2.3-x86_64-unknown-linux-gnu
android-signing-v1.2.3-aarch64-unknown-linux-gnu
android-signing-v1.2.3-x86_64-apple-darwin
android-signing-v1.2.3-aarch64-apple-darwin
android-signing-v1.2.3-x86_64-pc-windows-msvc.exe
```

Checksum files use the same name with `.sha256` appended. The action verifies them when the runner has `sha256sum` or `shasum`.

### Security notes

Android signing keys should be treated as long-lived production credentials. Keep the keystore, store password, and key password in separate CI secrets. Do not commit generated keystores or `.base64` files. Do not share dry-run output that was manually edited to include real secrets.

## Contributing

Contribution guidelines live in the `verzly/toolchain` `CONTRIBUTING.md`. Source changes are made in `verzly/toolchain`; this repository is the public distribution surface.

## License

This project is licensed under the AGPL-3.0-only license.
