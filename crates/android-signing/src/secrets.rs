//! Secret input helpers. Centralizing prompts and generated passwords makes secret handling easier to review.

use anyhow::Result;
use rand::distributions::{Alphanumeric, DistString};

pub fn random_password() -> String {
    Alphanumeric.sample_string(&mut rand::thread_rng(), 32)
}

pub fn prompt_password(label: &str) -> Result<String> {
    Ok(rpassword::prompt_password(format!("{label}: "))?)
}
