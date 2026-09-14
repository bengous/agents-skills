# Full Workflow

Run the interactive chain. Do not load all workflow files at once; load each file only when that stage starts.

1. Load `workflows/audit.md` and audit candidates.
2. Ask the user to choose one.
3. Load `workflows/deep-dive.md` for the chosen candidate.
4. Load `workflows/design.md` and design competing interfaces.
5. Ask the user to approve or adjust the recommendation.
6. Load `workflows/issue.md` and `references/issue-template.md` after approval.
7. Create the RFC issue.

If the user explicitly asks for autonomy, choose the strongest candidate and recommendation yourself, but still make issue creation explicit unless their request already authorized it.
