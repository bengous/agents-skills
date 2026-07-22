# Audit Grid

Use this grid to decide what to inspect and how to classify findings. Ignore
absent surfaces unless the absence matters for the repo's size, risk, or stated
agent goals.

## Intensities

| Intensity | Use when | Expected coverage |
| --- | --- | --- |
| `fast` | User wants a quick scan or the repo is small. | Inventory plus highest-risk source/context/validation checks. |
| `standard` | Default. | Inventory, size classification, focused inspection, and read-only forks when allowed and useful. |
| `deep` | User asks for full/deep audit or the harness is large/critical. | Systematic surface decomposition and coverage appendix. |

## Size Classification

Use judgment, not rigid thresholds.

- `small`: one root context file, few or no rules/skills/hooks, simple package
  scripts, little generated context.
- `medium`: multiple context layers, generated projections, hooks or validation
  lanes, a few skills/workflows, CI.
- `large`: many nested instructions, multiple agent runtimes, skills, hooks,
  MCP/connectors, templates/generated output, evals/traces, or release gates.

## Surfaces

### Source Context

Look for `CLAUDE.md`, `.claude/rules/**`, root docs, nested context files,
frontmatter/scope rules, stale paths, contradictions, duplicated instructions,
size bloat, and unclear precedence.

Good signs:
- Clear source of truth.
- Conditional workflows live outside root context.
- Instructions name side effects and ownership boundaries.
- Progressive disclosure is used for heavy detail.

Risks:
- Editable and generated instructions conflict.
- Root instructions carry every workflow in full.
- Generated files are treated as source.
- Rules mention stale paths, dead commands, or obsolete tool contracts.

### Generated Context

Look for root/nested `AGENTS.md`, sync scripts, manifests, checksums, generation
tests, and drift checks.

Good signs:
- Generated projections are reproducible.
- The source-to-output mapping is explicit.
- Drift checks run in local validation or CI.

Risks:
- Agents are told to edit projections directly.
- Generated projections are stale, huge, or missing scoped rules.
- No command proves projection parity.

### Skills And Workflows

Look for `.agents/skills/**`, `.claude/skills/**`, `SKILL.md` frontmatter,
references, assets, scripts, workflow gates, and overlap between skills.

Good signs:
- Descriptions trigger on concrete user intent.
- Heavy detail is in references.
- Side effects and validation are explicit.
- Human-only or explicit-only skills say so in both routing and body when
  relevant.

Risks:
- Skills hide side effects.
- Descriptions are too vague or over-trigger.
- Scripts exist without deterministic validation or ownership.
- Similar skills conflict.

### Agent Runtime Config

Repo-local only by default: `.codex/**`, `.claude/settings.json`, `.mcp.json`,
tool allow/deny lists, sandbox/approval settings, model/tool config, memory
routing, and subagent/plugin declarations.

Good signs:
- Permissions are least-privilege and explain trust boundaries.
- Runtime config is generated or checked when appropriate.
- Hooks and tools have explicit timeouts and failure behavior.

Risks:
- Broad permissions with no rationale.
- Live/global assumptions encoded in repo docs.
- Secret or external-service access is unclear.

### Tools, Hooks, And Side Effects

Look for hook wrappers, shared hook runtimes, MCP/connectors, environment use,
external calls, destructive-action guards, generated-file guards, install/push
commands, and approval gates.

Good signs:
- Side effects are named before execution.
- Destructive or external actions are gated.
- Hook logic is testable outside the harness.
- Failure paths are fail-closed where safety matters.

Risks:
- Silent fallback on hook failure.
- Hooks mutate files during read-only phases.
- Network, email, publish, or install surfaces are not explicit.

### Validation And Feedback

Look for package scripts, validation lanes, CI workflows, typecheck/lint/test,
security checks, link checks, generated-project contract tests, evals, traces,
and stop-hook validation.

Good signs:
- Cheap local gate exists.
- Deep gate exists for release or generated output.
- Validation covers agent-facing generated output.
- Test names describe behavior.

Risks:
- Validation relies on hidden global tools.
- Generated project behavior is not exercised.
- Stop/review hooks cannot prove read-only behavior.

### Continuity And Recovery

Look for specs, PRDs, plans, trackers, ADRs, domain docs, handoff prompts,
memory/recovery files, transcripts, and long-running workflow state.

Good signs:
- Long work can resume from durable artifacts.
- Human gates are explicit.
- Status files distinguish plan, progress, and final evidence.

Risks:
- Progress is only in chat.
- Generated plans drift from code.
- Recovery artifacts are untracked but treated as canonical.

### Code Intelligence

Look for LSP config, generated docs, code wiki, search/RAG/indexing, dependency
source workflows, and code ownership docs.

Good signs:
- Agents are pointed to mature local navigation tools.
- Generated docs are checked or explicitly non-canonical.

Risks:
- Stale code maps are trusted as source.
- Agents are told to use broad searches where precise code tools exist.

### Source Ownership

Classify each relevant path:

- source file
- generated projection
- copied preset/template source
- runtime/local config
- external system
- local-only artifact

Findings should say which owner must change. Do not recommend editing a generated
output when a source exists.

## Severity

| Severity | Meaning |
| --- | --- |
| Critical | Can cause destructive actions, external side effects, secret exposure, or systemic wrong edits. |
| High | Likely to mislead agents or break validation/source ownership in normal work. |
| Medium | Meaningful reliability, maintainability, or drift risk with contained blast radius. |
| Low | Local clarity, discoverability, or hygiene issue. |

## Evidence And Confidence

Each finding should include:

- Local evidence: path and line when possible.
- Source basis: bundled corpus, freshly verified primary source, or inference.
- Confidence: high, medium, or low.
- Limit: what was not checked.
