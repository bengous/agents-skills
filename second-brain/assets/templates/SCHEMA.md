# Wiki schema

The maintenance contract for this wiki. Generic by design: add your domain rules
under the line at the bottom, but keep the mechanics above it intact. An agent
reads this file to know how to maintain the wiki.

## Loading boundary

- Eager (every session): `index.md` + the recent window of `journal.md`.
- On demand (follow a link): `catalog.md`, `raw/`, `pages/`, `log/`.
- Keep the eager files small. Useful bounds: a thin index that links rather than
  inlines; a journal recent window on the order of a few hundred lines / ~25 KB
  before sealing.

## Page frontmatter

Maintained files (`index.md`, `journal.md`, `catalog.md`, `pages/*`, `raw/*`)
start with a minimal YAML frontmatter block:

```yaml
---
type: page           # this file's role (index, journal, catalog, capture, page...)
updated: YYYY-MM-DD  # date of the last substantive edit — bump it when you edit
---
```

- `updated:` drives the lint: a malformed date fails the check, and an
  `index.md` older than the newest page draws a warning — a stale map misroutes
  every future read.
- Frontmatter is opt-in per file: a file without it is simply not checked, so a
  wiki scaffolded before this contract keeps passing. New scaffolds carry it
  from birth.
- At larger scale the contract typically grows a `sources:` list naming the
  canonical files a page synthesizes, so claims can be verified before acting.

## Operations: ingest / query / lint

- Ingest: a new durable fact -> append a `journal.md` entry. A long capture or
  trace -> a `raw/` file plus a one-line entry in `catalog.md`. A durable
  synthesis -> a `pages/` file linked from `index.md`.
- Query: start at `index.md`, follow one link to the relevant page or source.
  Two hops max. If you need three, the index or a page is too coarse.
- Lint: run `scripts/check-wiki.sh`. It fails if any content file is unreachable
  from its index — the invariant that keeps the wiki navigable — or if a
  frontmatter `updated:` is malformed, and warns when `index.md` is older than
  the newest page.

## Invariants

- Anti-orphan: every `pages/*.md` is linked from `index.md`; every `raw/*.md` is
  listed in `catalog.md`; every `log/*.md` is listed in `journal.md`. A file no
  index points at is invisible to a future agent — it may as well not exist.
- The index links, never inlines. The catalog lives outside `index.md` so the
  eager context does not grow with the source count.
- Sealed archives are immutable. Roll over by sealing snapshots; never rewrite or
  re-summarize a sealed month in place.

## Rollover

When `journal.md` passes the seal threshold, run `scripts/archive-journal.sh`.
It seals the oldest whole months into `log/YYYY-MM.md` and refreshes the Archives
list. Re-running when nothing is over the threshold is a no-op.

## Domain rules

<!-- Add project-specific conventions, required files, or naming rules here.
     The mechanics above stay generic so the lint script stays portable. -->
