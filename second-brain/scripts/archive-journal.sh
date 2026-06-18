#!/usr/bin/env bash
# Window the journal: seal the oldest whole months into immutable log/YYYY-MM.md
# archives once the journal's dated entries grow past a byte threshold, and
# refresh the Archives list. The threshold is measured against the dated-entries
# region only (the preamble and the Archives list are not counted), so the metric
# is stable across runs and a re-run with nothing over the threshold is a no-op.
#
# Journal contract (see assets/templates/journal.md): a preamble, then dated
# entries headed `## YYYY-MM-DD ...`, then a final `## Archives` section.
#
# Usage: archive-journal.sh [threshold_bytes]   (default 25600)
set -euo pipefail
IFS=$'\n\t'

root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "${root}"

threshold="${1:-25600}"
journal="journal.md"
archive_dir="log"

if [[ ! -f "${journal}" ]]; then
  echo "No ${journal} in ${root}" >&2
  exit 1
fi

manifest=""
work=""
listing=""
cleanup() {
  [[ -n "${manifest}" ]] && rm -f -- "${manifest}"
  [[ -n "${work}" ]] && rm -f -- "${work}"
  [[ -n "${listing}" ]] && rm -f -- "${listing}"
}
trap cleanup EXIT
manifest="$(mktemp)"
work="$(mktemp)"
listing="$(mktemp)"

# Parse the journal once. Emit: total size, preamble end line, and one record per
# dated entry (start line, end line, byte size, YYYY-MM month). Character length
# tracks bytes for ASCII/UTF-8 single-byte content, which is all the threshold
# heuristic needs.
# ponytail: char-length sizing, exact byte accounting only if multibyte content
# ever makes the threshold drift materially.
awk '
  function close_entry(endline) {
    if (in_entry) {
      printf "E\t%d\t%d\t%d\t%s\n", e_start, endline, e_bytes, e_month
      in_entry = 0
    }
  }
  { line_bytes = length($0) + 1 }
  /^## Archives/ && !arch {
    close_entry(NR - 1); printf "A\t%d\n", NR; arch = NR; next
  }
  /^## [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]/ && !arch {
    if (!first) { printf "P\t%d\n", NR - 1; first = 1 }
    close_entry(NR - 1)
    in_entry = 1; e_start = NR; e_bytes = 0; e_month = substr($0, 4, 7)
  }
  { if (in_entry) e_bytes += line_bytes }
  END {
    close_entry(NR)
    if (!first) printf "P\t%d\n", (arch ? arch - 1 : NR)
  }
' "${journal}" > "${manifest}"

preamble_end=0
e_start=()
e_end=()
e_bytes=()
e_month=()
while IFS=$'\t' read -r tag f2 f3 f4 f5; do
  case "${tag}" in
    P) preamble_end="${f2}" ;;
    E) e_start+=("${f2}"); e_end+=("${f3}"); e_bytes+=("${f4}"); e_month+=("${f5}") ;;
    *) : ;;
  esac
done < "${manifest}"

n="${#e_start[@]}"

entries_total=0
for (( i = 0; i < n; i++ )); do
  entries_total=$(( entries_total + e_bytes[i] ))
done

if (( entries_total <= threshold )); then
  echo "journal entries are ${entries_total} bytes (<= ${threshold}); nothing to seal"
  exit 0
fi

if (( n <= 1 )); then
  echo "journal entries exceed ${threshold} bytes but there is <= 1 dated entry; nothing to seal" >&2
  exit 0
fi

# Seal whole oldest months (never splitting a month) until the kept entries fit,
# always keeping the newest month so the recent window never empties. Whole-month
# sealing is what keeps each log/YYYY-MM.md written exactly once: a partial month
# would later reach the top of the journal and collide with its own sealed
# archive, which the immutability guard would then refuse to extend — a deadlock.
newest_month="${e_month[n - 1]}"
removed=0
seal_count=0
i=0
while (( i < n )); do
  if (( entries_total - removed <= threshold )); then
    break
  fi
  month="${e_month[i]}"
  if [[ "${month}" == "${newest_month}" ]]; then
    break
  fi
  while (( i < n )) && [[ "${e_month[i]}" == "${month}" ]]; do
    removed=$(( removed + e_bytes[i] ))
    i=$(( i + 1 ))
  done
  seal_count="${i}"
done

if (( seal_count == 0 )); then
  echo "journal entries exceed ${threshold} bytes but the newest month alone fills it; nothing to seal"
  exit 0
fi

mkdir -p -- "${archive_dir}"
declare -A created
for (( i = 0; i < seal_count; i++ )); do
  month="${e_month[i]}"
  target="${archive_dir}/${month}.md"
  if [[ -z "${created[${month}]:-}" ]]; then
    if [[ -e "${target}" ]]; then
      echo "Refusing to rewrite sealed archive ${target} (archives are immutable)" >&2
      exit 1
    fi
    printf '# Journal archive %s\n\n' "${month}" > "${target}"
    created["${month}"]=1
  fi
  awk -v s="${e_start[i]}" -v e="${e_end[i]}" 'NR >= s && NR <= e' "${journal}" >> "${target}"
done

# Rebuild the Archives list from disk so it always matches the sealed files.
find "${archive_dir}" -maxdepth 1 -type f -name '*.md' -print > "${listing}"
LC_ALL=C sort -r -o "${listing}" "${listing}"

# Rewrite the journal: preamble + kept (newer) entries + fresh Archives section.
{
  if (( preamble_end >= 1 )); then
    awk -v e="${preamble_end}" 'NR <= e' "${journal}"
  fi
  for (( i = seal_count; i < n; i++ )); do
    awk -v s="${e_start[i]}" -v e="${e_end[i]}" 'NR >= s && NR <= e' "${journal}"
  done
  printf '## Archives\n\n'
  printf '%s\n\n' '<!-- Managed by archive-journal.sh. Sealed months, newest first. -->'
  while IFS= read -r f; do
    [[ -z "${f}" ]] && continue
    printf -- '- [%s](%s) — sealed entries\n' "${f}" "${f}"
  done < "${listing}"
} > "${work}"

mv -- "${work}" "${journal}"
work=""

final_size="$(wc -c < "${journal}")"
echo "sealed ${seal_count} entries into ${archive_dir}/; journal.md now ${final_size} bytes"
