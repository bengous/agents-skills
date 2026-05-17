#!/usr/bin/bash
set -euo pipefail

# Public entrypoint for agents and skills.
#
# The Rust helper needs root only to create immutable goal files with Linux
# inode flags. Keep that privilege boundary behind this wrapper so agents call:
#
#   /usr/local/bin/codex-goal --root "$PWD" --slug "$SLUG"
#
# The sudoers rule installed by install-codex-goal.sh permits only the helper
# path below. It does not grant SETENV or custom env_keep.
helper="/usr/local/libexec/codex-goal-helper"
PATH="/usr/sbin:/usr/bin:/sbin:/bin"
export PATH

if [[ ! -x "$helper" ]]; then
  echo "codex-goal: missing executable helper at $helper" >&2
  exit 127
fi

exec /usr/bin/sudo -n -- "$helper" "$@"
