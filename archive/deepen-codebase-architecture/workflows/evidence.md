# Evidence Workflow

Use before judging architecture when the mode requires code inspection.

1. Read repo instructions, boundary docs, package manifests, validation commands, and relevant rule files.
2. Use semantic navigation when reliable; otherwise use `rg`/file search.
3. Trace at least one real caller path, one test path, and one write/runtime path for the area.
4. Note conflicts between docs and runtime code. Follow code unless the user asked for docs-only repair.
5. Keep unrelated discoveries as TODO/FIXME notes in the report. Do not context-switch.

Friction is evidence only when it repeats across files or callers. One large file is not automatically an architecture problem.
