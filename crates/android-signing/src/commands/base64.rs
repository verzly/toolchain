//! Exports the keystore as base64 for CI transport. Base64 is not encryption; docs should keep saying that.

use crate::android;
use crate::cli::Base64Args;
use anyhow::Result;
use std::fs;

pub fn run(args: Base64Args) -> Result<()> {
    let value = android::keystore_base64(&args.path)?;
    if let Some(output) = args.output {
        fs::write(output, format!("{value}\n"))?;
    } else {
        println!("{value}");
    }
    Ok(())
}
