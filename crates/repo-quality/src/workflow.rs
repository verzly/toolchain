//! GitHub Actions workflow rendering.

use crate::project::ProjectProfile;

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
