# Limits

Where the index-first model stops scaling, and when not to use it at all.

## Where index-first stops scaling

This pattern is for a knowledge base curated by hand — tens to a few hundred
pages. It scales by keeping the eager set flat while the on-demand set grows. The
ceilings:

- A single `index.md` stays comfortable into the low hundreds of pages. Past
  that, the index itself stops being thin. Shard it — split into sub-indexes by
  area, each linked from the top index — before reaching for anything heavier.
- When even sharded indexes get unwieldy, add a search CLI over the markdown (a
  ripgrep wrapper, or a dedicated markdown query tool) so an agent finds files by
  query instead of by browsing a map. Still flat files and links — no new storage
  system.
- Reach for vector search / RAG only at very large scale, when neither a sharded
  index nor a search CLI can surface the right file. Embeddings add a build step,
  a store, and retrieval noise; index-first beats them for curated bases at this
  size, which is why this skill stops at flat markdown.

## Past the limits

Hitting these ceilings does not abandon the pattern — it graduates it. See
`scaling-up.md` for the architecture on the other side: the full typed page
contract, a registry-driven lint CLI, federated wikis, and the three-tier
memory model.

## When not to use this at all

- A handful of notes (two or three files): the overhead of index + journal +
  catalog is not worth it. Use plain files.
- A corpus of millions of documents you did not curate: that is a search/RAG
  problem, not a hand-maintained wiki.
