# Benchmark: harden-bash (iteration 1)

**Executor**: `gpt-5.6-terra` | **Grader**: `gpt-5.6-sol` | **Date**: 2026-07-14 | **Skill commit**: `78c95fc` | **Evals**: 4 | **Runs per config**: 1

## Summary

| Metric | with_skill | without_skill | Delta |
| --- | ---: | ---: | ---: |
| Assertion pass rate | 78.57% ± 15.97% | Not run | N/A |
| Root wall time | 211.94 s ± 123.03 s | Not run | N/A |
| Tokens | Unavailable | Not run | N/A |

Overall: 22 of 28 assertions passed. No baseline was run in v1, so this iteration does not measure skill lift.

## Per-Eval Breakdown

| Eval | Score | Result |
| --- | ---: | --- |
| 1. bash-untrusted-paths | 7/7 (100%) | Pass |
| 2. posix-bashism-rewrite | 5/7 (71.43%) | Partial |
| 3. errexit-trap-audit | 4/7 (57.14%) | Partial |
| 4. process-supervision | 6/7 (85.71%) | Partial |

## Failed Assertions

- Eval 2 omitted the external `grep -H` portability assumption and let a filename beginning with `-` be parsed as an option.
- Eval 3 did not read `process-lifecycle.md`, did not explain the relevant `set -e` ignored contexts, and incorrectly claimed that the supplied `EXIT` trap could obscure the selected status.
- Eval 4 checked shutdown before spawning but omitted the post-spawn signal check, leaving a signal/spawn boundary race.

## Integrity And Limits

- Executor runs used `gpt-5.6-terra`; independent graders used `gpt-5.6-sol`.
- Two Terra completion messages were redacted by OpenAI's cyber-safety rendering in the user interface. Their outputs and transcripts remained on disk.
- Codex agents did not expose token totals. Durations are root wall-clock measurements and are not suitable for model-efficiency claims.
- The first preflight verifier violated its static scope by executing the unsafe `TMPDIR` trap, deleting existing `/tmp` entries before recreating `/tmp` with mode `1777`. See `verification/incident.md`.
