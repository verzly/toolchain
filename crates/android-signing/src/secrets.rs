//! Secret input helpers. Centralizing prompts and generated passwords makes secret handling easier to review.

use anyhow::Result;
use rand::distr::{Alphanumeric, SampleString};

pub fn random_password() -> String {
    Alphanumeric.sample_string(&mut rand::rng(), 32)
}

pub fn prompt_password(label: &str) -> Result<String> {
    Ok(rpassword::prompt_password(format!("{label}: "))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_alphanumeric_passwords_with_expected_length() {
        let password = random_password();

        assert_eq!(password.len(), 32);
        assert!(password
            .chars()
            .all(|character| character.is_ascii_alphanumeric()));
    }
}
