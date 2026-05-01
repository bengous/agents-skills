---
name: content-architect
model: opus
effort: medium
description: >
  Design content architecture for digital products when the needed output is a
  sitemap, page map, navigation structure, screen inventory, content hierarchy,
  or CLI-to-frontend mapping. Use when deciding what pages/screens exist and how
  they connect. Do not use for UI implementation, visual design, design systems,
  complete PRDs, implementation tickets, or code architecture.
---

# Content Architect

Design what goes where. Every screen has one job, every journey has a destination, every piece of content lives in exactly one place.

## Routing

Use this skill for:

- Sitemap, page map, navigation structure, IA/content hierarchy.
- Screen inventory: what pages/screens exist, their jobs, and how they connect.
- CLI-to-frontend mapping: which commands deserve UI screens.
- Redesign planning where the first decision is which screens survive.

Do not use this skill for:

- Building or styling UI. Use frontend/design skills instead.
- Visual identity, color, typography, tokens, or design systems.
- Full PRDs, implementation tickets, or issue breakdowns.
- Code architecture, module design, or refactoring.

## Modes

Default to `quick` unless the user explicitly asks for a full blueprint, workshop, or staged validation.

- `quick`: ask only for missing high-impact context. If enough context is present, state assumptions and produce the minimum useful screen/nav recommendation. Do not pause at gates.
- `full workshop`: use the full workflow, ask in rounds, and pause after journeys, screen inventory, and priorities before producing the blueprint.

## Workflow

1. Classify the project type and read exactly one matching reference file.
2. Collect only missing inputs: product, audience, business goal, existing state, constraints.
3. Identify the 5-10 top tasks that capture most user intent.
4. Map the critical journeys before proposing screens.
5. Derive the minimum screen inventory and navigation structure.
6. Prioritize screens as MVP, v2, or nice-to-have.
7. Output either a quick recommendation or a full blueprint.

In `quick`, proceed with explicit assumptions when context is good enough. In `full workshop`, ask 3-5 questions at a time and wait at each validation gate.

## Rules

- One screen, one job: if the job cannot be stated in one sentence, split or cut the screen.
- Journeys before screens: screens exist to serve critical paths, not brainstormed feature lists.
- Subtract before adding: ask what breaks if a screen does not exist.
- Zero redundancy: content has one canonical place; other screens link to it.
- Content-first: use real headings, CTA labels, and descriptions; no placeholder copy in final output.
- Information scent: navigation labels must be specific, sincere, substantial, and succinct.
- Navigation sizing is product-type-aware: do not apply "7 items max" blindly.
- Blueprint as contract: implementation agents should not need to ask what each screen contains or links to.

Read `references/principles.md` only when you need the detailed rationale, named frameworks, or anti-patterns.

## Output

For `quick`, produce:

- Assumptions, if any.
- Top tasks or primary journeys.
- Recommended screens/pages with each screen's job.
- Navigation structure.
- MVP/v2/nice-to-have split when useful.

For `full workshop`, load `references/blueprint-template.md` before writing the final deliverable. Name sources/frameworks when they materially shaped the architecture.

## Reference Files

| File | Read when | What it contains |
|------|-----------|-----------------|
| `references/site-vitrine.md` | Showcase site, portfolio, landing page | Landing page frameworks, hero patterns, service listing decision tree, social proof placement |
| `references/app-web.md` | Dashboard, SaaS, internal tool | CRUD screen patterns, SaaS sidebar + Cmd+K navigation, empty states, onboarding patterns, role-based navigation |
| `references/cli-frontend.md` | CLI tool getting a web frontend | Command-to-screen decision tree, CRUD-to-UI mapping, CLI-first philosophy, Pareto rule |
| `references/refonte.md` | Redesign of existing product | Audit protocol, feature parity trap, URL preservation, migration strategy |
| `references/blueprint-template.md` | Full workshop final deliverable | Complete markdown blueprint template |
| `references/principles.md` | Need rationale or guardrails | Detailed principles, citations, anti-patterns |

Read only the files needed for the current mode and project type.
