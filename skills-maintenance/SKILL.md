---
name: skills-maintenance
description: "USER-INVOKED ONLY. Use only when the user explicitly invokes $skills-maintenance or explicitly asks to update installed agent skills globally, update project skills, refresh skills-lock files, reconcile dotfiles-managed skills.sh/bootstrap state, or commit/push skill-update maintenance changes. Runs skills.sh updates through a bundled script, then verifies source/live/dotfiles boundaries before any commit."
---

# Skills Maintenance

Update installed skills without making the human restate the whole maintenance
ritual.

This skill creates side effects: installed skills may change, project lock files
may change, and dotfiles-managed skill bootstrap state may need commit/push. Keep
live installs, project files, dotfiles source, and generated artifacts separate.

## Contract

- Use only after explicit human invocation or an explicit request to update
  installed skills.
- Run the bundled script first for deterministic update and status collection:
  `scripts/update-skills.sh`.
- Choose scope from the request and local evidence:
  - `global`: machine-wide installed skills.
  - `project`: the current repo's `skills-lock.json` / project skills.
  - `all`: global plus project when project skill markers exist.
- Never commit broad dirty state. Stage only skill update artifacts, such as
  `skills-lock.json`, related skill config, or the dotfiles bootstrap files.
- Commit and push only when the invocation explicitly asks for persistence, for
  example "commit", "push", "track", or "persist in dotfiles".
- Do not edit live installed skill directories by hand.
- Do not run broad `chezmoi apply`; use targeted paths or the known skills
  bootstrap only.
- Use `$skill-sync` instead when the task is to ship changed skill source from
  `agents-skills` to the live install.

## Workflow

1. Inspect the current repo and dotfiles state:
   ```bash
   git status --short --branch
   git -C ~/dotfiles status --short --branch
   dots status --json
   ```
   If `dots` reports `.codex/config.toml` as volatile-only review/no-op, leave it
   alone.
2. Run the updater with the right scope:
   ```bash
   <skill-dir>/scripts/update-skills.sh --scope global
   ```
   Use `--scope project --project-dir "$PWD"` for a project-only request, or
   `--scope all --project-dir "$PWD"` when both global and project updates are in
   scope.
3. If the request is for this machine's durable global setup, include the
   dotfiles skills bootstrap in that updater invocation:
   ```bash
   B3NGOUS_AGENTS_SKILLS_SOURCE=local B3NGOUS_AGENTS_SKILLS_LOCAL_REPO=~/projects/agents-skills \
   <skill-dir>/scripts/update-skills.sh --scope global --managed-bootstrap
   ```
   This keeps the managed bootstrap path primary while still allowing
   `skills update -g` to refresh the live install after bootstrap.
4. For proof after updates, prefer non-mutating inventory and status checks:
   ```bash
   DISABLE_TELEMETRY=1 bunx skills list -g --json
   git status --short --untracked-files=all
   git -C ~/dotfiles status --short --untracked-files=all
   test -f ~/.agents/.skill-lock.json && sha256sum ~/.agents/.skill-lock.json
   ```
   If the user specifically asks whether the install is current, a second
   `skills update ... -y` is allowed, but treat it as another mutating update
   attempt, not a read-only check.
5. Inspect changed files:
   ```bash
   git status --short --untracked-files=all
   git diff --stat
   git -C ~/dotfiles status --short --untracked-files=all
   git -C ~/dotfiles diff --stat
   ```
6. Commit and push only when the invocation explicitly asked for persistence.
   Split repo commits from dotfiles commits:
   ```bash
   git add <only-skill-maintenance-files>
   git commit -m "Update installed skills"
   git pull --rebase --autostash
   git log --oneline -5
   git push

   git -C ~/dotfiles add <only-dotfiles-skill-maintenance-files>
   git -C ~/dotfiles commit -m "Update installed skills"
   git -C ~/dotfiles pull --rebase --autostash
   git -C ~/dotfiles log --oneline -5
   git -C ~/dotfiles push
   ```
   Use a more specific imperative message when the change is narrower.
7. Final answer must report:
   - update scope and command(s);
   - skills updated or "already up to date";
   - changed lock/config/bootstrap files;
   - commits and push status;
   - live/dotfiles drift left untouched.

## Script

Use:

```bash
<skill-dir>/scripts/update-skills.sh --scope global
<skill-dir>/scripts/update-skills.sh --scope global --managed-bootstrap
<skill-dir>/scripts/update-skills.sh --scope project --project-dir "$PWD"
<skill-dir>/scripts/update-skills.sh --scope all --project-dir "$PWD"
```

For validation without side effects:

```bash
SKILLS_MAINTENANCE_DRY_RUN=1 <skill-dir>/scripts/update-skills.sh --scope all --project-dir "$PWD"
```
