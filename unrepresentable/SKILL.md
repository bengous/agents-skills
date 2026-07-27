---
name: unrepresentable
description: |
  Use when designing a new type or public signature in Rust, TypeScript, or
  Go whose data has more than one legitimate shape: correlated bool/optional
  fields, primitives carrying domain rules (email, money, IDs), or operations
  valid only in a specific order (connect before send). Makes invalid states
  unrepresentable: state enums, newtypes, typestate. NOT for reviewing
  existing code or modeling behavior over time (events, transitions, guards).
---

# Unrepresentable

Make invalid states unrepresentable: move correctness from runtime checks
into type design, so the compiler rejects bad states instead of code
detecting them at runtime. Enums delete impossible states, newtypes delete
mixed-up values, typestate deletes out-of-order calls.

This deletes precondition checks, never error handling. The operation itself
can still fail — a function whose arguments are all unforgeable still returns
a result.

## Agent contract

At a type-design moment (new type, new public signature, new module
boundary), before writing the implementation:

1. List the states, values, and orderings the data can take. State, per type
   you design, which of the three levels you applied or skipped, and why —
   that statement is the skill's reviewable output.
2. Run the three levels below; apply a level only when its signal is
   present, and skip it when its restraint criterion applies.
3. Load the one reference matching the stack being touched —
   `references/typescript.md`, `references/rust.md`, or `references/go.md`.
   Do not load the others.
4. Inspect the repo first; reuse existing brand/newtype/state helpers. Never
   add a dependency (Effect, a decimal library, a linter) because this skill
   triggered.
5. Shape only the types the current task touches. Never refactor
   surrounding code to these patterns outside the task's scope.

Boundary with the state-machine skill: state-machine models behavior over
time (events, transitions, guards); this skill shapes data types. When both
apply, model the behavior with state-machine first, then encode the
resulting states here.

## Level 1 — enums delete impossible states

- Signal: one type holds two or more correlated optional/boolean fields
  (`paid` + optional `receipt` + optional `error`) and some combinations
  are meaningless.
- Move: one variant per legitimate state; each datum lives only inside the
  variant it belongs to; consume with exhaustive matching so adding a state
  breaks every consumption site at compile time (where the language checks
  exhaustiveness — see the stack reference).
- Restraint: two independent booleans that genuinely vary freely are not a
  state machine; leave them alone.

## Level 2 — newtypes delete mixed-up values

- Signal: a primitive whose value has domain rules (email, money, non-empty
  name, ID) crosses at least two function boundaries; or two same-typed
  values can be confused at a call site.
- Move: wrap the primitive and keep the raw value private. First try to pick
  a representation that cannot hold the invalid value at all — an unsigned
  type deletes a positivity check outright, with nothing left to validate.
  Only when the representation cannot carry the invariant does the wrapper
  need a single fallible constructor. Parse, don't validate: past the
  boundary the type is the proof — no re-checking inside. For same-type role
  confusion (`transfer(from, to)`), one shared newtype is not enough: use two
  role types or a struct argument with named fields.
- Money: never a binary float; integer minor units at the currency's
  exponent (not always 2) plus an explicit currency, and unsigned when the
  domain has no credits.
- Restraint: wrap only if the value has a rule a constructor can enforce, or
  is confusable with another same-typed value at a call site. A newtype
  whose constructor has no validation logic buys nothing — leave it a
  primitive. If wrapping forces conversions at more than ~3 call sites
  without removing a check, it is not paying for itself.

## Level 3 — typestate deletes out-of-order calls

- Signal: operations are valid only in a specific order (connect before
  send, begin before commit, build before run) and an out-of-order call is
  currently a runtime panic or error.
- Move: encode the state in the type (marker types, state-generic struct,
  or one distinct type per state); transitions consume or replace the old
  state. An out-of-order call becomes a compile error where the language
  can enforce it — Rust fully, TypeScript structurally, Go only partially;
  keep the runtime guard where it cannot (see the stack reference).
- Restraint: typestate has real cost — generics propagate to every
  signature holding the value, and ergonomics suffer. Reserve it for
  genuine protocols: connections, transactions, builders, handshakes. A
  two-state flag checked in one place does not justify it.

## When NOT to apply

- Already-written code: this skill shapes new designs; it is not a review
  or refactoring mandate.
- Throwaway scripts and prototypes.
- Single boolean toggle with no correlated data.
- Behavior modeling — events, transitions, guards — belongs to the
  state-machine skill.
- When the language cannot express the guarantee, prefer the honest weaker
  pattern over ceremonial imitation — `references/go.md` is explicit about
  what Go can and cannot enforce.
