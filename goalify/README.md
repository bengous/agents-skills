# goalify

A Codex skill that converts rough intent into a compact payload for `/goal`.

The user types `/goal` manually. `goalify` outputs either the payload to paste after `/goal`, or a protected local file path for long goals.

## Required helper

`goalify` requires `codex-goal`, a small Linux helper that writes long goal payloads to `.agents/goals/*.md` and marks them immutable.

This matters when Codex runs with `danger-full-access`: a long goal file needs OS-level protection if it should not be modified during the task. Goal files live under `.agents/goals/` so other agent harnesses can use the same artifact without treating it as Codex configuration.

Install details and side effects are documented in [`script/references/install.md`](script/references/install.md). Installation writes an agent-facing wrapper to `/usr/local/bin/codex-goal`, a privileged helper to `/usr/local/libexec/codex-goal-helper`, and a narrow sudoers rule in `/etc/sudoers.d/codex-goal`, so an agent must ask for explicit approval before installing it.

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
