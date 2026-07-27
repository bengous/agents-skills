# Spawning read-only workers

Agent-type names and parameter spellings below are harness API surface and drift
between releases. Checked 2026-07. If a name is rejected, re-check against the
live harness rather than guessing a variant.

## What actually enforces read-only

| Harness | Mechanism | Actually restricted? |
|---|---|---|
| Codex | `sandbox_mode = "read-only"` on the agent, or `codex exec -s read-only` | Yes. Real sandbox. |
| Claude Code | `subagent_type: "Explore"` | Partly. Write/Edit/NotebookEdit denied; **Bash stays open**. |
| Claude Code | `subagent_type: "general-purpose"` | No. Full tool access; the brief's rules are the only thing holding. |

Keep the read-only rules in the brief regardless — they cost nothing and are the
only guard in the unrestricted case. But never present a worker's output as
sandboxed when only prose held it.

## Claude Code

Valid `subagent_type`: `Explore`, `Plan`, `general-purpose`, any custom agent
name, and `fork` when `CLAUDE_CODE_FORK_SUBAGENT=1`.

An unknown value does not error. It silently starts a `general-purpose` agent:
full tool access, no restriction, no warning. A typo costs you the restriction
you thought you had asked for.

- `Explore` — locating things in a codebase. It reads excerpts rather than whole
  files, so it finds where something lives and misses what it means.
- `general-purpose` — docs, web, logs, and anything needing real reading and
  synthesis. Unrestricted; the brief is the only guard.

`fork` inherits the parent's exact tool pool and skips restriction filters. It is
a prompt-cache optimization, not a safer option. Default to clean context.

Concurrency comes from issuing several `Agent` blocks in a single message. One
call per turn runs sequentially. Caps: 20 concurrent, 200 per session, nesting
depth 3.

`model` accepts `sonnet`, `opus`, `fable`, `inherit`.

## Codex

Collaboration tools: `spawn_agent`, `send_message`, `followup_task`,
`wait_agent`, `interrupt_agent`, `list_agents`. They cannot be called from inside
`functions.exec` — only as direct tool calls.

`agent_type` must name a registered agent. List what is registered before
spawning instead of assuming a name resolves; a TOML sitting in the agents
directory is not proof it is registered in live config.

`fork_turns` controls inherited context: `"none"`, `"all"`, or a positive integer
as a string. It supersedes `fork_context`, which MultiAgentV2 rejects outright.
Installs still on the older generation take the old spelling, so check which one
is live before writing either.

Per-worker `model` and `reasoning_effort` overrides are refused on a full-history
fork (`fork_turns` omitted or `"all"`), which also inherits the parent's
`agent_type`. Tiering models per worker therefore requires `fork_turns: "none"`
or an integer.

### When native multi-agent is unavailable

Degrade in this order, and state which rung you landed on:

1. A registered read-only agent, if listing shows one.
2. One `codex exec -s read-only "<self-contained prompt>"` per question, run in
   parallel, each writing to its own scratch file. Same sandbox guarantee, no
   installation required.
3. Sequential inline research — name the degradation in the synthesis.
