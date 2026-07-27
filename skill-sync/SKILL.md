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
- A skill that was never published has no store yet, and `publish-live` alone can
  never create one. Run `## First publication` before step 7.
- Never hand-edit `~/.agents/skills` or update project-local copies implicitly.
- `claude: true` in the desired manifest only drives the `~/.claude/skills/<name>`
  symlink. The shared store `~/.agents/skills` is what Codex reads natively, so a
  `claude: false` skill stays available to Codex. `~/.codex/skills` is a read-only
  legacy surface: never write there.
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
    - any unrelated dirty files deliberately left untouched;
    - when `## First publication` was taken, also: the desired-manifest entry added,
      the dotfiles commit covering manifest plus test, and the `reconcile` actions
      applied.

## First publication

Take this path when `~/.agents/skills/<name>` does not exist yet: a brand-new
skill, or one migrated from another repo. Steps 1 to 6 of `## Workflow` run first,
unchanged. The push is a hard prerequisite, because installing the store pulls the
skill from GitHub, not from the local checkout.

1. Before committing, add the skill's row to the `## Skills` table of `README.md`
   (`General` or `Claude Code (-cc)` per its `## Naming` section), then validate:
   ```bash
   cargo run --quiet -p skills-tools -- validate frontmatter <name>/SKILL.md
   ```
   Commit and push to `origin/master` as usual.
2. Register the skill in the dotfiles desired manifest:
   - edit the chezmoi source `~/dotfiles/dot_config/agent-skills/desired.jsonl.tmpl`,
     never the rendered `~/.config/agent-skills/desired.jsonl`;
   - line format `{"repo":"bengous/agents-skills","skill":"<name>","claude":true}`,
     inserted in alphabetical order inside the `bengous/agents-skills` block;
   - render it: `chezmoi apply ~/.config/agent-skills/desired.jsonl`, targeted path
     only, never a broad apply;
   - bump the hardcoded skill count in
     `~/dotfiles/dot_local/bin/reconcile-global-skills-lib/reconcile.test.ts`: three
     occurrences, the test title, `toHaveLength(N)`, and the `Set` size;
   - `bun test dot_local/bin/reconcile-global-skills-lib/` must pass, then commit
     manifest and test **together**, or the `~/dotfiles` pre-push gate blocks.
3. Create the store, the step `publish-live` cannot do:
   ```bash
   reconcile-global-skills            # dry run
   reconcile-global-skills --apply
   ```
   The dry run must announce exactly `install-store <name>`,
   `create-claude-link <name>`, and `0 conflict(s)`. Any extra action: stop and ask.
4. Publish normally with `scripts/publish-live <name>`. At this point only the
   refresh, the parity proof, and the project-copy report remain.
5. Verify both surfaces: `~/.agents/skills/<name>`, read by Codex, and
   `~/.claude/skills/<name>`, a relative symlink into the store. A final
   `reconcile-global-skills` must report `0 action(s)`.
6. Migration only, when the skill came from another repo: after the parity proof and
   never before, check that
   `diff -qr <repo>/.agents/skills/<name> ~/.agents/skills/<name>` is empty, then
   delete the origin copy (`git rm -r` the tracked directory, plus the
   `.claude/skills/<name>` symlink if it exists) and commit it in that repo.

Error signature and its cause:
`Refresh of "<name>" is forbidden: its store is not recorded in the registry`
means step 3 was skipped.

## Stop Conditions

Stop and ask before:

- changing the publisher policy in dotfiles, or modifying or removing an existing
  desired-manifest entry: hard stop, always. Adding the entry for the skill being
  first-published is part of `## First publication` and only needs one explicit
  human go-ahead;
- committing unrelated dirty files;
- resolving conflicts in files the user appears to be editing;
- publishing packages, tags, or releases.
