# Types and invariants

Read when designing data shapes, public signatures or order-dependent operations.

Identify invalid combinations or confusable roles first. Choose a representation
excluding them where the language permits. Keep runtime handling for external
errors and guarantees the type system cannot enforce.

| Problem | Useful representation | Restraint |
|---|---|---|
| Correlated flags and optional data | Tagged union/enum with data in its valid variant | Independent booleans do not need a state machine. |
| Values with domain constraints | Validated constructor and opaque/newtype value | Match the actual rule; do not invent validation requirements. |
| Same-shaped values with different roles | Distinct ID types or named parameters | Role separation can help without runtime validation. |
| Operations legal only in certain states | Typestate or state-specific capability | Consider aliasing, resource lifetime and language guarantees. |

Illustrative TypeScript:

```ts
type Payment =
  | { status: "pending" }
  | { status: "captured"; transactionId: string }
  | { status: "failed"; reason: string };
```

A captured payment requires its transaction identifier in this representation.
An exhaustive consumer can flag newly introduced states. This does not prove
the identifier exists at the provider or validate JSON received over a network.

Parse external input into the internal representation. Preserve the result of
validation in the type instead of discarding it and repeating the check
throughout the application. Reuse project parsing and branding conventions
rather than introducing a new library for this pattern.

Keep guarantees precise:

- TypeScript aliases do not create nominal distinctions; assertions and `any`
  bypass brands. Deserialization re-enters through an appropriate parser.
- An unsigned integer excludes negatives, not zero. Strict positivity needs an
  additional representation or check.
- `UserId`/`OrderId` can prevent interchange even when constructors accept all
  underlying values. A single shared `Id` type cannot distinguish the roles.
- Consuming transitions depend on language semantics. TypeScript cannot prevent
  reuse of an old alias as Rust move semantics can in an appropriate API.
- Constraints on concurrent persisted data may require database checks or
  transactions even when in-memory values are individually well typed.

Use a compiler check showing an invalid construction is rejected when useful to
verify the design. Avoid type machinery whose complexity outweighs the actual
mistake it prevents.

## Sources

Reviewed 2026-09-07. Language caveats and examples are synthesis; consult current
language documentation for the exact mechanism being implemented.

- [King: Parse, don't validate, 2019](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)
  — retaining knowledge in the representation after parsing.
