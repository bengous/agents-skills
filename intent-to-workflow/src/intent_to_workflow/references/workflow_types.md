# Workflow Types

Use the workflow phase as an explicit choice point. Recommend one workflow type
from the current intention, clarification, terminology, PRD, and issues, but
show the human the available alternatives before finalizing `workflow.md`.

## Workflow Type Bank

### PRD-to-slices reviewed execution

Use when the human starts with a fuzzy intention and wants to turn it into
implementation work for an agent.

Shape:

1. clarify/challenge the intention;
2. maintain terminology / local language;
3. produce a PRD;
4. review/harden the PRD;
5. produce coarse vertical-slice issues;
6. produce a workflow that covers those issues;
7. prepare reviewer personas and capabilities before execution;
8. execute each slice with this loop:
   - implement;
   - run slice validation;
   - ask three clean reviewers:
     - simplification reviewer using its embedded report-only simplify-wip
       capability;
     - security reviewer using its embedded WIP security-review capability;
     - contract reviewer using its embedded contract-review capability;
   - accept/reject/defer each finding with rationale;
   - apply accepted fixes;
   - revalidate;
   - commit;
   - move to the next slice.

## Customization Modes

- Choose recommended: keep the generated workflow type and refine wording only.
- Choose another type: replace the selected type with another bank entry when
  one exists.
- Tune a type: keep the base type, record what changed from the base workflow,
  and preserve the source type for future explanation.
- Pair-design from scratch: define a custom workflow with the human when no
  bank entry fits.

## Reviewer Personas and Capabilities

Define these before execution, not ad hoc after implementation starts.
For Codex, bind each persona to a real `agent_type`; the spawned subagent
message should contain only the task, scope, context, and requested output.
Other agent runtimes can be ported later without changing the workflow type.

- Worker: `itw-worker`; implements one issue and runs validation.
- Simplification Reviewer: `itw-simplification-reviewer`; read-only report-only
  simplification pass with an embedded report-only simplify-wip capability.
- Security Reviewer: `itw-security-reviewer`; read-only WIP security pass with
  an embedded security-review capability.
- Contract Reviewer: `itw-contract-reviewer`; read-only contract pass against
  `prd.md`, `terminology.md`, `issues.md`, acceptance criteria, validation, and
  tracker/workflow state, with an embedded contract-review capability.

Reviewer capabilities are permissions and tools, not authority to edit. The
workflow owner accepts, rejects, or defers findings before fixes are applied.

## Artifact Contract

- `issues.md` defines what to build.
- `workflow.md` defines how to execute the issues.
- `tracker.md` records where execution currently is and remains execution
  authority.

Even from-scratch workflows must preserve those three responsibilities.
