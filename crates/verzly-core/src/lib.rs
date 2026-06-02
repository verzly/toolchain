//! Shared helpers for the Verzly release tools.
//!
//! Keep this crate boring. It exists to prevent small process, platform, and
//! artifact rules from drifting between the CLI tools.

pub mod artifact;
pub mod platform;
pub mod process;
