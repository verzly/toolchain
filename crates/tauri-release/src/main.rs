//! Standalone compatibility entry point for `tauri-release`.

fn main() -> anyhow::Result<()> {
    tauri_release::run_from(std::env::args_os())
}
