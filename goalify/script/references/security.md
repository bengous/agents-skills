# codex-goal security notes

`codex-goal` is an agent-facing wrapper around a privileged helper. Keep the privileged surface small.

## Current Linux Model

- Wrapper: `/usr/local/bin/codex-goal`
- Helper: `/usr/local/libexec/codex-goal-helper`
- Owner/mode for both files: `root:root`, `0755`
- Sudoers file: `/etc/sudoers.d/codex-goal`
- Sudoers mode: `0440`
- Invocation from agents:

  ```bash
  /usr/local/bin/codex-goal --root "$PWD" --slug "$SLUG"
  ```

The wrapper pins `PATH` to system directories and runs `/usr/bin/sudo -n -- /usr/local/libexec/codex-goal-helper "$@"`. `sudo -n` must fail instead of asking for a password if the NOPASSWD rule is unavailable. Agents do not invoke `sudo` directly.

## Identity

The privileged helper requires:

- effective uid is root
- `SUDO_UID` and `SUDO_GID` exist
- both values are numeric and non-root

It ignores `SUDO_USER`. The goal file is created by root, then assigned to `SUDO_UID:SUDO_GID` through fd-based ownership correction.

## Filesystem safety

The helper refuses:

- relative `--root`
- missing or non-directory root
- root directories not owned by the invoking user
- symlink `.agents`
- symlink `.agents/goals`
- existing target file or symlink
- invalid slug
- empty stdin
- stdin over 256 KiB

The `.agents` directory remains owned by the invoking user and is not made immutable. The dedicated `.agents/goals` directory remains owned by the invoking user and is made immutable after each successful write. Before a later write, the helper clears the immutable flag on `.agents/goals`, creates the new goal file, then restores the immutable flag. This protects the goal artifact namespace without changing attributes on `.codex`.

The goal file remains user-owned, mode `0444`, and immutable.

## Privileged validation

Unit tests cover parsing, slug validation, target path construction, and stdin limits without requiring root.

Immutable write validation is intentionally manual/opt-in because it requires the installed sudoers rule, `SUDO_UID`/`SUDO_GID`, and a Linux filesystem that supports inode flags. Use the smoke test in `install.md` after explicit approval to install the helper.

## Future hardening

Evaluate Linux file capabilities such as `CAP_LINUX_IMMUTABLE` and `CAP_FOWNER` to avoid NOPASSWD sudo for the full helper.

Future platform work:

- macOS protection equivalent
- Windows protection equivalent
- signed or reproducible builds
- package-manager installation
- multi-architecture dist artifacts
