#!/usr/bin/env bash
set -euo pipefail

cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin"

find_tool() {
  local tool="$1"
  local path
  path="$(command -v "$tool" 2>/dev/null || true)"
  if [[ -n "$path" ]]; then
    printf '%s\n' "$path"
    return
  fi
  if [[ -x "$cargo_bin/$tool" ]]; then
    printf '%s\n' "$cargo_bin/$tool"
    return
  fi
  return 1
}

cargo_deny="$(find_tool cargo-deny)" || {
  echo "missing required Rust quality tool: cargo-deny" >&2
  echo "install missing tools, then rerun scripts/check-rust.sh" >&2
  exit 127
}
cargo_machete="$(find_tool cargo-machete)" || {
  echo "missing required Rust quality tool: cargo-machete" >&2
  echo "install missing tools, then rerun scripts/check-rust.sh" >&2
  exit 127
}

cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
"$cargo_deny" check
"$cargo_machete" --with-metadata crates/skills-tools goalify/script/codex-goal
