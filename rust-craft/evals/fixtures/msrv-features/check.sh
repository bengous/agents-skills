#!/bin/sh
set -u

cd "$(dirname "$0")"
status=0

for toolchain in 1.94.1 1.88; do
  cargo +"$toolchain" test --locked || status=1
  cargo +"$toolchain" test --locked --features json || status=1
done

exit "$status"
