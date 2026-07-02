---
name: premium-handoff
disable-model-invocation: true
description: "USER-INVOKED ONLY. Use only when the user explicitly invokes $premium-handoff or asks to turn a selected target, targetify seed, evidence summary, design/architecture problem, or premium-model target into a compact handoff prompt/artifact for a scarce frontier model. Do not target the whole repo, implement, or launch the premium model."
---

# Premium Handoff

Turn one selected target or design problem into a compact briefing for an expensive model.

## Contract

You are preparing the premium-model run, not running it.

- User-invoked only: require `$premium-handoff`, `premium-handoff`, or an explicit request for a premium/super/frontier model handoff.
- Accept a `targetify` Handoff Seed, a manually provided target with objective and evidence, or a design problem with constraints and decisions (e.g. distilled from a `$grill-me` session).
- Do not decide where premium attention should go across the repo; use `targetify` for that.
- Do not edit code, create branches, stage, commit, push, open PRs, install dependencies, apply live config, or implement.
- Produce only a handoff prompt or, when requested/needed, a handoff artifact.
- Do not launch any premium model.
- Do not include a generic "you are an AI" preamble or ask for chain of thought.

## Modes

Pick the mode from the input:

- Target mode: the premium model must diagnose or change existing behavior at a known location. Inputs are evidence, files, failures.
- Design mode: the premium model must decide what to build or how to structure it. Inputs are intent, constraints, and decisions, gathered from the user rather than from repo evidence.

A redesign of an existing system uses target mode plus the Rejected Designs and Decision Criteria sections.

## Gate

Target mode: confirm there is one selected target.

- If no target is selected, ask for the target or recommend `$targetify` first.
- If evidence is too weak, say premium handoff is not justified yet and name the missing evidence.
- If a `targetify` report or seed exists, consume it without redoing the full targeting pass.

Design mode: confirm the problem statement and hard constraints exist.

- If constraints, prior decisions, or decision criteria are missing, recommend `$grill-me` to extract them, or ask only the blocking questions.
- Do not recommend `$targetify` for design input; repo evidence cannot produce constraints or intent.

Either mode: if the work is mechanical or low-risk enough for a normal model, recommend not spending premium tokens. Ask only for blockers: missing target or problem, missing objective, missing evidence or constraints, unclear side-effect boundary.

## Input Handling

For a `targetify` Handoff Seed, preserve its fields as-is: target, objective and done criteria, expected premium output, authority files, verified evidence, known traps, constraints, and "do not include" notes. Verify only obvious path existence or stale assumptions when cheap.

For manual input, extract:

- selected target;
- objective and observable done criteria;
- verified evidence: files, commands, failures, logs, test gaps, docs drift;
- assumptions/inferences, each with a cheap check;
- prior attempts and ruled-out approaches;
- authority files to start with;
- side-effect boundaries;
- desired output: diagnosis, options, plan, implementation, review, or validation strategy.

For design input, extract:

- the problem and why now;
- hard constraints (stack, team, ops, budget, compliance) separated from preferences;
- decisions already made, marking the irreversible ones;
- non-goals;
- decision criteria: what makes one option win, ranked;
- pointers to existing code, schemas, or contracts that constrain the design;
- rejected designs and why;
- desired output: options, architecture proposal, plan, or review.

Do not pad missing evidence with summaries. Prefer paths, commands, and raw artifacts (full logs, failing test output, ADRs) over prose summaries of them.

## Handoff Rules

A frontier model explores for two reasons: discovery (locating things, understanding state) and verification (confirming claims before acting). The handoff eliminates discovery and makes verification cheap; verification itself stays with the premium model.

Give the model:

- the objective plus observable done criteria: the one thing it cannot derive;
- pointers with provenance rather than prose summaries: a stale summary anchors the model wrong, a path plus a command ages well;
- each unverified claim labeled as a hypothesis with a cheap check attached;
- ruled-out approaches and failed prior attempts: these prevent the costliest re-exploration;
- start-here files it may read beyond when the brief and the code disagree;
- exact validation commands and expected results, so spot-checking replaces re-derivation;
- side-effect boundaries and the conditions for pausing: missing evidence, high risk, scope expansion.

In design mode, decision criteria play the role of done criteria and rejected designs play the role of known traps; the rest of the list holds.

Provenance rule: every substantive line in the handoff traces back to the user's input or to a named file, command, or log. Options, libraries, or techniques the user mentioned appear verbatim and unranked under Candidates Heard. Anything the preparing model infers or knows beyond that enters only as a Verify First check or a labeled assumption; the solution space belongs to the premium model.

Keep context small. Include enough evidence to prevent rediscovery, not a transcript.

## Output Modes

Default mode is a copy-pasteable handoff prompt in chat.

Artifact mode is allowed only when the user asks for a durable artifact or the prompt is too long for chat. Write a single Markdown file under:

```text
.agents/handoffs/<slug>.md
```

Writing that artifact is the only allowed file mutation. Report the path and do not create supporting folders beyond the handoff path.

## Prompt Shape

Output only the prompt unless you are stopping at the gate.

```markdown
# <selected target>

## Objective
<one outcome-focused paragraph ending with the observable state that means done>

## Read First
- `<path>`: <why this file is authoritative>
- `<command>`: <what it proves>

## Verified Evidence
- <fact> (source: <file/command/commit>)

## Assumptions To Verify
- <assumption>: check via <command or file>

## Known Traps / Ruled Out
- <approach>: <why, e.g. tried in <commit>, reverted because <reason>>

## Scope
Focus on:
- <target boundary>

Do not spend time on:
- <explicit non-targets>

Allowed side effects:
- <match the desired output: read-only for diagnosis/options/review, edits plus tests for implementation>

Forbidden side effects:
- <branches, commits, pushes, installs, PRs, external systems unless approved>

## Deliverable
<one or two sentences: what to hand back and how to prove it, e.g. "a diagnosis naming the root cause, with the commands you ran to confirm it">

## Analysis Instructions
<only instructions that change default behavior for this target, e.g. "propose 2-3 competing options with tradeoffs" for an architectural or security-sensitive change; omit the whole section when defaults suffice>
```

In design mode use this shape instead:

```markdown
# <design problem>

## Problem
<what to build or restructure, why now, ending with the decision this run must produce>

## Hard Constraints
- <non-negotiable: stack, team size, ops maturity, budget, compliance>

## Preferences
- <negotiable, with the tradeoff the user would accept>

## Decided Already
- <decision>: <why it stands; mark irreversible ones>

## Non-Goals
- <explicitly out of scope>

## Existing System
- `<path or artifact>`: <what it constrains>

## Verify First
- <assumption about the existing system>: check via <path or command>

## Rejected Designs
- <design>: <why rejected>

## Candidates Heard
- <option the user mentioned, verbatim, unranked and without commentary; omit the section if none>

## Decision Criteria
<ranked: what makes one option win, e.g. debuggability over raw performance>

## Side Effects
- Allowed: <read-only by default; edits plus tests only if the deliverable includes an implementation slice>
- Forbidden: <branches, commits, pushes, installs, PRs unless approved>

## Deliverable
<e.g. "2-3 competing architectures scored against the decision criteria, a recommendation, and the first implementation slice; pause before implementing">
```

Design runs default to read-only recommendations; when implementation would follow, suggest running the premium model in plan mode.

Trim unused sections in either shape. If the user asked for a specific model, mention it only in the title or first line; do not add model-specific theater.

## Quality Check

Before final output, verify:

- the template matches the mode;
- exactly one selected target or design problem exists;
- the handoff does not re-rank the repo;
- the objective or problem ends with observable done criteria or the decision to produce;
- target mode: every fact carries a source, every assumption carries a cheap check, authority files and commands are concrete;
- design mode: hard constraints are separated from preferences, decisions already made and rejected designs are present, decision criteria are ranked, assumptions about the existing system carry a check;
- premium model scope is narrow;
- side effects are explicit and match the desired output type;
- every analysis instruction changes default behavior for this target; none restates root-cause analysis, option tradeoffs, or verification in generic form;
- every candidate, library, or technique named in the handoff was named by the user or appears in a cited artifact; the preparer added none;
- no generic AI preamble or chain-of-thought request exists;
- `$targetify` is recommended only in target mode with no selected target; `$grill-me` only when design inputs are missing;
- the result is usable manually without any model integration.
