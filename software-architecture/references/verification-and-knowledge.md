# Verification and knowledge

Read when making decisions discoverable or checking they remain true.

## Match a check to the intended guarantee

| Decision | Possible check |
|---|---|
| Module internals are private | Package visibility and focused import/dependency rules |
| Invalid combinations cannot be constructed | Compiler checks plus boundary parsing |
| Adapters implement the same behavior | Contract tests exercised against relevant adapters |
| Behavior survives a refactor | Domain or user-observable tests independent of internal call order |
| A flow must remain responsive | Representative measurement or traces with a stated budget |
| Persisted state respects an invariant | Database constraint or transaction/concurrency verification |

Select the smallest check resolving material uncertainty, using existing tools
first. Do not add a linter, test framework, blanket coverage target or full
end-to-end suite just because this reference was read. Fakes isolate a rule;
they do not establish fidelity to the real database or provider.

A boundary should fail with an actionable diagnostic when violated. For example:
`orders imports billing/internal; use billing's public refund operation`.
Enforce the meaningful dependency, not arbitrary file counts.

## Help the next reader reconstruct the system

Keep a small map of responsibilities, entry points, data ownership and non-obvious
invariants. Explain significant decisions through context, tradeoffs and
conditions for reconsideration. Prefer a nearby explanation for a local
constraint; use a project decision record when its scope justifies one.

Record facts that cannot be inferred cheaply. Do not maintain an inventory of
every file or copy generic architecture tutorials into project instructions.
An existing decision is evidence of intent, not permanent proof of correctness.

Make relevant checks and flows reproducible when needed. Humans and agents
benefit from accessible logs, startup commands and a way to exercise behavior.
Keep infrastructure proportional; a library need not acquire a browser harness.

## Evidence limits

A coherent guideline is not evidence that it improves agent outcomes. Treat this
skill as decision support. Broad agent evaluations are optional work, not a
prerequisite for applying it or an automatic step of an architecture task.
Report checks actually performed without implying independent validation.

## Sources

Reviewed 2026-09-07. Tool docs are options, not required dependencies.

- [Spring Modulith: verification](https://docs.spring.io/spring-modulith/reference/verification.html)
  — cycles, internal access and permitted dependencies.
- [Nx: module boundaries](https://nx.dev/docs/features/enforce-module-boundaries)
  — import rules for project boundaries.
- [matklad: ARCHITECTURE.md, 2021](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html)
  — a durable map for unfamiliar contributors.
- [rust-analyzer: architecture](https://rust-analyzer.github.io/book/contributing/architecture.html)
  — real responsibilities, entry points and explicit invariants.
- [OpenAI: Harness engineering, 2026-02-11](https://openai.com/index/harness-engineering/)
  — knowledge and mechanical feedback in one team's agent workflow. Its scale
  and merge policies are not prescriptions for other projects.
