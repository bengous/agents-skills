---
name: handoff-prompt
description: Generate a handoff prompt for another agent instance. Use when the user asks to write a prompt, meta-prompt, or briefing for a separate agent session — phrases like "write a prompt for another instance", "make a handoff prompt", "brief another agent", "prepare a prompt I can copy-paste", "write this up for another session", or when the user has been discussing/planning a task and wants to delegate execution to a fresh agent instance. Also triggers on "handoff", "meta-prompt", "delegate to another agent".
---

# Handoff Prompt

You are the **advisor** — you've been discussing, researching, or planning with the user. Now they want a prompt that a fresh **executor** instance will receive to do the actual work.

The executor has **zero context** from this conversation. It will read the prompt cold.

## What makes a good handoff prompt

The prompt answers three questions for the executor:
1. **What** — the goal, stated as intention, not as steps
2. **Why** — enough context that the executor can make judgment calls when ambiguity arises
3. **Where to look** — pointers to authoritative files, not summaries of their content

What it deliberately omits:
- **How** — no implementation plan, no step-by-step, no suggested approach. The executor figures this out with the user. Giving a plan constrains the executor's thinking and creates false confidence.
- **Summaries of docs** — summaries go stale and lose nuance. A file path is always more accurate than your paraphrase of that file.

## Generating the prompt

### 1. Mine the conversation

Before writing, extract from the current conversation:

- **Decisions made** — what the user chose and why (not just what, the *why* matters for the executor's judgment)
- **Constraints discovered** — anti-patterns, things that don't work, boundaries the user expressed
- **Authority documents** — files, docs, ADRs, specs that the executor should read. Prefer paths over URLs
- **Prior art** — if similar work was done before (e.g., "4 domains already migrated this way"), point to it as a reference the executor can study

### 2. Clarify before writing

Review what you extracted in step 1. If any of the following are true, use `AskUserQuestion` to resolve them **before** writing the prompt:

- A decision was discussed but never explicitly confirmed
- The scope is ambiguous (could reasonably be interpreted two ways)
- You're unsure whether something from the conversation is a hard constraint or just an idea that was floated
- Authority documents were mentioned vaguely ("the architecture doc") and you need to confirm which file
- Prior art exists but you're not sure it's the right reference for this task

Ask only what you genuinely can't resolve from the conversation. If the intent, scope, and constraints are all clear — skip this step entirely and go straight to writing. Forcing unnecessary questions is worse than skipping them: it signals you weren't paying attention.

Bundle related uncertainties into a single question when possible, rather than asking one at a time.

### 3. Write the prompt

Use this structure. Every section is optional — include only what's relevant. The prompt should be as short as it can be while remaining unambiguous to a reader with no context.

```markdown
# [Goal as a noun phrase]

## Context
[2-5 sentences. What exists, what state things are in, why this work matters.
Include anything the executor can't discover by reading code alone.]

## Intention
[1-3 sentences. What the user wants to achieve. Frame as outcome, not process.]

## Authority
[Bulleted list of file paths the executor should read before proposing anything.
One line per file, with a brief note on what it contains — just enough to triage reading order.]

## Constraints
[Bulleted list of things learned during the conversation.
Anti-patterns, boundaries, non-obvious requirements.
Each one should be something the executor wouldn't guess on their own.]

## Prior art
[Optional. Point to existing code/commits that exemplify the pattern to follow.
"Look at X, it was done the same way" is more useful than explaining the pattern.]
```

### 4. Self-check before presenting

- Could someone with zero context read this and understand the goal? (no jargon from *this* conversation leaking in)
- Does it tell the executor *what* to achieve without telling them *how*?
- Are all document references file paths, not paraphrases?
- Is every constraint something the executor couldn't figure out from the code alone?
- Is it short enough to actually read? (aim for under 40 lines)

### 5. Present

Output the prompt as a single fenced markdown block the user can copy-paste directly. **Nothing else.** No preamble, no postamble, no trailing notes, no meta-commentary about what was included or omitted, no "here's what I wrote", no "let me know if…". The block is the entire response.

If something is genuinely worth flagging (an intentional omission, an assumption you made), put it *inside* the prompt — in `Context` or `Constraints` — so the executor actually receives it. Notes outside the block don't reach the executor and force the user to strip them by hand.

## Anti-patterns

- **The trailing note** — adding "Note: I left out X because…" or any commentary after the fenced block. The user copy-pastes the block into a fresh session; anything outside it is noise they have to delete. If it matters, put it in `Context`.
- **The plan leak** — sneaking implementation steps into "context" or "constraints". If it sounds like a todo list, it's a plan.
- **The summary trap** — writing 10 lines summarizing a doc instead of giving the path. The executor will read the doc; your summary just adds noise and risks being wrong.
- **The kitchen sink** — including everything discussed in the conversation. Most of it is irrelevant to the executor. Ruthlessly filter.
- **The jargon leak** — using shorthand or references from *this* conversation that the executor won't understand ("like we discussed", "the approach from earlier").
- **Over-constraining** — listing 15 constraints turns the prompt into a spec. 3-6 constraints that the executor wouldn't guess is the sweet spot.
