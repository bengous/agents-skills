# Motion Principles — When, Why, and How to Animate

Motion is not decoration. It has three jobs: guide attention, confirm an action, create continuity. If an animation does none of these three jobs, it has no place.

This document defines motion **patterns**, not tools. Native CSS, GSAP, Framer Motion, Lenis — the stack choice depends on the project. The principles remain the same.

---

## The Three Speeds

| Token | Duration | Easing | Usage |
|---|---|---|---|
| `--transition-fast` | 0.15–0.2s | `ease` or `ease-out` | Hover, focus, toggle, micro-feedback |
| `--transition-medium` | 0.3–0.4s | `ease` or `ease-in-out` | Open/close (menu, accordion, modal) |
| `--transition-slow` | 0.5–0.7s | `ease-out` or `cubic-bezier(0.16, 1, 0.3, 1)` | Scroll reveal, page transitions, fade-in on load |

**Rule**: one single duration per category across the entire site. No 0.2s here and 0.35s there for the same type of interaction.

---

## Pattern 1 — Hover & Focus (fast)

The user touches something, the site responds instantly.

### What to animate
- Button background color
- Link text color
- Interactive card border
- Icon opacity
- Light transform (translateY 1-2px on buttons at `:active`)

### What NOT to animate
- Font size (layout jump)
- Element width/height (expensive reflow)
- Box-shadow in a loop (performance)

### Typical CSS

```css
.element {
  transition: color var(--transition-fast),
              background-color var(--transition-fast),
              border-color var(--transition-fast),
              transform var(--transition-fast);
}
```

**Performance tip**: always list properties explicitly. `transition: all` is an anti-pattern — it animates things you do not want animated (padding, margin, width) and costs performance.

---

## Pattern 2 — Open / Close (medium)

An element appears or disappears. Mobile menu, accordion, modal, dropdown.

### Principles
- The element arrives from somewhere (no magic pop)
- It leaves toward somewhere (no instant disappearance)
- Entry direction = reversed exit direction

### Typical CSS — Mobile menu (slide)

```css
.menu {
  transform: translateX(100%);
  transition: transform var(--transition-medium);
}
.menu--open {
  transform: translateX(0);
}
```

### Typical CSS — Accordion (expand)

```css
.accordion__content {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows var(--transition-medium);
}
.accordion__content--open {
  grid-template-rows: 1fr;
}
.accordion__inner {
  overflow: hidden;
}
```

### Typical CSS — Modal (fade + scale)

```css
.modal-overlay {
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--transition-medium);
}
.modal-overlay--visible {
  opacity: 1;
  pointer-events: auto;
}

.modal {
  transform: scale(0.95) translateY(10px);
  opacity: 0;
  transition: transform var(--transition-medium),
              opacity var(--transition-medium);
}
.modal-overlay--visible .modal {
  transform: scale(1) translateY(0);
  opacity: 1;
}
```

---

## Pattern 3 — Scroll Reveal (slow)

Content appears as the user scrolls.

### Principles
- **One single direction** of reveal for the entire site. Bottom → top in 95% of cases.
- **Short distance**: translateY of 20-30px max. Not 100px — that is exhausting.
- **Stagger** (offset) between elements in the same group: 80-120ms per element.
- **Once only**: once revealed, the element does not hide again on scroll up. (Exception: if the anchoring justifies continuous parallax.)

### Typical CSS — with Intersection Observer

```css
/* Initial state (before reveal) */
.reveal {
  opacity: 0;
  transform: translateY(24px);
  transition: opacity var(--transition-slow),
              transform var(--transition-slow);
}

/* Final state (after reveal) */
.reveal--visible {
  opacity: 1;
  transform: translateY(0);
}

/* Stagger for grids */
.reveal--stagger:nth-child(1) { transition-delay: 0ms; }
.reveal--stagger:nth-child(2) { transition-delay: 100ms; }
.reveal--stagger:nth-child(3) { transition-delay: 200ms; }
.reveal--stagger:nth-child(4) { transition-delay: 300ms; }
/* Max 4-5 elements in stagger. Beyond that, the last one waits too long. */
```

### Minimal JS for the trigger

```js
const observer = new IntersectionObserver((entries) => {
  entries.forEach(entry => {
    if (entry.isIntersecting) {
      entry.target.classList.add('reveal--visible');
      observer.unobserve(entry.target);  // once only
    }
  });
}, { threshold: 0.15 });

document.querySelectorAll('.reveal').forEach(el => observer.observe(el));
```

---

## Pattern 4 — Page Load (slow, once only)

The first impression. The hero composes itself on load.

### Principles
- **Sequential**: elements do not all arrive at the same time. Title → subtitle → CTA → image, with 150-200ms offset.
- **Short**: the full animation lasts 1.2s max. Beyond that the user is waiting.
- **No loader**: if the content is HTML/CSS, it must appear fast. Loading animations are for heavy apps, not showcase sites.

### Typical CSS

```css
.hero__title {
  opacity: 0;
  transform: translateY(20px);
  animation: hero-reveal 0.6s ease-out 0.1s forwards;
}
.hero__subtitle {
  opacity: 0;
  transform: translateY(20px);
  animation: hero-reveal 0.6s ease-out 0.3s forwards;
}
.hero__cta {
  opacity: 0;
  transform: translateY(20px);
  animation: hero-reveal 0.6s ease-out 0.5s forwards;
}

@keyframes hero-reveal {
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
```

---

## Pattern 5 — Scroll Behavior (smooth scroll, parallax)

### Smooth Scroll

```css
html {
  scroll-behavior: smooth;
}
```

Sufficient for 90% of sites. For a more elaborate smooth scroll (inertia, smoothing), tools like Lenis are an option — but only if the anchoring justifies it (premium editorial site, immersive portfolio). On an e-commerce or utility site, native scroll is preferable.

### Parallax

Parallax is rarely justified. It is when:
- The anchoring implies depth (architecture, space, nature)
- The site is primarily visual (portfolio, gallery)
- The content is short (landing page, one-pager)

It is NOT when:
- The site is text-heavy (blog, e-commerce)
- The user needs to act (forms, checkout)
- Mobile is the primary target (parallax is expensive on mobile performance)

---

## Reduced Motion

**Mandatory.** All animations must be disableable.

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

This block is non-negotiable. It goes in the global CSS, before everything else.

---

## Anti-patterns

| Mistake | Why | Alternative |
|---|---|---|
| Animating body text (fade-in paragraph by paragraph) | Exhausting, slows down reading | Animate headings and blocks, not individual paragraphs |
| Bounce / elastic easing everywhere | Playful ≠ universal. It only suits very playful universes. | `ease-out` for reveals, `ease` for hovers |
| Infinite scroll animations (continuous parallax) | Terrible mobile performance + nausea | Reserve for short sections, disable on mobile |
| Duration > 0.7s for a reveal | The user has time to get impatient | Max 0.6-0.7s, content must be readable quickly |
| TranslateY > 40px | Movement too wide, "falling" effect | 20-30px max — movement is a whisper, not a shout |
| `transition: all` | Animates unintended properties, costs perf | List each property explicitly |
| No reduced-motion | Inaccessible | The `prefers-reduced-motion` block is mandatory |

---

## Motion Checklist

- [ ] The three speeds (fast/medium/slow) are defined and consistent across the entire site
- [ ] Button and link hover/focus has a transition-fast
- [ ] The mobile menu has an open/close animation
- [ ] Scroll reveal uses IntersectionObserver (no continuous scroll calculation)
- [ ] Stagger does not exceed 4-5 elements
- [ ] The page load hero lasts 1.2s max total
- [ ] `prefers-reduced-motion: reduce` is implemented
- [ ] No animation on body text
- [ ] `transition: all` does not appear anywhere in the CSS
