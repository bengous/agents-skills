# TypeScript: discriminated unions

Pure TypeScript, no library. The most portable pattern. Works in any TS
runtime: Node, Bun, browser, edge worker, Cloudflare, Deno.

## The minimal viable FSM

```typescript
type State =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "success"; data: string }
  | { status: "error"; error: Error };

type MachineEvent =
  | { type: "FETCH" }
  | { type: "RESPONSE_OK"; data: string }
  | { type: "RESPONSE_ERROR"; error: Error }
  | { type: "RETRY" };

function reduce(state: State, event: MachineEvent): State {
  switch (state.status) {
    case "idle":
      if (event.type === "FETCH") return { status: "loading" };
      return state;
    case "loading":
      if (event.type === "RESPONSE_OK") return { status: "success", data: event.data };
      if (event.type === "RESPONSE_ERROR") return { status: "error", error: event.error };
      return state;
    case "success":
      return state;
    case "error":
      if (event.type === "RETRY") return { status: "loading" };
      return state;
  }
}
```

That is the entire FSM in 25 lines. No dependencies, no runtime overhead,
fully type-checked.

## Exhaustiveness with `assertNever`

A common oversight is forgetting to handle a new `status` after adding it.
TypeScript can enforce exhaustiveness with a compile-time check:

```typescript
function assertNever(x: never): never {
  throw new Error("Unhandled state: " + JSON.stringify(x));
}

function render(state: State) {
  switch (state.status) {
    case "idle":    return "Click fetch.";
    case "loading": return "Loading...";
    case "success": return state.data;
    case "error":   return state.error.message;
    default:        return assertNever(state);  // compile error if a case is missing
  }
}
```

Add a new state (`{ status: "retrying" }`) and the compiler points to every
`render`-style function that needs updating. This is the property that
makes discriminated unions safer than booleans.

## Narrowing pulls field types

```typescript
function show(state: State) {
  if (state.status === "success") {
    // state is narrowed to { status: "success"; data: string }
    return state.data.toUpperCase();
  }
  if (state.status === "error") {
    // state is narrowed to { status: "error"; error: Error }
    return state.error.stack;
  }
}
```

Each case sees exactly the fields that exist in that case. `data` cannot be
read in `error` and vice versa.

## Side effects (where they belong)

The reducer above is pure. To run side effects, dispatch from a host
function:

```typescript
class Store {
  private state: State = { status: "idle" };
  private listeners = new Set<(s: State) => void>();

  send(event: MachineEvent) {
    const prev = this.state;
    this.state = reduce(prev, event);
    if (prev !== this.state) {
      this.listeners.forEach((l) => l(this.state));
      this.runEffects(prev, this.state);
    }
  }

  private runEffects(prev: State, next: State) {
    if (prev.status !== "loading" && next.status === "loading") {
      fetch("/data")
        .then((r) => r.json())
        .then((data) => this.send({ type: "RESPONSE_OK", data }))
        .catch((error) => this.send({ type: "RESPONSE_ERROR", error }));
    }
  }

  subscribe(l: (s: State) => void) {
    this.listeners.add(l);
    return () => this.listeners.delete(l);
  }
}
```

This is essentially a tiny Redux store. The reducer stays pure (great for
tests), the effects live in `runEffects` (entry actions: "if we just
entered `loading`, kick off the fetch").

## When to graduate

Go to XState (or another library) when any of these become a problem:

- Too many atomic states to audit comfortably in one reducer
- Need for hierarchy (compound states with shared transitions)
- Need for parallel regions
- Need for history (resume where left off)
- Need for declarative inspection / visualization

For many UI use cases, discriminated unions are enough.

## Pairs well with

- `Reducer<S, E>` from React (`useReducer`) - see `react-usereducer.md`
- `$state` rune in Svelte 5 - see `svelte5-runes.md`
- A standalone class as shown above for vanilla TS or workers

## Sources

- [TypeScript Handbook: Discriminated Unions](https://www.typescriptlang.org/docs/handbook/2/narrowing.html#discriminated-unions)
- Kent C. Dodds, [*Stop using isLoading booleans*](https://kentcdodds.com/blog/stop-using-isloading-booleans)
- Kyle Shevlin, [*Eliminate Boolean Explosion by Enumerating States*](https://egghead.io/lessons/javascript-eliminate-boolean-explosion-by-enumerating-states)
- [TypeScript Deep Dive: Exhaustiveness Checking](https://basarat.gitbook.io/typescript/type-system/discriminated-unions)
