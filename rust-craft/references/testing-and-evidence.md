# Testing and Evidence

## Match Evidence to the Claim

- Unit tests exercise local branches and invariants; integration tests exercise the public crate surface and cross-module behavior.
- Doctests demonstrate supported public usage and compile with the documented API. They are executable documentation, not a substitute for behavioral coverage.
- Compile-fail tests verify rejected type/API usage. Keep diagnostics and compiler-version sensitivity narrow; `compile_fail` doctests can accidentally pass only because of an unrelated error.
- Property tests suit broad input spaces with stable invariants. Fuzzing suits parser, protocol, deserialization, and unsafe boundaries that benefit from adversarial inputs.
- Miri detects some undefined behavior in executed paths. Sanitizers detect supported runtime violations on supported targets. Neither is a proof of soundness.
- Benchmarks answer measured performance questions; they do not establish correctness.

## Required Coverage

- Test externally meaningful behavior, invariants, error variants, side effects, and cleanup—not private implementation shape.
- Cover every modified feature and target configuration promised by the project. Default host tests do not cover disabled `cfg` paths.
- Test public examples, package contents, or downstream compilation when those are part of the change.
- For concurrency, cover cancellation, failure, closure, backpressure, and shutdown with explicit coordination where possible.
- For unsafe/FFI, combine proof review with boundary cases and the strongest project-supported dynamic tools.
- For an MSRV claim, run a supported toolchain rather than only configuring a lint.

## Test Design

- Name the behavior and condition clearly. Use as many assertions as needed to prove one coherent behavior.
- Keep tests deterministic and bounded. Prefer controlled clocks, barriers, injected interfaces, and temporary directories over sleeps, live services, or global machine state.
- Test through the public seam unless a private invariant is otherwise expensive or ambiguous to diagnose.
- Preserve a regression test that fails for the original bug before claiming the fix.
- Avoid new test frameworks or snapshot dependencies when the existing harness expresses the behavior clearly.

## Evidence Reporting

- State exactly which commands and configurations ran, their result, and what remained untested.
- Do not imply coverage of a workspace, target, feature set, MSRV, race, or unsafe path that was not exercised.
- Distinguish static analysis, successful compilation, behavioral execution, compatibility checks, and measurement.

## Failure Modes

- “One assertion per test” splitting a coherent behavior into noisy setup duplication.
- Snapshotting unstable output when structural assertions express the contract.
- Running only `cargo test --all-features` in a crate whose important path is `--no-default-features` or a specific feature combination.
- Treating Clippy, Miri, fuzzing, or code coverage as a universal quality score.
- Using `#[ignore]` or retries to hide nondeterministic concurrency.

## Official Sources

- [Cargo test](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [Cargo check](https://doc.rust-lang.org/cargo/commands/cargo-check.html)
- [Cargo doc](https://doc.rust-lang.org/cargo/commands/cargo-doc.html)
- [rustdoc documentation tests](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html)
- [Clippy usage](https://doc.rust-lang.org/stable/clippy/usage.html)
- [Cargo package](https://doc.rust-lang.org/cargo/commands/cargo-package.html)
- [Miri](https://github.com/rust-lang/miri/)
