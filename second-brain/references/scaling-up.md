# Scaling up

What the pattern looks like after it graduates — a case study of the advanced
architecture, distilled from a production deployment (the IdeAs monorepo: three
federated wikis linted by one registry-driven Go CLI). `limits.md` says where
the baseline stops; this file describes what sits on the other side. None of it
replaces the baseline — a single wiki with the bash check stays the right
default. Each section names the trigger that justifies its cost.

## The full page contract

The scaffolded frontmatter (`type:`, `updated:`) is the seed of a larger
contract:

```yaml
---
type: reference        # one enum per wiki, declared in its SCHEMA.md
updated: 2026-07-02
sources:
  - ../data/projects.yaml
---
```

- `type` gains a per-wiki enum (e.g. `overview | index | log | user | project |
  reference`) and the lint rejects values outside it. Buys routing: an agent
  reads the type before deciding whether a page is worth loading.
- `sources` names the canonical files the page synthesizes. Buys two things: an
  agent verifies action-sensitive claims against the sources before acting on
  the page, and a checker compares the page's `updated:` against the git commit
  dates of its sources to flag stale syntheses.

Trigger: pages start making claims an agent acts on — the moment
wrong-but-plausible memory becomes expensive.

## From bash lint to a real CLI

The bundled `check-wiki.sh` validates shape: required files, anti-orphan
reachability, `updated:` format, index freshness. It deliberately stops at what
grep and find can do. Graduate to a real CLI (any language) when checks need
parsing or git:

- wikilink resolution — broken and ambiguous `[[links]]`, and index coverage by
  resolved link rather than literal path match;
- page-size ceilings (warn at large, fail at oversized), so no page quietly
  becomes the giant file this pattern exists to prevent;
- source-vs-page staleness via git history — too slow for a commit gate, so it
  runs as a separate warning-only command.

Make the CLI registry-driven: one table of (wiki id, root, allowed page types)
is the single source of truth, and every subcommand — lint, search, stats,
stale — iterates it. Adding a wiki is then a one-line change.

Unenforced discipline drifts. In the source deployment, one index sat 27 days
older than its pages until the freshness check landed and caught it. The checks
are not decoration; they are what keeps the contract true.

## Federating wikis instead of growing one

`limits.md` shards the index when it stops being thin — a split by size.
Federation is a split by ownership: when two content sets have different
canonical sources to verify against and different write disciplines, make them
separate wikis, each with its own `index.md` and `SCHEMA.md`, registered side
by side in the CLI. A root wiki holds only cross-domain memory and points at
the others — it never duplicates them. Queries stay read-first per wiki: that
wiki's index first, then its pages.

Trigger: you keep writing routing rules ("X goes here, Y goes over there")
inside one SCHEMA.md. That schema is telling you it is two wikis.

## The three-tier memory model

At federation scale the loading model grows a middle tier:

1. **Always loaded** — a small directives file, the agent's standing
   instructions. Small by design; the only unconditional cost.
2. **Path-routed** — entrypoints and rules that load when work touches their
   paths. Domain feedback lives here, not in a wiki.
3. **On demand** — the wikis: read-first indexes, typed pages.

The baseline's eager/on-demand boundary is the two-tier version of this. The
middle tier appears when memory must follow file paths, not just topics.

## Two journal placements — a choice, not an upgrade

- **Baseline (this skill): eager windowed journal.** The recent window loads
  every session; sealed months stay on demand. Right when one wiki is the
  working memory and continuity between sessions is the point.
- **Federated: everything on demand except each index.** The always-loaded tier
  is the directives file outside the wikis, and each journal is just another
  on-demand page behind its index.

Both are documented configurations, not stages of maturity. N eager journals
cannot all be cheap, so federation pushes journals on demand — but a single
wiki that never federates loses nothing by keeping its window eager. Pick per
wiki.

## Projecting one memory to many agents

The wiki is agent-agnostic markdown precisely so several agents and harnesses
can share it. Two rules keep it that way:

- Durable memory lives in the repo, never in an agent-private store — a memory
  only one agent can read is a memory the team does not have.
- Agent-specific always-loaded files (`CLAUDE.md`, `AGENTS.md`, ...) stay thin
  pointers to one shared source; when a harness cannot follow pointers, a sync
  step projects the shared guidance into its native format.
