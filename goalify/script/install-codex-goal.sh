#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: install-codex-goal.sh [--dry-run]

Installs codex-goal to /usr/local/bin/codex-goal and configures a narrow
NOPASSWD sudoers rule for that binary.
EOF
}

dry_run=0
case "${1:-}" in
"")
  ;;
--dry-run)
  dry_run=1
  shift
  ;;
--help | -h)
  usage
  exit 0
  ;;
*)
  usage
  exit 2
  ;;
esac
if [[ $# -ne 0 ]]; then
  usage
  exit 2
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
dist_dir="$script_dir/dist/arch-x86_64"
binary="$dist_dir/codex-goal"
sha_file="$dist_dir/codex-goal.sha256"
install_path="/usr/local/bin/codex-goal"
sudoers_path="/etc/sudoers.d/codex-goal"
user_name="${SUDO_USER:-${USER:-}}"
installed_binary=0
installed_sudoers=0
should_install_binary=0
had_existing_binary=0
sudoers_tmp=""
stage_dir=""
staged_binary=""

cleanup_on_error() {
  status=$?
  if [[ "$status" -ne 0 ]]; then
    if [[ "$installed_sudoers" -eq 1 ]]; then
      rm -f "$sudoers_path"
    fi
    if [[ "$installed_binary" -eq 1 ]]; then
      rm -f "$install_path"
    fi
  fi
  if [[ -n "$sudoers_tmp" ]]; then
    rm -f "$sudoers_tmp"
  fi
  if [[ -n "$stage_dir" ]]; then
    rm -rf "$stage_dir"
  fi
  exit "$status"
}

verify_owned_regular_file() {
  local path="$1"
  local expected_mode="$2"

  if [[ -L "$path" || ! -f "$path" ]]; then
    echo "install-codex-goal: $path must be a regular non-symlink file" >&2
    exit 1
  fi

  read -r owner group mode < <(stat -c '%u %g %a' "$path")
  if [[ "$owner" != "0" || "$group" != "0" || "$mode" != "$expected_mode" ]]; then
    echo "install-codex-goal: $path must be root:root mode 0$expected_mode" >&2
    exit 1
  fi
}

verify_path_components_safe() {
  local path="$1"
  local current="/"
  local component
  local owner group mode

  IFS='/' read -ra components <<<"${path#/}"
  for component in "${components[@]::${#components[@]}-1}"; do
    [[ -z "$component" ]] && continue
    current="${current%/}/$component"
    if [[ -L "$current" || ! -d "$current" ]]; then
      echo "install-codex-goal: $current must be a regular directory" >&2
      exit 1
    fi
    read -r owner group mode < <(stat -c '%u %g %a' "$current")
    if [[ "$owner" != "0" || "$group" != "0" ]]; then
      echo "install-codex-goal: $current must be root:root" >&2
      exit 1
    fi
    if ((8#$mode & 8#022)); then
      echo "install-codex-goal: $current must not be group/other writable" >&2
      exit 1
    fi
  done
}

if [[ -z "$user_name" || "$user_name" == "root" ]]; then
  echo "install-codex-goal: run via sudo from the target user" >&2
  exit 1
fi
if [[ ! -f "$binary" || ! -f "$sha_file" ]]; then
  echo "install-codex-goal: missing dist binary or checksum in $dist_dir" >&2
  exit 1
fi
if [[ -L "$binary" || -L "$sha_file" ]]; then
  echo "install-codex-goal: dist binary and checksum must not be symlinks" >&2
  exit 1
fi

verify_path_components_safe "$install_path"

read -r expected_sha expected_name <"$sha_file"
if [[ "$expected_sha" != +([[:xdigit:]]) || "${#expected_sha}" -ne 64 || "$expected_name" != "codex-goal" ]]; then
  echo "install-codex-goal: invalid checksum file format" >&2
  exit 1
fi
if ! (cd "$dist_dir" && sha256sum --check --status "$(basename -- "$sha_file")"); then
  echo "install-codex-goal: checksum mismatch for $binary" >&2
  exit 1
fi

stage_dir="$(mktemp -d)"
trap cleanup_on_error EXIT
staged_binary="$stage_dir/codex-goal"
install -o root -g root -m 0755 "$binary" "$staged_binary"
if ! cmp -s "$binary" "$staged_binary"; then
  echo "install-codex-goal: dist binary changed while staging; refusing install" >&2
  exit 1
fi
verify_owned_regular_file "$staged_binary" "755"
actual_sha="$(sha256sum "$staged_binary" | awk '{print $1}')"
if [[ "$actual_sha" != "$expected_sha" ]]; then
  echo "install-codex-goal: staged binary checksum mismatch" >&2
  exit 1
fi

if [[ -e "$install_path" || -L "$install_path" ]]; then
  had_existing_binary=1
  if [[ -L "$install_path" || ! -f "$install_path" ]]; then
    echo "install-codex-goal: $install_path exists but is not a regular non-symlink file" >&2
    exit 1
  fi
  installed_sha="$(sha256sum "$install_path" | awk '{print $1}')"
  verify_owned_regular_file "$install_path" "755"
  if [[ "$installed_sha" != "$actual_sha" ]]; then
    should_install_binary=1
  fi
else
  should_install_binary=1
fi

sudoers_tmp="$(mktemp)"
printf '%s ALL=(root) NOPASSWD: %s\n' "$user_name" "$install_path" >"$sudoers_tmp"
visudo -cf "$sudoers_tmp" >/dev/null
if [[ -e "$sudoers_path" ]]; then
  if [[ ! -f "$sudoers_path" || -L "$sudoers_path" ]]; then
    echo "install-codex-goal: $sudoers_path must be a regular non-symlink file" >&2
    exit 1
  fi
  if ! cmp -s "$sudoers_tmp" "$sudoers_path"; then
    echo "install-codex-goal: $sudoers_path exists with different content; refusing overwrite" >&2
    exit 1
  fi
  verify_owned_regular_file "$sudoers_path" "440"
fi

if [[ "$dry_run" -eq 1 ]]; then
  echo "Would install: $install_path"
  echo "Would install sudoers: $sudoers_path"
  exit 0
fi

if [[ "$should_install_binary" -eq 1 ]]; then
  install -o root -g root -m 0755 "$staged_binary" "$install_path"
  if [[ "$had_existing_binary" -eq 0 ]]; then
    installed_binary=1
  fi
fi
verify_owned_regular_file "$install_path" "755"
installed_sha="$(sha256sum "$install_path" | awk '{print $1}')"
if [[ "$installed_sha" != "$actual_sha" ]]; then
  echo "install-codex-goal: installed binary checksum mismatch" >&2
  exit 1
fi
if [[ ! -e "$sudoers_path" ]]; then
  install -o root -g root -m 0440 "$sudoers_tmp" "$sudoers_path"
  installed_sudoers=1
fi
visudo -cf "$sudoers_path" >/dev/null
sudo -u "$user_name" sudo -n "$install_path" --version >/dev/null
installed_binary=0
installed_sudoers=0

echo "Installed: $install_path"
echo "Installed sudoers: $sudoers_path"
