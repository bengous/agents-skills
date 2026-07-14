# Benchmark: harden-bash (iteration 2)

**Executor**: `gpt-5.6-terra` | **Grader**: `gpt-5.6-sol` | **Date**: 2026-07-14 | **Skill commit**: `2df563b` | **Evals**: 4 | **Runs per config**: 1

## Summary

| Metric | with_skill | without_skill | Delta |
| --- | ---: | ---: | ---: |
| Assertion pass rate | 92.86% ± 7.14% | Not run | N/A |
| Root wall time | 210.33 s ± 134.43 s | Not run | N/A |
| Tokens | Unavailable | Not run | N/A |

Overall: 26 of 28 assertions passed. Iteration 1 passed 22 of 28, but the comparison is not controlled because eval 2's prompt and eval 3's assertions changed. No baseline was run, so skill lift is not measured.

## Per-Eval Breakdown

| Eval | Iteration 2 | Iteration 1 |
| --- | ---: | ---: |
| 1. bash-untrusted-paths | 6/7 (85.71%) | 7/7 (100%) |
| 2. posix-bashism-rewrite | 6/7 (85.71%) | 5/7 (71.43%) |
| 3. errexit-trap-audit | 7/7 (100%) | 4/7 (57.14%) |
| 4. process-supervision | 7/7 (100%) | 6/7 (85.71%) |

## Failed Assertions

- Eval 1's focused test passed an absolute path ending in `/-leading`; the argument itself began with `/`, so it did not test a true leading-dash operand.
- Eval 2 used `grep -H` and `grep --` successfully in Debian and BusyBox but did not disclose those external utility portability assumptions.

## Integrity And Limits

- Executor runs used fresh `gpt-5.6-terra` agents; independent graders used fresh `gpt-5.6-sol` agents.
- Static preflight passed without executing embedded eval snippets.
- Eval 2's behavior check passed in dash and BusyBox ash but was not shfmt-formatted; the graded output was left unchanged.
- Codex agents did not expose token totals. Durations are root wall-clock measurements and are not suitable for model-efficiency claims.
- No without-skill baseline was added.
