# Rust: state machine implementations

Three idioms. The first two use only the standard language; the third
adds a crate for hierarchical machines.

## Approach 1: enum + match (the canonical functional pattern)

Idiomatic Rust. The enum is the state, `match` is the transition function.
Exhaustiveness is checked at compile time. Variant data carries
state-specific fields.

```rust
enum State {
    Idle,
    Loading { attempt: u8 },
    Success { data: String },
    Error   { error: String, attempt: u8 },
    GiveUp  { last_error: String },
}

enum Event {
    Fetch,
    ResponseOk(String),
    ResponseError(String),
    Retry,
    Refetch,
}

const MAX_ATTEMPTS: u8 = 3;

fn reduce(state: State, event: Event) -> State {
    match state {
        State::Idle => match event {
            Event::Fetch => State::Loading { attempt: 1 },
            Event::ResponseOk(_)
            | Event::ResponseError(_)
            | Event::Retry
            | Event::Refetch => State::Idle,
        },
        State::Loading { attempt } => match event {
            Event::ResponseOk(data) => State::Success { data },
            Event::ResponseError(error) => State::Error { error, attempt },
            Event::Fetch | Event::Retry | Event::Refetch => State::Loading { attempt },
        },
        State::Success { data } => match event {
            Event::Refetch => State::Loading { attempt: 1 },
            Event::Fetch
            | Event::ResponseOk(_)
            | Event::ResponseError(_)
            | Event::Retry => State::Success { data },
        },
        State::Error { error, attempt } => match event {
            Event::Retry if attempt < MAX_ATTEMPTS => {
                State::Loading { attempt: attempt + 1 }
            }
            Event::Retry => State::GiveUp { last_error: error },
            Event::Fetch | Event::ResponseOk(_) | Event::ResponseError(_) | Event::Refetch => {
                State::Error { error, attempt }
            }
        },
        State::GiveUp { last_error } => match event {
            Event::Retry => State::Loading { attempt: 1 },
            Event::Fetch | Event::ResponseOk(_) | Event::ResponseError(_) | Event::Refetch => {
                State::GiveUp { last_error }
            }
        }
    }
}
```

Pros: zero dependencies, exhaustive matching catches missing transitions
at compile time, variant data localizes state-specific fields.
Cons: still flat (no hierarchy). Pattern matching gets long for big
machines.

References: [corrode.dev: Using Enums to Represent State](https://corrode.dev/blog/enums/),
[Yoshua Wuyts: State Machines II (enum-based)](https://blog.yoshuawuyts.com/state-machines-2/index).

## Approach 2: typestate (compile-time enforced transitions)

Each state is a *separate type*. Transitions are method signatures whose
return type is the next state. Calling an invalid transition is a compile
error: the method does not exist on the current type.

```rust
pub struct Idle;
pub struct Loading { attempt: u8 }
pub struct Success { pub data: String }
pub struct Error   { pub error: String, attempt: u8 }

impl Idle {
    pub fn fetch(self) -> Loading { Loading { attempt: 1 } }
}

impl Loading {
    pub fn response_ok(self, data: String) -> Success {
        Success { data }
    }
    pub fn response_error(self, error: String) -> Error {
        Error { error, attempt: self.attempt }
    }
}

impl Error {
    pub fn retry(self) -> Result<Loading, GaveUp> {
        if self.attempt < 3 {
            Ok(Loading { attempt: self.attempt + 1 })
        } else {
            Err(GaveUp { last_error: self.error })
        }
    }
}

pub struct GaveUp { pub last_error: String }
```

Usage:

```rust
fn main() {
    let s: Idle = Idle;
    let s: Loading = s.fetch();
    // s.fetch();  // compile error: Loading has no method `fetch`
    let s: Success = s.response_ok("hello".into());
}
```

Pros: invalid transitions are compile errors. Self-documenting API.
Excellent for *protocols* (TCP-like handshakes, builder APIs).
Cons: less ergonomic for run-time-driven machines (event from a queue
where the next state is data-dependent). You typically wrap it in an
enum at the boundary, which gives back the runtime dispatch.

References: [Hoverbear, *Pretty State Machine Patterns in Rust*](https://hoverbear.org/blog/rust-state-machine-pattern/),
[Yoshua Wuyts: State Machines III (typestate)](https://blog.yoshuawuyts.com/state-machines-3),
[The Embedded Rust Book: typestate](https://docs.rust-embedded.org/book/static-guarantees/typestate-programming.html).

When to use which:

- **Protocol / builder APIs**: typestate. The order is known statically.
- **Event-loop FSMs**: enum + match. The order comes from a queue at runtime.

## Approach 3: `statig` crate (hierarchical state machines)

For HSMs (hierarchy, entry/exit, state-local storage), use the
[statig](https://github.com/mdeloof/statig) crate. Designed for embedded
and event-driven systems.

```toml
# Cargo.toml
[dependencies]
statig = "0.4"
```

```rust
use statig::prelude::*;

#[derive(Default)]
struct Fetch;

pub enum Event {
    Start,
    ResponseOk(String),
    ResponseError(String),
    Retry,
}

#[state_machine(initial = "State::idle()")]
impl Fetch {
    #[state]
    fn idle(event: &Event) -> Outcome<State> {
        match event {
            Event::Start => Transition(State::loading(1)),
            _ => Super,
        }
    }

    #[state]
    fn loading(attempt: &u8, event: &Event) -> Outcome<State> {
        match event {
            Event::ResponseOk(data) => Transition(State::success(data.clone())),
            Event::ResponseError(err) => {
                Transition(State::error(err.clone(), *attempt))
            }
            _ => Super,
        }
    }

    #[state]
    fn success(_data: &String, event: &Event) -> Outcome<State> {
        match event {
            Event::Start => Transition(State::loading(1)),  // refetch
            _ => Super,
        }
    }

    #[state]
    #[allow(clippy::ptr_arg)] // statig state data is passed as references to stored fields.
    fn error(error: &String, attempt: &u8, event: &Event) -> Outcome<State> {
        match event {
            Event::Retry if *attempt < 3 => Transition(State::loading(*attempt + 1)),
            Event::Retry => Transition(State::give_up(error.clone())),
            _ => Super,
        }
    }

    #[state]
    fn give_up(_last_error: &String, event: &Event) -> Outcome<State> {
        match event {
            Event::Retry => Transition(State::loading(1)),
            _ => Super,
        }
    }
}

fn main() {
    let mut sm = Fetch.state_machine();
    sm.handle(&Event::Start);
    sm.handle(&Event::ResponseOk("ok".into()));
}
```

Pros: hierarchical states, entry/exit actions, state-local storage,
`#[superstate]` for shared transition logic. Suitable for `no_std`
embedded code.
Cons: macro-heavy, learning curve.

References: [statig README](https://github.com/mdeloof/statig),
[docs.rs/statig](https://docs.rs/statig/).

## When to graduate

- Small machines with no hierarchy: enum + match.
- Static / compile-time-knowable order: typestate.
- > 10 states or hierarchy required: `statig` (or hand-rolled HSM, or
  another crate like `machine` or `sm`).

## Sources

- Hoverbear, [*Pretty State Machine Patterns in Rust*](https://hoverbear.org/blog/rust-state-machine-pattern/)
- Yoshua Wuyts, [*State Machines* (part 1)](https://blog.yoshuawuyts.com/state-machines/),
  [part 2](https://blog.yoshuawuyts.com/state-machines-2/index),
  [part 3](https://blog.yoshuawuyts.com/state-machines-3)
- [corrode.dev: Using Enums to Represent State](https://corrode.dev/blog/enums/)
- [The Embedded Rust Book: Typestate Programming](https://docs.rust-embedded.org/book/static-guarantees/typestate-programming.html)
- [statig (GitHub)](https://github.com/mdeloof/statig)
- [statig (docs.rs)](https://docs.rs/statig/)
- [DEV: Challenging the State Pattern with Rust Enums](https://dev.to/digclo/state-pattern-with-rust-enums-61g)
