# Terminology

Use these terms in reports so source ownership and audit scope stay precise.

| Term | Meaning |
| --- | --- |
| Agent harness | Repo-local and runtime system that makes agents reliable: context, instructions, tools, hooks, validation, traces, permissions, recovery, and governance. |
| Context engineering | Design of the context loaded by agents: persistent files, hierarchy, progressive disclosure, memory, recovery, sources of truth, and just-in-time context. |
| Source context | Editable instruction/context files that own the content. In Kitsmith-style repos this usually means `CLAUDE.md` and `.claude/rules/**`. |
| Generated agent projection | Agent-facing file generated from source context, such as root and nested `AGENTS.md`. Treat as runtime output, not the editable source. |
| Harness audit | Evidence-backed audit of the agent harness and context engineering against an explicit grid and source corpus. |
| Adaptive audit | Workflow that chooses inline audit or read-only forks according to repo size and harness complexity. |
| Audit fork | Read-only subagent/reviewer scoped to one harness surface. It returns facts, evidence, risks, and gaps; it does not make final product recommendations. |
| Audit intensity | User-selected or default depth: `fast`, `standard`, or `deep`. |
| Repo-source audit | Default v1 scope: inspect versioned repo files only, not global installed tools or live user config. |
| Runtime audit | Future or explicit scope: inspect global config, installed plugins, live MCP, memory, logs, and loaded hooks. |
| Audit artifact | Local Markdown report written by the audit mode. It is review output, not an implementation change. |
| Corpus refresh | Explicit maintenance mode that updates the source corpus or audit grid after consulting current primary sources. |
| Source-backed recommendation | Recommendation tied to local evidence and either the bundled corpus, a freshly verified primary source, or an explicit inference. |

## Ownership Rules

- Audit the current repo, not a hard-coded project.
- Preserve source/generated boundaries.
- Treat local runtime config, generated files, copied presets/templates, and
  external systems as separate ownership surfaces.
- State when a finding depends on a repo convention rather than a universal rule.
