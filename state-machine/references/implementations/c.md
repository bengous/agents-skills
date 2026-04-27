# C: state machine implementations

Three idioms, in order of increasing complexity. Pick the one that matches
your scale.

## Approach 1: switch-based FSM

The simplest form. Works for small machines, breaks down once transitions
become hard to scan in one switch.

```c
typedef enum { IDLE, LOADING, SUCCESS, ERROR } State;
typedef enum { FETCH, RESPONSE_OK, RESPONSE_ERROR, RETRY } Event;

static State current = IDLE;

void handle_event(Event e) {
    switch (current) {
        case IDLE:
            if (e == FETCH) current = LOADING;
            break;
        case LOADING:
            if (e == RESPONSE_OK)    current = SUCCESS;
            else if (e == RESPONSE_ERROR) current = ERROR;
            break;
        case SUCCESS:
            // terminal-ish
            break;
        case ERROR:
            if (e == RETRY) current = LOADING;
            break;
    }
}
```

Pros: zero dependencies, easy to read at small scale.
Cons: scales O(states * events), no compile-time check that all `(state,
event)` cells are covered, side effects mix with control flow.

## Approach 2: transition table

Move the transitions out of code and into data. Best for flat machines that
have outgrown one readable switch but do not need hierarchy. This is the form
most readable for non-engineers and most amenable to code generation.

```c
#include <stdbool.h>
#include <stddef.h>

typedef enum { IDLE, LOADING, SUCCESS, ERROR, STATE_COUNT } State;
typedef enum { FETCH, RESPONSE_OK, RESPONSE_ERROR, RETRY, EVENT_COUNT } Event;

typedef void (*action_fn)(void);

static void on_success(void);
static void on_error(void);

typedef struct {
    bool valid;
    State next;
    action_fn action;     /* may be NULL */
} Transition;

/* Designated initializers (C99). Missing entries default to { false, 0, NULL }. */
static const Transition table[STATE_COUNT][EVENT_COUNT] = {
    [IDLE] = {
        [FETCH] = { true, LOADING, NULL },
    },
    [LOADING] = {
        [RESPONSE_OK]    = { true, SUCCESS, on_success },
        [RESPONSE_ERROR] = { true, ERROR,   on_error },
    },
    [ERROR] = {
        [RETRY] = { true, LOADING, NULL },
    },
};

static State current = IDLE;

void handle_event(Event e) {
    if ((int)current < 0 || current >= STATE_COUNT ||
        (int)e < 0 || e >= EVENT_COUNT) {
        return;
    }

    Transition t = table[current][e];
    if (!t.valid) return;
    if (t.action) t.action();
    current = t.next;
}

static void on_success(void) {}
static void on_error(void) {}
```

Pros: the FSM is data, can be reviewed by anyone, can be generated from a
diagram. Adding a state is one row. Missing cells are explicit ignores, not
accidental transitions to the zero enum value. Tools like
[Quantum Modeler (QM)](https://www.state-machine.com/products/qm/) emit
exactly this form.
Cons: still flat (no hierarchy or parallel regions). Sparse tables spend
memory on invalid cells. Indirect call overhead per event.

## Approach 3: function-pointer state handlers (Samek QHsm-style)

Each state is a function. The function handles an event and mutates
`self->current` when it transitions. This is the basis of Miro Samek's
[Quantum Platform (QP)](https://www.state-machine.com/) framework, which
implements full hierarchical state machines (UML-compliant) for embedded
C and C++.

```c
typedef enum { FETCH, RESPONSE_OK, RESPONSE_ERROR, RETRY } Event;

typedef struct StateMachine StateMachine;
typedef void (*StateFn)(StateMachine *self, Event e);

struct StateMachine {
    StateFn current;
};

static void state_idle(StateMachine *self, Event e);
static void state_loading(StateMachine *self, Event e);
static void state_success(StateMachine *self, Event e);
static void state_error(StateMachine *self, Event e);

static void state_idle(StateMachine *self, Event e) {
    switch (e) {
        case FETCH: self->current = state_loading; break;
        case RESPONSE_OK:
        case RESPONSE_ERROR:
        case RETRY:
            break;
    }
}

static void state_loading(StateMachine *self, Event e) {
    switch (e) {
        case RESPONSE_OK:    self->current = state_success; break;
        case RESPONSE_ERROR: self->current = state_error;   break;
        case FETCH:
        case RETRY:
            break;
    }
}

static void state_success(StateMachine *self, Event e) { (void)self; (void)e; }

static void state_error(StateMachine *self, Event e) {
    switch (e) {
        case RETRY: self->current = state_loading; break;
        case FETCH:
        case RESPONSE_OK:
        case RESPONSE_ERROR:
            break;
    }
}

void sm_init(StateMachine *self) {
    if (self) self->current = state_idle;
}

void sm_dispatch(StateMachine *self, Event e) {
    if (!self || !self->current) return;
    self->current(self, e);
}
```

Pros: the state's behavior lives next to its name. Hierarchy is natural
(a state function can delegate to its parent's function). Performance is
good (one indirect call per event). The QP framework extends this with
entry / exit actions, history, and run-to-completion semantics.
Cons: more boilerplate per state. Hierarchy by hand is error-prone, which
is why most projects use QP and the QM modeler.

## Run-to-completion

In all three forms, treat each `handle_event` call as atomic: do not
recurse into `handle_event` from an action. If an action needs to fire
another event, queue it for later. This is the run-to-completion (RTC)
semantics from UML / SCXML. RTC keeps reasoning local and prevents
re-entrancy bugs. See Samek's PSiCC2 chapter 2.

## Embedded specifics

In bare-metal contexts, state machines pair with:

- **Active objects** (one machine per task, with a private event queue).
- **Hardware ISR -> event posting** (the ISR posts an event, the active
  object dispatches it on its own thread, no shared mutable state).
- **No dynamic allocation** in the table form.

QP provides all three primitives. See [state-machine.com](https://www.state-machine.com/).

## Sources

- Miro Samek, *Practical UML Statecharts in C/C++*, 2nd ed.
  ([book site](https://www.state-machine.com/psicc2),
  [free PDF](https://www.state-machine.com/doc/PSiCC2.pdf))
- [Quantum Leaps QP framework](https://www.state-machine.com/)
- [QM Modeler](https://www.state-machine.com/products/qm/)
  (generates production C/C++ from diagrams)
- [Wikipedia: Finite-state machine, software practice](https://en.wikipedia.org/wiki/Finite-state_machine#Implementation)
- [GeeksforGeeks: State machine implementation in C](https://www.geeksforgeeks.org/state-design-pattern/) (basic switch and table forms)
- [W3C SCXML 1.0](https://www.w3.org/TR/scxml/) (run-to-completion semantics)
