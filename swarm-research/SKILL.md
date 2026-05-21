---
name: swarm-research
description: Use when read-only research clearly splits into multiple independent code, docs, logs, tests, or web questions worth parallel subagents and concise synthesis.
---

## Gate

Use a swarm only for broad read-only research that may benefit from parallel
inspection. Handle inline when the work is small, sequential, obvious, or
already ready for the next action.

## Decompose

Before spawning, identify the decision the research must support. Walk the
decision tree yourself first. Keep only task-grounded questions that are all:

- independent;
- verifiable from code, docs, web sources, logs, tests, or explicit context;
- capable of changing the synthesis, recommendation, risk, or next action.

If fewer than two questions remain, handle inline.

## Agents

Use only built-in agent types. `explorer` is for focused codebase research;
`default` is for docs, web, general reasoning, and non-codebase research.

Subagents are read-only: they may inspect files, run read-only commands, and
consult docs or web sources when relevant. They must not edit, stage, commit,
push, install, delete, or apply live config.

Use clean-context subagents by default; fork context only when exact
conversation state cannot be summarized safely.

## Workflow

1. Select the surviving questions from `Decompose`.
2. If a swarm is still warranted, read `references/research-brief.md` for the
   subagent brief and synthesis template.
3. Spawn parallel agents with precise, non-overlapping briefs, then wait for
   the launched agents.
4. While they run, do at most one small read-only lookup, only to prepare the
   synthesis or resolve an immediate orchestration gap. Do not answer a
   delegated research question yourself.
5. Synthesize returned evidence. If an agent blocks, times out, or goes
   off-topic, continue with available results and name the gap.
6. Synthesize for the human; do not paste transcripts.

Own the final synthesis and decision. Do not delegate the final answer to
another subagent.

## Output

Use progressive disclosure:

- Start with the recommendation or answer in 1-3 sentences.
- Keep the default synthesis short; expand only when the task needs a report,
  audit trail, or detailed handoff.
- Include only evidence that affects the answer, risk, or next action.
- Separate verified facts, inferences, and meaningful gaps.
- End with one next move, or at most two options when there is a real fork.
