# Candidate Deep Dive

Use after the user chooses a candidate, before interface design.

1. Restate the candidate and why it matters.
2. Map the current flow with exact files and call sequence.
3. Name the responsibilities that belong together.
4. Name what must stay outside the module.
5. List constraints any interface must satisfy:
   - runtime ownership
   - persistence or I/O behavior
   - error semantics
   - compatibility needs
   - test environment
   - migration path
6. Provide a rough illustrative code sketch only to ground constraints.

Label the sketch: `constraint sketch, not proposal`.

Then proceed to `workflows/design.md` if design mode is active.
