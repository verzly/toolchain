//! GitHub Actions workflow rendering.

use crate::project::{ProjectProfile, ReleaseTarget};
use crate::standards::ManagedFile;
use std::path::PathBuf;

pub fn render_test_workflow(_profile: &ProjectProfile) -> String {
    r#"name: Test

on:
  pull_request:
    branches:
      - master
      - main

permissions:
  contents: read

concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref_name }}
  cancel-in-progress: true

jobs:
  quality:
    name: Quality
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0

      - name: Stop WIP commits
        shell: bash
        run: |
          set -euo pipefail
          subject="$(git log -1 --pretty=%s)"
          normalized="$(printf '%s' "${subject}" | tr '[:upper:]' '[:lower:]')"
          case "${normalized}" in
            wip|wip:*|wip\ -*|wip\ *)
              echo "::error::WIP commit detected: ${subject}"
              echo "Rename the commit before running the full quality workflow."
              exit 1
              ;;
          esac

      - uses: jdx/mise-action@v4
        with:
          cache: true

      - name: Quality gate
        run: mise exec -- hk check
"#
    .into()
}

pub fn release_workflow_files(profile: &ProjectProfile, force: bool) -> Vec<ManagedFile> {
    if !profile.release_enabled() {
        return Vec::new();
    }

    let mut files = Vec::new();
    files.push(ManagedFile {
        path: profile
            .root
            .join(".github/workflows/_release-datarose-tool.yml"),
        content: render_reusable_release_workflow(profile),
        force,
    });

    for target in &profile.stored_config.release.targets {
        files.push(ManagedFile {
            path: profile
                .root
                .join(format!(".github/workflows/release-{}.yml", target.name)),
            content: render_release_target_workflow(profile, target),
            force,
        });
    }

    if profile.stored_config.release.release_all && profile.stored_config.release.targets.len() > 1
    {
        files.push(ManagedFile {
            path: profile.root.join(".github/workflows/release-all.yml"),
            content: render_release_all_workflow(profile),
            force,
        });
    }

    files
}

fn render_release_target_workflow(profile: &ProjectProfile, target: &ReleaseTarget) -> String {
    let title = title_case(&target.name);
    let secret_name = &profile.stored_config.release.secret_name;
    format!(
        r#"name: Release {title}
run-name: Release {tool} ${{{{ inputs.version }}}}

on:
  workflow_dispatch:
    inputs:
      version:
        description: "Version to release, without the leading v."
        required: true
      prerelease:
        description: "Publish as a prerelease. Leave auto to infer from SemVer prerelease labels."
        required: false
        type: choice
        default: auto
        options:
          - auto
          - "true"
          - "false"

permissions:
  contents: write

jobs:
  release:
    uses: ./.github/workflows/_release-datarose-tool.yml
    with:
      tool: {tool}
      version: ${{{{ inputs.version }}}}
      prerelease: ${{{{ inputs.prerelease }}}}
      cargo-release-config: {cargo_release_config}
      distribution-path: {distribution_path}
    secrets:
      DISTRIBUTION_REPO_TOKEN: ${{{{ secrets.{secret_name} }}}}
"#,
        title = title,
        tool = target.name,
        cargo_release_config = target.cargo_release_config,
        distribution_path = target.distribution_path,
        secret_name = secret_name,
    )
}

fn render_reusable_release_workflow(profile: &ProjectProfile) -> String {
    let target_branch = &profile.stored_config.release.target_branch;
    r#"name: Release Datarose Tool

on:
  workflow_call:
    inputs:
      tool:
        required: true
        type: string
      version:
        required: true
        type: string
      prerelease:
        required: false
        type: string
        default: auto
      cargo-release-config:
        required: true
        type: string
      distribution-path:
        required: false
        type: string
        default: ""
    secrets:
      DISTRIBUTION_REPO_TOKEN:
        required: false

permissions:
  contents: write

concurrency:
  group: release-${{ inputs.tool }}-${{ inputs.version }}
  cancel-in-progress: false

jobs:
  prepare:
    name: Prepare release
    runs-on: ubuntu-latest
    outputs:
      release_branch: ${{ steps.prepare.outputs.release_branch }}

    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0

      - uses: jdx/mise-action@v4
        with:
          cache: true

      - name: Configure Git
        run: |
          git config user.name "datarose-release-bot"
          git config user.email "release-bot@datarose.dev"

      - name: Build github-release
        run: cargo build --release -p github-release

      - name: Prepare source release
        id: prepare
        run: >-
          ./.cache/rust/packages/toolchain/target/release/github-release prepare
          --config datarose.toml
          --release-target "${{ inputs.tool }}"
          --version "${{ inputs.version }}"

  quality:
    name: Quality gate
    needs: prepare
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v5
        with:
          ref: ${{ needs.prepare.outputs.release_branch }}
          fetch-depth: 0

      - uses: jdx/mise-action@v4
        with:
          cache: true

      - name: Quality gate
        run: mise exec -- hk check

  build:
    name: Build assets
    needs:
      - prepare
      - quality
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v5
        with:
          ref: ${{ needs.prepare.outputs.release_branch }}
          fetch-depth: 0

      - uses: jdx/mise-action@v4
        with:
          cache: true

      - name: Build cargo-release
        run: cargo build --release -p cargo-release

      - name: Build release assets
        run: >-
          ./.cache/rust/packages/toolchain/target/release/cargo-release build
          --config "${{ inputs.cargo-release-config }}"
          --version "${{ inputs.version }}"
          --output dist/release

      - uses: actions/upload-artifact@v4
        with:
          name: ${{ inputs.tool }}-release-assets
          path: dist/release
          if-no-files-found: error

  abort:
    name: Abort failed release
    needs:
      - prepare
      - quality
      - build
    if: ${{ always() && needs.prepare.result == 'success' && (needs.quality.result != 'success' || needs.build.result != 'success') }}
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0

      - uses: jdx/mise-action@v4
        with:
          cache: true

      - name: Build github-release
        run: cargo build --release -p github-release

      - name: Abort source release
        run: >-
          ./.cache/rust/packages/toolchain/target/release/github-release abort
          --config datarose.toml
          --release-target "${{ inputs.tool }}"
          --version "${{ inputs.version }}"

  finalize:
    name: Finalize release
    needs:
      - prepare
      - build
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0
          ref: __TARGET_BRANCH__

      - uses: actions/download-artifact@v4
        with:
          name: ${{ inputs.tool }}-release-assets
          path: dist/release

      - uses: jdx/mise-action@v4
        with:
          cache: true

      - name: Build github-release
        run: cargo build --release -p github-release

      - name: Finalize source release
        run: >-
          ./.cache/rust/packages/toolchain/target/release/github-release finalize
          --config datarose.toml
          --release-target "${{ inputs.tool }}"
          --version "${{ inputs.version }}"
          --assets-dir dist/release

      - name: Publish public release
        env:
          GH_TOKEN: ${{ secrets.DISTRIBUTION_REPO_TOKEN }}
        run: >-
          ./.cache/rust/packages/toolchain/target/release/github-release publish
          --config datarose.toml
          --release-target "${{ inputs.tool }}"
          --version "${{ inputs.version }}"
          --assets-dir dist/release
"#
    .replace("__TARGET_BRANCH__", target_branch)
}

fn render_release_all_workflow(profile: &ProjectProfile) -> String {
    let tools = profile
        .stored_config
        .release
        .targets
        .iter()
        .map(|target| target.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"name: Release All
run-name: Release all ${{{{ inputs.version }}}}

on:
  workflow_dispatch:
    inputs:
      version:
        description: "Version to release for every configured target, without the leading v."
        required: true
      prerelease:
        description: "Publish as a prerelease. Leave auto to infer from SemVer prerelease labels."
        required: false
        type: choice
        default: auto
        options:
          - auto
          - "true"
          - "false"

permissions:
  contents: write

concurrency:
  group: release-all-${{{{ inputs.version }}}}
  cancel-in-progress: false

jobs:
{jobs}
"#,
        jobs = render_release_all_jobs(profile, &tools),
    )
}

fn render_release_all_jobs(profile: &ProjectProfile, tools: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "  summary:\n    name: Release targets\n    runs-on: ubuntu-latest\n    steps:\n      - run: |\n          echo \"Targets: {tools}\"\n"
    ));

    for target in &profile.stored_config.release.targets {
        let job = target.name.replace('-', "_");
        let secret_name = &profile.stored_config.release.secret_name;
        out.push_str(&format!(
            r#"
  {job}:
    name: Release {tool}
    needs: summary
    uses: ./.github/workflows/_release-datarose-tool.yml
    with:
      tool: {tool}
      version: ${{{{ inputs.version }}}}
      prerelease: ${{{{ inputs.prerelease }}}}
      cargo-release-config: {cargo_release_config}
      distribution-path: {distribution_path}
    secrets:
      DISTRIBUTION_REPO_TOKEN: ${{{{ secrets.{secret_name} }}}}
"#,
            job = job,
            tool = target.name,
            cargo_release_config = target.cargo_release_config,
            distribution_path = target.distribution_path,
            secret_name = secret_name,
        ));
    }

    out
}

fn title_case(value: &str) -> String {
    value
        .split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[allow(dead_code)]
fn workflow_path(name: &str) -> PathBuf {
    PathBuf::from(format!(".github/workflows/{name}.yml"))
}
