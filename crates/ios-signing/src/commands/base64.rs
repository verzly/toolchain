//! Exports a signing file as base64 for CI transport. Base64 is not encryption.

use crate::cli::Base64Args;
use crate::ios;
use anyhow::Result;
use std::fs;

pub fn run(args: Base64Args) -> Result<()> {
    let value = ios::file_base64(&args.path)?;
    if let Some(output) = args.output {
        fs::write(output, format!("{value}\n"))?;
    } else {
        println!("{value}");
    }
    Ok(())
}
