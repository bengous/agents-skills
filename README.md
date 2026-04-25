# agents-skills

Custom skills for Claude Code and compatible AI agents.

## Install

```bash
# All skills
npx skills add b3ngous/agents-skills

# Single skill
npx skills add b3ngous/agents-skills --skill bun-cc
```

## Setup

```bash
./install.sh
```

## Validation

```bash
# Validate all skill frontmatter
cargo run -p skills-tools -- validate frontmatter

# Validate a single skill
cargo run -p skills-tools -- validate frontmatter content-architect/SKILL.md
```

The Rust workspace follows strict formatting and linting conventions (see `rustfmt.toml`, `clippy.toml`, and `Cargo.toml` workspace lints).

## Naming convention

- **No suffix** — portable, agent-agnostic skill
- **`-cc` suffix** — Claude Code specific or optimized

## Skills

### Portable

| Skill | Description |
|-------|-------------|
| `content-architect` | Design content architecture for digital products: screen inventory, user journeys, navigation |
| `design-system-creator` | Create design systems based on physical/sensory anchoring |
| `handoff-prompt` | Generate a copy-paste-ready handoff prompt for a fresh agent instance |
| `humanizer` | Remove signs of AI-generated writing from text |
| `skill-eval-methodology` | Operational discipline for running skill evaluations and benchmarks |
| `slop-detector` | Detect, score, and rewrite AI-generated or generic text |

### Claude Code (`-cc`)

| Skill | Description |
|-------|-------------|
| `ascii-diagram-builder-cc` | Generate pixel-perfect ASCII box diagrams for architecture docs and READMEs |
| `bun-cc` | TypeScript CLI scripts and applications using the Bun runtime |
| `effect-language-service-cc` | Tooling automation for @effect/language-service: diagnostics, quickfixes, codegen |
| `effect-usage-cc` | Decision support for Effect (effect-ts) — when to use it, which patterns to prefer |
| `null-as-error-cc` | Audit Effect codebases for silent error swallowing (catchAll → sentinel values) |
| `refactorlib-cc` | Audit a codebase for handcrafted code replaceable by existing dependencies |
| `rekooh-cc` | Author, audit, and manage Claude Code hooks — scaffolding, typed runtime, testing |
