---
name: batch-update
description: "USER-INVOKED ONLY. Use only when the user explicitly invokes $batch-update or explicitly names this skill to update a repository. Create an isolated git-wt worktree, inventory repo update surfaces, research and apply dependency/tooling update batches with native ecosystem tools, validate, commit each successful batch, and document deferrals. Never invoke implicitly."
---

# Batch Update

Manual dependency and tooling maintenance workflow for a repository.

This skill is executing by default. It creates local side effects: worktree, branch, package-manager updates, validations, commits, and a local run artifact. It must not push, open PRs, send messages, or publish anything.

## Hard Contract

- Invocation is `$batch-update`; V1 has no flags or modes.
- Always create an isolated worktree with `git-wt`.
- Worktree dir: `batch-update-YYYY-MM-DD`.
- Branch: `batch-update/YYYY-MM-DD`.
- Run `git fetch`, then base the branch on `origin/<default-branch>`.
- Do not run `git fetch --prune` by default.
- Do not touch, stash, pull, or clean the original worktree.
- Research every batch before modifying files, even patch updates.
- Apply version updates with native ecosystem tools only.
- Never manually edit lockfiles.
- Do not manually edit dependency version declarations unless the native tool cannot perform the operation and the user explicitly approves.
- Manual edits are allowed only for minimal config/code fixes required by an applied update.
- Commit every successful batch immediately after validation.
- If a batch cannot be fixed quickly, revert that batch and document why.
- Never leave successful applied changes uncommitted.
- Never push or open a PR.

If any hard contract conflicts with repo instructions, stop and report the conflict.

## Workflow

1. Load `references/workflow.md`.
2. Create the isolated worktree and continue all work there.
3. Create the local run artifact from `references/run-artifact-template.md`.
4. Inventory update surfaces across the repo.
5. Group updates into functional batches.
6. Classify each batch using `references/risk-classification.md`.
7. For each batch, use `references/research-brief.md`; delegate read-only research to subagents when available, otherwise research inline.
8. Apply reasonable batches with the native ecosystem commands from `references/ecosystems.md`.
9. Validate, commit, or revert+document each batch.
10. Run a lightweight post-update audit and reconcile every remaining outdated item.

Load reference files only when needed. Do not read all references upfront unless the run requires it.

## Stop And Ask

Ask or stop before:

- using raw `git worktree add` because `git-wt` is unavailable;
- reusing an existing dirty batch-update worktree;
- creating from an ambiguous or unavailable base;
- applying a major, breaking, runtime, package-manager, CI, Docker, or broad migration batch;
- manually editing dependency versions because the native tool cannot express the update;
- continuing after a blocked batch when independence is unclear.

## Final Answer

Report:

- worktree path and branch;
- commits created;
- validations run;
- batches applied;
- batches reverted, deferred, blocked, or needing human decision;
- remaining outdated items after the post-update audit;
- run artifact path;
- confirmation that no push/PR happened.
