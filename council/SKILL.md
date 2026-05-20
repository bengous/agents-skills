---
name: council
description: Convene a small council of fresh Codex subagents to challenge, research, and zoom out on the current problem. Use when the user invokes "$council", asks for a council of agents, wants multiple external viewpoints, wants the current topic challenged, or needs independent perspectives before deciding. If invoked with no extra prompt, infer the current subject from the active conversation and ask clean-context subagents to review it.
---

# Council

Get independent outside views on the current problem without dragging subagents through the whole conversation.

## Contract

When the user invokes `$council` with no extra text:

- Treat it as explicit permission to spawn subagents.
- Infer the current problem, decision, plan, code change, or question.
- Do not ask for clarification unless the current subject cannot be identified.
- Spawn clean-context subagents with `fork_context: false`.
- Brief each subagent with only the neutral problem statement and needed files, commands, URLs, or facts.

When the user adds a topic after `$council`, use that as the brief.

## Roles

Default to 3 subagents unless the problem is tiny:

- **Skeptic**: find weak assumptions, risks, missing constraints, and likely failure modes.
- **Researcher**: verify current or external facts, prioritizing official docs/sources.
- **Practitioner**: assess practical execution, repo fit, implementation path, testing, and operational tradeoffs.

For codebase questions, replace these with area-specific reviewers when that gives better coverage.

## Brief

Each subagent prompt must be short and self-contained:

```text
You are one member of a council. Clean context: assume no prior conversation.

Problem:
[neutral description]

Lens:
[skeptic/researcher/practitioner/etc.]

Evidence to inspect:
[paths, commands, URLs, or "none provided"]

Return:
- strongest concerns or recommendations
- supporting evidence
- what would change your mind
```

Do not leak private or irrelevant conversation history. For repo behavior, inspect files locally first and pass exact paths or commands.

## Synthesis

After subagents respond:

- Present a synthesis, not a transcript.
- Separate consensus from disagreement.
- Highlight the strongest challenge, recommended next move, and evidence gaps.
- Do not force agreement when disagreement is useful.
