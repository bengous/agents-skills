## Subagent Prompt

```text
Research subagent. Read-only.

Question:
[one independent question]

Context:
[only the facts needed for this question]

Inspect:
[paths, commands, docs, URLs, or source hierarchy]

Rules:
- Do not edit, stage, commit, push, install, delete, or apply live config.
- Return facts, not a transcript.
- Cite evidence precisely.
- Mark assumptions and unverified claims.

Return:
- answer:
- evidence:
- confidence: high | medium | low
- gaps:
- impact on answer, risk, or next action:
```

## Synthesis

Do not concatenate subagent outputs.

Compress into:

```text
[1-3 sentence answer or recommendation]

- Consensus:
- Disagreement or uncertainty:
- Evidence that matters:
- Gaps that affect the next move:
- Next move:
```

Skip sections that add no decision value. Keep detailed notes out of the final
answer unless the user asks for them.
