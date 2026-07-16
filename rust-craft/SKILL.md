---
name: rust-craft
description: Design, write, refactor, and review non-trivial Rust with deliberate Rust-native engineering judgment. Use for Rust source, Cargo workspaces, libraries and public APIs, ownership or type design, errors and resource lifecycles, async or concurrent code, unsafe or FFI boundaries, features and MSRV support, testing, or performance work. Skip mechanical prose-only edits, direct edits to generated files when a maintained source exists, and dependency updates that involve no Rust design decision.
---

# Rust Craft

Turn merely compiling Rust into intentional Rust. Treat ownership, types, APIs, lifecycles, compatibility, and evidence as design contracts—not style rules.

## Gate

Before changing code, inspect the relevant callers and:

- `Cargo.toml`, workspace inheritance, features, crate types, edition, and `rust-version`;
- `rust-toolchain*`, project lint configuration, lockfile policy, task runners, and CI;
- supported targets, runtimes, `no_std` constraints, generated boundaries, and downstream public consumers;
- the repository's existing error, testing, documentation, and dependency conventions.

Do not install tools, update dependencies, change the lockfile, raise the MSRV, or widen platform support unless the task authorizes that effect.

## Doctrine

- Model data flow and lifecycle before appeasing the borrow checker. Friction may reveal the wrong owner, boundary, or abstraction.
- Let signatures state retention: take ownership when retaining; borrow when not retaining. Make copies deliberate at the ownership boundary.
- Use types for stable, valuable invariants. Prefer concrete functions and data until real variation justifies a trait, generic, trait object, enum, or typestate.
- Treat public types, trait implementations, bounds, features, error shapes, and `Send`/`Sync` behavior as compatibility contracts.
- Treat async code as a cancellation and task-lifecycle protocol, not synchronous code with `.await` inserted.
- Give every unsafe operation a proof obligation. A safe wrapper must prevent safe callers from violating it.
- Make performance claims only from representative release-profile evidence; optimize the demonstrated bottleneck.

## Workflow

1. Establish the project contract and the behavior to preserve or introduce.
2. Trace ownership, mutation, failure, cancellation, resource cleanup, and public compatibility across the affected boundary.
3. Use the router below and load only the references needed for those decisions.
4. Choose the simplest design that satisfies the contract; state material tradeoffs instead of applying universal slogans.
5. Change the common implementation point. Avoid speculative abstraction, blanket cloning, and unrelated cleanup.
6. Validate the actual promise: affected package and targets first, then each supported feature, toolchain, platform, or public surface touched.

| Decision surface | Read |
| --- | --- |
| Workspace, crate kind, features, MSRV, editions, targets, Cargo validation | [`references/project-contract.md`](references/project-contract.md) |
| Ownership, borrowing, cloning, representation, interior mutability | [`references/ownership-and-data-modeling.md`](references/ownership-and-data-modeling.md) |
| Traits, generics, `dyn`, coherence, public API evolution, SemVer | [`references/traits-and-public-apis.md`](references/traits-and-public-apis.md) |
| `Result`, panic, error boundaries, `Drop`, explicit shutdown | [`references/errors-and-resource-lifecycle.md`](references/errors-and-resource-lifecycle.md) |
| Futures, tasks, cancellation, `Send`/`Sync`, locks, channels | [`references/async-and-concurrency.md`](references/async-and-concurrency.md) |
| Unsafe code, raw pointers, layout, FFI, unwind boundaries | [`references/unsafe-and-ffi.md`](references/unsafe-and-ffi.md) |
| Allocation, dispatch, layout, throughput, latency, benchmarks | [`references/performance.md`](references/performance.md) |
| Tests, doctests, compile-fail checks, Miri, fuzzing, evidence | [`references/testing-and-evidence.md`](references/testing-and-evidence.md) |

## Operating Modes

- **Write or fix:** implement the smallest coherent design and run directly affected checks.
- **Design:** identify owners, invariants, API commitments, failure and lifecycle behavior, compatibility risks, and a validation plan.
- **Review:** stay read-only. Report actionable findings by severity with location, violated contract, consequence, and minimum correction. Omit taste-only rewrites.

## Evidence Contract

Use project-native commands before generic Cargo invocations. Start narrow, then cover only the matrix the change promises. Formatting, compilation, Clippy, tests, docs, feature combinations, MSRV checks, cross-target builds, Miri, fuzzing, and benchmarks prove different things; no single command is a universal gate.

Prefer current local code and CI over this skill. For version-sensitive facts, check the supported toolchain's official Rust or Cargo documentation rather than assuming today's stable behavior applies.
