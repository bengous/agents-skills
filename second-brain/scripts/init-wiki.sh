#!/usr/bin/env bash
# Scaffold a progressive-loading markdown wiki from the bundled templates.
# Refuses a non-empty target unless --force is given, so it never scaffolds over
# existing content.
#
# Usage: init-wiki.sh [--force] <target-dir>
set -euo pipefail
IFS=$'\n\t'

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
skill_dir="$(cd -- "${script_dir}/.." && pwd -P)"
templates="${skill_dir}/assets/templates"

force=0
target=""
for arg in "$@"; do
  case "${arg}" in
    --force) force=1 ;;
    -*) echo "Unknown option: ${arg}" >&2; exit 2 ;;
    *)
      if [[ -n "${target}" ]]; then
        echo "Only one target directory is allowed" >&2
        exit 2
      fi
      target="${arg}"
      ;;
  esac
done

if [[ -z "${target}" ]]; then
  echo "Usage: init-wiki.sh [--force] <target-dir>" >&2
  exit 2
fi

if [[ -d "${target}" ]]; then
  target_contents="$(find "${target}" -mindepth 1 -maxdepth 1 -print -quit)"
  if [[ -n "${target_contents}" ]] && (( !force )); then
    echo "Refusing to scaffold into non-empty ${target} (use --force to override)" >&2
    exit 1
  fi
fi

mkdir -p -- "${target}"/raw "${target}"/pages "${target}"/log \
  "${target}"/scripts "${target}"/templates

cp -- "${templates}/index.md" "${target}/index.md"
cp -- "${templates}/journal.md" "${target}/journal.md"
cp -- "${templates}/catalog.md" "${target}/catalog.md"
cp -- "${templates}/SCHEMA.md" "${target}/SCHEMA.md"
cp -- "${templates}/entry.md" "${target}/templates/entry.md"
cp -- "${script_dir}/check-wiki.sh" "${target}/scripts/check-wiki.sh"
cp -- "${script_dir}/archive-journal.sh" "${target}/scripts/archive-journal.sh"
chmod +x -- "${target}/scripts/check-wiki.sh" "${target}/scripts/archive-journal.sh"

echo "Scaffolded progressive-loading wiki at ${target}"
echo "Next:"
echo "  1. Personalize ${target}/SCHEMA.md (add domain rules under the line)."
echo "  2. Edit ${target}/index.md and start ${target}/journal.md."
echo "  3. Wire scripts/check-wiki.sh into a commit or stop hook (see references/setup-guide.md)."
