---
name: shrinkify
description: Interactively shrink AI agent skills while preserving their useful semantics. Use when the user wants to reduce, compact, slim, shrinkify, deslop, or optimize a SKILL.md or skill folder, especially when they want fewer tokens without losing triggers, output shape, decision rules, validation behavior, or recurring workflow quality. Supports explicit audit and apply modes.
---

# Shrinkify

Reduce skills without changing what matters.

Default posture: interactive, conservative, semantics first. Token reduction is the goal, but not at the cost of the skill's real job.

## Modes

Use the mode the user requested. If unclear, start in `audit`.

- `audit`: inspect and discuss. Produce findings, proposed cuts, risks, and questions. Do not edit.
- `apply`: inspect, ask required questions, edit the skill, then validate.

If the user says "just do it", "shrink this", "apply", "fais la modif", or equivalent, treat that as `apply`.

## Preserve

Keep these unless the user explicitly gives them up:

- Trigger semantics: when the skill should and should not activate.
- Output contract: required sections, formatting, strict final response rules, copy-paste blocks.
- Decision rules: priority order, constraints, escalation conditions, safety boundaries.
- Progressive disclosure: references, scripts, and assets that keep `SKILL.md` small.
- Validation behavior: commands, checks, eval criteria, or smoke tests needed to trust the result.
- User voice: terse, strict, interactive, audit-first, or any explicit style the skill encodes.

## Cut

Prefer removing or compressing:

- General explanations a capable model already knows.
- Repeated restatements of the same rule.
- Long examples that can become one compact canonical example.
- Motivation and origin story.
- Lists of obvious cases when a single discriminating rule works.
- Anti-patterns that duplicate the positive instruction.
- Implementation detail that belongs in a reference file or script.

Do not remove an anti-pattern if it protects against a failure mode the skill repeatedly hits.

## Workflow

### 1. Establish The Target

Identify:

- Skill path and current size.
- Whether the user wants `audit` or `apply`.
- The main use cases they still care about.
- Any behavior they want deleted because the skill is stale.

Read the whole `SKILL.md`. Inspect referenced resources only when needed to understand whether content can move out of the body.

### 2. Extract The Contract

Write a short private contract before editing:

- `Triggers`: activation phrases and contexts.
- `Core job`: one sentence.
- `Must preserve`: output format, sequencing, hard constraints.
- `Can shrink`: verbosity, examples, redundant warnings, obvious background.
- `Can delete`: stale, unused, speculative, or contradicted material.

If any item changes the result users would see, ask before cutting it.

### 3. Ask Only High-Impact Questions

Ask the user when the answer is not discoverable and changes the rewrite:

- "Which outputs from this skill are sacred?"
- "Which use cases have you stopped using?"
- "Should this become audit-first or edit-first?"
- "Can this long example become a reference file?"
- "Is token count or behavioral fidelity more important here?"

Do not ask where files are if the repo answers it. Do not ask permission for obvious compression.

### 4. Rewrite

Apply these transformations in order:

1. Compress frontmatter without losing trigger coverage.
2. Replace explanatory prose with imperative rules.
3. Merge duplicate sections.
4. Convert long checklists into short gates.
5. Move optional detail into `references/` only when it will be read conditionally.
6. Delete stale sections.
7. Keep one compact example only if it prevents format drift.

For `apply`, edit narrowly. Do not rewrite unrelated resources unless the shrink requires moving content.

### 5. Self-Review

Compare before and after:

- Would the same user request still trigger this skill?
- Would the agent choose the same mode or workflow?
- Would the final answer shape stay compatible?
- Did any hard "do not" rule disappear?
- Did the rewrite make the skill too vague to use cold?

If the answer is uncertain, stop and surface the risk instead of pretending parity.

## Output

### Audit Mode

Use this format:

```markdown
## Shrinkify Audit

Current size: [lines or rough token signal]
Target behavior: [one sentence]

### Keep
- [semantic contract items]

### Cut Or Compress
- [specific sections or patterns]

### Questions
- [only blockers or high-impact choices]

### Proposed Shape
[short outline of the smaller skill]
```

### Apply Mode

After editing, respond with:

- What changed.
- What semantics were preserved.
- Any intentional losses.
- Validation run and result.

Keep the final short. The diff is the source of truth.

## Validation

Always run the repo's skill validator when available. Common commands:

```bash
cargo run -p skills-tools -- validate frontmatter
```

For meaningful rewrites, add or run one behavior smoke test:

- Trigger smoke: would likely user phrases still activate the skill?
- Output smoke: does a sample prompt still produce the required format?
- Reference smoke: moved content is discoverable from `SKILL.md`.

Evals are optional. Recommend them only for high-use skills where output parity matters and examples exist.

## Guardrails

- Do not optimize by hiding important behavior in vague words like "be concise".
- Do not delete validation because it costs tokens.
- Do not turn a skill into a generic prompt-engineering essay.
- Do not keep a long section only because it is well written.
- Do not silently change from interactive to autonomous, or from audit-first to apply-first.
- If the skill is obsolete, say so and propose archive/delete instead of shrinkifying it.
