# Setup guide

Scaffolding a new progressive-loading wiki and wiring its check.

## Scaffold

Run the bundled initializer from the skill's `scripts/`:

```bash
scripts/init-wiki.sh <target-dir>
```

It refuses a non-empty target unless you pass `--force`, so it never writes over
existing content. It creates the structure, copies the templates, and installs
the two maintenance scripts into `<target>/scripts/` so the wiki is
self-contained and portable.

## What each stub is for

- `index.md` — the always-loaded thin map. Links to pages and names the content
  directories. Never inline content here.
- `journal.md` — the recent-window activity log, plus the Archives section that
  `archive-journal.sh` manages.
- `catalog.md` — the index of `raw/` sources, kept out of `index.md` so the eager
  context stays flat as sources accumulate.
- `SCHEMA.md` — the maintenance contract. The mechanics are generic; add your
  domain rules under the line at the bottom.
- `templates/entry.md` — the shape of a dated `raw/` capture.
- `raw/`, `pages/`, `log/` — on-demand content: captures, synthesis, sealed
  archives.

After scaffolding, personalize `SCHEMA.md` with any domain rules, then make the
first real `index.md` and `journal.md` edits.

## Wire the check

`check-wiki.sh` should run automatically so an orphan can never land. Keep the
wiki portable by default — a wiki is often read by more than one agent or tool,
so prefer a mechanism that does not assume a particular runtime.

- Portable (recommended): a git `pre-commit` hook.

  ```bash
  printf '#!/usr/bin/env bash\nexec scripts/check-wiki.sh\n' > .git/hooks/pre-commit
  chmod +x .git/hooks/pre-commit
  ```

  If the repo uses a hook manager (lefthook, pre-commit, husky), add
  `scripts/check-wiki.sh` as a hook command there instead.

- Claude Code: a Stop hook in `.claude/settings.json` that runs
  `scripts/check-wiki.sh` when the wiki changed in a turn. Use it *in addition to*
  the portable hook when the wiki lives in a Claude Code project, not instead of
  it — the portable hook is what keeps the wiki honest for every other tool.

Whichever you wire, the contract is the same: a failing check blocks the commit
(or the stop), and the only fix is to catalog or link the orphan.
