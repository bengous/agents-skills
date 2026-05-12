# Research Brief

Use this shape for each batch. Subagents, when available, must be read-only.

```text
Research this dependency update batch before any modification.

Repo:
Ecosystem:
Batch:
Current versions:
Candidate target:
Known repo usage:
Risk status candidate:

Check:
- release notes/changelog between current and target;
- breaking changes or migration notes;
- peer dependency or engine/runtime changes;
- known issues/regressions;
- whether Update vs Latest changes the risk;
- validation commands that should cover this batch.

Return:
- proceed / ask / defer / blocked;
- 3-6 concrete reasons;
- exact sources consulted when web/docs were used;
- suggested target version;
- validation commands;
- notes for the run artifact.
```

Research does not approve broad migrations by itself. If the batch is major, breaking, runtime/package-manager, CI/Docker, or requires dependency replacement/removal, ask the user before applying.
