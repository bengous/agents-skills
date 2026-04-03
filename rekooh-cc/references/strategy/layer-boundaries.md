# Layer Boundaries

Hook authoring spans four distinct layers. Knowing which layer owns a concern prevents the skill from conflating runtime semantics with config management or authoring ergonomics.

## Layer 0: Claude Contract Layer

Mirrors Anthropic's official contract.

**Owns:** official event names, input field names, output and decision semantics, matcher behavior, handler-type support rules.

**Does not own:** framework ergonomics, presets, config file mutation.

**Design rule:** this layer speaks in Claude's terms, not in framework convenience types.

## Layer 1: Command-Hook Runtime Layer

Bun/TypeScript adapter for writing `type: "command"` hooks.

**Owns:** reading stdin JSON, parsing and narrowing official event payloads, exposing event-specific typed inputs, producing official stdout/stderr/exit code behavior, response builders for the real Claude decision families, raw payload access for unknown fields.

**Does not own:** `http`, `prompt`, or `agent` authoring; project config mutation; preset installation or project bootstrap.

**Design rule:** a faithful adapter, not a second contract.

## Layer 2: Authoring Layer

Developer-facing experience for writing hooks.

**Owns:** `define...`-style hook authoring helpers, event-specific helpers, presets, test helpers, template generation for hook source files.

**Does not own:** official contract truth, settings ownership.

**Design rule:** ergonomics may simplify the runtime, but must not lie about Claude's behavior.

## Layer 3: Config Integration Layer

Optional and separate.

**Owns:** emitting or updating hook registration snippets, integrating with `settings.json` or `__settings.jsonc` workflows, project bootstrap helpers.

**Does not own:** hook runtime semantics, event typing, response semantics.

**Design rule:** replaceable without changing the runtime or authoring APIs.

## Key Architectural Decisions

1. **V1 is a command-hook framework.** It owns authoring and running `type: "command"` hooks only. Prompt, agent, and HTTP hooks are configuration shapes, not TypeScript hook-runtime targets.

2. **Settings/config management is separate.** The hook framework core does not own `.claude/settings.json` mutation. Config automation is a separate integration layer.

3. **The runtime models all documented command-capable events** (18 events), even if presets only cover a subset. This prevents the public surface from lagging behind the official contract.

4. **Event-aware responses, not a universal result type.** Different events have different decision families (`permissionDecision`, `decision.behavior`, `decision` + `reason`, `continue` + `stopReason`, worktree stdout-path). Flattening them into one union throws away meaning.

5. **Unknown and future fields must survive.** Parse known fields explicitly, preserve unknown fields on the raw payload, allow raw escape hatches when the contract evolves.

## Practical Rule

Use this document when deciding where a feature belongs:

- Mirrors Anthropic's official contract -> **Layer 0**
- Helps command hooks consume or emit that contract -> **Layer 1**
- Improves developer experience for writing hooks -> **Layer 2**
- Edits project config -> **Layer 3**
