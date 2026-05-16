# goalify

A Codex skill that converts rough intent into a compact payload for `/goal`.

The user types `/goal` manually. `goalify` outputs either the payload to paste after `/goal`, or a protected local file path for long goals.

## Required helper

`goalify` requires `codex-goal`, a small local helper that writes long goal payloads to `.codex/goals/*.md` and marks them immutable.

This matters when Codex runs with `danger-full-access`: the normal Codex sandbox protection for `.codex/**` is not active, so a long goal file needs OS-level protection if it should not be modified during the task.

Install details and side effects are documented in [`script/references/install.md`](script/references/install.md). The helper installation writes to `/usr/local/bin/codex-goal` and `/etc/sudoers.d/codex-goal`, so an agent must ask for explicit approval before installing it.

Install globally for the current user:

```bash
mkdir -p "$HOME/.agents/skills"
cp -R goalify "$HOME/.agents/skills/goalify"
```

Then restart Codex and invoke explicitly:

```text
$goalify
```

Interactive clarification mode:

```text
$goalify interactive
<idea / draft to explore>
```

Implicit invocation is disabled in `agents/openai.yaml` so the skill does not hijack normal implementation, review, or planning prompts.
