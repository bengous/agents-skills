---
name: slice-runner
description: USER-INVOKED ONLY. Use when the user explicitly invokes $slice-runner or asks Codex to execute an already-approved multi-slice implementation plan with native Codex subagents. Requires runnable quality gates. Do not use for planning; use intent-to-workflow first when the plan is not yet agreed.
---

# Slice Runner

Execute an approved plan by bounded Codex-native slices. Do not design the plan here.

## Contract

- Treat the main Codex thread as architect, integrator, QA, and committer.
- Use native subagents only: `spawn_agent`, `wait_agent`, `send_input`, and `close_agent`.
- Never call `codex exec`, agents-bridge, shell wrappers, or external agent CLIs for the slice work.
- Execute one slice at a time unless the plan explicitly marks write scopes as independent.
- Keep worker prompts self-contained and strict. Do not let workers choose public APIs, broad architecture, or scope.
- Run gates yourself. A worker's "green" claim is not evidence.
- Commit locally only when the user or plan asks for commits. Never push without explicit user request.

## Preflight

1. Confirm the plan is already agreed and split into slices with exit gates.
2. Run the baseline gate before the first slice. If baseline fails, stop and report.
3. Inspect `git status --short --untracked-files=all`; preserve unrelated dirty work.
4. Use the smallest autonomous first slice as a smoke test.
5. If a cross-cutting prerequisite appears, insert a prerequisite slice instead of hiding it in the current one.

## Slice Loop

For each slice:

1. Write a short task checklist in the main thread.
2. Spawn one `worker` agent with a clean-context prompt.
3. Wait for completion only when the result blocks the next step.
4. Review the returned changed paths, `git diff --stat`, and only the risky files or contracts named in the prompt.
5. Run the slice gate locally.
6. If the gate fails:
   - code failure: `send_input` the exact error to the same worker once;
   - environment/tooling failure: fix or report it in the main thread.
7. After two failed attempts on the same slice, stop and report.
8. Commit the scoped slice only when commits are in scope.

## Worker Prompt Shape

Include these sections, in order:

- Context: stack, repo instructions to read, and exact files/patterns to imitate.
- Objective: one sentence; say "zero behavior change" for refactors.
- Scope: owned files/modules and explicit out-of-scope files.
- Interfaces: exact names, signatures, data shapes, and public behavior.
- Tricky behavior: semantics to preserve, edge cases, and suggested implementation.
- Tests: exact files/cases and commands; never remove existing tests.
- Rules: no commits, no new dependencies, no unrelated edits, preserve dirty work.
- Definition of done: exact gate command and a short final summary with changed files and non-obvious choices.

## Native Agent Defaults

- Use `agent_type: "worker"` for implementation slices.
- Use `agent_type: "explorer"` only for read-only repo questions that unblock a slice.
- Omit model overrides unless the user explicitly asks or the slice clearly needs a different model.
- Use `fork_context: false` unless exact conversation context is required.
- For parallel work, assign disjoint write sets and say workers are not alone in the codebase.
- Close completed agents after their output is integrated.

## Failure Rules

- Baseline red before slice 1: stop.
- Worker drifts from scope: resume once with the exact contract mismatch.
- Gate red after one correction attempt: diagnose whether code or environment is at fault.
- Two failures on the same slice: stop and report the blocker, changed files, and last gate output.
- Dirty unrelated files appear: classify them; never stage or revert them unless asked.
