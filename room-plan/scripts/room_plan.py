# /// script
# requires-python = ">=3.12"
# dependencies = ["matplotlib>=3.9"]
# ///
"""Two-page A4 line plan of a room, with its paint areas, from a JSON description.

    uv run room_plan.py room.json --out plan.pdf

Sheet 1: axonometric view and plan from above. Sheet 2: the four walls seen
from inside, and the area table. The room format is in references/room-format.md.
"""

import argparse
import json
import math
from dataclasses import dataclass
from itertools import pairwise
from pathlib import Path

import matplotlib

matplotlib.use("pdf")
import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages
from matplotlib.patches import Arc

LW, THIN, FS = 0.9, 0.45, 7
WALLS = ("back", "front", "left", "right")

TEXT = {
    "fr": {
        "sheet": "cotes en cm · feuille {n}/2",
        "view3d": "Vue 3D (depuis le mur de face, traits seuls)",
        "top": "Vue de dessus",
        "inside": "{name}, vu de l'intérieur",
        "unpainted": "non peint : {reason}",
        "flat": "plat",
        "slope": "pente",
        "breaks": "Traits pointillés : changements de pente du plafond.",
        "table": "Surfaces à peindre",
        "cols": ["Surface", "Calcul", "Net"],
        "gable": "pignon",
        "gross": "brut",
        "openings": "ouvertures",
        "walls_total": "TOTAL MURS",
        "ceiling_total": "TOTAL PLAFOND",
        "reveal": "tableau",
        "sides": "4 côtés",
        "coats_walls": "Murs, {n} couches",
        "coats_all": "Murs + plafond, {n} couches",
        "not_counted": "Non compté : {items}.",
        "ceiling": "plafond",
    },
    "en": {
        "sheet": "dimensions in cm · sheet {n}/2",
        "view3d": "3D view (from the front wall, lines only)",
        "top": "Plan from above",
        "inside": "{name}, seen from inside",
        "unpainted": "not painted: {reason}",
        "flat": "flat",
        "slope": "slope",
        "breaks": "Dashed lines: changes of ceiling slope.",
        "table": "Areas to paint",
        "cols": ["Surface", "Calculation", "Net"],
        "gable": "gable",
        "gross": "gross",
        "openings": "openings",
        "walls_total": "WALLS TOTAL",
        "ceiling_total": "CEILING TOTAL",
        "reveal": "reveal",
        "sides": "4 sides",
        "coats_walls": "Walls, {n} coats",
        "coats_all": "Walls + ceiling, {n} coats",
        "not_counted": "Not counted: {items}.",
        "ceiling": "ceiling",
    },
}


@dataclass(frozen=True)
class Opening:
    label: str
    at: float
    width: float
    sill: float
    height: float
    hinge: str | None


@dataclass(frozen=True)
class Wall:
    key: str
    name: str
    paint: bool
    reason: str
    openings: list[Opening]


@dataclass(frozen=True)
class CeilingOpening:
    label: str
    segment: int
    from_back: float
    across: float
    along: float
    along_start: float
    reveal: float


@dataclass(frozen=True)
class Room:
    title: str
    lang: str
    width: float
    length: float
    profile: list[tuple[float, float]]
    walls: dict[str, Wall]
    ceiling_openings: list[CeilingOpening]
    coats: int
    notes: list[str]

    def height(self, x: float) -> float:
        for (xa, za), (xb, zb) in pairwise(self.profile):
            if xa <= x <= xb:
                return za if xb == xa else za + (zb - za) * (x - xa) / (xb - xa)
        raise ValueError(f"x = {x} is outside the ceiling profile")

    def segment_length(self, i: int) -> float:
        (xa, za), (xb, zb) = self.profile[i], self.profile[i + 1]
        return math.hypot(xb - xa, zb - za)

    def on_segment(self, i: int, s: float) -> tuple[float, float]:
        """(x, z) at distance s along ceiling segment i from its left end."""
        (xa, za), (xb, zb) = self.profile[i], self.profile[i + 1]
        t = s / self.segment_length(i)
        return xa + t * (xb - xa), za + t * (zb - za)


def load(path: Path) -> Room:
    data = json.loads(path.read_text())
    width, length = data["width"], data["length"]
    profile = [(float(x), float(z)) for x, z in data["ceiling"]]
    if profile[0][0] != 0 or abs(profile[-1][0] - width) > 1e-9:
        raise ValueError("ceiling profile must run from x = 0 to x = width")
    if any(xb < xa for (xa, _), (xb, _) in pairwise(profile)):
        raise ValueError("ceiling profile x values must not decrease")
    walls = {}
    for key in WALLS:
        w = data["walls"][key]
        openings = [
            Opening(o["label"], o["at"], o["width"], o.get("sill", 0.0), o["height"], o.get("hinge")) for o in w.get("openings", [])
        ]
        walls[key] = Wall(key, w["name"], w.get("paint", True), w.get("reason", ""), openings)
    room = Room(
        title=data["title"],
        lang=data.get("lang", "fr"),
        width=width,
        length=length,
        profile=profile,
        walls=walls,
        ceiling_openings=[],
        coats=data.get("coats", 2),
        notes=data.get("notes", []),
    )
    ceiling_openings = []
    for c in data.get("ceiling_openings", []):
        seg = room.segment_length(c["segment"])
        start = c.get("along_start", (seg - c["along"]) / 2)
        ceiling_openings.append(
            CeilingOpening(c["label"], c["segment"], c["from_back"], c["across"], c["along"], start, c.get("reveal", 0.0))
        )
    return Room(**{**room.__dict__, "ceiling_openings": ceiling_openings})


def fmt(room: Room, v: float, digits: int = 2) -> str:
    s = f"{v:.{digits}f}"
    return s.replace(".", ",") if room.lang == "fr" else s


def m2(room: Room, v: float) -> str:
    return f"{fmt(room, v)} m²"


def cm(v: float) -> str:
    return f"{v * 100:.0f}"


def wall_length(room: Room, key: str) -> float:
    return room.width if key in ("back", "front") else room.length


def wall_outline(room: Room, key: str) -> list[tuple[float, float]]:
    """Wall outline in elevation (u, z), seen from inside, u from the viewer's left."""
    if key == "back":
        return [(room.width, 0), (0, 0), *room.profile, (room.width, 0)]
    if key == "front":
        return [(0, 0), (room.width, 0), *[(room.width - x, z) for x, z in room.profile], (0, 0)]
    h = room.height(0 if key == "left" else room.width)
    return [(0, 0), (room.length, 0), (room.length, h), (0, h), (0, 0)]


def ceiling_line(room: Room, key: str) -> list[tuple[float, float]]:
    """Top edge of a wall in elevation, (u, z) with u increasing."""
    if key == "back":
        return list(room.profile)
    if key == "front":
        return [(room.width - x, z) for x, z in reversed(room.profile)]
    h = room.height(0 if key == "left" else room.width)
    return [(0.0, h), (room.length, h)]


def opening_u(room: Room, key: str, o: Opening) -> tuple[float, float]:
    """Opening's u range in its wall's elevation. `at` runs from the left wall (back, front) or the back wall (sides)."""
    n = wall_length(room, key)
    if key in ("back", "right"):
        return o.at, o.at + o.width
    return n - o.at - o.width, n - o.at


def polygon_area(pts: list[tuple[float, float]]) -> float:
    return abs(sum(xa * yb - xb * ya for (xa, ya), (xb, yb) in pairwise(pts))) / 2


@dataclass(frozen=True)
class WallArea:
    wall: Wall
    gross: float
    openings: float

    @property
    def net(self) -> float:
        return self.gross - self.openings


def wall_areas(room: Room) -> list[WallArea]:
    return [WallArea(w, polygon_area(wall_outline(room, w.key)), sum(o.width * o.height for o in w.openings)) for w in room.walls.values()]


def line(ax, pts, lw=LW, ls="-", color="black"):
    xs, ys = zip(*pts)
    ax.plot(xs, ys, color=color, lw=lw, ls=ls, solid_capstyle="round")


def dim(ax, p0, p1, offset, text, fs=FS):
    """Dimension line from p0 to p1, shifted by `offset` along the left normal of p0 -> p1."""
    (x0, y0), (x1, y1) = p0, p1
    dx, dy = x1 - x0, y1 - y0
    n = math.hypot(dx, dy)
    if n < 1e-9:
        return
    nx, ny = -dy / n, dx / n
    a = (x0 + nx * offset, y0 + ny * offset)
    b = (x1 + nx * offset, y1 + ny * offset)
    s = math.copysign(0.04, offset) if offset else 0
    if offset:
        line(ax, [(x0, y0), (a[0] + nx * s, a[1] + ny * s)], lw=0.25)
        line(ax, [(x1, y1), (b[0] + nx * s, b[1] + ny * s)], lw=0.25)
    ax.annotate("", a, b, arrowprops={"arrowstyle": "<|-|>", "lw": 0.35, "color": "black", "mutation_scale": 4, "shrinkA": 0, "shrinkB": 0})
    if not text:
        return
    angle = math.degrees(math.atan2(dy, dx))
    if angle > 90.5 or angle < -89.5:
        angle -= 180
    t = math.copysign(0.07, offset) if offset else 0
    ax.text(
        (a[0] + b[0]) / 2 + nx * t,
        (a[1] + b[1]) / 2 + ny * t,
        text,
        fontsize=fs,
        ha="center",
        va="center",
        rotation=angle,
        rotation_mode="anchor",
        bbox={"facecolor": "white", "edgecolor": "none", "pad": 0.4},
    )


def label(ax, xy, text, fs=FS, **kw):
    ax.text(*xy, text, fontsize=fs, ha="center", va="center", **kw)


def frame(ax, title):
    ax.set_aspect("equal")
    ax.axis("off")
    ax.set_title(title, fontsize=10, loc="left", pad=6)


AZIMUTH, ELEVATION = math.radians(-35), math.radians(28)


def project(x, y, z, az=AZIMUTH, el=ELEVATION):
    """Orthographic view from above the front-left corner; y runs from the back wall to the front wall."""
    yv = -y
    xr = x * math.cos(az) - yv * math.sin(az)
    yr = x * math.sin(az) + yv * math.cos(az)
    return xr, z * math.cos(el) + yr * math.sin(el)


def to_room(room: Room, key: str, u: float, z: float) -> tuple[float, float, float]:
    """Elevation point (u, z) of a wall back to room coordinates."""
    if key == "back":
        return u, 0.0, z
    if key == "front":
        return room.width - u, room.length, z
    if key == "left":
        return 0.0, room.length - u, z
    return room.width, u, z


def ceiling_corners(room: Room, c: CeilingOpening) -> list[tuple[float, float, float]]:
    (xa, za), (xb, zb) = room.on_segment(c.segment, c.along_start), room.on_segment(c.segment, c.along_start + c.along)
    y0, y1 = c.from_back, c.from_back + c.across
    return [(xa, y0, za), (xb, y0, zb), (xb, y1, zb), (xa, y1, za), (xa, y0, za)]


def axonometric(ax, room: Room, t):
    frame(ax, t["view3d"])

    def poly(pts, lw=LW, ls="-"):
        line(ax, [project(*p) for p in pts], lw=lw, ls=ls)

    W, L = room.width, room.length
    for y in (0.0, L):
        poly([(W, y, 0), (0, y, 0), *[(x, y, z) for x, z in room.profile], (W, y, 0)])
    poly([(0, 0, 0), (0, L, 0)])
    poly([(W, 0, 0), (W, L, 0)])
    for x, z in room.profile:
        poly([(x, 0, z), (x, L, z)])
    for w in room.walls.values():
        for o in w.openings:
            u0, u1 = opening_u(room, w.key, o)
            z0, z1 = o.sill, o.sill + o.height
            pts = [(u0, z0), (u0, z1), (u1, z1), (u1, z0)] + ([] if z0 == 0 else [(u0, z0)])
            poly([to_room(room, w.key, u, z) for u, z in pts], lw=THIN)
            label(ax, project(*to_room(room, w.key, (u0 + u1) / 2, (z0 + z1) / 2)), o.label, fs=6)
        u = wall_length(room, w.key) / 2
        x, y, _ = to_room(room, w.key, u, 0)
        z = (0.55 if w.key in ("front", "right") else 0.2) * room.height(x)
        label(ax, project(x, y, z), w.name if w.paint else f"{w.name}\n({w.reason})", fs=6)
    for c in room.ceiling_openings:
        corners = ceiling_corners(room, c)
        poly(corners, lw=THIN)
        cx = sum(p[0] for p in corners[:4]) / 4
        cy = sum(p[1] for p in corners[:4]) / 4
        cz = sum(p[2] for p in corners[:4]) / 4
        label(ax, project(cx, cy, cz), c.label, fs=6)
    dim(ax, project(0, L, 0), project(W, L, 0), 0.25, f"{cm(W)} cm")
    dim(ax, project(W, L, 0), project(W, 0, 0), 0.25, f"{cm(L)} cm")
    dim(ax, project(0, 0, 0), project(0, 0, room.height(0)), 0.2, f"{cm(room.height(0))} cm")


def segment_kind(room: Room, i: int, t) -> str:
    (_, za), (_, zb) = room.profile[i], room.profile[i + 1]
    return f"{t['flat']}\nh {cm(za)}" if za == zb else f"{t['slope']}\nh {cm(za)} → {cm(zb)}"


def plan(ax, room: Room, t):
    frame(ax, t["top"])
    W, L = room.width, room.length

    def pt(x, y):
        """Plan coordinates: the back wall at the top."""
        return x, -y

    line(ax, [pt(0, 0), pt(W, 0), pt(W, L), pt(0, L), pt(0, 0)], lw=1.4)
    ends = {
        "back": (lambda a: pt(a, 0), (0, 1)),
        "front": (lambda a: pt(a, L), (0, -1)),
        "left": (lambda a: pt(0, a), (1, 0)),
        "right": (lambda a: pt(W, a), (-1, 0)),
    }
    for w in room.walls.values():
        at, (nx, ny) = ends[w.key]
        normal = (nx, -ny)
        for o in w.openings:
            a, b = at(o.at), at(o.at + o.width)
            line(ax, [a, b], lw=3.0)
            ax.plot([a[0], b[0]], [a[1], b[1]], color="white", lw=1.8)
            mid = ((a[0] + b[0]) / 2 + normal[0] * 0.12, (a[1] + b[1]) / 2 + normal[1] * 0.12)
            label(ax, mid, o.label, fs=6, rotation=0 if w.key in ("back", "front") else 90)
            if o.hinge:
                h, other = (a, b) if o.hinge == "start" else (b, a)
                leaf = (h[0] + normal[0] * o.width, h[1] + normal[1] * o.width)
                line(ax, [h, leaf], lw=THIN)
                a1 = math.degrees(math.atan2(other[1] - h[1], other[0] - h[0]))
                a2 = math.degrees(math.atan2(normal[1], normal[0]))
                t1, t2 = (a1, a2) if (a2 - a1) % 360 == 90 else (a2, a1)
                ax.add_patch(Arc(h, 2 * o.width, 2 * o.width, theta1=t1, theta2=t2, lw=0.3, ls="--"))
        if not w.paint:
            a, b = at(0), at(wall_length(room, w.key))
            line(ax, [(a[0] + normal[0] * 0.04, a[1] + normal[1] * 0.04), (b[0] + normal[0] * 0.04, b[1] + normal[1] * 0.04)], lw=THIN)
            c = ((a[0] + b[0]) / 2 + normal[0] * 0.13, (a[1] + b[1]) / 2 + normal[1] * 0.13)
            label(ax, c, w.reason, fs=6, rotation=0 if w.key in ("back", "front") else 90)
    xs = [x for x, _ in room.profile]
    for x in xs[1:-1]:
        line(ax, [pt(x, 0), pt(x, L)], lw=THIN, ls="--")
    for i, (xa, xb) in enumerate(pairwise(xs)):
        if xb > xa:
            label(ax, pt((xa + xb) / 2, L * 0.4), segment_kind(room, i, t), fs=6)
    for c in room.ceiling_openings:
        corners = ceiling_corners(room, c)
        line(ax, [pt(x, y) for x, y, _ in corners], lw=THIN, ls=":")
        label(ax, pt(sum(p[0] for p in corners[:4]) / 4, sum(p[1] for p in corners[:4]) / 4), c.label, fs=6)

    def chain_x(points, y, offset):
        pts = sorted(set(points))
        for a, b in pairwise(pts) if len(pts) > 2 else ():
            dim(ax, pt(a, y), pt(b, y), offset, cm(b - a))

    def chain_y(points, x, offset):
        pts = sorted(set(points), reverse=True)
        for a, b in pairwise(pts) if len(pts) > 2 else ():
            dim(ax, pt(x, a), pt(x, b), offset, cm(a - b))

    def edges(w: Wall) -> list[float]:
        return [p for o in w.openings for p in (o.at, o.at + o.width)]

    chain_x([0, W, *edges(room.walls["back"])], 0, 0.25)
    dim(ax, pt(0, 0), pt(W, 0), 0.55, f"{cm(W)} cm")
    chain_x([0, W, *edges(room.walls["front"])], L, -0.25)
    chain_x(xs, L, -0.55)
    chain_y([0, L, *edges(room.walls["left"])], 0, 0.3)
    dim(ax, pt(0, L), pt(0, 0), 0.6, f"{cm(L)} cm")
    chain_y([0, L, *edges(room.walls["right"])], W, -0.3)
    ceiling_y = [p for c in room.ceiling_openings for p in (c.from_back, c.from_back + c.across)]
    chain_y([0, *ceiling_y], W, -0.6)
    if len(xs) > 2:
        label(ax, pt(W / 2, L + 0.85), t["breaks"], fs=6)
    ax.set_xlim(-0.8, W + 0.8)
    ax.set_ylim(-L - 1.0, 0.9)


def elevation(ax, room: Room, key: str, t):
    w = room.walls[key]
    title = t["inside"].format(name=w.name)
    frame(ax, title if w.paint else f"{title}\n{t['unpainted'].format(reason=w.reason)}")
    outline = wall_outline(room, key)
    line(ax, outline, lw=1.2)
    n = wall_length(room, key)
    top = ceiling_line(room, key)
    for (ua, za), (ub, zb) in pairwise(top):
        if za != zb:
            dim(ax, (ua, za), (ub, zb), 0.17, cm(math.hypot(ub - ua, zb - za)))
    us = sorted({u for u, _ in top})
    for a, b in pairwise(us):
        dim(ax, (a, 0), (b, 0), -0.25, cm(b - a))
    left_h, right_h = top[0][1], top[-1][1]
    dim(ax, (0, 0), (0, left_h), 0.28, cm(left_h))
    if right_h != left_h:
        dim(ax, (n, 0), (n, right_h), -0.28, cm(right_h))
    groups: list[list[tuple[float, float]]] = []
    for u, z in top[1:-1]:
        if groups and groups[-1][-1][1] == z:
            groups[-1].append((u, z))
        else:
            groups.append([(u, z)])
    for g in groups:
        z = g[0][1]
        if z in (left_h, right_h):
            continue
        u = sum(p[0] for p in g) / len(g)
        dim(ax, (u, z), (u, 0), 0.0, cm(z))
    for o in w.openings:
        u0, u1 = opening_u(room, key, o)
        z0, z1 = o.sill, o.sill + o.height
        line(ax, [(u0, z0), (u0, z1), (u1, z1), (u1, z0)] + ([] if z0 == 0 else [(u0, z0)]))
        label(ax, ((u0 + u1) / 2, (z0 + z1) / 2), o.label)
        dim(ax, (u0, z1), (u1, z1), 0.12, cm(o.width))
        dim(ax, (u0, z0), (u0, z1), 0.15, cm(o.height))
        if z0 > 0:
            dim(ax, (u0, 0), (u0, z0), 0.15, cm(z0))
            if u0 > 0.15:
                dim(ax, (0, max(0.15, z0 - 0.2)), (u0, max(0.15, z0 - 0.2)), 0.0, cm(u0))
    ax.set_xlim(-0.6, n + 0.6)
    ax.set_ylim(-0.55, max(z for _, z in outline) + 0.45)


def table(ax, room: Room, t):
    ax.axis("off")
    ax.set_title(t["table"], fontsize=10, loc="left", pad=6)

    def f(v):
        return fmt(room, v)

    walls = wall_areas(room)
    rows: list[tuple[str, str, str]] = []
    for a in walls:
        if not a.wall.paint:
            continue
        if a.wall.key in ("back", "front") and len({z for _, z in room.profile}) > 1:
            calc = f"{t['gable']} {m2(room, a.gross)}"
        else:
            h = room.height(room.width if a.wall.key == "right" else 0)
            calc = f"{f(wall_length(room, a.wall.key))} × {f(h)}"
        for o in a.wall.openings:
            calc += f" − {o.label} {f(o.width)} × {f(o.height)}"
        rows.append((a.wall.name, calc, m2(room, a.net)))
    painted = [a for a in walls if a.wall.paint]
    total_walls = sum(a.net for a in painted)
    gross_openings = (
        f"{t['gross']} {m2(room, sum(a.gross for a in painted))} − {t['openings']} {m2(room, sum(a.openings for a in painted))}"
    )
    rows.append((t["walls_total"], gross_openings, m2(room, total_walls)))
    ceiling = 0.0
    for i in range(len(room.profile) - 1):
        width = room.segment_length(i)
        if width < 1e-9:
            continue
        area = width * room.length
        ceiling += area
        kind = segment_kind(room, i, t).replace("\n", " ")
        rows.append((f"  {t['ceiling']} {kind}", f"{f(width)} × {f(room.length)}", m2(room, area)))
    for c in room.ceiling_openings:
        ceiling -= c.along * c.across
        rows.append((f"  − {c.label}", f"{f(c.along)} × {f(c.across)}", f"− {m2(room, c.along * c.across)}"))
        if c.reveal:
            reveal = 2 * (c.along + c.across) * c.reveal
            ceiling += reveal
            rows.append((f"  + {t['reveal']} {c.label}", f"{t['sides']}, {f(c.along)} / {f(c.across)} × {f(c.reveal)}", m2(room, reveal)))
    rows.append((t["ceiling_total"], "", m2(room, ceiling)))
    rows.append((t["coats_walls"].format(n=room.coats), "", m2(room, room.coats * total_walls)))
    rows.append((t["coats_all"].format(n=room.coats), "", m2(room, room.coats * (total_walls + ceiling))))
    tab = ax.table(cellText=[list(r) for r in rows], colLabels=t["cols"], loc="upper left", colWidths=[0.30, 0.54, 0.16], cellLoc="left")
    tab.auto_set_font_size(False)
    tab.set_fontsize(7)
    tab.scale(1, 1.05)
    bold = {t["walls_total"], t["ceiling_total"], t["coats_walls"].format(n=room.coats), t["coats_all"].format(n=room.coats)}
    for (r, _), cell in tab.get_celld().items():
        cell.set_linewidth(0.3)
        if r == 0 or rows[r - 1][0] in bold:
            cell.set_text_props(fontweight="bold")
    excluded = [a.wall.reason or a.wall.name for a in walls if not a.wall.paint]
    notes = ([t["not_counted"].format(items=", ".join(excluded))] if excluded else []) + room.notes
    ax.text(0, -0.13, "\n".join(notes), fontsize=6.3, va="top", ha="left", transform=ax.transAxes, linespacing=1.5, wrap=True)
    return total_walls, ceiling


def header(fig, room: Room, t, n):
    fig.text(0.03, 0.965, room.title, fontsize=13, fontweight="bold")
    fig.text(0.97, 0.965, t["sheet"].format(n=n), fontsize=8, ha="right")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("room", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    room = load(args.room)
    t = TEXT[room.lang]
    side = room.length / (room.length + room.width)
    with PdfPages(args.out) as pdf:
        fig = plt.figure(figsize=(11.69, 8.27))
        header(fig, room, t, 1)
        axonometric(fig.add_axes((0.02, 0.05, 0.50, 0.86)), room, t)
        plan(fig.add_axes((0.52, 0.05, 0.46, 0.86)), room, t)
        pdf.savefig(fig)
        plt.close(fig)

        fig = plt.figure(figsize=(11.69, 8.27))
        header(fig, room, t, 2)
        elevation(fig.add_axes((0.02, 0.50, 0.47, 0.43)), room, "back", t)
        elevation(fig.add_axes((0.51, 0.50, 0.47, 0.43)), room, "front", t)
        elevation(fig.add_axes((0.01, 0.04, 0.44 * side + 0.02, 0.40)), room, "left", t)
        elevation(fig.add_axes((0.03 + 0.44 * side, 0.04, 0.44 * side + 0.02, 0.40)), room, "right", t)
        walls, ceiling = table(fig.add_axes((0.50, 0.19, 0.48, 0.27)), room, t)
        pdf.savefig(fig)
        plt.close(fig)
    print(f"walls {walls:.2f} m², ceiling {ceiling:.2f} m² -> {args.out}")


if __name__ == "__main__":
    main()
