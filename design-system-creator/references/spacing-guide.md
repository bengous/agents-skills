# Spacing Guide — When to Use Which Token

Spacing is not decorative. It creates visual hierarchy: what is close is related, what is distant is separate. This guide maps spacing tokens to usage contexts.

---

## Core Principle

**Proximity creates relationship.** Two elements that are close are perceived as related. Two elements that are far apart are perceived as independent. Spacing encodes structure without needing borders or separators.

---

## Reference Scale (base 4px)

| Token | Value | Ratio |
|---|---|---|
| `--space-xs` | 4px | ×1 |
| `--space-sm` | 8px | ×2 |
| `--space-md` | 16px | ×4 |
| `--space-lg` | 24px | ×6 |
| `--space-xl` | 32px | ×8 |
| `--space-2xl` | 48px | ×12 |
| `--space-3xl` | 64px | ×16 |
| `--space-4xl` | 96px | ×24 |
| `--space-5xl` | 128px | ×32 |

If the base is 8px, double all values. What matters is the ratio between levels, not the absolute values.

---

## Mapping by Context

### Micro (xs → sm) — Inside a Component

Spacing between elements that form an inseparable whole.

| Context | Token | Why |
|---|---|---|
| Icon ↔ label in a button | `--space-xs` (4px) | They are one unit — the space maintains readability without breaking unity |
| Gap between inline list items (tags, badges) | `--space-xs` to `--space-sm` | Items in the same group, visually related |
| Vertical padding of a badge/tag | `--space-xs` (4px) | Compact, dense |
| Label ↔ form field spacing | `--space-xs` (4px) | The label is attached to its field |

### Small (sm → md) — Between Elements in the Same Block

Spacing between related but distinct elements.

| Context | Token | Why |
|---|---|---|
| Internal button padding (vertical) | `--space-sm` (8px) | The text needs breathing room without inflating the button |
| Internal button padding (horizontal) | `--space-md` to `--space-lg` | Proportional width for a comfortable click area |
| Gap between form fields | `--space-md` (16px) | Separate fields without losing the form context |
| Space between paragraphs | `--space-md` (16px) | Typographic standard |
| Gap in a card grid | `--space-md` to `--space-lg` | Cards are related (same collection) but independent |

### Medium (lg → xl) — Component Padding

Internal spacing of containers.

| Context | Token | Why |
|---|---|---|
| Internal card padding | `--space-lg` (24px) | Enough breathing room for the content |
| Navigation padding | `--space-md` vertical, `--space-lg` horizontal | The nav is content-dense but must remain airy |
| Space between a heading and its content | `--space-md` to `--space-lg` | The heading introduces, the content follows — related but hierarchical |
| Internal container padding (horizontal) | `var(--container-padding)` | Use the layout token, not a spacing token |

### Large (2xl → 3xl) — Between Sections on the Same Page

Spacing between independent blocks that form a page.

| Context | Token | Why |
|---|---|---|
| Margin between a section heading and the previous section | `--space-2xl` to `--space-3xl` | Clear separation — new section = new context |
| Section heading ↔ first element of the section | `--space-lg` to `--space-xl` | Closer than the section margin but wider than component padding |
| Space between the hero and the first section | `--space-3xl` (64px) | The hero is self-contained, the next section is a new block |

### Very Large (4xl → 5xl) — Structural Breathing Room

Spacing reserved for major visual breaks.

| Context | Token | Why |
|---|---|---|
| Vertical padding of full-width sections | `--space-4xl` (96px) on desktop | A section must own its space — padding creates breathing room |
| Space before the footer | `--space-4xl` to `--space-5xl` | The footer is a separate world — the transition must be marked |
| Separation margin between major thematic blocks | `--space-5xl` (128px) | Rare — only between 2-3 large zones of the site |

---

## Responsive Patterns

Spacing shrinks on mobile. Not proportionally — small tokens stay stable, large ones compress.

```css
/* Typical pattern: sections shrink, components keep their padding */
.section {
  padding-block: var(--space-4xl);  /* 96px desktop */
}
@media (max-width: 768px) {
  .section {
    padding-block: var(--space-2xl);  /* 48px mobile — halved */
  }
}

.card {
  padding: var(--space-lg);  /* 24px — same on mobile and desktop */
}
```

**Rule**: below `--space-lg`, spacing does not change between mobile and desktop. Above it, spacing drops by one or two steps.

---

## Anti-patterns

| Mistake | Why It Is a Problem | Fix |
|---|---|---|
| Same spacing everywhere (e.g., everything at 16px) | No visual hierarchy — everything sits at the same level | Use at least 3 distinct levels |
| Spacing between sections < internal card padding | The eye cannot tell what is a block and what is a gap | Space between sections > internal component space. Always. |
| Different card padding per card | Visual inconsistency within the same collection | One padding token per component type |
| Hardcoded margins in `px` | Impossible to maintain, inconsistent | Always use tokens |
| `margin-top` + `margin-bottom` on every element | Double margins between elements | Prefer `gap` in flex/grid, or a margin-bottom-only convention |

---

## Quick Checklist

Before delivering, verify:
- [ ] Elements inside a component use xs-sm
- [ ] Components between each other use md-lg
- [ ] Sections between each other use 2xl-4xl
- [ ] No hardcoded px values (everything in tokens)
- [ ] Section spacing shrinks on mobile
- [ ] Internal component padding stays stable on mobile
