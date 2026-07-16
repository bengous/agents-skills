# Performance

## Evidence First

- Define the user-visible or system metric, representative workload, target hardware, profile, and baseline before optimizing.
- Measure optimized release or benchmark profiles. Debug behavior is not performance evidence.
- Reproduce the bottleneck and retain enough benchmark detail for another agent to compare the same thing.
- Report measured observations, not invented percentages or predictions. Separate CPU time, wall time, allocation, throughput, latency distribution, binary size, and compile time.

## Decision Order

1. Verify the algorithm, data movement, I/O pattern, batching, and concurrency model.
2. Inspect allocation, cloning, representation, cache behavior, and synchronization in the measured hot path.
3. Evaluate dispatch, inlining, bounds checks, layout, and code generation only when evidence points there.
4. Use unsafe or unchecked operations only when the remaining gain matters and the proof plus regression coverage are acceptable.

## Rust-Specific Tradeoffs

- Distinguish cheap handle clones from deep data clones. Count or profile the relevant allocation and copy rather than banning `.clone()` syntactically.
- Iterators and loops can both optimize well. Choose the clearest semantic form, then inspect generated behavior only for a demonstrated hot path.
- Generics can create monomorphized code and inlining opportunities while increasing code size, compile time, and instruction-cache pressure.
- Trait objects add indirection and can reduce inlining, but may reduce code size and provide the correct runtime boundary.
- Stack versus heap is not governed by a universal byte threshold. Consider target stack limits, recursion, move cost, alignment, object lifetime, and layout.
- `opt-level = 3` is not guaranteed faster than `2`; profile settings are empirical and compiler-version-sensitive.
- `target-cpu = "native"` can improve a local benchmark while producing a non-portable artifact. Match production deployment.

## Validation

- Run correctness tests before and after the optimization.
- Benchmark enough samples under controlled conditions to distinguish noise, but do not fabricate significance or stability.
- Check relevant counter-metrics: memory, tail latency, binary size, compile time, fairness, or power where the tradeoff matters.
- Keep benchmark dependencies and harnesses consistent with the repository; add a tool only when it answers a real question.

## Failure Modes

- Calling code faster because it uses iterators, generics, fewer visible allocations, or unsafe operations.
- Benchmarking a debug build or an unrealistic micro-operation and generalizing to production.
- Boxing a value because an arbitrary size threshold was crossed.
- Removing a deliberate clone and creating a longer borrow, lock hold, or API constraint without measuring the whole effect.
- Shipping host-specific code generation from a developer machine as a portable release.

## Official Sources

- [Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [rustc code generation options](https://doc.rust-lang.org/rustc/codegen-options/index.html)
- [Rust API Guidelines: flexibility](https://rust-lang.github.io/api-guidelines/flexibility.html)
- [Rust API Guidelines: dependability](https://rust-lang.github.io/api-guidelines/dependability.html)
