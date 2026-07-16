# Unsafe and FFI

## Proof Contract

Before each unsafe operation, establish:

1. the exact operation requiring unsafe;
2. every validity, provenance, aliasing, alignment, initialization, lifetime, thread, layout, and unwind precondition it relies on;
3. the local evidence that establishes those preconditions;
4. the invariant safe surrounding code must preserve afterward.

An unsafe block transfers proof responsibility; it does not relax Rust's validity rules. An `unsafe fn` states obligations for callers, while explicit unsafe blocks inside it discharge the implementation's own obligations.

## Boundary Design

- Keep unsafe code as small as the proof remains coherent, not mechanically one expression at a time.
- Keep invariant-bearing state private. Audit every safe mutation path because unsoundness is non-local.
- Expose a safe wrapper only when no safe caller can trigger undefined behavior. Otherwise keep the API unsafe and document its `# Safety` contract.
- Put a `SAFETY` explanation adjacent to the unsafe operation and make it specific enough to review after refactoring.
- Never assert or implement `Send` or `Sync` for foreign or raw-pointer-backed state without an explicit threading guarantee.

## Raw Data and Layout

- Validate nullability, length, alignment, initialization, allocation origin, lifetime, and mutability before creating references or slices from raw pointers.
- Preserve aliasing and provenance; an integer that happens to contain an address is not automatically a valid reference source.
- Use only documented layout guarantees. Default Rust representation has few stable guarantees and can change between compilations.
- `#[repr(C)]` gives the annotated item a documented C-oriented representation; it does not by itself validate enum discriminants, references or pointers, ownership, nested field ABIs, or invalid bit patterns.
- Avoid references to potentially unaligned packed fields. Use raw pointers and unaligned operations where the contract requires packed data.

## FFI Contract

- Verify the foreign ABI, exact signatures, integer widths, ownership transfer, allocation/free pairing, buffer capacity and returned length, string encoding, error channel, callbacks, thread affinity, and library lifetime.
- Model returned handles and allocations with a guard that frees exactly once, including partial failure paths.
- Convert foreign status into typed Rust errors before exposing a safe API.
- Prevent Rust unwinding from crossing a non-unwind ABI. Use an unwind ABI only when both sides explicitly support that contract; `catch_unwind` does not catch every possible failure mode.
- Treat build/link configuration, symbol availability, and target ABI as part of the tested boundary.

## Validation

- Test null, zero-length, maximum-length, partial-write, invalid-status, double-close prevention, callback failure, and ownership-transfer paths relevant to the API.
- Use Miri for executed Rust unsafe paths when supported. Add sanitizers, fuzzing, or platform integration tests when the project already supports them or the boundary risk justifies them.
- None of these tools proves soundness or covers unexecuted paths. Keep the proof reviewable independently.

## Failure Modes

- Adding an unsafe block because the compiler cannot prove a convenience path, without recording the missing proof.
- Creating a slice before validating the pointer and its length.
- Assuming `repr(C)` alone makes an arbitrary Rust type a valid C ABI type.
- Letting a foreign pointer escape the lifetime of its owner or library.
- Marking a wrapper `Send`/`Sync` because raw pointers prevented automatic implementation.
- Calling Miri success a soundness proof.

## Official Sources

- [Rust Reference: unsafe keyword](https://doc.rust-lang.org/reference/unsafe-keyword.html)
- [Rust Reference: undefined behavior](https://doc.rust-lang.org/reference/behavior-considered-undefined.html)
- [Rust Reference: type layout](https://doc.rust-lang.org/reference/type-layout.html)
- [Rust Reference: external blocks](https://doc.rust-lang.org/reference/items/external-blocks.html)
- [Rust Reference: panic behavior](https://doc.rust-lang.org/reference/panic.html)
- [Rustonomicon: working with unsafe](https://doc.rust-lang.org/nomicon/working-with-unsafe.html)
- [Rustonomicon: FFI](https://doc.rust-lang.org/nomicon/ffi.html)
- [Miri](https://github.com/rust-lang/miri/)
