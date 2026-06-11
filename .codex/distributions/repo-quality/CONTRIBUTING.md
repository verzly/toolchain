# Contributing

Development for `repo-quality` happens in `verzly/toolchain`.

The public `verzly/repo-quality` repository contains the synchronized distribution surface only:

```text
README.md
CONTRIBUTING.md
action.yml
LICENSE
```

## Development

Clone `verzly/toolchain`, then run checks from the workspace root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Run the executable locally with Cargo:

```sh
cargo run -p repo-quality -- plan
cargo run -p repo-quality -- init --dry-run
cargo run -p repo-quality -- doctor
```

## Documentation

User-facing documentation belongs in `.codex/distributions/repo-quality/README.md`.

Contributor and development workflow details belong in this `CONTRIBUTING.md` file, not in the README.
