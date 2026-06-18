#!/usr/bin/env bash
# Anti-orphan and index-integrity check for a progressive-loading markdown wiki.
# Generic: it only enforces that every content file is reachable from its index.
# Run it from a wiki created by init-wiki.sh (lives in <wiki>/scripts/).
set -euo pipefail
IFS=$'\n\t'

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
listing=""
cleanup() {
  [[ -n "${listing}" ]] && rm -f -- "${listing}"
}
trap cleanup EXIT

cd "${root}"

required_files=(index.md journal.md catalog.md SCHEMA.md)
for file in "${required_files[@]}"; do
  if [[ ! -f "${file}" ]]; then
    echo "Missing required wiki file: ${file}" >&2
    exit 1
  fi
done

for dir in raw pages log; do
  if [[ ! -d "${dir}" ]]; then
    echo "Missing required wiki directory: ${dir}/" >&2
    exit 1
  fi
done

for marker in raw/ pages/; do
  if ! grep -qF -- "${marker}" index.md; then
    echo "index.md must mention ${marker}" >&2
    exit 1
  fi
done

listing="$(mktemp)"
fail=0

# Every content file must be named in its index; otherwise it is an orphan that a
# future agent will never load.
require_referenced() {
  local content_dir="$1"
  local index_file="$2"
  local label="$3"
  local file
  find "${content_dir}" -type f -name '*.md' -print > "${listing}"
  sort -o "${listing}" "${listing}"
  while IFS= read -r file; do
    [[ -z "${file}" ]] && continue
    if ! grep -qF -- "${file}" "${index_file}"; then
      echo "${index_file} does not reference ${label}: ${file}" >&2
      fail=1
    fi
  done < "${listing}"
}

require_referenced pages index.md "page"
require_referenced raw catalog.md "raw source"
require_referenced log journal.md "archive"

if [[ "${fail}" -ne 0 ]]; then
  exit 1
fi

echo "wiki check OK"
