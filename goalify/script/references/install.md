# codex-goal install

`goalify` requires `codex-goal` before it asks clarification questions or writes a goal payload.

## Why this exists

Long goal payloads are written to `.codex/goals/*.md`. In normal `workspace-write` Codex sessions, `.codex/**` is protected by the sandbox. In `danger-full-access`, that protection does not apply. `codex-goal` writes the file and sets the Linux immutable flag so the executing agent cannot edit or delete the goal file without an explicit privileged unlock.

The helper also makes `.codex/goals` root-owned and `.codex` immutable after `.codex/goals` exists, so the printed path cannot be replaced by renaming the parent directory or swapping the goals directory.

## Side effects

Installation writes:

```text
/usr/local/bin/codex-goal
/etc/sudoers.d/codex-goal
```

The sudoers rule is narrow:

```text
<user> ALL=(root) NOPASSWD: /usr/local/bin/codex-goal
```

It does not use `SETENV`, shell wrappers, or custom `env_keep`.

## Install

From the `goalify` directory:

```bash
sudo script/install-codex-goal.sh --dry-run
sudo script/install-codex-goal.sh
```

The installer:

- verifies `script/dist/arch-x86_64/codex-goal.sha256`
- stages the binary into a root-owned temporary file before installation
- ships `script/dist/arch-x86_64/build.json` with the binary hash and source-file hashes
- refuses to overwrite a different existing `/usr/local/bin/codex-goal`
- installs the binary as `root:root` with mode `0755`
- validates sudoers with `visudo -cf`
- installs `/etc/sudoers.d/codex-goal` with mode `0440`
- verifies `sudo -n /usr/local/bin/codex-goal --version`

## Verify

```bash
command -v codex-goal
sudo -n /usr/local/bin/codex-goal --version
```

Manual privileged write check:

```bash
tmp_root="$(mktemp -d)"
sudo chown "$USER:$(id -gn)" "$tmp_root"
sudo -n /usr/local/bin/codex-goal --root "$tmp_root" --slug smoke-test <<'EOF'
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
sudo rm -f /etc/sudoers.d/codex-goal /usr/local/bin/codex-goal
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
