# Worked Example: Multi-Layer Architecture Diagram

This is a real example from the Etch project's `docs/architecture/system-overview.md`.
It shows the patterns and pitfalls encountered when building a non-trivial diagram.

## The Goal

Show event sources (3 boxes) flowing into a hooks pipeline, alongside a
timer connection, then down through application/services/domain layers.

## The Script

```typescript
const { writeFileSync } = require("fs");
const { box, padR, vert, arrow } = require("../scripts/box-helpers.ts");

const HW = 48, HI = 2;    // main boxes: width 48, indent 2
const TC = 50;              // timer │ column (derived from top box layout)
const CENTER = 21;          // center connector for lower boxes

// Timer │ helper — pads any line and adds │ at TC
function t(line) { return padR(line, TC) + "│"; }

const o = [];

// ── Top row: 3 event source boxes ──────────────────────
// No ┬ in bottom borders — just └──┘
o.push("  ┌────────────────┐ ┌──────────────────┐ ┌────────────────┐");
o.push("  │ Claude Code    │ │ Claude Code      │ │ Timer systemd/ │");
o.push("  │ (Stop hook)    │ │ (SubagentStop)   │ │ launchd/TaskSc │");
o.push("  └────────────────┘ └──────────────────┘ └────────────────┘");

// Vertical connectors — col 9, 30, 50 (derived from box centers)
o.push(padR(padR(vert(9), 30) + "│", TC) + "│");
o.push(padR(padR(arrow(9), 30) + "▼", TC) + "│");

// ── Hooks box with timer │ flowing alongside ───────────
for (const l of box(HI, HW, [
  " HOOKS LAYER",
  " src/hooks/",
  " ...",
])) {
  o.push(t(l));  // each line gets timer │ at col 50
}

// ── Connector to inbox, timer │ continues ──────────────
o.push(t("            │ insertInboxMessage()"));
o.push(t(arrow(12)));
o.push(t("  ┌──────────────────┐"));

// Timer arrow joins the inbox box
o.push("  │ SQLite: inbox    │◀" + "─".repeat(TC - 23) + "┘");
o.push("  └──────────────────┘     etch recap run");
o.push(vert(11) + " drain()");
o.push(arrow(11));

// ── Lower boxes: plain box() + vert/arrow ──────────────
for (const l of box(HI, HW, [
  " APPLICATION LAYER",
  " src/application/",
])) o.push(l);

o.push(vert(CENTER));
o.push(arrow(CENTER));

console.log(o.join("\n"));
```

## Key Decisions

### Why no `┬` in bottom borders?

`─` and space render at different pixel widths in most editor fonts.
After 20+ `─` chars, a `┬` appears visually shifted compared to a `│`
on the next (space-padded) line. Removing `┬` eliminates the reference
point that makes drift visible.

### Why `t()` helper for flowing alongside?

The timer `│` flows down the right side of the hooks box. Rather than
counting columns by hand (which inevitably drifts), `t(line)` pads
any line to `TC` and appends `│`. Every timer pipe is guaranteed to be
at column 50.

### Why indented text instead of nested boxes?

Three collector boxes side-by-side inside the recap box:
```
│ ┌────────────┐ ┌─────────────┐ │
│ │ GitCollect. │ │ ClaudeSess. │ │   ← inner ┐ drifts from │ below
```
The inner `─` chars accumulate drift relative to the inner content
`│`, which is padded with spaces. Use indented text instead:
```
│   GitCollector     git log, git branch     │
│   ClaudeSession    ~/.claude/projects/     │
```

### Why `└─── same Bun port ───┘` works

When connecting two `│` with a horizontal line:
```
              │                             │
              └─────── same Bun port ───────┘
```
This works because `└` and `┘` are at the **exact columns** of the `│`
above them. The `─` drift between them doesn't matter — there's no
reference point to misalign with on the same line. Just compute the
width: `right_col - left_col` characters total from `└` to `┘`.
