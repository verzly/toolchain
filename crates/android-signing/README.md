# verzly/android-signing

`verzly/android-signing` helps create and inspect the Android release signing material used by APK and AAB builds.

Android signing keys are long-lived. If the key changes, future APKs cannot update the already installed application. This is easy to forget when the first release is still small and local. This tool makes that first setup explicit.

It is not an Android build tool. It does not build APKs. It prepares the keystore and the CI values that Android and Tauri release builds need.

Use it for the small but important setup around Android release keys: generating the keystore, exporting it for CI, printing the certificate fingerprint, and checking that the local Android tooling is available.

Generate the key once, store it safely, and reuse it for every future Android release. The tool keeps the process explicit because changing the signing key later can prevent users from updating an already installed app.

- [How it works](#how-it-works)
  - [Android update compatibility](#android-update-compatibility)
  - [Keystore storage](#keystore-storage)
  - [CI secrets](#ci-secrets)
  - [Verification](#verification)
- [Get started](#get-started)
  - [Install](#install)
  - [Generate a keystore](#generate-a-keystore)
  - [Export for GitHub Actions](#export-for-github-actions)
- [Usage](#usage)
  - [Generate](#generate)
  - [Base64](#base64)
  - [Print secrets](#print-secrets)
  - [Write GitHub env](#write-github-env)
  - [Fingerprint](#fingerprint)
  - [Doctor](#doctor)
- [GitHub Actions](#github-actions)
- [Compatibility](#compatibility)
- [Known issues](#known-issues)
- [Contributing](#contributing)

Read on if this is your first Android signing setup. Jump to [Generate](#generate) if you already know the flow.

## How it works

Android uses both the package name and the signing certificate to decide whether an installed app can be updated.

```text
same package name + same signing key + higher versionCode = update allowed
same package name + different signing key                    = update rejected
```

### Android update compatibility

This works:

```text
v1.0.0 -> com.acme.app -> key A
v1.1.0 -> com.acme.app -> key A
```

This does not work:

```text
v1.0.0 -> com.acme.app -> key A
v1.1.0 -> com.acme.app -> key B
```

The release key is part of the app identity.

### Keystore storage

The keystore contains a private signing key.

```text
android-release.jks
```

Do not commit it. Do not publish it. Do not send it through chat. Store it in a password manager or a secure secret store with the passwords.

### CI secrets

For GitHub Actions, the usual values are:

```text
ANDROID_KEYSTORE_BASE64
ANDROID_KEYSTORE_PASSWORD
ANDROID_KEY_ALIAS
ANDROID_KEY_PASSWORD
```

Base64 is only a transport format. It is not encryption.

### Verification

Before publishing a replacement APK, compare the certificate fingerprint with the previous release key.

## Get started

### Install

```sh
cargo install --git https://github.com/verzly/android-signing
```

### Generate a keystore

```sh
android-signing generate --output android-release.jks --alias release-key
```

### Export for GitHub Actions

```sh
android-signing base64 android-release.jks --output android-release.jks.base64
android-signing print-secrets android-release.jks --alias release-key
```

Store the printed values as repository secrets. Do not commit the files.

## Usage

### Generate

Prompt for passwords:

```sh
android-signing generate --output android-release.jks
```

Generate random passwords and print them once:

```sh
android-signing generate --output android-release.jks --generate-passwords
```

### Base64

```sh
android-signing base64 android-release.jks
```

Write to a file:

```sh
android-signing base64 android-release.jks --output android-release.jks.base64
```

### Print secrets

```sh
android-signing print-secrets android-release.jks --alias release-key
```

This prints the secret names and the base64 value. Passwords are not guessed. You still need to store the passwords that were used when generating the keystore.

### Write GitHub env

Inside GitHub Actions:

```sh
android-signing write-github-env android-release.jks --alias release-key
```

This writes values to `$GITHUB_ENV`.

### Fingerprint

```sh
android-signing fingerprint android-release.jks --alias release-key
```

### Doctor

```sh
android-signing doctor
```

`doctor` checks whether `keytool` is available.

## GitHub Actions

This repository includes `action.yml` so signing helper commands can run inside GitHub Actions as well as on a local machine.

The release workflow builds this tool through the same Verzly release chain used by the other projects: `cargo-release` creates the executable artifacts and `github-release` publishes them.

## Compatibility

`android-signing` belongs near the Android release step. It is useful before `verzly/tauri-release` builds Android artifacts and before `verzly/github-release` publishes those artifacts.

It does not build Tauri apps and it does not publish releases. It prepares and inspects signing material so the release workflow can keep that responsibility separate from the application builder.

## Known issues

This project depends on `keytool`. It must be available locally or in CI.

The tool does not manage Google Play App Signing. It is focused on self-managed Android release signing material and CI export.

## Contributing

Keep secret handling boring. Avoid hidden defaults that make it unclear which password or alias was used.

## License

`verzly/android-signing` is released under the GNU Affero General Public License v3.0 only.
