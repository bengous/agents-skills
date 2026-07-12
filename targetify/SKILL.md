---
name: targetify
disable-model-invocation: true
description: "USER-INVOKED ONLY. Use only when the user explicitly invokes $targetify or asks where to spend scarce/premium/frontier/super-model attention in a repo. Read-only. Do not use for implementation, full audits, premium handoffs, or launching premium model runs."
---

# Targetify

Find where expensive model attention is worth spending.

## Contract

You are upstream targeting, not the premium model run.

- User-invoked only: require `$targetify`, `targetify`, or an explicit request to target scarce/premium/frontier/super-model attention.
- Read-only by default: inspect files, git history, tests, logs, docs, and local reports.
- Do not edit code, create branches, stage, commit, push, open PRs, install, delete, apply live config, or launch premium model runs.
- Convert vague intent into one targeting objective before scoring.
- Separate verified evidence from inference.
- Recommend `do not spend premium tokens yet` when evidence does not justify it.

## Gate

Skip full targeting when the lazy answer is obvious:

- the repo or scope is tiny enough for a normal pass;
- the user already named one clear target and no ranking is needed;
- the work is mechanical, repetitive, or well-covered by existing tests/tools;
- local evidence is too weak to rank targets responsibly;
- cheaper read-only inspection can answer the next decision.

In those cases, say what to do instead and stop.

## Workflow

1. Read repo instructions and lightweight project shape: `AGENTS.md`, README, manifests, test config, and `git status --short`.
2. State the targeting objective in one sentence. Ask only if the objective cannot be inferred safely.
3. Choose the target type: tech debt, architecture, bugs, security, performance, docs drift, test gaps, product value, portfolio/demo value, or a task-specific mix.
4. Gather local evidence with cheap commands first: `git log --stat`, `git blame` only when useful, `rg`, import/caller searches, test discovery, TODO/FIXME/HACK search, duplicate-pattern search, recent failure logs, docs/code comparisons.
5. Score only targets backed by evidence. Prefer 3-7 ranked targets; fewer is fine.
6. Name skipped areas and why they should not receive premium attention now.
7. If the recommendation is to spend premium tokens, end with the Handoff Seed block from the Output section for a future `$premium-handoff` invocation; otherwise name the evidence that would change the recommendation and stop.

Use `swarm-research` only when the evidence pass clearly splits into independent read-only questions. Own the final ranking yourself.

## Scoring

Default score:

```text
score = impact (1-5) x opportunity (1-5), confidence reported as high|medium|low
```

Rank by score; when scores are close, higher confidence wins. For any low-confidence target, name the cheap check that would raise its confidence. Do not pretend precision. Adapt what impact and opportunity mean to the target type (e.g. exploitability and blast radius for security, hot-path likelihood for performance) and state the factors you used.

If a factor is mostly inference, keep confidence low even when impact seems high.

## Evidence Signals

Prefer evidence available locally:

- churn: recent edits, repeated fixes, noisy ownership;
- prior attempts: reverted commits, abandoned branches or migrations near the target;
- centrality: imports, callers, public APIs, fan-in/fan-out;
- complexity: large branches, many modes, duplicated choreography;
- risk: auth, data loss, payments, migrations, concurrency, external I/O;
- weak tests: missing coverage near critical paths, skipped/flaky tests, broad mocks;
- failures: recent CI/test/log errors with reproducible context;
- TODOs: TODO/FIXME/HACK around live behavior, not stale wishlists;
- duplication: repeated patterns that hide inconsistent behavior;
- docs drift: docs or agent guidance disagreeing with current code;
- product signal: first-run UX, demos, onboarding, high-visibility workflows.

Do not score generated, vendored, build, cache, or local-install files as targets unless the source-of-truth boundary itself is the problem.

## Output

Default shape:

```markdown
## Recommendation
<one short answer, including whether premium tokens are worth spending yet>

## Ranked Targets
1. <target>
   - Why it matters: <impact>
   - Evidence: <verified files, commands, counts, failures, or paths>
   - Inference: <what you believe beyond direct evidence>
   - Score: impact <1-5>, opportunity <1-5>, confidence <low|medium|high>
   - Premium task: <what the premium model should think about>

## Skip
- <area>: <why not premium-model-worthy now>

## Next Handoff
<omit this whole section when recommending against premium spend>
Use `$premium-handoff` after selecting a target.
Handoff Seed:
- Target:
- Objective and done criteria:
- Expected premium output: <diagnosis | options | plan | implementation | review>
- Authority files: <path: why>
- Verified evidence: <fact (source: file/command/commit)>
- Known traps / ruled out: <approach: why>
- Constraints:
- Do not include:
```

Keep the report compact. Do not produce a full premium-model prompt, implementation plan, or audit transcript.
