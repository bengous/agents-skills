---
name: room-plan
description: Draw a printable two-page line plan of a room from its measurements (3D view, plan from above, every wall seen from inside, dimensions in cm) and compute its paint areas. Use when the user wants a plan, drawing or elevations of a real room, needs wall or ceiling areas for paint or wallpaper, or wants that plan printed.
license: "MIT. Copyright (c) 2026 Augustin BENGOLEA. Full terms in LICENSE at https://github.com/bengous/agents-skills"
---

# Room plan

The room is data: one JSON file describes it, and `scripts/room_plan.py` turns it into an A4 PDF. Sheet 1 holds the 3D view and the plan from above; sheet 2 holds the four walls seen from inside and the area table. The model covers a rectangular room whose ceiling profile runs across its width and stays the same from the back wall to the front wall: a flat ceiling, an attic with slopes and a flat ledge, a roof window in a slope.

## Steps

1. **Measure.** Write the room JSON from what the user gives and what the project holds; [`references/room-format.md`](references/room-format.md) is the format and its axes. Ask the user for each missing value. Done when every wall, opening and ceiling break has a number, and every number not taken with a tape (read off a photo, assumed, modelled) has its line in `notes`.
2. **Draw.** Run `uv run <skill-dir>/scripts/room_plan.py room.json --out plan.pdf`. It prints the wall and ceiling totals.
3. **Check.** Render with `pdftoppm -r 110 -png plan.pdf <scratch>/plan` and read both PNGs. Done when no text crosses a line or another text, and each dimension sits outside the shape it measures or on its own chain. A room the generic layout does not fit gets a copy of the script next to its JSON, edited there; `dim()` shifts a dimension along the left normal of `p0 -> p1`, so order the points with the shape on the right for an outside dimension.
4. **Report.** Give the user the net wall total, the ceiling total and the totals for the coats; then each assumption that moves a total, the same lines as `notes`.
5. **Print**, only when the user asked: [`references/printing.md`](references/printing.md).

## Area rules

- A wall's gross area is the polygon under its ceiling line: a gable for the back and front walls, a rectangle for the sides.
- Net = gross minus each opening's width × height. Window and door reveals are not counted unless the user gives their depth.
- A ceiling segment counts its true width along the slope × the room length. A roof window removes along × across and adds its reveal, 2 × (along + across) × depth.
- A wall with `"paint": false` (a closet front, a tiled wall) is drawn and named in the notes, never counted.
