# goalify

A Codex skill that converts a rough human intention into a compact `/goal` command plus an optional first-turn session prompt.

Install globally for the current user:

```bash
mkdir -p "$HOME/.agents/skills"
cp -R goalify "$HOME/.agents/skills/goalify"
```

Then restart Codex and invoke explicitly:

```text
$goalify
```

Implicit invocation is disabled in `agents/openai.yaml` so the skill does not hijack normal implementation, review, or planning prompts.
