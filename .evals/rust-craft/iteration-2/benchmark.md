# rust-craft capability benchmark — iteration 2

## Verdict

The capability claim is not demonstrated. No skill and Apollo passed all three tasks; `rust-craft` passed two.

| Configuration | Task passes | Critical assertions |
|---|---:|---:|
| No skill | 3/3 | 17/17 |
| Apollo | 3/3 | 17/17 |
| rust-craft | 2/3 | 16/17 |

All nine executor trials completed and their native project gates passed on independent reruns.

## Difference

Release-contract discovery and cancel-safe flush remained saturated: every configuration passed.

The FFI task discriminated against `rust-craft`. Its solution caught the callback panic, but `drain` checked the foreign status first. When a conforming grader mock both invoked a panicking callback and returned status 7, it produced `ForeignStatus(7)` instead of the required `CallbackPanicked`. No skill and Apollo checked the panic state first and passed.

## Limits

- One run per task/configuration; no reliability claim.
- The no-skill condition is an explicit prohibition because global skills cannot be physically removed by this harness.
- Effective model identities, traces, and token counts were unavailable.
- Async ran on Rust 1.96; its declared Rust 1.85 MSRV was not executed.
- The reference solution did not cover the combined panic-plus-status case. The two passing candidates prove the task is solvable, but preflight coverage was incomplete.

Do not rewrite the skill from this single run. First decide the API's error-precedence contract, encode it in the deterministic fixture, then repeat the FFI comparison. Release and async now belong in regression coverage.
