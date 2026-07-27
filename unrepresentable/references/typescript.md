# TypeScript — making invalid states unrepresentable

Load this only when the task touches TypeScript.

## State enums → discriminated unions

One variant per state; data lives in its variant. Discriminate on a literal
`kind`/`_tag` field.

```typescript
type Invoice =
  | { kind: "pending" }
  | { kind: "paid"; receipt: Receipt }
  | { kind: "failed"; error: string };
```

Exhaustiveness: end every `switch` with a `never` arm so adding a variant
breaks compilation at each consumption site.

```typescript
function handle(invoice: Invoice): void {
  switch (invoice.kind) {
    case "pending": return wait();
    case "paid": return store(invoice.receipt);
    case "failed": return log(invoice.error);
    default: {
      const unreachable: never = invoice;
      throw new Error(`unhandled state: ${JSON.stringify(unreachable)}`);
    }
  }
}
```

In Effect codebases, prefer `Data.TaggedEnum` or tagged classes with
`Match.exhaustive` over hand-rolled unions when the file already uses Effect.

When deriving a literal union from a value table, use `as const` so the
compiler keeps the literals instead of widening to `string`:

```typescript
const CURRENCIES = ["EUR", "USD"] as const;
type Currency = (typeof CURRENCIES)[number]; // "EUR" | "USD"
```

## Newtypes → branded types

Structural typing means `type Email = string` is an alias, not a guarantee.
Brand it, and return a result — not `null` — so the failure carries its reason:

```typescript
type ParseResult<T> = { ok: true; value: T } | { ok: false; error: string };

declare const EmailBrand: unique symbol;
type Email = string & { readonly [EmailBrand]: true };

function parseEmail(raw: string): ParseResult<Email> {
  // Illustrative check — the pattern (single constructor, single cast) is the
  // point, not the exact predicate.
  return /^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(raw)
    ? { ok: true, value: raw as Email }
    : { ok: false, error: `invalid email: ${raw}` };
}
```

The `as Email` cast appears in exactly one place: the smart constructor. Any
other cast to the brand is a defect — including typing a `JSON.parse` result
as `Email`. Deserialized data re-enters through the constructor like any
other external input.

In Effect codebases use the built-ins instead of hand-rolling:

```typescript
import { Schema } from "effect";

const Email = Schema.String.pipe(
  Schema.pattern(/^[^@\s]+@[^@\s]+\.[^@\s]+$/),
  Schema.brand("Email"),
);
type Email = typeof Email.Type;
// Schema.decodeUnknown(Email) is the single boundary parser — including for
// JSON.parse output.
```

Parse, don't validate: decode once at the boundary (HTTP handler, file read,
form submit); everything past the boundary takes `Email`, never `string`.

Money is the same pattern, not a separate topic. TypeScript has no unsigned
number, so unlike Rust it cannot push the "amount is positive" invariant into
the representation — the constructor has to carry it, and the type has to be
branded or the object literal walks straight past it:

```typescript
declare const MoneyBrand: unique symbol;
type Money = {
  readonly minorUnits: number;
  readonly currency: Currency;
} & { readonly [MoneyBrand]: true };

function money(minorUnits: number, currency: Currency): ParseResult<Money> {
  return Number.isSafeInteger(minorUnits) && minorUnits >= 0
    ? { ok: true, value: { minorUnits, currency } as Money }
    : { ok: false, error: `invalid amount: ${minorUnits}` };
}
```

Never a binary float: minor units are integers at the currency's exponent,
not always 2 (JPY 0, KWD 3), which is why the field is `minorUnits` and not
`cents`. `number` is exact for integers up to 2^53; `bigint` reaches further
but `JSON.stringify` throws on it, so never put a `bigint` in a type that
crosses a serialization boundary without a custom codec.

## Typestate

Required phantom keyed by a `unique symbol`; operations exist only on the
right instantiation:

```typescript
declare const state: unique symbol;

interface Connection<S extends "open" | "closed"> {
  readonly [state]: S;
  readonly socket: Socket;
}

declare function connect(conn: Connection<"closed">): Connection<"open">;
declare function send(conn: Connection<"open">, data: string): void;
```

`send` on a `Connection<"closed">` is a compile error, and no object literal
can be forged into a `Connection<"open">`: `[state]` is required and its key
is not nameable outside the declaring module. Do NOT model the phantom as an
optional property (`_phantom?: S`): `{}` would satisfy every state.

## Limits

- Brands and phantom states are compile-time only; `as any` defeats them. They
  are guardrails for honest code, not security boundaries.
- TypeScript cannot consume values (no move semantics): after
  `connect(closed)`, the old `closed` binding still exists. When stale-handle
  reuse is a real hazard, make the transition API take a callback or document
  the hand-off; do not pretend the type system prevents it.
- Restraint criteria live in `../SKILL.md`.
