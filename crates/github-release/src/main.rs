//! Standalone compatibility entry point for `github-release`.

fn main() -> anyhow::Result<()> {
    github_release::run_from(std::env::args_os())
}
