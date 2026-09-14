# Report Template

Use this structure for the audit artifact. Keep the report evidence-first and
avoid implementation slices.

```markdown
# Agent Harness Audit

Date: YYYY-MM-DD
Repo: /absolute/or/repo/path
Mode: audit
Intensity: fast|standard|deep
Artifact: .agents/reports/harness-audit-YYYY-MM-DD.md

## Scope

- Repo-source audit only: yes/no
- Runtime/global config checked: yes/no
- Sources consulted:
- Local validation run:
- Limits:

## Executive Summary

- Overall maturity: low|medium|high, with one-sentence rationale
- Strongest current property:
- Highest-risk gap:
- Highest-leverage recommendation:

## Harness Inventory

| Surface | Present | Evidence | Notes |
| --- | --- | --- | --- |
| Source context | yes/no | path:line | |
| Generated projection | yes/no | path:line | |
| Skills/workflows | yes/no | path:line | |
| Hooks/tools/MCP | yes/no | path:line | |
| Runtime repo config | yes/no | path:line | |
| Validation/CI | yes/no | path:line | |
| Continuity/recovery | yes/no | path:line | |
| Code intelligence | yes/no | path:line | |

## What Works Well

### Strength 1

- Evidence:
- Why it matters:
- Source basis:

## Findings

### Finding 1: Short actionable title

- Severity: critical|high|medium|low
- Surface:
- Evidence:
- Source basis: local proof|bundled corpus|fresh primary source|inference
- Confidence: high|medium|low
- Risk:
- Recommendation:
- Implementation boundary:

## Recommendations

| Priority | Recommendation | Evidence | Confidence | Boundary |
| --- | --- | --- | --- | --- |
| P1 | | | | |

## Notable Absences

List only absences that matter for this repo.

## Deferred / Out Of Scope

- Runtime/global audit:
- Implementation plan:
- External systems:

## Coverage Appendix

- Inline surfaces inspected:
- Fork/subagent surfaces inspected:
- Files or directories intentionally skipped:
- Commands run:

## Sources

- Local repo evidence:
- Bundled corpus:
- Fresh primary sources:
```

## Final Response Shape

After writing the artifact, answer briefly:

- report path
- top 3 findings or "no high-risk findings"
- commands run
- important limits
- note that no implementation changes were made
