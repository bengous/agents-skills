# Codex Guidance Checkpoints

Use official OpenAI Codex docs when current Codex behavior affects a finding.

## Official Sources

- `https://developers.openai.com/codex/concepts/customization`
  - Skills are reusable workflows with `SKILL.md` plus optional `scripts/`,
    `references/`, and `assets/`.
  - Codex uses progressive disclosure: metadata first, `SKILL.md` only after
    selection, references/scripts only as needed.
  - Global skills live under `$HOME/.agents/skills`; repo skills live under
    `.agents/skills`.
- `https://developers.openai.com/codex/guides/agents-md`
  - Codex reads `AGENTS.md` / `AGENTS.override.md` before work.
  - Global guidance comes from `~/.codex`, then project guidance is layered from
    repo root down to cwd.
  - Closer files override earlier guidance because they appear later.
  - Codex stops once combined project docs hit `project_doc_max_bytes`, default
    32 KiB.
- `https://developers.openai.com/codex/config-reference#configtoml`
  - User config lives in `~/.codex/config.toml`.
  - Project overrides can live in `.codex/config.toml` only for trusted projects.
  - Project-local config cannot override machine-local provider, auth,
    notification, profile, or telemetry routing keys.
  - `model = "gpt-5.5"` is a valid model setting; GPT-5 models support
    configurable reasoning and verbosity where available.
- `https://developers.openai.com/codex/prompting`
  - Codex works in a loop of model calls, file reads, edits, and tool calls until
    the task is complete or cancelled.
  - Codex output improves when prompts include validation steps.
  - Split complex work into focused steps and use plans when needed.

## When To Refresh

Refresh the official docs when auditing:

- skill discovery, skill metadata, or global/repo skill locations;
- `AGENTS.md` discovery, precedence, fallback filenames, or byte limits;
- `.codex/config.toml` ownership and trusted project behavior;
- sandbox, approval, hooks, MCP, apps, plugins, memories, or model settings;
- GPT-5.5-specific reasoning, verbosity, context, or web search behavior.
