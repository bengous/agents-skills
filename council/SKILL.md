---
name: council
description: Convene 3 fresh Codex subagents to give independent opinions on a question, idea, hypothesis, plan, or ambiguity. Use when the user invokes "$council", asks for "l'avis de 3 sous agents", wants multiple external viewpoints, says "je me demande si..." and appears to want validation, fact-checking, challenge, ambiguity-breaking, or a smarter decision review. If invoked with no extra prompt, infer the current subject from the active conversation and ask clean-context subagents to review it.
---

# Council

Get 3 independent outside views on the current question without dragging subagents through the whole conversation.

## Contract

When the user invokes `$council` with no extra text:

- Treat it as explicit permission to spawn subagents.
- Infer the current problem, decision, plan, code change, or question.
- Do not ask for clarification unless the current subject cannot be identified.
- Spawn clean-context subagents with `fork_context: false`.
- Brief each subagent with only the neutral problem statement and needed files, commands, URLs, or facts.
- If the user named a model or reasoning level, carry it through. Otherwise use the host defaults.

When the user adds a topic after `$council`, or asks for 3 subagent opinions in prose, use that as the brief.

## Roles

Default to exactly 3 subagents:

- **Skeptic**: find weak assumptions, risks, missing constraints, and likely failure modes.
- **Researcher**: verify current or external facts, prioritizing official docs/sources.
- **Practitioner**: assess practical execution, repo fit, implementation path, testing, and operational tradeoffs.

For codebase, product, strategy, or writing questions, replace these with sharper lenses when that gives better coverage. Keep one lens adversarial, one evidence-focused, and one practical.

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
