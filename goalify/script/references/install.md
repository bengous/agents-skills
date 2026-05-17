# codex-goal install

`goalify` requires `codex-goal` before it asks clarification questions or writes a goal payload.

## Why this exists

Long goal payloads are written to `.codex/goals/*.md`. In normal `workspace-write` Codex sessions, `.codex/**` is protected by the sandbox. In `danger-full-access`, that protection does not apply. `codex-goal` writes the file and sets the Linux immutable flag so the executing agent cannot edit or delete the goal file without an explicit privileged unlock.

The helper also makes `.codex/goals` root-owned and `.codex` immutable after `.codex/goals` exists, so the printed path cannot be replaced by renaming the parent directory or swapping the goals directory.

## Side effects

Installation writes:

```text
/usr/local/bin/codex-goal                  # agent-facing wrapper
/usr/local/libexec/codex-goal-helper       # privileged Rust helper
/etc/sudoers.d/codex-goal                  # narrow sudoers rule
```

Agents and skills call only:

```bash
/usr/local/bin/codex-goal --root "$PWD" --slug "$SLUG"
```

The wrapper pins `PATH` to system directories and runs `/usr/bin/sudo -n -- /usr/local/libexec/codex-goal-helper "$@"`.
Keeping sudo inside the wrapper makes the agent contract simple while
preserving an explicit sudoers policy boundary. The sudoers rule is narrow:

```text
<user> ALL=(root) NOPASSWD: /usr/local/libexec/codex-goal-helper
```

It does not use `SETENV` or custom `env_keep`.

## Install

From the `goalify` directory:

```bash
sudo script/install-codex-goal.sh --dry-run
sudo script/install-codex-goal.sh
```

The installer:

- verifies `script/dist/arch-x86_64/codex-goal.sha256`
- stages the helper binary into a root-owned temporary file before installation
- stages `script/codex-goal-wrapper.sh` into a root-owned temporary file before installation
- ships `script/dist/arch-x86_64/build.json` with the binary hash and source-file hashes
- refuses to overwrite an unrelated existing `/usr/local/bin/codex-goal`
- migrates the old v1 layout where `/usr/local/bin/codex-goal` was the privileged binary
- installs `/usr/local/bin/codex-goal` as a root-owned wrapper with mode `0755`
- installs `/usr/local/libexec/codex-goal-helper` as the root-owned helper with mode `0755`
- validates sudoers with `visudo -cf`
- installs `/etc/sudoers.d/codex-goal` with mode `0440`
- verifies `/usr/local/bin/codex-goal --version` as the target user

## Verify

```bash
test -x /usr/local/bin/codex-goal
test ! -L /usr/local/bin/codex-goal
[[ "$(/usr/bin/stat -c '%u:%g:%a' /usr/local/bin/codex-goal)" == "0:0:755" ]]
/usr/local/bin/codex-goal --version
```

Manual privileged write check:

```bash
tmp_root="$(mktemp -d)"
cd "$tmp_root"
/usr/local/bin/codex-goal --root "$tmp_root" --slug smoke-test <<'EOF'
Objective:
Smoke-test codex-goal immutable writes.
EOF
lsattr "$tmp_root/.codex/goals/smoke-test.md"
lsattr -d "$tmp_root/.codex"
sudo chattr -i "$tmp_root/.codex/goals/smoke-test.md"
sudo chattr -i "$tmp_root/.codex"
sudo chown "$USER:$(id -gn)" "$tmp_root/.codex/goals"
rm -rf "$tmp_root"
```

Both `lsattr` outputs must include `i`.

## Uninstall

This does not remove any `.codex/goals/*.md` files.

```bash
sudo rm -f /etc/sudoers.d/codex-goal /usr/local/bin/codex-goal /usr/local/libexec/codex-goal-helper
```

To delete a protected goal file manually:

```bash
sudo chattr -i .codex/goals/<slug>.md
rm .codex/goals/<slug>.md
```

If you need to remove `.codex` itself:

```bash
sudo chattr -i .codex
sudo chattr -i .codex/goals/*.md
sudo chown -R "$USER:$(id -gn)" .codex
rm -rf .codex
```

## Unsupported platforms

v1 targets Arch Linux x86_64. macOS, Windows, and other Linux distributions are future work.

## Workspace ownership

`codex-goal` only writes under a root directory owned by the invoking user. This prevents the sudo helper from creating user-owned `.codex` directories inside system-owned locations.
