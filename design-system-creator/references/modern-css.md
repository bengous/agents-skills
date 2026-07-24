# Modern CSS — Opt-In Additions

The default references use hex colors, px values and viewport media queries on
purpose: they work everywhere and are trivial to audit. Everything below is
**opt-in**. Adopt it when the project's browser matrix allows — every feature
here is Baseline "widely available" in 2026 **except scroll-driven animations**
(progressive enhancement only). When in doubt, ship the defaults.

Rule of thumb: opt in per feature, not wholesale. Each section states what it
replaces in the default references.

---

## 1. OKLCH for palette construction

Hex values are fine as delivery format, but OKLCH is a better *construction*
space: equal lightness steps look equally spaced (unlike HSL), so ramps stay
perceptually even across hues.

```css
:root {
  --color-action: oklch(62% 0.12 55);          /* copper */
  --color-action-hover: oklch(68% 0.12 55);    /* same hue/chroma, +6 lightness */
  --color-action-pressed: oklch(55% 0.14 55);  /* darker, slightly more chromatic */
}
```

Replaces: hand-picked hex stops for hover/pressed in `root-template.md`'s
action family. Keep the hex fallback comment if the matrix is uncertain.

## 2. color-mix() and relative color syntax

Derive alpha variants and state stops from one source token — this removes the
need for the `--color-*-rgb` triplet hack entirely.

```css
/* Focus ring at 20% — replaces rgba(var(--color-action-rgb), 0.2) */
.field__input:focus {
  box-shadow: 0 0 0 2px color-mix(in oklch, var(--color-action) 20%, transparent);
}

/* Same via relative color syntax */
.field--error .field__input:focus {
  box-shadow: 0 0 0 2px rgb(from var(--color-error) r g b / 20%);
}

/* Derived hover stop */
:root {
  --color-action-hover: color-mix(in oklch, var(--color-action) 85%, white);
}
```

Replaces: `--color-action-rgb` / `--color-error-rgb` fallbacks in
`component-patterns.md` (focus rings), hardcoded rgba gradients in the image
overlay pattern. If you opt in, drop the `*-rgb` tokens from the deliverable.

## 3. light-dark() + color-scheme

For 1:1 token swaps between themes, `light-dark()` removes the duplicated
`[data-theme]` blocks and fixes native form-control rendering.

```css
:root {
  color-scheme: light dark;  /* or "dark light" — anchoring decides the default */
  --color-bg: light-dark(#F5F0E8, #1C1A17);
  --color-text: light-dark(#1C1A17, #F0EBE3);
}

/* User override still possible */
[data-theme="dark"] { color-scheme: dark; }
[data-theme="light"] { color-scheme: light; }
```

Replaces: the theme-switching skeleton in `design-system-template.md` when the
theme pair is a pure token swap. Keep the `[data-theme]` skeleton when themes
differ structurally (different shadows, different imagery).

## 4. Container queries

Components that live in varying containers (cards in a 3-col grid, a sidebar,
a modal) should adapt to their container, not the viewport.

```css
.card-grid { container-type: inline-size; }

@container (max-width: 480px) {
  .card { flex-direction: column; }
  .card__media { aspect-ratio: 16/9; }
}
```

Replaces: viewport-only grid breakpoints in `responsive-rules.md` for
*component-level* adaptation. Page-level layout keeps viewport media queries.

## 5. :has() for state styling

Style a parent from its children's state — no JS class toggling.

```css
/* Field wrapper reacts to invalid input */
.field:has(:user-invalid) .field__label { color: var(--color-error); }

/* Card layout adapts when it contains an image */
.card:has(.card__media) { padding-top: 0; }
```

Replaces: the `.field--error` JS-managed class in `component-patterns.md`
(keep the class as fallback for server-rendered error states).

## 6. Scroll-driven animations (progressive enhancement ONLY)

Declarative alternative to the IntersectionObserver reveal pattern. Not yet
Baseline widely available — always wrap in `@supports` and keep the JS path.

```css
@supports (animation-timeline: view()) {
  .reveal {
    animation: reveal-in linear both;
    animation-timeline: view();
    animation-range: entry 0% entry 40%;
  }
  @keyframes reveal-in {
    from { opacity: 0; transform: translateY(24px); }
    to   { opacity: 1; transform: none; }
  }
}
```

Replaces (when supported): the IntersectionObserver JS in
`motion-principles.md` Pattern 3. Reduced-motion rules still apply.

## 7. @property for animatable tokens

Custom properties interpolate as strings by default — gradients and numeric
tokens jump instead of animating. `@property` declares the type so they
interpolate correctly.

```css
@property --gradient-angle {
  syntax: "<angle>";
  initial-value: 0deg;
  inherits: false;
}

.card--featured {
  background: conic-gradient(from var(--gradient-angle), var(--color-surface), var(--color-bg));
  transition: --gradient-angle var(--transition-medium);
}
.card--featured:hover { --gradient-angle: 180deg; }
```

Use sparingly — only when a token genuinely needs to animate.
