# To Issues

Break a plan into independently-grabbable local issues using vertical slices (tracer bullets). Write the result to `issues.md` in the current ITW root.

## Process

### 1. Gather context

Work from whatever is already in the conversation context and local ITW artifacts. Read `terminology.md` and use its canonical actor and term names when slicing.

### 2. Explore the codebase (optional)

If you have not already explored the codebase, do so to understand the current state of the code.

### 3. Draft vertical slices

Break the plan into **tracer bullet** issues. Each issue is a thin vertical slice that cuts through ALL integration layers end-to-end, NOT a horizontal slice of one layer.

Slices may be 'HITL' or 'AFK'. HITL slices require human interaction, such as an architectural decision or a design review. AFK slices can be implemented without human interaction. Prefer AFK over HITL where possible.

<vertical-slice-rules>
- Each slice delivers a narrow but COMPLETE path through every layer (schema, API, UI, tests)
- A completed slice is demoable or verifiable on its own
- Prefer many thin slices over few thick ones
</vertical-slice-rules>

### 4. Quiz the user

Present the proposed breakdown as a numbered list. For each slice, show:

- **Title**: short descriptive name
- **Type**: HITL / AFK
- **Blocked by**: which other slices (if any) must complete first
- **User stories covered**: which user stories this addresses (if the source material has them)

Ask the user:

- Does the granularity feel right? (too coarse / too fine)
- Are the dependency relationships correct?
- Should any slices be merged or split further?
- Are the correct slices marked as HITL and AFK?

Iterate until the user approves the breakdown.

### 5. Write the local issues

For each approved slice, write an entry in `issues.md` using the issue body template below.

Write issues in dependency order (blockers first) so you can reference earlier local issue numbers in the "Depends on" field.

<issue-template>

### <number>. <title>

Type: AFK | HITL

Depends on: none | issue numbers

Goal:

A concise description of this vertical slice. Describe the end-to-end behavior, not layer-by-layer implementation.

Acceptance:

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3

TDD:

- First behavior test: TODO

Validation:

Commands, checks, or manual verification required to prove the slice works.

Agent:

Short guidance for the future agent that will implement this slice.

</issue-template>

Do not modify `prd.md` while writing `issues.md` unless the human explicitly asks for a PRD correction.
Do not modify `terminology.md` while writing `issues.md` unless the human explicitly asks for a terminology correction.
