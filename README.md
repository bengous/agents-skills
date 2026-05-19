# agents-skills

Reusable agent workflows for writing, architecture, development, and repo
maintenance.

## Quick Start

Install every skill:

```bash
npx skills add bengous/agents-skills
```

Install one skill:

```bash
npx skills add bengous/agents-skills --skill bun-cc
```

Some skills need extra local setup. Notably, [`goalify`](goalify/README.md) uses
the `codex-goal` helper to write protected long goal files under
`.agents/goals/`.

The installer commands use the [skills.sh](https://skills.sh/) CLI.

## Start Here

Useful entry points:

- [`goalify`](goalify/) converts rough intent into a compact Codex `/goal`
  payload or protected `.agents/goals` file.
- [`skill-sync`](skill-sync/) commits, pushes, and deploys skill changes while
  checking source, live install, and dotfiles boundaries.
- [`intent-to-workflow`](intent-to-workflow/) turns a broad intention into local
  planning artifacts with human gates.
- [`bun-cc`](bun-cc/) captures Bun-specific conventions for TypeScript CLI
  scripts and apps.

## Local Development

Prepare the Rust toolchain and repo validation tools:

```bash
./install.sh
```

Validate all skill frontmatter:

```bash
cargo run -p skills-tools -- validate frontmatter
```

Validate one skill:

```bash
cargo run -p skills-tools -- validate frontmatter content-architect/SKILL.md
```

Run the full Rust quality gate:

```bash
scripts/check-rust.sh
```

If the Rust dependency tools are missing, install them with:

```bash
cargo install cargo-deny cargo-machete --locked
```

The Rust workspace uses strict formatting, linting, dependency, license, and
unused-dependency checks. See `rustfmt.toml`, `clippy.toml`, `Cargo.toml`,
`deny.toml`, and `scripts/check-rust.sh`.

## Naming

- No suffix: general skill or repo workflow.
- `-cc` suffix: Claude Code specific or optimized.

## Skills

### General

| Skill | Description |
|-------|-------------|
| [`batch-update`](batch-update/) | Update repo dependencies and tooling in researched, validated, committed batches |
| [`content-architect`](content-architect/) | Design content architecture for digital products: screen inventory, user journeys, navigation |
| [`deepen-codebase-architecture`](deepen-codebase-architecture/) | Audit or design deeper codebase modules with evidence-backed architecture proposals |
| [`design-system-creator`](design-system-creator/) | Create design systems based on physical/sensory anchoring |
| [`goalify`](goalify/) | Convert rough intent into a compact Codex `/goal` payload or protected `.agents/goals` file |
| [`handoff-prompt`](handoff-prompt/) | Generate a copy-paste-ready handoff prompt for a fresh agent instance |
| [`humanizer`](humanizer/) | Remove signs of AI-generated writing from text |
| [`intent-to-workflow`](intent-to-workflow/) | Human-gated planner that turns explicit intent into local workflow artifacts |
| [`promptify`](promptify/) | Transform rough dictated intent into a compact GPT-5.5-optimized prompt |
| [`pr-review-html`](pr-review-html/) | Create annotated HTML artifacts for PR reviews and unfamiliar code paths |
| [`shrinkify`](shrinkify/) | Interactively reduce skills while preserving behavior and output contracts |
| [`skill-eval-methodology`](skill-eval-methodology/) | Operational discipline for running skill evaluations and benchmarks |
| [`skill-sync`](skill-sync/) | Commit, push, and deploy skill changes while verifying source/live/dotfiles boundaries |
| [`slop-detector`](slop-detector/) | Detect, score, and rewrite AI-generated or generic text |
| [`state-machine`](state-machine/) | Model behavior as finite state machines and statecharts; references for TypeScript, XState v5, React, Svelte 5, C, Java, Rust |

### Claude Code (`-cc`)

| Skill | Description |
|-------|-------------|
| [`ascii-diagram-builder-cc`](ascii-diagram-builder-cc/) | Generate pixel-perfect ASCII box diagrams for architecture docs and READMEs |
| [`bun-cc`](bun-cc/) | TypeScript CLI scripts and applications using the Bun runtime |
| [`effect-language-service-cc`](effect-language-service-cc/) | Tooling automation for @effect/language-service: diagnostics, quickfixes, codegen |
| [`effect-usage-cc`](effect-usage-cc/) | Decision support for Effect (effect-ts): when to use it, which patterns to prefer |
| [`null-as-error-cc`](null-as-error-cc/) | Audit Effect codebases for silent error swallowing (`catchAll` to sentinel values) |
| [`refactorlib-cc`](refactorlib-cc/) | Audit a codebase for handcrafted code replaceable by existing dependencies |

## Maintenance

Use [`skill-sync`](skill-sync/) when changing a skill that must be committed,
pushed, and deployed live. Keep repo source, live installs, dotfiles-managed
bootstrap files, and generated artifacts separate.

The dotfiles-managed bootstrap for this machine lives at
[`~/dotfiles/.chezmoiscripts/run_after_install-global-skills.sh`](/home/b3ngous/dotfiles/.chezmoiscripts/run_after_install-global-skills.sh).

## License

[MIT](LICENSE)
