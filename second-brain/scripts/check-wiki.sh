#!/usr/bin/env bash
# Anti-orphan and index-integrity check for a progressive-loading markdown wiki.
# Generic: it enforces that every content file is reachable from its index, and
# — only on files that opt in with YAML frontmatter — that `updated:` dates are
# well-formed and that index.md is not older than the newest page. A wiki with
# no frontmatter passes exactly as before.
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

# Frontmatter checks — strictly opt-in per file: a file without a leading YAML
# frontmatter block (or without an `updated:` key in it) is skipped entirely.
# Sealed archives under log/ are never scanned: they are immutable, so a check
# that could demand an edit there would contradict the immutability guarantee.

# Print the `updated:` value from a leading frontmatter block, else nothing.
frontmatter_updated() {
  awk '
    NR == 1 { if ($0 != "---") exit; next }
    /^---[[:space:]]*$/ { exit }
    sub(/^updated:[[:space:]]*/, "") { sub(/[[:space:]]+$/, ""); print; exit }
  ' "$1"
}

iso_date='^[0-9]{4}-[0-9]{2}-[0-9]{2}$'
newest_page=""
newest_page_updated=""

{
  printf '%s\n' index.md journal.md catalog.md
  find pages raw -type f -name '*.md' -print
} > "${listing}"

while IFS= read -r file; do
  updated="$(frontmatter_updated "${file}")"
  [[ -z "${updated}" ]] && continue
  if [[ ! "${updated}" =~ ${iso_date} ]]; then
    echo "${file}: invalid frontmatter updated '${updated}' (expected YYYY-MM-DD)" >&2
    fail=1
    continue
  fi
  # ISO dates compare correctly as strings.
  if [[ "${file}" == pages/* && "${updated}" > "${newest_page_updated}" ]]; then
    newest_page="${file}"
    newest_page_updated="${updated}"
  fi
done < "${listing}"

# A stale map misroutes every future read. Warning only: the index may
# legitimately lag when a page edit did not change its one-line summary.
index_updated="$(frontmatter_updated index.md)"
if [[ "${index_updated}" =~ ${iso_date} && -n "${newest_page_updated}" ]] \
  && [[ "${index_updated}" < "${newest_page_updated}" ]]; then
  echo "warning: index.md updated (${index_updated}) is older than the newest page (${newest_page}: ${newest_page_updated})" >&2
fi

if [[ "${fail}" -ne 0 ]]; then
  exit 1
fi

echo "wiki check OK"
