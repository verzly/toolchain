# ios-signing

`ios-signing` prepares and validates the CI secret values used by iOS release signing workflows. Source code lives in `verzly/toolchain`; this public repository is only the distribution surface for the executable, GitHub Action, README, CONTRIBUTING, and LICENSE files.

- [Overview](#overview)
- [Get started](#get-started)
- [Usage](#usage)
- [Configuration](#configuration)
- [Reference](#reference)

## Overview

Apple signing assets are long-lived and sensitive. iOS release workflows usually need a `.p12` signing certificate, the certificate password, a provisioning profile, a temporary keychain password, and the Apple Team ID. This tool keeps the repeatable parts small and auditable:

- encode existing certificate and provisioning profile files as base64 values for GitHub Secrets;
- print the exact secret names a workflow should configure;
- validate required CI environment variables before a release job reaches the signing step;
- expose the same checks through a standalone executable and a composite GitHub Action.

The tool does not create Apple Developer certificates or provisioning profiles. Generate those in Apple Developer/Xcode, export them locally, then use `ios-signing` to prepare the workflow secrets.

## Get started

Use the action to install the executable:

```yaml
- uses: verzly/ios-signing@v1
  with:
    install-only: "true"
```

Check signing secrets in a release job before running Tauri iOS packaging:

```yaml
- uses: verzly/ios-signing@v1
  with:
    check-signing-secrets: "true"
    install-only: "true"
  env:
    IOS_SIGNING_CERTIFICATE_BASE64: ${{ secrets.IOS_SIGNING_CERTIFICATE_BASE64 }}
    IOS_SIGNING_CERTIFICATE_PASSWORD: ${{ secrets.IOS_SIGNING_CERTIFICATE_PASSWORD }}
    IOS_SIGNING_PROVISIONING_PROFILE_BASE64: ${{ secrets.IOS_SIGNING_PROVISIONING_PROFILE_BASE64 }}
    IOS_SIGNING_KEYCHAIN_PASSWORD: ${{ secrets.IOS_SIGNING_KEYCHAIN_PASSWORD }}
    APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
```

Generate secret values from local files:

```bash
ios-signing print-secrets \
  --certificate ios-release.p12 \
  --provisioning-profile AppStore.mobileprovision
```

## Usage

### GitHub Action inputs

| Input | Required | Default | Accepted values | Description |
| --- | --- | --- | --- | --- |
| `github-token` | No | `${{ github.token }}` | Any token that can read public release assets | Token used to download executable assets. |
| `version` | No | empty | `1.2.3`, `v1.2.3`, `latest`, `next`, `v1`, `v1.2` | Release selector for the executable. Empty uses the action ref when possible, otherwise latest. |
| `install-only` | No | `false` | `true`, `false` | Install the executable and add it to `PATH` without running `args`. |
| `args` | No | `--help` | Any `ios-signing` CLI arguments | Arguments passed to the executable when `install-only` is false. |
| `working-directory` | No | `.` | Any existing directory | Directory where the executable should run. |
| `check-signing-secrets` | No | `false` | `true`, `false` | Run `ios-signing check-env` before `args`. |
| `require-apple-team-id` | No | `true` | `true`, `false` | Require `APPLE_TEAM_ID` during secret preflight. |

### GitHub Action outputs

| Output | Value | Description |
| --- | --- | --- |
| `bin-path` | Absolute executable path | Path to the installed `ios-signing` binary. |
| `host-target` | `linux-x64`, `macos-x64`, `macos-arm64`, or `windows-x64` | Release asset target selected for the current runner. |

### CLI commands

```bash
ios-signing doctor
```

Prints whether common macOS signing tools such as `security` and `xcodebuild` are available. Non-macOS hosts can still encode files and validate CI environment variables.

```bash
ios-signing base64 <path> [--output encoded.txt]
```

Encodes a certificate or provisioning profile as base64. Base64 is a transport format only; store the output as a secret.

```bash
ios-signing print-secrets \
  --certificate ios-release.p12 \
  --provisioning-profile AppStore.mobileprovision
```

Prints `IOS_SIGNING_CERTIFICATE_BASE64`, `IOS_SIGNING_PROVISIONING_PROFILE_BASE64`, and placeholders for password/team secrets.

```bash
ios-signing write-github-env \
  --certificate ios-release.p12 \
  --provisioning-profile AppStore.mobileprovision
```

Writes the two non-password base64 values to `$GITHUB_ENV`. Passwords and Team ID stay intentionally outside this command.

```bash
ios-signing check-env [--skip-apple-team-id] [--require NAME]
```

Validates that required environment variables are present and non-empty without printing secret values. `--require NAME` can be repeated for project-specific signing variables.

## Configuration

The default required CI variables are:

| Variable | Purpose |
| --- | --- |
| `IOS_SIGNING_CERTIFICATE_BASE64` | Base64-encoded `.p12` signing certificate. |
| `IOS_SIGNING_CERTIFICATE_PASSWORD` | Password used when exporting the `.p12` certificate. |
| `IOS_SIGNING_PROVISIONING_PROFILE_BASE64` | Base64-encoded `.mobileprovision` profile. |
| `IOS_SIGNING_KEYCHAIN_PASSWORD` | Temporary keychain password used by the release workflow. |
| `APPLE_TEAM_ID` | Apple Developer Team ID. |

For Tauri releases, reference the same names from `tauri-release` platform `required_env` so unsupported iOS builds are skipped with a clear reason instead of failing late.

## Reference

### Release artifacts

Releases publish standalone executables named like:

```text
ios-signing-v1.2.3-linux-x64
ios-signing-v1.2.3-macos-x64
ios-signing-v1.2.3-macos-arm64
ios-signing-v1.2.3-windows-x64.exe
```

Each executable may also have a `.sha256` checksum file.

### Security notes

Do not commit certificates, provisioning profiles, encoded secret files, keychains, or passwords. Keep generated values in GitHub Secrets or a trusted secret manager. Rotating an Apple signing certificate may require updating provisioning profiles and release workflow secrets together.

### Troubleshooting

If `check-env` reports a missing value, add it to repository or environment secrets and pass it through `env:` in the workflow step. If `doctor` reports missing `security` or `xcodebuild`, run iOS signing and packaging on a macOS runner with Xcode installed.

## License

AGPL-3.0-only.
