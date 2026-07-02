---
name: second-brain
description: "USER-INVOKED ONLY. Invoke only when the user runs $second-brain or explicitly names this skill. Stand up and maintain an agent-maintained markdown wiki that loads progressively and stays bounded: a thin always-loaded index plus a recent-window journal, with the source catalog, sealed period archives, and synthesis pages pulled only on demand. Triggers: the user invokes $second-brain; wants to set up a second brain, knowledge base, agent memory, or LLM wiki; or a memory/log/notes file has grown large and is reloaded in full every session and they want it windowed and bounded. Not for a handful of notes, and not for a million-document corpus (that is a search/RAG problem). Never invoke implicitly."
disable-model-invocation: true
model: opus
effort: medium
allowed-tools: [Read, Write, Edit, Bash, Grep, Glob, AskUserQuestion]
---

# Second Brain

Stand up and maintain a markdown wiki an agent keeps for itself — one that loads
**progressively** and stays **bounded** no matter how large it grows.

## The model in one paragraph

A finite attention budget means you cannot load the whole knowledge base every
session, so split it with a single loading boundary. **Eager** (loaded every
session): a thin `index.md` that maps the wiki, plus the recent window of
`journal.md`. **On demand** (pulled only when a link is followed): the source
`catalog.md`, everything under `raw/` and `pages/`, and the sealed archives under
`log/`. The eager set is small and fixed; it does not grow as the wiki grows,
because the index *links* to content instead of *inlining* it. That boundary is
the whole idea — everything below serves it.

## When to use this

- The user invokes `$second-brain`, or asks to set up a second brain / knowledge
  base / agent memory / LLM wiki.
- A memory, log, or notes file has grown large and is reloaded in full every
  session — the symptom this fixes is "one giant file is eating my context".

When **not** to:

- A handful of notes (two or three files). Plain files are lighter; say so and stop.
- A corpus of millions of documents nobody curated. That is a search / RAG
  problem; this pattern is for a hand-curated base (see `references/limits.md`).

## Structure

```
index.md       eager: thin map, links to pages, names the content dirs
journal.md     eager: recent-window activity log + Archives pointers
catalog.md     on demand: one line per raw/ source
SCHEMA.md      the maintenance contract (generic mechanics + your domain rules)
raw/           on demand: immutable dated captures and long traces
pages/         on demand: maintained synthesis pages
log/           on demand: sealed period archives (log/YYYY-MM.md)
scripts/       check-wiki.sh, archive-journal.sh
```

## Workflow A — Setup (new wiki)

1. Load `references/setup-guide.md`.
2. Scaffold: `scripts/init-wiki.sh <target-dir>`. It refuses a non-empty target
   without `--force`, so it never writes over existing content.
3. Personalize `SCHEMA.md` — keep the generic mechanics, add domain rules under
   the line at the bottom.
4. Wire `scripts/check-wiki.sh` into a hook (portable pre-commit by default; a
   Claude Code Stop hook in addition when relevant). See the setup guide.
5. Run `scripts/check-wiki.sh` — it should print `wiki check OK`.

## Workflow B — Maintenance (existing wiki, or one that has bloated)

1. Diagnose the bloat: which file is loaded eagerly and large? Usually a fat
   index that inlines a catalogue, or a journal that never gets windowed.
2. Thin the index: move any inlined catalogue or content out of `index.md` into
   `catalog.md` or a page, leaving one-line links behind.
3. Window the journal: `scripts/archive-journal.sh` seals the oldest whole months
   into `log/YYYY-MM.md` and leaves a pointer. Re-run is a no-op once bounded.
4. Load `references/maintenance-protocol.md` for the ingest / query / lint cycle
   and the anti-orphan invariant.
5. Run `scripts/check-wiki.sh` and fix any orphan it reports.

## Scripts: run them, do not reimplement them

The scripts are deterministic and bundled so every invocation does not re-derive
them. Execute them; read their output, not their source.

- `init-wiki.sh <dir> [--force]` — scaffold from templates.
- `check-wiki.sh` — anti-orphan + index-integrity lint, plus frontmatter checks
  (`updated:` format as an error, index freshness as a warning) that no-op on
  files without frontmatter. Run from the wiki root (or via its hook). A
  non-zero exit means an orphan, a missing required file, or a malformed date.
- `archive-journal.sh [threshold_bytes]` — seal old months out of the journal.

## Gates

- **Never scaffold over existing content.** `init-wiki.sh` refuses a non-empty
  directory without `--force`; do not reach for `--force` to overwrite a real
  wiki without the user confirming.
- **Never rewrite a sealed archive.** `log/YYYY-MM.md` files are immutable;
  `archive-journal.sh` refuses to touch one that exists. Growth goes into new
  files, never edits of old ones.
- **The index links, never inlines.** If you are about to paste content into
  `index.md`, put it in a page and link instead.

## References — load on demand, not all at once

- `references/loading-model.md` — the tiers and the loading boundary in depth;
  why imports don't bound context and on-demand reads do. Load when deciding what
  goes eager vs on-demand.
- `references/maintenance-protocol.md` — ingest / query / lint, rollover, the
  anti-orphan invariant, freshness. Load when maintaining an existing wiki.
- `references/setup-guide.md` — scaffolding steps and how to wire the check
  (portable vs Claude Code). Load during setup.
- `references/limits.md` — where the model stops scaling (when to shard the index,
  add a search CLI, or — only at very large scale — reach for RAG) and when not to
  use this pattern at all. Load when hitting scale limits.
- `references/scaling-up.md` — the graduated architecture as a case study: full
  typed page contract, registry-driven lint CLI, federated wikis, three-tier
  memory, multi-agent projection. Load when a wiki outgrows the baseline.
