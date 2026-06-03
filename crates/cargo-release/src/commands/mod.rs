//! Command module registry. Keeping subcommands separate avoids turning `main.rs` into the application.

pub mod build;
pub mod clean;
pub mod doctor;
pub mod init;
pub mod plan;
