#!/usr/bin/env bash
set -euo pipefail

TOOL="${1:?tool name is required}"
REPO_DIR="${2:?distribution repository checkout is required}"

CONTENT_ROOT="${DISTRIBUTION_REPO_CONTENT_ROOT:-}"
if [ -z "${CONTENT_ROOT}" ]; then
  echo "DISTRIBUTION_REPO_CONTENT_ROOT is required because distribution repo contents are not part of verzly/toolchain." >&2
  echo "When using the handoff ZIP locally, run with: DISTRIBUTION_REPO_CONTENT_ROOT=../_repos" >&2
  exit 1
fi

TEMPLATE_DIR="$(cd "${CONTENT_ROOT}/${TOOL}" && pwd)"
REPO_PATH="$(cd "${REPO_DIR}" && pwd)"

if [ ! -d "${TEMPLATE_DIR}" ]; then
  echo "Unknown distribution repository content directory: ${CONTENT_ROOT}/${TOOL}" >&2
  exit 1
fi

if [ ! -d "${REPO_PATH}/.git" ]; then
  echo "Distribution repository checkout is not a Git repository: ${REPO_PATH}" >&2
  exit 1
fi

find "${REPO_PATH}" -mindepth 1 -maxdepth 1 ! -name .git -exec rm -rf {} +
cp -R "${TEMPLATE_DIR}/." "${REPO_PATH}/"
