//! GitHub Actions workflow rendering.

use crate::project::{ProjectProfile, ReleaseTarget};
use crate::standards::ManagedFile;
use std::path::PathBuf;

const REUSABLE_RELEASE_WORKFLOW: &str = "_release-target.yml";
const REUSABLE_RELEASE_WORKFLOW_PATH: &str = ".github/workflows/_release-target.yml";

pub fn render_test_workflow(profile: &ProjectProfile) -> String {
    let repository_policy_step = if profile.root.join("crates/repository/Cargo.toml").is_file() {
        r#"
      - name: Repository policy
        run: cargo run -p repository -- check
"#
    } else {
        ""
    };

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

__REPOSITORY_POLICY_STEP__
      - name: Quality gate
        run: mise exec -- hk check
"#
    .replace("__REPOSITORY_POLICY_STEP__", repository_policy_step)
}

pub fn release_workflow_files(profile: &ProjectProfile, force: bool) -> Vec<ManagedFile> {
    if !profile.release_enabled() || !profile.stored_config.release.manage_workflows {
        return Vec::new();
    }

    let managed_targets = profile
        .stored_config
        .release
        .targets
        .iter()
        .filter(|target| target.workflow == "managed" && target.strategy == "same-repo")
        .collect::<Vec<_>>();

    if managed_targets.is_empty() {
        return Vec::new();
    }

    let mut files = vec![ManagedFile {
        path: profile.root.join(REUSABLE_RELEASE_WORKFLOW_PATH),
        content: render_reusable_release_workflow(profile),
        force,
    }];

    for target in &managed_targets {
        files.push(ManagedFile {
            path: profile
                .root
                .join(format!(".github/workflows/release-{}.yml", target.name)),
            content: render_release_target_workflow(target),
            force,
        });
    }

    if profile.stored_config.release.release_all && managed_targets.len() > 1 {
        files.push(ManagedFile {
            path: profile.root.join(".github/workflows/release-all.yml"),
            content: render_release_all_workflow(profile),
            force,
        });
    }

    files
}

fn render_release_target_workflow(target: &ReleaseTarget) -> String {
    let title = title_case(&target.name);
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
    uses: ./.github/workflows/{reusable_workflow}
    with:
      tool: {tool}
      version: ${{{{ inputs.version }}}}
      prerelease: ${{{{ inputs.prerelease }}}}
"#,
        title = title,
        tool = target.name,
        reusable_workflow = REUSABLE_RELEASE_WORKFLOW,
    )
}

fn render_reusable_release_workflow(profile: &ProjectProfile) -> String {
    let target_branch = &profile.stored_config.release.target_branch;
    r#"name: Release Target

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

permissions:
  contents: write

concurrency:
  group: release-${{ inputs.tool }}-${{ inputs.version }}
  cancel-in-progress: false

jobs:
  release:
    name: Release ${{ inputs.tool }}
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0
          token: ${{ github.token }}

      - uses: verzly/toolchain@v1

      - name: Configure Git
        run: |
          git config user.name "verzly-release-bot"
          git config user.email "release-bot@verzly.dev"

      - name: Prepare release
        run: >-
          verzly github-release prepare
          --config datarose.toml
          --release-target "${{ inputs.tool }}"
          --version "${{ inputs.version }}"

      - name: Quality gate
        run: mise exec -- hk check

      - name: Build assets
        run: >-
          verzly cargo-release build
          --config datarose.toml
          --release-target "${{ inputs.tool }}"
          --version "${{ inputs.version }}"
          --output dist/release

      - name: Finalize release
        env:
          GH_TOKEN: ${{ github.token }}
        run: >-
          verzly github-release finalize
          --config datarose.toml
          --release-target "${{ inputs.tool }}"
          --version "${{ inputs.version }}"
          --assets dist/release
          --prerelease "${{ inputs.prerelease }}"
          --target-branch __TARGET_BRANCH__
"#
    .replace("__TARGET_BRANCH__", target_branch)
}

fn render_release_all_workflow(profile: &ProjectProfile) -> String {
    let tools = profile
        .stored_config
        .release
        .targets
        .iter()
        .filter(|target| target.workflow == "managed" && target.strategy == "same-repo")
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

    for target in profile
        .stored_config
        .release
        .targets
        .iter()
        .filter(|target| target.workflow == "managed" && target.strategy == "same-repo")
    {
        let job = target.name.replace('-', "_");
        out.push_str(&format!(
            r#"
  {job}:
    name: Release {tool}
    needs: summary
    uses: ./.github/workflows/{reusable_workflow}
    with:
      tool: {tool}
      version: ${{{{ inputs.version }}}}
      prerelease: ${{{{ inputs.prerelease }}}}
"#,
            job = job,
            tool = target.name,
            reusable_workflow = REUSABLE_RELEASE_WORKFLOW,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{DataroseConfig, ProjectProfile, ReleaseTarget};
    use std::collections::BTreeSet;

    #[test]
    fn renders_managed_release_workflow_files() {
        let mut config = DataroseConfig::default();
        config.release.enabled = true;
        config.release.manage_workflows = true;
        config.release.targets = vec![
            managed_target("api", "same-repo"),
            managed_target("web", "same-repo"),
            custom_target("ops"),
        ];
        let profile = profile_with_config(config);

        let files = release_workflow_files(&profile, false);
        let paths = files
            .iter()
            .map(|file| file.path.to_string_lossy().replace('\\', "/"))
            .collect::<Vec<_>>();

        assert!(paths.contains(&"/repo/.github/workflows/_release-target.yml".into()));
        assert!(paths.contains(&"/repo/.github/workflows/release-api.yml".into()));
        assert!(paths.contains(&"/repo/.github/workflows/release-web.yml".into()));
        assert!(paths.contains(&"/repo/.github/workflows/release-all.yml".into()));
        assert!(!paths.contains(&"/repo/.github/workflows/release-ops.yml".into()));
        assert!(!files
            .iter()
            .any(|file| file.content.contains("_release-datarose-tool.yml")));
        assert!(!files
            .iter()
            .any(|file| file.content.contains("DISTRIBUTION_REPO_TOKEN")));
    }

    #[test]
    fn test_workflow_runs_mise_backed_hk_quality_gate() {
        let workflow = render_test_workflow(&profile_with_config(DataroseConfig::default()));

        assert!(workflow.contains("jdx/mise-action@v4"));
        assert!(workflow.contains("mise exec -- hk check"));
    }

    #[test]
    fn skips_release_workflow_files_when_management_is_disabled() {
        let mut config = DataroseConfig::default();
        config.release.enabled = true;
        config.release.manage_workflows = false;
        config.release.targets = vec![managed_target("api", "same-repo")];
        let profile = profile_with_config(config);

        assert!(release_workflow_files(&profile, false).is_empty());
    }

    fn managed_target(name: &str, strategy: &str) -> ReleaseTarget {
        ReleaseTarget {
            name: name.into(),
            path: format!("packages/{name}"),
            strategy: strategy.into(),
            workflow: "managed".into(),
            repository: format!("verzly/{name}"),
            ..ReleaseTarget::default()
        }
    }

    fn custom_target(name: &str) -> ReleaseTarget {
        ReleaseTarget {
            name: name.into(),
            path: format!("packages/{name}"),
            strategy: "custom".into(),
            workflow: "custom".into(),
            ..ReleaseTarget::default()
        }
    }

    fn profile_with_config(config: DataroseConfig) -> ProjectProfile {
        ProjectProfile {
            root: PathBuf::from("/repo"),
            workspace: PathBuf::from("."),
            workspace_root: PathBuf::from("/repo"),
            config_path: PathBuf::from("/repo/datarose.toml"),
            languages: Vec::new(),
            js_runner: None,
            has_rector: false,
            has_pest: false,
            has_mise_toml: false,
            mise_tools: BTreeSet::new(),
            stored_config: config,
        }
    }
}
