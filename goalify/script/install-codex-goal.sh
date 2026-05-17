#!/usr/bin/bash
set -euo pipefail
PATH="/usr/sbin:/usr/bin:/sbin:/bin"
export PATH

usage() {
  cat <<'EOF'
Usage: install-codex-goal.sh [--dry-run]

Installs an agent-facing codex-goal wrapper and a privileged helper:

  /usr/local/bin/codex-goal
  /usr/local/libexec/codex-goal-helper

The wrapper calls /usr/bin/sudo -n for the helper. Agents call
/usr/local/bin/codex-goal directly; the sudoers rule only permits the helper.
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
helper_binary="$dist_dir/codex-goal"
sha_file="$dist_dir/codex-goal.sha256"
wrapper_source="$script_dir/codex-goal-wrapper.sh"
wrapper_path="/usr/local/bin/codex-goal"
helper_path="/usr/local/libexec/codex-goal-helper"
helper_dir="$(dirname -- "$helper_path")"
sudoers_path="/etc/sudoers.d/codex-goal"
user_name="${SUDO_USER:-${USER:-}}"
installed_wrapper=0
installed_helper=0
installed_sudoers=0
should_install_wrapper=0
should_install_helper=0
should_install_sudoers=0
had_existing_wrapper=0
had_existing_helper=0
sudoers_tmp=""
stage_dir=""
staged_helper=""
staged_wrapper=""

cleanup_on_error() {
  status=$?
  if [[ "$status" -ne 0 ]]; then
    if [[ "$installed_sudoers" -eq 1 ]]; then
      rm -f "$sudoers_path"
    fi
    if [[ "$installed_wrapper" -eq 1 ]]; then
      rm -f "$wrapper_path"
    fi
    if [[ "$installed_helper" -eq 1 ]]; then
      rm -f "$helper_path"
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
  local stat_output owner group mode

  if [[ -L "$path" || ! -f "$path" ]]; then
    echo "install-codex-goal: $path must be a regular non-symlink file" >&2
    exit 1
  fi

  stat_output="$(stat -c '%u %g %a' "$path")"
  read -r owner group mode <<<"$stat_output"
  if [[ "$owner" != "0" || "$group" != "0" || "$mode" != "$expected_mode" ]]; then
    echo "install-codex-goal: $path must be root:root mode 0$expected_mode" >&2
    exit 1
  fi
}

verify_safe_directory() {
  local path="$1"
  local stat_output owner group mode

  if [[ -L "$path" || ! -d "$path" ]]; then
    echo "install-codex-goal: $path must be a regular directory" >&2
    exit 1
  fi

  stat_output="$(stat -c '%u %g %a' "$path")"
  read -r owner group mode <<<"$stat_output"
  if [[ "$owner" != "0" || "$group" != "0" ]]; then
    echo "install-codex-goal: $path must be root:root" >&2
    exit 1
  fi
  if ((8#$mode & 8#022)); then
    echo "install-codex-goal: $path must not be group/other writable" >&2
    exit 1
  fi
}

verify_path_components_safe() {
  local path="$1"
  local current="/"
  local component

  IFS='/' read -ra components <<<"${path#/}"
  for component in "${components[@]::${#components[@]}-1}"; do
    [[ -z "$component" ]] && continue
    current="${current%/}/$component"
    verify_safe_directory "$current"
  done
}

if [[ -z "$user_name" || "$user_name" == "root" ]]; then
  echo "install-codex-goal: run via sudo from the target user" >&2
  exit 1
fi
if [[ ! -x /usr/bin/sudo ]]; then
  echo "install-codex-goal: missing executable sudo at /usr/bin/sudo" >&2
  exit 1
fi
if [[ ! -f "$helper_binary" || ! -f "$sha_file" || ! -f "$wrapper_source" ]]; then
  echo "install-codex-goal: missing dist binary, checksum, or wrapper source" >&2
  exit 1
fi
if [[ -L "$helper_binary" || -L "$sha_file" || -L "$wrapper_source" ]]; then
  echo "install-codex-goal: dist binary, checksum, and wrapper source must not be symlinks" >&2
  exit 1
fi

verify_path_components_safe "$wrapper_path"
verify_path_components_safe "$helper_dir"
if [[ -e "$helper_dir" || -L "$helper_dir" ]]; then
  verify_safe_directory "$helper_dir"
fi

read -r expected_sha expected_name <"$sha_file"
if [[ "$expected_sha" != +([[:xdigit:]]) || "${#expected_sha}" -ne 64 || "$expected_name" != "codex-goal" ]]; then
  echo "install-codex-goal: invalid checksum file format" >&2
  exit 1
fi
if ! (cd "$dist_dir" && sha256sum --check --status "$(basename -- "$sha_file")"); then
  echo "install-codex-goal: checksum mismatch for $helper_binary" >&2
  exit 1
fi

stage_dir="$(mktemp -d)"
trap cleanup_on_error EXIT
staged_helper="$stage_dir/codex-goal-helper"
staged_wrapper="$stage_dir/codex-goal-wrapper"
install -o root -g root -m 0755 "$helper_binary" "$staged_helper"
if ! cmp -s "$helper_binary" "$staged_helper"; then
  echo "install-codex-goal: dist binary changed while staging; refusing install" >&2
  exit 1
fi
verify_owned_regular_file "$staged_helper" "755"
helper_sha="$(sha256sum "$staged_helper" | awk '{print $1}')"
if [[ "$helper_sha" != "$expected_sha" ]]; then
  echo "install-codex-goal: staged binary checksum mismatch" >&2
  exit 1
fi

install -o root -g root -m 0755 "$wrapper_source" "$staged_wrapper"

if [[ -e "$helper_path" || -L "$helper_path" ]]; then
  had_existing_helper=1
  if [[ -L "$helper_path" || ! -f "$helper_path" ]]; then
    echo "install-codex-goal: $helper_path exists but is not a regular non-symlink file" >&2
    exit 1
  fi
  installed_helper_sha="$(sha256sum "$helper_path" | awk '{print $1}')"
  verify_owned_regular_file "$helper_path" "755"
  if [[ "$installed_helper_sha" != "$helper_sha" ]]; then
    should_install_helper=1
  fi
else
  should_install_helper=1
fi

if [[ -e "$wrapper_path" || -L "$wrapper_path" ]]; then
  had_existing_wrapper=1
  if [[ -L "$wrapper_path" || ! -f "$wrapper_path" ]]; then
    echo "install-codex-goal: $wrapper_path exists but is not a regular non-symlink file" >&2
    exit 1
  fi
  verify_owned_regular_file "$wrapper_path" "755"
  installed_wrapper_sha="$(sha256sum "$wrapper_path" | awk '{print $1}')"
  if cmp -s "$staged_wrapper" "$wrapper_path"; then
    should_install_wrapper=0
  elif [[ "$installed_wrapper_sha" == "$helper_sha" ]]; then
    # Migration path from v1, where /usr/local/bin/codex-goal was the helper
    # binary itself. Replacing it with the wrapper keeps the public command
    # stable while moving the privileged binary under libexec.
    should_install_wrapper=1
  else
    echo "install-codex-goal: $wrapper_path exists with different content; refusing overwrite" >&2
    exit 1
  fi
else
  should_install_wrapper=1
fi

sudoers_tmp="$(mktemp)"
legacy_sudoers_tmp="$stage_dir/codex-goal-legacy-sudoers"
printf '%s ALL=(root) NOPASSWD: %s\n' "$user_name" "$helper_path" >"$sudoers_tmp"
printf '%s ALL=(root) NOPASSWD: %s\n' "$user_name" "$wrapper_path" >"$legacy_sudoers_tmp"
visudo -cf "$sudoers_tmp" >/dev/null
if [[ -e "$sudoers_path" ]]; then
  if [[ ! -f "$sudoers_path" || -L "$sudoers_path" ]]; then
    echo "install-codex-goal: $sudoers_path must be a regular non-symlink file" >&2
    exit 1
  fi
  if ! cmp -s "$sudoers_tmp" "$sudoers_path"; then
    if cmp -s "$legacy_sudoers_tmp" "$sudoers_path"; then
      should_install_sudoers=1
    else
      echo "install-codex-goal: $sudoers_path exists with different content; refusing overwrite" >&2
      exit 1
    fi
  fi
  verify_owned_regular_file "$sudoers_path" "440"
else
  should_install_sudoers=1
fi

if [[ "$dry_run" -eq 1 ]]; then
  echo "Would install wrapper: $wrapper_path"
  echo "Would install helper: $helper_path"
  echo "Would install sudoers: $sudoers_path"
  exit 0
fi

if [[ ! -e "$helper_dir" ]]; then
  install -d -o root -g root -m 0755 "$helper_dir"
fi
verify_safe_directory "$helper_dir"
if [[ "$should_install_helper" -eq 1 ]]; then
  install -o root -g root -m 0755 "$staged_helper" "$helper_path"
  if [[ "$had_existing_helper" -eq 0 ]]; then
    installed_helper=1
  fi
fi
if [[ "$should_install_wrapper" -eq 1 ]]; then
  install -o root -g root -m 0755 "$staged_wrapper" "$wrapper_path"
  if [[ "$had_existing_wrapper" -eq 0 ]]; then
    installed_wrapper=1
  fi
fi
verify_owned_regular_file "$helper_path" "755"
verify_owned_regular_file "$wrapper_path" "755"
installed_helper_sha="$(sha256sum "$helper_path" | awk '{print $1}')"
if [[ "$installed_helper_sha" != "$helper_sha" ]]; then
  echo "install-codex-goal: installed helper checksum mismatch" >&2
  exit 1
fi
if ! cmp -s "$staged_wrapper" "$wrapper_path"; then
  echo "install-codex-goal: installed wrapper mismatch" >&2
  exit 1
fi
if [[ "$should_install_sudoers" -eq 1 ]]; then
  install -o root -g root -m 0440 "$sudoers_tmp" "$sudoers_path"
  installed_sudoers=1
fi
visudo -cf "$sudoers_path" >/dev/null
/usr/bin/sudo -u "$user_name" "$wrapper_path" --version >/dev/null
installed_wrapper=0
installed_helper=0
installed_sudoers=0

echo "Installed wrapper: $wrapper_path"
echo "Installed helper: $helper_path"
echo "Installed sudoers: $sudoers_path"
