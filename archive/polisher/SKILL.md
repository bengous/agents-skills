---
name: polisher
description: Run a post-feature polishing loop over the current WIP or branch diff. Use when the user wants Codex to simplify recently generated code, reduce slop, make TypeScript code more expressive with justified types/helpers, or run clean-context reviewers before accepting a change set.
license: MIT
compatibility: Designed for Codex CLI with custom agent types installed from this skill.
metadata:
  version: "0.1.0"
  author: "bengous"
---

# Polisher

Run a post-feature polishing loop on the current change set. The owner agent keeps
write authority. Reviewers stay clean-context and read-only.

## Setup Gate

Before spawning `polisher-*` agent types, verify they are installed:

```bash
<skill-dir>/scripts/setup status
```

If status fails, explain that setup writes live Codex runtime config under
`~/.codex/agents/` and `~/.codex/config.toml`, then ask before running:

```bash
<skill-dir>/scripts/setup install
```

Do not install automatically.

## Execution Model

The loop is checkpoint-driven, not a procedural runbook.

1. Checkpoint: map the current change set against the base ref.
2. Validate current state if validation is known and cheap enough.
3. Spawn clean-context read-only reviewers in parallel:
   - `polisher-simplification-reviewer`
   - `polisher-expressiveness-reviewer`
   - `polisher-architecture-reviewer`
4. Owner aggregates findings and accepts, rejects, or defers each one.
5. Apply at most one small accepted behavior-preserving fix.
6. Validate.
7. Repeat from checkpoint until the stop condition is met.

Do not overlap editing, validation, reviewer collection, and decision work.

## Change Set

Default base ref is `origin/main`.

Compare the full current work against the base:

- committed branch changes such as `<BASE_REF>..HEAD`
- staged changes
- unstaged changes
- relevant untracked code files

Use the target repo where the user invoked the loop. Do not edit this skill or
template files unless the user explicitly asks to change the skill itself.

## Owner Rules

- Preserve behavior.
- Keep fixes local to the current change set and directly relevant neighbors.
- Prefer one accepted fix per iteration.
- Do not broaden product behavior, architecture, public API, persistence,
  routing, or data model.
- Do not apply formatting-only churn.
- Do not edit generated files directly when a source file exists.
- Do not commit, push, publish, tag, install, or apply live config unless the
  user explicitly requests it.
- If validation fails because of the accepted fix, repair or revert that fix
  before continuing.

## Finding Gates

Accept only findings that are:

- concrete and evidenced with file:line references
- behavior-preserving
- local to the current change set
- simpler, clearer, or more idiomatic than the current code
- small enough to validate in the loop

Reject findings that are:

- speculative
- formatting-only
- broad architecture work
- public API or product behavior changes
- type decoration without readability or correctness leverage
- changes that add ceremony without making the code easier to understand

Architecture findings are deferred by default. Implement them only when the
finding is also a small local behavior-preserving simplification accepted by the
owner.

## Stop Condition

Stop when validation passes and either:

- no reviewer reports an actionable local improvement, or
- all remaining findings are rejected or deferred with reasons.

Final report:

- implementation attempts
- reviewer passes
- accepted findings
- rejected findings
- deferred architecture notes
- validation commands and results
- final dirty state
