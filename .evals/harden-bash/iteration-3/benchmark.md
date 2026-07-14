# Benchmark: harden-bash (iteration 3)

**Executor**: `gpt-5.6-terra` | **Grader**: `gpt-5.6-sol` | **Date**: 2026-07-14 | **Skill commit**: `5e21244` | **Evals**: 4 | **Runs per config**: 1

## Summary

| Metric | with_skill | without_skill | Delta |
| --- | ---: | ---: | ---: |
| Assertion pass rate | 100% ± 0% | Not run | N/A |
| Root wall time | 198.52 s ± 108.00 s | Not run | N/A |
| Tokens | Unavailable | Not run | N/A |

Overall: all 28 assertions passed. Iteration 2 passed 26 of 28 with identical prompts and assertions. No baseline was run, so skill lift is not measured.

## Per-Eval Breakdown

| Eval | Iteration 3 | Iteration 2 |
| --- | ---: | ---: |
| 1. bash-untrusted-paths | 7/7 (100%) | 6/7 (85.71%) |
| 2. posix-bashism-rewrite | 7/7 (100%) | 6/7 (85.71%) |
| 3. errexit-trap-audit | 7/7 (100%) | 7/7 (100%) |
| 4. process-supervision | 7/7 (100%) | 7/7 (100%) |

## Corrected Failures

- Eval 1 tested a real argument beginning with `-` from its temporary working directory.
- Eval 2 explicitly disclosed `grep -H` and utility-specific `--` as external portability assumptions.

## Integrity And Limits

- Executor runs used fresh `gpt-5.6-terra` agents; independent graders used fresh `gpt-5.6-sol` agents.
- Prompts and assertions are byte-identical to iteration 2.
- Static preflight passed without executing embedded eval snippets.
- One run per eval is stochastic evidence, not a general 100% guarantee.
- Codex agents did not expose token totals. Durations are root wall-clock measurements and are not suitable for model-efficiency claims.
- No without-skill baseline was added.
