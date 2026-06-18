//! Standalone compatibility entry point for `ios-signing`.

fn main() -> anyhow::Result<()> {
    ios_signing::run_from(std::env::args_os())
}
