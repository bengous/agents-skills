#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUSTUP_INIT_URL="https://sh.rustup.rs"
LEFTHOOK_VERSION="${LEFTHOOK_VERSION:-2.1.4}"
GITLEAKS_VERSION="${GITLEAKS_VERSION:-8.30.1}"
DRY_RUN="${DRY_RUN:-0}"
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
CARGO_BIN_DIR="$CARGO_HOME/bin"

log() {
  printf '[install] %s\n' "$*"
}

run_cmd() {
  log "$*"
  if [ "$DRY_RUN" = "1" ]; then
    return 0
  fi

  "$@"
}

require_command() {
  if command -v "$1" >/dev/null 2>&1; then
    return 0
  fi

  printf 'Missing required system command: %s\n' "$1" >&2
  exit 1
}

detect_os() {
  case "$(uname -s)" in
    Darwin) printf 'Darwin\n' ;;
    Linux) printf 'Linux\n' ;;
    *)
      printf 'Unsupported OS: %s\n' "$(uname -s)" >&2
      exit 1
      ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) printf 'x86_64\n' ;;
    arm64|aarch64) printf 'arm64\n' ;;
    *)
      printf 'Unsupported architecture: %s\n' "$(uname -m)" >&2
      exit 1
      ;;
  esac
}

toolchain_channel() {
  sed -n 's/^channel = "\(.*\)"/\1/p' "$ROOT_DIR/rust-toolchain.toml"
}

toolchain_components() {
  sed -n 's/^components = \[\(.*\)\]/\1/p' "$ROOT_DIR/rust-toolchain.toml" \
    | tr ',' '\n' \
    | sed 's/["[:space:]]//g' \
    | sed '/^$/d'
}

ensure_rustup() {
  if command -v rustup >/dev/null 2>&1; then
    return 0
  fi

  local tmpdir script_path
  tmpdir="$(mktemp -d)"
  script_path="$tmpdir/rustup-init.sh"
  trap 'rm -rf "$tmpdir"' RETURN

  run_cmd curl -fsSL "$RUSTUP_INIT_URL" -o "$script_path"
  run_cmd sh "$script_path" -y --profile minimal
}

ensure_cargo_path() {
  if [ -f "$CARGO_HOME/env" ]; then
    # shellcheck disable=SC1090
    source "$CARGO_HOME/env"
  fi

  export PATH="$CARGO_BIN_DIR:$PATH"
}

ensure_rust_toolchain() {
  local channel install_args
  channel="$(toolchain_channel)"
  install_args=("$channel")

  while IFS= read -r component; do
    install_args+=("--component" "$component")
  done < <(toolchain_components)

  run_cmd rustup toolchain install "${install_args[@]}"
}

resolve_lefthook_download_url() {
  local os arch api_url
  os="$(detect_os)"
  arch="$(detect_arch)"
  api_url="https://api.github.com/repos/evilmartians/lefthook/releases/tags/v${LEFTHOOK_VERSION}"

  curl -fsSL "$api_url" \
    | tr ',' '\n' \
    | sed -n 's/.*"browser_download_url":[[:space:]]*"\(.*\)".*/\1/p' \
    | grep "/lefthook_${LEFTHOOK_VERSION}_${os}_${arch}\.tar\.gz$" \
    | head -n 1
}

install_lefthook() {
  local download_url tmpdir archive_path extracted_binary

  mkdir -p "$CARGO_BIN_DIR"

  if command -v lefthook >/dev/null 2>&1; then
    if lefthook version 2>/dev/null | grep -q "$LEFTHOOK_VERSION"; then
      log "lefthook ${LEFTHOOK_VERSION} already installed"
      return 0
    fi
  fi

  download_url="$(resolve_lefthook_download_url)"
  if [ -z "$download_url" ]; then
    printf 'Could not resolve lefthook download URL for version %s\n' "$LEFTHOOK_VERSION" >&2
    exit 1
  fi

  tmpdir="$(mktemp -d)"
  archive_path="$tmpdir/lefthook.tar.gz"
  trap 'rm -rf "$tmpdir"' RETURN

  run_cmd curl -fsSL "$download_url" -o "$archive_path"
  run_cmd tar -xzf "$archive_path" -C "$tmpdir"

  extracted_binary="$(find "$tmpdir" -type f -name lefthook | head -n 1)"
  if [ -z "$extracted_binary" ]; then
    printf 'Downloaded lefthook archive did not contain a lefthook binary\n' >&2
    exit 1
  fi

  run_cmd install -m 0755 "$extracted_binary" "$CARGO_BIN_DIR/lefthook"
}

resolve_gitleaks_download_url() {
  local os arch
  os="$(detect_os | tr '[:upper:]' '[:lower:]')"
  arch="$(detect_arch)"

  case "$arch" in
    x86_64) arch="x64" ;;
  esac

  printf 'https://github.com/gitleaks/gitleaks/releases/download/v%s/gitleaks_%s_%s_%s.tar.gz\n' \
    "$GITLEAKS_VERSION" "$GITLEAKS_VERSION" "$os" "$arch"
}

install_gitleaks() {
  local download_url tmpdir archive_path extracted_binary

  mkdir -p "$CARGO_BIN_DIR"

  if command -v gitleaks >/dev/null 2>&1; then
    if gitleaks version 2>/dev/null | grep -q "$GITLEAKS_VERSION"; then
      log "gitleaks ${GITLEAKS_VERSION} already installed"
      return 0
    fi
  fi

  download_url="$(resolve_gitleaks_download_url)"

  tmpdir="$(mktemp -d)"
  archive_path="$tmpdir/gitleaks.tar.gz"
  trap 'rm -rf "$tmpdir"' RETURN

  run_cmd curl -fsSL "$download_url" -o "$archive_path"
  run_cmd tar -xzf "$archive_path" -C "$tmpdir"

  extracted_binary="$(find "$tmpdir" -type f -name gitleaks | head -n 1)"
  if [ -z "$extracted_binary" ]; then
    printf 'Downloaded gitleaks archive did not contain a gitleaks binary\n' >&2
    exit 1
  fi

  run_cmd install -m 0755 "$extracted_binary" "$CARGO_BIN_DIR/gitleaks"
}

install_repo_hooks() {
  if git -C "$ROOT_DIR" config --local --get core.hooksPath >/dev/null 2>&1; then
    run_cmd git -C "$ROOT_DIR" config --local --unset core.hooksPath
  fi

  run_cmd lefthook install
  run_cmd lefthook validate
}

verify_repo() {
  run_cmd cargo run --quiet -p skills-tools -- validate frontmatter
}

main() {
  require_command bash
  require_command curl
  require_command git
  require_command tar

  cd "$ROOT_DIR"

  ensure_rustup
  ensure_cargo_path
  require_command rustup
  require_command cargo

  ensure_rust_toolchain
  install_lefthook
  install_gitleaks
  ensure_cargo_path
  require_command lefthook
  require_command gitleaks
  install_repo_hooks
  verify_repo

  log "Setup complete"
}

main "$@"
