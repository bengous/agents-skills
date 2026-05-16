# codex-goal security notes

`codex-goal` is a privileged helper. Keep the surface small.

## v1 model

- Installed binary: `/usr/local/bin/codex-goal`
- Owner/mode: `root:root`, `0755`
- Sudoers file: `/etc/sudoers.d/codex-goal`
- Sudoers mode: `0440`
- Invocation from agents:

  ```bash
  sudo -n /usr/local/bin/codex-goal --root "$PWD" --slug "$SLUG"
  ```

`sudo -n` must fail instead of asking for a password if the NOPASSWD rule is unavailable.

## Identity

The helper requires:

- effective uid is root
- `SUDO_UID` and `SUDO_GID` exist
- both values are numeric and non-root

It ignores `SUDO_USER`. The goal file is created by root, then assigned to `SUDO_UID:SUDO_GID` through fd-based ownership correction.

## Filesystem safety

The helper refuses:

- relative `--root`
- missing or non-directory root
- root directories not owned by the invoking user
- symlink `.codex`
- symlink `.codex/goals`
- existing target file or symlink
- invalid slug
- empty stdin
- stdin over 256 KiB

The `.codex/goals` directory is switched to `root:root` mode `0755` before a goal file is created. The `.codex` directory is made immutable after `.codex/goals` exists. This prevents a same-user process from renaming `.codex`, replacing `.codex/goals`, or swapping the printed `/goal` path during file finalization.

The goal file remains user-owned, mode `0444`, and immutable.

## Privileged validation

Unit tests cover parsing, slug validation, target path construction, and stdin limits without requiring root.

Immutable write validation is intentionally manual/opt-in because it requires sudo, `SUDO_UID`/`SUDO_GID`, and a filesystem that supports Linux inode flags. Use the smoke test in `install.md` after explicit approval to install the helper.

## Future hardening

Evaluate Linux file capabilities such as `CAP_LINUX_IMMUTABLE` and `CAP_FOWNER` to avoid NOPASSWD sudo for the full helper.

Future platform work:

- macOS protection equivalent
- Windows protection equivalent
- signed or reproducible builds
- package-manager installation
- multi-architecture dist artifacts
