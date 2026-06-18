//! Standalone compatibility entry point for `repository`.

fn main() -> anyhow::Result<()> {
    repository::run_from(std::env::args_os())
}
