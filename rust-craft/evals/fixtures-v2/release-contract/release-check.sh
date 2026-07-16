#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

cargo +1.88 test --locked -p downstream
cargo +1.88 test --locked -p wire-core --no-default-features
cargo +1.88 test --locked -p wire-core --no-default-features --features schema

package_files="$(cargo +1.88 package --locked --allow-dirty --list -p wire-core)"
grep -Fxq 'schema.json' <<<"$package_files"
