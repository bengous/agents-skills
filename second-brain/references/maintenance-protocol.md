# Maintenance protocol

The operations that keep a progressive-loading wiki navigable and bounded. Three
verbs — ingest, query, lint — plus rollover.

## Ingest

Route new material by how durable and how large it is:

- A durable fact or event -> append one `## YYYY-MM-DD` entry to `journal.md`.
  Keep it short; link out rather than paste.
- A long capture, transcript, or trace -> a dated file under `raw/`, plus a
  one-line entry in `catalog.md`. The catalog line is what makes the file
  reachable.
- A durable synthesis that will be read again -> a file under `pages/`, linked
  from `index.md`.

Write the synthesis where it will be looked for, not where it was produced.

## Query

Start at `index.md` and follow one link. Two hops max: index -> page, or
index -> catalog -> source. If you need a third hop, the index or a page is too
coarse — add a more specific link.

## Lint

Run `scripts/check-wiki.sh`. It enforces the one invariant that keeps the wiki
usable:

- Anti-orphan: every `pages/*.md` is linked from `index.md`, every `raw/*.md` is
  in `catalog.md`, every `log/*.md` is in `journal.md`. A file no index points at
  is invisible — a future agent will never load it, so it may as well not exist.

Run the check after any change that adds or moves a content file, and wire it
into a hook (see `setup-guide.md`) so an orphan can never be committed.

## Rollover

When the journal's dated entries pass the seal threshold, run
`scripts/archive-journal.sh`. It cuts the journal at entry boundaries, seals the
oldest whole months into `log/YYYY-MM.md`, and refreshes the Archives list.

The rule that matters: seal immutable snapshots, never re-summarize in place. An
archived month is frozen — you read it, you do not rewrite it. Growth goes into
new files, not edits of old ones. The script enforces this: it refuses to write a
`log/YYYY-MM.md` that already exists.

## Freshness

The journal and pages drift; the archives do not (they are frozen by date). When
a page's synthesis is contradicted by a newer entry, update the page and add a
journal entry recording the change. Staleness lives in the maintained files
(`index.md`, `pages/`), so that is where to look when something seems wrong —
never in a sealed archive, which records what was true on its date.
