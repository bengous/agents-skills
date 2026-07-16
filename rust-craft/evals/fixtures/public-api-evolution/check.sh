#!/usr/bin/env bash
set -euo pipefail

cargo test --workspace --quiet
cargo check --workspace --all-targets --quiet
