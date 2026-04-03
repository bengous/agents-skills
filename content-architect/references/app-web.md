# Reference: Web Applications, Dashboards & SaaS

## 1. When This Applies

Dashboards, SaaS products, admin panels, internal tools, data-heavy applications
with user accounts and CRUD operations. Projects where users log in, manage data,
and perform workflows. Includes B2B tools, project management apps, CRMs, analytics
platforms, and any multi-user application with role-based access.

## 2. Specific Inputs to Collect

Beyond the base SKILL.md inputs, ask:

- **User roles and permissions**: Admin, member, viewer? What can each role do?
  Which screens are role-gated? Any role that can only read but not write?
- **CRUD entities**: What objects do users create/read/update/delete?
  (e.g., projects, tasks, invoices, users, reports)
- **Data relationships**: Which entities relate to each other?
  (e.g., projects have tasks, tasks have assignees, invoices belong to clients)
- **Notification model**: Email, in-app, push? What triggers notifications?
  Who receives them? Can users control frequency?
- **Onboarding needs**: First-run experience? Wizard? Template gallery?
  What is the single action that delivers first value?

## 3. SaaS Navigation Patterns

The convergent pattern from Linear, Notion, Figma, Slack:

- **Sidebar for orientation** + **Cmd+K command palette for action**. The sidebar
  tells you "where am I?", the command palette answers "how do I get to X?"
- 6-10 top-level items in the sidebar, collapsible sections for sub-items.
- User customization: favorites, reordering, pinned items.
- Standard width: 200-300px, fixed on desktop, overlay/drawer on mobile.
- Icons + text labels always (never icons alone -- discoverability).

**Linear as reference**: whitespace-dominant, zero visual noise by default, Cmd+K
as primary navigation tool. Linear is the current gold standard for SaaS UI --
minimal by default, powerful via keyboard.

**Progressive disclosure** (Nielsen 2006, still fully relevant): show the essential
first, reveal the rest on demand. Patterns: accordions, tabs, dropdowns, "Show more",
multi-step forms. Caveat: too much hiding kills discoverability -- Notion had Quick
Find buried in the sidebar for too long.

## 4. CRUD Screen Patterns

| Operation | Screen pattern | Key considerations |
|-----------|---------------|-------------------|
| List (index) | Data table with sort, filter, search, pagination | Column visibility controls, bulk actions if >50 items expected |
| View (detail) | Full page or side panel | Show relationships, include quick actions |
| Create | Modal or separate page with form | "+" button or "Create" CTA. If entity can start empty, create-then-edit |
| Edit | Inline editing or pre-filled form | Auto-save vs explicit save, real-time validation |
| Delete | Confirmation dialog or undo pattern | "Recently deleted" for recovery. Animate removal from list |
| Bulk operations | Multi-select + action bar | Clear selection counter, confirmation for destructive actions |

## 5. Dashboard Pattern

A dashboard answers "what needs my attention right now?" -- not "here is every
metric we track."

- Lead with actionable items (tasks due, notifications, alerts).
- 3-5 key metrics maximum, chosen by role.
- Recent activity feed (what happened since last login).
- Quick actions (create new X, resume last workflow).
- NEVER: 12 charts nobody reads, vanity metrics, data without context.

## 6. Screen States

Every list/data screen needs these states:

- **Empty state**: "No [X] yet. [Create one] to get started." Include the primary
  CTA. Reference IBM Carbon and PatternFly patterns. Vercel shows the exact CLI
  command in its empty states -- for dev tools, this is the gold standard.
- **Loading state**: Skeleton screens, not spinners. A skeleton for 500ms *feels*
  fast; a spinner for 500ms *feels* slow (20-30% improvement in perceived speed).
- **Error state**: What happened + why + what the user can do. "Could not load
  projects. Check your connection and try again." Never: "Error 500" or "Something
  went wrong."
- **Populated state**: The normal case -- with enough data to show the full UI
  (pagination, filters, etc.)

## 7. Onboarding Patterns

Named patterns that work in 2025-2026:

- **Welcome Survey / Routing Question**: One question at signup reshapes the entire
  experience (Notion: personal/team/school; Airtable: use case).
- **Learn-by-Doing**: Create something real immediately (Figma: draw a shape, apply
  a color).
- **Interactive Checklist**: Task list with progress bar (Notion "Getting Started",
  Shopify personalized by business type).
- **Empty State Onboarding**: Empty screens guide with context + CTA (Dropbox,
  Vercel).
- **Template Pre-loading**: Pre-populate with data/templates (Notion: personalized
  selection by role).

Key metrics: Time-to-Value < 2 minutes. Max 3 signup fields (each extra field costs
each extra field measurably reduces conversion). Static tooltip tours are dead -- users dismiss them in
seconds.

## 8. Common Screens

- **Dashboard**: Job: "Show what needs attention right now." MVP.
- **[Entity] List**: Job: "Browse, search, and filter [entities]." One per major entity. MVP.
- **[Entity] Detail**: Job: "View and manage a single [entity]." MVP.
- **[Entity] Create/Edit**: Job: "Create or modify an [entity]." MVP.
- **Settings (User)**: Job: "Manage personal preferences." v2.
- **Settings (Org/Admin)**: Job: "Manage workspace/team configuration." v2 (unless multi-tenant from day 1).
- **Profile**: Job: "View and edit account information." v2.
- **Onboarding flow**: Job: "Guide new users to first value." MVP if retention matters.
- **Notifications**: Job: "Show what happened and what requires action." v2.
- **Help/Docs**: Job: "Self-serve answers before contacting support." Nice-to-have if docs exist externally.

## 9. Worked Example

A project management SaaS with 3 roles (owner, member, viewer), 4 entities
(project, task, member, report).

**Journeys**:

- New user: Signup -> Welcome survey (role) -> Create first project -> Add first task -> Invite team member
- Daily worker: Dashboard -> Project list -> Project detail -> Task detail -> Update task status
- Manager: Dashboard -> Reports -> Project overview -> Team workload
- Admin: Settings -> Members -> Invite -> Permissions

**Screens** (12): Dashboard, Project list, Project detail, Task detail,
Task create/edit, Member list, Member invite, Report overview, Settings (user),
Settings (org), Profile, Onboarding

**Nav** (sidebar): Dashboard | Projects | Tasks (filtered view) | Reports | Members | Settings

**MVP** (6 screens): Dashboard, Project list, Project detail, Task create/edit,
Task detail, Onboarding

## 10. Pitfalls

- **Dashboard-as-homepage syndrome**: Cramming every metric and feature on one
  screen. A dashboard is a triage tool, not a feature showcase.
- **Settings sprawl**: 50 toggle switches on one page. Group by domain
  (notifications, privacy, appearance, billing), use tabs or accordion sections.
- **Missing empty states**: A blank table with column headers and no rows is
  hostile. Always tell the user what to do next.
- **Forgetting onboarding**: The first 2 minutes determine whether a user stays.
  If they do not reach value fast, they churn.
- **Navigation-by-org-chart**: Sections named after internal teams (Marketing,
  Engineering, Finance) instead of user tasks (Projects, Analytics, Billing).
