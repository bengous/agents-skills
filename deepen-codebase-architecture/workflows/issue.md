# Issue Workflow

Precondition: accepted design. If missing, ask for the accepted design or run design mode.

Load `references/issue-template.md`.

Create an issue only when the user requested `issue` mode, selected `full` mode and approved the design, or explicitly says to create it.

## Before Creation

1. Check repository remote and current issue tooling.
2. Prefer `gh issue create` when authenticated and available.
3. Include concrete authority files but avoid over-coupling the issue to transient paths.
4. State labels only if the repo already uses matching labels.

## Output

```markdown
Created: <url>

Summary: ...
```
