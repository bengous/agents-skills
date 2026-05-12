# Batch Update Workflow

## Setup

1. Inspect repo instructions before changing anything.
2. Detect default branch:
   - prefer `git symbolic-ref refs/remotes/origin/HEAD`;
   - fallback to the current upstream default only if clear.
3. Run `git fetch`.
4. Create the worktree:

```sh
git-wt -b origin/<default-branch> batch-update-YYYY-MM-DD batch-update/YYYY-MM-DD
```

If the directory or branch exists, choose a simple suffix such as `-2` and record it. If an existing worktree is dirty or branch history is ambiguous, stop.

After creation, switch all commands to the new worktree path. Do not modify the original worktree.

## Inventory

Inventory explicit update surfaces. Use only surfaces present in the repo.

- JavaScript package managers: `bun`, npm, pnpm, yarn.
- Go modules.
- Python project/dependency files.
- Cargo projects.
- `mise` and other tool/runtime version declarations.
- package-manager declarations such as `packageManager`.
- CI and Docker surfaces, detect and report only in V1.

Ignore Renovate and Dependabot. Do not search for, add, or modify their config.

## Batch Planning

Group by functional family:

- coupled libraries (`react`/`react-dom`, `remotion/*`, `tailwind/*`, `playwright/*`);
- app/framework stack;
- build/test/lint/format tooling;
- runtime/package-manager/tool versions;
- isolated language module or workspace.

Prefer smaller batches when validation boundaries are clearer.

For tools that distinguish `Update` and `Latest`:

- default to `Update`;
- keep `Latest` visible;
- use `Latest` directly only when research says it is non-breaking and useful;
- gate major/breaking `Latest` updates as separate human-decision batches.

## Execution Loop

For each batch:

1. Research first.
2. Apply updates with native tools.
3. Inspect diff.
4. Run targeted validation.
5. If validation fails and the cause is clear, attempt one minimal config/code fix.
6. If still failing or the fix becomes broad, revert the batch and document it.
7. If validation passes, stage only batch-related files and commit immediately.

Continue after a blocked batch only when independence is clear. If lockfile, runtime, package-manager, or framework state is uncertain, stop.

## Post-Update Audit

At the end:

- rerun relevant inventory commands;
- reconcile every remaining update with `done`, `defer`, `blocked`, or `needs-human-decision`;
- run an obvious full repo validation only if it is discoverable and reasonable;
- verify `git status`;
- list commits created;
- confirm no push or PR happened.
