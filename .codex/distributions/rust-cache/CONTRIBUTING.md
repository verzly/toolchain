# Contributing

This repository is a public distribution surface for a Verzly toolchain executable.

Source code, tests, release workflows, release configuration, and public documentation templates are maintained in `verzly/toolchain`. Public distribution repositories receive synchronized `README.md`, `CONTRIBUTING.md`, `action.yml`, and `LICENSE` files from that workspace.

Do not send source-code changes directly to this distribution repository. For bugs, documentation issues, or release asset problems, open an issue in the matching public repository with the tool name, version, operating system, command, expected behavior, and actual output.

## Public repository scope

The public distribution repository contains only:

```text
README.md
CONTRIBUTING.md
action.yml
LICENSE
GitHub Release assets
```

It intentionally does not contain Rust source code, Cargo manifests, build workflows, internal release configuration, or maintainer scripts.

## Documentation changes

Documentation changes are made in the matching `.codex/distributions/<tool>` template inside `verzly/toolchain`, then synchronized to the public repository by release or distribution sync workflow.

Keep the public `README.md` focused on installation, action usage, CLI usage, practical workflows, troubleshooting, operational notes, and license information. Keep contribution and development-process details in this file.

## Security

Do not include real signing keys, tokens, passwords, keystores, base64-encoded secrets, private release assets, or repository tokens in issues, pull requests, comments, screenshots, or logs.
