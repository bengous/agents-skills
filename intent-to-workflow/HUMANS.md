# Human Notes

`intent-to-workflow` was inspired by Matt Pocock-style agent skills that break a vague idea into clarification, PRD, issues, TDD, and execution prompts.

This implementation intentionally owns that workflow locally instead of depending on external companion skills. The packaged templates and references are modified for this repo's constraints:

- local artifacts first;
- human-gated phase transitions;
- local-only artifact behavior;
- no companion skill requirements;
- deterministic `itw get` and `itw advance` phase prompts.

`SKILL.md` is the agent contract. This file is only provenance and orientation for human maintainers.
