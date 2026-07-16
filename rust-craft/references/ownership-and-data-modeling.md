# Ownership and Data Modeling

## Required Decisions

- Name the long-lived owner of each value and the points where ownership moves, is shared, or is copied.
- If a callee retains a value, accepts it into storage, or transfers it onward, accept ownership when practical. The caller can move or deliberately copy.
- If a callee only observes or temporarily mutates a value, borrow it. Accept the least specific useful view such as `&str` or `&[T]` when that is the real contract.
- Make each clone explainable: cheap shared-handle clone, snapshot, ownership transfer substitute, or unavoidable boundary copy. Do not hide a deep clone inside an API that inevitably retains data.
- Prefer returning owned output over tying unrelated input and output lifetimes together merely to avoid allocation.

## Modeling Choices

- Use a struct with private fields when construction must preserve invariants or representation may evolve.
- Use a newtype for domain identity, units, validated values, trait-control, or coherence—not to rename every primitive.
- Use an enum for a closed set of meaningful alternatives or to eliminate invalid flag combinations. Keep booleans when they remain independent and clear.
- Use typestate only when illegal transitions are important, common, and worth the generic and API cost. Runtime validation is often clearer for dynamic workflows.
- Use `Cow` only when the operation genuinely has common borrowed and owned paths. It is not a default answer to uncertain ownership.
- Use `Arc` for shared ownership across lifetimes or threads. It does not provide interior mutability; choose immutability, a lock, atomics, or single-owner messaging separately.
- Choose `Cell`, `RefCell`, locks, or atomics only after stating the mutation, thread, blocking, and failure semantics they introduce.

## Lifetime Discipline

- Lifetimes describe relationships; they do not extend how long data lives.
- Avoid adding one lifetime parameter to unrelated borrows. It can over-constrain callers.
- Prefer ownership or a redesigned boundary when references would escape their natural owner or make the public API infectious.
- `'static` can mean a reference valid for the whole program or an owned value with no non-static borrows. It does not mean the value is immortal.

## Failure Modes

- Adding `.clone()` only because the compiler rejected the first ownership design.
- Requiring `&Vec<T>`, `&String`, or another concrete container when only its view is used.
- Borrowing an input and cloning it internally even though the operation necessarily consumes or stores it.
- Wrapping everything in `Arc<Mutex<_>>` before identifying owners and concurrency.
- Encoding every possible workflow state in types and exporting the complexity to callers.
- Using `Deref` as general delegation rather than smart-pointer behavior.

## Official Sources

- [Rust API Guidelines: caller controls copying and placement](https://rust-lang.github.io/api-guidelines/flexibility.html)
- [Rust API Guidelines: type safety](https://rust-lang.github.io/api-guidelines/type-safety.html)
- [Rust API Guidelines: future proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html)
- [The Rust Book: understanding ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [`Arc` documentation](https://doc.rust-lang.org/std/sync/struct.Arc.html)
