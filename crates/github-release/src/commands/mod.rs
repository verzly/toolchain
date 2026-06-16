//! Command module registry. One file per top-level CLI command keeps the release lifecycle easy to follow.

pub mod abort;
pub mod delete;
pub mod finalize;
pub mod floating_tags;
pub mod init;
pub mod plan;
pub mod prepare;
pub mod publish;
