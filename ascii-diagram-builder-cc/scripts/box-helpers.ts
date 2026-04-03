/**
 * ASCII box-drawing helpers for generating perfectly aligned diagrams.
 *
 * Usage from bun -e:
 *   const { box, padR, vert, arrow, sideBySide } = require("./scripts/box-helpers.ts");
 *
 * Or import directly:
 *   import { box, padR, vert, arrow, sideBySide } from "./scripts/box-helpers.ts";
 */

/** Right-pad a string with spaces to exact width. */
export function padR(s: string, n: number): string {
  return s + " ".repeat(Math.max(0, n - s.length));
}

/**
 * Draw a complete box with ┌┐└┘ borders.
 *
 * @param indent - Number of leading spaces
 * @param width  - Total box width including both │ borders
 * @param lines  - Content lines (will be padded/truncated to fit)
 * @returns Array of strings, one per line
 */
export function box(indent: number, width: number, lines: string[]): string[] {
  const p = " ".repeat(indent);
  const inner = width - 2;
  const out: string[] = [];
  out.push(p + "┌" + "─".repeat(inner) + "┐");
  for (const l of lines) {
    const t = l.slice(0, inner);
    out.push(p + "│" + t + " ".repeat(inner - t.length) + "│");
  }
  out.push(p + "└" + "─".repeat(inner) + "┘");
  return out;
}

/**
 * Create a vertical pipe │ at an exact column.
 *
 * @param col - 0-based column position
 */
export function vert(col: number): string {
  return " ".repeat(col) + "│";
}

/**
 * Create a down arrow ▼ at an exact column.
 *
 * @param col - 0-based column position
 */
export function arrow(col: number): string {
  return " ".repeat(col) + "▼";
}

/**
 * Merge multiple box arrays side-by-side with a gap between them.
 *
 * Each box is an array of strings from box(). The first box keeps its
 * original indent. Subsequent boxes are placed after the previous box
 * plus the gap.
 *
 * All boxes must have the same number of lines. If they don't, shorter
 * boxes are padded with blank interior lines.
 *
 * @param boxes - Array of string arrays from box()
 * @param gap   - Number of spaces between boxes
 */
export function sideBySide(boxes: string[][], gap: number): string[] {
  if (boxes.length === 0) return [];
  if (boxes.length === 1) return boxes[0]!;

  const maxLen = Math.max(...boxes.map((b) => b.length));

  // Pad shorter boxes
  const padded = boxes.map((b) => {
    if (b.length >= maxLen) return b;
    const width = b[0]!.length;
    const blankLine = " ".repeat(width);
    return [...b, ...Array(maxLen - b.length).fill(blankLine)];
  });

  const result: string[] = [];
  const spacer = " ".repeat(gap);

  for (let i = 0; i < maxLen; i++) {
    const parts: string[] = [];
    for (let j = 0; j < padded.length; j++) {
      const line = padded[j]![i]!;
      if (j === 0) {
        parts.push(line);
      } else {
        // Trim the indent from subsequent boxes (they're positioned by padding the previous)
        parts.push(spacer + line.trimStart());
      }
    }
    result.push(parts.join(""));
  }

  return result;
}

// If run directly, show a demo
if (import.meta.main) {
  console.log("=== Demo: Two boxes connected vertically ===\n");

  const top = box(2, 30, [" Source", " produces events"]);
  const bottom = box(2, 30, [" Sink", " consumes events"]);
  const center = 2 + Math.floor(30 / 2);

  for (const l of top) console.log(l);
  console.log(vert(center));
  console.log(arrow(center));
  for (const l of bottom) console.log(l);

  console.log("\n=== Demo: Side-by-side boxes ===\n");

  const left = box(2, 20, [" REST", " /api/*"]);
  const right = box(2, 20, [" RPC", " /rpc"]);
  for (const l of sideBySide([left, right], 4)) console.log(l);
}
