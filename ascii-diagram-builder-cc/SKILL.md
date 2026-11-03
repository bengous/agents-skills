---
name: ascii-diagram-builder-cc
disable-model-invocation: true
allowed-tools: Bash(bun ${SKILL_DIR}/scripts/*)
description: >
  Generate pixel-perfect ASCII box diagrams for architecture docs, READMEs, and
  code comments. Use when the user asks to create, fix, or improve ASCII diagrams,
  draw architecture diagrams, create box-and-arrow diagrams for documentation,
  or when you need to add diagrams to markdown files. Also use proactively when
  generating architecture documentation that would benefit from visual diagrams.
  Triggers on: "draw a diagram", "ASCII diagram", "box diagram", "architecture
  diagram", "add a diagram to the doc", "fix the alignment", "the boxes are
  misaligned", or any request to visualize system architecture in text.
license: "MIT. Copyright (c) 2026 Augustin BENGOLEA. Full terms in LICENSE at https://github.com/bengous/agents-skills"
---

# ASCII Diagram Builder

Generate correctly aligned ASCII box diagrams using programmatic helpers
rather than hand-counting characters. Hand-drawn ASCII diagrams inevitably
have alignment bugs because humans can't reliably count columns across
50+ character lines.

## The Font-Rendering Problem

Box-drawing horizontal characters (`─`) and ASCII spaces (` `) render at
**different pixel widths** in most editor fonts (VS Code, JetBrains,
terminal emulators). This is invisible at small scales but accumulates
over many characters. A `┬` at column 30 in a line of `─` characters
will appear visually shifted compared to a `│` at column 30 in a line
of spaces.

This means:

```
  └───────────────────┬────────────────────┘   ← ┬ at col 21 (after 19× ─)
                     │                         ← │ at col 21 (after 21× space)
                     THESE WON'T ALIGN VISUALLY
```

### The Core Rule

Drift happens when a `─` line and a space line need a character to
align at the same column. The accumulated pixel difference across
many characters shifts one relative to the other.

**Safe patterns** (no drift):
- Box side edges (`│` left and `│` right) — both lines use the same
  character mix (box chars + content), so they track together.
- `└───────┘` followed by `│` on the next line — the `│` is preceded
  by only spaces, and the `┘` above is a single character. No
  accumulated drift reaches the `│` column.
- `└─── label ───┘` spanning between two `│` above — works as long
  as `└` and `┘` are at the exact columns of the `│` they connect to.
  Compute the width programmatically.

**Broken patterns** (drift accumulates):
- `┬` in a box border with `│` below on a space-padded line — the `─`
  chars before `┬` accumulate drift, so `┬` and `│` appear misaligned.
- Side-by-side `┐ ┌` on a border line with `│ │` on a content line —
  the space between `┐` and `┌` drifts relative to `│ │` below.
- Nested boxes inside an outer box — inner box `┐` drifts from inner
  box `│` because the outer `│` + inner `─` mix differently from
  outer `│` + inner content.

## How to Generate Diagrams

Always use the bundled helper script rather than hand-drawing boxes.
Run it with `bun` to generate diagram fragments, then paste into markdown.

### The Helper Functions

The script at `scripts/box-helpers.ts` exports these functions:

```typescript
box(indent, width, lines)     → string[]  // Complete box with ┌┐└┘
padR(str, width)               → string    // Right-pad to exact width
vert(col)                      → string    // │ at exact column
arrow(col)                     → string    // ▼ at exact column
sideBySide(boxes, gap)         → string[]  // Merge boxes horizontally
```

### Usage Pattern

Write a `bun -e` script that builds the diagram, then copy the output
into your markdown:

```bash
bun -e '
const { box, vert, arrow } = require("./scripts/box-helpers.ts");

const appBox = box(2, 48, [
  " APPLICATION LAYER",
  " src/application/",
  "",
  " recap.ts: collect → orchestrate → apply",
]);

const out = [
  ...appBox,
  vert(21),      // │ at col 21 (below the box)
  arrow(21),     // ▼ at col 21
];

console.log(out.join("\n"));
'
```

### Connecting Boxes Vertically

Place `│` and `▼` on their own lines between boxes. The column should
be computed, not hardcoded:

```typescript
const appBox = box(2, 48, [...]); // left=2, right=49
const center = 2 + Math.floor(48 / 2); // col 26

const out = [
  ...appBox,
  vert(center),
  arrow(center),
  ...nextBox,
];
```

### Flowing a Line Alongside a Box

When a vertical line needs to flow alongside a box (e.g., a timer
connection running next to a larger box), pad each box line to the
line's column:

```typescript
const TIMER_COL = 50;
const hooksBox = box(2, 48, [...]);

// Add timer │ to the right of each hooks box line
for (const l of hooksBox) {
  out.push(padR(l, TIMER_COL) + "│");
}
```

### Side-by-Side Boxes

When placing boxes next to each other, use `sideBySide`:

```typescript
const left = box(2, 24, [" AppLayer", " (services)", ""]);
const right = box(32, 24, [" Own runtime", " ├─ Handlers", ""]);
const merged = sideBySide([left, right], 6); // 6-char gap
```

This avoids the `┐ ┌` gap problem by handling padding programmatically.

### Nested Content Without Nested Boxes

Instead of nesting boxes inside a box (which causes `─`/space drift
between the inner box borders and content), use indented text:

**Don't:**
```
│ ┌────────────┐ ┌─────────────┐ │
│ │ GitCollect. │ │ ClaudeSess. │ │   ← inner ┐ drifts from │ below
│ │ git log     │ │ ~/.claude/  │ │
│ └────────────┘ └─────────────┘ │
```

**Do:**
```
│   GitCollector     git log, git branch     │
│   ClaudeSession    ~/.claude/projects/     │
│   CodexHistory     ~/.codex/history.jsonl  │
```

## Validation

After generating a diagram, verify alignment by checking that:
1. Every `┌` has a `┐` at the exact column = indent + width - 1
2. Every interior line has `│` at both left (indent) and right (indent + width - 1)
3. The `└┘` bottom matches the `┌┐` top width exactly
4. No `┬` or `┴` in box borders (use plain `└──┘` + `│` below)
5. No nested side-by-side boxes inside an outer box
6. Any `└───┘` connector has `└` and `┘` at the exact `│` columns above

If the project has `scripts/validate-ascii-diagrams.ts`, run it to check.

## Style Guidelines

- Maximum line width: 78 characters (leaves room for 2-char markdown indent)
- Use box-drawing characters only: `─ │ ┌ ┐ └ ┘ ├ ┤`
- Arrows: `▼ ▲ ▶ ◀` for direction, `──▶` for horizontal connections
- `◀` for incoming connections (e.g., `│ inbox │◀────┘`)
- Keep text inside boxes left-aligned with 1 space indent
- Empty lines inside boxes for visual grouping
- Labels outside boxes for annotations (e.g., "etch recap run")
