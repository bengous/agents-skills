# XState v5

XState is the canonical statechart library for JavaScript / TypeScript.
Cross-framework (React, Svelte, Vue, Solid, vanilla). Implements the SCXML
semantics: hierarchy, parallel regions, history, guards, actions, invoked
actors. v5 introduced the `setup()` builder for first-class type inference.

## Install

```sh
bun add xstate
# or: npm install xstate / pnpm add xstate
```

Inspector support:

```sh
bun add @statelyai/inspect
```

Framework adapters (only if you want hooks):

```sh
bun add @xstate/react       # React
bun add @xstate/svelte      # Svelte
bun add @xstate/vue         # Vue
```

## The `setup()` pattern

`setup()` is the modern way to define a machine in TypeScript. It lets you
declare types and named implementations once, then build the machine with
full inference.

```typescript
import { setup, assign } from "xstate";

const counter = setup({
  types: {
    context: {} as { count: number },
    events: {} as { type: "inc" } | { type: "dec" } | { type: "reset" },
  },
  actions: {
    increment: assign({ count: ({ context }) => context.count + 1 }),
    decrement: assign({ count: ({ context }) => context.count - 1 }),
    reset:     assign({ count: 0 }),
  },
}).createMachine({
  context: { count: 0 },
  on: {
    inc:   { actions: "increment" },
    dec:   { actions: "decrement" },
    reset: { actions: "reset" },
  },
});
```

Source: [Stately docs: setup](https://stately.ai/docs/setup).

## Toggle (canonical first machine)

```typescript
import { createMachine, createActor } from "xstate";

const toggle = createMachine({
  id: "toggle",
  initial: "inactive",
  states: {
    inactive: { on: { TOGGLE: "active" } },
    active:   { on: { TOGGLE: "inactive" } },
  },
});

const actor = createActor(toggle);
actor.subscribe((snapshot) => console.log(snapshot.value));
actor.start();
actor.send({ type: "TOGGLE" });  // -> "active"
actor.send({ type: "TOGGLE" });  // -> "inactive"
```

## Fetch with retry (with `invoke` and an actor)

```typescript
import { setup, assign, fromPromise } from "xstate";

type FetchData = { id: number; title: string };

const fetchMachine = setup({
  types: {
    context: {} as { data: FetchData | null; error: Error | null; attempt: number },
    events:  {} as { type: "FETCH" } | { type: "RETRY" } | { type: "REFETCH" },
  },
  actors: {
    fetcher: fromPromise<FetchData>(async () => {
      const res = await fetch("/api/item/1");
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return res.json();
    }),
  },
  guards: {
    canRetry: ({ context }) => context.attempt < 3,
  },
}).createMachine({
  id: "fetch",
  initial: "idle",
  context: { data: null, error: null, attempt: 0 },
  states: {
    idle: {
      on: { FETCH: { target: "loading" } },
    },
    loading: {
      entry: assign({ attempt: ({ context }) => context.attempt + 1 }),
      invoke: {
        src: "fetcher",
        onDone: {
          target: "success",
          actions: assign({ data: ({ event }) => event.output, error: null }),
        },
        onError: {
          target: "error",
          actions: assign({ error: ({ event }) => event.error as Error }),
        },
      },
    },
    success: {
      on: { REFETCH: { target: "loading" } },
    },
    error: {
      on: {
        RETRY: [
          { target: "loading", guard: "canRetry" },
          { target: "giveUp" },
        ],
      },
    },
    giveUp: {
      entry: assign({ attempt: 0 }),
      on: { RETRY: { target: "loading" } },
    },
  },
});
```

`fromPromise` wraps an async function into an *actor* that the machine
invokes. The `onDone` / `onError` transitions fire when the promise settles.
This replaces the manual "kick off the fetch in `runEffects`" pattern.

## Hierarchical example: form with substates

```typescript
import { setup } from "xstate";

const form = setup({
  types: {
    context: {} as { data: Record<string, string>; errors: string[] },
    events:  {} as
      | { type: "CHANGE"; field: string; value: string }
      | { type: "SUBMIT" }
      | { type: "RESPONSE_OK" }
      | { type: "RESPONSE_ERROR" },
  },
}).createMachine({
  id: "form",
  initial: "editing",
  context: { data: {}, errors: [] },
  states: {
    editing: {
      initial: "pristine",
      states: {
        pristine: {},
        dirty:    {},
        invalid:  {},
      },
      on: {
        CHANGE: { target: ".dirty" },
        SUBMIT: { target: "submitting" },
      },
    },
    submitting: {
      on: {
        RESPONSE_OK:    { target: "done" },
        RESPONSE_ERROR: { target: "editing.invalid" },
      },
    },
    done: { type: "final" },
  },
});
```

`SUBMIT` is defined once on the parent `editing` and applies from any of its
substates. `editing.invalid` is the dotted path syntax for targeting a
substate from outside.

## Parallel regions

```typescript
import { setup } from "xstate";

const editor = setup({}).createMachine({
  id: "editor",
  type: "parallel",
  states: {
    text: {
      initial: "regular",
      states: {
        regular: { on: { TOGGLE_BOLD: "bold" } },
        bold:    { on: { TOGGLE_BOLD: "regular" } },
      },
    },
    document: {
      initial: "saved",
      states: {
        saved:    { on: { EDIT: "modified" } },
        modified: { on: { SAVE: "saved" } },
      },
    },
  },
});
```

Both `text` and `document` regions are active at once. `state.value` is
`{ text: "regular", document: "saved" }`.

## Inspector (live debugging)

```typescript
import { createBrowserInspector } from "@statelyai/inspect";
import { createActor } from "xstate";
import { machine } from "./machine";

const { inspect } = createBrowserInspector();
const actor = createActor(machine, { inspect });
actor.start();
```

Opens a panel that visualizes the machine, highlights the current state, and
shows event history. See [Stately inspector docs](https://stately.ai/docs/inspector).

## Sources

- [Stately docs: state machines](https://stately.ai/docs/machines)
- [Stately docs: setup](https://stately.ai/docs/setup)
- [Stately docs: TypeScript](https://stately.ai/docs/typescript)
- [Stately docs: actors](https://stately.ai/docs/actors)
- [Stately docs: inspector](https://stately.ai/docs/inspector)
- [XState by Example](https://xstatebyexample.com/)
- David Khourshid, [*Infinitely Better UIs with Finite Automata*](https://www.youtube.com/watch?v=VU1NKX6Qkxc)
- [Sandro Maglione: State machines and Actors in XState v5](https://www.sandromaglione.com/articles/state-machines-and-actors-in-xstate-v5)
