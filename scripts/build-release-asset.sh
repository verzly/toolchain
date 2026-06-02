#!/usr/bin/env bash
set -euo pipefail

TOOL="${1:?tool name is required}"
VERSION="${2:?version is required}"

case "${RUNNER_OS:-$(uname -s)}" in
  Linux)
    OS_NAME="Linux"
    EXT=""
    ARCH="$(uname -m)"
    case "${ARCH}" in
      x86_64|amd64) HOST="x86_64-unknown-linux-gnu" ;;
      aarch64|arm64) HOST="aarch64-unknown-linux-gnu" ;;
      *) echo "Unsupported Linux architecture: ${ARCH}" >&2; exit 1 ;;
    esac
    ;;
  macOS|Darwin)
    OS_NAME="macOS"
    EXT=""
    ARCH="$(uname -m)"
    case "${ARCH}" in
      x86_64|amd64) HOST="x86_64-apple-darwin" ;;
      aarch64|arm64) HOST="aarch64-apple-darwin" ;;
      *) echo "Unsupported macOS architecture: ${ARCH}" >&2; exit 1 ;;
    esac
    ;;
  Windows*)
    OS_NAME="Windows"
    EXT=".exe"
    ARCH="${PROCESSOR_ARCHITECTURE:-AMD64}"
    case "${ARCH}" in
      AMD64|amd64|x86_64) HOST="x86_64-pc-windows-msvc" ;;
      ARM64|arm64|aarch64) HOST="aarch64-pc-windows-msvc" ;;
      *) echo "Unsupported Windows architecture: ${ARCH}" >&2; exit 1 ;;
    esac
    ;;
  *)
    echo "Unsupported runner OS: ${RUNNER_OS:-$(uname -s)}" >&2
    exit 1
    ;;
esac

CONFIG="${RUNNER_TEMP:-/tmp}/cargo-release-${TOOL}-${HOST}.toml"
DIST="${RUNNER_TEMP:-/tmp}/dist-${TOOL}-${HOST}"
mkdir -p "$(dirname "${CONFIG}")" "release-assets/${TOOL}"

cat > "${CONFIG}" <<EOF
[project]
root = "."
binary = "${TOOL}"

[build]
out_dir = "${DIST}"
default_strategy = "host"
container_engine = "podman"

[artifacts]
checksum = true
manifest = true

[targets.host]
enabled = true
triple = "${HOST}"
strategy = "host"
command = "cargo build --release -p ${TOOL}"
artifacts = ["target/release/${TOOL}${EXT}"]
EOF

# cargo-release builds every public executable, including cargo-release itself.
# The first binary is still bootstrapped with plain Cargo so the workflow can use its own builder afterwards.
cargo build --release -p cargo-release
"target/release/cargo-release${EXT}" build --config "${CONFIG}"

SOURCE="${DIST}/host/${TOOL}${EXT}"
TARGET="release-assets/${TOOL}/${TOOL}-v${VERSION}-${HOST}${EXT}"
cp "${SOURCE}" "${TARGET}"

if command -v sha256sum >/dev/null 2>&1; then
  (cd "$(dirname "${TARGET}")" && sha256sum "$(basename "${TARGET}")" > "$(basename "${TARGET}").sha256")
elif command -v shasum >/dev/null 2>&1; then
  (cd "$(dirname "${TARGET}")" && shasum -a 256 "$(basename "${TARGET}")" > "$(basename "${TARGET}").sha256")
else
  echo "No checksum command found; skipping checksum for ${TARGET}" >&2
fi

echo "Built ${TARGET} on ${OS_NAME} (${HOST})"
