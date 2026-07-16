# rust-craft capability benchmark — iteration 1

## Verdict

The capability claim is not demonstrated. `rust-craft`, Apollo, and no skill each passed 3 of 4 tasks and 19 of 20 critical assertions. No observed lift.

| Configuration | Task passes | Critical assertions |
|---|---:|---:|
| No skill | 3/4 | 19/20 |
| Apollo | 3/4 | 19/20 |
| rust-craft | 3/4 | 19/20 |

All 12 executor trials completed without an infrastructure failure. Their deterministic project checks passed before blind grading.

## Task results

| Task | No skill | Apollo | rust-craft | Finding |
|---|---:|---:|---:|---|
| MSRV and feature wiring | Pass | Pass | Pass | Final code states were identical, excluding reports. |
| Managed async shutdown | Pass | Pass | Pass | All produced owned, joinable task lifecycles. |
| Public API evolution | Fail | Fail | Fail | All omitted public documentation of the non-`Send` future/executor limitation. |
| Safe FFI wrapper | Pass | Pass | Pass | All passed boundary tests and independent `Send`/`Sync` probes. |

## Interpretation

This is a useful negative result, not proof that the skill has no value. The suite did not isolate value supplied by `rust-craft`: three tasks were easy enough for the baseline, while the same documentation omission defeated all configurations.

The largest design flaw is prompt leakage. `TASK.md` states many decisive constraints directly, so a capable baseline can solve the task without applying `rust-craft`'s project-contract discovery workflow. These fixtures should become regression candidates, not capability evidence.

## Validity limits

- One run per task/configuration; no reliability estimate.
- The no-skill condition uses an explicit prohibition because the harness cannot physically remove globally available skills.
- Executor traces, token counts, and effective runtime model identity were unavailable.
- Skill, fixture, and eval snapshots were frozen and hashed; grading was configuration-blind.

## Next evaluation

Keep the three-way comparison and blind grading. Replace capability tasks with harder fixtures where decisive Rust constraints live in `Cargo.toml`, CI, downstream crates, target configuration, safety contracts, and existing APIs, without restating those constraints in the prompt. Freeze the new suite before running it.
