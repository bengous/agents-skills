# Go — making invalid states unrepresentable

Load this only when the task touches Go. Go cannot express most of these
guarantees; this file states what is achievable and what is not. Prefer the
honest weaker pattern over ceremonial imitation of Rust.

## State enums

Go has no sum types and no exhaustive switch. Closest pattern: a sealed
interface — unexported marker method, one struct per state, data in its state.

```go
//sumtype:decl
type InvoiceState interface{ isInvoiceState() }

type Pending struct{}
type Paid struct{ Receipt Receipt }
type Failed struct{ Error string }

func (*Pending) isInvoiceState() {}
func (*Paid) isInvoiceState()    {}
func (*Failed) isInvoiceState()  {}
```

The unexported method stops another package from declaring a new variant, but
it does not close the set: an outside package can embed one of your variants
(`type Cancelled struct{ inv.Pending }`) and satisfy the interface through
method promotion. The seal is a convention, not a compiler guarantee — end
every type switch with a `default:` that returns an error or panics. Use
pointer receivers: with value receivers both `Paid` and `*Paid` satisfy the
interface, so every switch needs two cases per state.

Type switches are not compiler-checked for exhaustiveness. If the module
already runs a linter, `gochecksumtype` checks them, with caveats: the
interface needs the `//sumtype:decl` comment, a `default` clause counts as
exhaustive unless `default-signifies-exhaustive: false`, and under
golangci-lint it only reports switches in the package declaring the sum type.
The `exhaustive` linter does NOT apply here — it covers iota-style constants
and map keys only. Never claim compile-time exhaustiveness in Go.

What NOT to do: `iota` constants plus parallel optional fields — that is the
bool-flag explosion with extra steps.

## Newtypes

`type Email string` alone is weak: any `Email("junk")` conversion bypasses
validation. For real invariants, unexported field + validating constructor:

```go
type Email struct{ raw string }

func NewEmail(raw string) (Email, error) {
    // Illustrative check — the pattern is the point, not the predicate.
    if !strings.Contains(raw, "@") {
        return Email{}, fmt.Errorf("invalid email: %q", raw)
    }
    return Email{raw: raw}, nil
}

func (e Email) String() string { return e.raw }
```

Outside the defining package, `Email{}` zero value is still constructible —
document that the zero value is invalid, and keep boundary parsing as the
single entry point. `json.Unmarshal` cannot reach the constructor either:
implement `UnmarshalJSON` calling `NewEmail`, or decode into a raw DTO and
construct explicitly.

Defined types (`type AccountID uint64`) prevent mixing with other defined
types, but untyped constants still convert (`transfer(1, 2)` compiles), and
two parameters of the same defined type can still be swapped. For from/to
call sites, use two distinct role types or a single struct argument with
named fields.

Money is the same pattern, not a separate topic — so it gets the same
unexported fields, never exported ones:

```go
type Currency string

const (
    EUR Currency = "EUR"
    USD Currency = "USD"
)

type Money struct {
    minorUnits uint64
    currency   Currency
}

func NewMoney(minorUnits uint64, currency Currency) (Money, error) {
    if currency == "" {
        return Money{}, errors.New("money requires a currency")
    }
    return Money{minorUnits: minorUnits, currency: currency}, nil
}
```

`uint64` deletes the negative-amount check at the type level, as in Rust. The
constructor still exists because the zero value would otherwise be a
currency-less amount — that hole is Go's, not the pattern's. Never `float64`:
minor units are integers at the currency's exponent, not always 2 (JPY 0,
KWD 3), which is why the field is `minorUnits` and not `cents`.

Arithmetic helpers must reject mixed currencies with an error; Go cannot do
it at the type level without one type per currency (only worth it when the
currency set is closed and small).

## Typestate

No generics-based typestate idiom in Go worth its cost. Use one distinct type
per state and transitions as functions between them:

```go
type ClosedConn struct{ addr string }
type OpenConn struct{ conn net.Conn }

func (c ClosedConn) Connect() (OpenConn, error) { /* ... */ }
func (c OpenConn) Send(data []byte) error       { /* ... */ }
```

`Send` does not exist on `ClosedConn` — that part holds. Go cannot consume
the old value: the `ClosedConn` remains usable after `Connect`. If reuse is a
real hazard, keep the runtime guard inside `Connect` and return a clear error.

## Limits

- Zero values bypass constructors outside the package; interfaces can be nil.
  Go designs keep one validating boundary and trust it, but cannot delete all
  runtime checks the way Rust can.
- When a guarantee is not expressible, write the runtime check once, close to
  the constructor, with a clear error — not scattered at every call site.
- Restraint criteria live in `../SKILL.md`.
