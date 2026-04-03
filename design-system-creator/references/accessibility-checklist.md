# Accessibility Checklist — Design System Requirements

Accessibility is not a layer added after the fact. It is in every token, every component, every rule. This document lists the minimum requirements that the design system must meet.

Target level: **WCAG 2.1 AA** (the legal standard in the EU and most jurisdictions).

---

## 1. Color Contrast

### Minimum Ratios (WCAG AA)

| Text Type | Minimum Ratio | How to Verify |
|---|---|---|
| Normal text (< 18px or < 14px bold) | **4.5:1** | Text color vs background color |
| Large text (≥ 18px or ≥ 14px bold) | **3:1** | Display headings, h1, h2 |
| Non-text UI elements (icons, borders, focus rings) | **3:1** | Icons, field outlines, indicators |

### Combinations to Verify Systematically

For each design system, validate these pairs:

```
On dark background (--color-fond):
  ✓ --color-texte        vs --color-fond         → must be ≥ 4.5:1
  ✓ --color-texte-mute   vs --color-fond         → must be ≥ 4.5:1
  ✓ --color-action       vs --color-fond         → must be ≥ 3:1 (if large text/UI) or 4.5:1 (if normal text)
  ✓ --color-texte-mute   vs --color-surface      → must be ≥ 4.5:1

On light background (--color-fond-alt):
  ✓ --color-texte-alt    vs --color-fond-alt     → must be ≥ 4.5:1
  ✓ --color-action       vs --color-fond-alt     → must be ≥ 4.5:1 for links
  ✓ --color-texte-mute   vs --color-fond-alt     → must be ≥ 4.5:1 (caution, muted is often too light)

Primary button:
  ✓ Button text          vs --color-action       → must be ≥ 4.5:1
  ✓ --color-action        vs surrounding background → must be ≥ 3:1 (button distinction)
```

### Verification Tools

- [WebAIM Contrast Checker](https://webaim.org/resources/contrastchecker/) — quick check for a pair
- [Colour Contrast Analyser](https://www.tpgi.com/color-contrast-checker/) — desktop app, measures on any screen
- Chrome DevTools: Inspect → Accessibility → Contrast ratio (displayed automatically)

### Common Trap: Muted Text

`--color-texte-mute` is the token most often in violation. A gray #8A8580 on a #1C1A17 background gives 3.8:1 — below the 4.5:1 required for normal text.

**Solutions**:
- Increase the muted luminosity (e.g., #9A9590 instead of #8A8580)
- Use muted only for large text (≥ 18px) where the 3:1 ratio is sufficient
- Explicitly declare in the absolute rules: "Muted text is only used at size ≥ 18px"

---

## 2. Visible Focus

### Requirement

Every interactive element must have a **visible focus indicator** when reached via keyboard (Tab).

### CSS Rule

```css
/* Visible focus — appears only on keyboard, not on mouse click */
:focus-visible {
  outline: 2px solid var(--color-action);
  outline-offset: 2px;
}

/* Optional: remove default browser outline and replace it */
:focus:not(:focus-visible) {
  outline: none;
}
```

### What You Must NEVER Do

```css
/* FORBIDDEN — removes focus without replacement */
*:focus { outline: none; }
a:focus { outline: none; }
button:focus { outline: none; }
```

If the browser default focus is deemed "ugly", REPLACE it with a custom focus. Never remove it.

### On Dark vs Light Backgrounds

The focus ring must be visible on both. If `--color-action` is too close to the background in one of the two contexts, use a dedicated focus color:

```css
/* Fallback for light background if action is too light on light */
.on-light :focus-visible {
  outline-color: var(--color-action-pressed);
}
```

---

## 3. Touch Targets

### Minimum Size

Every clickable/tappable element must have a tap area of at least **44×44px** (WCAG) or **48×48px** (Material Design, recommended).

### Elements to Verify

| Element | Risk | Solution |
|---|---|---|
| Buttons | Rarely an issue if padding is correct | `min-height: 44px` |
| Links in text | Often too small | `padding-block: 4px; display: inline-block;` to increase the area |
| Navigation links | Can be too tight | `padding: var(--space-sm) var(--space-md)` minimum |
| Clickable icons | The icon is 24px but the tap area must be 44px | Wrapper with `padding: 10px` or `min-width: 44px; min-height: 44px` |
| Checkbox / radio | Native inputs are often too small | Custom styling with clickable label of 44px |
| Footer links | Often forgotten | Same rules as nav |

### Spacing Between Targets

Two adjacent tappable elements must have at least **8px** of space between their tap areas to prevent touch errors.

---

## 4. Reduced Motion

### Requirement

Users who have enabled "Reduce motion" in their OS must see no animation.

```css
@media (prefers-reduced-motion: reduce) {
  *,
  *::before,
  *::after {
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
    scroll-behavior: auto !important;
  }
}
```

This block is **mandatory** and must be present in the design system's global CSS.

**Caution**: JS animations (GSAP, Framer Motion) are NOT affected by this CSS. If the project uses JS animations, check `window.matchMedia('(prefers-reduced-motion: reduce)').matches` and disable animations programmatically.

---

## 5. Accessible Typography

### Minimum Size

| Context | Minimum Size |
|---|---|
| Body text | 15px (0.9375rem) |
| Small text (captions, metadata) | 13px (0.8125rem) |
| Form labels | 14px (0.875rem) |
| Button text | 14px (0.875rem) |

Below 13px, text is unreadable for a significant portion of the population.

### Minimum Line-height

| Usage | Line-height |
|---|---|
| Body text | ≥ 1.5 (WCAG recommends 1.5 minimum for running text) |
| Headings | ≥ 1.1 (short headings can be tighter) |

### Paragraph Spacing

WCAG recommends paragraph spacing of at least 1.5× the text size. With `--text-body` at 16px, that means `margin-bottom: 24px` minimum between paragraphs.

---

## 6. Color and Meaning

### Color Must Never Be the Only Indicator

| Case | Bad | Correct |
|---|---|---|
| Field in error | Red border only | Red border + error icon + error text |
| Link in text | Color only (no underline) | Color + underline (or at minimum on hover) |
| Status (success/error) | Green/red dot | Dot + text label ("Active" / "Error") |
| Disabled button | Grayed out only | Grayed out + `disabled` attribute + `cursor: not-allowed` |

### Links in Text

Links in body text must be distinguishable without color:
- Underline by default (the safest)
- OR underline on hover + different font weight (acceptable but less accessible)

---

## 7. Semantic Structure

The design system must encourage correct use of semantic HTML. This is not CSS, but components must be documented with their expected tags.

| Component | Expected Tag | Not This |
|---|---|---|
| Navigation | `<nav>` with `<ul>` / `<li>` | `<div>` with loose `<a>` tags |
| Button | `<button>` (action) or `<a>` (navigation) | `<div onclick>` or `<span>` |
| Heading | `<h1>` → `<h6>` in order | `<div class="title">` or `<p class="big">` |
| Form | `<form>` with `<label for>` + `<input id>` | `<div>` with `<span>` as labels |
| Image | `<img alt="...">` | `<div>` with background-image without alt |
| Section | `<section>` or `<article>` | `<div class="section">` |

### Heading Hierarchy

One single `<h1>` per page. Levels never skip (no h1 → h3 without h2).

---

## 8. Images

### Alt Attribute

| Image Type | Alt Attribute |
|---|---|
| Informative image (product, illustration) | Description of the content: `alt="Patinated zinc coffee table with copper legs"` |
| Decorative image (texture, background, separator) | `alt=""` (empty, not absent) |
| Image-link | Description of the destination: `alt="View the lighting collection"` |
| Logo | `alt="[Brand name]"` |

### CSS Background Images

If an image is set via `background-image` and carries meaning, add a `role="img"` and `aria-label` attribute to the element.

---

## Validation Checklist

### Per Component

- [ ] **Buttons**: focus-visible defined, min-height 44px, text ≥ 14px, text/background contrast ≥ 4.5:1
- [ ] **Links**: distinguishable without color (underline), focus-visible, contrast ≥ 4.5:1
- [ ] **Form fields**: associated label, focus-visible, error state with text (not just color), min-height 44px
- [ ] **Interactive cards**: full click area, focus-visible on the card
- [ ] **Navigation**: `<nav>` semantic, focus-visible on every link, mobile menu keyboard-accessible
- [ ] **Images**: alt on all `<img>`, empty alt for decorative ones

### Global

- [ ] `prefers-reduced-motion: reduce` is implemented
- [ ] No `outline: none` without replacement
- [ ] All color/background combinations validated at ≥ 4.5:1 (or ≥ 3:1 for large text)
- [ ] Muted text has sufficient ratio on all backgrounds
- [ ] Touch targets are ≥ 44px
- [ ] Body text is ≥ 15px
- [ ] Body line-height is ≥ 1.5
- [ ] Heading hierarchy respected (unique h1, no level skipping)
- [ ] Links in text are underlined
