use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn schema_directive_line() -> String {
    match env!("VERZLY_SCHEMA_REF") {
        "local" => "#:schema ./schemas/datarose.toml.schema.json".to_string(),
        reference => format!(
            "#:schema https://raw.githubusercontent.com/verzly/toolchain/{reference}/schemas/datarose.toml.schema.json"
        ),
    }
}

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
        .stdout(predicate::str::contains("- verzly"))
        .stdout(predicate::str::contains("source tag: vX.Y.Z"))
        .stdout(predicate::str::contains("public repo: verzly/toolchain"))
        .stdout(predicate::str::contains(
            "action surface: action.yml, actions/",
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
fn check_rejects_legacy_distribution_directory() {
    let repo = fixture_repo();
    fs::create_dir_all(repo.path().join(".verzly/distributions/api")).unwrap();

    let mut cmd = Command::cargo_bin("repository").expect("repository binary");

    cmd.arg("check")
        .arg("--root")
        .arg(repo.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(".verzly"))
        .stderr(predicate::str::contains("distributions"))
        .stderr(predicate::str::contains("unified root action"));
}

#[test]
fn check_allows_custom_release_without_toolchain_workflows() {
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
    fs::create_dir_all(root.join(".github/workflows")).unwrap();
    fs::create_dir_all(root.join("schemas")).unwrap();
    fs::write(root.join("schemas/datarose.toml.schema.json"), "{}").unwrap();

    fs::write(
        root.join("datarose.toml"),
        format!(
            "{}\n{}",
            schema_directive_line(),
            r#"version = 1

[quality]
workspace = "."
languages = []

[release]
enabled = true
target_branch = "main"
source_repository = "acme/platform"
release_all = false
manage_cargo_packages = false
manage_workflows = false

[[release.targets]]
name = "api"
path = "packages/api"
strategy = "same-repo"
workflow = "custom"
source_kind = "custom"
repository = "acme/platform"
version_files = false
source_tag_prefix = "api-v"
include_scopes = ["api", "all"]
include_paths = ["packages/api/"]

[rust_cache.cache]
package = "platform"
"#,
        ),
    )
    .unwrap();

    repo
}
