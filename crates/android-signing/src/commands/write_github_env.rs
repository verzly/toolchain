//! Writes non-password CI values to `$GITHUB_ENV`. Passwords stay outside this command intentionally.

use crate::android;
use crate::cli::WriteGithubEnvArgs;
use anyhow::{Context, Result};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;

pub fn run(args: WriteGithubEnvArgs) -> Result<()> {
    let path = env::var("GITHUB_ENV").context("GITHUB_ENV is not set")?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "ANDROID_KEYSTORE_BASE64={}", android::keystore_base64(&args.path)?)?;
    writeln!(file, "ANDROID_KEY_ALIAS={}", args.alias)?;
    Ok(())
}
