---
name: looper
disable-model-invocation: true
description: >
  USER-INVOKED ONLY. Use when the user explicitly invokes $looper or /looper
  or asks to design, audit, review, or convert a task into a Codex loop:
  design a loop, make this a loop, turn this into a Codex loop, Theo loop,
  Pete loop, loopify, inner loop, watch this PR, loop until review passes,
  or create a dynamic workflow. Do not invoke for normal implementation,
  review, or planning unless the user asks for a loop.
license: MIT
compatibility: Designed for Codex CLI with the Agent Skills format. Emits raw
  `/goal` payloads, `.agents/loops/` artifacts, and runner specs; defers to
  `goalify`/`codex-goal` for long protected goals.
metadata:
  version: "0.2.1"
  author: "bengous"
---

# Looper

Co-design bounded agent loops with the user: interrogate, challenge, propose.
The user decides; nothing is emitted before their explicit ok.

A loop is a bounded control policy around agent work: observe state, act or
delegate, validate on evidence, then continue, stop, pause, or escalate. A
bound (max iterations, budget, poll interval) only binds where a mechanism
enforces it: a runner enforces it in code; a `/goal` payload or loop
artifact merely states it and relies on the executing agent honoring it.
Every design names which of the two it relies on.

## Contract

- Design only. The deliverable ends at the emitted payload, spec, or
  artifact. Do not create, set, update, pause, resume, clear, or complete an
  actual Codex goal (writing a protected goal file per Emission is emission,
  not goal creation), and do not start executing a designed loop, even when
  asked to "just run it" — emit, then hand over.
- The user owns the loop's shape, bounds, and side effects. A configurable
  control number (iteration cap, budget, poll interval, batch size) enters
  a design in exactly two ways: the user set it during the dialogue, or it
  appears as `<user sets — suggestion: X>` and stays marked until the user
  confirms it; facts read from the repo (PR numbers, versions) need no
  marker. The ok on a recap confirms each marker that recap displays:
  replace them with their values, so no emitted payload, spec, or artifact
  contains a marker.
- Emission happens only after the user answers ok to a recap (dialogue step
  3). A qualified ok ("ok, but...") is an amendment: emit nothing, revise,
  recap again. Review and Examples modes return inline text and emit
  nothing.

## Side-effect classes

Classify the designed loop's execution, not looper's own one-time emission,
by the strongest side effect it can cause: the deepest row it reaches in
this table, ordered weakest to strongest. The class appears in the recap
and in every emitted payload, spec, or artifact.

| Class | Meaning |
| --- | --- |
| Read-only | research, planning, PR/CI/issue inspection, local status checks |
| External watcher | polls GitHub, CI, trackers, feeds, production-like state |
| Local artifact | writes local non-source files: plans, reports, state, logs |
| Code-editing | modifies tracked files or generated source |
| Branch/PR | creates branches, pushes, opens/updates/comments on PRs |
| Merge/deploy/system | merges, deploys, installs, credentials, other external mutations |

Merge/deploy/system actions never run unattended: each one is a human gate
inside the loop. For client or production repos, prefer a pause gate over
agent discretion.

## Entry gate

Route the request before any design work:

- A single direct action or one plain prompt covers it → name that action,
  stop. No loop shapes, no template.
- The user submits an existing loop, goal, or runner to audit → Review mode.
- The user asks what could be looped here → Examples mode.
- Otherwise → co-design dialogue.

## Co-design dialogue

1. **Interrogate.** One question at a time, each with your recommended
   answer. When the answer is checkable in the repo (CI commands, open PRs,
   test runners, workflows), inspect instead of asking. Cover until new
   answers stop changing the design — typically 3-6 questions:
   - observed state: what the loop watches;
   - allowed actions and the resulting side-effect class;
   - what stays human: decisions that must return to the user;
   - bounds: iterations, budget, intervals — the user sets the numbers;
   - validation: what evidence proves an iteration worked;
   - weight: goal payload, runner spec, or durable artifact.
2. **Challenge and propose.** Present 1-3 loop shapes. Object where the
   intent deserves it: loop not justified, wrong boundary, missing gate,
   runaway risk, cheaper alternative. Name tradeoffs. The user picks or
   amends; iterate here until a shape stabilizes.
3. **Recap and wait.** Restate the agreed loop in one compact block: shape,
   observed state, actions and side-effect class, human gates, bounds and
   their enforcement mechanism (still-unconfirmed numbers keep their
   markers), validation evidence, stop and pause conditions, escalation
   points. End with exactly one question: ok to emit?
4. **Emit.** On the user's ok, produce the agreed artifact per Emission.

## Emission

Match the artifact to the weight agreed in the dialogue.

**Goal payload** — light loops: watchers, retry-until-green, single-PR
feedback. Output the raw payload only; the human types `/goal` themselves.
Objective sentence first with no label, then the sections the dialogue
settled: Context / Constraints / Loop structure / Gates / Validate with /
Stop when / Pause if / Escalate when / Allowed side effects / Forbidden
side effects. The payload states that its bounds are behavioral
commitments, not enforced limits. Over 4000 characters: read the goalify
skill (sibling `../goalify/SKILL.md`), run its entry-gate helper check,
follow its protected goal-file procedure (its shorter canonical section
list does not replace the sections above), and return its exact
`Wrote` / `Protected` / `Usage` result block.

**Runner spec** — heavy loops: persistent state across sessions, slices,
sidecar reviewers, ledgers, unattended stretches. A Markdown spec for a
driver program whose outer loop is deterministic code, in the spirit of the
deep-modules `ralf-loop` runner (local reference when present:
`~/projects/etch/.local/deep-modules/ralf-loop.go`): state file, phase
sequence, fresh
implementer per unit, reviewer sidecars, feedback pass back to the
implementer, validation commands, JSONL run/agent ledgers, human gates and
escalation points as prompts in the driver. The spec fixes the workflow policy (units, prompts,
done criteria, gates) and leaves engine internals to the implementation.
This is a reference form for heavy cases, not the default; most loops do not
need a runner.

**Loop artifact** — a durable spec another agent will execute later. Write
`.agents/loops/<slug>.md` containing the recap's agreed sections: purpose,
loop structure, gates, bounds with enforcement, validation,
stop/pause/escalation, audit-log expectations. Sections the dialogue never touched do not appear.

Writing an artifact is never approval to run the loop.

## Review mode

Audit the submitted loop for: unbounded iteration or watching, token
runaway, missing human gates, unsafe side effects, self-reported validation,
unclear success criteria, scope creep. Return a revised loop, not only a
critique:

```markdown
## Audit

## Required changes

## Revised loop

## Remaining risks
```

The revised loop is inline text; emitting it as a goal payload, runner
spec, or loop artifact goes through dialogue step 3 and its ok.

## Examples mode

Practical shapes, tailored to the current repo when possible:

- PR feedback watcher: poll one PR's CI and review threads, fix what the
  gates allow, pause on ambiguity.
- Stacked PR loop: slice a project into dependent PRs; review/fix per PR;
  merges stay human.
- Maintenance batch: bounded dependency or hygiene batches, plan gated
  before any edit.
- Repo cleanup: one bounded cleanup theme per run, diff-size stop condition.
- Skill iteration: edit a skill in small validated iterations, evals gated
  by cost.
- Implement-review runner: ralf-loop shape — fresh implementer per slice,
  sidecar reviewers, feedback pass, human verdict at the end.

## Red flags

- About to emit a payload, spec, or artifact with no ok'd recap above it →
  back to step 3.
- A configurable control number in the design the user never set or
  confirmed → mark it `<user sets — suggestion: X>`.
- About to write "say go and I'll start the loop" → you design; offer the
  artifact instead.
- Loop shapes drafted for work the entry gate should have caught → answer
  with the single action.
