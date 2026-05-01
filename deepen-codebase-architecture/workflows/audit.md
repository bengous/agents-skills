# Audit Workflow

Goal: find candidate clusters where deepening a module would improve locality, leverage, clear seams, or AI entropy. Do not propose interfaces.

Load `workflows/evidence.md` first.

## Process

1. Scope the pass: repo, directory, feature, or workflow.
2. Explore organically like a future maintainer or agent trying to change behavior.
3. Track navigation friction:
   - one concept requires bouncing across many shallow files
   - callers repeat choreography, validation, defaulting, or error wrapping
   - types, tests, docs, and behavior for one concept are far apart
   - boundaries expose internal state or sequencing instead of stable behavior
   - tests assert implementation details because no useful boundary exists
   - agents would need broad context to make a narrow change safely
4. Group files by concept ownership, not by directory names alone.
5. Classify dependencies:
   - `in-process`: pure or local in-memory behavior
   - `local-substitutable`: I/O with a local stand-in
   - `remote-owned`: owned service behind a port/adapter
   - `true-external`: third-party system mocked at the boundary
6. Score each candidate against the matrix.
7. Present candidates and ask which one to explore.

## Candidate Matrix

| Signal | Strong candidate | Weak candidate |
|---|---|---|
| Locality / colocation | one concept scattered across files, tests, config, docs | files are separate for a clear platform/runtime reason |
| Leverage | many callers repeat the same dance | only one caller benefits |
| Clear seams | boundary can expose behavior and hide sequencing | boundary would just rename existing functions |
| AI entropy | future agents can inspect fewer files safely | change still needs global context |
| Test boundary | behavior can be tested at one public interface | tests must still poke internals |
| Blast radius | migration can be staged by callers | requires repo-wide rewrite first |

## Output

```markdown
## Candidates

1. <cluster>
   - Authority files: ...
   - Coupling: ...
   - Signals: locality ..., leverage ..., seams ..., AI entropy ...
   - Dependency category: ...
   - Test impact: ...
   - Risks: ...

Which candidate should I design?
```
