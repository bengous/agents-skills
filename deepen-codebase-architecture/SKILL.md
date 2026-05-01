---
name: deepen-codebase-architecture
disable-model-invocation: true
model: opus
effort: max
description: >
  Human-invoked architecture workflow for finding and designing deeper codebase
  modules. Use when the user wants an architecture audit, interface design, or
  RFC issue focused on locality/colocation, leverage, clear seams, and reducing
  AI navigation entropy. Not for broad cleanup, style refactors, or immediate
  implementation.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
  - Task
---

# Deepen Codebase Architecture

Architecture deepening workflow inspired by Matt Pocock-style organic codebase exploration and Ousterhout's deep module framing, but self-contained. It turns code navigation friction into candidate clusters, then designs smaller public interfaces that hide more implementation.

## Start

If the user did not name a mode, ask for one before doing heavy work:

- `audit`: find candidate clusters. Do not propose an interface.
- `design`: take one selected candidate and design competing interfaces.
- `issue`: turn an accepted design into a GitHub RFC issue.
- `full`: audit, ask for a candidate, design, ask for approval, then create the issue.

Ask in one line: `Mode: audit, design, issue, or full?`

If the mode is clear, continue without asking. Load:

- `workflows/evidence.md` when inspecting code.
- `workflows/audit.md` for `audit`.
- `workflows/deep-dive.md` plus `workflows/design.md` for `design`.
- `workflows/issue.md` plus `references/issue-template.md` for `issue`.
- `workflows/full.md` first for `full`, then only the workflow files it routes to.
- `references/terminology.md` when terms need to be defined precisely.

Do not load every workflow file up front.

## Vocabulary

Optimize for four signals:

- **Locality / colocation**: related decisions, data, tests, and behavior are near each other.
- **Leverage**: one interface removes repeated caller work or repeated agent reasoning.
- **Clear seams**: boundaries expose stable behavior and hide choreography.
- **AI entropy reduction**: future agents can inspect fewer files and still act correctly.

## Rules

- Evidence first: cite real files, commands, tests, docs, and call sites.
- Audit mode stops at candidates. No interface proposals.
- Design mode needs one candidate. If absent, ask or run audit first.
- Issue mode needs an accepted design. If absent, ask for the design or run design first.
- Prefer local repo patterns and existing domain language over new architecture vocabulary.
- Do not weaken existing boundaries unless the tradeoff is explicit and local.
- Replace shallow tests with boundary tests when a deeper interface makes them redundant.
- If docs/rules disagree with code, flag the drift and follow runtime code.
- Do not implement the refactor during this skill unless the user separately asks.

## Subagents

For `design`, use 3+ independent design passes when the runtime and user authorization allow it. Give each pass a different constraint and compare the results. If subagents are unavailable or not authorized, produce the same competing designs locally and say so.

## Decision Gates

At candidate selection and design approval, use a `$grill-me`-style loop: ask questions one at a time and provide the recommended answer.

If a question can be answered by exploring the codebase, explore the codebase instead.
