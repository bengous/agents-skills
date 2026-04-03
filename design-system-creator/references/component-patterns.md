# Component Patterns — CSS Templates

## Table of Contents
- [Buttons](#buttons)
- [Cards](#cards)
- [Navigation](#navigation)
- [Forms](#forms)
- [Images](#images)
- [Iconography](#iconography)
- [Page Container](#page-container)
- [Delivery Checklist](#delivery-checklist)

Each template is a skeleton to fill with the design system tokens. Comments marked `/* ADAPT */` indicate choices to make based on the anchoring.

**Rule**: never copy a template without verifying it is justified by the anchoring. If the anchoring is "raw concrete", pill buttons have no place there.

---

## Buttons

### Primary (CTA)

```css
.btn-primary {
  /* Typography */
  font-family: var(--font-body);
  font-size: var(--text-label);
  font-weight: var(--text-label-weight);
  letter-spacing: var(--text-label-tracking);
  line-height: var(--text-label-lh);
  text-transform: uppercase;  /* ADAPT — uppercase if rigorous anchoring, normal if organic */
  text-decoration: none;

  /* Colors */
  color: var(--color-fond);  /* text on action color = inverted main background */
  background: var(--color-action);
  border: var(--border-width) solid var(--color-action);

  /* Dimensions */
  padding: var(--space-sm) var(--space-lg);  /* ADAPT — vertical sm, horizontal lg minimum */
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-xs);  /* if icon + text */

  /* Shape */
  border-radius: var(--radius-md);  /* ADAPT — md if angular, pill if organic */
  cursor: pointer;

  /* Animation */
  transition: background var(--transition-fast),
              border-color var(--transition-fast),
              transform var(--transition-fast);
}

.btn-primary:hover {
  background: var(--color-action-hover);
  border-color: var(--color-action-hover);
}

.btn-primary:active {
  background: var(--color-action-pressed);
  border-color: var(--color-action-pressed);
  transform: translateY(1px);  /* ADAPT — 1px if subtle, 0 if rigid */
}

.btn-primary:focus-visible {
  outline: 2px solid var(--color-action);
  outline-offset: 2px;
  /* IMPORTANT: never remove focus-visible — accessibility */
}

.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  pointer-events: none;
}
```

### Secondary (ghost/outline)

```css
.btn-secondary {
  /* Inherits typography from primary */
  font-family: var(--font-body);
  font-size: var(--text-label);
  font-weight: var(--text-label-weight);
  letter-spacing: var(--text-label-tracking);
  line-height: var(--text-label-lh);
  text-transform: uppercase;  /* ADAPT — consistent with primary */
  text-decoration: none;

  /* Colors — transparent background */
  color: var(--color-action);
  background: transparent;
  border: var(--border-width) solid var(--color-action);

  /* Dimensions — identical to primary for alignment */
  padding: var(--space-sm) var(--space-lg);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-xs);

  /* Shape — identical to primary */
  border-radius: var(--radius-md);
  cursor: pointer;

  transition: background var(--transition-fast),
              color var(--transition-fast);
}

.btn-secondary:hover {
  background: var(--color-action);
  color: var(--color-fond);
}

.btn-secondary:active {
  background: var(--color-action-pressed);
  color: var(--color-fond);
}

.btn-secondary:focus-visible {
  outline: 2px solid var(--color-action);
  outline-offset: 2px;
}
```

### Text Link

```css
.btn-link {
  font-family: var(--font-body);
  font-size: inherit;
  font-weight: 500;
  color: var(--color-action);
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  text-decoration: none;
  position: relative;
  transition: color var(--transition-fast);
}

/* ADAPT — choose ONE underline style based on the anchoring:

   Option A — classic underline (editorial, legal, sober) */
.btn-link {
  text-decoration: underline;
  text-underline-offset: 3px;
  text-decoration-color: var(--color-action);
}
.btn-link:hover {
  color: var(--color-action-hover);
  text-decoration-color: var(--color-action-hover);
}

/* Option B — underline that appears on hover (modern, minimal)
.btn-link::after {
  content: '';
  position: absolute;
  bottom: -2px;
  left: 0;
  width: 0;
  height: 1px;
  background: var(--color-action);
  transition: width var(--transition-fast);
}
.btn-link:hover::after {
  width: 100%;
}
*/
```

---

## Cards

```css
.card {
  background: var(--color-surface);
  border: 1px solid var(--color-bordure);
  border-radius: var(--radius-md);  /* ADAPT */
  padding: var(--space-lg);
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);

  /* ADAPT — shadow or not? Justify by the anchoring.
     Shadow = floating elements, light materials.
     No shadow = dense materials (metal, concrete, wood). */
  /* box-shadow: var(--shadow-sm); */

  transition: border-color var(--transition-fast);
}

/* Interactive card (product, link) */
.card--interactive {
  cursor: pointer;
}
.card--interactive:hover {
  border-color: var(--color-action);
}

/* Card on light background (alternating section) */
.card--on-light {
  background: var(--color-fond-alt);  /* use the light background token, never hardcoded white */
  border-color: rgba(0, 0, 0, 0.08);
}

/* Image in a card */
.card__image {
  width: 100%;
  aspect-ratio: 4/3;  /* ADAPT — 4/3, 16/9, 1/1 depending on content type */
  object-fit: cover;
  border-radius: var(--radius-sm);  /* ADAPT — same radius or 0 */
  margin-bottom: var(--space-xs);
}

.card__title {
  font-family: var(--font-display);
  font-size: var(--text-h3);
  line-height: var(--text-h3-lh);
  color: var(--color-texte);
}

.card__body {
  font-family: var(--font-body);
  font-size: var(--text-body);
  line-height: var(--text-body-lh);
  color: var(--color-texte-mute);
}

.card__link {
  font-family: var(--font-body);
  font-size: var(--text-small);
  font-weight: 500;
  color: var(--color-action);
  text-decoration: none;
  margin-top: auto;  /* pushes the link to the bottom of the card */
}
```

---

## Navigation

```css
.nav {
  position: sticky;  /* ADAPT — sticky or fixed depending on scroll behavior */
  top: 0;
  z-index: var(--z-sticky);
  background: var(--color-fond);
  border-bottom: 1px solid var(--color-bordure);
  padding: var(--space-md) var(--container-padding);
  display: flex;
  align-items: center;
  justify-content: space-between;

  /* ADAPT — backdrop-blur if the nav should be semi-transparent
  background: rgba(28, 26, 23, 0.9);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  */
}

.nav__logo {
  font-family: var(--font-display);
  font-size: var(--text-h3);
  color: var(--color-texte);
  text-decoration: none;
}

.nav__links {
  display: flex;
  gap: var(--space-lg);
  list-style: none;
}

.nav__link {
  font-family: var(--font-body);
  font-size: var(--text-label);
  font-weight: var(--text-label-weight);
  color: var(--color-texte-mute);
  text-decoration: none;
  letter-spacing: var(--text-label-tracking);
  transition: color var(--transition-fast);
}

.nav__link:hover,
.nav__link--active {
  color: var(--color-action);
}

/* CTA in the nav */
.nav__cta {
  /* Uses the btn-primary style but in a compact version */
  font-family: var(--font-body);
  font-size: var(--text-small);
  font-weight: 500;
  letter-spacing: var(--text-label-tracking);
  padding: var(--space-xs) var(--space-md);
  background: var(--color-action);
  color: var(--color-fond);
  border-radius: var(--radius-md);
  text-decoration: none;
  transition: background var(--transition-fast);
}
.nav__cta:hover {
  background: var(--color-action-hover);
}

/* --- Mobile --- */
@media (max-width: 768px) {
  .nav__links {
    display: none;  /* ADAPT — replace with hamburger menu */
  }

  /* Mobile menu — basic structure */
  .nav__mobile-menu {
    position: fixed;
    inset: 0;
    z-index: var(--z-overlay);
    background: var(--color-fond);
    padding: var(--space-3xl) var(--container-padding);
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
    transform: translateX(100%);
    transition: transform var(--transition-medium);
  }
  .nav__mobile-menu--open {
    transform: translateX(0);
  }
  .nav__mobile-menu .nav__link {
    font-size: var(--text-h2);
    font-family: var(--font-display);
    color: var(--color-texte);
  }
}
```

---

## Forms

```css
/* Field container */
.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
}

.field__label {
  font-family: var(--font-body);
  font-size: var(--text-small);
  font-weight: 500;
  color: var(--color-texte);
  letter-spacing: 0.01em;
}

.field__input {
  font-family: var(--font-body);
  font-size: var(--text-body);
  line-height: var(--text-body-lh);
  color: var(--color-texte);
  background: var(--color-surface);
  border: var(--border-width) solid var(--color-bordure);
  border-radius: var(--radius-md);
  padding: var(--space-sm) var(--space-md);
  transition: border-color var(--transition-fast),
              box-shadow var(--transition-fast);
  outline: none;
}

.field__input::placeholder {
  color: var(--color-texte-mute);
  opacity: 0.6;
}

.field__input:hover {
  border-color: var(--color-structure, var(--color-bordure));
}

.field__input:focus {
  border-color: var(--color-action);
  box-shadow: 0 0 0 2px rgba(var(--color-action-rgb, 184, 115, 64), 0.2);
  /* ADAPT — use the action color at 20% opacity.
     The rgb() fallback must match the project's --color-action. */
}

/* Error state */
.field--error .field__input {
  border-color: var(--color-erreur);
}
.field--error .field__input:focus {
  box-shadow: 0 0 0 2px rgba(217, 64, 64, 0.2);
}

.field__error {
  font-family: var(--font-body);
  font-size: var(--text-small);
  color: var(--color-erreur);
}

/* Textarea */
.field__textarea {
  /* Inherits from field__input */
  resize: vertical;
  min-height: 120px;
}

/* Select */
.field__select {
  /* Inherits from field__input */
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='8' viewBox='0 0 12 8'%3E%3Cpath d='M1 1l5 5 5-5' stroke='%238A8580' stroke-width='1.5' fill='none'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right var(--space-md) center;
  padding-right: var(--space-xl);
  cursor: pointer;
}
```

---

## Images

```css
/* Standard image */
.img {
  width: 100%;
  height: auto;
  display: block;
  border-radius: var(--radius-md);  /* ADAPT — 0 if the anchoring is angular */
}

/* Full-screen image (hero, section) */
.img--full {
  width: 100%;
  border-radius: 0;
  object-fit: cover;
}

/* Image with text overlay */
.img-overlay {
  position: relative;
  overflow: hidden;
  border-radius: var(--radius-md);
}
.img-overlay__image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.img-overlay__content {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  justify-content: flex-end;
  padding: var(--space-lg);
  /* ADAPT — choose ONE gradient type based on the anchoring:
     Dark = artisan, luxury, editorial */
  background: linear-gradient(to top, rgba(28, 26, 23, 0.85) 0%, transparent 60%);
  /* Light = wellness, nature
  background: linear-gradient(to top, rgba(245, 240, 232, 0.9) 0%, transparent 60%); */
}

/* Image with fixed aspect ratio */
.img--landscape { aspect-ratio: 16/9; object-fit: cover; }
.img--portrait  { aspect-ratio: 3/4;  object-fit: cover; }
.img--square    { aspect-ratio: 1/1;  object-fit: cover; }

/* ADAPT — filters if and only if the anchoring justifies it.
   Examples:
   - B&W for an editorial site: filter: grayscale(100%);
   - Slight desaturation for a nature site: filter: saturate(0.85);
   - No filter if the photos are the main content (artisan, portfolio)

   Rule: if the photos show the client's work, NO FILTER.
   Filters are for atmosphere, not for hiding reality. */
```

---

## Iconography

The icon is part of the visual language. Mixing icon styles breaks consistency as much as mixing typefaces.

### Rules

1. **One single style for the entire site.** Outline OR filled OR duotone. No mixing.
2. **One single source.** Choose ONE library and stick with it.
3. **Consistent stroke width.** If outline, define the stroke width (1.5px, 2px) and never vary.
4. **Size on grid.** Icons align to the spacing scale: 16px, 20px, 24px, 32px.
5. **Color = context.** Icon in a button = button color. Standalone icon = `--color-texte-mute`. Interactive icon = `--color-action`.

### Recommended Sources by Anchoring

| Anchoring | Library | Why |
|---|---|---|
| Minimal, technical, SaaS | [Lucide](https://lucide.dev/) | Outline 2px, consistent, lightweight |
| Organic, nature, wellness | [Phosphor](https://phosphoricons.com/) | 6 available styles (thin, light, regular, bold, fill, duotone), smooth transition |
| Artisan, editorial, luxury | [Tabler Icons](https://tabler.io/icons) | Outline 1.5px, sober, large library |
| Data, dashboard, dense interface | [Heroicons](https://heroicons.com/) | Outline + mini + solid, designed for dense UIs |
| Playful, consumer, mobile app | [Phosphor](https://phosphoricons.com/) duotone | Duotone adds personality without overwhelming |

### CSS Token for Size

```css
--icon-sm:  16px;  /* inline with small text */
--icon-md:  20px;  /* inline with body text */
--icon-lg:  24px;  /* buttons, nav */
--icon-xl:  32px;  /* features, highlights */
```

---

## Page Container

```css
.container {
  width: 100%;
  max-width: var(--container-max);
  margin-inline: auto;
  padding-inline: var(--container-padding);
}

/* Full-width section (hero, CTA, full-bleed image) */
.section--full {
  width: 100%;
  padding-inline: 0;
}

/* Standard section with vertical padding */
.section {
  padding-block: var(--space-4xl);  /* ADAPT — reduce on mobile */
}
@media (max-width: 768px) {
  .section {
    padding-block: var(--space-2xl);
  }
}
```

---

## Delivery Checklist

- [ ] Primary and secondary buttons have the same dimensions (side-by-side alignment)
- [ ] All `:focus-visible` states are defined (accessibility)
- [ ] Cards on both dark AND light backgrounds have been handled
- [ ] One single underline style for text links across the entire site
- [ ] Icons come from a single source and a single style
- [ ] Real content images have no CSS filter
- [ ] The mobile menu has been designed (not just `display: none`)
