# CLI-to-Frontend Migration Reference

## 1. When This Applies

An existing CLI tool (developer tool, ops tool, deployment tool, data tool) needs a web frontend. Typical motivations: onboarding non-technical users, adding team visibility and audit trails, complementing the CLI with visual workflows, or reducing operator errors through guided forms and validation.

## 2. Specific Inputs to Collect

Beyond the base SKILL.md inputs, gather:

- **Full command inventory**: every command and subcommand (from `--help`, README, or docs).
- **Per-command details**: what it does, arguments/options, output format (table, JSON, logs, status), whether it is read-only or mutating.
- **Usage frequency**: daily, weekly, rarely -- per command.
- **Current audience**: developers only, ops, non-technical users, mixed.
- **Multi-command workflows**: sequences like "first run X, then verify with Y, then apply with Z."
- **Motivation for the frontend**: onboard non-technical users, add visibility, compliance, reduce errors, other.

## 3. The CLI-First Philosophy

**Vercel** embodies "CLI-first, dashboard optional." The dashboard complements the CLI, never replaces it.

- The GUI must not remove any capability that exists in the CLI.
- Terminal parity check: every CLI workflow must be achievable in the GUI. The GUI can add, but never subtract.
- Performance IS the design: developers notice 300ms delays. Use optimistic UI and stale-while-revalidate patterns (show expected state immediately, sync in background).
- Dark mode by default for dev tools -- developers work in dark terminals.
- Empty states show the exact CLI command to get started, not decorative illustrations.

**Supabase** separates concerns: visual exploration (dashboard) for understanding, CLI for reproducible operations (migrations as versioned files).

**Shopify** is migrating the other direction -- from dashboards to CLI for extension management: "you can no longer create and manage app extensions via dashboards -- only via Shopify CLI." This reinforces that CLI and GUI serve different jobs; forcing everything into one surface backfires.

## 4. Decision Tree: Screen, Action, or Stays CLI?

For each CLI command, apply this decision tree:

| Command type | Frontend treatment | Rationale |
|---|---|---|
| READ with complex/relational data | Dashboard table or visualization | Humans scan tables and charts faster than terminal output |
| READ status/monitoring | Dashboard with real-time updates | Visual monitoring beats polling with `watch` |
| CREATE/UPDATE with many inputs | Form or wizard | Forms prevent typos, validate in real-time, show defaults |
| Automation/CI-CD operations | Stays CLI | Must be scriptable, non-interactive, pipeable |
| Operations needing version control | Stays CLI | Config files on filesystem, diffs in git |
| Non-technical audience | Dashboard essential | They will not learn CLI syntax |
| Developer solo | CLI preferred | Faster, scriptable, keyboard-native |
| Both audiences | Both (Vercel model) | CLI for power, dashboard for visibility |

**Pareto rule**: surface 20% of operations for 80% of users in the dashboard. Not every flag needs a form field.

## 5. CRUD-to-UI Mapping

| CLI operation | UI pattern | Key considerations |
|---|---|---|
| `list` / `get all` | Data table with sort, filter, search, pagination | Semantic `<table>`, column visibility toggles |
| `get <id>` / `show` / `describe` | Detail view (full page or side panel) | Show relationships, include quick actions |
| `create` | Form (modal or separate page) | Required CLI flags = required form fields; optional flags = advanced section |
| `update` / `set` | Inline editing or pre-filled form | Decide auto-save vs explicit save based on risk |
| `delete` / `remove` | Confirmation dialog or undo pattern | Consider "recently deleted" for recovery |
| Bulk operations | Multi-select + action bar | Only if CLI has bulk equivalents |
| `logs` / `tail` | Streaming log viewer | Auto-scroll, pause, filter by level, search |
| `status` / `health` | Status dashboard or badge | Color-coded, with timestamp |
| `config` / `init` | Settings form | Group by section, show current values |

## 6. Multi-Command Workflows as Wizards

CLI users chain commands: `deploy create` then `deploy status` then `deploy logs` then `deploy promote`. These become **wizard flows** in the GUI:

- Each command in the chain = one step in the wizard.
- Maximum 10 steps per wizard.
- Validate at each step, not at final submission.
- Final button = specific verb ("Deploy to production", not "Finish").
- Show the equivalent CLI command at each step -- for learning and for power users who want to script it later.

## 7. Common Screens for CLI-to-Frontend Projects

- **Overview / Dashboard**: replaces `status` commands. Job: show current system state at a glance.
- **[Resource] List**: replaces `list` commands. Job: browse, search, and act on resources.
- **[Resource] Detail**: replaces `show`/`describe` commands. Job: view full details of a single resource.
- **Create / Edit forms**: replaces `create`/`update` commands. Job: create or modify a resource with validation.
- **Logs / Output viewer**: replaces streaming CLI output. Job: monitor real-time output with search and filtering.
- **Configuration**: replaces config file editing. Job: manage settings without touching files.
- **Activity / History**: no direct CLI equivalent. Job: see who did what and when. Often the reason the frontend exists in the first place.

## 8. Worked Example

A deployment CLI with 12 commands:

```
deploy list, deploy create, deploy status, deploy logs, deploy promote, deploy rollback,
config get, config set, env list, env set, team list, team invite
```

**Decision tree results**:
- Dashboard screens: deploy list, deploy status, deploy logs, env list, team list, config (6 resources to visualize).
- Form screens: deploy create, env set, config set, team invite (4 mutating operations).
- Wizard: deploy create -> status -> logs -> promote (multi-step workflow).
- Stays CLI-only: deploy rollback (dangerous, must be intentional, rare).

**Screens (8)**: Dashboard (deploy status overview), Deploys list, Deploy detail (status + logs + promote action), Create deploy (form), Environments, Team, Configuration, Activity log.

**Nav (4 items)**: Deploys | Environments | Team | Settings.

**MVP (4 screens)**: Dashboard, Deploys list, Deploy detail, Create deploy -- the core deployment workflow.

## 9. Pitfalls

- **1:1 command-to-screen mapping**: not every command deserves its own screen. `deploy promote` and `deploy rollback` are actions on the Deploy Detail screen, not separate pages.
- **Losing CLI power**: if a dev can do something in the terminal but not in the GUI, the GUI feels like a downgrade. Terminal parity is essential.
- **Ignoring the terminal mental model**: CLI users expect immediacy. Multi-step wizards with slow animations frustrate them. Keep interactions fast and direct.
- **Exposing everything in UI**: 50 CLI flags do not need 50 form fields. Show common options by default, hide advanced options behind an "Advanced" accordion.
- **Generic empty states**: decorative illustrations (a rocket ship, a desert) tell the user nothing. Show the CLI command that would create the first item (`vercel deploy` or `git push origin main`).
- **Forgetting the Activity log**: often the entire reason a frontend exists is visibility -- who deployed what, when. This is frequently the highest-value screen and gets overlooked in planning.
