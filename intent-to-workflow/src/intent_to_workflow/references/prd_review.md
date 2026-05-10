# PRD Review Gate

Review `prd.md` and `terminology.md` only. Do not continue PRD generation, create issues, or write next-phase artifacts during this gate.

Check `prd.md` against `intake`, `clarification.md`, and `terminology.md`:

- the problem and solution match the captured intent;
- actors, roles, and user-story actor names match the terminology model;
- canonical terms are used consistently, and aliases to avoid do not leak into the PRD;
- language ambiguities are resolved or explicitly flagged;
- important relationships between actors, artifacts, and decisions are respected;
- user stories cover the accepted scope without inventing unrelated work;
- implementation decisions are concrete enough for issue slicing;
- out-of-scope notes are explicit;
- open questions or blockers are listed.

Report either approval or blocking findings. If corrections are needed, describe the exact change and wait for the human decision before editing.
