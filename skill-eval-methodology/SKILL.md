---
name: skill-eval-methodology
description: Portable discipline for designing, running, and interpreting skill evaluations with skill-creator. Use when defining eval success criteria, running or comparing trials, grading outputs or end states, reviewing traces, benchmarking a skill, iterating on evals, or maintaining eval artifacts. Separates experimental validity from artifact integrity and harness-specific formats.
---

# Skill Eval Methodology

Use skill-creator for skill mechanics. Use this discipline to make eval claims supportable and eval records auditable.

## Reference

Read @references/artifact-examples.md when defining an artifact layout, adapting an existing harness, or auditing completeness. Treat its records as a logical model, not as required filenames or a universal JSON schema.

## 1. Define The Evaluation Contract

- State the user behavior or failure being measured and the claim the eval should support.
- Classify the suite:
  - **Capability:** measure what the system can do and preserve headroom for improvement.
  - **Regression:** verify that known behavior still works; a near-perfect result is the intended steady state.
- Identify the system under test: model and version, agent harness or scaffold, eval harness, skill version or fingerprint, tools, prompts, and environment.
- Define task success before running. Keep task, trial, grader, assertion, and suite metrics distinct; assertion pass rate is not automatically task success.
- Require a controlled baseline only for comparative claims such as skill lift. Do not require one for a standalone regression check.

## 2. Design Tasks And Graders

- Draw tasks from real use, requirements, or observed failures. Make each task unambiguous, passable, and representative of the intended distribution.
- Prove task solvability with a reference solution or another verified passing path.
- Test both when a behavior should occur and when it should not when over-triggering and under-triggering are possible.
- Grade the output or final environment state. Grade a tool path or transcript only when the path itself is part of the contract.
- Prefer deterministic graders where they express the requirement. Use model graders for necessary nuance, with explicit rubrics, evidence, and calibration against human judgment when the stakes warrant it.
- Reject criteria that are trivial, unfalsifiable, factually wrong, or prescriptive about one valid implementation.
- Verify tasks, graders, and external facts before a run. Keep static verification read-only; execute embedded commands only inside an explicitly scoped, isolated trial.

## 3. Plan Comparable Trials

- Freeze or fingerprint the exact skill, task definitions, graders, and harness configuration. Use a commit when authorized; otherwise use an immutable snapshot or content hash.
- Choose trial count from the claim and expected non-determinism. Treat one trial as evidence about that trial, not a reliability estimate.
- Start each trial from a clean, isolated state. Prevent prior outputs, caches, histories, or concurrent trials from leaking information.
- Match intended deployment conditions when evaluating the deployed system. Record material differences.
- Record resource guarantees and limits, timeout, concurrency, network access, tool versions, and other conditions that can affect the result.
- Predeclare retry and exclusion rules. Track infrastructure failures separately from task failures.
- Plan to preserve raw outputs, final state, available traces, grader evidence, and metrics. Mark unavailable data; never infer or fabricate it.

## 4. Run And Preserve

- Write the evaluation definition and system snapshot before launching trials.
- Preserve each trial as it completes. Never overwrite a previous trial or silently repair its output.
- Blind graders to the tested configuration when practical and prevent expected answers or prior conclusions from leaking into execution.
- Record incomplete, interrupted, excluded, and infrastructure-failed trials with their reason.
- Treat artifact loss or mismatch as an integrity failure. Treat unsafe execution, contamination, broken graders, or ambiguous tasks as validity failures even when every expected file exists.

## 5. Review And Interpret

- Reconcile scheduled trials, completed trials, grades, and aggregate counts.
- Inspect representative successes and failures. Confirm that grades are fair and supported by output, final-state, or trace evidence.
- Compare configurations or iterations only when the relevant task, grader, model, harness, and environment controls support the comparison. Name every material change.
- Interpret a perfect score by suite purpose: useful regression evidence, but saturation for capability improvement. Do not automatically discard it.
- Treat equal scores as “no measured difference in these trials,” not proof that the skill has no value.
- Use no fixed delta as proof of discrimination or significance. Judge trial count, variance, task coverage, grader reliability, and infrastructure noise before making a comparative claim.
- Bound the conclusion to the observed system and conditions. Separate observations, inferences, limitations, and unknowns.
- Keep artifact completeness and experimental validity as separate review outcomes.

## Iteration Boundary

- Preserve the prior iteration and its feedback.
- Change the skill, task, grader, or harness deliberately; prefer controlled changes when claiming causality.
- Reverify changed criteria and rerun the relevant regression coverage.
- Promote saturated capability tasks to regression coverage when they still represent valuable behavior.

## Method Basis

- [Anthropic: Demystifying evals for AI agents](https://www.anthropic.com/engineering/demystifying-evals-for-ai-agents)
- [Anthropic: Quantifying infrastructure noise in agentic coding evals](https://www.anthropic.com/engineering/infrastructure-noise)
- [Claude Platform: Define success criteria and build evaluations](https://platform.claude.com/docs/en/test-and-evaluate/develop-tests)
