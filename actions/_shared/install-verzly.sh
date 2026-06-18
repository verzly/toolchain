#!/usr/bin/env bash
set -euo pipefail

repo="${VERZLY_REPOSITORY:-verzly/toolchain}"
version="${VERZLY_VERSION:-latest}"
install_dir="${VERZLY_INSTALL_DIR:-${RUNNER_TEMP:-/tmp}/verzly/bin}"
create_shims="${VERZLY_CREATE_SHIMS:-true}"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) target="linux-x64"; exe="" ;;
  Darwin-x86_64) target="macos-x64"; exe="" ;;
  Darwin-arm64) target="macos-arm64"; exe="" ;;
  MINGW*-x86_64|MSYS*-x86_64|CYGWIN*-x86_64) target="windows-x64"; exe=".exe" ;;
  *)
    echo "::error::Unsupported runner platform: $(uname -s)-$(uname -m)"
    exit 1
    ;;
esac

mkdir -p "${install_dir}"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "${tmp_dir}"' EXIT

pattern="verzly-v*-${target}${exe}"
args=(release download --repo "${repo}" --pattern "${pattern}" --dir "${tmp_dir}" --clobber)
if [ -n "${version}" ] && [ "${version}" != "latest" ]; then
  tag="${version#refs/tags/}"
  case "${tag}" in
    v*) ;;
    *) tag="v${tag}" ;;
  esac
  args=(release download "${tag}" --repo "${repo}" --pattern "${pattern}" --dir "${tmp_dir}" --clobber)
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "::error::The GitHub CLI (gh) is required to install Verzly from GitHub Releases. GitHub-hosted runners include it by default."
  exit 1
fi

if [ -n "${GH_TOKEN:-}" ]; then
  export GH_TOKEN
fi

gh "${args[@]}"

binary="$(find "${tmp_dir}" -type f -name "${pattern}" ! -name '*.sha256' | sort | tail -n 1)"
if [ -z "${binary}" ]; then
  echo "::error::Could not find downloaded Verzly asset matching ${pattern} from ${repo}."
  exit 1
fi

cp "${binary}" "${install_dir}/verzly${exe}"
chmod 755 "${install_dir}/verzly${exe}"

tools=(github-release cargo-release tauri-release rust-cache android-signing ios-signing repository)
if [ "${create_shims}" = "true" ]; then
  for tool in "${tools[@]}"; do
    cat > "${install_dir}/${tool}" <<SHIM
#!/usr/bin/env bash
exec "${install_dir}/verzly${exe}" ${tool} "\$@"
SHIM
    chmod 755 "${install_dir}/${tool}"

    cat > "${install_dir}/${tool}.cmd" <<SHIM
@echo off
"${install_dir}\\verzly.exe" ${tool} %*
SHIM
  done
fi

{
  echo "${install_dir}"
} >> "${GITHUB_PATH}"

resolved_version="$(${install_dir}/verzly${exe} --version | awk '{print $NF}')"

if [ -n "${GITHUB_OUTPUT:-}" ]; then
  {
    echo "path=${install_dir}/verzly${exe}"
    echo "install-dir=${install_dir}"
    echo "version=${resolved_version}"
    echo "target=${target}"
  } >> "${GITHUB_OUTPUT}"
fi

echo "Installed Verzly ${resolved_version} for ${target}."
