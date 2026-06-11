# android-signing

`android-signing` prepares and validates Android release signing keys for CI-based Android releases.

This repository is a public distribution repository. The source code is maintained in the private `verzly/toolchain` monorepo. This repository contains the public surface needed by users: `README.md`, `CONTRIBUTING.md`, `action.yml`, `LICENSE`, and GitHub Release assets.

The public repository intentionally does not contain `src/`, `Cargo.toml`, build workflows, or release configuration. That separation keeps the user-facing repository small while allowing all tools to share the same release infrastructure in `verzly/toolchain`.

- [Overview](#overview)
  - [What this tool does](#what-this-tool-does)
  - [What this tool does not do](#what-this-tool-does-not-do)
  - [How Android release signing should work](#how-android-release-signing-should-work)
  - [Local setup versus CI release](#local-setup-versus-ci-release)
- [Get started](#get-started)
  - [GitHub Action](#github-action)
- [Usage](#usage)
  - [Action inputs](#action-inputs)
  - [Action outputs](#action-outputs)
  - [CLI usage](#cli-usage)
  - [Command help](#command-help)
  - [CLI commands and arguments](#cli-commands-and-arguments)
- [Practical workflows](#practical-workflows)
  - [Create a release keystore locally](#create-a-release-keystore-locally)
  - [Prepare GitHub Actions secrets](#prepare-github-actions-secrets)
  - [Validate the release key in CI](#validate-the-release-key-in-ci)
  - [Use with a Tauri Android release job](#use-with-a-tauri-android-release-job)
- [Reference](#reference)
  - [Required GitHub secrets and variables](#required-github-secrets-and-variables)
  - [Troubleshooting](#troubleshooting)
  - [Release artifacts](#release-artifacts)
  - [Security notes](#security-notes)

## Overview

### What this tool does

`android-signing` is a small helper around Android release keystore handling. It helps you:

- create one long-lived Android release keystore for an app;
- export that keystore as base64 for CI secret transport;
- print the signing certificate fingerprint;
- verify in CI that the restored keystore still matches the expected fingerprint;
- write safe, non-password signing values to `$GITHUB_ENV`.

The signing key is the Android release identity of your app. Once an app has been released with a package name and release key, future APK updates for the same package must be signed with the same key.

### What this tool does not do

`android-signing` is not a build tool and not a release manager. It does not:

- build APKs or app bundles;
- run Tauri;
- generate a GitHub Release;
- create GitHub Secrets automatically;
- generate a new signing key during a release workflow;
- replace `tauri-release`, Gradle, or Android Studio.

If required signing secrets are missing in CI, the release should fail. A release workflow must not silently generate a new key, because that would create a different Android app signing identity.

### How Android release signing should work

Use one release keystore per Android app identity. For example:

```text
net.datarose.nutrino.mobile      -> production release key
net.datarose.nutrino.mobile.dev  -> development/debug identity
```

For production releases, the keystore, alias, and signing certificate fingerprint must stay stable across versions.

The typical release chain is:

```text
android-signing  -> prepare and verify the release key
tauri-release    -> build the Android artifact using that key
github-release   -> publish the release only after signing/build checks pass
```

### Local setup versus CI release

Run `android-signing generate`, `fingerprint`, and `print-secrets` locally when setting up a new app release key.

Run `verify-fingerprint` in CI before building a release. CI should only restore and validate an existing key from secrets. It should not create a replacement key.

## Get started

### GitHub Action

Run the tool directly:

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

The composite action detects the runner operating system and CPU architecture, maps that host to a release asset name, downloads the matching executable from this repository's GitHub Releases with `gh release download`, verifies a `.sha256` file when one is present, copies the executable into a temporary bin directory, and adds that directory to `PATH`.

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
| `host-target` | Host target such as `linux-x64`, `macos-arm64`, or `windows-x64` | Shows which release asset was selected for the current runner. |

### CLI usage

```sh
android-signing doctor
android-signing generate --output android-release.jks --alias release-key
android-signing generate --generate-passwords --print-base64
android-signing base64 android-release.jks --output keystore.base64
android-signing fingerprint android-release.jks --alias release-key
android-signing verify-fingerprint android-release.jks --alias release-key --expected-sha256 AA:BB:CC
android-signing print-secrets android-release.jks --alias release-key
android-signing write-github-env android-release.jks --alias release-key
```


### Command help

Every top-level and subcommand help output points back to this README:

```sh
android-signing --help
android-signing <command> --help
```

Use the README for workflow-level guidance and the command help for the exact arguments supported by the installed executable version.

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

#### `verify-fingerprint`

Checks that an existing keystore still matches the expected SHA-256 signing certificate fingerprint.

| Argument | Required | Default | Accepted values | Purpose |
| --- | --- | --- | --- | --- |
| `path` | Yes | none | File path | Keystore file to inspect. |
| `-a`, `--alias` | No | `release-key` | String | Key alias to inspect. |
| `--store-password` | No | Prompted | String | Keystore password. |
| `--expected-sha256` | Yes | none | SHA-256 fingerprint | Expected release signing certificate fingerprint. |

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

### Create a release keystore locally

```sh
android-signing generate --output nutrino-release.jks --alias nutrino-release
android-signing fingerprint nutrino-release.jks --alias nutrino-release
```

The command prompts for passwords when they are not passed as arguments. This is safer for local use because passwords are not left in shell history.

Keep the `.jks` file in a secure offline backup. Do not rely only on GitHub Secrets because secret values cannot be read back later.

### Prepare GitHub Actions secrets

```sh
android-signing base64 nutrino-release.jks --output nutrino-release.jks.base64
android-signing print-secrets nutrino-release.jks --alias nutrino-release
```

Store these values in GitHub Actions secrets:

```text
ANDROID_KEYSTORE_BASE64
ANDROID_KEYSTORE_PASSWORD
ANDROID_KEY_ALIAS
ANDROID_KEY_PASSWORD
```

Store the expected fingerprint as a GitHub Actions variable or secret:

```text
ANDROID_SIGNING_CERT_SHA256
```

Base64 is transport encoding, not encryption. The base64 value is still sensitive and must be stored as a secret.

### Validate the release key in CI

```yaml
- name: Restore Android release keystore
  shell: bash
  run: |
    echo "${{ secrets.ANDROID_KEYSTORE_BASE64 }}" | base64 --decode > "$RUNNER_TEMP/android-release.jks"

- name: Verify Android release signing key
  uses: verzly/android-signing@v1
  with:
    args: >-
      verify-fingerprint
      "$RUNNER_TEMP/android-release.jks"
      --alias "${{ secrets.ANDROID_KEY_ALIAS }}"
      --store-password "${{ secrets.ANDROID_KEYSTORE_PASSWORD }}"
      --expected-sha256 "${{ vars.ANDROID_SIGNING_CERT_SHA256 }}"
```

If any secret is missing or the fingerprint does not match, the release job should fail before building or publishing artifacts.

### Use with a Tauri Android release job

`android-signing` verifies the key. The Android build tool still has to use that key.

```yaml
- name: Build Android release
  uses: verzly/tauri-release@v1
  with:
    platform: android
  env:
    ANDROID_KEYSTORE_PATH: ${{ runner.temp }}/android-release.jks
    ANDROID_KEYSTORE_PASSWORD: ${{ secrets.ANDROID_KEYSTORE_PASSWORD }}
    ANDROID_KEY_ALIAS: ${{ secrets.ANDROID_KEY_ALIAS }}
    ANDROID_KEY_PASSWORD: ${{ secrets.ANDROID_KEY_PASSWORD }}
```

A release finalization job should depend on the Android signing validation and Android build job. If validation fails, do not merge, tag, or publish the release.

## Reference

### Required GitHub secrets and variables

| Name | Type | Purpose |
| --- | --- | --- |
| `ANDROID_KEYSTORE_BASE64` | Secret | Base64-encoded release keystore. |
| `ANDROID_KEYSTORE_PASSWORD` | Secret | Keystore password. |
| `ANDROID_KEY_ALIAS` | Secret | Alias inside the keystore. |
| `ANDROID_KEY_PASSWORD` | Secret | Key password. |
| `ANDROID_SIGNING_CERT_SHA256` | Variable or secret | Expected SHA-256 signing certificate fingerprint. |

### Troubleshooting

If generation fails, run `android-signing doctor` and confirm `keytool` is installed.

If fingerprint inspection fails, verify the alias and store password.

If Android builds cannot read the keystore from CI, check that the base64 value was not wrapped, truncated, or committed as a plain file.

If a release workflow has no Android signing secrets, fix the repository secrets. Do not generate a new release key inside CI as a fallback.

### Release artifacts

Release assets are named by tool, version, and host target. Typical examples:

```text
android-signing-v1.2.3-linux-x64
android-signing-v1.2.3-macos-x64
android-signing-v1.2.3-macos-arm64
android-signing-v1.2.3-windows-x64.exe
```

Checksum files use the same name with `.sha256` appended. The action verifies them when the runner has `sha256sum` or `shasum`.

### Security notes

Android signing keys should be treated as long-lived production credentials. Keep the keystore, store password, and key password in separate CI secrets. Do not commit generated keystores or `.base64` files. Do not share dry-run output that was manually edited to include real secrets.

`android-signing` intentionally does not create GitHub Secrets automatically. It can prepare secret values and validate them later, but adding secrets to a repository is an explicit repository administration step.

## License

This project is licensed under the AGPL-3.0-only license.
