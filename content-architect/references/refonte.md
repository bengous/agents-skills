# Reference: Redesign of Existing Products (Sites, Apps)

## 1. When This Applies

Redesign of an existing site or app. The product exists, has users, but the structure has grown organically, pages are redundant, navigation is confusing, or the product is being repositioned. Also covers migrations (e.g., moving from one CMS to another where structure is reconsidered).

## 2. Specific Inputs to Collect

Beyond the base SKILL.md inputs:
- URL of current product (or screenshots, repo, sitemap.xml)
- Analytics if available (top pages by traffic, bounce rates, user flows, search queries)
- Known pain points from users or stakeholders ("users can't find X", "nobody uses the Y page")
- What must be preserved: branding, specific URLs (for SEO), features users depend on
- What the redesign is trying to fix (and what prompted it -- a rebrand? a business pivot? accumulated debt?)

## 3. The Feature Parity Trap

Before auditing, settle this question:

**Martin Fowler**: "People massively underestimate the effort required for feature parity." The **Standish Group (2014)** reports that **50% of existing features are unused**. Zalando discovered features untouched for years when rebuilding their CMS. Basecamp's "Files 2.0" launched without a defined scope and failed -- they recovered by cutting into specific projects ("Better file previews", "Custom folder colors").

**Jackie Bavaro's diagnostic question** -- ask this before deciding what to keep:
> "Suppose we reach feature parity in a year. Why would buyers choose us then?"

If the answer is "because of the new features we'll add after parity" -- then parity is a trap. Cut harder. If the answer is "because the existing features will work better" -- then parity matters, but audit which 50% to keep.

The **debt.design** framework distinguishes three levels:
- **Facelift**: Visual UI changes only (paint job)
- **Refresh**: UI + targeted UX adjustments (renovation)
- **Redesign**: Fundamental structural changes (demolition + rebuild)

Knowing which level you're at prevents scope drift.

## 4. Audit Protocol

This is the core of a redesign. Run these steps in order:

### Step 1: Screen Census
List every existing screen/page:
- For websites: crawl the sitemap.xml, walk the navigation, check the footer, check for orphan pages
- For apps: walk every navigation path, check settings, check onboarding, check error pages
- Record: URL/path, title, brief content description

### Step 2: Job Assignment
For each existing screen, state its job in one sentence: "This screen [does X] so the user can [Y]."
- If you cannot state the job -- mark as "unclear job" (candidate for cut)
- If two screens have the same job -- mark both as "redundant" (candidate for merge)
- If a screen has multiple jobs -- mark as "overloaded" (candidate for split)

### Step 3: Redundancy Scan
Identify content that appears on multiple screens:
- Same text blocks copy-pasted across pages
- Same data shown in different formats on different screens
- Same CTA leading to the same destination from multiple places (this one is OK -- CTAs can repeat; content cannot)

For each duplicate, mark which version is canonical.

### Step 4: Dead-End Detection
Find broken paths:
- **Orphan pages**: Exist but are not linked from any navigation or other page (found only via direct URL or search)
- **Dead-end pages**: No outbound links, no CTA -- the user is stuck
- **Broken links**: CTAs that lead to 404s, forms with no confirmation, buttons that do nothing
- **Circular paths**: User follows a CTA only to end up back where they started

### Step 5: Analytics Overlay (if available)
- Top 10 pages by traffic (critical -- don't remove or break their URLs)
- Bottom 10 pages by traffic (candidates for cut, but check if they serve niche but important users)
- Highest bounce rate pages (content mismatch? slow load? bad UX?)
- Most common exit pages (is the user leaving satisfied or frustrated?)
- Internal search queries (what are users looking for that they can't find in the nav?)

### Step 6: Verdict per Screen
Every existing screen gets exactly one verdict:

| Verdict | Meaning | Action |
|---------|---------|--------|
| **Keep as-is** | The screen works, the job is clear, the content is good | Migrate content, preserve URL |
| **Keep but restructure** | The job is right but the content/sections need rework | Rewrite sections, may change URL |
| **Merge with [screen]** | Two screens do the same job or serve the same journey step | Combine content, 301 redirect old URL |
| **Split into [A] and [B]** | The screen is overloaded with multiple jobs | Create two screens, redirect old URL to primary |
| **Cut** | Nobody uses it, job is unclear, or content is stale | 301 redirect to nearest relevant page |

## 5. URL Preservation Strategy

For any page with organic search traffic:
- **301 redirect** old URL to the new equivalent (not to the homepage -- to the closest match)
- Preserve URL structure where possible (/services/plumbing stays /services/plumbing)
- Create a redirect map: old URL -> new URL, and include it in the blueprint
- For pages being cut: redirect to the parent category or the most relevant remaining page

## 6. Progressive Migration Strategy

If the redesign is incremental (not a big-bang launch):
- **Phase 1**: Migrate the highest-traffic pages first (they carry the most SEO value and user impact)
- **Phase 2**: Migrate the journey-critical pages (the ones that serve the critical paths)
- **Phase 3**: Migrate or cut the remaining pages
- At each phase: verify redirects work, monitor analytics for traffic drops, check for broken internal links

## 7. Worked Example

A company website with 22 existing pages:
- **Audit results**:
  - Clear job: 10 pages (Home, About, 4 Service pages, Contact, Blog index, 2 Blog posts)
  - Unclear job: 5 pages (Resources, Partners, Press, Careers intro, "Why us")
  - Redundant: 4 pages (3 landing pages repeating service content, 1 duplicate About variant)
  - Overloaded: 2 pages (Services overview mixing all services with testimonials and pricing)
  - Dead-end: 1 page (old event page with no links out)
- **Verdicts**: 8 keep (some restructured), 3 merge into existing, 3 merge pairs into singles, 8 cut
- **Result**: 22 pages -> 11 pages
- **Before sitemap**: 22 nodes, 3-level deep, 8 nav items
- **After sitemap**: 11 nodes, 2-level deep, 5 nav items
- **Redirect map**: 11 old URLs -> new destinations

## 8. Pitfalls

- **Redesigning without auditing**: You will recreate the same problems with a fresh coat of paint. Always audit first.
- **Cutting pages with SEO value**: Check search traffic before removing any page. A page with 500 monthly organic visits needs a 301 redirect, not deletion.
- **Preserving screens out of politics**: "The CEO's pet page" or "Marketing spent 3 months on this". If nobody visits it and it has no clear job, it should go. Present the data.
- **Scope creep**: "While we're at it, let's add 15 new features." A redesign is about restructuring existing value, not building new things. New features go into v2.
- **The content migration gap**: Redesigning the structure without migrating the content. A perfect new structure with placeholder text is not a redesign -- it's a skeleton. Plan the content migration as part of the project.
- **Not finishing the redesign**: The debt.design framework warns: not finishing a redesign creates MORE debt than existed before. An inconsistent half-migrated product is worse than the original. Scope tightly, finish what you start.
