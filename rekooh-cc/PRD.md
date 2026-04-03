# PRD: `rekooh` — Manual Skill for Designing, Scaffolding, and Evolving Claude Code Hooks

## Summary
`rekooh` is a **manual personal Claude Code skill** for building Claude Code hook systems inside arbitrary repositories. It inspects the target repo, recommends the right hook strategy, scaffolds the necessary files and folder conventions, updates `.claude/settings.json`, and validates the final setup. It also supports explain-only mode for pair-programming and architecture discussion, but its primary job is operational: help the user end up with working hooks.

`rekooh` is **not** a replacement for Claude Code’s hook engine. Claude Code owns the hook runtime contract; `rekooh` owns the authoring, composition, scaffolding, and validation workflow around that contract.

The skill must be **manual-only** with `disable-model-invocation: true`. The skill should be placed at `~/.claude/skills/rekooh-cc/` so it can be reused across many repos while still inspecting the current target repo at invocation time.

## Product Goals, Constraints, and Defaults
- Goal: turn “I need a Claude Code hook” into a validated implementation with minimal ambiguity.
- Goal: support both **assistant mode** and **execution mode**. Assistant mode explains tradeoffs and proposes an approach. Execution mode scaffolds files, patches settings, and validates the result.
- Goal: prefer **standalone hooks first**, then offer an **opinionated typed runtime** only when complexity justifies it.
- Goal: vendor the current `_hooks-lib` as a **readable optional source** inside the skill so Claude can reuse its ideas and structure without forcing every project to adopt it.
- Goal: keep the skill highly navigable through **progressive disclosure** and problem-oriented routing.

Chosen defaults:
- Internal skill tooling uses **Bun + TypeScript**, not Python or shell. Rationale: this is a personal skill; Bun is already the preferred runtime in the existing hook runtime repo; one language across support scripts, templates, vendored runtime, and future extensions is easier to maintain than a mixed Bun/Python/shell toolchain.
- Generated hooks are still strategy-dependent: shell, Python, Bun standalone, or typed-runtime-based.
- No `allowed-tools` field in the skill metadata. Let normal Claude permissions apply.
- No skill-scoped hooks for `rekooh` itself.
- The skill may ask concise questions only when the answer materially changes the generated implementation.

## Core Product Behavior
Default workflow on every invocation:
1. Inspect the target repository first.
2. Summarize current hook/tooling context.
3. Recommend one strategy and one fallback.
4. Ask a question only if the choice is architecturally meaningful.
5. If the user asked for implementation, scaffold/update/validate directly.
6. If the user asked for explanation only, stop after the recommendation and rationale.

Strategy rules:
- Choose **standalone shell** for tiny command wrappers or notification hooks.
- Choose **standalone Python** for medium-complexity JSON/file logic where portability matters more than TS.
- Choose **standalone Bun/TypeScript** when the repo already uses Bun/TS or the user wants typed standalone logic without a shared runtime.
- Choose the **opinionated typed runtime** when the hook logic is complex, shared across multiple hooks, or benefits from typed event decoding, response builders, and reusable test helpers.

Non-goals:
- Do not act like a governance or security product.
- Do not own Claude Code’s official contract.
- Do not force the vendored runtime into every generated setup.
- Do not hide the difference between hook authoring and settings/config integration.

## Information Architecture
The skill root is:

```text
~/.claude/skills/rekooh-cc/
├── SKILL.md
├── scripts/
│   ├── inspect-project.ts
│   ├── bootstrap-hooks-infra.ts
│   ├── scaffold-standalone-hook.ts
│   ├── scaffold-opinionated-hook.ts
│   ├── patch-settings.ts
│   ├── validate-hooks.ts
│   └── sync-official-docs.ts
├── references/
│   ├── audit/
│   ├── strategy/
│   ├── bootstrap/
│   ├── authoring/
│   ├── events/
│   ├── config/
│   ├── testing/
│   ├── upstream/
│   └── opinionated-lib/
│       └── source/
├── assets/
│   ├── templates/
│   └── scaffolds/
└── examples/
```

Required initial content:
- `references/audit/`: inspect existing hooks, identify migration opportunities, explain current repo state.
- `references/strategy/`: decision matrix, standalone-first rationale, when to use the typed runtime.
- `references/bootstrap/`: folders, conventions, validation infra, and setup expectations.
- `references/authoring/`: create hook, modify hook, standalone patterns, typed-runtime patterns.
- `references/events/`: `pretooluse`, `posttooluse`, `permissionrequest`, `lifecycle`, `advanced`.
- `references/config/`: settings JSON, matcher patterns, config integration rules.
- `references/testing/`: smoke tests, deterministic validation, troubleshooting.
- `references/upstream/`: synced official docs and a distilled contracts-by-event reference.
- `references/opinionated-lib/`: tradeoffs, runtime map, create-hook guide, and vendored source snapshot.
- `assets/templates/`: generic file templates for standalone hooks and settings fragments.
- `assets/scaffolds/`: copyable multi-file scaffolds for the opinionated runtime path.
- `examples/`: complete, problem-oriented examples with `index.md` entrypoints.

Required initial examples:
- `block-destructive-bash`
- `protect-paths`
- `run-lint-after-edit`
- `stop-notification`
- `typed-hook-with-opinionated-lib`
- `complete-project-bootstrap`

## Routing and Progressive Disclosure
Main routing table in `SKILL.md`:

| User intent | Primary route |
|---|---|
| Audit existing hooks in this repo | `@references/audit/index.md` |
| Choose a hook architecture | `@references/strategy/index.md` |
| Bootstrap full hook infrastructure | `@references/bootstrap/index.md` |
| Create a new hook | `@references/authoring/index.md` |
| Modify an existing hook | `@references/authoring/modifying-existing-hooks.md` |
| Patch or fix `.claude/settings.json` | `@references/config/index.md` |
| Test or validate hooks | `@references/testing/index.md` |
| Verify official Claude Code contracts | `@references/upstream/index.md` |
| Use the vendored typed runtime | `@references/opinionated-lib/index.md` |
| Start from a concrete example | `@examples/index.md` |

Routing rules:
- `SKILL.md` must instruct Claude to load **one primary route first**.
- Every `index.md` routes to **one direct file** or **one deeper index**, never a broad fan-out.
- `events/index.md` routes by event family, not by exhaustive theory dump.
- `upstream/` is only loaded when contract verification is needed.
- `opinionated-lib/source/` is only loaded after the typed-runtime strategy is already chosen.
- `examples/` is loaded after the strategy or event family is known.
- The top-level mental model in `SKILL.md` should explain that the skill is solving **hook authoring and setup problems**, not teaching Claude Code from scratch.

This routing model follows the progressive-disclosure pattern described in the routing rules above.

## Technical Implementation Requirements
Support scripts are Bun scripts and must be executable via `bun` from the skill root.

Script contracts:

| Script | Purpose | Contract |
|---|---|---|
| `inspect-project.ts` | Repository inspection | Prints JSON to stdout. Detects `.claude/` directories, settings files, existing hook files, runtimes, package managers, quality commands, and returns a recommended hook strategy plus rationale. |
| `bootstrap-hooks-infra.ts` | Infra setup | Idempotently creates missing hook folders, validation files, and minimal fixtures in the target repo. |
| `scaffold-standalone-hook.ts` | Local hook generation | Generates final standalone hook files and minimal validation artifacts. Supports shell, Python, and Bun/TS standalone outputs. |
| `scaffold-opinionated-hook.ts` | Typed-runtime setup | Scaffolds from `assets/scaffolds/opinionated-lib/`, writes final hook entrypoints, and installs the minimal runtime files needed in the target repo. |
| `patch-settings.ts` | Settings mutation | Idempotently inserts or updates hook config in `.claude/settings.json`, avoids duplicate handlers, and prints a JSON summary of changes. |
| `validate-hooks.ts` | Deterministic validation | Runs smoke validations against generated hooks and exits non-zero on failure. |
| `sync-official-docs.ts` | Upstream refresh | Fetches current official Claude Code docs into `references/upstream/`, preferably from direct markdown URLs when available, and prepends retrieval metadata with source URL and fetch date. |

`inspect-project.ts` JSON must include at least:
- `projectRoot`
- `settingsFiles`
- `existingHooks`
- `existingSkills`
- `availableRuntimes`
- `packageManagers`
- `qualityCommands`
- `recommendedStrategy`
- `rationale`

Rules for the vendored runtime:
- `references/opinionated-lib/source/` contains a curated snapshot of the current `_hooks-lib` codebase for reading and reasoning.
- `assets/scaffolds/opinionated-lib/` contains the copyable scaffold material used to generate files into target repos.
- `references/opinionated-lib/runtime-map.md` maps the vendored source to its responsibilities so Claude can load only the relevant files.
- The skill must treat the vendored runtime as **optional**. Standalone remains the default recommendation.

Rules for official docs:
- `references/upstream/official-hooks.md` must represent the current Claude Code hooks docs.
- `references/upstream/official-settings.md` must represent the current Claude Code settings docs and config shape references.
- `references/upstream/contracts-by-event.md` must distill the official runtime contracts into a compact reference for authoring decisions.
- These docs are source-of-truth references, not the default learning path.

## Acceptance Criteria and Test Scenarios
Acceptance criteria:
- `rekooh` works as a personal skill from `~/.claude/skills/rekooh-cc/`.
- The skill is never auto-invoked by the model.
- The skill can inspect a repo and recommend a strategy without modifying files.
- The skill can scaffold a simple standalone hook end-to-end.
- The skill can scaffold a typed-runtime hook end-to-end.
- The skill can patch `.claude/settings.json` idempotently.
- The skill can validate the generated setup deterministically.
- The skill ships with curated examples and syncable official docs.
- The skill uses progressive disclosure rather than loading large documents by default.

Required test scenarios:
- Repo with no `.claude/` directory.
- Repo with existing hooks and partial settings.
- Repo that uses Bun/TypeScript heavily.
- Repo that uses only Python/shell tooling.
- Creation of a `PreToolUse` standalone hook.
- Creation of a `PostToolUse` validation hook.
- Creation of a typed-runtime hook from the vendored scaffold.
- Re-running the same scaffold flow should be idempotent or update in place without duplicates.
- Upstream docs sync should update only the expected reference files.
- The skill should fail early with a clear message if Bun is unavailable on the machine that owns the skill.

## References and Design Grounding
Official references:
- Claude Code skills docs: [code.claude.com/docs/en/skills](https://code.claude.com/docs/en/skills)
- Claude Code hooks reference: [code.claude.com/docs/en/hooks](https://code.claude.com/docs/en/hooks)
- Claude Code hooks guide: [code.claude.com/docs/en/hooks-guide](https://code.claude.com/docs/en/hooks-guide)
- Claude Code settings docs: [docs.claude.com/en/docs/claude-code/settings](https://docs.claude.com/en/docs/claude-code/settings)

Internal skill references:
- Vendored typed runtime: [references/opinionated-lib/](references/opinionated-lib/) — source snapshot, runtime map, and create-hook guide
- Layer-boundary architecture: [references/strategy/layer-boundaries.md](references/strategy/layer-boundaries.md) — 4-layer separation of concerns
- Official contracts by event: [references/upstream/contracts-by-event.md](references/upstream/contracts-by-event.md) — compact cheat sheet for all 18 hook events

Rationale summary:
- Bun is the right internal skill runtime because this is a personal skill, the current hook runtime already uses Bun/TS, and a single language/toolchain makes the skill easier to maintain and extend.
- Standalone-first prevents premature frameworking.
- Vendoring the current `_hooks-lib` preserves its strongest ideas without forcing all repos to depend on it.
- The skill must own authoring and setup, while respecting the layer boundary between Claude’s official contract, the typed runtime, and config integration.
- Progressive disclosure is mandatory because the domain is broad, multi-modal, and documentation-heavy; the skill must route by problem, not by theory.
