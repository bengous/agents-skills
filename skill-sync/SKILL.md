---
name: skill-sync
description: "USER-INVOKED ONLY. Use only when Augustin explicitly invokes $skill-sync or asks to commit, push, and deploy agent skill changes live from agents-skills. Commits scoped source, pushes the default branch, publishes exact managed skill names through the shared post-push gate, and verifies live parity without touching project-pinned copies."
---

# Skill Sync

Ship a changed skill from source to live install.

This skill creates side effects: validation, commits, pushes, and named live
installs. Keep source, global live state, project-pinned copies, and dotfiles
ownership separate.

## Contract

- Work from the skill source repo, usually `~/projects/agents-skills`.
- Commit only the skill/code/docs touched for the requested change.
- Do not commit generated caches, local workflow roots, build output, or unrelated
  dirty files.
- Push only after `git log --oneline` and a rebase pull.
- Deploy live only after push through `scripts/publish-live <exact-name>...`.
- Never hand-edit `~/.agents/skills` or update project-local copies implicitly.
- Never hide unrelated dotfiles drift by staging it.

## Workflow

1. Inspect source status:
   ```bash
   git status --short
   git diff --stat
   ```
2. Identify the changed skill(s) and their package/runtime, if any.
3. Run relevant validation:
   - `cargo run --quiet -p skills-tools -- validate frontmatter <skill>/SKILL.md`
   - package tests/typecheck/lint when the skill has code, such as `uv run pytest`,
     `uv run ruff check .`, and `uv run basedpyright` for `intent-to-workflow`.
4. Stage and commit only the scoped source files.
5. Push source:
   ```bash
   git pull --rebase --autostash
   git log --oneline -5
   git push
   ```
6. Confirm the source is clean, on the pushed remote default branch:
   ```bash
   git status --short
   git branch --show-current
   git rev-list --left-right --count HEAD...@{u}
   ```
7. Publish only the changed managed skills by exact name:
   ```bash
   scripts/publish-live <skill>...
   ```
   The publisher rejects dirty, detached, divergent, unpushed, non-default-branch,
   unmanaged, or colliding inputs. It refreshes the shared store, verifies
   source/live parity, and reports divergent project-local copies without changing
   them.
   For packaged CLIs, also reinstall the CLI from source, for example:
   ```bash
   uv tool install ~/projects/agents-skills/intent-to-workflow --force --reinstall
   ```
8. Inspect dotfiles status without resolving unrelated drift:
   ```bash
   dots status --json
   ```
9. Final answer must report:
    - source commit hash and push status;
    - validation commands;
    - exact names passed to the live publisher;
    - parity proof;
    - dotfiles status and whether any dotfiles commit was needed;
    - any unrelated dirty files deliberately left untouched.

## Stop Conditions

Stop and ask before:

- changing the desired manifest or publisher policy in dotfiles;
- committing unrelated dirty files;
- resolving conflicts in files the user appears to be editing;
- publishing packages, tags, or releases.
