//! Thin wrapper around Android `keytool` operations and keystore encoding. Keep command construction here for auditability.

use anyhow::{Context, Result};
use base64::Engine;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
// Android signing keys are long-lived; generation refuses to overwrite unless the caller opts in.
pub fn keytool_available() -> bool {
    Command::new("keytool")
        .arg("-help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub struct GenerateKeystore<'a> {
    pub output: &'a Path,
    pub alias: &'a str,
    pub store_type: &'a str,
    pub key_alg: &'a str,
    pub key_size: u32,
    pub validity: u32,
    pub dname: &'a str,
    pub store_password: &'a str,
    pub key_password: &'a str,
}

pub fn generate_keystore(args: GenerateKeystore<'_>, dry_run: bool) -> Result<()> {
    let key_size = args.key_size.to_string();
    let validity = args.validity.to_string();
    let output = args.output.display().to_string();
    let command_args = vec![
        "-genkeypair".to_string(),
        "-v".to_string(),
        "-keystore".to_string(),
        output,
        "-storetype".to_string(),
        args.store_type.to_string(),
        "-alias".to_string(),
        args.alias.to_string(),
        "-keyalg".to_string(),
        args.key_alg.to_string(),
        "-keysize".to_string(),
        key_size,
        "-validity".to_string(),
        validity,
        "-dname".to_string(),
        args.dname.to_string(),
        "-storepass".to_string(),
        args.store_password.to_string(),
        "-keypass".to_string(),
        args.key_password.to_string(),
    ];

    // Never print raw passwords in dry-run output. Contributors should be able to debug commands safely.
    if dry_run {
        println!(
            "keytool -genkeypair -v -keystore {} -storetype {} -alias {} -keyalg {} \
             -keysize {} -validity {} -dname <hidden> -storepass <hidden> -keypass <hidden>",
            args.output.display(),
            args.store_type,
            args.alias,
            args.key_alg,
            args.key_size,
            args.validity
        );
        return Ok(());
    }

    let status = Command::new("keytool")
        .args(command_args)
        .status()
        .context("failed to run keytool")?;
    if !status.success() {
        anyhow::bail!("keytool failed to generate keystore");
    }
    Ok(())
}

pub fn keystore_base64(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

pub fn fingerprint(path: &Path, alias: &str, store_password: &str) -> Result<()> {
    let command_args = vec![
        "-list".to_string(),
        "-v".to_string(),
        "-keystore".to_string(),
        path.display().to_string(),
        "-alias".to_string(),
        alias.to_string(),
        "-storepass".to_string(),
        store_password.to_string(),
    ];
    let status = Command::new("keytool")
        .args(command_args)
        .status()
        .context("failed to run keytool")?;

    if !status.success() {
        anyhow::bail!("keytool failed to print fingerprint");
    }
    Ok(())
}

pub fn fingerprint_sha256(path: &Path, alias: &str, store_password: &str) -> Result<String> {
    let command_args = vec![
        "-list".to_string(),
        "-v".to_string(),
        "-keystore".to_string(),
        path.display().to_string(),
        "-alias".to_string(),
        alias.to_string(),
        "-storepass".to_string(),
        store_password.to_string(),
    ];
    let output = Command::new("keytool")
        .args(command_args)
        .stdin(Stdio::null())
        .output()
        .context("failed to run keytool")?;

    if !output.status.success() {
        anyhow::bail!("keytool failed to print fingerprint");
    }

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_sha256_fingerprint(&combined)
        .context("keytool output did not contain a SHA-256 fingerprint")
}

pub fn verify_sha256_fingerprint(
    path: &Path,
    alias: &str,
    store_password: &str,
    expected: &str,
) -> Result<()> {
    let actual = fingerprint_sha256(path, alias, store_password)?;
    let expected = normalize_fingerprint(expected);
    let actual = normalize_fingerprint(&actual);

    if actual != expected {
        anyhow::bail!("fingerprint mismatch: expected {expected}, got {actual}");
    }

    Ok(())
}

fn parse_sha256_fingerprint(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix("SHA256:")
            .or_else(|| line.strip_prefix("SHA-256:"))
            .map(str::trim)
            .map(ToOwned::to_owned)
    })
}

fn normalize_fingerprint(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_hexdigit())
        .flat_map(|character| character.to_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn encodes_keystore_as_base64() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("android-signing-keystore-{suffix}.jks"));
        std::fs::write(&path, b"keystore").expect("write keystore");

        assert_eq!(
            keystore_base64(&path).expect("encode keystore"),
            "a2V5c3RvcmU="
        );
    }

    #[test]
    fn parses_and_normalizes_sha256_fingerprint() {
        let output = "\
Alias name: release-key
Certificate fingerprints:
         SHA256: AA:bb:01:ff
";

        assert_eq!(
            parse_sha256_fingerprint(output).expect("sha256"),
            "AA:bb:01:ff"
        );
        assert_eq!(normalize_fingerprint("aa bb:01-ff"), "AABB01FF");
    }
}
