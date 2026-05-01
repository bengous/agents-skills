# Content Architecture Principles

These principles govern content architecture decisions. Use them when the main skill's short rules are not enough.

## Core Principles

### 1. One Screen, One Job

Each screen is hired for exactly one purpose. If you cannot state the job in a single sentence ("This screen [does X] so the user can [achieve Y]"), the screen tries to do too much. Split it or cut it.

This principle converges from Job Stories, "one thing per page" guidance, and single-CTA landing page practice.

### 2. Zero Redundancy

Content lives in one canonical place. If the same information appears on two screens, one of them links to the other. Do not duplicate content because duplicated content creates maintenance debt and conflicting versions.

### 3. Information Scent

Users predict what they will find behind a link based on its label. Every navigation label must pass the 4S test:

- Specific: not "Resources" or "Solutions".
- Sincere: delivers what it promises.
- Substantial: leads to real content, not a redirect.
- Succinct: 1-3 words.

Validation gate: can a first-time visitor predict what they will find behind every nav label?

### 4. Product-Type-Aware Navigation Sizing

The "7 items max" rule is a debunked misreading of Miller's Law. Navigation capacity depends on product type:

- Horizontal nav for showcase sites: 5-7 items.
- Mobile tab bar: 3-5 items, max 5.
- SaaS sidebar: 6-10 groups with collapsible sub-items.
- Mega menus: 30+ items are fine with proper grouping.

### 5. Subtract Before Adding

The first question for every proposed screen: what breaks if this screen does not exist? If nothing breaks, cut it. Every screen has maintenance cost.

### 6. Journeys Dictate Structure

Map where users come from and where they need to go. Screens exist to serve journeys, not the other way around. Trace the critical user journeys first, then derive the minimum screen set.

### 7. Content-First

Priority Guides replace wireframes: list content elements by priority with real text, not layout rectangles. No screen spec uses placeholder text. Every section has a real headline, CTA label, and description, even if approximate.

### 8. Blueprint Is The Contract

A developer or AI agent must be able to implement each screen from the blueprint alone, without asking questions about content or structure. If the blueprint is ambiguous, it is incomplete.

## Full Workshop Gates

- Present journeys to the user for validation before proposing screens.
- Present screen inventory for validation before designing navigation.
- Present priorities for validation before producing the final blueprint.

## Anti-Patterns

- Screen without a job: define the job or cut the screen.
- Duplicated content across screens: choose the canonical location and link to it.
- Navigation built around the org chart: navigation reflects user goals, not company structure.
- Vague nav labels: avoid labels like "Resources", "Solutions", "Insights", "Explore".
- Screens added just in case: subtract before adding.
- Screens before journeys: screens without journeys are guesses.
- Placeholder text in blueprints: real headline, CTA, and description are required.
- Feature parity as default: in redesigns, audit first and keep only what serves a journey.

## Source Hygiene

Name frameworks and patterns when they materially inform the architecture. Examples: Jobs to Be Done, information foraging, 4S navigation labels, PAS landing page framework, story mapping, RICE, Shape Up appetite, CLI-first dashboard-optional models, and the feature parity trap.
