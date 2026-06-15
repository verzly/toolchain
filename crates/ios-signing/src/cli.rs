//! Command-line interface for iOS signing helpers. This file defines inputs only; secret handling lives in commands.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "ios-signing")]
#[command(bin_name = "ios-signing")]
#[command(
    author,
    version,
    about = "iOS release signing secret helper",
    after_help = "Read the full README: https://github.com/verzly/ios-signing"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Base64(Base64Args),
    #[command(name = "print-secrets")]
    PrintSecrets(PrintSecretsArgs),
    #[command(name = "write-github-env")]
    WriteGithubEnv(WriteGithubEnvArgs),
    #[command(name = "check-env")]
    CheckEnv(CheckEnvArgs),
    #[command(after_help = "Read the full README: https://github.com/verzly/ios-signing")]
    Doctor,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/ios-signing")]
pub struct Base64Args {
    /// Path to a .p12 certificate or .mobileprovision profile.
    pub path: PathBuf,

    /// Write the encoded value to a file instead of stdout.
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/ios-signing")]
pub struct PrintSecretsArgs {
    /// Path to the exported Apple signing certificate, usually a .p12 file.
    #[arg(long)]
    pub certificate: PathBuf,

    /// Path to the provisioning profile, usually a .mobileprovision file.
    #[arg(long = "provisioning-profile")]
    pub provisioning_profile: PathBuf,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/ios-signing")]
pub struct WriteGithubEnvArgs {
    /// Path to the exported Apple signing certificate, usually a .p12 file.
    #[arg(long)]
    pub certificate: PathBuf,

    /// Path to the provisioning profile, usually a .mobileprovision file.
    #[arg(long = "provisioning-profile")]
    pub provisioning_profile: PathBuf,
}

#[derive(Args, Debug)]
#[command(after_help = "Read the full README: https://github.com/verzly/ios-signing")]
pub struct CheckEnvArgs {
    /// Do not require APPLE_TEAM_ID. Use this only for workflows that inject the team another way.
    #[arg(long, default_value_t = false)]
    pub skip_apple_team_id: bool,

    /// Additional environment variable names to require. Repeatable.
    #[arg(long = "require")]
    pub required: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::PathBuf;

    #[test]
    fn parses_base64_command() {
        let cli = Cli::parse_from(["ios-signing", "base64", "ios-release.p12"]);

        let Commands::Base64(args) = cli.command else {
            panic!("expected base64 command");
        };

        assert_eq!(args.path, PathBuf::from("ios-release.p12"));
        assert!(args.output.is_none());
    }

    #[test]
    fn parses_print_secrets_paths() {
        let cli = Cli::parse_from([
            "ios-signing",
            "print-secrets",
            "--certificate",
            "ios-release.p12",
            "--provisioning-profile",
            "app.mobileprovision",
        ]);

        let Commands::PrintSecrets(args) = cli.command else {
            panic!("expected print-secrets command");
        };

        assert_eq!(args.certificate, PathBuf::from("ios-release.p12"));
        assert_eq!(
            args.provisioning_profile,
            PathBuf::from("app.mobileprovision")
        );
    }

    #[test]
    fn parses_check_env_options() {
        let cli = Cli::parse_from([
            "ios-signing",
            "check-env",
            "--skip-apple-team-id",
            "--require",
            "APPLE_ID",
        ]);

        let Commands::CheckEnv(args) = cli.command else {
            panic!("expected check-env command");
        };

        assert!(args.skip_apple_team_id);
        assert_eq!(args.required, vec!["APPLE_ID"]);
    }
}
