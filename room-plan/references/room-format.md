# Room format

All lengths in metres. [`../examples/attic-bedroom.json`](../examples/attic-bedroom.json) is a full example: an attic room with a flat ceiling, an upper slope, a flat ledge, a lower slope with a roof window, and a closet front on the right.

## Axes

Stand inside, facing the **back** wall. The **left** wall is on your left, the **right** wall on your right, the **front** wall behind you.

- `x` runs along the back wall, from the left wall (0) to the right wall (`width`).
- `y` runs from the back wall (0) to the front wall (`length`).
- `z` runs up from the floor.

## Fields

| Field | Meaning |
|---|---|
| `title` | Heading of both sheets. |
| `lang` | `fr` or `en`: language of the labels the script writes. Default `fr`. |
| `width`, `length` | Floor size along `x` and `y`. |
| `ceiling` | `[[x, z], ...]` from `x = 0` to `x = width`, `x` never decreasing: the ceiling height across the room. A flat ceiling is `[[0, h], [width, h]]`. |
| `walls.back`, `walls.front`, `walls.left`, `walls.right` | Each `{ "name", "paint", "reason", "openings" }`. `paint` defaults to `true`; `reason` says why a wall is not painted. |
| `openings[]` | `{ "label", "at", "width", "height", "sill", "hinge" }`. `at` is the edge nearest the origin: its `x` on the back and front walls, its `y` on the side walls. `sill` defaults to 0 (a door). `hinge` is `start` or `end` (the `at` edge or the far edge) for a door that swings into the room; leave it out for no swing arc. |
| `ceiling_openings[]` | `{ "label", "segment", "from_back", "across", "along", "along_start", "reveal" }`. `segment` indexes the ceiling profile's segments from 0. `from_back` is the `y` of its edge nearest the back wall, `across` its size along `y`, `along` its size along the slope. `along_start` defaults to centring it on the segment. `reveal` is the shaft depth, 0 by default. |
| `coats` | Coats of paint for the totals. Default 2. |
| `notes` | Lines printed under the table: each value that is not a tape measurement, each surface left out. |
