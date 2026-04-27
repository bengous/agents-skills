# React: `useReducer` with status enum

The React-canonical refactor of an `isLoading` boolean to a state machine.
The example below is an adapted reducer excerpt from Kent C. Dodds'
[*Stop using isLoading booleans*](https://kentcdodds.com/blog/stop-using-isloading-booleans).

## Before: boolean flag

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

The contradiction: the *initial* state has `isLoading: true` and `position:
null` and `error: null`. That is supposed to mean "we are pending for the
first time". But "pending the first time", "pending after an error" and
"pending while we still hold a previous position" are all distinguishable
in the UI. A single boolean cannot express that.

## After: status enum

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

The `status` field has four discrete values: `idle | pending | resolved |
rejected`. Components branch on `status`. No two flags can be true at once
because there is only one flag.

## TypeScript-idiomatic version

```typescript
import * as React from "react";

type State =
  | { status: "idle"; position: null; error: null }
  | { status: "pending"; position: GeolocationPosition | null; error: Error | null }
  | { status: "resolved"; position: GeolocationPosition; error: null }
  | { status: "rejected"; position: GeolocationPosition | null; error: Error };

type Action =
  | { type: "started" }
  | { type: "success"; position: GeolocationPosition }
  | { type: "error"; error: Error };

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "started":
      return { ...state, status: "pending" };
    case "success":
      return { status: "resolved", position: action.position, error: null };
    case "error":
      return { status: "rejected", position: state.position, error: action.error };
  }
}

const initialState: State = { status: "idle", position: null, error: null };

function useGeoPosition() {
  const [state, dispatch] = React.useReducer(reducer, initialState);

  React.useEffect(() => {
    let cancelled = false;
    dispatch({ type: "started" });
    const id = navigator.geolocation.watchPosition(
      (position) => { if (!cancelled) dispatch({ type: "success", position }); },
      (error)    => { if (!cancelled) dispatch({ type: "error", error: new Error(error.message) }); },
    );
    return () => { cancelled = true; navigator.geolocation.clearWatch(id); };
  }, []);

  return state;
}
```

The discriminated union on the state side gives you exhaustive narrowing in
components:

```tsx
function GeoPosition() {
  const state = useGeoPosition();
  switch (state.status) {
    case "idle":     return <p>Preparing geolocation...</p>;
    case "pending":
      return state.position
        ? <p>Updating from {format(state.position)}...</p>
        : <p>Locating...</p>;
    case "resolved": return <p>{format(state.position)}</p>;
    case "rejected": return <p>Error: {state.error.message}</p>;
  }
}
```

Note how `pending` can show *either* "first locate" or "updating" depending
on whether `position` is already known. That distinction was unrepresentable
in the boolean version.

## When `useReducer` is enough

`useReducer` covers many component-local FSMs. You get:

- Pure reducer (testable in isolation)
- React batching (multiple dispatches per event are batched)
- Predictable updates (no race between two `setState` calls)

It does **not** give you:

- Hierarchy / parallel / history (graduate to XState)
- Cross-component state sharing (lift to context, Zustand, or XState actor)
- Visual diagram tooling
- Built-in async actor model

## When to graduate to XState

The break-even is roughly when you write your second hand-rolled
`runEffects(prev, next)` to dispatch entry actions. XState bakes those in.
See `xstate.md`.

## Sources

- Kent C. Dodds, [*Stop using isLoading booleans*](https://kentcdodds.com/blog/stop-using-isloading-booleans) (source of the adapted example)
- Kent C. Dodds, [*Application State Management with React*](https://kentcdodds.com/blog/application-state-management-with-react)
- [React docs: useReducer](https://react.dev/reference/react/useReducer)
- cassiozen, [*useStateMachine* hook](https://github.com/cassiozen/useStateMachine) (sub-1KB FSM hook)
- [LogRocket: Goodbye, useState? Smarter state modeling for modern React apps](https://blog.logrocket.com/goodbye-usestate-react-state-modeling/)
