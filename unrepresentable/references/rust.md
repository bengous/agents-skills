# Rust — making invalid states unrepresentable

Load this only when the task touches Rust.

```rust
// Before: three runtime guards, three chances to forget one.
fn send_invoice(conn: &Connection, email: &str, amount: f64) -> Result<Invoice, Error> {
    if !conn.is_open { return Err(Error::NotConnected); }
    if email.is_empty() { return Err(Error::BadEmail); }
    if amount <= 0.0 { return Err(Error::InvalidAmount); }
    // ...only now can we do the real work
}

// After: no guard left, because no argument can hold a bad state.
fn send_invoice(conn: &Connection<Open>, email: Email, amount: Money) -> Result<Invoice, Error> {
    // ...just do the real work
}
```

The `Result` stays — the real work can still fail. What disappeared is
precondition checking, not error handling. Each section below deletes one
guard.

## State enums

One variant per state; data lives in its variant; `match` is exhaustive by
default, so adding a variant breaks every consumption site at compile time.

```rust
enum Invoice {
    Pending,
    Paid(Receipt),
    Failed(String),
}

match invoice {
    Invoice::Pending => wait(),
    Invoice::Paid(receipt) => store(receipt),
    Invoice::Failed(err) => log(err),
}
```

Adding `Refunded` to the enum turns every unpatched `match` into
`error[E0004]: non-exhaustive patterns`. Never add a `_ => {}` arm to your own
enums to silence it: that trades away the one guarantee it was giving you.

## Newtypes

Two shapes, and the difference between them is the whole lesson.

**When the representation itself can hold the invariant, the constructor
validates nothing.** An amount cannot be negative if its type has no negative
values:

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Currency { Eur, Usd, Jpy }

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Money { minor_units: u64, currency: Currency }

impl Money {
    pub fn new(minor_units: u64, currency: Currency) -> Self {
        Money { minor_units, currency }
    }

    pub fn try_add(self, other: Money) -> Result<Money, CurrencyMismatch> {
        /* same-currency check, then add */
    }
}
```

`u64` deletes `amount <= 0.0` outright — there is no check left to write
anywhere, which is why `new` is infallible. Minor units sit at the currency's
exponent, not always 2 (JPY 0, KWD 3), so `minor_units` is the honest field
name and `cents` is not. If the domain genuinely needs credits, move to `i64`
with a fallible constructor; never keep an unsigned type plus a sign flag.

Prefer the inherent `try_add` over `impl Add` with a `Result` output: that
`Add` poisons `+=` (`AddAssign`) and `Sum`.

**When the representation cannot hold the invariant, one fallible constructor
carries it.** No `String` excludes non-emails, so validation happens once, on
the way in:

```rust
pub struct Email(String);

impl TryFrom<String> for Email {
    type Error = EmailError;
    fn try_from(raw: String) -> Result<Self, Self::Error> {
        // Illustrative check — the single entry point is the pattern, not the
        // predicate.
        if raw.contains('@') { Ok(Email(raw)) } else { Err(EmailError::Invalid) }
    }
}

impl Email {
    pub fn as_str(&self) -> &str { &self.0 }
}
```

Keep the inner field private, expose read access via a method. Functions past
the boundary take `Email`, never `&str` re-validated.

Deserialization bypasses the constructor: a bare `#[derive(Deserialize)]` on a
validated newtype is a defect. Route serde through it with
`#[serde(try_from = "String")]`, plus `#[serde(into = "String")]` on the write
side when needed.

## Typestate

Marker types + state-generic struct; transitions consume `self`:

```rust
use std::marker::PhantomData;

pub struct Open;
pub struct Closed;

pub struct Connection<State> {
    stream: TcpStream,
    _state: PhantomData<State>,
}

impl Connection<Closed> {
    /// Hands the closed connection back on failure so the caller can retry.
    pub fn connect(self) -> Result<Connection<Open>, (Connection<Closed>, ConnectError)> {
        /* ... */
    }
}

impl Connection<Open> {
    pub fn send(&mut self, data: &[u8]) -> Result<(), SendError> { /* ... */ }
}
```

`send` on a closed connection is `error[E0599]: no method named 'send' found
for 'Connection<Closed>'` — the call cannot be written, so the `is_open` field
and its `panic!` both disappear. `connect` consuming `self` makes the stale
closed handle unusable: that is the guarantee TypeScript and Go cannot give.

## Limits

- Typestate generics propagate: everything storing a `Connection<S>` becomes
  generic or must pick a state. For long-lived heterogeneous storage, an enum
  of states (`enum Conn { Open(Connection<Open>), Closed(Connection<Closed>) }`)
  is often the pragmatic wrapper.
- Consuming transitions only protect if the type is not `Clone`/`Copy`;
  deriving either silently restores the stale handle.
- Inside the defining module, `Email(raw)` bypasses `TryFrom` — the privacy
  boundary is the module, so keep newtypes in their own module.
- If the states share most behavior and only one method is state-bound, a
  runtime check with a clear error may cost less than the generic spread.
  Restraint criteria live in `../SKILL.md`.
