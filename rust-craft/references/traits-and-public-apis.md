# Traits and Public APIs

## Select the Abstraction

- Start with a concrete function or type when there is one implementation and no caller needs polymorphism.
- Use an enum when the variants form a closed set controlled by the crate and exhaustive behavior is useful.
- Use generics or `impl Trait` when callers supply statically known implementations and monomorphization is acceptable.
- Use `dyn Trait` when type erasure, heterogeneous values, runtime substitution, a stable plugin seam, or reduced monomorphization is the actual requirement.
- Use a public trait only when downstream implementation is an intended extension point. Otherwise keep it private or seal it.

Static dispatch is not universally faster, and dynamic dispatch is not inherently a design failure. Evaluate indirection, inlining, binary size, compile time, code ownership, and API evolution together.

## Trait Contracts

- Verify dyn compatibility against the supported compiler. Methods with generic type or const parameters, `Self` as a concrete parameter or return type, opaque return types, or `async fn` are not dispatchable through a trait object unless excluded with `Self: Sized` or redesigned.
- Put bounds where behavior needs them. Bounds on a public type declaration constrain every user and are difficult to relax or strengthen compatibly.
- Treat `Send`, `Sync`, `Unpin`, unwind-safety traits, and other auto-trait behavior as observable API when public values cross those boundaries.
- Implement `From` or `TryFrom`; callers receive the reciprocal `Into` or `TryInto` implementation.
- Use blanket implementations cautiously. They consume future coherence space and can overlap with downstream designs.

## Coherence and Evolution

- Rust permits an implementation only when coherence holds. For a foreign trait, the orphan rule requires an allowed local type in the impl inputs and no uncovered type parameter before the first such local type; implementations also must not overlap.
- Use a local newtype when behavior is needed for a foreign type under a foreign trait. Do not invent unsafe or global registry workarounds.
- Assume downstream crates may name, implement, match, construct, or place bounds on every public item the language permits.
- Adding a required method to a public trait breaks downstream implementations. A default can preserve implementors but may still affect trait objects, behavior, ambiguity, or object safety.
- Adding an implementation, bound, enum variant, feature, public field, or re-export can be source-breaking in some contexts. Consult Cargo's SemVer guide rather than relying on “additions are safe.”
- Add `#[non_exhaustive]`, private fields, sealing, or constructors when an API is first designed for that evolution path; adding restrictions later can itself break users.
- Evolve async public traits with special care: runtime choice, allocation, dyn use, MSRV, and language support are all part of the contract.

## Failure Modes

- Introducing a trait only to mock one local concrete dependency.
- Choosing generics solely because “zero cost” sounds faster, without considering code size or public bounds.
- Choosing `dyn` solely to avoid generics, without checking object safety or ownership.
- Publishing an open trait while expecting to add required methods in minor releases.
- Treating compilation inside the defining crate as proof of downstream compatibility.

## Official Sources

- [Rust Reference: implementations and coherence](https://doc.rust-lang.org/reference/items/implementations.html)
- [Rust Reference: dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html)
- [Cargo SemVer compatibility guide](https://doc.rust-lang.org/cargo/reference/semver.html)
- [Cargo dependency resolver](https://doc.rust-lang.org/cargo/reference/resolver.html)
- [Rust API Guidelines: future proofing](https://rust-lang.github.io/api-guidelines/future-proofing.html)
- [Rust API Guidelines: interoperability](https://rust-lang.github.io/api-guidelines/interoperability.html)
