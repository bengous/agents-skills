# Portable Eval Artifact Records

Use this reference to preserve enough evidence to reproduce, audit, and interpret a skill eval. Adopt the native schema and layout of the active harness or viewer; the examples below describe logical records, not mandatory filenames or field names.

## Contents

- [Record Set](#record-set)
- [Minimal Examples](#minimal-examples)
- [Integrity Audit](#integrity-audit)
- [Validity Audit](#validity-audit)
- [Harness Adapters](#harness-adapters)

## Record Set

| Record | Minimum content |
| --- | --- |
| Evaluation definition | Suite purpose, claim, target population, tasks, success criteria, graders, metrics, configurations, trial policy, retry and exclusion rules |
| System snapshot | Model, agent harness, eval harness, skill snapshot or fingerprint, prompts, tools, environment, resource limits, timeout, concurrency, network policy |
| Trial | Stable task/config/trial IDs, attempt, timestamps, execution status, raw output, final-state evidence, available trace, metrics and their source |
| Grade | Grader type and version, rubric or checks, criterion results, evidence, task-level result, grading errors |
| Aggregate report | Scheduled and observed trials, per-task results, infrastructure failures, exclusions, uncertainty, comparisons, limitations, unknowns |
| Human feedback | Reviewer decisions, grader disagreements, accepted alternative solutions, and changes requested for the next iteration |

Keep stable IDs across records. Preserve unavailable values as explicitly missing with a reason. Append trials; never reuse a trial ID or overwrite raw evidence.

## Minimal Examples

Adapt these shapes to the active consumer. Add fields only when they support a real claim or required integration.

### Evaluation definition

```json
{
  "suite_id": "shell-safety-regression",
  "purpose": "regression",
  "claim": "The skill still guides the tested shell-safety tasks correctly",
  "tasks": [
    {
      "task_id": "hostile-paths",
      "input_ref": "tasks/hostile-paths.md",
      "success_criteria": ["preserves argument boundaries", "prevents partial output"],
      "grader_refs": ["graders/hostile-paths.json"]
    }
  ],
  "configurations": ["with_skill"],
  "trial_policy": {
    "trials_per_task": 1,
    "interpretation": "single-trial regression evidence",
    "retry_rule": "retry infrastructure failures only"
  }
}
```

### System snapshot

```json
{
  "config_id": "with_skill",
  "model": {"id": "record-exact-model-id"},
  "agent_harness": {"name": "record-harness", "version": "record-version"},
  "eval_harness": {"name": "record-runner", "version": "record-version"},
  "skill": {"source": "shell-safety-skill", "fingerprint": "record-content-hash"},
  "tools": ["record-tool-versions"],
  "environment": {
    "runtime": "record-runtime",
    "resource_guarantees": "record-if-known",
    "resource_limits": "record-if-known",
    "timeout_seconds": "record-value",
    "concurrency": "record-value",
    "network": "record-policy"
  }
}
```

### Trial and grade

```json
{
  "trial_id": "hostile-paths.with-skill.1",
  "task_id": "hostile-paths",
  "config_id": "with_skill",
  "attempt": 1,
  "execution_status": "completed",
  "artifacts": {
    "raw_output_ref": "outputs/response.md",
    "final_state_ref": "state/result.json",
    "trace_ref": "traces/trial.jsonl"
  },
  "metrics": {
    "wall_time_ms": null,
    "unavailable_reason": "not exposed by eval harness"
  },
  "grade": {
    "grader": {"type": "code", "version": "grader-content-hash"},
    "criteria": [
      {"id": "preserves-argv", "result": "pass", "evidence": "tests/hostile-paths: pass"}
    ],
    "task_result": "pass"
  }
}
```

Use `infrastructure_failure`, `interrupted`, or `excluded` instead of `completed` when execution does not produce a grade, with a reason and any retained evidence. Record a completed task failure as `grade.task_result: "fail"`; do not collapse it into an infrastructure failure. Omit unavailable metrics or mark them unavailable; do not estimate them.

### Aggregate report

```json
{
  "suite_id": "shell-safety-regression",
  "scheduled_trials": 1,
  "observed_trials": 1,
  "task_results": {"passed": 1, "failed": 0},
  "infrastructure_failures": 0,
  "comparison": {
    "status": "not_measured",
    "reason": "no baseline configuration"
  },
  "limitations": ["one trial per task; no reliability estimate"]
}
```

Report assertion-level totals separately when useful. Do not relabel them as task success. Include uncertainty only when the design and number of trials support it.

## Integrity Audit

- Confirm every scheduled task and configuration has a trial record or an explicit missing reason.
- Link every trial to the exact evaluation definition, grader, system snapshot, and skill snapshot it used.
- Preserve raw output, final-state evidence, traces when available, grader output, and metric provenance.
- Recompute aggregate counts from trial records; do not trust stale summaries.
- Mark manual repairs, exclusions, retries, and reviewer overrides without replacing original evidence.
- Keep prior iterations immutable and distinguish observed data from analyst commentary.

Passing this audit means the records are complete and internally consistent. It does not establish that the tasks, graders, harness, or conclusions are valid.

## Validity Audit

- Confirm tasks represent the stated population and expose all grader-checked requirements.
- Confirm a verified path can pass every task and grader.
- Confirm graders accept valid alternatives and measure output or final state unless process is explicitly in scope.
- Confirm compared configurations differ only in intended variables, or disclose the confounders.
- Confirm trials are isolated and material environment, resource, timeout, concurrency, and network conditions are recorded.
- Separate infrastructure failures from agent failures according to the predeclared policy.
- Limit conclusions to the observed trials; require repeated trials for reliability or variance claims.
- Read representative outputs and traces before trusting aggregate scores.

## Harness Adapters

Inspect the actual harness, aggregator, and viewer before writing artifacts. If a consumer requires exact paths or fields, follow and validate that local contract; do not present it as general eval methodology. Prefer its generator or exporter over hand-built compatibility files.

The [Claude Console Evaluation Tool](https://platform.claude.com/docs/en/test-and-evaluate/eval-tool) compares prompt variants and test cases in Console. Treat it as one optional prompt-eval surface, not as the required storage or execution model for agent or skill evals.
