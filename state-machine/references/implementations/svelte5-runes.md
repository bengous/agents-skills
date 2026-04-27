# Svelte 5: runes-based FSM

Svelte 5 introduced [runes](https://svelte.dev/docs/svelte/$state) for
explicit reactivity. `$state` is the building block; an FSM wraps it in a
class with typed transitions.

## Approach 1: a typed FSM class with `$state`

```typescript
// fsm.svelte.ts
type Status = "idle" | "loading" | "success" | "error";

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

export class FetchFsm {
  state = $state<State>({ status: "idle" });

  send(event: MachineEvent) {
    this.state = this.reduce(this.state, event);
  }

  private reduce(state: State, event: MachineEvent): State {
    switch (state.status) {
      case "idle":
        if (event.type === "FETCH") return { status: "loading" };
        return state;
      case "loading":
        if (event.type === "RESPONSE_OK")    return { status: "success", data: event.data };
        if (event.type === "RESPONSE_ERROR") return { status: "error", error: event.error };
        return state;
      case "success":
        return state;
      case "error":
        if (event.type === "RETRY") return { status: "loading" };
        return state;
    }
  }
}
```

Usage in a component:

```svelte
<script lang="ts">
  import { FetchFsm } from "./fsm.svelte.ts";
  const fsm = new FetchFsm();

  $effect(() => {
    if (fsm.state.status === "loading") {
      fetch("/api")
        .then((r) => r.json())
        .then((data) => fsm.send({ type: "RESPONSE_OK", data }))
        .catch((error) => fsm.send({ type: "RESPONSE_ERROR", error }));
    }
  });
</script>

{#if fsm.state.status === "idle"}
  <button onclick={() => fsm.send({ type: "FETCH" })}>Fetch</button>
{:else if fsm.state.status === "loading"}
  <p>Loading...</p>
{:else if fsm.state.status === "success"}
  <pre>{fsm.state.data}</pre>
{:else if fsm.state.status === "error"}
  <p>Error: {fsm.state.error.message}</p>
  <button onclick={() => fsm.send({ type: "RETRY" })}>Retry</button>
{/if}
```

The `.svelte.ts` extension is required for files using runes outside a
component. The class survives across re-renders because it is a stable
reference.

## Approach 2: derive matchers with `$derived`

```svelte
<script lang="ts">
  import { FetchFsm } from "./fsm.svelte.ts";
  const fsm = new FetchFsm();

  const isLoading = $derived(fsm.state.status === "loading");
  const data      = $derived(fsm.state.status === "success" ? fsm.state.data : null);
</script>

{#if isLoading}<p>Loading...</p>{/if}
{#if data}<pre>{data}</pre>{/if}
```

`$derived` recomputes only when its dependencies change. Useful when one
piece of the state is consumed by many child components.

## Approach 3: small libraries

Two Svelte FSM libraries:

- **[@githubnext/tiny-svelte-fsm](https://github.com/githubnext/tiny-svelte-fsm)** by GitHub Next.
  Strongly typed, runes-powered, and intentionally tiny, but deprecated on npm;
  use it only as a readable API reference unless you accept the maintenance
  risk.
- **[svelte-state-machine](https://github.com/dadencukillia/svelte-state-machine)**.
  Strategy-pattern routing for transitions, useful when transitions span
  modules.

Example with `@githubnext/tiny-svelte-fsm`:

```typescript
import { fsm } from "@githubnext/tiny-svelte-fsm";

const traffic = fsm("red", {
  red:    { TIMER: () => "green" },
  green:  { TIMER: () => "yellow" },
  yellow: { TIMER: () => "red" },
});

traffic.send("TIMER");       // -> "green"
console.log(traffic.current); // -> "green"
```

## Approach 4: XState via `@xstate/svelte`

For statecharts (hierarchy, parallel regions, history) reach for XState.
The Svelte adapter exposes the actor snapshot as a Svelte store:

```sh
bun add xstate @xstate/svelte
```

```svelte
<script lang="ts">
  import { useMachine } from "@xstate/svelte";
  import { toggle } from "./toggle.machine";

  const { snapshot, send } = useMachine(toggle);
</script>

<button onclick={() => send({ type: "TOGGLE" })}>
  {$snapshot.value === "active" ? "On" : "Off"}
</button>
```

See [@xstate/svelte docs](https://stately.ai/docs/xstate-svelte).

## Picking between the approaches

- **Class + `$state`** when you want zero deps and a small flat machine.
- **`@githubnext/tiny-svelte-fsm`** only as an API reference, unless the
  deprecation risk is acceptable.
- **`@xstate/svelte`** when you need statecharts (hierarchy, parallel,
  history) or want to inspect machines visually.

## Reactive guards

Guards can read other reactive state without extra plumbing thanks to
runes:

```typescript
class CartFsm {
  state = $state<{ status: "viewing" | "checkout" }>({ status: "viewing" });
  items = $state<CartItem[]>([]);

  send(event: { type: "GO_TO_CHECKOUT" } | { type: "BACK" }) {
    if (event.type === "GO_TO_CHECKOUT" && this.items.length > 0) {
      this.state = { status: "checkout" };
    }
    if (event.type === "BACK") {
      this.state = { status: "viewing" };
    }
  }
}
```

The guard `this.items.length > 0` is evaluated at dispatch time and does
not require subscribing to a derived store.

## Sources

- [Svelte 5 docs: $state](https://svelte.dev/docs/svelte/$state)
- [Svelte 5 docs: $derived](https://svelte.dev/docs/svelte/$derived)
- [Svelte 5 docs: $effect](https://svelte.dev/docs/svelte/$effect)
- [Introducing runes (Svelte blog)](https://svelte.dev/blog/runes)
- [tiny-svelte-fsm](https://github.com/githubnext/tiny-svelte-fsm)
- [svelte-state-machine](https://github.com/dadencukillia/svelte-state-machine)
- [@xstate/svelte](https://stately.ai/docs/xstate-svelte)
- [Mainmatter: Runes and Global state, do's and don'ts](https://mainmatter.com/blog/2025/03/11/global-state-in-svelte-5/)
