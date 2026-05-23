---
name: of
description: "USER-INVOKED ONLY. Use only when the user explicitly invokes $of or $op, or explicitly names this skill to open/view a local file. Never invoke implicitly. Opens visualizable files such as images, Mermaid, Markdown, HTML, PDF, and text using the system default app."
---

# Open File

Open one local file the user explicitly asks to view.

Human-invoked only. Do not activate implicitly from a merely mentioned path or
artifact.

## Input

Optional:

```text
path: <file path>
```

If no path is provided, infer the target from the most recent concrete local file
path in the conversation or tool output. Prefer generated/opened artifacts over
their sources when the user says "open it", "view it", "$of", or "$op" without a
path.

## Workflow

1. Pick the target:
   - explicit path in the user prompt wins;
   - otherwise use the latest unambiguous local file path from context;
   - if several files are equally plausible, ask one short question.
2. Resolve the path against the current working directory when relative. Expand
   `~` to `$HOME`; do not use shell `eval`.
3. Verify the file exists before opening it.
4. Open with the platform default viewer:
   - Linux: `xdg-open -- "$path"`
   - macOS: `open "$path"`
   - Windows/Git Bash: `start "" "$path"`
5. Run the opener in the background when the terminal command would otherwise
   block. Report the exact file opened.

## Guardrails

- Open local files only. For URLs or remote resources, use the normal browser
  tooling only when the user clearly asked for that.
- Do not modify, render, convert, or regenerate the file unless the user asked.
- Do not guess between multiple candidate files with the same recency.
