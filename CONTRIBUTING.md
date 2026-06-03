# Contributing

Source changes for the Verzly toolchain happen in `verzly/toolchain`. The public distribution repositories are release surfaces only and should stay limited to `README.md`, `action.yml`, `LICENSE`, and published release assets.

Before opening or merging a pull request, run the workspace checks locally when possible:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Use focused pull requests, keep responsibilities narrow, and follow Conventional Commits for commit messages and PR titles so package release notes can be generated correctly.
