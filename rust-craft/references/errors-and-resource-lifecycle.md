# Errors and Resource Lifecycle

## Required Decisions

- Separate expected operational failure, invalid caller input, violated internal invariant, and unrecoverable process policy.
- Return `Result` for failures callers may handle. Panic for programmer errors or proven invariant violations, not ordinary external input or environmental failure.
- Define errors at the boundary where callers make decisions. Preserve structured causes and context until presentation; do not stringify early.
- Include the failed operation and relevant non-secret identity when propagating an error. Avoid context-free `map_err` and silent fallback.
- Test error variants, sources, and side effects structurally. Assert display text only when wording is a contract.

## Error Type Choices

- Follow the project's dependency and `no_std` policy. No crate is universally required.
- A public library usually needs a stable, meaningful error type implementing `Debug`, `Display`, and `Error`; consider `Send + Sync` where errors cross thread or task boundaries.
- An application or internal orchestration layer may erase heterogeneous errors when callers need context rather than exhaustive matching.
- Never use `()` as a public error merely to satisfy `Result`.
- `unwrap` or `expect` can be correct for a locally proven invariant. An `expect` message should state why success is guaranteed. They are not appropriate for unvalidated production inputs.
- `catch_unwind` is an unwind-boundary mechanism, not a replacement for `Result` and not a guarantee that every panic is catchable.

## Cleanup and Completion

- Make resource ownership and cleanup idempotence explicit. Destructors may run during unwinding and partial initialization.
- `Drop` cannot report an error and should not perform surprising blocking work. Provide `finish`, `flush`, `close`, or `shutdown` returning `Result` when completion can fail.
- Decide what happens if explicit completion is skipped: best-effort cleanup, discard, warning under existing policy, or a documented invariant.
- Do not rely on `Drop` for async completion. Own and await the shutdown path explicitly.
- Preserve the primary failure when cleanup also fails; expose aggregate or secondary context only when callers can act on it.

## Public Documentation

- Document error conditions for public fallible functions, panic conditions for public functions that may panic, and caller obligations for unsafe functions.
- Keep examples realistic and propagating errors with `?` where failure is part of the API.

## Failure Modes

- “Never panic,” “never unwrap,” or “always use thiserror/anyhow” as universal rules.
- Converting a source error to a string before crossing an abstraction boundary.
- Returning a giant global error enum from every crate layer.
- Hiding failure behind `Option`, a default value, logging, or a best-effort branch without a declared contract.
- Advertising durable flush or close semantics that exist only in `Drop`.

## Official Sources

- [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html)
- [`Result` documentation](https://doc.rust-lang.org/std/result/enum.Result.html)
- [`catch_unwind` documentation](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)
- [Rust API Guidelines: error interoperability](https://rust-lang.github.io/api-guidelines/interoperability.html)
- [Rust API Guidelines: dependability](https://rust-lang.github.io/api-guidelines/dependability.html)
- [Rust API Guidelines: documentation](https://rust-lang.github.io/api-guidelines/documentation.html)
