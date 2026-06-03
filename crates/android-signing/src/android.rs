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
            "keytool -genkeypair -v -keystore {} -storetype {} -alias {} -keyalg {} -keysize {} -validity {} -dname <hidden> -storepass <hidden> -keypass <hidden>",
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
