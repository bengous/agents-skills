---
name: goalify
description: Use this skill when the user wants to convert rough coding, product, architecture, audit, review, migration, cleanup, debugging, or orchestration intent into a compact Codex `/goal` payload. Trigger on "goalify", "Codex goal", "/goal", "make this a goal", "RALF loop", "turn this into a Codex prompt", "$goalify interactive", or requests to be questioned before creating a goal. Do not use for normal implementation or review unless the user explicitly asks to produce a Codex goal/prompt.
license: MIT
compatibility: Designed for Codex CLI 0.128+ with GPT-5.5 and the Agent Skills format. Requires the local Linux `codex-goal` helper for protected long goal files.
metadata:
  version: "0.2.0"
  author: "bengous"
---

# Goalify

Convert messy intent into the smallest useful payload to put after Codex `/goal`.

You are not doing the downstream implementation. You are producing goal text the human can paste after `/goal`, or a protected local file path the human can pass to `/goal`.

Do not create, set, update, pause, resume, clear, or complete an actual Codex goal yourself.

## Entry Gate

Before asking clarification questions or generating any goal payload, verify the mandatory helper:

```bash
test -x /usr/local/bin/codex-goal
test ! -L /usr/local/bin/codex-goal
[[ "$(/usr/bin/stat -c '%u:%g:%a' /usr/local/bin/codex-goal)" == "0:0:755" ]]
/usr/local/bin/codex-goal --version >/dev/null
```

If either command fails, stop. Explain briefly that `codex-goal` is required to write immutable goal files, especially when Codex runs with `danger-full-access`. Read `script/references/install.md`, summarize the exact system side effects, and ask for explicit approval before running any installer. Do not install automatically.

## Modes

Default mode is draft-first. Use it for `$goalify`, `make this a goal`, and similar requests. Infer reasonable details and ask only when a missing detail materially changes scope, risk, or acceptance criteria.

Interactive mode is question-first. Use it when the user says `$goalify interactive`, `interroge-moi`, `pose les questions une par une`, `build this goal with me`, or equivalent. Ask one question at a time. Each question must include your recommended answer. If a question can be resolved by inspecting the codebase or local docs, inspect instead of asking.

## Core Principle

`/goal` needs the durable destination, not a runbook.

Include only information that changes execution:

- objective
- relevant context
- constraints and side-effect boundaries
- success criteria
- validation or evidence
- stop condition
- pause/blocker condition

Do not include generic role text such as "You are Codex CLI". Do not restate obvious working-directory or repository facts unless they materially affect the goal.

## Prompt Shape

Use compact GPT-5.5-oriented structure. Prefer these labels when they carry real information:

```text
Objective:
Context:
Constraints:
Success means:
Validate with:
Stop when:
Pause if:
```

Only include useful sections. For simple goals, fewer sections are better.

## Output Rules

The human types `/goal` manually. Do not prefix short output with `/goal`.

For payloads at or under 4000 characters, output only the raw goal payload. Do not add headings, analysis, notes, context placement tables, or a separate session prompt.

For payloads over 4000 characters, write the payload automatically with `codex-goal` and do not reprint the payload:

```bash
/usr/local/bin/codex-goal --root "$PWD" --slug "$SLUG" <<'EOF'
<goal payload>
EOF
```

Use a single-quoted heredoc delimiter so shell interpolation cannot alter the payload.

After a successful write, output only the helper result shape:

```text
Wrote: .codex/goals/<slug>.md
Protected: immutable

Usage:
/goal .codex/goals/<slug>.md
```

If the target root is not the current working directory, use the absolute path printed by `codex-goal`.

## Slug Rules

Generate slugs automatically from the objective:

- lowercase ASCII
- punctuation and spaces become `-`
- strict safe pattern: `[a-z0-9][a-z0-9._-]{0,80}`
- no `.md` suffix in the slug
- truncate to roughly 60-80 characters
- if there is a collision, add a short suffix

Ask for a slug only if one cannot be derived.

## Clarification Rules

Extract only what affects the goal payload:

- task type
- deliverable
- scope boundaries
- source-of-truth files or artifacts
- acceptance criteria
- validation commands or evidence
- side-effect permissions
- manual approval gates
- stopping condition
- blockers that should pause the goal

Ask at most one question at a time in interactive mode. In default mode, ask only for blocking ambiguity.

## Helper Contract

`codex-goal` is the required local writer for protected long goal files. Agents call `/usr/local/bin/codex-goal` directly; the installed wrapper performs the narrow non-interactive sudo call to the privileged helper. The helper writes under `.codex/goals/`, keeps the file owned by the invoking user, sets mode `0444`, sets the Linux immutable flag, and verifies protection before success.

Do not fall back to weak `chmod`-only file writing.

## Quality Check

Before final output, verify:

- the output is a goal payload, not a wrapper
- short output does not start with `/goal`
- no separate session prompt exists
- no generic Codex role preamble is included
- success criteria are observable
- validation or evidence is specified when useful
- stop and pause conditions are clear
- destructive or external side effects require explicit approval
- long payloads are written through `codex-goal`
