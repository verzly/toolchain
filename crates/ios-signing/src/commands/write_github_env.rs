//! Writes non-password CI values to `$GITHUB_ENV`. Passwords stay outside this command intentionally.

use crate::cli::WriteGithubEnvArgs;
use crate::ios;
use anyhow::{Context, Result};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;

pub fn run(args: WriteGithubEnvArgs) -> Result<()> {
    let path = env::var("GITHUB_ENV").context("GITHUB_ENV is not set")?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(
        file,
        "IOS_SIGNING_CERTIFICATE_BASE64={}",
        ios::file_base64(&args.certificate)?
    )?;
    writeln!(
        file,
        "IOS_SIGNING_PROVISIONING_PROFILE_BASE64={}",
        ios::file_base64(&args.provisioning_profile)?
    )?;
    Ok(())
}
