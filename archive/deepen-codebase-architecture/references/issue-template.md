# RFC Issue Template

Use this structure for GitHub issues. Keep concrete evidence, but frame the target architecture by ownership and contract, not by a fragile step-by-step patch list.

````markdown
## Problem

<Describe the architectural friction. Include the shallow modules, scattered concept ownership, unclear seams, repeated caller work, or AI navigation entropy.>

## Evidence

- Authority files:
  - `<path>`: <why it matters>
- Representative callers:
  - `<path>`: <current dependency/choreography>
- Tests:
  - `<path>`: <what they currently verify or miss>

## Proposed Interface

<Chosen interface signature or command/contract.>

```ts
// Example only. Adjust language to the repo.
```

### Usage

```ts
// Realistic caller example.
```

## Ownership

- Owns:
  - <responsibility>
- Hides:
  - <sequencing, validation, persistence, transport, formatting, etc.>
- Does not own:
  - <explicit exclusions>

## Dependency Strategy

Category: `<in-process | local-substitutable | remote-owned | true-external>`

<Explain ports, adapters, local stand-ins, mocks, or runtime dependencies.>

## Testing Strategy

- New boundary tests:
  - <behavior visible through the public interface>
- Old tests to delete or collapse:
  - <shallow tests made redundant>
- Test environment:
  - <fixtures, local stand-ins, clocks, temp dirs, services>

## Migration Plan

1. <Smallest reversible step>
2. <Move one caller or one vertical slice>
3. <Delete redundant public surface/tests after migration>

## Acceptance Criteria

- <Boundary owns the named behavior>
- <Callers no longer repeat the hidden choreography>
- <Boundary tests cover observable behavior>
- <Docs/rules updated only where they describe real runtime behavior>
````
