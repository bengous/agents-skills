---
name: skill-sync
description: "USER-INVOKED ONLY. Use only when Augustin explicitly invokes $skill-sync or explicitly asks to commit, push, and deploy an agent skill change live through the agents-skills repo plus dotfiles-managed skills.sh/bootstrap path. Ensures source, git, live install, and relevant dotfiles state are verified without committing unrelated artifacts."
---

# Skill Sync

Ship a changed skill from source to live install.

This skill creates side effects: validation, commits, pushes, live install, and
possibly scoped dotfiles commits. Keep repo source, live install, and dotfiles
ownership separate.

## Contract

- Work from the skill source repo, usually `~/projects/agents-skills`.
- Commit only the skill/code/docs touched for the requested change.
- Do not commit generated caches, local workflow roots, build output, or unrelated
  dirty files.
- Push only after `git log --oneline` and a rebase pull.
- Deploy live through the managed path, not by hand-editing `~/.agents/skills`.
- If dotfiles changes are required, stage only the relevant dotfiles files.
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
6. Inspect dotfiles skill management before deploying:
   ```bash
   rg -n "skills|agents-skills|intent-to-workflow|skills-lock" ~/dotfiles
   git -C ~/dotfiles status --short --untracked-files=all
   ```
7. Deploy using the managed path. Prefer the dotfiles bootstrap when it is known
   to be current:
   ```bash
   B3NGOUS_AGENTS_SKILLS_SOURCE=local \
   B3NGOUS_AGENTS_SKILLS_LOCAL_REPO=~/projects/agents-skills \
   ~/dotfiles/.chezmoiscripts/run_after_install-global-skills.sh
   ```
   If the full bootstrap hangs or is too broad, use the same underlying surfaces
   narrowly:
   ```bash
   bunx skills add ~/projects/agents-skills --skill <skill> -g -a codex -y
   ```
   For packaged CLIs, also reinstall the CLI from source, for example:
   ```bash
   uv tool install ~/projects/agents-skills/intent-to-workflow --force --reinstall
   ```
8. Verify live equals source:
   ```bash
   diff -qr ~/projects/agents-skills/<skill> ~/.agents/skills/<skill>
   ```
   Ignore only expected caches/build output such as `__pycache__`, `.venv`,
   `.pytest_cache`, `.ruff_cache`, or `dist`.
9. If dotfiles-managed files or locks changed, inspect, validate, commit, and
   push only those relevant dotfiles files. If no relevant dotfiles change exists,
   say so explicitly.
10. Final answer must report:
    - source commit hash and push status;
    - validation commands;
    - live deploy commands;
    - parity proof;
    - dotfiles status and whether any dotfiles commit was needed;
    - any unrelated dirty files deliberately left untouched.

## Stop Conditions

Stop and ask before:

- editing dotfiles when the source-of-truth path is unclear;
- committing unrelated dirty files;
- resolving conflicts in files the user appears to be editing;
- publishing packages, tags, or releases.
