# Responsive Rules — Behavior Patterns by Breakpoint

Responsive is not "shrinking". It is reorganizing the hierarchy for a different context. A mobile device is not a shrunken desktop — it is a screen held in one hand on the subway.

---

## Reference Breakpoints

```css
/* Mobile first — base styles are mobile */
/* Tablet  */ @media (min-width: 768px)  { }
/* Desktop */ @media (min-width: 1024px) { }
/* Wide    */ @media (min-width: 1440px) { }
```

**Convention**: mobile first. Default styles target the smallest screen. Complexity is added upward, not downward.

---

## What Changes by Breakpoint

### Mobile (< 768px) — One Column, Touch, Speed

| Element | Behavior |
|---|---|
| **Layout** | Everything in one column. No exceptions. |
| **Navigation** | Hamburger menu or bottom nav. Desktop links disappear. |
| **Typography** | `clamp()` does the work. No media query on typography if the clamp() values are well calibrated. |
| **Section spacing** | `--space-2xl` (48px) instead of `--space-4xl` (96px) |
| **Component spacing** | Unchanged. Card padding stays `--space-lg`. |
| **Card grid** | 1 column. `grid-template-columns: 1fr;` |
| **Images** | Full width. No side padding on hero images. |
| **CTA buttons** | `width: 100%` if standalone (hero CTA). Normal size if inline. |
| **Forms** | Every field at full width. No side-by-side fields. |
| **Touch targets** | Minimum 44×44px. Check buttons, nav links, clickable icons. |

### Tablet (768px → 1023px) — Two Columns, Mixed

| Element | Behavior |
|---|---|
| **Layout** | 2 columns possible. No more. |
| **Navigation** | Desktop-style OK if ≤5 links. Otherwise hamburger. |
| **Card grid** | `grid-template-columns: repeat(2, 1fr);` |
| **Hero** | Can switch to side-by-side (text + image) if content allows |
| **Section spacing** | `--space-3xl` (64px) |

### Desktop (1024px → 1439px) — Full Layout

| Element | Behavior |
|---|---|
| **Layout** | Up to 3-4 columns depending on content |
| **Navigation** | Complete with all links |
| **Card grid** | `repeat(3, 1fr)` or `repeat(auto-fit, minmax(280px, 1fr))` |
| **Section spacing** | `--space-4xl` (96px) |
| **Container** | Max-width active, centered |

### Wide (≥ 1440px) — Content Stops Stretching

| Element | Behavior |
|---|---|
| **Container** | `max-width: var(--container-max)` is firm. The background continues, the content stops. |
| **Grid** | Maximum 4 columns. No 5-6 columns on wide — it dilutes attention. |
| **Full-bleed images** | Continue to stretch (no max-width on full-width sections) |
| **Typography** | `clamp()` reaches its max. Size stops growing. |

---

## Grid Patterns

### Auto-responsive Grid (most common)

```css
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: var(--space-lg);
}
```

Advantage: zero media queries. Columns are added/removed based on width.
Drawback: no exact control over the number of columns.

### Breakpoint-controlled Grid

```css
.grid-controlled {
  display: grid;
  grid-template-columns: 1fr;
  gap: var(--space-md);
}

@media (min-width: 768px) {
  .grid-controlled {
    grid-template-columns: repeat(2, 1fr);
    gap: var(--space-lg);
  }
}

@media (min-width: 1024px) {
  .grid-controlled {
    grid-template-columns: repeat(3, 1fr);
  }
}
```

### Asymmetric Layout (text + image)

```css
.split {
  display: grid;
  grid-template-columns: 1fr;
  gap: var(--space-xl);
  align-items: center;
}

@media (min-width: 768px) {
  .split {
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3xl);
  }
}

/* Variant: large text / small image */
@media (min-width: 768px) {
  .split--text-heavy {
    grid-template-columns: 3fr 2fr;
  }
}
```

---

## Responsive Typography via clamp()

Reminder: if the `clamp()` values are well calibrated, no media queries are needed for typography.

```css
/* Formula: clamp(min, preferred, max) */
/* The preferred value is a combination of rem + vw */

--text-display: clamp(2.25rem, 1.5rem + 3vw, 4rem);
--text-h1:      clamp(1.75rem, 1.25rem + 2vw, 3rem);
--text-h2:      clamp(1.375rem, 1rem + 1.5vw, 2.25rem);
--text-h3:      clamp(1.125rem, 0.9rem + 1vw, 1.75rem);
--text-body:    clamp(0.9375rem, 0.875rem + 0.25vw, 1.0625rem);
--text-small:   clamp(0.8125rem, 0.78rem + 0.15vw, 0.875rem);
```

**Rule**: body barely varies (15px → 17px). Headings vary a lot (36px → 64px for display). The heading/body ratio increases on large screens.

---

## Responsive Navigation

```
Mobile (< 768px):
┌──────────────────────┐
│ Logo     ☰ Hamburger │
└──────────────────────┘

Tablet (768px+):
┌────────────────────────────────┐
│ Logo   Link Link Link    [CTA]│
└────────────────────────────────┘

Wide (1440px+):
           ┌────────────────────────────────┐
           │ Logo   Link Link Link    [CTA]│
           └────────────────────────────────┘
           (centered within the max container)
```

The mobile hamburger menu must:
- Occupy full screen (not a small dropdown)
- Have links in large size (display font, not body)
- Have a visible close button
- Close on link click

---

## Responsive Anti-patterns

| Mistake | Why | Fix |
|---|---|---|
| Horizontal scroll on mobile | Content or image wider than the viewport | `max-width: 100%` on everything, `overflow-x: hidden` on body as a last resort |
| Text too small on mobile | `clamp()` goes too low | Minimum 15px for body, 12px for small |
| Hover states as the only indicator | No hover on touch devices | Always a distinct active visual state (not just `:hover`) |
| 3+ columns on tablet | Too dense for 768px | Max 2 columns between 768-1023px |
| Container without mobile padding | Text flush against edges | `padding-inline: var(--container-padding)` with a clamp that gives at least 16px |
| Buttons too small for fingers | Target < 44px | `min-height: 44px; min-width: 44px` on every clickable element |

---

## Responsive Checklist

Before delivering, test at these widths: 375px, 768px, 1024px, 1440px.

- [ ] No horizontal scroll at any width
- [ ] Navigation works on mobile (menu visible, clickable, closable)
- [ ] Cards transition from 1 → 2 → 3 columns correctly
- [ ] Body text remains readable (≥ 15px) on mobile
- [ ] CTA buttons are large enough for touch (44px minimum)
- [ ] Images do not overflow (`max-width: 100%`)
- [ ] Section spacing shrinks on mobile
- [ ] Container has side padding on mobile (no text flush against the edge)
