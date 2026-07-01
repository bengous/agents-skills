---
name: premium-handoff
disable-model-invocation: true
description: "USER-INVOKED ONLY. Use only when the user explicitly invokes $premium-handoff or asks to turn a selected target, targetify seed, evidence summary, or premium-model target into a compact handoff prompt/artifact for a scarce frontier model. Do not target the whole repo, implement, or launch the premium model."
---

# Premium Handoff

Turn one selected target into a compact briefing for an expensive model.

## Contract

You are preparing the premium-model run, not running it.

- User-invoked only: require `$premium-handoff`, `premium-handoff`, or an explicit request for a premium/super/frontier model handoff.
- Accept either a `targetify` Handoff Seed or a manually provided target, objective, and evidence summary.
- Do not decide where premium attention should go across the repo; use `targetify` for that.
- Do not edit code, create branches, stage, commit, push, open PRs, install dependencies, apply live config, or implement.
- Produce only a handoff prompt or, when requested/needed, a handoff artifact.
- Do not launch any premium model.
- Do not include a generic "you are an AI" preamble or ask for chain of thought.

## Gate

Before writing a handoff, confirm there is one selected target.

- If no target is selected, ask for the target or recommend `$targetify` first.
- If evidence is too weak, say premium handoff is not justified yet and name the missing evidence.
- If a `targetify` report or seed exists, consume it without redoing the full targeting pass.
- If the target is mechanical or low-risk enough for a normal model, recommend not spending premium tokens.

Ask only for blockers: missing target, missing objective, missing evidence, or unclear side-effect boundary.

## Input Handling

For a `targetify` seed, preserve its target, objective, authority files, constraints, and "do not include" notes. Verify only obvious path existence or stale assumptions when cheap.

For manual input, extract:

- selected target;
- objective for the premium model;
- verified evidence: files, commands, failures, logs, test gaps, docs drift;
- assumptions/inferences;
- authority files to read first;
- side-effect boundaries;
- desired output: diagnosis, options, implementation plan, review, or validation strategy.

Do not pad missing evidence with summaries. Prefer paths and commands over prose.

## Handoff Rules

The premium prompt must guide the model to:

- focus on the selected target, not the whole repo;
- read only listed authority files first;
- verify critical assumptions before recommendations;
- look for root causes and recurring patterns, not only local point fixes;
- distinguish point fix, class-of-issues fix, and architectural change;
- propose competing options when the change is architectural, security-sensitive, or high-risk;
- respect allowed and forbidden side effects;
- pause when evidence is missing, risk is high, or scope must expand;
- include validation expectations or commands when relevant.

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
<one outcome-focused paragraph>

## Read First
- `<path>`: <why this file is authoritative>
- `<command>`: <what it proves>

## Verified Evidence
- <fact with file/command/log reference>

## Assumptions To Verify
- <assumption and how to check it>

## Scope
Focus on:
- <target boundary>

Do not spend time on:
- <explicit non-targets>

Allowed side effects:
- <usually read-only recommendations unless user explicitly allowed more>

Forbidden side effects:
- <edits, branches, commits, pushes, installs, PRs, external systems unless approved>

## Analysis Instructions
- Find root causes and recurring patterns, not only local symptoms.
- Separate point fix, class-of-issues fix, and architectural change.
- For high-risk or architectural changes, propose 2-3 competing options with tradeoffs.
- Call out evidence gaps and pause if they block a responsible recommendation.

## Expected Output
- Recommendation:
- Evidence used:
- Options or fix classes:
- Validation expectations:
- Stop/pause conditions:
```

Trim unused sections. If the user asked for a specific model, mention it only in the title or first line; do not add model-specific theater.

## Quality Check

Before final output, verify:

- exactly one selected target exists;
- the handoff does not re-rank the repo;
- verified evidence and assumptions are separated;
- authority files and commands are concrete;
- premium model scope is narrow;
- side effects are explicit;
- no generic AI preamble or chain-of-thought request exists;
- `$targetify` is recommended only when no target has been selected;
- the result is usable manually without any model integration.
