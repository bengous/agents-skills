---
name: pr-review-html
description: "USER-INVOKED ONLY. Use only when the user explicitly invokes $pr-review-html or explicitly names this skill. Create a self-contained HTML artifact for reviewing a pull request, patch, or local diff with real diff excerpts, inline margin annotations, severity-colored findings, reviewer handoff context, and explanations for unfamiliar code paths such as streaming, backpressure, concurrency, retry, cancellation, or state-machine logic. Never invoke implicitly."
---

# PR Review HTML

## Overview

Produce a browser-readable review artifact that combines real diff excerpts, inline margin annotations, severity-coded findings, and a focused concept explanation for the code paths the user is least comfortable reviewing.

The HTML artifact is the human-facing deliverable. Keep source evidence, commands, and uncertainty visible so the artifact is reviewable rather than decorative.

## Workflow

1. Identify the PR source: GitHub PR, local branch, patch file, or provided diff. If the target is ambiguous, ask for the PR number, branch, or diff.
2. Collect the actual diff with the best available source, for example `gh pr diff <id>`, `git diff <base>...<head>`, or the provided patch. Do not recreate the diff from memory.
3. Inspect the surrounding code needed to understand the changed behavior. Follow imports, call sites, tests, and ownership boundaries before writing findings.
4. If the user names a focus area, make it the review lens. For streaming, backpressure, cancellation, retries, async iteration, or queueing, load `references/review-lenses.md`.
5. Run only relevant validation commands that fit the repo and review scope. If validation is skipped, state why in the artifact.
6. Build a single `.html` file from `assets/pr-review-artifact-template.html` or an equivalent self-contained structure.
7. Open or sanity-check the artifact when feasible. For complex layouts, use browser automation or a screenshot check if available.

## Artifact Contract

- Save a self-contained HTML file with inline CSS and optional minimal inline JavaScript. Do not require a build step, CDN, external assets, or network access.
- Render the actual diff text, preserving file names, hunk headers, additions, deletions, and line numbers where available.
- Add margin annotations beside the relevant diff lines. Each annotation must link to a finding or explanatory note.
- Color-code review items by severity: `blocking`, `major`, `minor`, `nit`, and `positive`. Use severity for impact, not confidence.
- Include a compact concept panel for the focus area: what the code path does, what changed, why the risky part is subtle, and how backpressure or control flow is supposed to behave.
- Include evidence: commands run, tests inspected or executed, docs/source consulted, and explicit unknowns.
- Keep the artifact useful on a laptop screen: sticky file index, jump links, collapsible low-risk files, and readable code blocks.
- Do not hide low confidence. Mark uncertain findings as "needs confirmation" and explain the missing evidence.

## Review Rules

- Findings first: a reader should see blockers and major risks before general description.
- Anchor every finding to a changed line, nearby unchanged context, or a named missing test.
- Prefer one precise finding over broad commentary. State impact and concrete fix direction.
- Separate "what changed" from "what may break". The first is descriptive; the second is review judgment.
- Use the user's focus area to prioritize attention, but still mention unrelated blockers if found.
- Avoid inventing PR metadata, author intent, test results, or production impact. If unavailable, label it unavailable.
- Do not send, publish, or attach the artifact externally unless the user explicitly asks.

## Output Path

If the user provides a path, use it. Otherwise prefer an existing ignored artifact/report directory. If none is obvious, create `pr-review-<pr-or-branch>.html` in the current worktree and report that it is a local artifact. Do not commit it unless the user asks.

## Template Use

Use `assets/pr-review-artifact-template.html` as a structural starting point when it fits. Replace all placeholder content, duplicate file and annotation blocks as needed, and delete unused example sections. It is acceptable to write a custom artifact when the diff shape needs a different layout.

## Final Response

Report the artifact path, highest-severity findings count, validation run, and any important uncertainty. Do not paste the full HTML unless the user asks.
