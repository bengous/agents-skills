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
- [`looper`](looper/) designs dynamic Codex loops, including PR feedback loops,
  stacked PR loops, maintenance loops, and loop-to-`/goal` payloads.
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

| Skill                                                           | Description                                                                                                                   |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| [`batch-update`](batch-update/)                                 | Update repo dependencies and tooling in researched, validated, committed batches                                              |
| [`content-architect`](content-architect/)                       | Design content architecture for digital products: screen inventory, user journeys, navigation                                 |
| [`council`](council/)                                           | Get fresh Codex subagent perspectives on the current problem                                                                  |
| [`deepen-codebase-architecture`](deepen-codebase-architecture/) | Audit or design deeper codebase modules with evidence-backed architecture proposals                                           |
| [`design-system-creator`](design-system-creator/)               | Create design systems based on physical/sensory anchoring                                                                     |
| [`goalify`](goalify/)                                           | Convert rough intent into a compact Codex `/goal` payload or protected `.agents/goals` file                                   |
| [`handoff-prompt`](handoff-prompt/)                             | Generate a copy-paste-ready handoff prompt for a fresh agent instance                                                         |
| [`harden-bash`](harden-bash/)                                   | Write and harden production shell scripts                                                                                     |
| [`humanizer`](humanizer/)                                       | Remove signs of AI-generated writing from text                                                                                |
| [`intent-to-workflow`](intent-to-workflow/)                     | Human-gated planner that turns explicit intent into local workflow artifacts                                                  |
| [`looper`](looper/)                                             | Design, audit, and convert rough work into dynamic Codex loops with gates, validation, and stop/pause conditions              |
| [`of`](of/)                                                     | Open an explicit or recently mentioned local file in the default viewer                                                       |
| [`premium-handoff`](premium-handoff/)                           | Turn selected targets into compact handoffs for scarce premium models                                                        |
| [`promptify`](promptify/)                                       | Transform rough dictated intent into a compact GPT-5.5-optimized prompt                                                       |
| [`pr-review-html`](pr-review-html/)                             | Create annotated HTML artifacts for PR reviews and unfamiliar code paths                                                      |
| [`shrinkify`](shrinkify/)                                       | Interactively reduce skills while preserving behavior and output contracts                                                    |
| [`skill-eval-methodology`](skill-eval-methodology/)             | Operational discipline for running skill evaluations and benchmarks                                                           |
| [`skills-maintenance`](skills-maintenance/)                     | Update installed global or project skills and persist related maintenance changes                                             |
| [`skill-sync`](skill-sync/)                                     | Commit, push, and deploy skill changes while verifying source/live/dotfiles boundaries                                        |
| [`slice-runner`](slice-runner/)                                 | Execute approved plans by Codex-native implementation slices                                                                  |
| [`slop-detector`](slop-detector/)                               | Detect, score, and rewrite AI-generated or generic text                                                                       |
| [`state-machine`](state-machine/)                               | Model behavior as finite state machines and statecharts; references for TypeScript, XState v5, React, Svelte 5, C, Java, Rust |
| [`swarm-research`](swarm-research/)                             | Fan out parallel Codex research subagents and synthesize the findings                                                         |
| [`targetify`](targetify/)                                       | Rank evidence-backed repo targets before spending scarce premium model attention                                             |
| [`teach-back`](teach-back/)                                     | Teach a session incrementally and verify the human's understanding before advancing                                           |

### Claude Code (`-cc`)

| Skill                                                       | Description                                                                        |
| ----------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| [`ascii-diagram-builder-cc`](ascii-diagram-builder-cc/)     | Generate pixel-perfect ASCII box diagrams for architecture docs and READMEs        |
| [`bun-cc`](bun-cc/)                                         | TypeScript CLI scripts and applications using the Bun runtime                      |
| [`effect-language-service-cc`](effect-language-service-cc/) | Tooling automation for @effect/language-service: diagnostics, quickfixes, codegen  |
| [`effect-usage-cc`](effect-usage-cc/)                       | Decision support for Effect (effect-ts): when to use it, which patterns to prefer  |
| [`null-as-error-cc`](null-as-error-cc/)                     | Audit Effect codebases for silent error swallowing (`catchAll` to sentinel values) |
| [`refactorlib-cc`](refactorlib-cc/)                         | Audit a codebase for handcrafted code replaceable by existing dependencies         |
| [`rekooh-cc`](rekooh-cc/)                                   | Author, audit, register, and test Claude Code hooks against official docs          |

## Maintenance

Use [`skill-sync`](skill-sync/) when changing a skill that must be committed,
pushed, and deployed live. After the commit reaches `origin/master`, publish
only the changed managed skills by exact name:

```bash
scripts/publish-live <skill-name>...
```

The shared publisher refuses dirty, detached, non-default, divergent, or
unpushed source and verifies the installed content before succeeding.

## License

[MIT](LICENSE)
