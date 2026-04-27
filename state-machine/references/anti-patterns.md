# Anti-patterns

Common mistakes that state machines exist to prevent. Each entry has a BEFORE
example, an AFTER example, and a one-paragraph explanation.

## 1. Boolean explosion

### Before

```typescript
function Component() {
  const [isLoading, setIsLoading] = useState(false);
  const [isError, setIsError] = useState(false);
  const [isSuccess, setIsSuccess] = useState(false);
  const [isRetrying, setIsRetrying] = useState(false);
  const [hasSubmitted, setHasSubmitted] = useState(false);
  const [isStale, setIsStale] = useState(false);
  // ...
}
```

Six booleans give 64 combinations. About 6 are valid. The rest are bugs
waiting to happen: `isLoading && isSuccess`, `isError && isRetrying &&
hasSubmitted`, `isLoading && !hasSubmitted`. Nothing in the type system
prevents you from setting two at once.

### After

```typescript
type State =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "success"; data: T; isStale: boolean }
  | { status: "error"; error: Error }
  | { status: "retrying"; attempt: number };
```

Five valid configurations. Mutual exclusion enforced by the discriminated
union. `isStale` survives because it is a *property* of the success state,
not an independent dimension.

### Why it matters

The combinatorial argument: with `n` independent booleans, `2^n` total
combinations exist, but the system has at most a few "valid" ones. Every
unconstrained `setIsXxx(true)` call risks landing the UI in an unreachable
combination. State machines forbid those combinations at the type level.

[statecharts.dev: state explosion](https://statecharts.dev/state-machine-state-explosion.html)

## 2. The `isLoading` boolean (Kent C. Dodds canonical example)

Kent C. Dodds' [*Stop using isLoading booleans*](https://kentcdodds.com/blog/stop-using-isloading-booleans) is the canonical write-up. The example below is an
adapted excerpt of the reducer shape from that post.

### Before

```javascript
function geoPositionReducer(state, action) {
  switch (action.type) {
    case 'error': {
      return {
        ...state,
        isLoading: false,
        error: action.error,
      }
    }
    case 'success': {
      return {
        ...state,
        isLoading: false,
        position: action.position,
      }
    }
    default: {
      throw new Error(`Unhandled action type: ${action.type}`)
    }
  }
}

function useGeoPosition() {
  const [state, dispatch] = React.useReducer(geoPositionReducer, {
    isLoading: true,
    position: null,
    error: null,
  })
  // ... rest of implementation
}
```

Initial state has `isLoading: true` *and* `position: null` *and* `error:
null` simultaneously. That is supposed to mean "pending", but the UI cannot
distinguish "loading for the first time" from "we have a stale position and
are loading a new one" or from "the previous attempt errored and we are
retrying".

### After

```javascript
function geoPositionReducer(state, action) {
  switch (action.type) {
    case 'error': {
      return {
        ...state,
        status: 'rejected',
        error: action.error,
      }
    }
    case 'success': {
      return {
        ...state,
        status: 'resolved',
        position: action.position,
      }
    }
    case 'started': {
      return {
        ...state,
        status: 'pending',
      }
    }
    default: {
      throw new Error(`Unhandled action type: ${action.type}`)
    }
  }
}

function useGeoPosition() {
  const [state, dispatch] = React.useReducer(geoPositionReducer, {
    status: 'idle',
    position: null,
    error: null,
  })
  // ... rest of implementation
}
```

A single `status` field with values `idle | pending | resolved | rejected`.
Components branch on `status`. No more contradictory rendering.

### Why it matters

Loading is not a binary. There is at least: never started, started for the
first time, started again with stale data, finished with a result, finished
with an error. A single boolean cannot represent all five. A status enum can.

Kent's full post: [stop-using-isloading-booleans](https://kentcdodds.com/blog/stop-using-isloading-booleans).

## 3. Dead-end state

### Before

```typescript
type State =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "success"; data: T }
  | { status: "error"; error: Error };  // no transition out
```

The `error` state has no transition out. The user is stuck. They have to
refresh the page.

### After

```typescript
type State =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "success"; data: T }
  | { status: "error"; error: Error };

type MachineEvent =
  | { type: "FETCH" }
  | { type: "RESPONSE_OK"; data: T }
  | { type: "RESPONSE_ERROR"; error: Error }
  | { type: "RETRY" };  // <- new event

// In the reducer, error + RETRY -> loading
```

### Why it matters

statecharts.dev: *"Every state should have an exit (no dead ends)."* Apply
this rule mechanically to every diagram: walk each node and confirm at
least one outgoing arrow. If a state truly is final (the machine is done),
mark it as a final state in the diagram so the absence of exits is
intentional.

## 4. Syncing derivable state

### Before

```typescript
const [items, setItems] = useState<Item[]>([]);
const [count, setCount] = useState(0);
const [hasItems, setHasItems] = useState(false);

function addItem(item: Item) {
  const next = [...items, item];
  setItems(next);
  setCount(next.length);          // can drift
  setHasItems(next.length > 0);   // can drift
}
```

Three pieces of state, but `count` and `hasItems` are functions of `items`.
Any code path that updates one but forgets the others creates an
inconsistency.

### After

```typescript
const [items, setItems] = useState<Item[]>([]);
const count = items.length;
const hasItems = count > 0;

function addItem(item: Item) {
  setItems([...items, item]);
}
```

### Why it matters

Kent C. Dodds, [*Don't Sync State, Derive It*](https://kentcdodds.com/blog/dont-sync-state-derive-it): if a value can be computed from other state, don't
store it. This applies to FSMs too: do not duplicate the discriminant tag in
a separate boolean field.

## 5. Mixing UI state with server cache state

### Before

```typescript
type State =
  | { status: "loading" }
  | { status: "success"; data: User; isStale: boolean }
  | { status: "error"; error: Error }
  | { status: "refetching"; data: User }   // ?
  | { status: "validating"; data: User };  // ?
```

Five states for what is fundamentally one logical thing: "we have a server
resource, here is its current cache status". Trying to model server cache
in your UI FSM duplicates work that a query library already does well.

### After

```typescript
// Server cache: TanStack Query, SWR, or Apollo
const { data, status, isFetching, error } = useUserQuery();

// UI FSM: only the local interaction modes
type UiState =
  | { status: "viewing" }
  | { status: "editing"; draft: UserEdits }
  | { status: "confirming-discard" };
```

### Why it matters

Server cache has its own lifecycle (stale-while-revalidate, background
refetch, retry, garbage collection) that is well-handled by mature
libraries. Reinventing it in your FSM means reinventing that whole
lifecycle. Keep the FSM focused on UI modes the user perceives.

## 6. Switch-statement FSM without an explicit transition table

### Before (C / Java)

```c
void handle_event(Event e) {
    switch (current_state) {
        case STATE_A:
            if (e == EVENT_X) current_state = STATE_B;
            // What if EVENT_Y comes in STATE_A? Silently ignored.
            // What if STATE_C is missing from the switch? Silently ignored.
            break;
        case STATE_B:
            if (e == EVENT_Z) current_state = STATE_A;
            break;
        // STATE_C: forgot to add a case
    }
}
```

This works only while the machine is tiny. It rots fast. Adding a state means
editing one function in multiple places. Missing cases are not caught at
compile time.

### After

```c
#include <stdbool.h>
#include <stddef.h>

typedef enum { STATE_A, STATE_B, STATE_C, STATE_COUNT } State;
typedef enum { EVENT_X, EVENT_Y, EVENT_Z, EVENT_COUNT } Event;

static void action_a_to_b(void);

typedef struct {
    bool valid;
    State next;
    void (*action)(void);
} Transition;

static const Transition table[STATE_COUNT][EVENT_COUNT] = {
    [STATE_A] = {
        [EVENT_X] = { true, STATE_B, action_a_to_b },
        [EVENT_Y] = { true, STATE_C, NULL },
    },
    [STATE_B] = {
        [EVENT_Z] = { true, STATE_A, NULL },
    },
    [STATE_C] = {
        [EVENT_X] = { false, STATE_C, NULL },
    },
};

static State current_state = STATE_A;

void handle_event(Event e) {
    if ((int)current_state < 0 || current_state >= STATE_COUNT ||
        (int)e < 0 || e >= EVENT_COUNT) {
        return;
    }

    Transition t = table[current_state][e];
    if (!t.valid) return;
    if (t.action) t.action();
    current_state = t.next;
}

static void action_a_to_b(void) {}
```

The transition table is data, not control flow. Adding a state adds a row.
You can dump the table to a CSV and review it with a non-engineer. You can
generate it from a diagram (Samek's QM tool does exactly this). The
`valid` sentinel makes missing entries explicit ignores instead of accidental
transitions to the zero enum value.

### Why it matters

Samek, *Practical UML Statecharts in C/C++* (2nd ed.), chapter 3, makes the
case at length: switch-based FSMs do not scale and lose the diagram-as-spec
property. A table makes the FSM auditable.

## Sources

- Kent C. Dodds, *Stop using isLoading booleans*, [blog](https://kentcdodds.com/blog/stop-using-isloading-booleans)
- Kent C. Dodds, *Don't Sync State, Derive It*, [blog](https://kentcdodds.com/blog/dont-sync-state-derive-it)
- Kent C. Dodds, *Application State Management with React*, [blog](https://kentcdodds.com/blog/application-state-management-with-react)
- [statecharts.dev: state explosion](https://statecharts.dev/state-machine-state-explosion.html)
- [LogRocket: Goodbye, useState? Smarter state modeling for modern React](https://blog.logrocket.com/goodbye-usestate-react-state-modeling/)
- Miro Samek, *Practical UML Statecharts in C/C++*, 2nd ed., ch. 3
- David Khourshid, *Infinitely Better UIs with Finite Automata*, [YouTube](https://www.youtube.com/watch?v=VU1NKX6Qkxc)
