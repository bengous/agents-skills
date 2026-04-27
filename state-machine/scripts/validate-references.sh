#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT_DIR/.." && pwd)"
VALIDATION_DIR="$ROOT_DIR/config/validation"
WORK_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/state-machine-validation.XXXXXX")"
JS_WORK="$WORK_ROOT/js"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'missing required command: %s\n' "$1" >&2
    exit 127
  fi
}

run() {
  printf '\n==> %s\n' "$*"
  "$@"
}

printf 'state-machine validation scratch: %s\n' "$WORK_ROOT"

for cmd in rustc cargo gcc clang javac node npm shellcheck; do
  require_cmd "$cmd"
done

printf '\n==> observed tool versions\n'
rustc --version
cargo clippy --version
gcc --version | sed -n '1p'
clang --version | sed -n '1p'
javac -version 2>&1
node --version
npm --version
shellcheck --version | sed -n '1,2p'
if command -v bun >/dev/null 2>&1; then
  bun --version
else
  printf 'bun: not found\n'
fi

mkdir -p "$JS_WORK"
cp "$VALIDATION_DIR/toolchains/npm/package.json" "$VALIDATION_DIR/toolchains/npm/package-lock.json" "$JS_WORK/"

run npm ci --prefix "$JS_WORK" --ignore-scripts --audit=false --fund=false --cache "$WORK_ROOT/npm-cache"

run env \
  STATE_MACHINE_ROOT="$ROOT_DIR" \
  STATE_MACHINE_REPO_ROOT="$REPO_ROOT" \
  STATE_MACHINE_VALIDATION_DIR="$VALIDATION_DIR" \
  STATE_MACHINE_WORK_ROOT="$WORK_ROOT" \
  STATE_MACHINE_NODE_MODULES="$JS_WORK/node_modules" \
  node "$ROOT_DIR/scripts/validate-references.mjs"
