//! Checks local tooling commonly needed for iOS signing. Non-macOS hosts can still encode files and check CI env.

use crate::ios;
use anyhow::Result;

pub fn run() -> Result<()> {
    println!(
        "security: {}",
        if ios::command_available("security") {
            "ok"
        } else {
            "missing"
        }
    );
    println!(
        "xcodebuild: {}",
        if ios::command_available("xcodebuild") {
            "ok"
        } else {
            "missing"
        }
    );
    Ok(())
}
