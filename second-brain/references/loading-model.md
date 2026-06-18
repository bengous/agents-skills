# Loading model

How a progressive-loading wiki keeps the working context bounded no matter how
large the wiki grows.

## The tiers

1. Metadata you always carry: the agent's own instructions plus this wiki's
   `index.md`. Thin by construction.
2. Eager session context: `index.md` + the recent window of `journal.md`. This is
   the only wiki content loaded automatically at the start of a session.
3. On-demand context: `catalog.md`, every file under `raw/` and `pages/`, and the
   sealed archives under `log/`. None of it loads until a link is followed.

## The loading boundary

The whole design is one boundary: a small fixed eager set, and an unbounded
on-demand set reached by links. The eager set does not grow with the wiki — add a
thousand sources and the thousand catalog lines land in `catalog.md`, which is
on-demand, not in `index.md`. That is the point.

- The index links, it never inlines. One line per item, link to the file. A
  reader takes at most two hops: index -> page, or index -> catalog -> source.
- The journal is a recent window. Old entries are sealed into period archives
  (`log/YYYY-MM.md`) and replaced by a single pointer under Archives.

## Bounds worth holding

- Index: a thin map. If it starts carrying content, move the content to a page
  and leave a link.
- Journal recent window: a few hundred lines / ~25 KB of dated entries before
  sealing — small enough to reload every session without a second thought.

## The caveat that makes or breaks it

Importing a file (an `@import`, an include, an "always read X" rule) does NOT
bound the context: the imported file is loaded whole, every session — exactly the
cost you were trying to avoid. Only reading on demand — following a link when the
content is actually needed — bounds the context. Keep the eager set to the index
and the journal window; everything else must be reached by a link, never imported.

See `maintenance-protocol.md` for the operations that keep this boundary intact,
and `limits.md` for where the model stops scaling.
