# Source Corpus

The bundled corpus is a seed, not proof that every claim is current forever.
Use source labels in every recommendation so freshness is explicit.

## Seed Corpus

When available in this environment, the local seed report is:

```text
/home/b3ngous/Downloads/deep-research-report-corrige.md
```

It was captured during the initial Kitsmith planning work and covered current
agent-harness themes across OpenAI, Anthropic, Google, GitHub, Microsoft, Meta,
Cohere, MCP, hooks, subagents, context engineering, validation, evals, LSP/RAG,
and deterministic feedback loops.

Do not assume this absolute path exists in a target repo or on another machine.
If it is absent, rely on the bundled references and clearly mark the corpus limit.

## Evidence Labels

Use one of these labels for each recommendation:

- `local proof`: directly observed in the repo, preferably with path and line.
- `bundled corpus`: based on this skill's references or the seed report.
- `fresh primary source`: verified during this audit or refresh against official
  source material.
- `inference`: reasoned from local evidence and harness principles, but not
  directly asserted by a source.

## Freshness Rules

- If the user asks for "latest", "current", "most recent", or `corpus refresh`,
  verify against current primary sources before making normative claims.
- Prefer official docs, engineering posts, product docs, standards, and source
  repositories over commentary.
- For OpenAI product/API claims, use the official OpenAI documentation tooling
  when available.
- For library/framework/API details, use official docs or Context7 where the
  current runtime requires it.
- Record the date of source verification in the report.

## Corpus Refresh

Only run on explicit request. Suggested maintenance flow:

1. State that this mode may edit skill references.
2. Inventory current reference files.
3. Search/consult primary sources by source family.
4. Update `source-corpus.md`, `audit-grid.md`, or other references only where
   the new source changes the audit method or evidence basis.
5. Keep historical claims cautious; prefer "verified on YYYY-MM-DD" over
   timeless claims.
6. Summarize sources consulted and leave a visible Git diff.

Do not refresh sources during a normal audit unless the user asks for latest
source verification.
