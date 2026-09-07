# Existing systems

Read when assessing or changing established code.

## Find evidence of a problem

Start in the area named by the user. Follow a representative request, command,
UI event or data transformation through dependencies and persistence. Inspect
callers, tests and nearby decisions. Recent history can identify recurring
friction, but a quiet component can still contain a serious correctness risk.

Useful evidence includes callers repeating coordination, inconsistently enforced
invariants, hidden global state, dependency cycles, unrelated changes forced
together, and required behavior that is difficult to verify. File length, a
disliked name, a missing pattern or a large module count alone does not establish
an architectural defect.

If relevant code is unavailable, label the analysis provisional. Do not fabricate
file-level findings from a project description.

## Form a reviewable proposal

Connect each improvement to:

1. Evidence: flows, callers, tests or dependency edges showing the issue.
2. Consequence: what becomes fragile, slow to change or hard to understand.
3. Change: the responsibility or contract that moves, not just renamed folders.
4. Cost: migration, new indirection, operational effects and affected consumers.
5. Verification: how to detect regression or failure of the premise.

If removing an abstraction duplicates its responsibility across callers, it may
be doing useful work. If it only forwards arguments, ask whether it still
provides a needed contract, translation or isolation boundary before deleting it.
Small implementation size is not evidence of uselessness.

Prioritize consequential findings without filling a quota. Returning no
architectural change is valid. Describe cosmetic improvements as such.

## Migrate at the smallest useful boundary

Capture behavior that matters, especially compatibility and historical edge
cases. Characterization checks document current behavior; suspected bugs still
need a decision about intended behavior. Avoid freezing every accident forever.

Example: replace a tangled document exporter by introducing one export contract
around its current implementation. Route a caller through it, verify results,
then replace the implementation incrementally. Retain temporary indirection only
as long as its migration or architectural purpose exists.

For coexisting implementations, describe the cutover and removal condition.
Data migrations need compatibility planning; reverting application code does not
necessarily reverse data changes. Shadow comparisons must not duplicate external
writes or notifications.

Avoid combining code moves, behavior changes and persistence changes unless
their dependency makes that necessary. Preserve useful tests; replace
implementation-coupled tests only when meaningful behavior remains covered.
Test effort should address remaining risk, not a universal coverage target.
Broad rewrites need a credible account of compatibility.

## Sources

Reviewed 2026-09-07; the workflow is an adaptation, not a verbatim recipe.

- [Fowler: Branch by Abstraction, 2014](https://martinfowler.com/bliki/BranchByAbstraction.html)
  — gradual replacement while keeping the system usable.
- [Fowler: Refactoring Module Dependencies](https://martinfowler.com/articles/refactoring-dependencies.html)
  — confining the consequences of change.
- [Pocock: improve-codebase-architecture, current source](https://github.com/mattpocock/skills/blob/main/skills/engineering/improve-codebase-architecture/SKILL.md)
  — navigation friction and recent changes. Its mandatory vocabulary, report
  format and multi-agent workflow are not adopted here.
