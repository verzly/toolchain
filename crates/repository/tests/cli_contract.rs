use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[test]
fn help_lists_repository_tui() {
    let mut cmd = Command::cargo_bin("repository").expect("repository binary");

    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("tui"))
        .stdout(predicate::str::contains("release"));
}

#[test]
fn check_accepts_toolchain_repository_contract() {
    let mut cmd = Command::cargo_bin("repository").expect("repository binary");

    cmd.current_dir(workspace_root())
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("datarose configuration is valid."));
}

#[test]
fn plan_prints_release_graph_contract() {
    let mut cmd = Command::cargo_bin("repository").expect("repository binary");

    cmd.current_dir(workspace_root())
        .args(["plan", "--root", "."])
        .assert()
        .success()
        .stdout(predicate::str::contains("Release graph:"))
        .stdout(predicate::str::contains("- repository"))
        .stdout(predicate::str::contains("source tag: repository-vX.Y.Z"))
        .stdout(predicate::str::contains("public repo: verzly/repository"))
        .stdout(predicate::str::contains(
            "distribution path: .codex/distributions/repository",
        ));
}

#[test]
fn tui_renders_command_palette_and_cli_equivalents() {
    let mut cmd = Command::cargo_bin("repository").expect("repository binary");

    cmd.current_dir(workspace_root())
        .arg("tui")
        .write_stdin("/quit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Repository standards command center",
        ))
        .stdout(predicate::str::contains("Mode: PLAN"))
        .stdout(predicate::str::contains("Command palette"))
        .stdout(predicate::str::contains("/projects"))
        .stdout(predicate::str::contains("/customize"))
        .stdout(predicate::str::contains("/targets"))
        .stdout(predicate::str::contains("/release"))
        .stdout(predicate::str::contains("/mode act"))
        .stdout(predicate::str::contains("/refresh"))
        .stdout(predicate::str::contains("Esc/q"))
        .stdout(predicate::str::contains("Ctrl+C"))
        .stdout(predicate::str::contains("repository projects --root ."))
        .stdout(predicate::str::contains(
            "repository update --dry-run --skip-mise-use --skip-hk-install",
        ));
}

#[test]
fn projects_prints_inventory_contract() {
    let mut cmd = Command::cargo_bin("repository").expect("repository binary");

    cmd.current_dir(workspace_root())
        .args(["projects", "--root", "."])
        .assert()
        .success()
        .stdout(predicate::str::contains("Projects"))
        .stdout(predicate::str::contains("Cargo packages:"))
        .stdout(predicate::str::contains("Package"))
        .stdout(predicate::str::contains("Release target"))
        .stdout(predicate::str::contains("repository"))
        .stdout(predicate::str::contains("Release targets:"));
}

#[test]
fn repository_without_subcommand_opens_tui() {
    let mut cmd = Command::cargo_bin("repository").expect("repository binary");

    cmd.current_dir(workspace_root())
        .write_stdin("/quit\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Repository standards command center",
        ))
        .stdout(predicate::str::contains("Command palette"));
}

#[test]
fn check_rejects_unsupported_distribution_files() {
    let repo = fixture_repo();
    fs::write(
        repo.path().join(".codex/distributions/api/AGENTS.md"),
        "do not put agent files here\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("repository").expect("repository binary");

    cmd.arg("check")
        .arg("--root")
        .arg(repo.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "contains unsupported distribution file `AGENTS.md`",
        ))
        .stderr(predicate::str::contains(
            "must not contain AI instruction files",
        ));
}

#[test]
fn check_allows_custom_distribution_release_without_workflows() {
    let repo = fixture_repo();
    fs::remove_dir_all(repo.path().join(".github")).unwrap();

    let mut cmd = Command::cargo_bin("repository").expect("repository binary");

    cmd.arg("check")
        .arg("--root")
        .arg(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("datarose configuration is valid."));
}

#[test]
fn check_rejects_undocumented_action_inputs() {
    let repo = fixture_repo();
    fs::write(
        repo.path().join(".codex/distributions/api/README.md"),
        "# api\n\nSource code lives in `verzly/toolchain`.\n\n| Input | Required |\n| --- | --- |\n| `github-token` | No |\n\n| Output | Value |\n| --- | --- |\n| `bin-path` | path |\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("repository").expect("repository binary");

    cmd.arg("check")
        .arg("--root")
        .arg(repo.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "README.md does not document action input `args`",
        ))
        .stderr(predicate::str::contains(
            "README.md does not document action output `host-target`",
        ));
}

#[test]
fn check_rejects_missing_distribution_source_boundary() {
    let repo = fixture_repo();
    fs::write(
        repo.path().join(".codex/distributions/api/README.md"),
        r#"# api

This repository installs the API binary.

| Input | Required |
| --- | --- |
| `github-token` | No |
| `args` | No |

| Output | Value |
| --- | --- |
| `bin-path` | path |
| `host-target` | host |
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("repository").expect("repository binary");

    cmd.arg("check")
        .arg("--root")
        .arg(repo.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "README.md should explain that source lives in verzly/toolchain",
        ));
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn fixture_repo() -> TempDir {
    let repo = TempDir::new().unwrap();
    let root = repo.path();

    fs::create_dir_all(root.join("packages/api")).unwrap();
    fs::create_dir_all(root.join(".codex/distributions/api")).unwrap();
    fs::create_dir_all(root.join(".github/workflows")).unwrap();

    fs::write(
        root.join("datarose.toml"),
        r#"version = 1

[quality]
workspace = "."
languages = []

[release]
enabled = true
target_branch = "main"
source_repository = "acme/platform"
secret_name = "DISTRIBUTION_REPO_TOKEN"
release_all = true
manage_cargo_packages = false
manage_workflows = false

[[release.targets]]
name = "api"
path = "packages/api"
strategy = "distribution-repo"
workflow = "custom"
source_kind = "custom"
repository = "acme/api"
distribution_path = ".codex/distributions/api"
version_files = false
source_tag_prefix = "api-v"
include_scopes = ["api", "all"]
include_paths = ["packages/api/"]

[rust_cache.cache]
package = "platform"
"#,
    )
    .unwrap();

    fs::write(
        root.join(".codex/distributions/api/CONTRIBUTING.md"),
        "# Contributing\n",
    )
    .unwrap();
    fs::write(
        root.join(".codex/distributions/api/LICENSE"),
        "AGPL-3.0-only\n",
    )
    .unwrap();
    fs::write(
        root.join(".codex/distributions/api/action.yml"),
        r#"name: API
description: Install API tool.

inputs:
  github-token:
    description: Token.
    required: false
    default: ""
  args:
    description: Arguments.
    required: false
    default: "--help"

outputs:
  bin-path:
    description: Path.
    value: ${{ steps.install.outputs.bin-path }}
  host-target:
    description: Host.
    value: ${{ steps.host.outputs.host-target }}

runs:
  using: composite
  steps:
    - shell: bash
      run: echo ok
"#,
    )
    .unwrap();
    fs::write(
        root.join(".codex/distributions/api/README.md"),
        r#"# api

Source code lives in `verzly/toolchain`.

| Input | Required |
| --- | --- |
| `github-token` | No |
| `args` | No |

| Output | Value |
| --- | --- |
| `bin-path` | path |
| `host-target` | host |
"#,
    )
    .unwrap();

    for workflow in [
        "_release-tool.yml",
        "_release-build-assets.yml",
        "sync-distributions.yml",
        "delete-release.yml",
        "update-floating-tags.yml",
        "release-all.yml",
    ] {
        fs::write(
            root.join(".github/workflows").join(workflow),
            "name: Test\n",
        )
        .unwrap();
    }
    fs::write(
        root.join(".github/workflows/release-api.yml"),
        "name: Release API\njobs:\n  release:\n    with:\n      tool: api\n",
    )
    .unwrap();

    repo
}
