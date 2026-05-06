---
name: promptify
description: "USER-INVOKED ONLY. Use only when Augustin explicitly invokes $promptify or explicitly names this skill with one raw prompt argument. Transform a rough dictated instruction into a clean GPT-5.5-optimized prompt. If blocking ambiguities remain, do not produce the final prompt; route to $grill-me first."
---

# Promptify

## Input

One parameter only:

```text
prompt: <raw dictated instruction>
```

If no prompt is provided, ask for the raw prompt. Do not ask a questionnaire upfront.

## Purpose

Turn a messy intention into a prompt that GPT-5.5 can execute reliably: explicit goal, context, constraints, source-of-truth boundaries, allowed actions, output shape, validation, and stopping rules.

## GPT-5.5 Prompt Shape

Prefer a compact, outcome-first prompt:

```text
Objective:
Context:
Inputs:
Constraints:
Allowed actions:
Clarify first if:
Output:
Validation:
Stop when:
```

Only include sections that matter. Keep the final prompt shorter than the raw prompt when possible.

## Ambiguity Gate

Before writing the final prompt, check for blockers:

- unclear objective or success criteria;
- missing target audience, repo/path, channel, or artifact;
- unclear write/send/commit permission;
- ambiguous source of truth;
- hidden tradeoff that changes the work;
- missing output format;
- high-stakes or irreversible action without confirmation.

If any blocker remains, do **not** output the final prompt. Instead, invoke `$grill-me` with the smallest clarification brief:

```text
$grill-me
Goal: clarify this raw prompt before Promptify writes the final GPT-5.5 prompt.
Raw prompt: <raw prompt>
Blocking ambiguities: <short list>
Ask one question at a time. Stop when the final prompt can be written safely.
```

After `$grill-me` resolves the blockers, resume and produce the final prompt.

## Rewrite Rules

- Preserve the user's real intent; do not add ambition, scope, or claims.
- Remove filler, repetitions, speech tics, and chronology noise.
- Convert vague verbs into concrete actions.
- Make side effects explicit: edit files, send mail, commit, browse, ask, or report-only.
- Put uncertainty in `Clarify first if`, not hidden assumptions.
- Add validation only when it changes reliability.
- Do not include chain-of-thought requests.

## Output

If clear:

```text
Prompt final GPT-5.5:

<copy-pasteable prompt>
```

If not clear:

```text
Ambiguites bloquantes:
- ...

Je lance $grill-me avant de generer le prompt final.
```
